//! A constrained string type
//! UTF-8 and < 256 bytes

pub struct MCString {
    value: String,
}

impl MCString {}

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
