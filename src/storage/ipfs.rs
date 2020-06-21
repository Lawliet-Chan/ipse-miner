use crate::storage::Storage;
use ipfs_api::IpfsClient;
use std::io::{Cursor, Result};

pub struct IpfsStorage {
    cli: IpfsClient,
}

impl Storage for IpfsStorage {
    fn write(&self, file: Vec<u8>) -> Result<String> {
        async_std::task::block_on(async move {
            let file = Cursor::new(file);
            let res = self.cli.add(file).await?;
            Ok(res.hash)
        })
    }

    fn read(&self, key: &str) -> Result<Vec<u8>> {
        async_std::task::block_on(async move {
            self.cli
                .cat(key)
                .map_ok(|chunk| chunk.to_vec())
                .try_concat()
                .await
        })
    }

    fn delete(&self, key: &str) -> Result<()> {
        async_std::task::block_on(async move { self.cli.pin_rm(key, false).await })?;
        Ok(())
    }
}
