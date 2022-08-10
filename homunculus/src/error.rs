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

    /// Invalid Axis
    #[error("Invalid axis: {0}")]
    InvalidAxis(String),

    /// Invalid Branches
    #[error("Invalid branches: {0} {1}")]
    InvalidBranches(String, String),

    /// Invalid Branch Label
    #[error("Invalid branch label: {0}")]
    InvalidBranchLabel(String),

    /// Invalid Point Definition
    #[error("Invalid point definition: {0}")]
    InvalidPointDef(String),

    /// Invalid Repeat Count
    #[error("Invalid repeat count: {0}")]
    InvalidRepeatCount(String),

    /// Invalid Ring
    #[error("Invalid ring: {0}")]
    InvalidRing(usize),

    /// Invalid Smoothing
    #[error("Invalid smoothing: {0}")]
    InvalidSmoothing(String),

    /// Unknown Branch Label
    #[error("Unknown branch label: {0}")]
    UnknownBranchLabel(String),
}

pub type Result<T> = std::result::Result<T, Error>;
