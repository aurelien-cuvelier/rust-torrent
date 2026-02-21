use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};

use log::{debug, error, info, warn};

use crate::torrent_file::TorrentFile;
use crate::torrent_net::{MessageType, get_handshake_data};

pub struct ConnectionHandler<'a> {
    peer: &'a str,
    connected: bool, //success TCP connection + validated info hash
    interested: bool,
    unchoked: bool,
    bitfield: Vec<u8>,
}

impl<'a> ConnectionHandler<'a> {
    pub fn new(peer: &'a str) -> Self {
        ConnectionHandler {
            peer,
            connected: false,
            interested: false,
            unchoked: false,
            bitfield: Vec::new(),
        }
    }

    pub fn connect(&mut self, torrent_file: &TorrentFile, peer: &str) {
        //handshakes are 68 bytes long

        info!("Connection to {peer}");
        let mut stream = match TcpStream::connect(peer) {
            Ok(stream) => stream,
            Err(e) => {
                error!("Could not initiate TCP connection with {peer} {}", e);
                return;
            }
        };

        let handshake_data = get_handshake_data(&torrent_file.info_hash);
        info!(
            "{peer} => sending handshake data {:?}",
            String::from_utf8_lossy(&handshake_data)
        );
        stream.write_all(&handshake_data).unwrap();

        let mut handshake_response = [0u8; 68];
        let received_data = stream.read(&mut handshake_response);

        if received_data.is_err() {
            error!(
                "peer {} closed connection {}",
                peer,
                received_data.unwrap_err()
            );
            return;
        }

        info!(
            "{peer} => received handshake response: {:?}",
            String::from_utf8_lossy(&handshake_response)
        );

        let peer_id = &handshake_response[48..68];
        let info_hash = &handshake_response[28..48];

        let info_hash_match = torrent_file.info_hash.eq(info_hash);

        info!(
            "{peer} - {}\n{peer} => info hash match: {}",
            String::from_utf8_lossy(peer_id),
            info_hash_match
        );

        loop {
            //4 first bytes is the payload length
            let mut payload_length_raw = [0u8; 4];
            match stream.read_exact(&mut payload_length_raw) {
                Ok(()) => {}
                Err(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        info!("{peer} => peer closed the connection");
                    } else {
                        error!("{peer} => stream read error: {}", e);
                    }
                    break;
                }
            }

            let payload_length = u32::from_be_bytes(payload_length_raw);

            if payload_length == 0 {
                debug!("{peer} => received keep-alive");
                continue;
            }

            let mut full_payload = vec![0u8; payload_length as usize];
            stream.read_exact(&mut full_payload).unwrap();

            let msg_id = full_payload[0];

            let msg_type = match MessageType::from_byte(msg_id) {
                None => {
                    warn!("{peer} => unknown message type: {}", msg_id);
                    continue;
                }
                Some(m) => m,
            };

            debug!("{peer} => received message type: {:?}", msg_type);
            debug!("{peer} => payload: {:?}", full_payload);

            match msg_type {
                MessageType::Choke => {
                    self.unchoked = false;
                }
                MessageType::Unchoke => {
                    self.unchoked = true;
                }
                MessageType::Interested => {
                    self.interested = true;
                }
                MessageType::NotInterested => self.interested = false,
                MessageType::Have => {}
                MessageType::Bitfield => {
                    //let bitfield = &full_payload[1..];
                    self.bitfield = full_payload[1..].to_vec();
                }
                MessageType::Request => {}
                MessageType::Piece => {}
                MessageType::Cancel => {}
                MessageType::Port => {}
            }
        }

        stream.shutdown(Shutdown::Both).unwrap();
    }
}
