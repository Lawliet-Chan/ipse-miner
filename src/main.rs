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

pub const CONF_PATH: &'static str = "conf_path";

fn main() {
    let matches = App::new("Ipse Miner")
                                .version("0.1.0")
                                .about("Mining for Ipse chain")
                                .arg(Arg::with_name(CONF_PATH)
                                    .short('c')
                                    .long(CONF_PATH)
                                    .default_value("config.yaml"))
                                .get_matches();
    let conf_fpath = matches.value_of(CONF_PATH).unwrap();

    let cfg = config::load_conf(conf_fpath);

    *m = miner::Miner::new(cfg);

}

#[get("/order/new?id=<num>")]
pub fn new_order(num: usize) {

}

#[get("/order?id=<num>")]
pub fn get_data(num: usize) {
    m.read_file(num)
}

#[delete("/order?id=<num>")]
pub fn delete_order(num: usize) {
    m.delete_file(num)
}


