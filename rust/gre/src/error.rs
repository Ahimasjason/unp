use core::fmt;



#[derive(Debug,)]
pub enum Error {

    IncorrectIPV4Packet,
    IoError(std::io::Error),
    IncorrectGREPacket,
    IncorrectInnerIpv4Packet,
    NotAGrePacket,
    NotAIpv4Packet,

}



impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}



pub type Result<T> = ::std::result::Result<T, Error>;
