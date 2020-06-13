mod storage;
mod miner;
mod config;

use storage::Storage;
use clap::{App, Arg};

fn main() {
    let matches = App::new("Ipse Miner")
                                .version("0.1.0")
                                .about("Mining for Ipse chain")
                                .arg(Arg::with_name("ipfs_url")
                                    .short('i')
                                    .long("ipfs_url"))
                                .arg(Arg::with_name("meta_path")
                                    .short('m')
                                    .long("meta_path"))
                                .get_matches();
    let ipfs_url = matches.value_of("ipfs_url").unwrap_or("localhost:5001");
    let meta_path = matches.value_of("meta_path").unwrap_or("/ipse-miner/meta");
    let cfg = config::Conf {
        ipfs_url,
        meta_path,
    };

    let m = miner::Miner::<storage::ipfs::IpfsStorage>::new(cfg);
}
