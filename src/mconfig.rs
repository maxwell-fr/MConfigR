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
    const VERSION_INDEX: usize = MConfig::MAGIC_HEADER_BYTES.len();
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

#[derive(Debug)]
pub enum MCError {
    TooShort,
    TooBig,
    BadHeader,
    UnknownVersion,
    TruncatedKey,
    TruncatedValue,
    MissingKey,
    InvalidUTF8
}
impl TryFrom<Vec<u8>> for MConfig {
    type Error = MCError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() < MConfig::HEADER_SIZE {
            return Err(MCError::TooShort); //minimum length
        }
        if value.len() > MConfig::MCONFIG_SIZE {
            return Err(MCError::TooBig); //maximum length
        }

        //check header magic
        if value[0..MConfig::MAGIC_HEADER_BYTES.len()] != MConfig::MAGIC_HEADER_BYTES {
            return Err(MCError::BadHeader);
        }

        //check version
        if value[MConfig::VERSION_INDEX] != 0u8 {
            return Err(MCError::UnknownVersion);
        }

        let mut entries = MConfig::without_secret();
        let mut value_iter = value[MConfig::HEADER_SIZE..].iter().copied();

        while let Some(b) = value_iter.next() {
            //key length
            let key_len = b as usize;
            //key length zero means end of data/start of padding
            if key_len == 0 {
                break;
            }

            let mut key_bytes: Vec<u8> = Vec::with_capacity(key_len);
            for _ in 0..key_len {
                match value_iter.next() {
                    Some(k) => key_bytes.push(k),
                    None => return Err(MCError::TruncatedKey) // truncated key
                }
            }

            let key: String = match String::from_utf8(key_bytes) {
                Ok(k) => k,
                Err(_) => return Err(MCError::InvalidUTF8) //invalid UTF-8
            };

            let val_len = match value_iter.next() {
                Some(v) => v as usize,
                None => return Err(MCError::MissingKey) // key with no value marker
            };

            if val_len > 0 {
                let mut val_bytes: Vec<u8> = Vec::with_capacity(val_len);
                for _ in 0..val_len {
                    match value_iter.next() {
                        Some(v) => val_bytes.push(v),
                        None => return Err(MCError::TruncatedValue) //truncated value
                    }
                }

                let val = match String::from_utf8(val_bytes) {
                    Ok(v) => v,
                    Err(_) => return Err(MCError::InvalidUTF8)
                };

                entries.try_add(key, Some(val));
            }
            else {
                entries.try_add(key, None); //valueless keys are allowed
            }

        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_retrievable_with_secret(){
        let mut mc = MConfig::new("secret secret, I've got a secret");

        let _ = mc.try_add("Test Key".to_string(), Some("Test Value".to_string()));

        assert_eq!(mc.get("Test Key"), Some(Some("Test Value".to_string())).as_ref());
    }

    #[test]
    fn key_retrievable_without_secret(){
        let mut mc = MConfig::without_secret();

        let _ = mc.try_add("Test Key".to_string(), Some("Test Value".to_string()));

        assert_eq!(mc.get("Test Key"), Some(Some("Test Value".to_string())).as_ref());
    }
}
