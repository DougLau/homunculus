// lib.rs      homunculus crate.
//
// Copyright (c) 2022  Douglas Lau
//
mod error;
mod gltf;
mod mesh;
mod model;
mod plane;

pub use error::Error;
pub use mesh::Smoothing;
pub use model::{Model, ModelDef, Ring};
