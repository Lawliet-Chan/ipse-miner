use std::fs;
use std::io::Read;
use serde::de::{Deserialize, Deserializer};

#[derive(Debug, Serialize, Deserialize)]
pub struct Conf {
    pub nickname: &'static str,
    pub region: &'static str,
    pub url: &'static str,
    pub capacity: u64,
    pub unit_price: u64,

    pub meta_path: &'static str,
    pub ipfs_url: &'static str,
    pub chain_url: &'static str,
    pub sign: &'static str,
    pub pwd: Option<&'static str>,
}

pub fn load_conf(fpath: &str) -> Conf{
    let buf = fs::read_to_string(fpath).expect("load config file failed");
    serde_yaml::from_str(buf.as_ref()).expect("parse config file failed")
}