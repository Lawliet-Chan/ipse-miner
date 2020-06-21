#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
use clap::{App, Arg};
use lazy_static::lazy_static;
use rocket::Data;
use std::io::Read;
use storage::Storage;
use crate::miner::Miner;
use crate::error::IpseError;

mod config;
mod miner;
mod error;
mod storage;
mod runtimes;

lazy_static! {
    pub static ref m: Miner::<storage::ipfs::IpfsStorage> = ();
}

pub const CONF_PATH: &'static str = "conf_path";

fn main() {
    let matches = App::new("Ipse Miner")
        .version("0.1.0")
        .about("Mining for Ipse chain")
        .arg(
            Arg::with_name(CONF_PATH)
                .short('c')
                .long(CONF_PATH)
                .default_value("config.yaml"),
        )
        .get_matches();
    let conf_fpath = matches.value_of(CONF_PATH).unwrap();

    let cfg = config::load_conf(conf_fpath);

    *m = miner::Miner::new(cfg);

    rocket::ignite().mount("/", routes![new_order, delete_order]).launch();
}

#[post("/order?<id>", data = "<file>")]
pub fn new_order(id: usize, file: Data) -> Result<(), IpseError> {
    let mut data = Vec::new();
    file.open().read(&mut data)?;
    m.write_file(id as u64, data)
}

#[delete("/order?<id>")]
pub fn delete_order(id: usize) {
    m.delete_file(id as u64)
}
