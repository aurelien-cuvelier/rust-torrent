use std::collections::HashSet;

pub const REQUEST_PIECE_SIZE: u32 = 16u32 * 1024u32;

#[derive(Debug)]
pub enum MessageType {
    //https://wiki.theory.org/BitTorrentSpecification
    Choke = 0,         // (choke): Peer notifies that it will not send data.
    Unchoke = 1,       // (unchoke): Peer notifies that it will send data.
    Interested = 2,    // (interested): Peer expresses interest in obtaining data.
    NotInterested = 3, // (not interested): Peer expresses no interest in data.
    Have = 4,          // (have): Tells peers that a piece has been downloaded.
    Bitfield = 5, // (bitfield): Sent immediately after handshake to show all pieces a peer has.
    Request = 6,  // (request): Requests a block of data.
    Piece = 7,    // (piece): Delivers a block of data.
    Cancel = 8,   // (cancel): Cancels a previously sent request.
    Port = 9,     // (port): Used for DHT tracker connectivity.
}

impl MessageType {
    pub fn from_byte(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Choke),
            1 => Some(Self::Unchoke),
            2 => Some(Self::Interested),
            3 => Some(Self::NotInterested),
            4 => Some(Self::Have),
            5 => Some(Self::Bitfield),
            6 => Some(Self::Request),
            7 => Some(Self::Piece),
            8 => Some(Self::Cancel),
            9 => Some(Self::Port),
            _ => None,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            Self::Choke => 0,
            Self::Unchoke => 1,
            Self::Interested => 2,
            Self::NotInterested => 3,
            Self::Have => 4,
            Self::Bitfield => 5,
            Self::Request => 6,
            Self::Piece => 7,
            Self::Cancel => 8,
            Self::Port => 9,
        }
    }
}

#[derive(Debug)]
pub struct Piece {
    pub index: u32,
    pub data: Vec<u8>,
    pub received_offsets: HashSet<u32>,
    pub missing_data: usize,
}

#[derive(Debug)]
pub struct Message {
    pub msg_type: Option<MessageType>, //None for keep-alive msg as they have 0 length
    pub data: Vec<u8>,
}
