use crate::mconfigurator::{MCError, MCHashMap, MConfig, MCResult};

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
    /// Attempt to parse a Vec<u8> into a viable hashmap.
    fn try_parse(buffer: Vec<u8>, secret: &Option<String>, version: u8) -> MCResult<MCHashMap> {
        let buffer = MConfig::deobfuscate(buffer, secret, version);

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

                //check and select version
                if raw[MConfig::VERSION_INDEX] != 0u8 {
                    return Err(MCError::UnknownVersion);
                }
                let version = raw[MConfig::VERSION_INDEX];
                MConfigBuilder::try_parse(raw[MConfig::HEADER_SIZE..].to_owned(), &self.secret, version)
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


impl Default for MConfigBuilder {
    fn default() -> Self {
        MConfigBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::mconfigurator::*;

    #[test]
    fn key_retrievable_with_secret() {
        let mut mc = MConfig::builder()
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
        let mut mc = MConfig::builder().try_build().unwrap();

        let _ = mc.try_insert("Test Key".to_string(), Some("Test Value".to_string()));

        assert_eq!(
            mc.get("Test Key"),
            Some(Some("Test Value".to_string())).as_ref()
        );
    }

    #[test]
    fn to_and_from_vec() {
        let mut before_vec = MConfig::builder()
            .secret("I like TACOS")
            .try_build()
            .unwrap();
        before_vec
            .try_insert("Hello".to_string(), Some("World".to_string()))
            .unwrap();

        let mcv = before_vec.to_vec();

        let after_vec = MConfig::builder()
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
        let mut before_vec = MConfig::builder()
            .secret("I like TACOS")
            .try_build()
            .unwrap();
        before_vec
            .try_insert("Hello".to_string(), Some("World".to_string()))
            .unwrap();

        let mcv = before_vec.to_vec();

        let after_vec = MConfig::builder()
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
        let mut testmcnf = MConfig::builder()
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
