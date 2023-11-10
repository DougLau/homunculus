// pyramid example
use homunculus::{Husk, Ring};
use std::fs::File;

fn main() {
    let mut husk = Husk::new();
    let base = Ring::default().spoke(1.0).spoke(1.0).spoke(1.0).spoke(1.0);
    husk.ring(base).unwrap();
    husk.ring(Ring::default().spoke(0.0)).unwrap();
    let file = File::create("pyramid.glb").unwrap();
    husk.write_gltf(file).unwrap();
}
