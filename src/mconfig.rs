//! Implementation of the MConfig obfuscated key-value storage library
//!

use rand;

type MCHashMap = std::collections::HashMap<String, Option<String>>;
type MCResult<T> = std::result::Result<T, ()>;


pub struct MConfig {
    version: u8,
    entries: MCHashMap
}

impl MConfig {
    const MAGIC_HEADER_BYTES: [u8; 5] = [0x4d, 0x43, 0x4f, 0x4e, 0x46];
    const MCONFIG_SIZE: usize = 8_192;
    const MAX_KEY_LEN: usize = 255;
    const MAX_VALUE_LEN: usize = 255;


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


        for (entry_k, entry_v) in self.entries.iter() {
            v.push(entry_k.len() as u8);
            v.append(&mut entry_k.as_bytes().to_vec());

            if let Some(val) = entry_v {
                v.push(val.len() as u8);
                v.append(&mut val.as_bytes().to_vec());
            }
            else {
                v.push(0);
            }
        }

        //pad the rest with random
        for _ in v.len() .. MConfig::MCONFIG_SIZE {
            v.push(rand::random::<u8>());
        }

        //todo: the obfuscation of course
        v

    }

    pub fn try_add(&mut self, key: String, value: Option<String>) -> MCResult<Option<String>> {
        if key.len() > MConfig::MAX_KEY_LEN {
            return Err(());
        }
        if let Some(ref val) = value {
            if val.len()> MConfig::MAX_VALUE_LEN {
                return Err(());
            }
        }
        Ok(self.entries.insert(key, value).unwrap_or(None))

    }
}



