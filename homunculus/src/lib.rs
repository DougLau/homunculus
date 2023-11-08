// lib.rs      homunculus crate.
//
// Copyright (c) 2022-2023  Douglas Lau
//
#![doc = include_str!("../README.md")]

mod error;
mod gltf;
mod husk;
mod mesh;
mod ring;

pub use error::Error;
pub use husk::Husk;
pub use ring::{Ring, RingPoint};
