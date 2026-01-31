use rust_torrent::bencode;
use std::fs::File;

#[test]
fn test_unsupported_torrent_keys() {
    let file = File::open("./tests/test_minimal.torrent").unwrap();

    _ = bencode::parse_torrent_file(file);
}
