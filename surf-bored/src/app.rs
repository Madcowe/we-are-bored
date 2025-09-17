/*
Copyright (C) 2025 We are bored

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use autonomi::data::DataAddress;
use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::{Hyperlink, Notice, get_hyperlinks};
use bored::url::{BoredAddress, URL};
use bored::{Bored, BoredError, Coordinate, Direction};
use ratatui::{Terminal, backend::Backend, buffer::Buffer};
use std::io::Error;
use std::path::PathBuf;

use crate::directory::{self, Directory, Listing};
use crate::display_bored::BoredViewPort;
use crate::theme::Theme;
use crate::ui::wait_pop_up;

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum SurfBoredError {
    #[error("{0}")]
    Message(String),
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
    #[error("Antnet call timed out as never returned")]
    StillWaiting,
    #[error("Failed to render waiting pop up")]
    CannotRenderWait,
    #[error("The directory of boreds is currently empty")]
    DirectoryIsEmpty,
    #[error("The index: {0} is out of bounds of directory of len {1}")]
    DirectoryOutOfBounds(usize, usize),
    #[error("{0}")]
    IOError(String),
    #[error("The application command in the hyperlink is not know by this appication:\n{0}")]
    LinkCommandUnknown(String),
}

impl From<BoredError> for SurfBoredError {
    fn from(e: BoredError) -> Self {
        Self::BoredError(e)
    }
}

impl From<Error> for SurfBoredError {
    fn from(e: Error) -> Self {
        let s = format!("{e}");
        Self::IOError(s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    ErrorView(SurfBoredError),
    BoredView,
    NoticeView { hyperlinks_index: Option<usize> },
    DraftView(DraftMode),
    CreateView(CreateMode),
    GoToView,
    DirectoryView(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CreateMode {
    Name,
    URLName,
    PrivateKey,
}
impl CreateMode {
    pub fn toggle(&self) -> CreateMode {
        match self {
            CreateMode::Name => CreateMode::URLName,
            CreateMode::URLName => CreateMode::PrivateKey,
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
impl HyperlinkMode {
    pub fn toggle(&self) -> HyperlinkMode {
        match self {
            HyperlinkMode::Text => HyperlinkMode::URL,
            HyperlinkMode::URL => HyperlinkMode::Text,
        }
    }
}

#[derive(Debug)]
pub enum NoticeSelection {
    Direction(Direction),
    Next,
    Previous,
    Current,
}

pub struct App {
    pub client: Option<BoredClient>,
    pub directory: Directory,
    pub directory_path: String,
    pub download_path: String,
    pub path_to_open: Option<PathBuf>,
    // pub history: History,
    pub current_view: View,
    pub previous_view: View,
    pub interupted_view: View,
    pub selected_notice: Option<usize>,
    pub theme: Theme,
    pub bored_view_port: Option<BoredViewPort>,
    // pub status: String,
    pub name_input: String,
    pub url_name_input: String,
    pub key_input: String,
    pub content_input: String,
    pub link_text_input: String,
    pub link_url_input: String,
    pub goto_input: String,
    pub menu_visible: bool,
}
impl App {
    pub fn new() -> App {
        App {
            client: None,
            directory: Directory::new(),
            directory_path: "directory_of_boreds.toml".to_string(),
            download_path: "downloads/".to_string(),
            path_to_open: None,
            // history: History::new(),
            current_view: View::BoredView,
            previous_view: View::BoredView,
            interupted_view: View::BoredView,
            selected_notice: None,
            theme: Theme::surf_bored_synth_wave(),
            bored_view_port: None,
            // status: String::new(),
            name_input: String::new(),
            url_name_input: String::new(),
            key_input: String::new(),
            content_input: String::new(),
            link_text_input: String::new(),
            link_url_input: String::new(),
            goto_input: String::new(),
            menu_visible: false,
        }
    }

    pub async fn init_client(&mut self, connection_type: ConnectionType) -> Result<(), BoredError> {
        self.client = Some(BoredClient::init(connection_type).await?);
        Ok(())
    }

    pub fn load_directory(&mut self) -> Result<(), SurfBoredError> {
        self.directory = Directory::load_file(&self.directory_path)?;
        Ok(())
    }

    pub fn save_directory(&self) -> Result<(), SurfBoredError> {
        self.directory.save_file(&self.directory_path)?;
        Ok(())
    }

    pub fn set_home(&mut self, directory_index: usize) -> Result<(), SurfBoredError> {
        self.directory.set_home(directory_index);
        self.directory.save_file(&self.directory_path)?;
        Ok(())
    }

    pub fn next_directory_item(&mut self, directory_index: usize) -> Result<usize, SurfBoredError> {
        let bored_addresses = self.directory.get_bored_addresses();
        if bored_addresses.is_empty() {
            return Err(SurfBoredError::DirectoryIsEmpty);
        } else if directory_index + 1 > bored_addresses.len() - 1 {
            return Ok(0);
        }
        Ok(directory_index + 1)
    }

    pub fn previous_directory_item(
        &mut self,
        directory_index: usize,
    ) -> Result<usize, SurfBoredError> {
        let bored_addresses = self.directory.get_bored_addresses();
        if bored_addresses.is_empty() {
            return Err(SurfBoredError::DirectoryIsEmpty);
        } else if directory_index >= 1 {
            return Ok(directory_index - 1);
        }
        Ok(bored_addresses.len() - 1)
    }

    pub fn display_error(&mut self, surf_bored_error: SurfBoredError) {
        self.change_view(View::ErrorView(surf_bored_error));
    }

    /// set previous view so can allways go back
    pub fn change_view(&mut self, view: View) {
        match view {
            View::ErrorView(_) => self.interupted_view(self.current_view.clone()),
            View::DirectoryView(_) => self.interupted_view(self.current_view.clone()),
            _ => {
                self.previous_view = self.current_view.clone();
            }
        }
        self.current_view = view.clone();
        self.menu_visible = false;
    }

    /// only sets interupted view if it is not an error/diretory
    fn interupted_view(&mut self, view: View) {
        match view {
            View::ErrorView(_) => (),
            View::DirectoryView(_) => (),
            _ => self.interupted_view = self.current_view.clone(),
        }
    }

    /// go back to previous view
    pub fn revert_view(&mut self) {
        match self.current_view {
            View::ErrorView(SurfBoredError::BoredError(BoredError::MoreRecentVersionExists(
                ..,
            ))) => self.current_view = View::BoredView,
            View::ErrorView(_) => self.current_view = self.interupted_view.clone(),
            View::DirectoryView(_) => self.current_view = self.interupted_view.clone(),
            _ => self.current_view = self.previous_view.clone(),
        }
        self.menu_visible = false;
    }

    pub async fn goto_bored(&mut self, bored_address: BoredAddress) -> Result<(), SurfBoredError> {
        let Some(ref mut client) = self.client else {
            return Err(SurfBoredError::BoredError(
                BoredError::ClientConnectionError,
            ));
        };
        client.go_to_bored(&bored_address).await?;
        self.selected_notice = None;
        // could this happen befored the bored is loaded and hence still be None?
        let bored = client.get_current_bored()?;
        self.revert_view();
        // self.change_view(View::BoredView);
        self.bored_view_port = Some(BoredViewPort::create(
            &bored,
            bored.get_dimensions(),
            self.selected_notice,
        ));
        Ok(())
    }

    pub fn get_current_bored(&self) -> Option<Bored> {
        if let Some(client) = &self.client {
            if let Ok(bored) = client.get_current_bored() {
                return Some(bored);
            }
        }
        return None;
    }

    pub fn get_current_address(&self) -> Option<BoredAddress> {
        if let Some(client) = &self.client {
            return client.get_bored_address().ok();
        }
        None
    }

    pub fn has_local_connection(&self) -> bool {
        if let Some(client) = &self.client {
            match client.get_connection_type() {
                ConnectionType::Local => return true,
                _ => (),
            }
        }
        false
    }

    pub async fn create_bored_on_network(
        &mut self,
        name: &str,
        private_key: &str,
        dimensions: Coordinate,
        url_name: Option<&str>,
    ) -> Result<(), SurfBoredError> {
        let Some(ref mut client) = self.client else {
            return Err(SurfBoredError::BoredError(
                BoredError::ClientConnectionError,
            ));
        };
        client
            .create_bored(name, dimensions, private_key.trim(), url_name)
            .await?;
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
        Ok(())
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

    pub async fn add_draft_to_bored(&mut self) -> Result<(), SurfBoredError> {
        // self.change_view(View::Waiting("Updating bored on antnet".to_string()));
        let Some(ref mut client) = self.client else {
            return Err(SurfBoredError::BoredError(
                BoredError::ClientConnectionError,
            ));
        };
        client
            .add_draft_to_bored()
            .await
            .map_err(|e| SurfBoredError::BoredError(e))?;
        // self.revert_view();
        Ok(())
    }

    pub fn select_notice(&mut self, direction: Direction) {
        if let Some(bored) = self.get_current_bored() {
            if bored.get_notices().len() > 0 {
                if self.selected_notice.is_none() {
                    self.selected_notice = bored.get_upper_left_most_notice();
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
                    // self.status = format!("hyperlinks: {:?}", hyperlinks);
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

    pub fn get_selected_hyperlink(&self) -> Option<Hyperlink> {
        if let (Some(notice), View::NoticeView { hyperlinks_index }) =
            (self.get_selected_notice(), &self.current_view)
        {
            if let Some(hyperlinks_index) = hyperlinks_index {
                if let Ok(hyperlinks) = get_hyperlinks(notice.get_content()) {
                    if let Some(hyperlink) = hyperlinks.get(*hyperlinks_index) {
                        return Some(hyperlink.clone());
                    }
                }
            }
        }
        None
    }

    pub async fn download_file(
        &mut self,
        data_address: &DataAddress,
        download_path: &str,
        file_name: &str, // only used if not archive
    ) -> Result<(), SurfBoredError> {
        let Some(ref client) = self.client else {
            return Err(BoredError::ClientConnectionError.into());
        };
        self.path_to_open = client
            .download_file(&data_address, download_path, &file_name)
            .await?;
        Ok(())
    }

    pub async fn go_home(&mut self) -> Result<(), SurfBoredError> {
        if let Some(home) = self.directory.get_home() {
            let home_address = BoredAddress::from_string(home)?;
            self.goto_bored(home_address).await?
        }
        Ok(())
    }

    pub async fn handle_hyperlink<B: Backend>(
        &mut self,
        hyperlink: Hyperlink,
        terminal: &mut Terminal<B>,
        previous_buffer: Buffer,
    ) -> Result<(), SurfBoredError> {
        let theme = self.theme.clone();
        let file_name = hyperlink.get_text();
        let url = URL::from_string(hyperlink.get_link())?;
        match url {
            URL::BoredNet(bored_address) => {
                let going_to_bored = self.goto_bored(bored_address);
                match wait_pop_up(
                    terminal,
                    previous_buffer,
                    going_to_bored,
                    "Loading bored from antnet...",
                    theme,
                )
                .await
                {
                    Err(e) => self.display_error(e),
                    _ => (),
                }
                // self.revert_view();
                return Ok(());
            }
            URL::BoredApp(command) => {
                let executing_command = self.hyperlink_command(&command);
                let message = if command == "home" {
                    "Loading home bored from antnet"
                } else {
                    ""
                };
                match wait_pop_up(terminal, previous_buffer, executing_command, message, theme)
                    .await
                {
                    Err(e) => Ok(self.display_error(e)),
                    _ => Ok(()),
                }
            }
            URL::ClearNet(clear_net_url) => {
                if let Err(_) = open::that(clear_net_url) {
                    return Err(SurfBoredError::Message(
                        "Could not open old fashioned (https/http) link".to_string(),
                    ));
                };
                // self.revert_view();
                return Ok(());
            }
            URL::AntNet(data_address) => {
                let download_path = self.download_path.clone();
                let downloading_file =
                    self.download_file(&data_address, &download_path, &file_name);
                match wait_pop_up(
                    terminal,
                    previous_buffer,
                    downloading_file,
                    "Downloading file(s) from antnet...\nIf it is a large file it may take some time.",
                    theme,
                )
                .await
                {
                    Err(e) => self.display_error(e),
                    _ => (),
                }
                if let Some(path) = self.path_to_open.clone() {
                    if let Err(_) = open::that(path) {
                        return Err(SurfBoredError::Message(
                            "Could not open downloaded file".to_string(),
                        ));
                    };
                }
                // self.revert_view();
                Ok(())
            }
        }
    }

    pub async fn hyperlink_command(&mut self, command: &str) -> Result<(), SurfBoredError> {
        if command == "about" {
            let Some(ref mut client) = self.client else {
                return Err(SurfBoredError::BoredError(
                    BoredError::ClientConnectionError,
                ));
            };
            self.selected_notice = None;
            self.current_view = View::BoredView;
            self.menu_visible = false;
            let about = directory::about_bored();
            self.bored_view_port = Some(BoredViewPort::create(
                &about,
                about.get_dimensions(),
                self.selected_notice,
            ));
            client.load_app_bored(about);
            Ok(())
        } else if command == "home" {
            self.go_home().await?;
            Ok(())
        } else {
            return Err(SurfBoredError::LinkCommandUnknown(command.to_string()));
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
            app.init_client(ConnectionType::Local).await?;
            app.create_bored_on_network("I am bored", "", Coordinate { x: 120, y: 40 }, None)
                .await?;
            directory = app.directory.clone();
        }
        {
            let mut app = App::new();
            app.directory_path = "test_directory.toml".to_string();
            app.init_client(ConnectionType::Local).await?;
            app.load_directory()?;
            assert_eq!(directory, app.directory);
            app.create_bored_on_network(
                "We are bored",
                "",
                Coordinate { x: 120, y: 40 },
                Some("bored.of.domains"),
            )
            .await?;
            directory = app.directory.clone();
        }
        let mut app = App::new();
        app.directory_path = "test_directory.toml".to_string();
        app.init_client(ConnectionType::Local).await?;
        app.load_directory()?;
        assert_eq!(directory, app.directory);
        Ok(())
    }
}
