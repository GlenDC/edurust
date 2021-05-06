use std::io;
// use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    MissingArg(&'static str),
    IO(String),
    Runtime(String),
    NoResults,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(match err.kind() {
            io::ErrorKind::NotFound => format!("file not found: {}", err.to_string()),
            _ => format!("unexpected IO Error: {}", err.to_string()),
        })
    }
}

// impl fmt::Display for Error {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", match self {
//             Error::MissingArg(ref s) => s,
//             Error::IOError(s) => s.as_str(),
//         })
//     }
// }
