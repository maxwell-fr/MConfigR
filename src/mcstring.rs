//! A constrained string type
//! UTF-8 and < 256 bytes
#[derive(Hash, Eq, PartialEq)]
pub struct MCString {
    value: String,
}

impl MCString {
    pub fn len(&self) -> u8 {
        self.value.len() as u8
    }
}

impl TryFrom<&str> for MCString {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() > 255 {
            Err(())
        } else {
            Ok(MCString {
                value: value.to_string(),
            })
        }
    }
}


impl AsRef<str> for MCString {
    fn as_ref(&self) -> &str {
        self.value.as_str()
    }
}
