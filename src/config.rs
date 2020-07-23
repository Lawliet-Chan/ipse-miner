use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conf {
    pub nickname: String,
    pub region: String,
    pub url: String,
    pub capacity: u64,
    pub unit_price: u64,

    pub meta_path: String,
    pub ipfs_url: String,
    pub chain_url: String,
}

pub fn load_conf(fpath: &str) -> Conf {
    let buf = fs::read_to_string(fpath).expect("load config file failed");
    serde_yaml::from_str::<Conf>(buf.as_str()).expect("parse config file failed")
}
