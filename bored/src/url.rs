use crate::BoredError;
use autonomi::{Bytes, SecretKey, client::key_derivation::DerivationIndex, data::DataAddress};
use std::fmt::{self};

/// Hex key of base (not) secret key of bored derive names
const BORED_DERIVED_NAME_BASE: &str =
    "000000000000000000000000000000000000000000000000000000000020D5B0";

/// The address of a bored, currently this can only be autonomi::private key that has been
/// used to create a scratchpad with a bored stored in it.
/// Hence this means anyone who has the address can update the board which probalby won't
/// be sensible in a long term project but this is an experiment so starting with the
/// most basic level of a human trust network seems appropriate, you share it you bare it!
#[derive(Clone, Debug, PartialEq)]
pub enum BoredAddress {
    ScratchpadKey(autonomi::SecretKey),
    DerivedName(String),
}
impl fmt::Display for BoredAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            BoredAddress::ScratchpadKey(key) => write!(f, "bored://{}", key.to_hex()),
            BoredAddress::DerivedName(name) => write!(f, "bored://{}", name),
        }
    }
}

impl BoredAddress {
    /// Generates a new BoredAdress, ie a new random autonomi secrkey encapsulated inside this enum
    pub fn new() -> BoredAddress {
        BoredAddress::ScratchpadKey(autonomi::SecretKey::random())
    }

    /// Tries to create bored URL from string, will remove protocol part of URL if it exists
    /// attempt to create with that text, fails if it doesn't make valid secret key
    /// this doesn't neccsiarily imply it is an existing bored address
    pub fn from_string(s: &str) -> Result<Self, BoredError> {
        let mut s = s.trim();

        if let Some(bored) = &s.get(0..8) {
            if *bored == "bored://" {
                s = &s[8..s.len()];
            }
        }
        if let Ok(key) = SecretKey::from_hex(&s) {
            return Ok(BoredAddress::ScratchpadKey(key));
        } else if let Ok(_) = derive_key_from_name(&s) {
            return Ok(BoredAddress::DerivedName(s.to_string()));
        }
        Err(BoredError::NotBoredURL(s.to_string()))
    }

    pub fn get_key(&self) -> Result<autonomi::SecretKey, BoredError> {
        match &self {
            BoredAddress::ScratchpadKey(key) => Ok(key.to_owned()),
            BoredAddress::DerivedName(name) => {
                let key = derive_key_from_name(&name)?;
                Ok(key)
            }
        }
    }

    pub fn get_public_key(&self) -> Result<autonomi::PublicKey, BoredError> {
        let key = self.get_key()?;
        Ok(key.public_key())
    }
}

/// BoredApp is holds a command that may be handled by a client application so that it can
/// represent information of an interface via a bored
#[derive(Clone, Debug, PartialEq)]
pub enum URL {
    BoredNet(BoredAddress),
    BoredApp(String),
    ClearNet(String),
    AntNet(DataAddress),
}

impl URL {
    pub fn from_string(s: String) -> Result<Self, BoredError> {
        let s = s.trim();
        if s.len() > 7 {
            if &s[0..8] == "https://" || &s[0..7] == "http://" {
                return Ok(URL::ClearNet(s.to_string()));
            } else if &s[0..6] == "ant://" && s.len() == 70 {
                let s = &s[6..s.len()];
                if let Ok(data_address) = DataAddress::from_hex(s) {
                    return Ok(URL::AntNet(data_address));
                }
            } else if &s[0..6] == "app://" {
                return Ok(URL::BoredApp(s[6..s.len()].to_string()));
            // check bored last as will also try sans protocol identifier
            } else if let Ok(bored_address) = BoredAddress::from_string(&s) {
                return Ok(URL::BoredNet(bored_address));
            }
        }
        Err(BoredError::UnknownURLType(s.to_string()))
    }
}

pub fn derive_key_from_name(derived_name: &str) -> Result<SecretKey, BoredError> {
    if derived_name.replace(".", "").len() == 0 {
        return Err(BoredError::NotBoredURL(derived_name.to_string()));
    }
    let domains = derived_name.split('.');
    let mut key = SecretKey::from_hex(&BORED_DERIVED_NAME_BASE)?;
    for domain in domains {
        // don't make domain for empty string
        if domain.len() > 0 {
            key = key.derive_child(&Bytes::copy_from_slice(domain.as_bytes()));
        }
    }
    Ok(key)
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn test_bored_address_display() {
        let bored_address = BoredAddress::new();
        if let BoredAddress::ScratchpadKey(ref scratchpad_key) = bored_address {
            assert_eq!(
                format!("bored://{}", scratchpad_key.to_hex()),
                format!("{}", bored_address)
            );
        } else {
            panic!("new method didn't create ScratchpadKey variant")
        }
    }

    #[test]
    fn test_bored_address_from_string() {
        let bored_address = BoredAddress::from_string("");
        assert_eq!(bored_address, Err(BoredError::NotBoredURL("".to_string())));
        let string =
            "bored://2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330".to_string();
        let bored_address = BoredAddress::from_string(&string);
        assert_eq!(
            bored_address.unwrap().get_key().unwrap().to_hex(),
            "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330"
        );
        let string = "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330".to_string();
        let bored_address = BoredAddress::from_string(&string);
        assert_eq!(
            bored_address.unwrap().get_key().unwrap().to_hex(),
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
            URL::BoredNet(bored_address) => bored_address.get_key().unwrap().clone(),
            _ => SecretKey::from_hex("00000000000000000000000000000000").unwrap(),
        };
        assert_eq!(
            secret_key.to_hex(),
            "2f67b46da5e6d62c07fb97889c7e7155ca7e1fd3efb711a5468eeda8e1501330"
        );
        let url = URL::from_string("app://about".to_string()).unwrap();
        assert_eq!(url, URL::BoredApp("about".to_string()));
        let url = URL::from_string("https://autonomi.com".to_string()).unwrap();
        assert_eq!(url, URL::ClearNet("https://autonomi.com".to_string()));
        let url = URL::from_string("http://www.bbsdocumentary.com/".to_string()).unwrap();
        assert_eq!(
            url,
            URL::ClearNet("http://www.bbsdocumentary.com/".to_string())
        );
        let url_result = URL::from_string("not a url".to_string());
        // assert_eq!(
        //     url_result,
        //     Err(BoredError::UnknownURLType("not a url".to_string()))
        // );
        // currently any string would make a derive name url as ones without any valid protocol
        // identfiers will eventually get tried as derived name and work...maybe this should
        // be changed but for time beign test adjuted to below.
        assert_eq!(
            url_result,
            Ok(URL::BoredNet(BoredAddress::DerivedName(
                "not a url".to_string()
            )))
        );
        let url_result = URL::from_string("".to_string());
        assert_eq!(url_result, Err(BoredError::UnknownURLType("".to_string())));
        let url = URL::from_string(
            "ant://a7d2fdbb975efaea25b7ebe3d38be4a0b82c1d71e9b89ac4f37bc9f8677826e0".to_string(),
        )
        .unwrap();
        let data_address = DataAddress::from_hex(
            "a7d2fdbb975efaea25b7ebe3d38be4a0b82c1d71e9b89ac4f37bc9f8677826e0",
        )
        .unwrap();
        assert_eq!(url, URL::AntNet(data_address));
        let url_result = URL::from_string(
            "ant://a7d2fdbb975efaea25b7ebe3d38be4a0b82c1d71e9b89ac4f37bc9f8677826e".to_string(),
        );
        // currently any string would make a derive name url as ones without any valid protocol
        // identfiers will eventually get tried as derived name and work...maybe this should
        // be changed but for time beign test adjuted to below.
        assert_eq!(
            url_result,
            Ok(URL::BoredNet(BoredAddress::DerivedName(
                "ant://a7d2fdbb975efaea25b7ebe3d38be4a0b82c1d71e9b89ac4f37bc9f8677826e".to_string()
            )))
        );
        // assert!(url_result.is_err());
        let url_result = URL::from_string(
            "ant://a7d2fdbb975efaea25b7ebe3d38be4a0b82c1d71e9b89ac4f37bc9f8677826e00".to_string(),
        );
        // currently any string would make a derive name url as ones without any valid protocol
        // identfiers will eventually get tried as derived name and work...maybe this should
        // be changed but for time beign test adjuted to below.
        assert_eq!(
            url_result,
            Ok(URL::BoredNet(BoredAddress::DerivedName(
                "ant://a7d2fdbb975efaea25b7ebe3d38be4a0b82c1d71e9b89ac4f37bc9f8677826e00"
                    .to_string()
            )))
        );
        // assert!(url_result.is_err());
    }

    #[test]
    fn test_derive_key_from_name() {
        let key = derive_key_from_name("in.the.domain.of.the.names").unwrap();
        assert_eq!(
            SecretKey::from_hex("19433cecc0dad585849f83a257b91ffb582dc77f50191865ec7d52c3469a0554")
                .unwrap(),
            key
        );
    }
}
