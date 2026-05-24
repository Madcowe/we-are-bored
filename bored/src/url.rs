use crate::BoredError;
use std::fmt::{self};

/// The address of a bored, now represented as an x0x store topic.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BoredAddress {
    Topic(String),
    DerivedName(String),
}

impl fmt::Display for BoredAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            BoredAddress::Topic(topic) => write!(f, "bored://{}", topic),
            BoredAddress::DerivedName(name) => write!(f, "bored://{}", name),
        }
    }
}

impl BoredAddress {
    /// Generates a new random BoredAddress (Topic)
    pub fn new() -> BoredAddress {
        let id = uuid::Uuid::new_v4().to_string();
        BoredAddress::Topic(format!("bored.{}", id))
    }

    /// Tries to create bored URL from string
    pub fn from_string(s: &str) -> Result<Self, BoredError> {
        let mut s = s.trim();

        if let Some(prefix) = s.get(0..8) {
            if prefix == "bored://" {
                s = &s[8..];
            }
        }
        if s.is_empty() {
            return Err(BoredError::NotBoredURL(s.to_string()));
        }

        if s.starts_with("bored.") {
            return Ok(BoredAddress::Topic(s.to_string()));
        }

        Ok(BoredAddress::DerivedName(s.to_string()))
    }

    /// Get the x0x topic string for this address
    pub fn get_topic(&self) -> String {
        match &self {
            BoredAddress::Topic(topic) => topic.clone(),
            BoredAddress::DerivedName(name) => format!("bored.{}", name),
        }
    }
}

/// A parsed URL that can be handled by a client application
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum URL {
    BoredNet(BoredAddress),
    BoredApp(String),
    ClearNet(String),
}

impl URL {
    pub fn from_string(s: String) -> Result<Self, BoredError> {
        let s = s.trim();
        if s.len() > 7 {
            if &s[0..8] == "https://" || &s[0..7] == "http://" {
                return Ok(URL::ClearNet(s.to_string()));
            } else if &s[0..6] == "app://" {
                return Ok(URL::BoredApp(s[6..].to_string()));
            } else if let Ok(bored_address) = BoredAddress::from_string(s) {
                return Ok(URL::BoredNet(bored_address));
            }
        } else if let Ok(bored_address) = BoredAddress::from_string(s) {
            return Ok(URL::BoredNet(bored_address));
        }
        Err(BoredError::UnknownURLType(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bored_address_display() {
        let bored_address = BoredAddress::new();
        match &bored_address {
            BoredAddress::Topic(topic) => {
                assert_eq!(format!("bored://{}", topic), format!("{}", bored_address));
            }
            _ => panic!("new method didn't create Topic variant"),
        }
    }

    #[test]
    fn test_bored_address_from_string() {
        assert_eq!(
            BoredAddress::from_string(""),
            Err(BoredError::NotBoredURL("".to_string()))
        );
        let bored_address = BoredAddress::from_string("bored://bored.test-uuid").unwrap();
        assert_eq!(bored_address.get_topic(), "bored.test-uuid");

        let bored_address = BoredAddress::from_string("bored.test-uuid").unwrap();
        assert_eq!(bored_address.get_topic(), "bored.test-uuid");

        let bored_address = BoredAddress::from_string("genesis").unwrap();
        assert_eq!(bored_address.get_topic(), "bored.genesis");
    }

    #[test]
    fn test_url_from_string() {
        let url = URL::from_string("bored://bored.test-uuid".to_string()).unwrap();
        assert_eq!(
            url,
            URL::BoredNet(BoredAddress::Topic("bored.test-uuid".to_string()))
        );

        let url = URL::from_string("app://about".to_string()).unwrap();
        assert_eq!(url, URL::BoredApp("about".to_string()));

        let url = URL::from_string("https://autonomi.com".to_string()).unwrap();
        assert_eq!(url, URL::ClearNet("https://autonomi.com".to_string()));

        let url_result = URL::from_string("".to_string());
        assert_eq!(url_result, Err(BoredError::UnknownURLType("".to_string())));
    }
}
