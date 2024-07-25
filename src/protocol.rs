use anyhow::{bail, Error};
use chrono::Utc;
use md5::{Digest, Md5};
use positioned_io::{ReadAt, WriteAt};
use serde_bytes::ByteBuf;
use serde_derive::{Deserialize, Serialize};
use std::io::{self, BufReader, Read};
use std::os::unix::fs::OpenOptionsExt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MyPkg {
    pub name: String,
    pub md5sum: String, // sum of all md5sum in vec<file>
    // pub version: u64,
    // pub ephemeral: String,
    pub author: String,
    pub built_on: i64,
    pub expires: Option<i64>,
    pub os: String,
    pub arch: String,
    pub tags: Vec<String>,
    pub commit: String,
    pub files: Vec<File>,
}

impl MyPkg {
    pub fn new(name: String, paths: Vec<String>) -> Result<MyPkg, Error> {
        let files: Vec<File> = paths.iter().map(|p| hash_file(&p).unwrap()).collect();
        let all_files_hasher = files.iter().fold(Md5::new(), |mut hasher, v| {
            hasher.update(&v.md5sum);
            hasher
        });
        return Ok(MyPkg {
            name,
            md5sum: format!("{:x}", all_files_hasher.finalize()),
            // version: todo!(), // refactor
            // ephemeral: todo!(), // refactor
            author: "Joe".into(),
            built_on: Utc::now().timestamp_millis(),
            expires: None,
            os: "macos".into(),
            arch: "arm64".into(),
            tags: vec![],
            commit: "dirty".into(),
            files,
        });
    }
    pub fn load() {}
    pub fn write() {}
}

pub const BLOCK_SIZE: usize = 16384;
pub const MSG_SIZE: usize = 2; // prefix len is u16
pub const MSG_TYPE: usize = 2; // msg type is a u16 and represents the max number of message types our protocol has
pub const HEADER_SIZE: usize = MSG_SIZE + MSG_TYPE;
pub const BLOCK_SIZE_LESS_HEADER: usize = BLOCK_SIZE - HEADER_SIZE;

fn hash_file(p: &str) -> Result<File, Error> {
    let f = std::fs::File::open(p)?;
    let mut reader = BufReader::new(f);
    let mut buf = [0; BLOCK_SIZE];
    let mut hasher = Md5::new();
    let mut total_read = 0;
    while let Ok(n) = reader.read(&mut buf) {
        total_read += n;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(File {
        path: p.to_string(),
        length: total_read as u64,
        md5sum: format!("{:x}", hasher.finalize()),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    pub path: String,
    pub length: u64, // byte len of file
    pub md5sum: String,
}

impl File {
    pub fn filename(&self) -> String {
        let path = std::path::Path::new(&self.path);
        if let Some(filename) = path.file_name() {
            return filename.to_string_lossy().to_string();
        }
        self.path.to_owned()
    }
    pub fn chunk_count(self) -> usize {
        let block_size = BLOCK_SIZE as u64;
        let mut s = self.length / block_size;
        if self.length & block_size != 0 {
            s += 1;
        }
        if s == 0 {
            // tiny files are zero due to int division
            return 1;
        }
        s as usize
    }
    pub fn read_at(
        self,
    ) -> Result<Box<dyn Fn(u64, &mut [u8; BLOCK_SIZE]) -> io::Result<usize>>, Error> {
        let f = std::fs::File::open(self.path)?;
        let block_size = BLOCK_SIZE as u64;
        let capturing_closure =
            move |p: u64, buf: &mut [u8; BLOCK_SIZE]| f.read_at(p * block_size, buf);
        Ok(Box::new(capturing_closure)
            as Box<
                dyn Fn(u64, &mut [u8; BLOCK_SIZE]) -> io::Result<usize>,
            >)
    }
    pub fn write_at(
        self,
        path: String,
    ) -> Result<Box<dyn FnMut(u64, &[u8]) -> io::Result<usize> + Send>, Error> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true) // Create the file if it doesn't exist
            .mode(0o644) // Set file permissions
            .open(path)?;
        let block_size = BLOCK_SIZE as u64;
        let capturing_closure = move |p: u64, buf: &[u8]| f.write_at(p * block_size, buf);
        Ok(Box::new(capturing_closure) as Box<dyn FnMut(u64, &[u8]) -> io::Result<usize> + Send>)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MyPkgAck {
    pub md5sum: Option<String>, // sum of all md5sum in vec<file>
    pub files: Option<Vec<File>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NegotiateMyPkg {
    pub md5sum: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NegotiateMyPkgAck {
    pub md5sum: String,
    pub peers: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PieceExchange {
    pub pieces: [u64; 2],
    pub file: File,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PieceExchangeAck {
    // todo use pieces to signal a resume. for now this is always None
    pub pieces: Option<[u64; 2]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Piece {
    pub piece: u64,
    pub ack: Option<u64>,
    pub data: ByteBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PieceAck {
    pub piece: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Done {
    pub md5sum: String,
}

pub enum MessageType {
    MyPkg(MyPkg),
    File(File),
    MyPkgAck(MyPkgAck),
    NegotiateMyPkg(NegotiateMyPkg),
    NegotiateMyPkgAck(NegotiateMyPkgAck),
    PieceExchange(PieceExchange),
    PieceExchangeAck(PieceExchangeAck),
    Piece(Piece),
    PieceAck(PieceAck),
    Done(Done),
}
impl MessageType {
    pub fn is_valid_message_type(value: u16) -> bool {
        matches!(value, 10 | 20 | 30 | 40 | 50 | 60 | 70 | 80 | 90 | 100)
    }
    pub fn message_type(&self) -> u16 {
        match self {
            MessageType::MyPkg(_) => 10,
            MessageType::File(_) => 20,
            MessageType::MyPkgAck(_) => 30,
            MessageType::NegotiateMyPkg(_) => 40,
            MessageType::NegotiateMyPkgAck(_) => 50,
            MessageType::PieceExchange(_) => 60,
            MessageType::PieceExchangeAck(_) => 70,
            MessageType::Piece(_) => 80,
            MessageType::PieceAck(_) => 90,
            MessageType::Done(_) => 100,
        }
    }

    pub fn serialize_inner(&self) -> Result<Vec<u8>, serde_bencode::Error> {
        match self {
            MessageType::MyPkg(inner) => serde_bencode::to_bytes(inner),
            MessageType::File(inner) => serde_bencode::to_bytes(inner),
            MessageType::MyPkgAck(inner) => serde_bencode::to_bytes(inner),
            MessageType::NegotiateMyPkg(inner) => serde_bencode::to_bytes(inner),
            MessageType::NegotiateMyPkgAck(inner) => serde_bencode::to_bytes(inner),
            MessageType::PieceExchange(inner) => serde_bencode::to_bytes(inner),
            MessageType::PieceExchangeAck(inner) => serde_bencode::to_bytes(inner),
            MessageType::Piece(inner) => serde_bencode::to_bytes(inner),
            MessageType::PieceAck(inner) => serde_bencode::to_bytes(inner),
            MessageType::Done(inner) => serde_bencode::to_bytes(inner),
        }
        // .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
    pub fn deserialize(v: u16, raw_msg: &Vec<u8>) -> Result<MessageType, Error> {
        match v {
            10 => Ok(MessageType::MyPkg(serde_bencode::from_bytes::<MyPkg>(
                raw_msg,
            )?)),
            20 => Ok(MessageType::File(serde_bencode::from_bytes::<File>(
                raw_msg,
            )?)),
            30 => Ok(MessageType::MyPkgAck(
                serde_bencode::from_bytes::<MyPkgAck>(raw_msg)?,
            )),
            40 => Ok(MessageType::NegotiateMyPkg(serde_bencode::from_bytes::<
                NegotiateMyPkg,
            >(raw_msg)?)),
            50 => Ok(MessageType::NegotiateMyPkgAck(serde_bencode::from_bytes::<
                NegotiateMyPkgAck,
            >(raw_msg)?)),
            60 => Ok(MessageType::PieceExchange(serde_bencode::from_bytes::<
                PieceExchange,
            >(raw_msg)?)),
            70 => Ok(MessageType::PieceExchangeAck(serde_bencode::from_bytes::<
                PieceExchangeAck,
            >(raw_msg)?)),
            80 => Ok(MessageType::Piece(serde_bencode::from_bytes::<Piece>(
                raw_msg,
            )?)),
            90 => Ok(MessageType::PieceAck(
                serde_bencode::from_bytes::<PieceAck>(raw_msg)?,
            )),
            100 => Ok(MessageType::Done(serde_bencode::from_bytes::<Done>(
                raw_msg,
            )?)),
            _ => bail!("invalid message type {}", v),
        }
        // .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
    // pub fn oneshot<T: FromMessageType>(&self, mt: MessageType) -> Result<T, Error> {
    //     T::from_message_type(mt)
    // }
}

// macro_rules! impl_from_message_type {
//     ($($variant:ident),*) => {
//         $(
//             impl FromMessageType for $variant {
//                 fn from_message_type(message: MessageType) -> Result<Self, Error> {
//                     if let MessageType::$variant(data) = message {
//                         Ok(data)
//                     } else {
//                         Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid message type"))
//                     }
//                 }
//             }
//         )*
//     }
// }

// pub trait FromMessageType: Sized {
//     fn from_message_type(message: MessageType) -> Result<Self, Error>;
// }

// impl_from_message_type!(
//     Done,
//     File,
//     MyPkg,
//     MyPkgAck,
//     NegotiateMyPkg,
//     NegotiateMyPkgAck,
//     Piece,
//     PieceAck,
//     PieceExchange,
//     PieceExchangeAck
// );

pub trait ToMessageType {
    fn to_message_type(self) -> MessageType;
}

macro_rules! impl_to_message_type {
    ($($variant:ident),*) => {
        $(
            impl ToMessageType for $variant {
                fn to_message_type(self) -> MessageType {
                    MessageType::$variant(self)
                }
            }
        )*
    }
}

impl_to_message_type!(
    Done,
    File,
    MyPkg,
    MyPkgAck,
    NegotiateMyPkg,
    NegotiateMyPkgAck,
    Piece,
    PieceAck,
    PieceExchange,
    PieceExchangeAck
);
