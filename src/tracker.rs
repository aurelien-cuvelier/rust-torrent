use reqwest;
use urlencoding::encode_binary;

use crate::client;
use crate::connection_handler::ConnectionHandler;
use crate::file_handler::FileHandler;
use crate::torrent_file::TorrentFile;
use crate::tracker_data::TrackerData;

pub fn get_tracker_data(torrent_file: &TorrentFile) -> Result<TrackerData, String> {
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
        return Err(format!("server answered with: {:?}", res.unwrap_err()));
    }

    let res_data = res.unwrap();

    println!("{:#?}", res_data);

    let body = res_data.bytes().unwrap();

    println!(
        "{:?}",
        String::from_utf8_lossy(body.clone().to_vec().as_slice())
    );

    let tracker_data = TrackerData::from(Vec::from(body));

    return Ok(tracker_data);
}

pub fn get_handshake_data(info_hash: &[u8; 20]) -> [u8; 68] {
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

pub fn get_connections_handler<'a>(
    torrent_file: &'a TorrentFile,
    _tracker_data: &'a TrackerData,
    file_handler: &'a mut FileHandler,
    _max_peers: Option<usize>,
) -> Vec<ConnectionHandler<'a>> {
    let mut _connections = Vec::<ConnectionHandler<'a>>::new();

    let mut connection_handler =
        ConnectionHandler::new("127.0.0.1:57496", torrent_file, file_handler);
    connection_handler.connect();

    return _connections;
}
