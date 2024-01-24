//! Implementation of the MConfig obfuscated key-value storage library
//!
mod mconfig_builder;

use rand;
use std::collections::hash_map::Iter as HashMapIter;
use crate::mconfig::mconfig_builder::MConfigBuilder;

#[derive(Debug, PartialEq)]
pub enum MCError {
    TooShort,
    TooBig,
    BadHeader,
    UnknownVersion,
    TruncatedKey,
    TruncatedValue,
    MissingKey,
    InvalidUTF8,
    ValueTooBig,
    KeyTooBig,
}

type MCHashMap = std::collections::HashMap<String, Option<String>>;
pub type MCResult<T> = Result<T, MCError>;

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

    /// Get a new Builder
    pub fn builder() -> MConfigBuilder {
        MConfigBuilder::new()
    }

    /// Return a Vec<u8> of the MConfig. It will be obfuscated if there is a secret configured.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::with_capacity(MConfig::MCONFIG_SIZE);
        v.append(&mut MConfig::MAGIC_HEADER_BYTES.to_vec());
        v.push(self.version);
        let mut e = MConfig::obfuscate(self.entries_to_vec(), &self.secret, self.version);
        v.append(&mut e);
        assert_eq!(v.len(), MConfig::MCONFIG_SIZE);
        v
    }

    /// Return a Vec<u8> of the entries that is not obfuscated.
    fn entries_to_vec(&self) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();

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
        v.push(0); //end of data
        assert!(v.len() <= MConfig::MCONFIG_SIZE - MConfig::HEADER_SIZE);

        //pad the rest with random, leaving space for a header
        for _ in v.len()..MConfig::MCONFIG_SIZE - MConfig::HEADER_SIZE {
            v.push(rand::random::<u8>());
        }

        v
    }

    /// Insert a key-value pair. The value is optional.
    /// This will fail if the key, the value is too long or if the addition would make the overall length
    /// exceed MCONFIG_SIZE.
    pub fn try_insert(&mut self, key: String, value: Option<String>) -> MCResult<Option<String>> {
        if key.len() > MConfig::MAX_KEY_LEN {
            return Err(MCError::KeyTooBig);
        }
        if let Some(ref val) = value {
            if val.len() > MConfig::MAX_VALUE_LEN {
                return Err(MCError::ValueTooBig);
            }
        }

        //check overall length if the new entry is added.
        let overall_len = key.len()
            + 1
            + match value {
                Some(ref v) => v.len() + 1,
                None => 1,
            }
            + self
                .entries
                .iter()
                .fold(MConfig::HEADER_SIZE, |acc, (k, v)| {
                    acc + k.len()
                        + 1
                        + match v {
                            Some(v) => v.len() + 1,
                            None => 1,
                        }
                });

        if overall_len < MConfig::MCONFIG_SIZE {
            Ok(self.entries.insert(key, value).unwrap_or(None))
        } else {
            Err(MCError::TooBig)
        }
    }

    /// Try to retrieve a value at key. Will fail if the key is not present.
    pub fn try_get(&self, key: &str) -> MCResult<&Option<String>> {
        self.entries.get(key).ok_or(MCError::MissingKey)
    }

    /// Retrieve the value at key. Returns None if the key is not set.
    pub fn get(&self, key: &str) -> Option<&Option<String>> {
        self.entries.get(key)
    }

    /// Check if a given key is present.
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Remove a key if present. Returns the old value or None if not set.
    pub fn remove(&mut self, key: &str) -> Option<Option<String>> {
        self.entries.remove(key)
    }

    /// Change the secret used during obfuscation.
    pub fn set_secret(&mut self, secret: Option<String>) {
        self.secret = secret;
    }

    /// Applies the obfuscation algorithm if a secret is set.
    fn obfuscate(buffer: Vec<u8>, secret: &Option<String>, _version: u8) -> Vec<u8> {
        match secret {
            Some(ref secret) => MConfig::xor_buffer(buffer.clone(), secret.as_bytes().to_vec()),
            None => buffer,
        }
    }

    /// Applies the deobfuscation algorithm if a secret is set.
    fn deobfuscate(buffer: Vec<u8>, secret: &Option<String>, _version: u8) -> Vec<u8> {
        match secret {
            Some(ref secret) => MConfig::xor_buffer(buffer.clone(), secret.as_bytes().to_vec()),
            None => buffer,
        }
    }

    /// The algorithm used in v0. This is reversible so it is used for both ob- and deobfuscation.
    /// This simply XORs the bytes of data against the bytes of the secret.
    /// In theory, if the secret were longer than MCONFIG_SIZE, the actual obfuscation would be unbreakable if
    /// only used once (e.g., one-time pad) but the nature of this whole implementation precludes that sort of security.
    fn xor_buffer(mut buf: Vec<u8>, secret: Vec<u8>) -> Vec<u8> {
        for (b, s) in buf.iter_mut().zip(secret.iter().cycle()) {
            *b ^= s;
        }
        buf
    }

    /// Helper function to get an Iterator
    pub fn iter(&self) -> MConfigIter {
        MConfigIter::new(self)
    }

}

/// Iterator support
pub struct MConfigIter<'a> {
    mc_iter: HashMapIter<'a, String, Option<String>>,
}

impl MConfigIter<'_> {
    fn new(mconfig: &MConfig) -> MConfigIter {
        MConfigIter {
            mc_iter: mconfig.entries.iter()
        }
    }
}

impl<'a> Iterator for MConfigIter<'a> {
    type Item = (&'a String, &'a Option<String>);

    fn next(&mut self) -> Option<Self::Item> {
        self.mc_iter.next()
    }
}
