use std::{env::args, fs::File, process};

use dotenvy;
use log::error;
use rust_torrent::{torrent_file::TorrentFile, torrent_net};

fn main() {
    dotenvy::dotenv().unwrap();
    env_logger::init();

    let args: Vec<String> = args().collect();

    if args.len() != 2 {
        panic!("No file path given in args")
    }

    let file_name = &args[1];

    let file_res = File::open(file_name);

    if file_res.is_err() {
        let err = file_res.unwrap_err();
        error!("Error opening file {}: {}", file_name, err);
        process::exit(1);
    }

    let file = file_res.unwrap();
    let torrent = TorrentFile::from(file);

    torrent_net::get_announce(torrent);
}
