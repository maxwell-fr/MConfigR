//! Implementation of the MConfig obfuscated key-value storage library
//!
use crate::mcstring::MCString;


pub struct MConfig {
    version: u8,
    entries: std::collections::HashMap<MCString, MCString>
}

impl MConfig {
    const MAGIC_HEADER_BYTES: [u8; 5] = [0x4d, 0x43, 0x4f, 0x4e, 0x46];
    const MCONFIG_SIZE: usize = 8_192;
}

impl AsRef<Vec<u8>> for MConfig {
    fn as_ref(&self) -> &Vec<u8> {
        todo!()
    }
}