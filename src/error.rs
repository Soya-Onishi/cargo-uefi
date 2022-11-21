use std::fmt::Display;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    kind: ErrorKind,
    msg: String,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ErrorKind {
    NotAbleDetermineBinary,
    BinaryNotFound,
}

impl Error {
    pub fn new(kind: ErrorKind, msg: String) -> Error {
        Error {
            kind, 
            msg,
        }
    }

    #[allow(dead_code)]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(self)
    }

    fn description(&self) -> &str {
        self.msg.as_str()
    }
}