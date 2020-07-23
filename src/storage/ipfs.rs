use crate::error::IpseError;
use crate::storage::Storage;
use futures::TryStreamExt;
use ipfs_api::IpfsClient;
use std::io::Cursor;

pub struct IpfsStorage {
    pub cli: IpfsClient,
}

impl Storage for IpfsStorage {
    fn write(&self, file: Vec<u8>) -> Result<String, IpseError> {
        async_std::task::block_on(async move {
            let file = Cursor::new(file);
            let res = self.cli.add(file).await?;
            Ok(res.hash)
        })
    }

    fn read(&self, key: &str) -> Result<Vec<u8>, IpseError> {
        async_std::task::block_on(async move {
            self.cli
                .cat(key)
                .map_ok(|chunk| chunk.to_vec())
                .try_concat()
                .await
                .map_err(|e| From::from(e))
        })
    }

    fn delete(&self, key: &str) -> Result<(), IpseError> {
        async_std::task::block_on(async move { self.cli.pin_rm(key, false).await })?;
        Ok(())
    }
}
