use crate::storage::Storage;
use std::io::Result;
use ipfs_api::IpfsClient;

pub struct IpfsStorage {
    cli: IpfsClient
}

impl Storage for IpfsStorage {
    fn write(&mut self, key: &str, file: Vec<u8>) -> Result<()> {
        async_std::task::block_on(async move {
            self.cli.files_write(key, true, true, file).await
        })
    }

    fn read(&self, key: &str) -> Result<Vec<u8>> {
        async_std::task::block_on(async move {
            self.cli.files_read(key)
                .map_ok(|chunk| chunk.to_vec() )
                .try_concat()
                .await
        })
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        async_std::task::block_on(async move {
            self.cli.files_rm(key, false).await
        })
    }
}