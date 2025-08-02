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

use crate::notice::{Display, Notice};
use crate::url::{BoredAddress, URL};
use crate::{Bored, BoredError, Coordinate};
use autonomi::client::payment::PaymentOption;
use autonomi::data::DataAddress;
use autonomi::scratchpad::ScratchpadError;
use autonomi::{Bytes, Client, Network, Scratchpad, SecretKey, Wallet};
use regex::bytes;
use std::clone;
use std::error::Error;
use std::fmt::Debug;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub enum ConnectionType {
    Local,
    Antnet,
}

/// An client implementing the methods of the Bored protocol via an autonomi client for storage
pub struct BoredClient {
    connection_type: ConnectionType,
    client: Client,
    current_bored: Option<Bored>,
    draft_notice: Option<Notice>,
    scratchpad_counter: Option<u64>,
    bored_address: Option<BoredAddress>,
}

impl BoredClient {
    pub async fn init(connection_type: ConnectionType) -> Result<BoredClient, BoredError> {
        // let connection_type = connection_type;
        let client = match connection_type {
            ConnectionType::Antnet => match Client::init().await {
                Err(_) => return Err(BoredError::ClientConnectionError),
                Ok(client) => client,
            },

            ConnectionType::Local => match Client::init_local().await {
                Err(_) => return Err(BoredError::ClientConnectionError),
                Ok(client) => client,
            },
        };
        Ok(BoredClient {
            connection_type,
            client,
            current_bored: None,
            draft_notice: None,
            scratchpad_counter: None,
            bored_address: None,
        })
    }

    /// estimate of storing bored in scratchpad
    pub async fn get_cost(&self) -> Result<String, BoredError> {
        let cost = self
            .client
            .scratchpad_cost(&BoredAddress::new().get_key().public_key())
            .await?;
        Ok(cost.to_string())
    }

    /// Creates a new instance of a board and places in current_bored and attempts to create
    /// a scratchpad containing it at the BoredAddress
    pub async fn create_bored(
        &mut self,
        name: &str,
        dimensions: Coordinate,
        private_key: &str,
    ) -> Result<(), BoredError> {
        let bored = Bored::create(name, dimensions);
        self.bored_address = Some(BoredAddress::new());
        let serialized_bored = serde_json::to_vec(&bored)?;
        let content = Bytes::from(serialized_bored);
        let wallet = match self.get_funded_wallet(private_key).await {
            Ok(wallet) => wallet,
            Err(e) => {
                return Err(BoredError::FailedToGetWallet(
                    private_key.to_string(),
                    format!("{:?}", e),
                ));
            }
        };
        let payment_option = PaymentOption::from(&wallet);
        let (..) = self
            .client
            .scratchpad_create(
                &self.bored_address.as_ref().unwrap().get_key(),
                27,
                &content,
                payment_option,
            )
            .await?;
        // wait for the scratchpad to be replicated
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        self.refresh_bored().await?; // local bored is update by downloading to make sure in sync
        Ok(())
    }

    /// Get bored address if it exists
    pub fn get_bored_address(&self) -> Result<BoredAddress, BoredError> {
        let Some(bored_address) = &self.bored_address else {
            return Err(BoredError::NoBored);
        };
        Ok(bored_address.clone())
    }

    /// Get bored name if it exists
    pub fn get_bored_name(&self) -> Result<&str, BoredError> {
        let Some(bored) = &self.current_bored else {
            return Err(BoredError::NoBored);
        };
        Ok(&bored.name)
    }

    /// Attempt to download a bored and set current_bored to it if succesfful
    pub async fn go_to_bored(&mut self, bored_address: &BoredAddress) -> Result<(), BoredError> {
        let bored_address = bored_address.clone();
        let (bored, scratchpad_counter) = self.retrieve_bored(&bored_address).await?;
        self.current_bored = Some(bored);
        self.bored_address = Some(bored_address);
        self.scratchpad_counter = Some(scratchpad_counter);
        Ok(())
    }

    /// Downloads an existing bored
    pub async fn retrieve_bored(
        &mut self,
        bored_address: &BoredAddress,
    ) -> Result<(Bored, u64), BoredError> {
        let got = self
            .client
            .scratchpad_get_from_public_key(&bored_address.get_public_key())
            .await?;
        let content = match got.decrypt_data(bored_address.get_key()) {
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
        // if bored.protocol_version.get_version() < 1 {
        //     return Err(BoredError::MethodNotInProtocol);
        // }
        Ok((bored, got.counter()))
    }

    /// Refresh the current bored
    pub async fn refresh_bored(&mut self) -> Result<(), BoredError> {
        let Some(bored_address) = &self.bored_address else {
            return Err(BoredError::NoBored);
        };
        let bored_address = bored_address.clone();
        let (bored, scratchpad_counter) = self.retrieve_bored(&bored_address).await?;
        self.current_bored = Some(bored);
        self.scratchpad_counter = Some(scratchpad_counter);
        Ok(())
    }

    /// Updates the current bored, if there is a newer version of the bored on th antnet it
    /// returns it within the error so that the local version can be updated
    /// if bored to big for scratchpad it will try and remove the oldest notice from the bored
    pub async fn update_bored(&mut self) -> Result<(), BoredError> {
        let Some(bored_address) = &self.bored_address else {
            return Err(BoredError::NoBored);
        };
        let bored_address = bored_address.clone();
        if self.scratchpad_counter.is_none() {
            return Err(BoredError::BoredNotYetDownloaded);
        }
        let (bored, scratchpad_counter) = &self.retrieve_bored(&bored_address).await?;
        if scratchpad_counter > &self.scratchpad_counter.unwrap() {
            return Err(BoredError::MoreRecentVersionExists(
                bored.clone(),
                scratchpad_counter.clone(),
            ));
        }
        let serialized_bored = serde_json::to_vec(&self.current_bored)?;
        let content = Bytes::from(serialized_bored);
        match self
            .client
            .scratchpad_update(bored_address.get_key(), 27, &content)
            .await
        {
            Err(ScratchpadError::ScratchpadTooBig(_)) => {
                if let Some(current_bored) = &mut self.current_bored {
                    // remove the notice that was just added making it to big
                    current_bored.remove_newest_notice();
                    // remove the oldest notice so there may be room to add more
                    current_bored.remove_oldest_notice();
                }
                Err(BoredError::BoredTooBig)
            }
            Err(e) => Err(e.into()),
            Ok(()) => Ok(()),
        }?;
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

    /// create a draft notice that can be edited and added to the bored
    pub fn create_draft(&mut self, dimensions: Coordinate) -> Result<(), BoredError> {
        let Some(bored) = &self.current_bored else {
            return Err(BoredError::NoBored);
        };
        if dimensions.within(&bored.get_dimensions()) {
            self.draft_notice = Some(Notice::create(dimensions));
            return Ok(());
        }
        Err(BoredError::NoticeOutOfBounds(
            bored.get_dimensions(),
            dimensions,
        ))
    }

    /// get the draft notice as an option
    pub fn get_draft(&self) -> Option<Notice> {
        self.draft_notice.clone()
    }

    /// check the content will fit in the notice and update content if so
    pub fn edit_draft(&mut self, content: &str) -> Result<(), BoredError> {
        let Some(bored) = &self.current_bored else {
            return Err(BoredError::NoBored);
        };
        if let Some(mut notice) = self.draft_notice.clone() {
            notice.write(content)?;
            self.draft_notice = Some(notice);
        }
        Ok(())
    }

    /// Position draft on bored
    pub fn position_draft(&mut self, new_top_left: Coordinate) -> Result<(), BoredError> {
        let Some(bored) = &self.current_bored else {
            return Err(BoredError::NoBored);
        };
        if let Some(mut notice) = self.draft_notice.clone() {
            notice.relocate(&bored, new_top_left)?;
            self.draft_notice = Some(notice);
        }
        Ok(())
    }

    /// Add draft notice to bored
    pub async fn add_draft_to_bored(&mut self) -> Result<(), BoredError> {
        let Some(bored) = &mut self.current_bored else {
            return Err(BoredError::NoBored);
        };
        if let Some(notice) = &self.draft_notice {
            bored.add(notice.clone(), notice.get_top_left())?;
            self.draft_notice = None;
        }
        match self.update_bored().await {
            Err(bored_error) => match bored_error {
                // if more recent version update to new version but also pass error
                // so ui can out put message
                BoredError::MoreRecentVersionExists(ref bored, scratchpad_counter) => {
                    self.current_bored = Some(bored.clone());
                    self.scratchpad_counter = Some(scratchpad_counter);
                    return Err(bored_error);
                }
                _ => return Err(bored_error),
            },
            _ => (),
        }
        Ok(())
    }

    /// Download public archive from antnet for antnet hyperlinks
    /// not supporting single files as no way to know file name
    pub async fn download_file(
        &self,
        data_address: &DataAddress,
        download_path: &str,
        file_name: &str,
    ) -> Result<Option<PathBuf>, BoredError> {
        let mut path_to_open = None;
        let archive_result = self.client.archive_get_public(&data_address).await;
        match archive_result {
            Ok(archive) => {
                for (index, (item_path, addr, _)) in archive.iter().enumerate() {
                    let bytes = self.client.data_get_public(addr).await.unwrap();
                    let path = PathBuf::from(download_path).join(item_path);
                    let here = PathBuf::from(".");
                    let parent = path.parent().unwrap_or_else(|| &here);
                    std::fs::create_dir_all(parent).unwrap();
                    std::fs::write(path.clone(), bytes).unwrap();
                    if index == 0 {
                        path_to_open = Some(path);
                    }
                }
            }
            Err(_) => {
                if let Ok(bytes) = self.client.data_get_public(&data_address).await {
                    let path: PathBuf = [download_path, file_name].iter().collect();
                    let here = PathBuf::from(".");
                    let parent = path.parent().unwrap_or_else(|| &here);
                    std::fs::create_dir_all(parent)?;
                    std::fs::write(path.clone(), bytes)?;
                    path_to_open = Some(path);
                } else {
                    return Err(BoredError::NotValidAntAddress);
                }
            }
        }
        Ok(path_to_open)
    }

    async fn get_funded_wallet(&self, private_key: &str) -> Result<Wallet, Box<dyn Error>> {
        let private_key = match self.connection_type {
            ConnectionType::Antnet => private_key,
            ConnectionType::Local => {
                "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            }
        };
        let wallet = Wallet::new_from_private_key(self.client.evm_network().clone(), private_key)?;
        Ok(wallet)
    }
}

// async fn get_funded_wallet(
//     connection_type: ConnectionType,
//     private_key: &str,
// ) -> Result<Wallet, Box<dyn Error>> {
//     let (local, private_key) = match connection_type {
//         ConnectionType::Antnet => (false, private_key),
//         ConnectionType::Local => (
//             true,
//             "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
//         ),
//     };
//     let network = Network::new(local)?;
//     let wallet = Wallet::new_from_private_key(network, private_key)?;
//     Ok(wallet)
// }

#[cfg(test)]
mod tests {

    use std::ops::DerefMut;

    use super::*;

    // Test marked with ignore require a local network to be running

    #[tokio::test]
    #[ignore]
    async fn test_get_cost() -> Result<(), BoredError> {
        let bored_client = BoredClient::init(ConnectionType::Local).await?;
        assert!(bored_client.get_cost().await.is_ok());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_create_bored() -> Result<(), BoredError> {
        let mut bored_client = BoredClient::init(ConnectionType::Local).await?;
        bored_client
            .create_bored("", Coordinate { x: 120, y: 40 }, "")
            .await?;
        let bored = bored_client.current_bored.as_ref().unwrap().clone();
        bored_client.refresh_bored().await?;
        assert_eq!(bored_client.current_bored.unwrap(), bored);
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_update_bored() -> Result<(), BoredError> {
        let mut bored_client = BoredClient::init(ConnectionType::Local).await?;
        bored_client
            .create_bored("I am BORED", Coordinate { x: 120, y: 40 }, "")
            .await?;
        let scrachpad_counter = bored_client.scratchpad_counter.unwrap();
        let mut bored = bored_client.get_current_bored().unwrap();
        let mut notice = Notice::new();
        notice.write("We are bored")?;
        bored.add(notice, Coordinate { x: 1, y: 1 })?;
        bored_client.current_bored = Some(bored.clone());
        bored_client.update_bored().await?;
        // wait for the scratchpad to be replicated
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let (antnet_bored, antnet_counter) = bored_client
            .retrieve_bored(&bored_client.get_bored_address()?)
            .await?;
        assert_eq!(
            bored_client.scratchpad_counter.unwrap(),
            scrachpad_counter + 1,
        );
        assert_eq!(bored_client.scratchpad_counter.unwrap(), antnet_counter);
        assert_eq!(antnet_bored, bored.clone());
        assert_eq!(bored_client.current_bored.unwrap(), bored.clone());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_add_draft_to_bored() -> Result<(), BoredError> {
        let mut bored_client = BoredClient::init(ConnectionType::Local).await?;
        bored_client
            .create_bored("", Coordinate { x: 120, y: 40 }, "")
            .await?;
        bored_client.create_draft(Coordinate { x: 60, y: 18 })?;
        bored_client.edit_draft("I am BORED")?;
        let draft = bored_client.get_draft().unwrap().clone();
        bored_client.add_draft_to_bored().await?;
        let bored = bored_client.current_bored.as_ref().unwrap().clone();
        assert_eq!(bored.notices[0], draft);
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let (antnet_bored, antnet_counter) = bored_client
            .retrieve_bored(&bored_client.get_bored_address()?)
            .await?;
        assert_eq!(bored_client.scratchpad_counter.unwrap(), antnet_counter);
        assert_eq!(antnet_bored, bored.clone());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_edit_draft() -> Result<(), BoredError> {
        let mut bored_client = BoredClient::init(ConnectionType::Local).await?;
        bored_client
            .create_bored("", Coordinate { x: 120, y: 40 }, "")
            .await?;
        // let bored = bored_client.current_bored.as_ref().unwrap().clone();
        bored_client
            .create_draft(Coordinate { x: 0, y: 0 })
            .unwrap();
        assert_eq!(
            bored_client.edit_draft("I am BORED"),
            Err(BoredError::TooMuchText)
        );
        bored_client
            .create_draft(Coordinate { x: 7, y: 4 })
            .unwrap();
        assert_eq!(
            bored_client.edit_draft("I am BORED!"),
            Err(BoredError::TooMuchText)
        );
        bored_client
            .create_draft(Coordinate { x: 7, y: 4 })
            .unwrap();
        assert_eq!(bored_client.edit_draft("I am BORED"), Ok(()));
        assert_eq!(
            bored_client.draft_notice.as_ref().unwrap().get_content(),
            "I am BORED"
        );
        bored_client
            .create_draft(Coordinate { x: 7, y: 4 })
            .unwrap();
        assert_eq!(
            bored_client.edit_draft("I\nam\nBORED"),
            Err(BoredError::TooMuchText)
        );
        bored_client
            .create_draft(Coordinate { x: 7, y: 6 })
            .unwrap();
        assert_eq!(bored_client.edit_draft("I\nam\nBORED"), Ok(()));
        let draft_notice = bored_client.draft_notice.clone();
        assert_eq!(draft_notice.as_ref().unwrap().get_content(), "I\nam\nBORED");
        bored_client
            .create_draft(Coordinate { x: 7, y: 4 })
            .unwrap();
        assert_eq!(
            bored_client.edit_draft("I am [BORED](NOT)!"),
            Err(BoredError::TooMuchText)
        );
        assert_eq!(bored_client.edit_draft("I am [BORED](NOT)"), Ok(()));
        assert_eq!(
            bored_client.draft_notice.as_ref().unwrap().get_content(),
            "I am [BORED](NOT)"
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_adding_notice_too_big_for_scratchpad() -> Result<(), BoredError> {
        let mut bored_client = BoredClient::init(ConnectionType::Local).await?;
        bored_client
            .create_bored("", Coordinate { x: 10000, y: 10000 }, "")
            .await?;
        bored_client
            .create_draft(Coordinate { x: 4000, y: 4000 })
            .unwrap();
        // this string should be about half the max a scratchpad can store
        // so one should be fine but two is two many
        let realy_long_string = "a".repeat(2 * 1024 * 1024);
        bored_client.edit_draft(&realy_long_string).unwrap();
        assert_eq!(bored_client.add_draft_to_bored().await, Ok(()));
        assert_eq!(bored_client.get_current_bored().unwrap().notices.len(), 1);
        bored_client
            .create_draft(Coordinate { x: 4000, y: 4000 })
            .unwrap();
        bored_client.position_draft(Coordinate { x: 1, y: 0 })?;
        let realy_long_string = "b".repeat(2 * 1024 * 1024);
        bored_client.edit_draft(&realy_long_string).unwrap();
        assert_eq!(
            bored_client.add_draft_to_bored().await,
            Err(BoredError::BoredTooBig)
        );
        // check oldest (and only) notice has been removed
        assert_eq!(bored_client.get_current_bored().unwrap().notices.len(), 0);
        bored_client
            .create_draft(Coordinate { x: 4000, y: 4000 })
            .unwrap();
        bored_client.position_draft(Coordinate { x: 2, y: 0 })?;
        let realy_long_string = "b".repeat(2 * 1024 * 1024);
        bored_client.edit_draft(&realy_long_string).unwrap();
        assert_eq!(bored_client.add_draft_to_bored().await, Ok(()));
        assert_eq!(bored_client.get_current_bored().unwrap().notices.len(), 1);
        assert_eq!(
            bored_client.get_current_bored().unwrap().notices[0].get_content(),
            realy_long_string
        );
        Ok(())
    }
}
