// main.rs      hom program
//
// Copyright (c) 2022  Douglas Lau
//
use anyhow::{Context, Result};
use argh::FromArgs;
use homunculus::Model;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Command-line arguments
#[derive(FromArgs, PartialEq, Debug)]
struct Args {
    /// view model
    #[argh(switch, short = 'v')]
    view: bool,

    /// model file name (.hom, .glb, .gltf)
    #[argh(positional)]
    model_file: String,
}

/// Main function
fn main() -> Result<()> {
    let args: Args = argh::from_env();
    let _path = args.build_model()?;
    if args.view {
        // TODO: view model
    }
    Ok(())
}

impl Args {
    /// Build glTF model
    fn build_model(&self) -> Result<PathBuf> {
        let path = Path::new(&self.model_file);
        let stem = path.file_stem().context("Invalid model_file name")?;
        match path.extension() {
            Some(ext) if ext == "glb" || ext == "gltf" => {
                if !self.view {
                    anyhow::bail!("{} already glTF model", path.display());
                }
                Ok(path.into())
            }
            _ => build_homunculus(&path, &stem),
        }
    }
}

/// Build homunculus model
fn build_homunculus(path: &Path, stem: &OsStr) -> Result<PathBuf> {
    let file = File::open(path)
        .with_context(|| format!("{} not found", path.display()))?;
    let model: Model =
        muon_rs::from_reader(file).context("Invalid homunculus model")?;
    let out = path.with_file_name(Path::new(stem).with_extension("glb"));
    let writer = File::create(&out)
        .with_context(|| format!("Cannot create {}", out.display()))?;
    model.write_gltf(&writer).context("Writing glTF")?;
    Ok(out)
}
