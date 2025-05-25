use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::Notice;
use bored::{Bored, BoredAddress, BoredError, Coordinate, Direction};
use rand::seq::IndexedRandom;
use ratatui::style::{Color, Style};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, thiserror::Error, Clone)]
pub enum SurfBoredError {
    #[error("{0}")]
    BoredError(BoredError),
    #[error("Could not read directory file so directory is empty.")]
    DirectoryFileReadError,
    #[error("Directory not saved to disk as could not write to file.")]
    DirectoryFileWriteError,
    #[error("Could not serialize directory file so directory is empty.")]
    DirectorySerialzationError,
    #[error("Could not derserialize directory file so directory is empty.")]
    DirectoryDeserialzationError,
}

impl From<BoredError> for SurfBoredError {
    fn from(e: BoredError) -> Self {
        Self::BoredError(e)
    }
}

#[derive(Clone, Debug)]
pub enum View {
    ErrorView(SurfBoredError),
    BoredView,
    NoticeView { hyperlinks_index: Option<usize> },
    DraftView(DraftMode),
    CreateView(CreateMode),
    GoToView(GoToMode),
    Quitting,
}

#[derive(Clone, Debug)]
pub enum CreateMode {
    Name,
    PrivateKey,
}
impl CreateMode {
    pub fn toggle(&self) -> CreateMode {
        match self {
            CreateMode::Name => CreateMode::PrivateKey,
            CreateMode::PrivateKey => CreateMode::Name,
        }
    }
}

#[derive(Clone, Debug)]
pub enum DraftMode {
    Content,
    Hyperlink(HyperlinkMode),
    Position,
}

#[derive(Clone, Debug)]
pub enum HyperlinkMode {
    Text,
    URL,
}

#[derive(Clone, Debug)]
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
            return Err(SurfBoredError::DirectoryFileReadError);
        }
    }

    fn add(&mut self, listing: Listing, path: &str) -> Result<(), SurfBoredError> {
        self.bored_addresses.push(listing);
        // this is only for convience in testing remove once working
        self.home_bored = self.bored_addresses.len() - 1;
        if let Ok(directory_string) = toml::to_string(&self) {
            let Ok(()) = fs::write(path, &directory_string) else {
                return Err(SurfBoredError::DirectoryFileWriteError);
            };
        } else {
            return Err(SurfBoredError::DirectorySerialzationError);
        }
        Ok(())
    }

    pub fn set_home(&mut self, home_bored: usize) {
        self.home_bored = home_bored
    }

    pub fn get_home(&self) -> Option<&str> {
        if self.home_bored < self.bored_addresses.len() {
            return Some(&self.bored_addresses[self.home_bored].bored_address);
        } else {
            None
        }
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

/// Represent colours in theme used by app
pub struct Theme {
    name: String,
    text_fg: Color,
    text_bg: Color,
    header_bg: Color,
}

impl Theme {
    pub fn surf_bored_synth_wave() -> Theme {
        Theme {
            name: "Surf bored synth wave".to_string(),
            text_fg: Color::Rgb(205, 152, 211),
            text_bg: Color::Rgb(23, 21, 41),
            header_bg: Color::Rgb(109, 228, 175), // bright green
                                                  // header_bg: Color::Rgb(149, 232, 196), // pale green
        }
    }

    pub fn header_style(&self) -> Style {
        Style::new().fg(self.text_bg).bg(self.header_bg)
    }

    pub fn text_style(&self) -> Style {
        Style::new().fg(self.text_fg).bg(self.text_bg)
    }

    pub fn inverted_text_style(&self) -> Style {
        Style::new().fg(self.text_bg).bg(self.text_fg)
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
    pub theme: Theme,
    pub status: String,
    pub name_input: String,
    pub key_input: String,
    pub content_input: String,
    pub link_text_input: String,
    pub link_url_input: String,
}
impl App {
    pub fn new() -> App {
        App {
            client: None, //BoredClient::init(ConnectionType::Local).await.ok(),
            directory: Directory::new(),
            directory_path: "directory_of_boreds.toml".to_string(),
            history: History::new(),
            current_view: View::BoredView,
            previous_view: View::BoredView,
            selected_notice: None,
            theme: Theme::surf_bored_synth_wave(),
            status: String::new(),
            name_input: String::new(),
            key_input: String::new(),
            content_input: String::new(),
            link_text_input: String::new(),
            link_url_input: String::new(),
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

    pub fn display_error(&mut self, surf_bored_error: SurfBoredError) {
        self.previous_view = self.current_view.clone();
        self.current_view = View::ErrorView(surf_bored_error);
    }

    /// set previous view so can allways go back
    pub fn change_view(&mut self, view: View) {
        self.previous_view = self.current_view.clone();
        self.current_view = view;
    }

    /// go back to previous view
    pub fn revert_view(&mut self) {
        self.current_view = self.previous_view.clone();
    }

    pub fn goto(&mut self) {
        self.previous_view = self.current_view.clone();
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

    pub async fn goto_bored(&mut self, bored_address: BoredAddress) -> Result<(), SurfBoredError> {
        let Some(ref mut client) = self.client else {
            return Err(SurfBoredError::BoredError(
                BoredError::ClientConnectionError,
            ));
        };
        client.go_to_bored(&bored_address).await?;
        self.previous_view = self.current_view.clone();
        self.current_view = View::BoredView;
        Ok(())
    }

    pub fn view_notice(&mut self) {
        self.previous_view = self.current_view.clone();
        self.current_view = View::NoticeView {
            hyperlinks_index: None,
        };
    }

    pub fn create_bored(&mut self) {
        self.previous_view = self.current_view.clone();
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
        self.current_view = View::BoredView;
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
        self.change_view(View::DraftView(DraftMode::Content));
        Ok(())
    }

    pub fn get_draft(&self) -> Option<Notice> {
        let Some(ref client) = self.client else {
            return None;
        };
        client.get_draft()
    }

    pub fn edit_draft(&mut self, content: &str) -> Result<(), BoredError> {
        let Some(ref mut client) = self.client else {
            return Err(BoredError::ClientConnectionError);
        };
        client.edit_draft(content)?;
        Ok(())
    }

    pub fn create_hyperlink(&mut self) {
        self.current_view = View::DraftView(DraftMode::Hyperlink(HyperlinkMode::Text));
    }

    pub fn position_draft(&mut self, new_top_left: Coordinate) -> Result<(), BoredError> {
        let Some(ref mut client) = self.client else {
            return Err(BoredError::ClientConnectionError);
        };
        match client.position_draft(new_top_left) {
            Err(bored_error) => match bored_error {
                // if new position is out of bound do nothing so user can't move it there
                BoredError::NoticeOutOfBounds(..) => return Ok(()),
                _ => return Err(bored_error),
            },
            Ok(_) => Ok(()),
        }
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
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
