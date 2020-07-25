#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
use crate::error::IpseError;
use crate::miner::Miner;
use clap::{App, Arg};
use once_cell::sync::Lazy;
use rocket::Data;
use std::io::Read;
use std::sync::Mutex;

mod calls;
mod config;
mod error;
mod miner;
mod storage;

static MINER: Lazy<Mutex<Miner>> = Lazy::new(|| {
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
    Mutex::new(miner::Miner::new(cfg))
});

pub const CONF_PATH: &'static str = "conf_path";

fn main() {
    rocket::ignite()
        .mount("/", routes![new_order, delete_order])
        .launch();
}

#[post("/order?<id>", data = "<file>")]
pub fn new_order(id: usize, file: Data) -> Result<String, IpseError> {
    let mut data = Vec::new();
    file.open().read(&mut data)?;
    MINER.lock().unwrap().write_file(id as i64, data)
}

#[delete("/order?<id>")]
pub fn delete_order(id: usize) -> Result<(), IpseError> {
    MINER.lock().unwrap().delete_file(id as i64)
}
