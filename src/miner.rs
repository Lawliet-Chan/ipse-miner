use crate::storage::*;
use sled::Db;
use crate::config::Conf;

pub struct Miner<S: Storage> {
    meta_db: Db,
    storage: S,
}

struct DataInfo {
    pub sector: u64,
    pub offset: u64,
    pub length: u64,
}

impl<S: Storage> Miner<S>{
    pub fn new(cfg: Conf) -> Self {
        let meta_db = sled::open(cfg.meta_path).expect("open metadata db");
        let storage = new_storage::<ipfs::IpfsStorage>(cfg.ipfs_url);
        Self {
            meta_db,
            storage,
        }
    }

    pub fn write(&mut self) {

    }

    pub fn read(&self) {

    }

    pub fn delete(&mut self) {

    }

}