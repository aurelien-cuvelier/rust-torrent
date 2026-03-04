mod handlers;
mod message;

use std::cmp::min;
use std::collections::{HashSet, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;

use log::{debug, error, info, warn};

use crate::file_handler::FileHandler;

use crate::torrent_file::TorrentFile;
use crate::tracker::get_handshake_data;
use message::{Message, MessageType, Piece, REQUEST_PIECE_SIZE};

pub struct ConnectionHandler<'a> {
    peer: &'a str,
    torrent_file: &'a TorrentFile,
    file_handler: &'a mut FileHandler,
    connected: bool, //success TCP connection + validated info hash
    peer_interested: bool,
    peer_unchoked: bool,
    current_piece: Option<Piece>,
    next_downloadable_pieces: VecDeque<usize>,
    peer_has_missing_pieces: bool,

    peer_bitfield: Option<Vec<u8>>,
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
            peer_interested: false,
            peer_unchoked: false,
            peer_bitfield: None,
            stream: None,
            torrent_file,
            file_handler,
            current_piece: None,
            next_downloadable_pieces: VecDeque::new(),

            //setting this to true at ini because we don't know
            peer_has_missing_pieces: true,
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
                format!("payload: {:?}...{} elements", &data[0..32], data.len()).as_str(),
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

        let peer_bitfield = match &self.peer_bitfield {
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

    fn send_bitfield(&mut self) {
        let mut raw_msg = vec![0u8; 4 + 1 + self.file_handler.bitfield.len()];

        raw_msg[0..4]
            .copy_from_slice(&(1u32 + self.file_handler.bitfield.len() as u32).to_be_bytes());

        raw_msg[4] = MessageType::Bitfield.to_byte();

        raw_msg[5..].copy_from_slice(&self.file_handler.bitfield);

        self.log_info("Sending bitfield");

        self.stream_mut().write_all(&raw_msg).unwrap();
    }

    fn send_have(&mut self, piece_index: u32) {
        let mut raw_msg = [0u8; 4 + 1 + 4];

        raw_msg[0..4].copy_from_slice(&(1u32 + 4u32).to_be_bytes());
        raw_msg[4] = MessageType::Have.to_byte();
        raw_msg[5..9].copy_from_slice(&piece_index.to_be_bytes());

        self.log_info(format!("sending have: {piece_index}").as_str());

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

        self.log_info(format!("sending request msg for piece index {piece} offset {offset} length {req_size}\n{:?}", raw_msg).as_str());

        if self.current_piece.is_none() {
            self.start_new_piece(piece);
        }

        //self.waiting_for_piece = Some(piece);

        self.stream_mut().write_all(&raw_msg).unwrap();
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
        self.send_bitfield();
        if self.file_handler.needed_pieces.len() > 0 {
            self.send_intention(MessageType::Interested);
        } else {
            self.send_intention(MessageType::NotInterested);
        }

        if self.file_handler.needed_pieces.len() < self.torrent_file.pieces_amount {
            //we already have some pieces to we can unchoke
            self.send_intention(MessageType::Unchoke);
        }

        self.run_message_loop();

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

    fn run_message_loop(&mut self) {
        loop {
            let new_msg = self.await_next_msg();

            if new_msg.is_err() {
                self.log_err(format!("{}", new_msg.unwrap_err()).as_str());
                return;
            }

            let new_msg = new_msg.unwrap();

            if new_msg.data.len() == 0 {
                self.log_info("received keep-alive msg");
                /*
                 * Currently sending KA message in response to a KA message as
                 * we are running sync single thread.
                 * Should implement own KA timer with multi thread implem
                 */
                self.send_keep_alive();
                continue;
            }

            let msg_type = new_msg.msg_type.unwrap();
            self.log_debug(format!("received message type: {:?}", msg_type).as_str());
            self.log_payload(&new_msg.data);

            match msg_type {
                MessageType::Choke => self.peer_unchoked = false,
                MessageType::Unchoke => self.peer_unchoked = true,
                MessageType::Interested => self.peer_interested = true,
                MessageType::NotInterested => self.peer_interested = false,

                MessageType::Have => {
                    assert!(
                        new_msg.data.len() == 5,
                        "received HAVE msg with length of {}",
                        new_msg.data.len()
                    );
                    self.handle_have(&new_msg.data[1..]);
                }
                MessageType::Bitfield => {
                    self.handle_bitfield(&new_msg.data[1..]);
                }
                MessageType::Request => {
                    self.handle_request_piece(&new_msg.data[1..]);
                }
                MessageType::Piece => {
                    let truncated_payload = &new_msg.data[1..];
                    self.handle_new_piece(truncated_payload);
                }
                MessageType::Cancel => {}
                MessageType::Port => {}
            }

            if !self.connected {
                /*
                 * Should not happen because we connect on top of
                 * the loop, and return in case of disconnect.
                 * So most likely a bug if came here
                 */
                self.log_err("peer is not connected");
                break;
            }

            if self.peer_bitfield.is_some()
                && (!self.peer_interested && self.file_handler.needed_pieces.len() == 0)
            {
                self.log_info(
                    "dropping connection as we downloaded all and peer is not interested",
                );
                break;
            }

            if self.peer_unchoked && self.peer_bitfield.is_some() && self.current_piece.is_none() {
                //plan new next pieces available from peer if the inner buffer is empty
                self.plan_next_pieces();

                if self.next_downloadable_pieces.len() > 0 {
                    let next_piece = self.next_downloadable_pieces.pop_front().unwrap();
                    self.start_new_piece(next_piece as u32);
                    self.request_piece(next_piece as u32, 0u32);
                }

                if self.peer_has_missing_pieces {}

                //self.download_next_piece();
            }
        }

        //we don't unwrap because it will panic if already closed
        let _ = self.stream_mut().shutdown(std::net::Shutdown::Both);
    }

    pub fn plan_next_pieces(&mut self) {
        if self.next_downloadable_pieces.len() > 0 {
            return;
        }

        let mut not_available_pieces = HashSet::<usize>::new();

        loop {
            let next_needed_piece = self.file_handler.needed_pieces.pop_front();

            if next_needed_piece.is_none() {
                break;
            }

            let next_needed_piece = next_needed_piece.unwrap();

            if !self.has_piece(next_needed_piece) {
                not_available_pieces.insert(next_needed_piece);
                continue;
            }

            self.next_downloadable_pieces.push_back(next_needed_piece);

            if self.next_downloadable_pieces.len() > 4 {
                break;
            }
        }

        not_available_pieces.iter().for_each(|piece| {
            self.file_handler.needed_pieces.push_front(*piece);
        });
        self.log_info(
            format!(
                "putting back {} pieces that peer does not have\n{:?}",
                not_available_pieces.len(),
                not_available_pieces
            )
            .as_str(),
        );

        if self.file_handler.needed_pieces.len() == 0 {
            self.log_info(format!("Done downloading all torrent pieces",).as_str());
            self.send_intention(MessageType::NotInterested);
        } else if self.next_downloadable_pieces.len() == 0 {
            self.log_info("cannot download any more pieces from peer");
        }
    }

    pub fn update_peer_bitfield(&mut self, piece_index: u32, available: bool) {
        let bitfield = self.peer_bitfield.as_mut().unwrap();

        let index_in_bitfield = piece_index.div_euclid(8) as usize;
        let bit_offset = 7 - (piece_index % 8);

        if available {
            bitfield[index_in_bitfield] |= 1 << bit_offset;
        } else {
            bitfield[index_in_bitfield] &= !(1 << bit_offset);
        }
    }

    pub fn send_keep_alive(&mut self) {
        self.log_info("Sending keep-alive msg");
        self.stream_mut().write_all(&0u32.to_be_bytes()).unwrap();
    }
}
