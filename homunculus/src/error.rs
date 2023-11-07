// error.rs     Error definitions
//
// Copyright (c) 2022  Douglas Lau
//

/// Homunculus errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error
    #[error("I/O {0}")]
    Io(#[from] std::io::Error),

    /// Invalid Ring
    #[error("Invalid ring: {0}")]
    InvalidRing(usize),

    /// Invalid Branches
    #[error("Invalid branches: {0}")]
    InvalidBranches(String),

    /// Unknown Branch Label
    #[error("Unknown branch label: {0}")]
    UnknownBranchLabel(String),
}

pub type Result<T> = std::result::Result<T, Error>;
