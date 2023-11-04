// lib.rs      homunculus crate.
//
// Copyright (c) 2022-2023  Douglas Lau
//
mod error;
mod gltf;
mod mesh;
mod model;

pub use error::Error;
pub use mesh::Smoothing;
pub use model::{Model, Ring};
