use std::{env::args, fs::File, process};

use dotenvy;
use log::{error, info};
use rust_torrent::{file_download, torrent_file::TorrentFile, torrent_net};

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

    let res_tracker_data = torrent_net::get_tracker_data(&torrent);

    if res_tracker_data.is_err() {
        error!(
            "error getting tracker data: {}",
            res_tracker_data.unwrap_err()
        );
        process::exit(1)
    }

    let tracker_data = res_tracker_data.unwrap();

    info!("{:?}", tracker_data);

    file_download::get_file_handlder(&torrent, &tracker_data);

    torrent_net::get_connections_handler(&tracker_data, Some(1));

    println!("END");
}
