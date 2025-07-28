#[non_exhaustive]
#[derive(Debug)]
pub enum WebdavError {
    RequestFailed(reqwest::Error),
    LockingFailed,
    UnLockingFailed,
}

impl From<reqwest::Error> for WebdavError {
    fn from(value: reqwest::Error) -> Self {
        WebdavError::RequestFailed(value)
    }
}
