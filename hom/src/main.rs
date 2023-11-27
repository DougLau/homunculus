// main.rs      hom program
//
// Copyright (c) 2022-2023  Douglas Lau
//
mod cube;
mod mesh;
mod model;
mod view;

use crate::model::ModelDef;
use anyhow::{Context, Result};
use argh::FromArgs;
use homunculus::Husk;
use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Crate name
const NAME: &str = std::env!("CARGO_PKG_NAME");

/// Crate version
const VERSION: &str = std::env!("CARGO_PKG_VERSION");

/// Command-line arguments
#[derive(FromArgs, PartialEq, Debug)]
struct Args {
    /// build model only
    #[argh(switch, short = 'b')]
    build: bool,

    /// show version
    #[argh(switch, short = 'V')]
    version: bool,

    /// model file name (.hom, .glb, .gltf)
    #[argh(positional)]
    file: Option<OsString>,
}

/// Main function
fn main() -> Result<()> {
    let args: Args = argh::from_env();
    if args.version {
        println!("{NAME} {VERSION}");
        return Ok(());
    }
    if let Some(file) = &args.file {
        let path = build_homunculus(Path::new(file))?;
        if !args.build {
            view(path)?;
        }
    }
    Ok(())
}

/// Build homunculus model
fn build_homunculus(path: &Path) -> Result<PathBuf> {
    let file = File::open(path)
        .with_context(|| format!("{} not found", path.display()))?;
    match path.extension() {
        Some(ext) if ext == "glb" || ext == "gltf" => {
            eprintln!("{path:?} already glTF model");
            return Ok(path.to_path_buf());
        }
        _ => {}
    }
    let stem = path.file_stem().context("Invalid file name")?;
    let def: ModelDef = muon_rs::from_reader(file).context("Invalid model")?;
    let husk = Husk::try_from(&def).context("Invalid model")?;
    let out = path.with_file_name(Path::new(stem).with_extension("glb"));
    let writer = File::create(&out)
        .with_context(|| format!("Cannot create {}", out.display()))?;
    husk.write_gltf(&writer).context("Writing glTF")?;
    Ok(out)
}

/// View glTF model
fn view(path: PathBuf) -> Result<()> {
    let folder = std::env::current_dir()?.display().to_string();
    view::view_gltf(folder, path);
    Ok(())
}
