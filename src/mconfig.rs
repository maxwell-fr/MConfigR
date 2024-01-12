//! Implementation of the MConfig obfuscated key-value storage library
//!

use rand;
use crate::mcstring::MCString;

type MCHashMap = std::collections::HashMap<MCString, MCString>;

pub struct MConfig {
    version: u8,
    entries: MCHashMap
}

impl MConfig {
    const MAGIC_HEADER_BYTES: [u8; 5] = [0x4d, 0x43, 0x4f, 0x4e, 0x46];
    const MCONFIG_SIZE: usize = 8_192;

    pub fn new() -> MConfig {
        MConfig {
            version: 0,
            entries: MCHashMap::new()
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();

        v.append(&mut MConfig::MAGIC_HEADER_BYTES.to_vec());
        v.push(self.version);

        //pad the rest with random
        for _ in 6..MConfig::MCONFIG_SIZE {
            v.push(rand::random::<u8>());
        }

        v

    }
}



