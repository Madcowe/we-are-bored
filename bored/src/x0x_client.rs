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
use crate::{Bored, BoredError, Coordinate};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
enum GossipMsg {
    #[serde(rename = "meta")]
    Meta {
        name: String,
        dimensions: Coordinate,
    },
    #[serde(rename = "notice")]
    NoticeMsg {
        notice: Notice,
    },
    #[serde(rename = "sync-request")]
    SyncRequest,
    #[serde(rename = "sync-response")]
    SyncResponse {
        name: String,
        dimensions: Coordinate,
        notices: Vec<Notice>,
    },
}

pub fn get_x0x_data_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Some(std::path::PathBuf::from(home).join("Library/Application Support/x0x"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return Some(std::path::PathBuf::from(local_app_data).join("x0x"));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            return Some(std::path::PathBuf::from(user_profile).join("AppData/Local/x0x"));
        }
    }

    // Default Linux & Android/Termux
    if let Ok(home) = std::env::var("HOME") {
        return Some(std::path::PathBuf::from(home).join(".local/share/x0x"));
    }

    None
}

pub fn get_we_are_bored_data_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Some(std::path::PathBuf::from(home).join("Library/Application Support/we-are-bored"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return Some(std::path::PathBuf::from(local_app_data).join("we-are-bored"));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            return Some(std::path::PathBuf::from(user_profile).join("AppData/Local/we-are-bored"));
        }
    }

    // Default Linux & Android/Termux
    if let Ok(home) = std::env::var("HOME") {
        return Some(std::path::PathBuf::from(home).join(".local/share/we-are-bored"));
    }

    None
}

fn get_api_credentials() -> Option<(String, String)> {
    let path = get_x0x_data_dir()?;
    let port_str = std::fs::read_to_string(path.join("api.port")).ok()?.trim().to_string();
    let token = std::fs::read_to_string(path.join("api-token")).ok()?.trim().to_string();
    
    let api_base = if port_str.contains(':') {
        format!("http://{}", port_str)
    } else {
        format!("http://127.0.0.1:{}", port_str)
    };
    Some((api_base, token))
}

/// A client implementing the Bored protocol via gossip pub/sub and local caching
pub struct X0xBoredClient {
    http: reqwest::Client,
    api_base: String,
    api_token: String,
    agent_id: String,
    current_bored: Option<Bored>,
    draft_notice: Option<Notice>,
    bored_address: Option<BoredAddress>,
    cache_dir: std::path::PathBuf,
}

impl X0xBoredClient {
    /// Initialize the client by discovering local daemon settings, fetching local agent ID,
    /// and starting a persistent background loop to process synchronization events.
    pub async fn init() -> Result<X0xBoredClient, BoredError> {
        let (api_base, api_token) = match get_api_credentials() {
            Some(creds) => creds,
            None => ("http://127.0.0.1:12700".to_string(), String::new()),
        };

        let http = reqwest::Client::new();

        // Validate local daemon is running and reachable
        let health_url = format!("{}/health", api_base);
        let mut request = http.get(&health_url).timeout(std::time::Duration::from_secs(5));
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
        let mut request = http.get(&agent_url).timeout(std::time::Duration::from_secs(5));
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

        let cache_dir = get_we_are_bored_data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);

        // Spawn background listener task to monitor all `/events` (gossip updates)
        let http_clone = http.clone();
        let api_base_clone = api_base.clone();
        let api_token_clone = api_token.clone();
        let cache_dir_clone = cache_dir.clone();

        tokio::spawn(async move {
            let mut buffer = String::new();
            loop {
                let url = format!("{}/events", api_base_clone);
                let mut request = http_clone.get(&url);
                if !api_token_clone.is_empty() {
                    request = request.bearer_auth(&api_token_clone);
                }

                let resp = match request.send().await {
                    Ok(resp) => resp,
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        continue;
                    }
                };

                if !resp.status().is_success() {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                }

                let mut resp = resp;
                loop {
                    match resp.chunk().await {
                        Ok(Some(chunk)) => {
                            if let Ok(s) = std::str::from_utf8(&chunk) {
                                buffer.push_str(s);
                                while let Some(pos) = buffer.find('\n') {
                                    let line = buffer[..pos].trim().to_string();
                                    buffer = buffer[pos + 1..].to_string();

                                    if line.starts_with("data:") {
                                        let data_str = line["data:".len()..].trim();
                                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data_str) {
                                            let data_obj = json.get("data").unwrap_or(&json);
                                            if let (Some(topic), Some(payload_base64)) = (
                                                data_obj.get("topic").and_then(|v| v.as_str()),
                                                data_obj.get("payload").and_then(|v| v.as_str())
                                            ) {
                                                if let Ok(decoded) = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, payload_base64) {
                                                    if let Ok(msg) = serde_json::from_slice::<GossipMsg>(&decoded) {
                                                        let _ = Self::handle_background_msg(
                                                            &http_clone,
                                                            &api_base_clone,
                                                            &api_token_clone,
                                                            &cache_dir_clone,
                                                            topic,
                                                            msg
                                                        ).await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            // Connection dropped or finished; reconnect
                            break;
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        Ok(X0xBoredClient {
            http,
            api_base,
            api_token,
            agent_id,
            current_bored: None,
            draft_notice: None,
            bored_address: None,
            cache_dir,
        })
    }

    /// Check if the x0xd daemon is up and responsive
    pub async fn check_daemon(&self) -> Result<bool, BoredError> {
        let health_url = format!("{}/health", self.api_base);
        let mut request = self.http.get(&health_url).timeout(std::time::Duration::from_secs(5));
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

    fn cache_path(cache_dir: &std::path::Path, address: &BoredAddress) -> std::path::PathBuf {
        let filename = format!("{}.json", address.get_topic());
        cache_dir.join(filename)
    }

    fn load_cache(cache_dir: &std::path::Path, address: &BoredAddress) -> Option<Bored> {
        let path = Self::cache_path(cache_dir, address);
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    fn save_cache(cache_dir: &std::path::Path, address: &BoredAddress, bored: &Bored) -> Result<(), BoredError> {
        let path = Self::cache_path(cache_dir, address);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = serde_json::to_string(bored)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> Result<(), BoredError> {
        let url = format!("{}/subscribe", self.api_base);
        let mut request = self.http.post(&url).timeout(std::time::Duration::from_secs(5)).json(&serde_json::json!({
            "topic": topic
        }));
        if !self.api_token.is_empty() {
            request = request.bearer_auth(&self.api_token);
        }
        let _ = request.send().await;
        Ok(())
    }

    async fn publish_msg(&self, topic: &str, msg: &GossipMsg) -> Result<(), BoredError> {
        let serialized = serde_json::to_string(msg)?;
        let base64_payload = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, serialized.as_bytes());

        let url = format!("{}/publish", self.api_base);
        let mut request = self.http.post(&url).timeout(std::time::Duration::from_secs(5)).json(&serde_json::json!({
            "topic": topic,
            "payload": base64_payload
        }));
        if !self.api_token.is_empty() {
            request = request.bearer_auth(&self.api_token);
        }

        let resp = request.send().await?;
        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(BoredError::X0xError(err_body));
        }
        Ok(())
    }

    async fn handle_background_msg(
        http: &reqwest::Client,
        api_base: &str,
        api_token: &str,
        cache_dir: &std::path::Path,
        topic: &str,
        msg: GossipMsg,
    ) -> Result<(), BoredError> {
        // topic is usually "bored.bum"
        let name = if topic.starts_with("bored.") {
            &topic["bored.".len()..]
        } else {
            topic
        };
        let address = BoredAddress::from_string(name)?;

        // Only process events if we have joined/created the board (indicated by cache existence),
        // or if the message is a SyncResponse (which we can use to discover/join a board from the network).
        let path = Self::cache_path(cache_dir, &address);
        if !path.exists() {
            if !matches!(msg, GossipMsg::SyncResponse { .. }) {
                return Ok(());
            }
        }

        match msg {
            GossipMsg::SyncRequest => {
                if let Some(bored) = Self::load_cache(cache_dir, &address) {
                    let response_msg = GossipMsg::SyncResponse {
                        name: bored.get_name().to_string(),
                        dimensions: bored.get_dimensions(),
                        notices: bored.get_notices(),
                    };
                    let serialized = serde_json::to_string(&response_msg)?;
                    let base64_payload = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, serialized.as_bytes());

                    let url = format!("{}/publish", api_base);
                    let mut request = http.post(&url).timeout(std::time::Duration::from_secs(5)).json(&serde_json::json!({
                        "topic": topic,
                        "payload": base64_payload
                    }));
                    if !api_token.is_empty() {
                        request = request.bearer_auth(api_token);
                    }
                    let _ = request.send().await;
                }
            }
            GossipMsg::Meta { name, dimensions } => {
                if let Some(mut bored) = Self::load_cache(cache_dir, &address) {
                    if bored.name == "Untitled Bored" || bored.name == address.get_topic() {
                        bored.name = name;
                        bored.dimensions = dimensions;
                        Self::save_cache(cache_dir, &address, &bored)?;
                    }
                }
            }
            GossipMsg::NoticeMsg { notice } => {
                if let Some(mut bored) = Self::load_cache(cache_dir, &address) {
                    let already_exists = bored.notices.iter().any(|n| n.get_notice_id() == notice.get_notice_id());
                    if !already_exists {
                        let _ = bored.add(notice.clone(), notice.get_top_left());
                        let _ = bored.prune_non_visible();
                        Self::save_cache(cache_dir, &address, &bored)?;
                    }
                }
            }
            GossipMsg::SyncResponse { name, dimensions, notices } => {
                let mut bored = if let Some(bored) = Self::load_cache(cache_dir, &address) {
                    bored
                } else {
                    Bored::create(&name, dimensions)
                };
                let mut changed = false;
                if bored.name == "Untitled Bored" || bored.name == address.get_topic() {
                    if name != "Untitled Bored" && name != address.get_topic() {
                        bored.name = name;
                        bored.dimensions = dimensions;
                        changed = true;
                    }
                }
                for notice in notices {
                    let already_exists = bored.notices.iter().any(|n| n.get_notice_id() == notice.get_notice_id());
                    if !already_exists {
                        let _ = bored.add(notice.clone(), notice.get_top_left());
                        changed = true;
                    }
                }
                let is_new = !Self::cache_path(cache_dir, &address).exists();
                if changed || is_new {
                    let _ = bored.prune_non_visible();
                    Self::save_cache(cache_dir, &address, &bored)?;
                }
            }
        }

        Ok(())
    }

    /// Create a new board by subscribing to topic and initializing cache
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

        self.subscribe(&topic).await?;

        let bored = Bored::create(name, dimensions);
        self.current_bored = Some(bored.clone());

        Self::save_cache(&self.cache_dir, &address, &bored)?;

        let meta_msg = GossipMsg::Meta {
            name: name.to_string(),
            dimensions,
        };
        self.publish_msg(&topic, &meta_msg).await?;

        Ok(())
    }

    /// Retrieve and enter an existing bored topic
    pub async fn go_to_bored(&mut self, bored_address: &BoredAddress) -> Result<(), BoredError> {
        let bored_address = bored_address.clone();
        let topic = bored_address.get_topic();

        self.subscribe(&topic).await?;
        self.bored_address = Some(bored_address.clone());

        let cache_exists = Self::cache_path(&self.cache_dir, &bored_address).exists();
        if !cache_exists {
            // Publish a SyncRequest so any online peers reply with their visible notices
            self.publish_msg(&topic, &GossipMsg::SyncRequest).await?;

            // Sleep and poll to let the background thread receive and merge the SyncResponse into the cache file
            let mut found = false;
            for _ in 0..10 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if Self::cache_path(&self.cache_dir, &bored_address).exists() {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(BoredError::BoardDoesNotExist(bored_address.to_string()));
            }
        } else {
            // It is cached, but let's publish a SyncRequest to get any new updates from peers in the background
            let _ = self.publish_msg(&topic, &GossipMsg::SyncRequest).await;
            // Sleep briefly to let the background thread receive and merge the SyncResponse into the cache file
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Now load from the cache
        if let Some(bored) = Self::load_cache(&self.cache_dir, &bored_address) {
            self.current_bored = Some(bored);
            Ok(())
        } else {
            Err(BoredError::NoBored)
        }
    }

    /// Retrieve and process gossip events for Bored Address
    pub async fn retrieve_bored(
        &mut self,
        bored_address: &BoredAddress,
    ) -> Result<(Bored, u64), BoredError> {
        if let Some(ref bored) = self.current_bored {
            Ok((bored.clone(), bored.get_notices().len() as u64))
        } else if let Some(bored) = Self::load_cache(&self.cache_dir, bored_address) {
            self.current_bored = Some(bored.clone());
            Ok((bored.clone(), bored.get_notices().len() as u64))
        } else {
            Err(BoredError::NoBored)
        }
    }

    /// Refresh the current bored state from network
    pub async fn refresh_bored(&mut self) -> Result<(), BoredError> {
        let Some(address) = self.bored_address.clone() else {
            return Err(BoredError::NoBored);
        };
        let topic = address.get_topic();

        let _ = self.publish_msg(&topic, &GossipMsg::SyncRequest).await;
        
        // Sleep briefly to let background thread catch and write any new notices
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        if let Some(bored) = Self::load_cache(&self.cache_dir, &address) {
            self.current_bored = Some(bored);
            Ok(())
        } else {
            Err(BoredError::NoBored)
        }
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

    /// Write notice and publish via gossip message
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
            notice.set_notice_id(notice_key);

            // Add locally
            bored.add(notice.clone(), notice.get_top_left())?;
            bored.prune_non_visible()?;

            // Save cache
            Self::save_cache(&self.cache_dir, bored_address, bored)?;

            // Publish notice via gossip Msg
            let notice_msg = GossipMsg::NoticeMsg {
                notice: notice.clone(),
            };
            self.publish_msg(&topic, &notice_msg).await?;

            self.draft_notice = None;
        }

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
        
        // 2. Load it with a fresh client2
        let mut client2 = X0xBoredClient::init().await.expect("Failed init");
        let res = client2.go_to_bored(&address).await;
        assert!(res.is_ok(), "go_to_bored failed: {:?}", res);
    }

    #[tokio::test]
    async fn test_go_to_bored_non_existent() {
        let mut client = X0xBoredClient::init().await.expect("Failed init");
        let unique_suffix = uuid::Uuid::new_v4().to_string()[0..8].to_string();
        let topic = format!("bored.test.nonexistent.{}", unique_suffix);
        let address = BoredAddress::from_string(&topic).expect("invalid address");

        // Force delete cache file if it somehow exists
        let cache_path = X0xBoredClient::cache_path(&client.cache_dir, &address);
        let _ = std::fs::remove_file(cache_path);

        let res = client.go_to_bored(&address).await;
        assert!(matches!(res, Err(BoredError::BoardDoesNotExist(_))), "expected BoardDoesNotExist, got: {:?}", res);
    }
}
