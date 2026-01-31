use std::{env::args, fs::File, process};

use rust_torrent::bencode;

fn main() {
    let args: Vec<String> = args().collect();

    if args.len() != 2 {
        panic!("No file path given in args")
    }

    let file_name = &args[1];

    let file_res = File::open(file_name);

    if file_res.is_err() {
        let err = file_res.unwrap_err();
        println!("Error opening file {}: {}", file_name, err);
        process::exit(1);
    }

    let file = file_res.unwrap();

    _ = bencode::parse_torrent_file(file)
}
