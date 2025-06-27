use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::{self, Notice, NoticeHyperlinkMap};
use bored::{Bored, BoredAddress, BoredError, Coordinate, Direction};
use ratatui::style::{Color, Style, Stylize};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::display_bored::BoredViewPort;

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    ErrorView(SurfBoredError),
    BoredView,
    NoticeView { hyperlinks_index: Option<usize> },
    DraftView(DraftMode),
    CreateView(CreateMode),
    GoToView(GoToMode),
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum DraftMode {
    Content,
    Hyperlink(HyperlinkMode),
    Position,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HyperlinkMode {
    Text,
    URL,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GoToMode {
    Directory,
    PasteAddress,
}

pub enum NoticeSelection {
    Direction(Direction),
    Next,
    Previous,
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
#[derive(Clone)]
pub struct Theme {
    name: String,
    text_fg: Color,
    text_bg: Color,
    dimmed_text_fg: Color,
    header_bg: Color,
    hyperlink_style: Style,
}

impl Theme {
    pub fn surf_bored_synth_wave() -> Theme {
        Theme {
            name: "Surf bored synth wave".to_string(),
            text_fg: Color::Rgb(205, 152, 211),
            text_bg: Color::Rgb(23, 21, 41),
            dimmed_text_fg: Color::Rgb(205, 152, 211),
            header_bg: Color::Rgb(109, 228, 175), // bright green header_bg: Color::Rgb(149, 232, 196), // pale green
            hyperlink_style: Style::new().underlined(),
        }
    }

    /// to use for tests so should not be amended
    pub fn default() -> Theme {
        let style = Style::default();
        Theme {
            name: "Default".to_string(),
            text_fg: style.fg.unwrap_or_default(),
            text_bg: style.bg.unwrap_or_default(),
            dimmed_text_fg: style.fg.unwrap_or_default(),
            header_bg: style.bg.unwrap_or_default(),
            hyperlink_style: Style::new().underlined(),
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

    pub fn dimmed_text_style(&self) -> Style {
        Style::new().fg(self.dimmed_text_fg).bg(self.text_bg)
    }

    pub fn hyperlink_style(&self) -> Style {
        self.hyperlink_style
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
    pub bored_view_port: Option<BoredViewPort>,
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
            bored_view_port: None,
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
        self.selected_notice = None;
        self.previous_view = self.current_view.clone();
        self.current_view = View::BoredView;
        // could this happen befored the bored is loaded and hence still be None?
        let bored = client.get_current_bored()?;
        self.bored_view_port = Some(BoredViewPort::create(
            &bored,
            bored.get_dimensions(),
            self.selected_notice,
        ));
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
        dimensions: Coordinate,
    ) -> Result<Bored, SurfBoredError> {
        let Some(ref mut client) = self.client else {
            return Err(SurfBoredError::BoredError(
                BoredError::ClientConnectionError,
            ));
        };
        client.create_bored(name, dimensions, private_key).await?;
        let bored = client.get_current_bored()?;
        self.selected_notice = None;
        self.current_view = View::BoredView;
        self.bored_view_port = Some(BoredViewPort::create(
            &bored,
            bored.get_dimensions(),
            self.selected_notice,
        ));
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

    pub async fn add_draft_to_bored(&mut self) -> Result<(), BoredError> {
        let Some(ref mut client) = self.client else {
            return Err(BoredError::ClientConnectionError);
        };
        client.add_draft_to_bored().await?;
        Ok(())
    }

    pub fn select_notice(&mut self, direction: Direction) {
        if let Some(bored) = self.get_current_bored() {
            if bored.get_notices().len() > 0 {
                if self.selected_notice.is_none() {
                    self.selected_notice = bored.get_upper_left_most_notice()
                } else {
                    self.selected_notice =
                        match bored.get_cardinal_notice(self.selected_notice.unwrap(), direction) {
                            Some(notice_index) => Some(notice_index),
                            None => self.selected_notice,
                        }
                }
            }
        }
    }

    pub fn get_selected_notice(&self) -> Option<Notice> {
        if let Some(notice_index) = self.selected_notice {
            return self
                .get_current_bored()
                .map(|b| b.get_notices()[notice_index].clone());
        }
        None
    }

    pub fn increment_selected_notice(&mut self) {
        if let Some(bored) = self.get_current_bored() {
            if self.selected_notice.is_none() && !bored.get_notices().len() > 0 {
                self.selected_notice = Some(0);
            } else {
                if let Some(notices_index) = self.selected_notice {
                    if notices_index >= bored.get_notices().len() - 1 {
                        self.selected_notice = Some(0);
                    } else {
                        self.selected_notice = Some(notices_index + 1);
                    }
                }
            }
        }
    }

    pub fn decrement_selected_notice(&mut self) {
        if let Some(notices_index) = self.selected_notice {
            if let Some(bored) = self.get_current_bored() {
                if notices_index == 0 {
                    self.selected_notice = Some(bored.get_notices().len() - 1);
                } else {
                    self.selected_notice = Some(notices_index - 1);
                }
            }
        }
    }

    pub fn create_hyperlink(&mut self) {
        self.current_view = View::DraftView(DraftMode::Hyperlink(HyperlinkMode::Text));
    }

    pub fn position_draft(&mut self, new_top_left: Coordinate) -> Result<bool, BoredError> {
        if let Some(draft) = self.get_draft() {
            let new_bottom_right = new_top_left.add(&draft.get_dimensions());
            let Some(ref mut client) = self.client else {
                return Err(BoredError::ClientConnectionError);
            };
            match client.position_draft(new_top_left) {
                Err(bored_error) => match bored_error {
                    // if new position is out of bound do nothing so user can't move it there
                    BoredError::NoticeOutOfBounds(..) => return Ok(true),
                    _ => return Err(bored_error),
                },
                Ok(_) => {
                    if let Some(bored_view_port) = &self.bored_view_port {
                        return Ok(bored_view_port.in_view(new_top_left, new_bottom_right));
                    }
                }
            }
        }
        Ok(true)
    }

    pub fn next_hyperlink(&mut self) {
        if let View::NoticeView { hyperlinks_index } = self.current_view {
            if let (Some(notices), Some(notice_index)) = (
                self.get_current_bored().map(|b| b.get_notices()),
                self.selected_notice,
            ) {
                if let Some(Ok(hyperlinks)) = notices
                    .get(notice_index)
                    .map(|n| n.get_display().map(|d| d.get_hyperlink_locations()))
                {
                    // self.status = format!("hyperlinks: {:?}", hyperlinks);
                    self.current_view = if hyperlinks_index.is_none() && !hyperlinks.is_empty() {
                        View::NoticeView {
                            hyperlinks_index: Some(0),
                        }
                    } else if hyperlinks_index.is_some_and(|i| i + 1 < hyperlinks.len()) {
                        View::NoticeView {
                            hyperlinks_index: Some(hyperlinks_index.unwrap() + 1),
                        }
                    } else if hyperlinks_index.is_some_and(|i| i + 1 >= hyperlinks.len()) {
                        View::NoticeView {
                            hyperlinks_index: Some(0),
                        }
                    } else {
                        View::NoticeView {
                            hyperlinks_index: None,
                        }
                    }
                }
            }
        }
    }

    pub fn previous_hyperlink(&mut self) {
        if let View::NoticeView { hyperlinks_index } = self.current_view {
            if let (Some(notices), Some(notice_index)) = (
                self.get_current_bored().map(|b| b.get_notices()),
                self.selected_notice,
            ) {
                if let Some(Ok(hyperlinks)) = notices
                    .get(notice_index)
                    .map(|n| n.get_display().map(|d| d.get_hyperlink_locations()))
                {
                    self.status = format!("hyperlinks: {:?}", hyperlinks);
                    self.current_view = if hyperlinks_index.is_none() && !hyperlinks.is_empty() {
                        View::NoticeView {
                            hyperlinks_index: Some(hyperlinks.len() - 1),
                        }
                    } else if hyperlinks_index.is_some_and(|i| i > 0) {
                        View::NoticeView {
                            hyperlinks_index: Some(hyperlinks_index.unwrap() - 1),
                        }
                    } else if hyperlinks_index.is_some_and(|i| i == 0) {
                        View::NoticeView {
                            hyperlinks_index: Some(hyperlinks.len() - 1),
                        }
                    } else {
                        View::NoticeView {
                            hyperlinks_index: None,
                        }
                    }
                }
            }
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
            app.create_bored_on_network("I am bored", "", Coordinate { x: 120, y: 40 })
                .await?;
            directory = app.directory.clone();
        }
        {
            let mut app = App::new();
            app.directory_path = "test_directory.toml".to_string();
            app.init_client().await?;
            app.load_directory()?;
            assert_eq!(directory, app.directory);
            app.create_bored_on_network("We are bored", "", Coordinate { x: 120, y: 40 })
                .await?;
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
