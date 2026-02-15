use std::io::{BufRead, BufReader, Cursor};

use crate::bencode::{BencodeKey, BencodeParsable, decode_dictionary};

#[derive(Debug)]
pub struct TrackerData {
    pub interval: usize,
    // pub complete: Option<u64>,
    // pub incomplete: Option<u64>,
    pub peers: Vec<u8>,

    //converted ip:port format
    pub peers_str: Vec<String>,
    // pub failure_reason: Option<String>,
    // pub warning_message: Option<String>,
    // pub min_interval: Option<u64>,
    // pub tracker_id: Option<String>,
}

impl BencodeKey for TrackerDataKeys {
    fn is_unsupported_key(&self) -> bool {
        *self == TrackerDataKeys::UnsupportedKey
    }

    fn is_integer_field(&self) -> bool {
        match self {
            Self::Interval => true,
            _ => false,
        }
    }

    fn is_binary_field(&self) -> bool {
        match self {
            Self::Peers => true,
            _ => false,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            TrackerDataKeys::Interval => "interval",
            TrackerDataKeys::Peers => "peers",
            TrackerDataKeys::UnsupportedKey => "unsupported-key",
        }
    }

    fn from_str(key: &str) -> Self {
        match key {
            "interval" => TrackerDataKeys::Interval,
            "peers" => TrackerDataKeys::Peers,
            _ => TrackerDataKeys::UnsupportedKey,
        }
    }
}

impl From<Vec<u8>> for TrackerData {
    fn from(source: Vec<u8>) -> Self {
        let mut tracker_data = Self::default();
        let mut buf_reader = BufReader::new(Cursor::new(source));

        buf_reader.fill_buf().unwrap();
        decode_dictionary(&mut tracker_data, &mut buf_reader);

        return tracker_data;
    }
}

impl BencodeParsable for TrackerData {
    type Key = TrackerDataKeys;
    type R = Cursor<Vec<u8>>;

    fn key_from_str(s: &str) -> Self::Key {
        TrackerDataKeys::from_str(s)
    }

    fn on_integer(&mut self, key: Self::Key, value: usize) {
        match key {
            Self::Key::Interval => {
                self.interval = value;
            }
            _ => {}
        }
    }

    fn on_string_or_bytes(&mut self, key: Self::Key, value: Vec<u8>) {
        if key.is_binary_field() {
            match key {
                Self::Key::Peers => {
                    self.peers = value;
                    self.on_peers_updated();
                }
                _ => {}
            }
        }
    }
}

impl TrackerData {
    fn on_peers_updated(&mut self) {
        self.peers_str = self
            .peers
            .chunks_exact(6)
            .map(|chunk| {
                //Parsing raw peers to string. 1 byte for each part of ip address + 2 bytes for port
                let port = u16::from_be_bytes(chunk[4..6].try_into().unwrap());

                format!(
                    "{}.{}.{}.{}:{}",
                    chunk[0], chunk[1], chunk[2], chunk[3], port
                )
            })
            .collect();
    }
}

impl Default for TrackerData {
    fn default() -> Self {
        TrackerData {
            interval: 0,
            // complete: None,
            // incomplete: None,
            peers: Vec::<u8>::new(),
            peers_str: Vec::<String>::new(),
            // failure_reason: None,
            // warning_message: None,
            // min_interval: None,
            // tracker_id: None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TrackerDataKeys {
    Interval,
    // Complete,
    // Incomplete,
    Peers,
    // FailureReason,
    // WarningMessages,
    // MinInterval,
    // TrackerId,
    UnsupportedKey,
}

impl TrackerDataKeys {}
