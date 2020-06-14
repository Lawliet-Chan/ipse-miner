#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
use storage::Storage;
use clap::{App, Arg};
use lazy_static::lazy_static;

mod storage;
mod miner;
mod config;

lazy_static! {
    pub static ref m: Miner::<storage::ipfs::IpfsStorage> = ();
}

pub const IPFS_URL: &'static str = "ipfs_url";
pub const META_PATH: &'static str = "meta_path";
pub const CHAIN_URL: &'static str = "chain_url";

fn main() {
    let matches = App::new("Ipse Miner")
                                .version("0.1.0")
                                .about("Mining for Ipse chain")
                                .arg(Arg::with_name(IPFS_URL)
                                    .short('i')
                                    .long(IPFS_URL))
                                .arg(Arg::with_name(META_PATH)
                                    .short('m')
                                    .long(META_PATH))
                                .arg(Arg::with_name(CHAIN_URL)
                                    .short('c')
                                    .long(CHAIN_URL))
                                .get_matches();
    let ipfs_url = matches.value_of(IPFS_URL).unwrap_or("localhost:5001");
    let meta_path = matches.value_of(META_PATH).unwrap_or("/ipse-miner/meta");
    let chain_url = matches.value_of(CHAIN_URL).unwrap();
    let cfg = config::Conf {
        ipfs_url,
        meta_path,
        chain_url,
    };

    *m = miner::Miner::new(cfg);

}


#[get("/order/new?id=<num>")]
pub fn new_order(num: usize) {

}

#[get("/order/delete?id=<num>")]
pub fn delete_order(num: usize) {

}

#[get("/data/transfer?id=<num>")]
pub fn transfer_data(num: usize) {

}