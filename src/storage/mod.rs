pub(crate) mod ipfs;

use std::io::Result;
use ipfs::IpfsStorage;
use ipfs_api::IpfsClient;
use http::{Request, Uri};

pub trait Storage {
    fn write(&mut self, key: &str, file: Vec<u8>) -> Result<String>;
    fn read(&self, key: &str) -> Result<Vec<u8>>;
    fn delete(&mut self, key: &str) -> Result<()>;
}

pub fn new_storage<S: Storage>(ipfs_url: &'static str) -> S {
    let uri = ipfs_url.parse::<Uri>().expect("url parse failed");
    let cli = IpfsClient::build_with_base_uri(uri);
    IpfsStorage {
        cli
    }
}