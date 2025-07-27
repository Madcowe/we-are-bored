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

pub enum URL {
    BoredNet(BoredAddress),
    ClearNet(String),
}

impl URL {
    pub fn from_string(s: String) -> Result<Self, BoredError> {
        let s = s.trim();
        if let Ok(bored_address) = BoredAddress::from_string(&s) {
            return Ok(URL::BoredNet(bored_address));
        } else if &s[0..8] == "https://" || &s[0..7] == "http://" {
            return Ok(URL::ClearNet(s.to_string()));
        }
        Err(BoredError::UnknownURLType(s.to_string()))
    }
}
