pub(crate) mod ipfs;

use http::Uri;
use ipfs::IpfsStorage;
use ipfs_api::IpfsClient;
use crate::error::IpseError;

pub trait Storage {
    fn write(&self, file: Vec<u8>) -> Result<String, IpseError>;
    fn read(&self, key: &str) -> Result<Vec<u8>, IpseError>;
    fn delete(&self, key: &str) -> Result<(), IpseError>;
}

pub fn new_storage<S: Storage>(ipfs_url: &'static str) -> S {
    let uri = ipfs_url.parse::<Uri>().expect("url parse failed");
    let cli = IpfsClient::build_with_base_uri(uri);
    IpfsStorage { cli }
}
