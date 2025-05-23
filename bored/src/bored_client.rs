use crate::notice::{Display, Notice};
use crate::{Bored, BoredAddress, BoredError, Coordinate};
use autonomi::client::payment::PaymentOption;
use autonomi::{Bytes, Client, Network, Scratchpad, SecretKey, Wallet};
use std::clone;
use std::error::Error;

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
        let connection_type = connection_type;
        let client = match Client::init_local().await {
            Err(_) => return Err(BoredError::ClientConnectionError),
            Ok(client) => client,
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
        let wallet = match get_funded_wallet(self.connection_type, private_key).await {
            Ok(wallet) => wallet,
            Err(_) => return Err(BoredError::FailedToGetWallet),
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
    pub async fn update_bored(&mut self, updated_bored: &Bored) -> Result<(), BoredError> {
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
        let serialized_bored = serde_json::to_vec(&updated_bored)?;
        let content = Bytes::from(serialized_bored);
        self.client
            .scratchpad_update(bored_address.get_key(), 27, &content)
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

    /// create a draft notice that can be edited and added to the bored
    pub fn create_draft(&mut self, dimensions: Coordinate) -> Result<(), BoredError> {
        let Some(bored) = &self.current_bored else {
            return Err(BoredError::NoBored);
        };
        if dimensions.within(&bored.get_dimensions()) {
            self.draft_notice = Some(Notice::create(dimensions));
            return Ok(());
        }
        Err(BoredError::NoticeOutOfBounds)
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
        let mut bored = bored_client.current_bored.as_ref().unwrap().clone();
        let mut notice = Notice::new();
        notice.write("We are bored")?;
        bored.add(notice, Coordinate { x: 1, y: 1 })?;
        bored_client.update_bored(&bored).await?;
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
        assert_eq!(antnet_bored, bored);
        assert_eq!(bored_client.current_bored.unwrap(), bored);
        Ok(())
    }
}
