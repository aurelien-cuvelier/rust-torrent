use reqwest;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use urlencoding::encode_binary;

use crate::client;
use crate::torrent_file::TorrentFile;
use crate::tracker_data::TrackerData;

pub fn get_announce(torrent_file: TorrentFile) {
    println!("Announcing to tracker...");
    let req = format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact=1",
        torrent_file.announce,
        encode_binary(torrent_file.info_hash.as_slice()),
        client::PEER_ID,
        6881,
        0,
        0,
        torrent_file.info.length,
        "started"
    );

    let res = reqwest::blocking::get(req);

    if res.is_err() {
        println!("server answered with error: {:#?}", res.unwrap_err());
        return;
    }

    let res_data = res.unwrap();

    println!("{:#?}", res_data);

    let body = res_data.bytes().unwrap();

    println!(
        "{:?}",
        String::from_utf8_lossy(body.clone().to_vec().as_slice())
    );

    let tracker_data = TrackerData::from(Vec::from(body));

    println!("{:?}", tracker_data);

    connect_to_peer(&torrent_file, &tracker_data.peers_str[0]);
}

fn get_handshake_data(info_hash: &[u8; 20]) -> [u8; 68] {
    /*
    * Bytes	Content
        1	Protocol string length = 19
        19	BitTorrent protocol (literal ASCII)
        8	Reserved (all zeros: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00)
        20	Info hash (your torrent’s 20-byte info hash)
        20	Peer ID (your 20-byte peer id)
    */
    let mut buf = [0u8; 68];

    buf[0] = 19;
    buf[1..20].copy_from_slice(b"BitTorrent protocol");
    //20 => 28 is alrweady 0;
    buf[28..48].copy_from_slice(info_hash);
    buf[48..68].copy_from_slice(client::PEER_ID.as_ref());
    return buf;
}

pub fn connect_to_peer(torrent_file: &TorrentFile, peer: &str) {
    //handshakes are 68 bytes long
    let mut stream = TcpStream::connect(peer).unwrap();

    let handshake_data = get_handshake_data(&torrent_file.info_hash);
    println!(
        "\n\nsending handshake to {} data {:?}",
        peer,
        String::from_utf8_lossy(&handshake_data)
    );
    stream.write_all(&handshake_data).unwrap();

    let mut response = [0u8; 68];
    stream.read_exact(&mut response).unwrap();

    println!(
        "\n\nreceived handshake response: {:?}",
        String::from_utf8_lossy(&response)
    );

    let peer_id = &response[48..68];
    let info_hash = &response[28..48];

    println!(
        "\n\n{peer} - {}\ninfo hash match: {}",
        String::from_utf8(peer_id.to_vec()).unwrap(),
        torrent_file.info_hash.eq(info_hash)
    );

    stream.shutdown(Shutdown::Both).unwrap();
}
