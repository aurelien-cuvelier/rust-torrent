use std::fmt;

pub struct MetaInfo {
    pub name: String,
    pub length: usize,
    pub piece_length: usize,
    pub pieces: Vec<u8>,
    pub md5sum: Option<String>,
    pub sha1: Option<[u8; 20]>,
    pub sha256: Option<[u8; 32]>,
}

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

#[derive(PartialEq, Debug)]
pub enum TorrentKeys {
    Announce,
    AnnounceList,
    Info,
    Comment,
    CreatedBy,
    CreationDate,
    Name,
    Length,
    Pieces,
    PieceLength,
    Md5Sum,
    Sha256,
    Sha1,
    Sources,
    UrlList,
}

impl TorrentKeys {
    pub fn is_binary_field(&self) -> bool {
        match self {
            TorrentKeys::Pieces => true,
            TorrentKeys::Sha1 => true,
            TorrentKeys::Sha256 => true,
            _ => false,
        }
    }

    pub fn is_integer_field(&self) -> bool {
        match self {
            TorrentKeys::CreationDate => true,
            _ => false,
        }
    }

    pub fn from_str(key: &str) -> Result<Self, String> {
        match key {
            "announce" => Ok(TorrentKeys::Announce),
            "announce-list" => Ok(TorrentKeys::AnnounceList),
            "info" => Ok(TorrentKeys::Info),
            "comment" => Ok(TorrentKeys::Comment),
            "created by" => Ok(TorrentKeys::CreatedBy),
            "creation date" => Ok(TorrentKeys::CreationDate),
            "name" => Ok(TorrentKeys::Name),
            "length" => Ok(TorrentKeys::Length),
            "pieces" => Ok(TorrentKeys::Pieces),
            "piece length" => Ok(TorrentKeys::PieceLength),
            "md5sum" => Ok(TorrentKeys::Md5Sum),
            "sha256" => Ok(TorrentKeys::Sha256),
            "sha1" => Ok(TorrentKeys::Sha1),
            "sources" => Ok(TorrentKeys::Sources),
            "url-list" => Ok(TorrentKeys::UrlList),
            _ => Err(format!("{key} is not a torrent key")),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            TorrentKeys::Announce => "announce",
            TorrentKeys::AnnounceList => "announce-list",
            TorrentKeys::Info => "info",
            TorrentKeys::Comment => "comment",
            TorrentKeys::CreatedBy => "created by",
            TorrentKeys::CreationDate => "creation date",
            TorrentKeys::Name => "name",
            TorrentKeys::Length => "length",
            TorrentKeys::Pieces => "pieces",
            TorrentKeys::PieceLength => "piece length",
            TorrentKeys::Md5Sum => "md5sum",
            TorrentKeys::Sha256 => "sha256",
            TorrentKeys::Sha1 => "sha1",
            TorrentKeys::Sources => "sources",
            TorrentKeys::UrlList => "url-list",
        }
    }
}
