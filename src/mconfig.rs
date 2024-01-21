//! Implementation of the MConfig obfuscated key-value storage library
//!

use rand;

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
type MCResult<T> = Result<T, MCError>;

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

    /// Return a Vec<u8> of the MConfig. It will be obfuscated if there is a secret configured.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::with_capacity(MConfig::MCONFIG_SIZE);
        v.append(&mut MConfig::MAGIC_HEADER_BYTES.to_vec());
        v.push(self.version);
        let mut e = MConfig::obfuscate(self.entries_to_vec(), &self.secret);
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

    /// Applies the obfuscation algorithm if a secret is set.
    fn obfuscate(buffer: Vec<u8>, secret: &Option<String>) -> Vec<u8> {
        match secret {
            Some(ref secret) => MConfig::xor_buffer(buffer.clone(), secret.as_bytes().to_vec()),
            None => buffer,
        }
    }

    /// Applies the deobfuscation algorithm if a secret is set.
    fn deobfuscate(buffer: Vec<u8>, secret: &Option<String>) -> Vec<u8> {
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

    /// Attempt to parse a Vec<u8> into a viable hashmap.
    fn try_parse(buffer: Vec<u8>, secret: &Option<String>) -> MCResult<MCHashMap> {
        let buffer = MConfig::deobfuscate(buffer, &secret);

        let mut entries = MCHashMap::new();
        let mut value_iter = buffer.iter().copied();

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
                    None => return Err(MCError::TruncatedKey),
                }
            }

            let key: String = match String::from_utf8(key_bytes) {
                Ok(k) => k,
                Err(_) => return Err(MCError::InvalidUTF8),
            };

            let val_len = match value_iter.next() {
                Some(v) => v as usize,
                None => return Err(MCError::MissingKey),
            };

            if val_len > 0 {
                let mut val_bytes: Vec<u8> = Vec::with_capacity(val_len);
                for _ in 0..val_len {
                    match value_iter.next() {
                        Some(v) => val_bytes.push(v),
                        None => return Err(MCError::TruncatedValue),
                    }
                }

                let val = match String::from_utf8(val_bytes) {
                    Ok(v) => v,
                    Err(_) => return Err(MCError::InvalidUTF8),
                };

                entries.insert(key, Some(val));
            } else {
                entries.insert(key, None); //valueless keys are allowed
            }
        }

        Ok(entries)
    }
}

/// Builder for the MConfig struct
pub struct MConfigBuilder {
    secret: Option<String>,
    raw_bytes: Option<Vec<u8>>,
}

impl MConfigBuilder {
    /// Returns an empty builder
    pub fn new() -> MConfigBuilder {
        MConfigBuilder {
            secret: None,
            raw_bytes: None,
        }
    }

    /// Sets the optional secret for this builder
    pub fn secret(mut self, secret: &str) -> MConfigBuilder {
        self.secret = Some(secret.to_string());
        self
    }

    /// Loads raw bytes which may or may not be obfuscated
    pub fn load(mut self, raw_bytes: Vec<u8>) -> MConfigBuilder {
        self.raw_bytes = Some(raw_bytes);
        self
    }

    /// Attempts to construct the MConfig object.
    /// The resulting object will be of the most recent version.
    /// This can fail if invalid raw data is loaded.
    /// Note that, while failure is likely if an invalid key is provided, it is not guaranteed.
    pub fn try_build(self) -> MCResult<MConfig> {
        let maybe_entries = match self.raw_bytes {
            Some(raw) => {
                if raw.len() < MConfig::HEADER_SIZE {
                    return Err(MCError::TooShort); //minimum length
                }
                if raw.len() > MConfig::MCONFIG_SIZE {
                    return Err(MCError::TooBig); //maximum length
                }

                //check header magic
                if raw[0..MConfig::MAGIC_HEADER_BYTES.len()] != MConfig::MAGIC_HEADER_BYTES {
                    return Err(MCError::BadHeader);
                }

                //check version
                if raw[MConfig::VERSION_INDEX] != 0u8 {
                    return Err(MCError::UnknownVersion);
                }
                MConfig::try_parse(raw[MConfig::HEADER_SIZE..].to_owned(), &self.secret)
            }
            None => Ok(MCHashMap::new()),
        };

        match maybe_entries {
            Ok(entries) => Ok(MConfig {
                secret: self.secret.clone(),
                entries,
                version: 0,
            }),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_retrievable_with_secret() {
        let mut mc = MConfigBuilder::new()
            .secret("secret secret, I've got a secret")
            .try_build()
            .unwrap();

        let _ = mc.try_insert("Test Key".to_string(), Some("Test Value".to_string()));

        assert_eq!(
            mc.get("Test Key"),
            Some(Some("Test Value".to_string())).as_ref()
        );
    }

    #[test]
    fn key_retrievable_without_secret() {
        let mut mc = MConfigBuilder::new().try_build().unwrap();

        let _ = mc.try_insert("Test Key".to_string(), Some("Test Value".to_string()));

        assert_eq!(
            mc.get("Test Key"),
            Some(Some("Test Value".to_string())).as_ref()
        );
    }

    #[test]
    fn to_and_from_vec() {
        let mut before_vec = MConfigBuilder::new()
            .secret("I like TACOS")
            .try_build()
            .unwrap();
        before_vec
            .try_insert("Hello".to_string(), Some("World".to_string()))
            .unwrap();

        let mcv = before_vec.to_vec();

        let after_vec = MConfigBuilder::new()
            .load(mcv)
            .secret("I like TACOS")
            .try_build()
            .unwrap();

        assert_eq!(
            before_vec.get("Hello"),
            Some(Some("World".to_string())).as_ref()
        );
        assert_eq!(
            after_vec.get("Hello"),
            Some(Some("World".to_string())).as_ref()
        );
    }

    #[test]
    #[should_panic]
    fn bad_key_fails() {
        let mut before_vec = MConfigBuilder::new()
            .secret("I like TACOS")
            .try_build()
            .unwrap();
        before_vec
            .try_insert("Hello".to_string(), Some("World".to_string()))
            .unwrap();

        let mcv = before_vec.to_vec();

        let after_vec = MConfigBuilder::new()
            .load(mcv)
            .secret("I hate TACOS")
            .try_build()
            .unwrap();

        assert_eq!(
            before_vec.get("Hello"),
            Some(Some("World".to_string())).as_ref()
        );
        assert_eq!(
            after_vec.get("Hello"),
            Some(Some("World".to_string())).as_ref()
        );
    }

    #[test]
    fn maximum_length_fails() {
        let mut testmcnf = MConfigBuilder::new()
            .try_build()
            .unwrap();

        //insert key-value pairs totalling 10 bytes, plus two for length, totalling 12
        //enough times to almost fill it
        for i in 0.. (MConfig::MCONFIG_SIZE - MConfig::HEADER_SIZE) / 12 {
            let k = format!("key{:0>3}", i);

            testmcnf.try_insert(k, Some("1234".to_string())).expect("Too big too soon");
        }

        assert_eq!(testmcnf.try_insert("final_key".to_string(), Some("oops".to_string())), Err(MCError::TooBig));
    }
}
