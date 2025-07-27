use crate::BoredError;
use autonomi::SecretKey;
use std::fmt::{self};

/// The address of a bored, currently this can only be autonomi::private key that has been
/// used to create a scratchpad with a bored stored in it.
/// Hence this means anyone who has the address can update the board which probalby won't
/// be sensible in a long term project but this is an experiment so starting with the
/// most basic level of a human trust network seems appropriate, you share it you bare it!
#[derive(Clone, Debug, PartialEq)]
pub enum BoredAddress {
    ScratchpadKey(autonomi::SecretKey),
}
impl fmt::Display for BoredAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            BoredAddress::ScratchpadKey(key) => write!(f, "bored://{}", key.to_hex()),
        }
    }
}

impl BoredAddress {
    /// Generates a new BoredAdress, ie a new autonomi secrkey encapsulated inside this enum
    pub fn new() -> BoredAddress {
        BoredAddress::ScratchpadKey(autonomi::SecretKey::random())
    }

    /// Tries to create bored URL from string, will remove protocol part of URL if it exists
    /// attempt to create with that text, fails if it doesn't make valid secret key
    /// this doesn't neccsiarily imply it is an existing bored address
    pub fn from_string(s: &str) -> Result<Self, BoredError> {
        let mut s = s.trim();
        if s.len() == 72 {
            if &s[0..8] == "bored://" {
                s = &s[8..s.len()];
            }
        }
        let key = match SecretKey::from_hex(&s) {
            Ok(key) => key,
            Err(_) => return Err(BoredError::NotBoredURL(s.to_string())),
        };
        Ok(BoredAddress::ScratchpadKey(key))
    }

    pub fn get_key(&self) -> &autonomi::SecretKey {
        let BoredAddress::ScratchpadKey(key) = self;
        key
    }

    pub fn get_public_key(&self) -> autonomi::PublicKey {
        match self {
            BoredAddress::ScratchpadKey(key) => key.public_key(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum URL {
    BoredNet(BoredAddress),
    ClearNet(String),
}

impl URL {
    pub fn from_string(s: String) -> Result<Self, BoredError> {
        let s = s.trim();
        if s.len() > 7 {
            if let Ok(bored_address) = BoredAddress::from_string(&s) {
                return Ok(URL::BoredNet(bored_address));
            } else if &s[0..8] == "https://" || &s[0..7] == "http://" {
                return Ok(URL::ClearNet(s.to_string()));
            }
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
        let BoredAddress::ScratchpadKey(ref scratchpad_key) = bored_address;
        assert_eq!(
            format!("bored://{}", scratchpad_key.to_hex()),
            format!("{}", bored_address)
        );
    }

    #[test]
    fn test_bored_address_from_string() {
        let bored_address = BoredAddress::from_string("");
        assert_eq!(bored_address, Err(BoredError::NotBoredURL("".to_string())));
        let string =
            "bored://2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330".to_string();
        let bored_address = BoredAddress::from_string(&string);
        assert_eq!(
            bored_address.unwrap().get_key().to_hex(),
            "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330"
        );
        let string = "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330".to_string();
        let bored_address = BoredAddress::from_string(&string);
        assert_eq!(
            bored_address.unwrap().get_key().to_hex(),
            "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330"
        );
    }

    #[test]
    fn test_url_from_string() {
        let url = URL::from_string(
            "bored://2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330".to_string(),
        )
        .unwrap();
        let secret_key = match url {
            URL::BoredNet(bored_address) => bored_address.get_key().clone(),
            _ => SecretKey::from_hex("00000000000000000000000000000000").unwrap(),
        };
        assert_eq!(
            secret_key.to_hex(),
            "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330"
        );
        let url = URL::from_string("https://autonomi.com".to_string()).unwrap();
        assert_eq!(url, URL::ClearNet("https://autonomi.com".to_string()));
        let url = URL::from_string("http://www.bbsdocumentary.com/".to_string()).unwrap();
        assert_eq!(
            url,
            URL::ClearNet("http://www.bbsdocumentary.com/".to_string())
        );
        let url_result = URL::from_string("not a url".to_string());
        assert_eq!(
            url_result,
            Err(BoredError::UnknownURLType("not a url".to_string()))
        );
        let url_result = URL::from_string("".to_string());
        assert_eq!(url_result, Err(BoredError::UnknownURLType("".to_string())));
    }
}
