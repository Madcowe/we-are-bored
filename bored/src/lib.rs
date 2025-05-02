use autonomi::client::payment::PaymentOption;
use autonomi::{Bytes, Client, Network, Scratchpad, SecretKey, Wallet};
use notice::{Display, Notice};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self};
use std::ops::Add;

mod notice;

// Should be entered in order as created as default looks at last element
const PROTOCOL_VERSIONS: [ProtocolVersion; 1] = [ProtocolVersion(1)];

/// Bored protocol version 1 is recorded here and subseqnet version incremented by 1
const CONTENT_TYPE_PROTOCOL_BASE: u64 = 2151856;

/// Version number of the "we are bored" protocol using semantic versioning (major.minor.patch)
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
struct ProtocolVersion(u64);

impl ProtocolVersion {
    /// Create a new instance of ProtocolVersion returning the latest supported by this library
    pub fn new() -> ProtocolVersion {
        PROTOCOL_VERSIONS[PROTOCOL_VERSIONS.len() - 1]
    }

    /// Converts scratchpad content_type to protcol version and check if it is supported
    pub fn check(content_type: u64) -> Result<ProtocolVersion, BoredError> {
        if content_type < CONTENT_TYPE_PROTOCOL_BASE {
            return Err(BoredError::InvalidProtocolVersion(content_type));
        }
        let version = content_type - CONTENT_TYPE_PROTOCOL_BASE + 1;
        for exiting_protocol_version in PROTOCOL_VERSIONS {
            if ProtocolVersion(version) == exiting_protocol_version {
                return Ok(ProtocolVersion(version));
            }
        }
        Err(BoredError::InvalidProtocolVersion(content_type))
    }

    pub fn get_version(&self) -> u64 {
        self.0
    }
}

/// Errors that can occur when using Bored client
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum BoredError {
    #[error("Version of protocol {0} is not know to exist by this implementation of bored")]
    InvalidProtocolVersion(u64),
    #[error("Method is not in this version of the protocol")]
    MethodNotInProtocol,
    #[error("Cannot place notice outside of board")]
    NoticeOutOfBounds,
    #[error("Too much text for notice size")]
    TooMuchText,
    #[error("Could initiate autonomi client")]
    ClientConnectionError,
    #[error("Could not get funded wallet")]
    FailedToGetWallet,
    #[error("JSON serializing/deserializing error")]
    JSONError,
    #[error("Binary serializing/deserializing error")]
    BinaryError,
    #[error("{0}")]
    ScratchpadError(String),
    #[error("{0}")]
    DecryptionError(String),
    #[error("Cannot updated bored as it has not be downloaded this session")]
    BoredNotYetDownloaded,
    #[error("Cannot update as a more recent versoin exists on the bored net")]
    MoreRecentVersionExists(Bored, u64),
    #[error("Hyperlink url is too long at max is {}", notice::MAX_URL_LENGTH)]
    URLTooLong,
    #[error("Error performing regular expression search")]
    RegexError,
    #[error("No notice in that directions")]
    NoNotice,
}

impl From<serde_json::Error> for BoredError {
    fn from(_: serde_json::Error) -> Self {
        Self::JSONError
    }
}

impl From<autonomi::scratchpad::ScratchpadError> for BoredError {
    fn from(e: autonomi::scratchpad::ScratchpadError) -> Self {
        let message = format!("{e}");
        BoredError::ScratchpadError(message)
    }
}

impl From<regex::Error> for BoredError {
    fn from(_: regex::Error) -> Self {
        BoredError::RegexError
    }
}

/// The address of a bored, currently this can only be autonomi::private key that has been
/// used to create a scratchpad with a bored stored in it.
/// Hence this means anyone who has the address can update the board which probalby won't
/// be sensible in a long term project but this is an experiment so starting with the
/// most basic level of a human trust network seems appropriate, you share it you bare it!
enum BoredAddress {
    ScratchpadKey(autonomi::SecretKey),
}
impl fmt::Display for BoredAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            BoredAddress::ScratchpadKey(key) => write!(f, "bored://{}", key.to_hex()),
        }
    }
}

impl BoredAddress {
    /// Generates a new BoredAdress, ie a new autonomi secrkey encapsulated inside this enum
    pub fn new() -> BoredAddress {
        BoredAddress::ScratchpadKey(autonomi::SecretKey::random())
    }

    pub fn get_key(&self) -> &autonomi::SecretKey {
        let BoredAddress::ScratchpadKey(key) = self;
        key
    }

    pub fn get_public_key(&self) -> autonomi::PublicKey {
        match self {
            BoredAddress::ScratchpadKey(key) => key.public_key(),
        }
    }
}

/// A coordiante on a board, the unit of mesauremeant is a character that might appear on screen
// Unsigned as all notice must be within board space, u16 as no readablle board would be that big
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub struct Coordinate {
    x: u16,
    y: u16,
}
impl Coordinate {
    /// returns true if self entirely contained between (0,0) and other
    fn within(&self, other: &Self) -> bool {
        if self.x <= other.x && self.y <= other.y {
            return true;
        }
        false
    }

    fn add(&self, other: &Self) -> Coordinate {
        Coordinate {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

/// Indicate direction of movement
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// a 2d vector of option<uszie> representing the visible contents of the bored
/// if the coordinate is empty it will be none otherwise it will be the
/// notices index of the topmost (most recently added) notice in that position
#[derive(Debug, Clone)]
pub struct WhatsOnTheBored {
    visible: Vec<Vec<Option<usize>>>,
}
impl Iterator for WhatsOnTheBored {
    type Item = Vec<Option<usize>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.visible.iter().next().cloned()
    }
}

impl fmt::Display for WhatsOnTheBored {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut display = String::new();
        for row in &self.visible {
            for coordinate in row {
                let text = match coordinate {
                    None => "*",
                    Some(notice_index) => &notice_index.to_string(),
                };
                display.push_str(text);
            }
            display.push_str("\n");
        }
        write!(f, "{}", display)
    }
}

impl WhatsOnTheBored {
    pub fn create(bored: &Bored) -> WhatsOnTheBored {
        let mut visible = vec![vec![None; bored.dimensions.x.into()]; bored.dimensions.y.into()];
        // for each element in notices put the index in the locations it occupies in whats on the
        // board as the top most items will be later on in the vector hence will overwrite
        // any earlier notices they are occulding
        for (notices_index, notice) in bored.notices.iter().enumerate() {
            for y in notice.get_top_left().y..notice.get_top_left().y.add(notice.get_dimensions().y)
            {
                for x in
                    notice.get_top_left().x..notice.get_top_left().x.add(notice.get_dimensions().x)
                {
                    visible[y as usize][x as usize] = Some(notices_index);
                }
            }
        }
        WhatsOnTheBored { visible }
    }

    fn rotate_horizontally(&mut self) {
        let mut visible: Vec<Vec<Option<usize>>> =
            vec![vec![None; self.visible.len()]; self.visible[0].len()];
        for (y, row) in self.visible.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                visible[x][y] = *cell;
                // eprintln!("{} : {}", x, y);
            }
        }
        self.visible = visible;
        self.visible.iter_mut().for_each(|r| r.reverse());
    }

    fn flip_vertically(&mut self) {
        self.visible.reverse();
    }

    /// flattens into a one dimesonal vectors
    pub fn get_1d(&self) -> Vec<Option<usize>> {
        let mut whats_on_the_bored_1d = vec![];
        for row in &self.visible {
            for cell in row {
                whats_on_the_bored_1d.push(*cell);
            }
        }
        whats_on_the_bored_1d
    }
}

/// Bored, inspired by a pin board a 2d area onto which notices can be placed.
/// If a notice becomes entirley occluded it no longer exists. Once placed notices cannot be
/// moved/edited but can be covered by new ones.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Bored {
    protocol_version: ProtocolVersion,
    name: String,
    dimensions: Coordinate, // the board will range from (0,0) up to this
    notices: Vec<Notice>,
    draft_notice: Option<Notice>,
}

// only methods dealing with the interal items of bored need to perform the protocol check
// so as to avoid calling methods on items that don't exist in bored that are currently
// using an older version of the protocol
impl Bored {
    /// Creates a new board using the most recent protocol version
    pub fn create(name: &str) -> Bored {
        Bored {
            protocol_version: PROTOCOL_VERSIONS[PROTOCOL_VERSIONS.len() - 1],
            name: name.to_string(),
            dimensions: Coordinate { x: 120, y: 40 },
            notices: Vec::new(),
            draft_notice: None,
        }
    }

    /// Add a notice to the board in the spcefied position returns an error if out of bounds
    // Takes cordinate parametre to make sure it is correct with respect to self even
    // though relocate performs a check to a specfifed bored
    pub fn add(&mut self, mut notice: Notice, top_left: Coordinate) -> Result<(), BoredError> {
        if self.protocol_version.get_version() < 1 {
            return Err(BoredError::MethodNotInProtocol);
        }
        notice.relocate(&self, top_left)?;
        self.notices.push(notice);
        return Ok(());
    }

    /// create a draft notice that can be edited and added to the bored
    pub fn create_draft(&mut self, dimensions: Coordinate) -> Result<(), BoredError> {
        if dimensions.within(&self.dimensions) {
            self.draft_notice = Some(Notice::create(dimensions));
            return Ok(());
        }
        Err(BoredError::NoticeOutOfBounds)
    }

    /// check the content will fit in the notice and update content if so
    pub fn edit_draft(&mut self, content: &str) -> Result<(), BoredError> {
        if let Some(mut notice) = self.draft_notice.clone() {
            notice.write(content)?;
            self.draft_notice = Some(notice);
        }
        Ok(())
    }

    /// Removes any notices that are entirely occluded by notices above them
    pub fn prune_non_visible(&mut self) -> Result<(), BoredError> {
        if self.protocol_version.get_version() < 1 {
            return Err(BoredError::MethodNotInProtocol);
        }
        let whats_on_the_bored = WhatsOnTheBored::create(&self);
        // flaten whats_on_the_bored into 1 dimension
        let mut whats_on_the_bored_1d = whats_on_the_bored.get_1d();
        let notices_indexes: Vec<usize> = self
            .notices
            .iter()
            .enumerate()
            .map(|(notices_index, _)| notices_index)
            .collect();
        whats_on_the_bored_1d.sort();
        whats_on_the_bored_1d.dedup();
        let whats_on_the_bored_1d: Vec<_> = whats_on_the_bored_1d
            .iter()
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect();
        for notice_index in &notices_indexes {
            let mut remove = true;
            for on_bored in &whats_on_the_bored_1d {
                if notice_index == on_bored {
                    remove = false;
                }
            }
            if remove {
                self.notices.remove(*notice_index);
            }
        }
        Ok(())
    }

    /// Attempts to get the index of the first notice (most upward and leftward) in that direction
    /// Diagram shows order of coordinates checked 1 - 8 when going up from the notice
    /// the first notice found in rhia order is the one that will be returned
    ///   ----- - edge of bored
    ///  | 8634
    ///  | 7512   
    ///  |   XX - border of notice   
    pub fn get_cardinal_notice(
        &self,
        current_notice: usize,
        direction: Direction,
    ) -> Option<usize> {
        let notice = &self.notices[current_notice];
        let whats_on_the_bored = WhatsOnTheBored::create(&self);
        // let coordinate = match direction {
        //     Direction::Up => {
        //         let start_point = notice.get_top_left()

        //     }
        //     Direction::Right => notice.get_top_left().add(&Coordinate {
        //         x: notice.get_dimensions().x,
        //         y: 0,
        //     }),
        //     Direction::Down => notice.get_top_left().add(&notice.get_dimensions()),
        //     Direction::Left => notice.get_top_left().add(&Coordinate {
        //         x: 0,
        //         y: notice.get_dimensions().y,
        //     }),
        // };
        None
    }
}

#[derive(Clone, Copy)]
enum ConnectionType {
    Local,
    Antnet,
}

/// An client implementing the methods of the Bored protocol via an autonomi client for storage
struct BoredClient {
    connection_type: ConnectionType,
    client: Client,
    current_bored: Option<Bored>,
    scratchpad_counter: Option<u64>,
    bored_address: BoredAddress,
}

impl BoredClient {
    pub async fn init() -> Result<BoredClient, BoredError> {
        let connection_type = ConnectionType::Local;
        let client = match Client::init_local().await {
            Err(_) => return Err(BoredError::ClientConnectionError),
            Ok(client) => client,
        };
        Ok(BoredClient {
            connection_type,
            client,
            current_bored: None,
            scratchpad_counter: None,
            bored_address: BoredAddress::new(),
        })
    }

    /// Creates a new instance of a board and places in current_bored and attempts to create
    /// a scratchpad containing it at the BoredAddress
    pub async fn create_bored(&mut self, name: &str, private_key: &str) -> Result<(), BoredError> {
        let bored = Bored::create(name);
        let serialized_bored = serde_json::to_vec(&bored)?;
        let content = Bytes::from(serialized_bored);
        let wallet = match get_funded_wallet(self.connection_type, private_key).await {
            Ok(wallet) => wallet,
            Err(_) => return Err(BoredError::FailedToGetWallet),
        };
        let payment_option = PaymentOption::from(&wallet);
        let (..) = self
            .client
            .scratchpad_create(&self.bored_address.get_key(), 27, &content, payment_option)
            .await?;
        // wait for the scratchpad to be replicated
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        self.refresh_bored().await?; // local bored is update by downloading to make sure in sync
        Ok(())
    }

    /// Downloads an existing bored
    pub async fn retrieve_bored(&mut self) -> Result<(Bored, u64), BoredError> {
        let got = self
            .client
            .scratchpad_get_from_public_key(&self.bored_address.get_public_key())
            .await?;
        let content = match got.decrypt_data(&self.bored_address.get_key()) {
            Ok(content) => content,
            Err(e) => return Err(BoredError::DecryptionError(format!("{e}"))),
        };
        let serialized_bored = match String::from_utf8(content.to_vec()) {
            Err(_) => return Err(BoredError::BinaryError),
            Ok(serialzed_bored) => serialzed_bored,
        };
        let bored = match serde_json::from_str(&serialized_bored) {
            Err(_) => return Err(BoredError::JSONError),
            Ok(bored) => bored,
        };
        // probably should do check that it is valud bored protcol
        // if not reset as new bored with same name to deal with scratchpad hijacking
        Ok((bored, got.counter()))
    }

    /// Refresh the current bored
    pub async fn refresh_bored(&mut self) -> Result<(), BoredError> {
        let (bored, scratchpad_counter) = self.retrieve_bored().await?;
        self.current_bored = Some(bored);
        self.scratchpad_counter = Some(scratchpad_counter);
        Ok(())
    }

    /// Updates the current bored, if there is a newer version of the bored on th antnet it
    /// returns it within the error so that the local version can be updated
    pub async fn update_bored(&mut self, updated_bored: &Bored) -> Result<(), BoredError> {
        if self.scratchpad_counter.is_none() {
            return Err(BoredError::BoredNotYetDownloaded);
        }
        let (bored, scratchpad_counter) = self.retrieve_bored().await?;
        if scratchpad_counter > self.scratchpad_counter.unwrap() {
            return Err(BoredError::MoreRecentVersionExists(
                bored,
                scratchpad_counter,
            ));
        }
        let serialized_bored = serde_json::to_vec(&updated_bored)?;
        let content = Bytes::from(serialized_bored);
        self.client
            .scratchpad_update(&self.bored_address.get_key(), 27, &content)
            .await?;
        // wait for the scratchpad to be replicated
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        self.refresh_bored().await?; // local bored is update by downloading to make sure in sync
        Ok(())
    }

    /// Returns the current bored or error if not yet populated
    pub fn get_current_bored(&self) -> Result<Bored, BoredError> {
        let Some(bored) = self.current_bored.clone() else {
            return Err(BoredError::BoredNotYetDownloaded);
        };
        Ok(bored)
    }
}

async fn get_funded_wallet(
    connection_type: ConnectionType,
    private_key: &str,
) -> Result<Wallet, Box<dyn Error>> {
    let (local, private_key) = match connection_type {
        ConnectionType::Antnet => (false, private_key),
        ConnectionType::Local => (
            true,
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        ),
    };
    let network = Network::new(local)?;
    let wallet = Wallet::new_from_private_key(network, private_key)?;
    Ok(wallet)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_protcol_version() {
        let protocol_version = ProtocolVersion::check(CONTENT_TYPE_PROTOCOL_BASE);
        assert_eq!(protocol_version, Ok(ProtocolVersion(1)));
        let protocol_version = ProtocolVersion::check(CONTENT_TYPE_PROTOCOL_BASE - 1);
        assert_eq!(
            protocol_version,
            Err(BoredError::InvalidProtocolVersion(
                CONTENT_TYPE_PROTOCOL_BASE - 1
            ))
        );
        let protocol_version = ProtocolVersion::check(CONTENT_TYPE_PROTOCOL_BASE + 99999);
        assert_eq!(
            protocol_version,
            Err(BoredError::InvalidProtocolVersion(
                CONTENT_TYPE_PROTOCOL_BASE + 99999
            ))
        );
    }

    #[test]
    fn test_coordinate_within() {
        let coordianate = Coordinate { x: 1, y: 9 };
        assert!(!coordianate.within(&Coordinate { x: 0, y: 0 }));
        assert!(coordianate.within(&Coordinate { x: 1, y: 9 }));
        assert!(!coordianate.within(&Coordinate { x: 0, y: 10 }));
        assert!(coordianate.within(&Coordinate { x: 1, y: 10 }));
    }

    #[test]
    fn test_coordinate_add() {
        let coordianate = Coordinate { x: 0, y: 999 };
        assert_eq!(
            coordianate.add(&Coordinate { x: 1, y: 1 }),
            Coordinate { x: 1, y: 1000 }
        );
    }

    #[test]
    fn test_bored_add() {
        let mut bored = Bored::create("");
        let notice = Notice::new();
        assert!(bored.add(notice, Coordinate { x: 0, y: 0 }).is_ok());
        let notice = Notice::new();
        assert!(bored.add(notice, Coordinate { x: 999, y: 999 }).is_err());
    }

    #[test]
    fn test_prune_non_visible() -> Result<(), BoredError> {
        let mut bored = Bored::create("");
        let mut notice = Notice::new();
        notice.write("hello")?;
        bored.add(notice, Coordinate { x: 0, y: 0 }).unwrap();
        notice = Notice::new();
        bored.add(notice, Coordinate { x: 0, y: 0 }).unwrap();
        notice = Notice::new();
        notice.write("world")?;
        bored.add(notice, Coordinate { x: 1, y: 0 }).unwrap();
        assert_eq!(bored.notices[0].get_content(), "hello");
        assert_eq!(bored.notices.len(), 3);
        bored.prune_non_visible()?;
        assert_eq!(bored.notices[0].get_content(), "");
        assert_eq!(bored.notices.len(), 2);
        assert_eq!(bored.notices[1].get_content(), "world");
        Ok(())
    }

    #[test]
    fn test_bored_address_display() {
        let bored_address = BoredAddress::new();
        let BoredAddress::ScratchpadKey(ref scratchpad_key) = bored_address;
        assert_eq!(
            format!("bored://{}", scratchpad_key.to_hex()),
            format!("{}", bored_address)
        );
    }

    #[test]
    fn test_edit_draft() {
        let mut bored = Bored::create("");
        bored.create_draft(Coordinate { x: 0, y: 0 }).unwrap();
        assert_eq!(bored.edit_draft("I am BORED"), Err(BoredError::TooMuchText));
        bored.create_draft(Coordinate { x: 7, y: 4 }).unwrap();
        assert_eq!(
            bored.edit_draft("I am BORED!"),
            Err(BoredError::TooMuchText)
        );
        bored.create_draft(Coordinate { x: 7, y: 4 }).unwrap();
        assert_eq!(bored.edit_draft("I am BORED"), Ok(()));
        assert_eq!(
            bored.draft_notice.as_ref().unwrap().get_content(),
            "I am BORED"
        );
        bored.create_draft(Coordinate { x: 7, y: 4 }).unwrap();
        assert_eq!(
            bored.edit_draft("I\nam\nBORED"),
            Err(BoredError::TooMuchText)
        );
        bored.create_draft(Coordinate { x: 7, y: 6 }).unwrap();
        assert_eq!(bored.edit_draft("I\nam\nBORED"), Ok(()));
        let draft_notice = bored.draft_notice.clone();
        assert_eq!(draft_notice.as_ref().unwrap().get_content(), "I\nam\nBORED");
        bored.create_draft(Coordinate { x: 7, y: 4 }).unwrap();
        assert_eq!(
            bored.edit_draft("I am [BORED](NOT)!"),
            Err(BoredError::TooMuchText)
        );
        assert_eq!(bored.edit_draft("I am [BORED](NOT)"), Ok(()));
        assert_eq!(
            bored.draft_notice.as_ref().unwrap().get_content(),
            "I am [BORED](NOT)"
        );
    }

    #[test]
    fn test_get_cardinal_notice() {
        let mut bored = Bored::create("");
        let notice = Notice::create(Coordinate { x: 10, y: 10 });
        bored.add(notice, Coordinate { x: 0, y: 0 }).unwrap();
        let mut visible = WhatsOnTheBored::create(&bored);
        eprintln!("{}", visible);
        visible.rotate_horizontally();
        eprintln!("{}", visible);
        visible.flip_vertically();
        eprintln!("{}", visible);
        visible.rotate_horizontally();
        eprintln!("{}", visible);
    }

    #[tokio::test]
    #[ignore]
    async fn test_create_bored() -> Result<(), BoredError> {
        let mut bored_client = BoredClient::init().await?;
        bored_client.create_bored("", "").await?;
        let bored = bored_client.current_bored.as_ref().unwrap().clone();
        bored_client.refresh_bored().await?;
        assert_eq!(bored_client.current_bored.unwrap(), bored);
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_update_bored() -> Result<(), BoredError> {
        let mut bored_client = BoredClient::init().await?;
        bored_client.create_bored("I am BORED", "").await?;
        let scrachpad_counter = bored_client.scratchpad_counter.unwrap();
        let mut bored = bored_client.current_bored.as_ref().unwrap().clone();
        let mut notice = Notice::new();
        notice.write("We are bored")?;
        bored.add(notice, Coordinate { x: 1, y: 1 })?;
        bored_client.update_bored(&bored).await?;
        // wait for the scratchpad to be replicated
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let (antnet_bored, antnet_counter) = bored_client.retrieve_bored().await?;
        assert_eq!(
            bored_client.scratchpad_counter.unwrap(),
            scrachpad_counter + 1,
        );
        assert_eq!(bored_client.scratchpad_counter.unwrap(), antnet_counter);
        assert_eq!(antnet_bored, bored);
        assert_eq!(bored_client.current_bored.unwrap(), bored);
        Ok(())
    }
}
