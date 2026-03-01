use std::{fmt, fs::File};

use crate::bencode::{BencodeKey, BencodeParsable};

#[derive(PartialEq, Debug, Clone)]
pub enum MetaInfoKeys {
    Name,
    Length,
    Pieces,
    PieceLength,
    Md5Sum,
    Sha256,
    Sha1,

    UnsupportedKey,
}

pub struct MetaInfo {
    pub name: String,
    pub length: usize,
    pub piece_length: usize,
    pub pieces: Vec<u8>,
    pub md5sum: Option<String>,
    pub sha1: Option<[u8; 20]>,
    pub sha256: Option<[u8; 32]>,
}

impl BencodeParsable for MetaInfo {
    type Key = MetaInfoKeys;
    type R = File;

    fn key_from_str(s: &str) -> Self::Key {
        return MetaInfoKeys::from_str(s);
    }

    fn on_string_or_bytes(&mut self, key: Self::Key, value: Vec<u8>) {
        assert!(
            key.is_binary_field() || key.is_string_field(),
            "{} is neither string or bytes type",
            key.as_str()
        );

        if key.is_string_field() {
            match key {
                Self::Key::Name => self.name = String::from_utf8(value).unwrap(),
                Self::Key::Md5Sum => self.md5sum = Some(String::from_utf8(value).unwrap()),
                _ => {}
            }
        } else if key.is_binary_field() {
            match key {
                Self::Key::Sha1 => self.sha1 = Some(value.try_into().unwrap()),
                Self::Key::Sha256 => self.sha256 = Some(value.try_into().unwrap()),
                Self::Key::Pieces => self.pieces = value,
                _ => {}
            }
        }
    }

    fn on_integer(&mut self, key: Self::Key, value: usize) {
        assert!(
            key.is_integer_field(),
            "{} is not an integer field",
            key.as_str()
        );

        match key {
            Self::Key::PieceLength => self.piece_length = value,
            Self::Key::Length => self.length = value,
            _ => {}
        }
    }
}

impl fmt::Debug for MetaInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MetaInfo")
            .field("name", &self.name)
            .field("length", &self.length)
            .field("piece_length", &self.piece_length)
            .field("md5sum", &self.md5sum)
            .field("sha1", &self.sha1)
            .field("sha256", &self.sha256)
            .field(
                "pieces",
                &if self.pieces.len() > 40 {
                    format!(
                        "[{:?}, {} total pieces...]",
                        &self.pieces[..40],
                        self.pieces.len()
                    )
                } else {
                    format!("{:?}", &self.pieces)
                },
            )
            .finish()
    }
}

impl BencodeKey for MetaInfoKeys {
    fn is_unsupported_key(&self) -> bool {
        *self == Self::UnsupportedKey
    }
    fn as_str(&self) -> &str {
        match self {
            Self::Name => "name",
            Self::Length => "length",
            Self::Pieces => "pieces",
            Self::PieceLength => "piece length",
            Self::Md5Sum => "md5sum",
            Self::Sha256 => "sha256",
            Self::Sha1 => "sha1",
            Self::UnsupportedKey => "unsupported-key",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "name" => Self::Name,
            "length" => Self::Length,
            "pieces" => Self::Pieces,
            "piece length" => Self::PieceLength,
            "md5sum" => Self::Md5Sum,
            "sha256" => Self::Sha256,
            "sha1" => Self::Sha1,
            _ => Self::UnsupportedKey,
        }
    }

    fn is_string_field(&self) -> bool {
        match self {
            Self::Name | Self::Md5Sum => true,
            _ => false,
        }
    }

    fn is_list_field(&self) -> bool {
        match self {
            Self::Pieces | Self::Sha1 | Self::Sha256 => true,
            _ => false,
        }
    }

    fn is_integer_field(&self) -> bool {
        match self {
            Self::Length | Self::PieceLength => true,
            _ => false,
        }
    }

    fn is_binary_field(&self) -> bool {
        match self {
            Self::Sha1 | Self::Sha256 | Self::Pieces => true,
            _ => true,
        }
    }
}
