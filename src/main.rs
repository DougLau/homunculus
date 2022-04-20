pub mod cube;
pub mod gltf;
pub mod mesh;
pub mod solid;

use argh::FromArgs;
use std::fs::File;

/// Command-line arguments
#[derive(FromArgs, PartialEq, Debug)]
struct Args {
    #[argh(positional)]
    file: String,
}

fn main() {
    let args: Args = argh::from_env();
    let file = File::open(&args.file).unwrap();
    let cfg: solid::Config = muon_rs::from_reader(file).unwrap();
    let mesh = cfg.build();
    gltf::export("test.glb", &mesh).unwrap();
}
