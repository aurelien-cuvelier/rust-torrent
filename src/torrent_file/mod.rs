use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Seek},
};

use sha1::{Digest, Sha1};

mod meta_info;

use meta_info::MetaInfo;

use crate::bencode::{BencodeKey, BencodeParsable, decode_dictionary};

#[derive(Debug)]
pub struct TorrentFile {
    pub info: MetaInfo,
    pub announce: String,
    pub announce_list: Option<Vec<Vec<String>>>,
    pub comment: String,
    pub created_by: String,
    pub creation_date: usize,
    pub sources: Option<Vec<String>>,
    pub url_list: Option<Vec<String>>,

    //not part of the actual torrent BEP impl, but convenient to keep it here
    pub info_hash: [u8; 20],
    pub info_hash_str: String,
    pub pieces_amount: usize,
}

impl Default for TorrentFile {
    fn default() -> Self {
        TorrentFile {
            info: MetaInfo {
                name: String::new(),
                length: 0,
                piece_length: 0,
                pieces: Vec::new(),
                md5sum: None,
                sha1: None,
                sha256: None,
            },
            announce: String::new(),
            announce_list: None,
            comment: String::new(),
            created_by: String::new(),
            creation_date: 0,
            sources: None,
            url_list: None,

            info_hash: [0u8; 20],
            info_hash_str: String::new(),
            pieces_amount: 0,
        }
    }
}

impl BencodeKey for TorrentKeys {
    fn is_unsupported_key(&self) -> bool {
        *self == Self::UnsupportedKey
    }

    fn is_dictionary_field(&self) -> bool {
        match self {
            Self::Info => true,
            _ => false,
        }
    }

    fn is_list_field(&self) -> bool {
        match self {
            Self::AnnounceList | Self::Sources | Self::UrlList => true,
            _ => false,
        }
    }

    fn is_integer_field(&self) -> bool {
        match self {
            Self::CreationDate => true,
            _ => false,
        }
    }

    fn is_string_field(&self) -> bool {
        match self {
            Self::Info | Self::Comment | Self::CreatedBy | Self::Announce => true,
            _ => false,
        }
    }

    fn is_list_of_strings(&self) -> bool {
        match self {
            Self::Sources | Self::UrlList => true,
            _ => false,
        }
    }

    fn is_nested_list_string(&self) -> bool {
        return *self == Self::AnnounceList;
    }

    fn from_str(key: &str) -> Self {
        match key {
            "announce" => TorrentKeys::Announce,
            "announce-list" => TorrentKeys::AnnounceList,
            "info" => TorrentKeys::Info,
            "comment" => TorrentKeys::Comment,
            "created by" => TorrentKeys::CreatedBy,
            "creation date" => TorrentKeys::CreationDate,

            "sources" => TorrentKeys::Sources,
            "url-list" => TorrentKeys::UrlList,
            _ => TorrentKeys::UnsupportedKey,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Announce => "announce",
            Self::AnnounceList => "announce-list",
            Self::Info => "info",
            Self::Comment => "comment",
            Self::CreatedBy => "created by",
            Self::CreationDate => "creation date",

            Self::Sources => "sources",
            Self::UrlList => "url-list",
            Self::UnsupportedKey => "unsupported-key",
        }
    }
}

impl From<File> for TorrentFile {
    fn from(source: File) -> Self {
        let mut torrent_file = Self::default();
        let mut buf_reader = BufReader::new(source);

        //making sure buffer is not empty as start
        buf_reader.fill_buf().unwrap();
        decode_dictionary(&mut torrent_file, &mut buf_reader);
        return torrent_file;
    }
}

impl BencodeParsable for TorrentFile {
    type Key = TorrentKeys;
    type R = File;

    fn key_from_str(s: &str) -> Self::Key {
        TorrentKeys::from_str(s)
    }

    fn on_nested_list_string(&mut self, key: Self::Key, value: Vec<Vec<String>>) {
        match key {
            Self::Key::AnnounceList => self.announce_list = Some(value),
            _ => {}
        }
    }

    fn on_list_string(&mut self, key: Self::Key, value: Vec<String>) {
        match key {
            Self::Key::Sources => self.sources = Some(value),
            Self::Key::UrlList => self.url_list = Some(value),
            _ => {}
        }
    }

    fn on_integer(&mut self, key: Self::Key, value: usize) {
        match key {
            Self::Key::CreationDate => {
                self.creation_date = value;
            }
            _ => {}
        };
    }

    fn on_string_or_bytes(&mut self, key: Self::Key, data: Vec<u8>) {
        //no binary field in torrent
        if key.is_string_field() {
            let string_data = String::from_utf8(data).unwrap();

            match key {
                Self::Key::Comment => self.comment = string_data,
                Self::Key::CreatedBy => self.created_by = string_data,
                Self::Key::Announce => self.announce = string_data,
                _ => {}
            };
        }
    }

    fn on_dictionary(&mut self, key: Self::Key, buf_reader: &mut BufReader<Self::R>) {
        match key {
            Self::Key::Info => {
                let info_index_start = buf_reader.stream_position().unwrap();
                decode_dictionary(&mut self.info, buf_reader);
                let info_index_end = buf_reader.stream_position().unwrap();

                let info_data_length: usize =
                    (info_index_end - info_index_start).try_into().unwrap();
                let mut info_raw_bytes = vec![0u8; info_data_length];

                buf_reader
                    .seek(std::io::SeekFrom::Start(info_index_start))
                    .unwrap();

                buf_reader.read_exact(&mut info_raw_bytes).unwrap();

                let mut hasher = Sha1::new();

                hasher.update(&info_raw_bytes);

                self.info_hash = hasher.finalize().try_into().unwrap();
                self.info_hash_str = hex::encode(self.info_hash);

                self.pieces_amount = self.info.length.div_ceil(self.info.piece_length);
            }
            _ => {
                decode_dictionary(&mut self.info, buf_reader);
            }
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum TorrentKeys {
    Announce,
    AnnounceList,
    Info,
    Comment,
    CreatedBy,
    CreationDate,

    Sources,
    UrlList,

    //We use this key whenever we meeta key in the torrent file that we don't support
    UnsupportedKey,
}
