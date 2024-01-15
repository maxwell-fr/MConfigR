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
    const MAX_KEY_LEN: usize = u8::MAX as usize;
    const MAX_VALUE_LEN: usize = u8::MAX as usize;


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
            assert!(entry_k.len() <= MConfig::MAX_KEY_LEN);

            v.push(entry_k.len() as u8);
            v.append(&mut entry_k.as_bytes().to_vec());

            if let Some(val) = entry_v {
                assert!(val.len() <= MConfig::MAX_VALUE_LEN);
                v.push(val.len() as u8);
                v.append(&mut val.as_bytes().to_vec());
            }
            else {
                v.push(0);
            }
        }
        v.push(0);
        assert!(v.len() <= MConfig::MCONFIG_SIZE);

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

    pub fn try_get(&self, key: &String) -> MCResult<&Option<String>> {
        self.entries.get(key).ok_or(())
    }

    pub fn get(&self, key: &String) -> Option<&Option<String>> {
        self.entries.get(key)
    }

    pub fn contains_key(&self, key: &String) -> bool {
        self.entries.contains_key(key)
    }

    pub fn remove(&mut self, key: &String) -> Option<Option<String>> {
        self.entries.remove(key)
    }


}

impl TryFrom<Vec<u8>> for MConfig {
    type Error = ();

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        todo!()
    }
}

