use bored::bored_client::{BoredClient, ConnectionType};
use bored::{Bored, BoredAddress, BoredError, Coordinate};

#[derive(Clone)]
pub enum View {
    ErrorView(BoredError),
    BoredView(Option<BoredAddress>),
    NoticeView { hyperlinks_index: Option<usize> },
    DraftView(DraftMode),
    CreateView(CreateMode),
    GoToView(GoToMode),
    Quitting,
}

#[derive(Clone)]
pub enum CreateMode {
    Name,
    PrivateKey,
}

#[derive(Clone)]
pub enum DraftMode {
    Content,
    Hyperlink(HyperlinkMode),
    Position,
}

#[derive(Clone)]
pub enum HyperlinkMode {
    Text,
    URL,
}

#[derive(Clone)]
pub enum GoToMode {
    Directory,
    PasteAddress,
}

/// The directory of boreds...list of bored the user has saved for future reference
pub struct Directory {
    bored_addresses: Vec<BoredAddress>,
    home_bored: usize, // indicates which bored is the home bored
}
impl Directory {
    fn new() -> Directory {
        Directory {
            bored_addresses: vec![],
            home_bored: 0,
        }
    }

    fn add(&mut self, bored_address: BoredAddress) {
        self.bored_addresses.push(bored_address);
    }

    fn set_home(&mut self, home_bored: usize) {
        self.home_bored = home_bored
    }
}

/// History of boreds surfed in current session
pub struct History {
    boreds: Vec<Bored>,
    current_position: usize,
}
impl History {
    fn new() -> History {
        History {
            boreds: vec![],
            current_position: 0,
        }
    }
}

pub struct App {
    pub client: Option<BoredClient>,
    pub directory: Directory,
    pub history: History,
    pub current_view: View,
    pub previous_view: View,
    pub selected_notice: Option<usize>,
}
impl App {
    pub async fn init() -> Result<App, BoredError> {
        Ok(App {
            client: BoredClient::init(ConnectionType::Local).await.ok(),
            directory: Directory::new(),
            history: History::new(),
            current_view: View::BoredView(None),
            previous_view: View::BoredView(None),
            selected_notice: None,
        })
    }

    pub fn display_error(&mut self, bored_error: BoredError) {
        self.current_view = View::ErrorView(bored_error);
    }

    pub fn goto(&mut self) {
        self.current_view = View::GoToView(GoToMode::PasteAddress)
    }

    pub fn goto_view_toggle(&mut self) {
        match &self.current_view {
            View::GoToView(goto_mode) => {
                self.current_view = View::GoToView(match goto_mode {
                    GoToMode::Directory => GoToMode::PasteAddress,
                    GoToMode::PasteAddress => GoToMode::PasteAddress,
                })
            }
            _ => (),
        }
    }

    pub fn goto_bored(&mut self, bored_address: BoredAddress) {
        self.current_view = View::BoredView(Some(bored_address));
    }

    pub fn view_notice(&mut self) {
        self.current_view = View::NoticeView {
            hyperlinks_index: None,
        };
    }

    pub fn create_bored(&mut self) {
        self.current_view = View::CreateView(CreateMode::Name);
    }

    /// returns the current bored of the cliet if both exist otherwise None
    pub fn get_current_bored(&self) -> Option<Bored> {
        if let Some(client) = &self.client {
            if let Ok(bored) = client.get_current_bored() {
                return Some(bored);
            }
        }
        return None;
    }

    pub fn create_view_toggle(&mut self) {
        match &self.current_view {
            View::CreateView(create_mode) => {
                self.current_view = View::CreateView(match create_mode {
                    CreateMode::Name => CreateMode::PrivateKey,
                    CreateMode::PrivateKey => CreateMode::Name,
                })
            }
            _ => (),
        }
    }

    pub async fn create_bored_on_network(
        &mut self,
        name: &str,
        private_key: &str,
    ) -> Result<Bored, BoredError> {
        let Some(ref mut client) = self.client else {
            return Err(BoredError::ClientConnectionError);
        };
        client
            .create_bored(name, Coordinate { x: 120, y: 40 }, private_key)
            .await?;
        let bored = client.get_current_bored()?;
        self.current_view = View::BoredView(Some(client.get_bored_address()?));
        Ok(bored)
    }

    pub fn create_draft(&mut self, dimensions: Coordinate) -> Result<(), BoredError> {
        let Some(ref mut client) = self.client else {
            return Err(BoredError::ClientConnectionError);
        };
        client.create_draft(dimensions)?;
        self.current_view = View::DraftView(DraftMode::Content);
        Ok(())
    }

    pub fn edit_draft(&mut self, content: &str) -> Result<(), BoredError> {
        let Some(ref mut client) = self.client else {
            return Err(BoredError::ClientConnectionError);
        };
        match client.edit_draft(content) {
            Err(bored_error) => match bored_error {
                // if to much text does nothing so user can't type more
                BoredError::TooMuchText => return Ok(()),
                _ => return Err(bored_error),
            },
            Ok(_) => Ok(()),
        }
    }

    pub fn create_hyperlink(&mut self) {
        self.current_view = View::DraftView(DraftMode::Hyperlink(HyperlinkMode::Text));
    }
}
