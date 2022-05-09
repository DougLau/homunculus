pub mod cube;
pub mod geom;
pub mod gltf;
pub mod mesh;
pub mod solid;

use argh::FromArgs;
use std::fs::File;
use std::path::Path;

/// Command-line arguments
#[derive(FromArgs, PartialEq, Debug)]
struct Args {
    #[argh(positional)]
    file: String,
}

fn main() {
    let args: Args = argh::from_env();
    let path = Path::new(&args.file);
    let stem = path.file_stem().unwrap();
    let out = path.with_file_name(Path::new(stem).with_extension("glb"));
    let file = File::open(path).unwrap();
    let cfg: solid::Config = muon_rs::from_reader(file).unwrap();
    let mesh = cfg.build();
    gltf::export(&out, &mesh).unwrap();
}
