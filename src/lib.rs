use thiserror::{self, Error};

#[derive(Debug, Error)]
pub enum LoopError {
    /// Derived IO error
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error),

    /// Got less bytes than required to continue parsing the request
    #[error("Packet sent was smaller than expected, expected: {expected} got: {got}")]
    IncompleteRequest {
        expected: u32,
        got: u32,
    },

    /// Something fatal happend to the connection, like reading 0/EOF
    /// indicating the client dropped the connection
    #[error("Closing connection due to {0}")]
    CloseConnection(String),

    /// The request was malformed in some way, like the strings werent UTF-8
    #[error("Invalid request: {0}")]
    InvalidRequest(String)
}
