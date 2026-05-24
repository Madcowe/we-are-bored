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

use crate::notice::Notice;
use crate::url::BoredAddress;
use crate::{Bored, BoredError, Coordinate, ProtocolVersion};

fn get_api_credentials() -> Option<(String, String)> {
    let home = std::env::var("HOME").ok()?;
    let path = std::path::PathBuf::from(home).join(".local/share/x0x");
    let port_str = std::fs::read_to_string(path.join("api.port")).ok()?.trim().to_string();
    let token = std::fs::read_to_string(path.join("api-token")).ok()?.trim().to_string();
    
    let api_base = if port_str.contains(':') {
        format!("http://{}", port_str)
    } else {
        format!("http://127.0.0.1:{}", port_str)
    };
    Some((api_base, token))
}

/// A client implementing the Bored protocol via local x0x daemon storage
pub struct X0xBoredClient {
    http: reqwest::Client,
    api_base: String,
    api_token: String,
    agent_id: String,
    current_bored: Option<Bored>,
    draft_notice: Option<Notice>,
    bored_address: Option<BoredAddress>,
}

impl X0xBoredClient {
    /// Initialize the client by discovering local daemon settings and fetching local agent ID.
    pub async fn init() -> Result<X0xBoredClient, BoredError> {
        let (api_base, api_token) = match get_api_credentials() {
            Some(creds) => creds,
            None => ("http://127.0.0.1:12700".to_string(), String::new()),
        };

        let http = reqwest::Client::new();

        // Validate local daemon is running and reachable
        let health_url = format!("{}/health", api_base);
        let mut request = http.get(&health_url);
        if !api_token.is_empty() {
            request = request.bearer_auth(&api_token);
        }

        let resp = match request.send().await {
            Ok(resp) => resp,
            Err(_) => return Err(BoredError::ClientConnectionError),
        };

        if !resp.status().is_success() {
            return Err(BoredError::ClientConnectionError);
        }

        // Fetch local Agent ID
        let agent_url = format!("{}/agent", api_base);
        let mut request = http.get(&agent_url);
        if !api_token.is_empty() {
            request = request.bearer_auth(&api_token);
        }

        let resp = match request.send().await {
            Ok(resp) => resp,
            Err(_) => return Err(BoredError::ClientConnectionError),
        };

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(BoredError::X0xError(err_body));
        }

        let json = resp.json::<serde_json::Value>().await
            .map_err(|e| BoredError::X0xError(format!("Invalid JSON response from /agent: {}", e)))?;
            
        let agent_id = json.get("agent_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BoredError::X0xError("agent_id missing in response".to_string()))?
            .to_string();

        Ok(X0xBoredClient {
            http,
            api_base,
            api_token,
            agent_id,
            current_bored: None,
            draft_notice: None,
            bored_address: None,
        })
    }

    /// Check if the x0xd daemon is up and responsive
    pub async fn check_daemon(&self) -> Result<bool, BoredError> {
        let health_url = format!("{}/health", self.api_base);
        let mut request = self.http.get(&health_url);
        if !self.api_token.is_empty() {
            request = request.bearer_auth(&self.api_token);
        }
        let resp = request.send().await;
        Ok(resp.is_ok() && resp.unwrap().status().is_success())
    }

    /// Is daemon integration initialized successfully
    pub fn is_available(&self) -> bool {
        !self.agent_id.is_empty()
    }

    /// Create a new board by initializing an x0x KV store with topic and metadata
    pub async fn create_bored(
        &mut self,
        name: &str,
        dimensions: Coordinate,
        url_name: Option<&str>,
    ) -> Result<(), BoredError> {
        let address = match url_name {
            None => BoredAddress::new(),
            Some(name) => BoredAddress::from_string(name)?,
        };
        self.bored_address = Some(address.clone());
        let topic = address.get_topic();

        // 1. Create or join KV store
        let url = format!("{}/stores", self.api_base);
        let mut request = self.http.post(&url).json(&serde_json::json!({
            "name": &topic,
            "topic": &topic
        }));
        if !self.api_token.is_empty() {
            request = request.bearer_auth(&self.api_token);
        }

        let resp = match request.send().await {
            Ok(resp) => resp,
            Err(_) => return Err(BoredError::ClientConnectionError),
        };

        if !resp.status().is_success() && url_name.is_some() {
            // Best effort join if already exists
            let join_url = format!("{}/stores/{}/join", self.api_base, topic);
            let mut req = self.http.post(&join_url);
            if !self.api_token.is_empty() {
                req = req.bearer_auth(&self.api_token);
            }
            let _ = req.send().await;
        }

        // 2. Put metadata key
        let meta_value = serde_json::json!({
            "protocol_version": 3,
            "name": name,
            "dimensions": dimensions
        });
        
        let meta_str = serde_json::to_string(&meta_value)?;
        let base64_meta = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, meta_str.as_bytes());

        let put_url = format!("{}/stores/{}/meta", self.api_base, topic);
        let mut request = self.http.put(&put_url).json(&serde_json::json!({
            "value": base64_meta,
            "content_type": "application/json"
        }));
        if !self.api_token.is_empty() {
            request = request.bearer_auth(&self.api_token);
        }

        let resp = match request.send().await {
            Ok(resp) => resp,
            Err(_) => return Err(BoredError::ClientConnectionError),
        };

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(BoredError::X0xError(err_body));
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.refresh_bored().await?;
        Ok(())
    }

    /// Retrieve and enter an existing bored KV store
    pub async fn go_to_bored(&mut self, bored_address: &BoredAddress) -> Result<(), BoredError> {
        let bored_address = bored_address.clone();
        let topic = bored_address.get_topic();

        // Check if store already joined/created in local daemon
        let mut already_joined = false;
        let stores_url = format!("{}/stores", self.api_base);
        let mut request = self.http.get(&stores_url);
        if !self.api_token.is_empty() {
            request = request.bearer_auth(&self.api_token);
        }
        if let Ok(resp) = request.send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(stores) = json.get("stores").and_then(|v| v.as_array()) {
                        for store in stores {
                            if let Some(t) = store.get("topic").and_then(|t| t.as_str()) {
                                if t == topic {
                                    already_joined = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if !already_joined {
            // Join KV store only if not already joined
            let join_url = format!("{}/stores/{}/join", self.api_base, topic);
            let mut request = self.http.post(&join_url);
            if !self.api_token.is_empty() {
                request = request.bearer_auth(&self.api_token);
            }
            let _ = request.send().await;
        }

        let mut last_err = BoredError::NoBored;
        for _ in 0..5 {
            match self.retrieve_bored(&bored_address).await {
                Ok((bored, _)) => {
                    self.current_bored = Some(bored);
                    self.bored_address = Some(bored_address);
                    return Ok(());
                }
                Err(e) => {
                    last_err = e;
                    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                }
            }
        }
        Err(last_err)
    }

    /// Retrieve keys and reconstruct Bored data structure
    pub async fn retrieve_bored(
        &mut self,
        bored_address: &BoredAddress,
    ) -> Result<(Bored, u64), BoredError> {
        let topic = bored_address.get_topic();

        // 1. Fetch keys
        let keys_url = format!("{}/stores/{}/keys", self.api_base, topic);
        let mut request = self.http.get(&keys_url);
        if !self.api_token.is_empty() {
            request = request.bearer_auth(&self.api_token);
        }

        let resp = match request.send().await {
            Ok(resp) => resp,
            Err(_) => return Err(BoredError::ClientConnectionError),
        };

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(BoredError::X0xError(err_body));
        }

        let keys_json = resp.json::<serde_json::Value>().await?;
        let keys = keys_json.get("keys")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        if let Some(s) = v.as_str() {
                            Some(s.to_string())
                        } else {
                            v.get("key").and_then(|k| k.as_str()).map(|s| s.to_string())
                        }
                    })
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        let mut meta_opt: Option<serde_json::Value> = None;
        let mut notices = Vec::new();

        for key in &keys {
            if key == "meta" {
                if let Ok(val) = self.get_store_value(&topic, "meta").await {
                    meta_opt = Some(val);
                }
            } else if key.starts_with("notice:") {
                if let Ok(val) = self.get_store_value(&topic, key).await {
                    if let Ok(notice) = serde_json::from_value::<Notice>(val) {
                        notices.push(notice);
                    }
                }
            }
        }

        let meta = match meta_opt {
            Some(m) => m,
            None => {
                return Err(BoredError::X0xError("No metadata found in the store".to_string()));
            }
        };

        let name = meta.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled Bored").to_string();
        let dims_json = meta.get("dimensions");
        let dimensions = if let Some(dims) = dims_json {
            serde_json::from_value::<Coordinate>(dims.clone()).unwrap_or(Coordinate { x: 120, y: 40 })
        } else {
            Coordinate { x: 120, y: 40 }
        };

        let bored = Bored {
            protocol_version: ProtocolVersion::new(),
            name,
            dimensions,
            notices,
        };

        Ok((bored, keys.len() as u64))
    }

    async fn get_store_value(&self, topic: &str, key: &str) -> Result<serde_json::Value, BoredError> {
        let url = format!("{}/stores/{}/{}", self.api_base, topic, key);
        let mut request = self.http.get(&url);
        if !self.api_token.is_empty() {
            request = request.bearer_auth(&self.api_token);
        }

        let resp = request.send().await?;
        if !resp.status().is_success() {
            return Err(BoredError::X0xError(format!("Failed to retrieve key {}", key)));
        }

        let json = resp.json::<serde_json::Value>().await?;
        let base64_val = json.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
            BoredError::X0xError("Value field missing in x0x response".to_string())
        })?;

        let decoded_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, base64_val)
            .map_err(|e| BoredError::X0xError(format!("Base64 decode error: {}", e)))?;

        let val = serde_json::from_slice(&decoded_bytes)?;
        Ok(val)
    }

    /// Refresh the current bored state from network
    pub async fn refresh_bored(&mut self) -> Result<(), BoredError> {
        let Some(address) = self.bored_address.clone() else {
            return Err(BoredError::NoBored);
        };
        let mut last_err = BoredError::NoBored;
        for _ in 0..5 {
            match self.retrieve_bored(&address).await {
                Ok((bored, _)) => {
                    self.current_bored = Some(bored);
                    return Ok(());
                }
                Err(e) => {
                    last_err = e;
                    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                }
            }
        }
        Err(last_err)
    }

    /// Returns the cached current bored
    pub fn get_current_bored(&self) -> Result<Bored, BoredError> {
        let Some(bored) = self.current_bored.clone() else {
            return Err(BoredError::NoBored);
        };
        Ok(bored)
    }

    /// Get current bored address
    pub fn get_bored_address(&self) -> Result<BoredAddress, BoredError> {
        let Some(bored_address) = &self.bored_address else {
            return Err(BoredError::NoBored);
        };
        Ok(bored_address.clone())
    }

    /// Get current bored name
    pub fn get_bored_name(&self) -> Result<&str, BoredError> {
        let Some(bored) = &self.current_bored else {
            return Err(BoredError::NoBored);
        };
        Ok(&bored.name)
    }

    /// Create a draft notice that fits on the board
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

    /// Get current draft notice
    pub fn get_draft(&self) -> Option<Notice> {
        self.draft_notice.clone()
    }

    /// Write content into draft notice
    pub fn edit_draft(&mut self, content: &str) -> Result<(), BoredError> {
        let Some(_) = &self.current_bored else {
            return Err(BoredError::NoBored);
        };
        if let Some(mut notice) = self.draft_notice.clone() {
            notice.write(content)?;
            self.draft_notice = Some(notice);
        }
        Ok(())
    }

    /// Relocate the draft notice
    pub fn position_draft(&mut self, new_top_left: Coordinate) -> Result<(), BoredError> {
        let Some(bored) = &self.current_bored else {
            return Err(BoredError::NoBored);
        };
        if let Some(mut notice) = self.draft_notice.clone() {
            notice.relocate(bored, new_top_left)?;
            self.draft_notice = Some(notice);
        }
        Ok(())
    }

    /// Write notice and prune fully occluded ones (Option A - explicit deletes)
    pub async fn add_draft_to_bored(&mut self) -> Result<(), BoredError> {
        let Some(bored) = &mut self.current_bored else {
            return Err(BoredError::NoBored);
        };
        let Some(bored_address) = &self.bored_address else {
            return Err(BoredError::NoBored);
        };
        let topic = bored_address.get_topic();

        if let Some(mut notice) = self.draft_notice.clone() {
            // Generate globally unique notice key: notice:<timestamp>:<agent_id_prefix>
            let timestamp = chrono::Utc::now().timestamp_millis();
            let agent_prefix = if self.agent_id.len() >= 8 {
                &self.agent_id[0..8]
            } else {
                "local"
            };
            let notice_key = format!("notice:{}:{}", timestamp, agent_prefix);
            notice.set_notice_id(notice_key.clone());

            // Add locally
            bored.add(notice.clone(), notice.get_top_left())?;

            // Push to x0x
            let serialized = serde_json::to_string(&notice)?;
            let base64_notice = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, serialized.as_bytes());

            let put_url = format!("{}/stores/{}/{}", self.api_base, topic, notice_key);
            let mut request = self.http.put(&put_url).json(&serde_json::json!({
                "value": base64_notice,
                "content_type": "application/json"
            }));
            if !self.api_token.is_empty() {
                request = request.bearer_auth(&self.api_token);
            }

            let resp = request.send().await?;
            if !resp.status().is_success() {
                return Err(BoredError::X0xError(format!("Failed to write notice {}", notice_key)));
            }

            // --- Explicit Pruning (Option A) ---
            let original_notices = bored.notices.clone();
            
            // Prune locally
            bored.prune_non_visible()?;
            
            // Issue DELETE for pruned keys
            for orig in original_notices {
                if !bored.notices.contains(&orig) && !orig.get_notice_id().is_empty() {
                    let del_url = format!("{}/stores/{}/{}", self.api_base, topic, orig.get_notice_id());
                    let mut request = self.http.delete(&del_url);
                    if !self.api_token.is_empty() {
                        request = request.bearer_auth(&self.api_token);
                    }
                    let _ = request.send().await;
                }
            }

            self.draft_notice = None;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.refresh_bored().await?;
        Ok(())
    }

    /// Load standard board
    pub fn load_app_bored(&mut self, bored: Bored) {
        self.current_bored = Some(bored);
        self.bored_address = None;
    }
}

#[cfg(test)]
mod x0x_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_bored_integration() {
        let mut client = X0xBoredClient::init().await.expect("Failed init");
        let unique_suffix = uuid::Uuid::new_v4().to_string()[0..8].to_string();
        let topic = format!("bored.test.integration.{}", unique_suffix);
        let res = client.create_bored("Integration Board", Coordinate { x: 120, y: 40 }, Some(&topic)).await;
        assert!(res.is_ok(), "create_bored failed: {:?}", res);
    }

    #[tokio::test]
    async fn test_go_to_bored_integration() {
        let mut client1 = X0xBoredClient::init().await.expect("Failed init");
        let unique_suffix = uuid::Uuid::new_v4().to_string()[0..8].to_string();
        let topic = format!("bored.test.goto.{}", unique_suffix);
        
        // 1. Create a board with client1
        client1.create_bored("GoTo Board", Coordinate { x: 120, y: 40 }, Some(&topic)).await.expect("create failed");
        let address = client1.get_bored_address().expect("no address");
        
        // 2. Load it with a fresh client2 (already created/joined in daemon)
        let mut client2 = X0xBoredClient::init().await.expect("Failed init");
        let res = client2.go_to_bored(&address).await;
        assert!(res.is_ok(), "go_to_bored failed: {:?}", res);
    }
}


