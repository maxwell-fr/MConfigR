//! Implementation of the MConfig obfuscated key-value storage library
//!

use rand;
use crate::mcstring::MCString;

type MCHashMap = std::collections::HashMap<MCString, Option<MCString>>;
type MCResult = std::result::Result<(), ()>;


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
        let mut byte_count: u32 = 0;

        v.append(&mut MConfig::MAGIC_HEADER_BYTES.to_vec());
        v.push(self.version);
        byte_count += 6;


        for (entry_k, entry_v) in self.entries.iter() {
            v.push(entry_k.len());
            byte_count = byte_count + entry_k.len() as u32;
            //todo: need to have MCString convert to Vec

            if let Some(val) = entry_v {
                v.push(val.len());
                byte_count = byte_count + val.len() as u32;
                //todo: MCString conversion to vec
            }
            else {
                v.push(0);
            }
        }

        //pad the rest with random
        for _ in byte_count as usize..MConfig::MCONFIG_SIZE {
            v.push(rand::random::<u8>());
        }

        //todo: the obfuscation of course
        v

    }

    pub fn try_add(&mut self, key: MCString, value: Option<MCString>) -> MCResult {
        self.entries.insert(key, value);

        //todo: check maximum length?

        Ok(())

    }
}



