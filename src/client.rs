//! Webdav client
//!
//! Create a client
//! ```ignore
//! let client = Client::init("username", "password");
//! ```
//! Now you can use the client to call any of the methods listed in the **Client** struct.
//!
//! All the paths used by the methods should be absolute on the webdav server to the
//! required file, folder, zip.
//!
//! Every method will return a Result<Response, Error>
//! ```rust
//! # let result: Result<&str, String> = Ok("test");
//! if result.is_ok() {
//!    // the method completed with success
//! } else {
//!    // something went wrong
//! }
//! ```

use super::prelude::*;
#[cfg(feature = "locking")]
use crate::file_lock::LockToken;
use crate::{error::WebdavError, header::HeaderBuilder};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Client {
    username: String,
    password: String,
    client: reqwest::blocking::Client,
}

impl Client {
    /// Initialization of the client
    ///
    /// Initialized client will be stored for future requests
    pub fn init(username: &str, password: &str) -> Self {
        Client {
            username: username.to_owned(),
            password: password.to_owned(),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn form_params(&self, key: &'static str, value: &'static str) -> HashMap<&str, &str> {
        let mut params = HashMap::new();
        params.insert(key, value);

        params
    }

    /// Main function that creates the RequestBuilder, sets the method, url and the basic_auth
    fn start_request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, Url::parse(path).unwrap())
            .basic_auth(&self.username, Some(&self.password))
    }

    /// Get a file from Webdav server
    ///
    /// Use absolute path to the webdav server file location
    pub fn get(&self, path: &str) -> Result<Response, WebdavError> {
        self.start_request(Method::GET, path)
            .send()?
            .error_for_status()
            .map_err(WebdavError::RequestFailed)
    }

    /// Upload a file/zip on Webdav server
    ///
    /// It can be any type of file as long as it is transformed to a vector of bytes (Vec<u8>).
    /// This can be achieved with **std::fs::File** or **zip-rs** for sending zip files.
    ///
    /// Use absolute path to the webdav server folder location
    pub fn put<B: Into<Body>>(&self, body: B, path: &str) -> Result<Response, WebdavError> {
        self.start_request(Method::PUT, path)
            .headers(
                HeaderBuilder::new()
                    .add_item("content-type", "application/octet-stream")
                    .unwrap()
                    .build(),
            )
            .body(body)
            .send()?
            .error_for_status()
            .map_err(WebdavError::RequestFailed)
    }

    /// Deletes the collection, file, folder or zip archive at the given path on Webdav server
    ///
    /// Use absolute path to the webdav server file location
    pub fn delete(&self, path: &str) -> Result<Response, WebdavError> {
        self.start_request(Method::DELETE, path)
            .send()?
            .error_for_status()
            .map_err(WebdavError::RequestFailed)
    }

    /// Unzips the .zip archieve on Webdav server
    ///
    /// Use absolute path to the webdav server file location
    pub fn unzip(&self, path: &str) -> Result<Response, WebdavError> {
        self.start_request(Method::POST, path)
            .form(&self.form_params("method", "UNZIP"))
            .send()?
            .error_for_status()
            .map_err(WebdavError::RequestFailed)
    }

    /// Creates a directory on Webdav server
    ///
    /// Use absolute path to the webdav server file location
    pub fn mkcol(&self, path: &str) -> Result<Response, WebdavError> {
        self.start_request(Method::from_bytes(b"MKCOL").unwrap(), path)
            .send()?
            .error_for_status()
            .map_err(WebdavError::RequestFailed)
    }

    /// Rename or move a collection, file, folder on Webdav server
    ///
    /// If the file location changes it will move the file, if only the file name changes it will rename it.
    ///
    /// Use absolute path to the webdav server file location
    pub fn mv(&self, from: &str, to: &str) -> Result<Response, WebdavError> {
        self.start_request(Method::from_bytes(b"MOVE").unwrap(), from)
            .headers(
                HeaderBuilder::new()
                    .add_item("destination", to)
                    .unwrap()
                    .build(),
            )
            .send()?
            .error_for_status()
            .map_err(WebdavError::RequestFailed)
    }

    /// List files and folders at the given path on Webdav server
    ///
    /// Depth of "0" applies only to the resource, "1" to the resource and it's children, "infinity" to the resource and all it's children recursively
    /// The result will contain an xml list with the remote folder contents.
    ///
    /// Use absolute path to the webdav server folder location
    pub fn list(&self, path: &str, depth: &str) -> Result<Response, WebdavError> {
        let body = r#"<?xml version="1.0" encoding="utf-8" ?>
            <D:propfind xmlns:D="DAV:">
                <D:allprop/>
            </D:propfind>
        "#;

        self.start_request(Method::from_bytes(b"PROPFIND").unwrap(), path)
            .headers(
                HeaderBuilder::new()
                    .add_item("depth", depth)
                    .unwrap()
                    .build(),
            )
            .body(body)
            .send()?
            .error_for_status()
            .map_err(WebdavError::RequestFailed)
    }

    /// Try to lock resource at the given path on Webdav server
    #[cfg(feature = "locking")]
    pub fn lock(&self, path: &str, duration: &str) -> Result<(LockToken, Response), WebdavError> {
        let body = r#"<?xml version="1.0" encoding="utf-8" ?>
            <D:lockinfo xmlns:D='DAV:'>
                <D:lockscope><D:exclusive/></D:lockscope>
                <D:locktype><D:write/></D:locktype>
                <D:owner>
                    <D:href>http://example.org/~ejw/contact.html</D:href>
                </D:owner>
            </D:lockinfo>"#;

        let response = self
            .start_request(Method::from_bytes(b"LOCK").unwrap(), path)
            .headers(
                HeaderBuilder::new()
                    .add_item("timeout", duration)
                    .unwrap()
                    .build(),
            )
            .body(body)
            .send()?
            .error_for_status()?;

        match LockToken::extract_from_response(&response) {
            None => Err(WebdavError::LockingFailed),
            Some(lock_token) => Ok((lock_token, response)),
        }
    }

    /// Try to unlock resource with a lock token at the given path on Webdav server
    #[cfg(feature = "locking")]
    pub fn unlock(&self, path: &str, lock_token: &LockToken) -> Result<Response, WebdavError> {
        self.start_request(Method::from_bytes(b"UNLOCK").unwrap(), path)
            .headers(
                HeaderBuilder::new()
                    .add_item("lock-token", &lock_token.to_string())
                    .unwrap()
                    .build(),
            )
            .send()
            .map_err(WebdavError::RequestFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use std::{
        fs::{copy, create_dir, remove_dir_all, File},
        io::{Read, Write},
        path::Path,
    };

    const SERVER_URL: &str = "http://localhost:6065";

    fn get_server_path(path: &str) -> String {
        format!("{SERVER_URL}/{path}")
    }

    fn get_client() -> Client {
        Client::init("test", "password")
    }

    fn reset_webdav_server_directories() {
        let test_folder = "webdav_server/tests/";
        if let Err(err) = remove_dir_all(test_folder) {
            if err.kind() != std::io::ErrorKind::NotFound {
                panic!("Couldn't remove all directories and files due to {}", err);
            }
        }

        if let Err(err) = create_dir(test_folder) {
            panic!("Couldn't create fresh test directory due to {}", err);
        }
    }

    fn create_zipped_file() {
        copy("webdav_server/test.zip", "webdav_server/tests/test.zip")
            .expect("Couldn't create zip file");
    }

    #[test]
    #[serial_test::serial]
    fn test_1_mkcol() {
        // preparation of webdav server
        reset_webdav_server_directories();

        let webdav_client = get_client();

        let result = webdav_client
            .mkcol(&get_server_path("new_collection"))
            .expect("Response failed");

        assert_eq!(reqwest::StatusCode::CREATED, result.status());
        assert!(Path::new("webdav_server/tests/new_collection").exists());
    }

    #[test]
    #[serial_test::serial]
    fn test_2_put() {
        // preparation of webdav server
        reset_webdav_server_directories();

        let webdav_client = get_client();

        let result = webdav_client
            .put("Hello World!", &get_server_path("test.txt"))
            .expect("Response failed");

        assert_eq!(reqwest::StatusCode::CREATED, result.status());
        assert!(Path::new("webdav_server/tests/test.txt").exists());

        let mut file = File::open("webdav_server/tests/test.txt").expect("Couldn't open file");
        let mut file_content = String::new();
        File::read_to_string(&mut file, &mut file_content).expect("Couldn't read file");
        assert_eq!(&file_content, "Hello World!");
    }

    #[test]
    #[serial_test::serial]
    fn test_3_get() {
        // preparation of webdav server
        reset_webdav_server_directories();
        let mut file =
            File::create("webdav_server/tests/test.txt").expect("Couldn't create new file");
        file.write_all(b"Hello World!")
            .expect("Couldn't write content to file");

        let webdav_client = get_client();

        let result = webdav_client
            .get(&get_server_path("test.txt"))
            .expect("Response failed");

        assert_eq!(reqwest::StatusCode::OK, result.status());
        assert_eq!(
            result.text().expect("Couldn't read response text"),
            "Hello World!"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_4_mv() {
        // preparation of webdav server
        reset_webdav_server_directories();
        File::create("webdav_server/tests/test.txt").expect("Couldn't create new file");
        create_dir("webdav_server/tests/target/").expect("Couldn't create directory");

        let webdav_client = get_client();

        let result = webdav_client
            .mv(
                &get_server_path("test.txt"),
                &get_server_path("target/test.txt"),
            )
            .expect("Response failed");

        assert_eq!(reqwest::StatusCode::CREATED, result.status());
        assert!(!Path::new("webdav_server/tests/test.txt").exists());
        assert!(Path::new("webdav_server/tests/target/test.txt").exists());
    }

    #[test]
    #[serial_test::serial]
    fn test_5_delete() {
        // preparation of webdav server
        reset_webdav_server_directories();
        File::create("webdav_server/tests/test.txt").expect("Couldn't create new file");

        let webdav_client = get_client();

        let result = webdav_client
            .delete(&get_server_path("test.txt"))
            .expect("Response failed");

        assert_eq!(reqwest::StatusCode::NO_CONTENT, result.status());
        assert!(!Path::new("webdav_server/tests/test.txt").exists());
    }

    #[test]
    #[serial_test::serial]
    fn test_6_unzip() {
        // preparation of webdav server
        reset_webdav_server_directories();
        create_zipped_file();

        let webdav_client = get_client();
        let result = webdav_client
            .unzip(&get_server_path("test.zip"))
            .expect("Response failed");

        assert_eq!(reqwest::StatusCode::OK, result.status());
    }

    #[test]
    #[serial_test::serial]
    fn test_7_list() {
        // preparation of webdav server
        reset_webdav_server_directories();
        File::create("webdav_server/tests/test_1.txt").expect("Couldn't create new file");
        File::create("webdav_server/tests/test_2.txt").expect("Couldn't create new file");

        let webdav_client = get_client();

        let result = webdav_client
            .list(&get_server_path(""), "1")
            .expect("Response failed");

        assert_eq!(reqwest::StatusCode::MULTI_STATUS, result.status());
        let result_text = result.text().expect("Couldn't read response text");
        assert!(result_text.contains("test_1.txt"));
        assert!(result_text.contains("test_2.txt"));
    }

    #[cfg(feature = "locking")]
    #[test]
    #[serial_test::serial]
    fn test_8_lock() {
        // preparation of webdav server
        reset_webdav_server_directories();
        File::create("webdav_server/tests/test.txt").expect("Couldn't create new file");

        let webdav_client = get_client();

        // Locking
        let (_lock_token, result_first_locking) = webdav_client
            .lock(&get_server_path("test.txt"), "Second-1")
            .expect("Should retrieve any response for first locking");

        assert_eq!(reqwest::StatusCode::OK, result_first_locking.status());

        // Try locking second time
        let error_second_locking = webdav_client
            .lock(&get_server_path("test.txt"), "Second-1")
            .expect_err("Second locking should fail");

        assert!(matches!(
            error_second_locking,
            WebdavError::RequestFailed(..)
        ));

        thread::sleep(Duration::from_secs(1));
    }

    #[cfg(feature = "locking")]
    #[test]
    #[serial_test::serial]
    fn test_9_lock_and_unlock() {
        // preparation of webdav server
        reset_webdav_server_directories();
        File::create("webdav_server/tests/test.txt").expect("Couldn't create new file");

        let webdav_client = get_client();

        // Locking
        let (lock_token, result_locking) = webdav_client
            .lock(&get_server_path("test.txt"), "Second-1")
            .expect("Should retrieve any response for first locking");

        assert_eq!(reqwest::StatusCode::OK, result_locking.status());

        // Unlocking
        let result_unlocking = webdav_client
            .unlock(&get_server_path("test.txt"), &lock_token)
            .expect("Should retrieve any response for first locking");

        assert_eq!(reqwest::StatusCode::NO_CONTENT, result_unlocking.status());

        thread::sleep(Duration::from_secs(1));
    }
}
