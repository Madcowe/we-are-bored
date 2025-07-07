use crate::app::SurfBoredError;
use bored::Bored;
use serde::{Deserialize, Serialize};
use std::fs;

/// The directory of boreds...list of bored the user has saved for future reference
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Directory {
    bored_addresses: Vec<Listing>,
    home_bored: usize, // indicates which bored is the home bored
}
impl Directory {
    pub fn new() -> Directory {
        Directory {
            bored_addresses: vec![],
            home_bored: 0,
        }
    }

    pub fn load_file(path: &str) -> Result<Directory, SurfBoredError> {
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

    pub fn add(&mut self, listing: Listing, path: &str) -> Result<(), SurfBoredError> {
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
    pub fn new() -> History {
        History {
            boreds: vec![],
            current_position: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Listing {
    pub name: String,
    pub bored_address: String,
}
