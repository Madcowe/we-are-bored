use bored::bored_client::{BoredClient, ConnectionType};
use bored::{Bored, BoredAddress, BoredError};
use ratatui::backend::ClearType;

pub enum CurrentView {
    ErrorView(BoredError),
    BoredView(Option<BoredAddress>),
    NoticeView,
    DraftView(DraftMode),
    CreateView(CreateMode),
    GoToView(GoToMode),
}

pub enum CreateMode {
    Name,
    PrivateKey,
}

pub enum DraftMode {
    Content,
    Hyperlink,
}

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
    bored_addresses: Vec<BoredAddress>,
    current_position: usize,
}
impl History {
    fn new() -> History {
        History {
            bored_addresses: vec![],
            current_position: 0,
        }
    }
}

pub struct App {
    pub client: BoredClient,
    pub connection_type: ConnectionType,
    pub directory: Directory,
    pub history: History,
    pub current_view: CurrentView,
}
impl App {
    pub async fn init() -> Result<App, BoredError> {
        Ok(App {
            client: BoredClient::init().await?,
            connection_type: ConnectionType::Local,
            directory: Directory::new(),
            history: History::new(),
            current_view: CurrentView::BoredView(None),
        })
    }

    pub fn display_error(&mut self, bored_error: BoredError) {
        self.current_view = CurrentView::ErrorView(bored_error);
    }

    pub fn goto(&mut self, bored_address: BoredAddress) {
        self.current_view = CurrentView::BoredView(Some(bored_address));
    }

    pub async fn create(&mut self, name: &str, private_key: &str) -> Result<Bored, BoredError> {
        self.client.create_bored(name, private_key).await?;
        let bored = self.client.get_current_bored()?;
        self.current_view = CurrentView::BoredView(Some(self.client.get_bored_address()?));
        Ok(bored)
    }
}
