//! Implementation of the MConfig obfuscated key-value storage library
//!
//! Keys are `String`s, and values are `Option<String>`s. It is possible
//! to store a Key without a Value by using None.
//!
//! ```
//!
//! use mconfig::mconfigurator::MConfig;
//! pub fn demo() {
//!     let mut mcnf = MConfig::builder().secret("TACOS").try_build().unwrap();
//!     mcnf.try_insert("Hello".to_string(), Some("World".to_string())).expect("Hello failed");
//!     mcnf.try_insert("Bye".to_string(), None).expect("Bye failed");
//!
//!     // Convert it to a vec
//!     let mcv = mcnf.to_vec();
//!     println!("{:?}", mcv.len());
//!
//!     // Make a new one using the vec
//!     let mcnf1 = MConfig::builder().load(mcv).secret("TACOS").try_build();
//!
//!     // Retrieve a key and print
//!     println!("{:?}", mcnf.get("Hello").unwrap());
//!
//!     // Retrieve a key from the duplicated one
//!     println!("{:?}", mcnf1.unwrap()["Hello"].as_ref().unwrap());
//!
//!     // Demonstrate the iterator function
//!     for e in mcnf.iter() {
//!         println!("{:?}", e);
//!     }
//! }
//! ```
//!
mod mconfig_builder;

use crate::mconfigurator::mconfig_builder::MConfigBuilder;
use rand;
use std::collections::hash_map::Iter as HashMapIter;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Index;

/// Errors that can be generated by MConfig.
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

impl Display for MCError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for MCError {

}

type MCHashMap = std::collections::HashMap<String, Option<String>>;
pub type MCResult<T> = Result<T, MCError>;

/// Key-value storage with optional secret
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
    const LATEST_VERSION: u8 = 0;

    /// Get a new Builder
    pub fn builder() -> MConfigBuilder {
        MConfigBuilder::new()
    }

    /// Return a `Vec<u8>` of the MConfig. It will be obfuscated if there is a secret configured.
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
    /// Returns old value if Ok and key was present.
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

    /// Get the number of elements in the collection.
    pub fn len(&self) -> usize {
        self.entries.len()
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

/// Index notation support
/// Returns a reference to the value at the key/index.
/// #Panics
/// Panics if the key is not present.
impl Index<&str> for MConfig {
    type Output = Option<String>;

    fn index(&self, index: &str) -> &Self::Output {
        &self.entries[index]
    }
}

/// Iterator support
pub struct MConfigIter<'a> {
    mc_iter: HashMapIter<'a, String, Option<String>>,
}

impl MConfigIter<'_> {
    fn new(mconfig: &MConfig) -> MConfigIter {
        MConfigIter {
            mc_iter: mconfig.entries.iter(),
        }
    }
}

impl<'a> Iterator for MConfigIter<'a> {
    type Item = (&'a String, &'a Option<String>);

    fn next(&mut self) -> Option<Self::Item> {
        self.mc_iter.next()
    }
}

/// Convert a plain hashmap to an MConfig
/// This sets up the object with the latest version and no secret.
impl TryFrom<std::collections::HashMap<String, Option<String>>> for MConfig {
    type Error = MCError;

    fn try_from(value: HashMap<String, Option<String>>) -> Result<Self, Self::Error> {
        let mut total_len: usize = 0;

        // validate lengths; UTF-8 constraint already ensured by String
        for (key, value) in &value {
            if key.len() > MConfig::MAX_KEY_LEN {
                return Err(MCError::KeyTooBig);
            }
            total_len += key.len() + 1;

            if let Some(v) = value {
                if v.len() > MConfig::MAX_VALUE_LEN {
                    return Err(MCError::ValueTooBig);
                }
                else {
                    total_len += v.len();
                }
            }
            total_len += 1;
            if total_len > MConfig::MCONFIG_SIZE - MConfig::HEADER_SIZE {
                return Err(MCError::TooBig);
            }
        }

        Ok(MConfig {
            version: MConfig::LATEST_VERSION,
            entries: value,
            secret: None,
        })
    }
}