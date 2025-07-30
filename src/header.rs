#[cfg(feature = "locking")]
use crate::file_lock::LockToken;
use crate::{
    error::WebdavError,
    prelude::header::{HeaderMap, HeaderName, HeaderValue},
};

pub struct HeaderBuilder {
    header_map: HeaderMap,
}

impl HeaderBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            header_map: HeaderMap::new(),
        }
    }

    pub fn build(self) -> HeaderMap {
        self.header_map
    }

    pub fn add_item(mut self, name: &str, value: &str) -> Result<Self, WebdavError> {
        self.header_map.insert(
            HeaderName::from_bytes(name.as_bytes())
                .map_err(|_err| WebdavError::HeaderElementInvalid)?,
            HeaderValue::from_bytes(value.as_bytes())
                .map_err(|_err| WebdavError::HeaderElementInvalid)?,
        );

        Ok(self)
    }

    #[cfg(feature = "locking")]
    pub fn add_lock_token(self, lock_token: &LockToken) -> Result<Self, WebdavError> {
        self.add_item("if", &format!("({lock_token})"))
    }
}
