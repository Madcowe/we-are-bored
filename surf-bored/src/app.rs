use bored::bored_client::{BoredClient, ConnectionType};
use bored::{Bored, BoredAddress, BoredError, Coordinate, Direction};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum SurfBoredError {
    #[error("{0}")]
    BoredError(BoredError),
    #[error("Could not read directory file so directory is empty")]
    DirectoryFileReadError,
    #[error("Directory not save to disk as could not write to file")]
    DirectoryFileWriteError,
    #[error("could not serialize directory file so directory is empty")]
    DirectorySerialzationError,
    #[error("could not derserialize directory file so directory is empty")]
    DirectoryDeserialzationError,
}

impl From<BoredError> for SurfBoredError {
    fn from(e: BoredError) -> Self {
        Self::BoredError(e)
    }
}

// impl<T: Error + 'static> From<T> for SurfBoredError {
//     fn from(e: T) -> Self {
//         Self::Other(Box::new(e))
//     }
// }

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Listing {
    name: String,
    bored_address: String,
}

/// The directory of boreds...list of bored the user has saved for future reference
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Directory {
    bored_addresses: Vec<Listing>,
    home_bored: usize, // indicates which bored is the home bored
}
impl Directory {
    fn new() -> Directory {
        Directory {
            bored_addresses: vec![],
            home_bored: 0,
        }
    }

    fn load_file(path: &str) -> Result<Directory, SurfBoredError> {
        if let Ok(directory_string) = fs::read_to_string(path) {
            if let Ok(directory) = toml::from_str(&directory_string) {
                return Ok(directory);
            } else {
                return Err(SurfBoredError::DirectoryDeserialzationError);
            }
        } else {
            return Err(SurfBoredError::DirectoryFileWriteError);
        }
    }

    // need to sort out error handling and remove unwraps
    fn add(&mut self, listing: Listing, path: &str) -> Result<(), SurfBoredError> {
        self.bored_addresses.push(listing);
        if let Ok(directory_string) = toml::to_string(&self) {
            let Ok(()) = fs::write(path, &directory_string) else {
                return Err(SurfBoredError::DirectoryFileWriteError);
            };
        } else {
            return Err(SurfBoredError::DirectorySerialzationError);
        }
        Ok(())
        // fs::write(path, toml::to_string(&self).unwrap()).unwrap();
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
    pub directory_path: String,
    pub history: History,
    pub current_view: View,
    pub previous_view: View,
    pub selected_notice: Option<usize>,
}
impl App {
    pub fn new() -> App {
        App {
            client: None, //BoredClient::init(ConnectionType::Local).await.ok(),
            directory: Directory::new(),
            directory_path: "directory_of_bored.toml".to_string(),
            history: History::new(),
            current_view: View::BoredView(None),
            previous_view: View::BoredView(None),
            selected_notice: None,
        }
    }

    pub async fn init_client(&mut self) -> Result<(), BoredError> {
        self.client = Some(BoredClient::init(ConnectionType::Local).await?);
        Ok(())
    }

    pub fn load_directory(&mut self) -> Result<(), SurfBoredError> {
        self.directory = Directory::load_file(&self.directory_path)?;
        Ok(())
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
    ) -> Result<Bored, SurfBoredError> {
        let Some(ref mut client) = self.client else {
            return Err(SurfBoredError::BoredError(
                BoredError::ClientConnectionError,
            ));
        };
        client
            .create_bored(name, Coordinate { x: 120, y: 40 }, private_key)
            .await?;
        let bored = client.get_current_bored()?;
        self.current_view = View::BoredView(Some(client.get_bored_address()?));
        self.directory.add(
            Listing {
                name: client.get_bored_name()?.to_string(),
                bored_address: format!("{}", client.get_bored_address()?),
            },
            &self.directory_path,
        )?;
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

#[cfg(test)]

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_load() -> Result<(), SurfBoredError> {
        let mut directory;
        {
            let mut app = App::new();
            app.directory_path = "test_directory.toml".to_string();
            app.init_client().await?;
            app.create_bored_on_network("I am bored", "").await?;
            directory = app.directory.clone();
        }
        {
            let mut app = App::new();
            app.directory_path = "test_directory.toml".to_string();
            app.init_client().await?;
            app.load_directory()?;
            assert_eq!(directory, app.directory);
            app.create_bored_on_network("We are bored", "").await?;
            directory = app.directory.clone();
        }
        let mut app = App::new();
        app.directory_path = "test_directory.toml".to_string();
        app.init_client().await?;
        app.load_directory()?;
        assert_eq!(directory, app.directory);
        Ok(())
    }
}
