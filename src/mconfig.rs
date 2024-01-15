//! Implementation of the MConfig obfuscated key-value storage library
//!

use rand;

type MCHashMap = std::collections::HashMap<String, Option<String>>;
type MCResult<T> = std::result::Result<T, ()>;

pub struct MConfig {
    version: u8,
    entries: MCHashMap,
    secret: Option<String>,
}

impl MConfig {
    const MAGIC_HEADER_BYTES: [u8; 5] = [0x4d, 0x43, 0x4f, 0x4e, 0x46];
    const HEADER_SIZE: usize = MConfig::MAGIC_HEADER_BYTES.len() + 1;
    const MCONFIG_SIZE: usize = 8_192;
    const MAX_KEY_LEN: usize = u8::MAX as usize;
    const MAX_VALUE_LEN: usize = u8::MAX as usize;

    pub fn new(secret: &str) -> MConfig {
        MConfig {
            version: 0,
            entries: MCHashMap::new(),
            secret: Some(secret.to_string()),
        }
    }

    pub fn without_secret() -> MConfig {
        MConfig {
            version: 0,
            entries: MCHashMap::new(),
            secret: None,
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.obfuscate()
    }

    fn to_raw_vec(&self) -> Vec<u8> {
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
            } else {
                v.push(0);
            }
        }
        v.push(0);
        assert!(v.len() <= MConfig::MCONFIG_SIZE);

        //pad the rest with random
        for _ in v.len()..MConfig::MCONFIG_SIZE {
            v.push(rand::random::<u8>());
        }

        v
    }

    pub fn try_add(&mut self, key: String, value: Option<String>) -> MCResult<Option<String>> {
        if key.len() > MConfig::MAX_KEY_LEN {
            return Err(());
        }
        if let Some(ref val) = value {
            if val.len() > MConfig::MAX_VALUE_LEN {
                return Err(());
            }
        }
        Ok(self.entries.insert(key, value).unwrap_or(None))
    }

    pub fn try_get(&self, key: &str) -> MCResult<&Option<String>> {
        self.entries.get(key).ok_or(())
    }

    pub fn get(&self, key: &str) -> Option<&Option<String>> {
        self.entries.get(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<Option<String>> {
        self.entries.remove(key)
    }

    fn obfuscate(&self) -> Vec<u8> {
        match self.secret {
            Some(ref secret) => MConfig::xor_buffer(self.to_raw_vec(), secret.as_bytes().to_vec()),
            None => self.to_raw_vec(),
        }
    }

    fn deobfuscate(&self) -> Vec<u8> {
        match self.secret {
            Some(ref secret) => MConfig::xor_buffer(self.to_raw_vec(), secret.as_bytes().to_vec()),
            None => self.to_raw_vec(),
        }
    }

    fn xor_buffer(mut buf: Vec<u8>, secret: Vec<u8>) -> Vec<u8> {
        for (b, s) in buf
            .iter_mut()
            .skip(MConfig::HEADER_SIZE)
            .zip(secret.iter().cycle())
        {
            *b ^= s;
        }
        buf
    }
}

impl TryFrom<Vec<u8>> for MConfig {
    type Error = ();

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        todo!()
    }
}
