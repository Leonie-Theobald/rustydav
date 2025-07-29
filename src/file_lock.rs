#[cfg(feature = "locking")]
use reqwest::blocking::Response;

#[derive(Debug)]
pub struct LockToken {
    token: String,
}

impl LockToken {
    #[cfg(feature = "locking")]
    pub(crate) fn extract_from_response(response: &Response) -> Option<LockToken> {
        match response.headers().get("lock-token") {
            None => None,
            Some(lock_token) => lock_token
                .to_str() // converts header value to &str
                .ok() // converts result to option
                .map(|token| LockToken {
                    token: String::from(token),
                }), // creates LockToken
        }
    }
}

impl std::fmt::Display for LockToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token)
    }
}
