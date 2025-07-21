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

    pub fn default() -> Directory {
        let mut directory = Directory::new();
        let listing = Listing::new(
            "The genesis bored",
            "bored://00a477e5e70ba4c1b8943db7b68c848f358a6059c8b6c514b5c47fcaeacbb3d0",
        );
        directory.bored_addresses.push(listing);
        let listing = Listing::new(
            "Bored of Phil Collins",
            "bored://0ea6aa74936fcf6afc1fcd75391b7bcbcf26a20e7e2c50583f8f1d61dc9fa28a",
        );
        directory.bored_addresses.push(listing);
        directory
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

    pub fn save_file(&self, path: &str) -> Result<(), SurfBoredError> {
        if let Ok(directory_string) = toml::to_string(&self) {
            let Ok(()) = fs::write(path, &directory_string) else {
                return Err(SurfBoredError::DirectoryFileWriteError);
            };
        } else {
            return Err(SurfBoredError::DirectorySerialzationError);
        }
        Ok(())
    }

    pub fn add(&mut self, listing: Listing, path: &str) -> Result<(), SurfBoredError> {
        self.bored_addresses.push(listing);
        // this is only for convience in testing remove once working
        self.home_bored = self.bored_addresses.len() - 1;
        self.save_file(path)?;
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
impl Listing {
    pub fn new(name: &str, bored_address: &str) -> Listing {
        Listing {
            name: name.to_string(),
            bored_address: bored_address.to_string(),
        }
    }
}
