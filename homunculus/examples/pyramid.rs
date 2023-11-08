// pyramid example
use homunculus::{Model, Ring};
use std::fs::File;

fn main() {
    let mut model = Model::new();
    let base = Ring::default().point(1.0).point(1.0).point(1.0).point(1.0);
    model.ring(base).unwrap();
    model.ring(Ring::default().point(0.0)).unwrap();
    let file = File::create("pyramid.glb").unwrap();
    model.write_gltf(file).unwrap();
}
