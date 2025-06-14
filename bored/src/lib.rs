use autonomi::SecretKey;
use notice::{Display, Notice, NoticeHyperlinkMap};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::{self};
use std::ops::Add;

pub mod bored_client;
pub mod notice;

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
#[derive(Debug, thiserror::Error, PartialEq, Clone)]
pub enum BoredError {
    #[error("Version of protocol {0} is not know to exist by this implementation of bored")]
    InvalidProtocolVersion(u64),
    #[error("Method is not in this version of the protocol")]
    MethodNotInProtocol,
    #[error(
        "Cannot place notice outside of board, attempted to place notice with max bounds of {1} in bored with max bounds of {0}"
    )]
    NoticeOutOfBounds(Coordinate, Coordinate),
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
    #[error(
        "More recent version of bored exists so cannot add notice, the bored has now been refreshed so please try again"
    )]
    MoreRecentVersionExists(Bored, u64),
    #[error("Hyperlink url is too long at max is {}", notice::MAX_URL_LENGTH)]
    URLTooLong,
    #[error("Error performing regular expression search")]
    RegexError,
    #[error("No notice in that directions")]
    NoNotice,
    #[error("{0}")]
    QuoteError(String),
    #[error("No bored has been set yet")]
    NoBored,
    #[error("Cannot parse as bored URL")]
    NotBoredURL,
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

impl From<autonomi::client::quote::CostError> for BoredError {
    fn from(e: autonomi::client::quote::CostError) -> Self {
        let message = format!("{e}");
        BoredError::QuoteError(message)
    }
}

/// The address of a bored, currently this can only be autonomi::private key that has been
/// used to create a scratchpad with a bored stored in it.
/// Hence this means anyone who has the address can update the board which probalby won't
/// be sensible in a long term project but this is an experiment so starting with the
/// most basic level of a human trust network seems appropriate, you share it you bare it!
#[derive(Clone, Debug, PartialEq)]
pub enum BoredAddress {
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

    /// Tries to create bored URL from string, will remove protocol part of URL if it exists
    /// attempt to create with that text, fails if it doesn;t make valid secret key
    /// this doesn't neccsiarily imply it is an exist bored address
    pub fn from_string(mut s: String) -> Result<Self, BoredError> {
        if s.len() == 72 {
            if &s[0..8] == "bored://" {
                s = s[8..s.len()].to_string();
            }
        }
        let key = match SecretKey::from_hex(&s) {
            Ok(key) => key,
            Err(_) => return Err(BoredError::NotBoredURL),
        };
        Ok(BoredAddress::ScratchpadKey(key))
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
    pub x: u16,
    pub y: u16,
}
impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x: {} y: {}", self.x, self.y)
    }
}

impl Coordinate {
    /// returns true if self entirely contained between (0,0) and other
    pub fn within(&self, other: &Self) -> bool {
        if self.x <= other.x && self.y <= other.y {
            return true;
        }
        false
    }

    pub fn add(&self, other: &Self) -> Coordinate {
        Coordinate {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    /// will not subtract below zero
    pub fn subtact(&self, other: &Self) -> Coordinate {
        let x = if self.x >= other.x { other.x } else { 0 };
        let y = if self.y >= other.y { other.y } else { 0 };
        Coordinate {
            x: self.x - x,
            y: self.y - y,
        }
    }

    /// adds a possibley negative i32 tuple
    pub fn add_i32_tuple(&self, other: (i32, i32)) -> Coordinate {
        let x = if self.x as i32 + other.0 >= 0 {
            self.x as i32 + other.0
        } else {
            0
        } as u16;
        let y = if self.y as i32 + other.1 >= 0 {
            self.y as i32 + other.1
        } else {
            0
        } as u16;
        Coordinate { x, y }
    }
}

/// Indicate direction of movement
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// a 2d vector of option<(usize, usize)> reprentiny the index of the notice,
// and hyperlink of the top most noteset as per whats on the bored
pub struct BoredHyperlinkMap {
    visible: Vec<Vec<Option<(usize, usize)>>>,
}
impl Iterator for BoredHyperlinkMap {
    type Item = Vec<Option<(usize, usize)>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.visible.iter().next().cloned()
    }
}
impl fmt::Display for BoredHyperlinkMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut display = String::new();
        for row in &self.visible {
            for coordinate in row {
                let text = match coordinate {
                    None => "*",
                    // just display hypelink index as unlikely to be more then 1 char
                    Some(notice_index) => &notice_index.1.to_string(),
                };
                display.push_str(text);
            }
            display.push_str("\n");
        }
        write!(f, "{}", display)
    }
}

impl BoredHyperlinkMap {
    pub fn create(bored: &Bored) -> Result<BoredHyperlinkMap, BoredError> {
        let mut visible = vec![vec![None; bored.dimensions.x.into()]; bored.dimensions.y.into()];
        for (notices_index, notice) in bored.notices.iter().enumerate() {
            let notice_hyperlink_map = NoticeHyperlinkMap::create(&notice)?;
            // set all charter in notice none so as to occlude any previous notices hyperlinks
            for y in notice.get_top_left().y..notice.get_top_left().y.add(notice.get_dimensions().y)
            {
                for x in
                    notice.get_top_left().x..notice.get_top_left().x.add(notice.get_dimensions().x)
                {
                    visible[y as usize][x as usize] = None;
                }
            }
            let notice_hyperlink_map = notice_hyperlink_map.get_map();
            let (mut map_x, mut map_y) = (0, 0);
            // +/- 1 to account for border
            for y in notice.get_top_left().y + 1
                ..(notice.get_top_left().y.add(notice.get_dimensions().y)) - 1
            {
                for x in notice.get_top_left().x + 1
                    ..(notice.get_top_left().x.add(notice.get_dimensions().x)) - 1
                {
                    if let Some(hyperlink_index) = notice_hyperlink_map[map_y][map_x] {
                        visible[y as usize][x as usize] = Some((notices_index, hyperlink_index));
                    }
                    map_x += 1;
                }
                map_x = 0;
                map_y += 1;
            }
        }
        Ok(BoredHyperlinkMap { visible })
    }

    pub fn get_map(&self) -> Vec<Vec<Option<(usize, usize)>>> {
        self.visible.clone()
    }
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

    fn get_x_len(&self) -> usize {
        self.visible[0].len()
    }

    fn get_y_len(&self) -> usize {
        self.visible.len()
    }

    fn rotate_horizontally(&mut self) {
        let mut visible: Vec<Vec<Option<usize>>> =
            vec![vec![None; self.visible.len()]; self.visible[0].len()];
        for (y, row) in self.visible.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                visible[x][y] = *cell;
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

    /// get value at coordiante
    fn get_vaule_at_coordinate(&self, coordinate: Coordinate) -> Option<usize> {
        self.visible[coordinate.y as usize][coordinate.x as usize]
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
}

// only methods dealing with the interal items of bored need to perform the protocol check
// so as to avoid calling methods on items that don't exist in bored that are currently
// using an older version of the protocol
impl Bored {
    /// Creates a new board using the most recent protocol version
    pub fn create(name: &str, dimensions: Coordinate) -> Bored {
        Bored {
            protocol_version: PROTOCOL_VERSIONS[PROTOCOL_VERSIONS.len() - 1],
            name: name.to_string(),
            dimensions,
            notices: Vec::new(),
        }
    }

    /// Add a notice to the board in the specified position returns an error if out of bounds
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

    pub fn get_notices(&self) -> Vec<Notice> {
        self.notices.clone()
    }

    pub fn get_name(&self) -> &str {
        &self.name
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

    pub fn get_dimensions(&self) -> Coordinate {
        self.dimensions
    }

    /// Get all the coordiantes to check going up from a notice
    fn get_up_coordinates(&self, notice: &Notice) -> [Vec<Coordinate>; 2] {
        let mut coordinate_sets: [Vec<Coordinate>; 2] = [vec![], vec![]];
        // above set
        if notice.get_top_left().y > 0 {
            let mut coordinates = vec![];
            for y in (0..notice.get_top_left().y).rev() {
                for x in
                    notice.get_top_left().x..notice.get_top_left().x + notice.get_dimensions().x
                {
                    coordinates.push(Coordinate { x, y });
                }
            }
            coordinate_sets[0] = coordinates;
            // to left of above set
            if notice.get_top_left().x > 0 {
                let mut coordinates = vec![];
                for x in (0..notice.get_top_left().x).rev() {
                    for y in (0..notice.get_top_left().y).rev() {
                        coordinates.push(Coordinate { x, y });
                    }
                }
                coordinate_sets[1] = coordinates;
            }
        }
        coordinate_sets
    }

    /// Get all the coordiantes to check going right from a notice
    fn get_right_coordinates(&self, notice: &Notice) -> [Vec<Coordinate>; 2] {
        let mut coordinate_sets: [Vec<Coordinate>; 2] = [vec![], vec![]];
        let top_right = Coordinate {
            x: notice.get_top_left().x + notice.get_dimensions().x,
            y: notice.get_top_left().y,
        };
        // right set
        if top_right.x < self.dimensions.x {
            let mut coordinates = vec![];
            for y in top_right.y..top_right.y + notice.get_dimensions().y {
                for x in top_right.x..self.dimensions.x {
                    coordinates.push(Coordinate { x, y });
                }
            }
            coordinate_sets[0] = coordinates;
            // above right set
            if top_right.y > 0 {
                let mut coordinates = vec![];
                for y in (0..top_right.y).rev() {
                    for x in top_right.x..self.dimensions.x {
                        coordinates.push(Coordinate { x, y });
                    }
                }
                coordinate_sets[1] = coordinates;
            }
        }
        coordinate_sets
    }

    /// Get all the coordiantes to check going down from a notice
    fn get_down_coordinates(&self, notice: &Notice) -> [Vec<Coordinate>; 2] {
        let mut coordinate_sets: [Vec<Coordinate>; 2] = [vec![], vec![]];
        let bottom_left = Coordinate {
            x: notice.get_top_left().x,
            y: notice.get_top_left().y + notice.get_dimensions().y,
        };
        // down set
        if bottom_left.y < self.dimensions.y {
            let mut coordinates = vec![];
            for x in bottom_left.x..bottom_left.x + notice.get_dimensions().x {
                for y in bottom_left.y..self.dimensions.y {
                    coordinates.push(Coordinate { x, y });
                }
            }
            coordinate_sets[0] = coordinates;
            // right of down set
            if bottom_left.x + notice.get_dimensions().x < self.dimensions.x {
                let mut coordinates = vec![];
                for y in (bottom_left.y..self.dimensions.y).rev() {
                    for x in bottom_left.x + notice.get_dimensions().x..self.dimensions.x {
                        coordinates.push(Coordinate { x, y });
                    }
                }
                coordinate_sets[1] = coordinates;
            }
        }
        coordinate_sets
    }

    /// Get all the coordiantes to check going left from a notice
    fn get_left_coordinates(&self, notice: &Notice) -> [Vec<Coordinate>; 2] {
        let mut coordinate_sets: [Vec<Coordinate>; 2] = [vec![], vec![]];
        // left set
        if notice.get_top_left().x > 0 {
            let mut coordinates = vec![];
            for y in notice.get_top_left().y..notice.get_top_left().y + notice.get_dimensions().y {
                for x in (0..notice.get_top_left().x).rev() {
                    coordinates.push(Coordinate { x, y });
                }
            }
            coordinate_sets[0] = coordinates;
            // below left set
            if notice.get_top_left().y + notice.get_dimensions().y < self.dimensions.y {
                let mut coordinates = vec![];
                for y in notice.get_top_left().y + notice.get_dimensions().y..self.dimensions.y {
                    for x in 0..notice.get_top_left().x {
                        coordinates.push(Coordinate { x, y });
                    }
                }
                coordinate_sets[1] = coordinates;
            }
        }
        coordinate_sets
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
        let visible = WhatsOnTheBored::create(&self);
        let to_check = match direction {
            Direction::Up => self.get_up_coordinates(&notice),
            Direction::Right => self.get_right_coordinates(&notice),
            Direction::Down => self.get_down_coordinates(&notice),
            Direction::Left => self.get_left_coordinates(&notice),
        };
        for coordinate_set in to_check {
            for coordinate in coordinate_set {
                if let Some(notice_index) = visible.get_vaule_at_coordinate(coordinate) {
                    return Some(notice_index);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {

    use std::error::Error;

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
        let mut bored = Bored::create("", Coordinate { x: 120, y: 40 });
        let notice = Notice::new();
        assert!(bored.add(notice, Coordinate { x: 0, y: 0 }).is_ok());
        assert_eq!(bored.notices.len(), 1);
        assert_eq!(bored.notices[0], Notice::new());
        let notice = Notice::new();
        assert!(bored.add(notice, Coordinate { x: 999, y: 999 }).is_err());
    }

    #[test]
    fn test_prune_non_visible() -> Result<(), BoredError> {
        let mut bored = Bored::create("", Coordinate { x: 120, y: 40 });
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
    fn test_get_cardinal_notice() -> Result<(), BoredError> {
        let mut bored = Bored::create("", Coordinate { x: 120, y: 40 });
        let notice = Notice::create(Coordinate { x: 10, y: 20 });
        bored.add(notice, Coordinate { x: 50, y: 10 }).unwrap();
        assert_eq!(bored.get_cardinal_notice(0, Direction::Left), None);
        let notice = Notice::create(Coordinate { x: 10, y: 10 });
        bored.add(notice, Coordinate { x: 0, y: 0 })?;
        assert_eq!(bored.get_cardinal_notice(0, Direction::Up), Some(1));
        let notice = Notice::create(Coordinate { x: 10, y: 10 });
        bored.add(notice, Coordinate { x: 59, y: 0 })?;
        assert_eq!(bored.get_cardinal_notice(0, Direction::Up), Some(2));
        assert_eq!(bored.get_cardinal_notice(0, Direction::Right), Some(2));
        let notice = Notice::create(Coordinate { x: 10, y: 10 });
        bored.add(notice, Coordinate { x: 100, y: 25 })?;
        assert_eq!(bored.get_cardinal_notice(0, Direction::Right), Some(3));
        assert_eq!(bored.get_cardinal_notice(0, Direction::Down), Some(3));
        let notice = Notice::create(Coordinate { x: 10, y: 10 });
        bored.add(notice, Coordinate { x: 45, y: 29 })?;
        assert_eq!(bored.get_cardinal_notice(0, Direction::Down), Some(4));
        assert_eq!(bored.get_cardinal_notice(0, Direction::Left), Some(4));
        let notice = Notice::create(Coordinate { x: 10, y: 10 });
        bored.add(notice, Coordinate { x: 1, y: 5 })?;
        assert_eq!(bored.get_cardinal_notice(0, Direction::Left), Some(5));
        assert_eq!(bored.get_cardinal_notice(0, Direction::Up), Some(2));
        let visible = WhatsOnTheBored::create(&bored);
        eprintln!("{}", visible);
        Ok(())
    }

    #[test]
    fn test_bored_address_from_string() {
        let bored_address = BoredAddress::from_string("".to_string());
        assert_eq!(bored_address, Err(BoredError::NotBoredURL));
        let string =
            "bored://2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330".to_string();
        let bored_address = BoredAddress::from_string(string);
        assert_eq!(
            bored_address.unwrap().get_key().to_hex(),
            "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330"
        );
        let string = "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330".to_string();
        let bored_address = BoredAddress::from_string(string);
        assert_eq!(
            bored_address.unwrap().get_key().to_hex(),
            "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330"
        );
    }

    #[test]
    fn test_bored_hyperlink_map() -> Result<(), BoredError> {
        let mut bored = Bored::create("Hello", Coordinate { x: 40, y: 20 });
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write(
            "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
        )?;
        bored.add(notice, Coordinate { x: 5, y: 3 })?;
        let mut notice = Notice::create(Coordinate { x: 10, y: 13 });
        notice.write(
            "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
        )?;
        bored.add(notice, Coordinate { x: 10, y: 5 })?;
        let mut notice = Notice::create(Coordinate { x: 10, y: 13 });
        notice.write(
            "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
        )?;
        bored.add(notice, Coordinate { x: 14, y: 7 })?;
        let bored_hyperlink_map = BoredHyperlinkMap::create(&bored)?;
        eprintln!("{bored_hyperlink_map}");
        let expected_output = r#"****************************************
****************************************
****************************************
****************************************
*************0000*11111*****************
****************************************
******************0*********************
******3333*000**************************
***********1**********0*****************
***************000*1111*****************
***********222*1************************
****************************************
***************2222*********************
***********33***************************
***********333******333*****************
***************33***********************
***************333333*******************
****************************************
****************************************
****************************************
"#;
        assert_eq!(expected_output, format!("{}", bored_hyperlink_map));
        Ok(())
    }

    #[test]
    fn test_add_i32_tuple() {
        let mut coordinate = Coordinate { x: 0, y: 0 };
        coordinate = coordinate.add_i32_tuple((1, 1));
        assert_eq!(coordinate, Coordinate { x: 1, y: 1 });
        coordinate = coordinate.add_i32_tuple((-1, -1));
        assert_eq!(coordinate, Coordinate { x: 0, y: 0 });
        coordinate = coordinate.add_i32_tuple((-1, -1));
        assert_eq!(coordinate, Coordinate { x: 0, y: 0 });
        coordinate = coordinate.add_i32_tuple((1, 0));
        assert_eq!(coordinate, Coordinate { x: 1, y: 0 });
        coordinate = coordinate.add_i32_tuple((0, 1));
        assert_eq!(coordinate, Coordinate { x: 1, y: 1 });
    }
}
