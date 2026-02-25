use std::cmp::min;
use std::collections::HashSet;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;

use log::{debug, error, info, warn};
use sha1::{Digest, Sha1};

use crate::file_handler::FileHandler;
use crate::torrent_file::TorrentFile;
use crate::torrent_net::{MessageType, get_handshake_data};

const REQUEST_PIECE_SIZE: u32 = 16u32 * 1024u32;

#[derive(Debug)]
struct Piece {
    index: u32,
    data: Vec<u8>,
    received_offsets: HashSet<u32>,
    missing_data: usize,
}

#[derive(Debug)]
struct Message {
    msg_type: Option<MessageType>, //None for keep-alive msg as they have 0 length
    data: Vec<u8>,
}

pub struct ConnectionHandler<'a> {
    peer: &'a str,
    torrent_file: &'a TorrentFile,
    file_handler: &'a mut FileHandler,
    connected: bool, //success TCP connection + validated info hash
    interested: bool,
    unchoked: bool,
    current_piece: Option<Piece>,

    bitfield: Option<Vec<u8>>,
    stream: Option<TcpStream>,
}

impl<'a> ConnectionHandler<'a> {
    pub fn new(
        peer: &'a str,
        torrent_file: &'a TorrentFile,
        file_handler: &'a mut FileHandler,
    ) -> Self {
        ConnectionHandler {
            peer,
            connected: false,
            interested: false,
            unchoked: false,
            bitfield: None,
            stream: None,
            torrent_file,
            file_handler,
            current_piece: None,
        }
    }

    fn log_err(&self, msg: &str) {
        error!("[{}] {}", self.peer, msg);
    }

    fn log_info(&self, msg: &str) {
        info!("[{}] {}", self.peer, msg);
    }

    fn log_debug(&self, msg: &str) {
        debug!("[{}] {}", self.peer, msg);
    }

    fn _log_warn(&self, msg: &str) {
        warn!("[{}] {}", self.peer, msg);
    }

    fn log_payload(&self, data: &[u8]) {
        if data.len() < 100 {
            self.log_debug(format!("payload: {:?}", data).as_str());
        } else {
            self.log_debug(
                format!("payload: {:?}...{} elements", &data[0..100], data.len()).as_str(),
            );
        }
    }

    fn start_new_piece(&mut self, piece_index: u32) {
        let is_last_piece = piece_index as usize == self.torrent_file.pieces_amount - 1;
        let piece_data_length = if is_last_piece {
            self.torrent_file.info.length
                - (piece_index as usize * self.torrent_file.info.piece_length)
        } else {
            self.torrent_file.info.piece_length
        };

        self.current_piece = Some(Piece {
            index: piece_index,
            data: Vec::with_capacity(self.torrent_file.info.piece_length),
            received_offsets: HashSet::new(),
            missing_data: piece_data_length,
        });

        self.log_info(format!("Starting new piece {:?}", self.current_piece).as_str());
    }

    fn stream_mut(&mut self) -> &mut TcpStream {
        assert!(
            self.stream.is_some(),
            "stream_mut: stream is not instantiated!"
        );

        return self.stream.as_mut().unwrap();
    }

    pub fn has_piece(&mut self, piece: usize) -> bool {
        if !self.connected {
            self.log_err("cannot check pieces for non-connected peer");
            return false;
        }

        let peer_bitfield = match &self.bitfield {
            None => {
                self.log_err("cannot check pieces before receiving bitfield");
                return false;
            }
            Some(bitfield) => bitfield,
        };

        let piece_bitfield_index = piece.div_euclid(8);
        let piece_bit_pos = 7 - piece % 8;

        if peer_bitfield.len() - 1 < piece_bitfield_index {
            self.log_err(
                format!(
                    "has bitfield of length {} but trying to reach index {piece_bitfield_index}",
                    peer_bitfield.len()
                )
                .as_str(),
            );
        }

        let pieces_in_index = peer_bitfield[piece_bitfield_index];

        return ((pieces_in_index >> piece_bit_pos) & 0b1) == 1;
    }

    fn send_intention(&mut self, msg_type: MessageType) {
        let mut raw_msg = [0u8; 5];

        raw_msg[0..4].copy_from_slice(&1u32.to_be_bytes());
        raw_msg[4] = msg_type.to_byte();

        self.log_info(format!("Sending intention: {:?}", msg_type).as_str());

        self.stream_mut().write_all(&raw_msg).unwrap();
    }

    fn request_piece(&mut self, piece: u32, offset: u32) {
        let mut raw_msg = [0u8; 17];

        //4 bytes BE => payload length
        raw_msg[0..4].copy_from_slice(&13u32.to_be_bytes());

        //1 byte => message type
        raw_msg[4] = MessageType::Request.to_byte();

        //4 bytes BE => piece index
        raw_msg[5..9].copy_from_slice(&piece.to_be_bytes());

        //4 bytes BE => offset inside the piece
        raw_msg[9..13].copy_from_slice(&offset.to_be_bytes());

        //4 bytes BE => amount of bytes to request
        // raw_msg[13..17]
        //     .copy_from_slice(&(self.torrent_file.info.piece_length as u32).to_be_bytes());

        let req_size = min(
            self.current_piece.as_ref().unwrap().missing_data as u32,
            REQUEST_PIECE_SIZE,
        );

        raw_msg[13..17].copy_from_slice(&(req_size).to_be_bytes());

        self.log_info(format!("sending request msg: {:?}", raw_msg).as_str());

        if self.current_piece.is_none() {
            self.start_new_piece(piece);
        }

        //self.waiting_for_piece = Some(piece);

        self.stream_mut().write_all(&raw_msg).unwrap();
    }

    /// * raw_msg should be the value after the payload length & msg type!
    fn handle_new_piece(&mut self, raw_msg: &[u8]) {
        let piece_index = u32::from_be_bytes(raw_msg[0..4].try_into().unwrap());
        let offset_inside_piece = u32::from_be_bytes(raw_msg[4..8].try_into().unwrap());

        let current_piece_index = self.current_piece.as_ref().unwrap().index;

        if current_piece_index != piece_index {
            self.log_err(
                format!(
                    "expected piece index {} but received {piece_index}",
                    current_piece_index
                )
                .as_str(),
            );
            return;
        }

        let block_data = &raw_msg[8..];

        let current_piece = self.current_piece.as_mut().unwrap();

        let min_required_length = (offset_inside_piece as usize) + block_data.len();
        if current_piece.data.len() < min_required_length {
            current_piece.data.resize(min_required_length, 0);
        }

        current_piece.data[(offset_inside_piece as usize)..min_required_length]
            .copy_from_slice(&block_data);

        current_piece.received_offsets.insert(offset_inside_piece);
        current_piece.missing_data -= block_data.len();

        /*
         * We shadow current_piece with a non-mut ref because otherwise we cannot
         * borrow self as non-mutable below. Removing the shadowing does not allow us to compile
         */

        let current_piece = self.current_piece.as_ref().unwrap();

        self.log_info(
                format!(
                    "received {} bytes for piece index {piece_index} & offset {offset_inside_piece} missing data: {}",
                    block_data.len(),
                    self.current_piece.as_ref().unwrap().missing_data
                )
                .as_str(),
        );

        if current_piece.missing_data > 0 {
            let max_offset_index_in_piece =
                (self.torrent_file.info.piece_length as u32).div_ceil(REQUEST_PIECE_SIZE);

            let next_offset = (0..max_offset_index_in_piece).find_map(|offset_index| {
                let offset = offset_index * REQUEST_PIECE_SIZE;
                if current_piece.received_offsets.contains(&offset) {
                    None
                } else {
                    Some(offset)
                }
            });

            if next_offset.is_none() {
                panic!(
                    "searched next offset but found none for piece index: {piece_index} received offsets: {:?}",
                    current_piece.received_offsets
                );
            }

            self.request_piece(piece_index, next_offset.unwrap());
            return;
        }

        let hash_data = Sha1::new()
            .chain_update(self.current_piece.as_ref().unwrap().data.as_slice())
            .finalize();
        let calculated_hash = hash_data.as_slice();

        let expected_hash = &self.torrent_file.info.pieces
            [(piece_index as usize * 20)..(piece_index as usize * 20) + 20];

        let hashes_match = calculated_hash == expected_hash;

        self.log_info(
            format!(
                "Piece index {} is done.\nDownloaded: {}\nExpected:   {}\nmatch: {hashes_match}",
                piece_index,
                hex::encode(calculated_hash),
                hex::encode(expected_hash)
            )
            .as_str(),
        );

        if hashes_match {
            self.file_handler.write_piece_to_file(
                piece_index as usize * self.torrent_file.info.piece_length,
                &current_piece.data,
            );
        }

        self.current_piece = None
        //let next_offset = 0..
    }

    pub fn connect(&mut self) {
        //handshakes are 68 bytes long

        self.log_info("Connecting to peer");
        let mut stream = match TcpStream::connect(self.peer) {
            Ok(stream) => {
                self.connected = true;
                stream
            }
            Err(e) => {
                self.log_err(format!("Could not initiate TCP connection with {}", e).as_str());
                return;
            }
        };

        let handshake_data = get_handshake_data(&self.torrent_file.info_hash);
        self.log_info(
            format!(
                "sending handshake data {:?}",
                String::from_utf8_lossy(&handshake_data)
            )
            .as_str(),
        );
        stream.write_all(&handshake_data).unwrap();

        let mut handshake_response = [0u8; 68];
        let received_data = stream.read(&mut handshake_response);

        if received_data.is_err() {
            self.log_err(format!("closed connection {}", received_data.unwrap_err()).as_str());

            self.connected = false;
            return;
        }

        self.log_info(
            format!(
                "received handshake response: {:?}",
                String::from_utf8_lossy(&handshake_response)
            )
            .as_str(),
        );

        let peer_id = &handshake_response[48..68];
        let info_hash = &handshake_response[28..48];

        let info_hash_match = self.torrent_file.info_hash.eq(info_hash);

        self.log_info(
            format!(
                "peer id: {} | info hash match: {}",
                String::from_utf8_lossy(peer_id),
                info_hash_match
            )
            .as_str(),
        );

        self.stream = Some(stream);
        self.send_intention(MessageType::Interested);
        self.handle_new_messages()

        //stream.shutdown(Shutdown::Both).unwrap();
    }

    fn await_next_msg(&mut self) -> Result<Message, String> {
        //4 first bytes is the payload length
        let mut payload_length_raw = [0u8; 4];
        match self.stream_mut().read_exact(&mut payload_length_raw) {
            Ok(()) => {}
            Err(e) => {
                if e.kind() == ErrorKind::UnexpectedEof {
                    return Err(String::from("peer closed the connection"));
                } else {
                    return Err(format!("stream read error: {}", e));
                }
            }
        }

        let payload_length = u32::from_be_bytes(payload_length_raw);

        if payload_length == 0 {
            self.log_debug("received keep-alive");
            return Ok(Message {
                msg_type: None,
                data: Vec::new(),
            });
        }

        let mut full_payload = vec![0u8; payload_length as usize];
        self.stream_mut().read_exact(&mut full_payload).unwrap();

        let msg_id = full_payload[0];

        let msg_type = match MessageType::from_byte(msg_id) {
            None => {
                return Err(format!("unknown message type: {}", msg_id));
            }
            Some(m) => m,
        };

        return Ok(Message {
            msg_type: Some(msg_type),
            data: full_payload,
        });
    }

    fn handle_new_messages(&mut self) {
        loop {
            let new_msg = self.await_next_msg();

            if new_msg.is_err() {
                self.log_err(format!("{}", new_msg.unwrap_err()).as_str());
                return;
            }

            let new_msg = new_msg.unwrap();

            if new_msg.data.len() == 0 {
                self.log_info("received keep-alive msg");
                continue;
            }

            let msg_type = new_msg.msg_type.unwrap();
            self.log_debug(format!("received message type: {:?}", msg_type).as_str());
            self.log_payload(&new_msg.data);

            match msg_type {
                MessageType::Choke => self.unchoked = false,
                MessageType::Unchoke => self.unchoked = true,
                MessageType::Interested => self.interested = true,
                MessageType::NotInterested => self.interested = false,

                MessageType::Have => {}
                MessageType::Bitfield => {
                    //let bitfield = &full_payload[1..];
                    let bit_field = new_msg.data[1..].to_vec();
                    let required_bitfield_length = self.torrent_file.pieces_amount.div_ceil(8);
                    if bit_field.len() != required_bitfield_length {
                        self.log_err(
                            format!(
                                "sent a bitfield of length {} while torrent needs {}",
                                bit_field.len(),
                                required_bitfield_length
                            )
                            .as_str(),
                        );

                        break;
                    }

                    self.bitfield = Some(bit_field);
                }
                MessageType::Request => {}
                MessageType::Piece => {
                    let truncated_payload = &new_msg.data[1..];
                    self.handle_new_piece(truncated_payload);
                }
                MessageType::Cancel => {}
                MessageType::Port => {}
            }

            if self.file_handler.needed_pieces.len() == 0 {
                self.log_info("closing connection has no piece left");
                break;
            }

            if self.connected
                && self.unchoked
                && self.bitfield.is_some()
                && self.current_piece.is_none()
                && self.file_handler.needed_pieces.len() > 0
            {
                self.download_next_piece();
            }
        }

        //we don't unwrap because it will panic if already closed
        let _ = self.stream_mut().shutdown(std::net::Shutdown::Both);
    }

    pub fn download_next_piece(&mut self) {
        let next_needed_piece = match self.file_handler.needed_pieces.pop_front() {
            Some(piece) => piece,
            _ => {
                return;
            }
        };

        if !self.has_piece(next_needed_piece) {
            self.file_handler
                .needed_pieces
                .push_front(next_needed_piece);
            self.log_info("putting back piece {next_needed_piece} as peer does not have it");
        }

        if self.current_piece.is_none() {
            self.start_new_piece(next_needed_piece as u32);
        }

        info!("{} has piece {next_needed_piece}", self.peer);
        self.request_piece(next_needed_piece as u32, 0u32);
    }
}
