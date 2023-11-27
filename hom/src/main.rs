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
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::path::{Path, PathBuf};

/// Crate name
const NAME: &str = std::env!("CARGO_PKG_NAME");

/// Crate version
const VERSION: &str = std::env!("CARGO_PKG_VERSION");

/// Command-line arguments
#[derive(FromArgs, PartialEq, Debug)]
struct Args {
    #[argh[subcommand]]
    cmd: Option<Command>,

    /// show version
    #[argh(switch, short = 'V')]
    version: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Command {
    Build(BuildCommand),
    View(ViewCommand),
}

/// build only (.hom -> .glb)
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "build")]
struct BuildCommand {
    /// model file name (.hom)
    #[argh(positional)]
    file: OsString,
}

/// view model
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "view")]
struct ViewCommand {
    /// model file name (.hom, .glb, .gltf)
    #[argh(positional)]
    file: OsString,

    /// spawn stage
    #[argh(switch, short = 's')]
    stage: bool,
}

/// Main function
fn main() -> Result<()> {
    let args: Args = argh::from_env();
    if args.version {
        println!("{NAME} {VERSION}");
        return Ok(());
    }
    match &args.cmd {
        Some(Command::Build(build_cmd)) => build_cmd.build()?,
        Some(Command::View(view_cmd)) => view_cmd.view()?,
        None => todo!(),
    }
    Ok(())
}

impl BuildCommand {
    /// Build glTF model
    fn build(&self) -> Result<()> {
        let path = Path::new(&self.file);
        let stem = path.file_stem().context("Invalid file name")?;
        match path.extension() {
            Some(ext) if ext == "glb" || ext == "gltf" => {
                anyhow::bail!("{path:?} already glTF model");
            }
            _ => build_homunculus(path, stem),
        }
    }
}

/// Build homunculus model
fn build_homunculus(path: &Path, stem: &OsStr) -> Result<()> {
    let file = File::open(path)
        .with_context(|| format!("{} not found", path.display()))?;
    let def: ModelDef = muon_rs::from_reader(file).context("Invalid model")?;
    let husk = Husk::try_from(&def).context("Invalid model")?;
    let out = path.with_file_name(Path::new(stem).with_extension("glb"));
    let writer = File::create(&out)
        .with_context(|| format!("Cannot create {}", out.display()))?;
    husk.write_gltf(&writer).context("Writing glTF")?;
    Ok(())
}

impl ViewCommand {
    fn view(&self) -> Result<()> {
        let path = self.model_path()?;
        let folder = std::env::current_dir()?.display().to_string();
        view::view_gltf(folder, path, self.stage);
        Ok(())
    }

    /// Get path to glTF model
    fn model_path(&self) -> Result<PathBuf> {
        let path = Path::new(&self.file);
        let stem = path.file_stem().context("Invalid file name")?;
        match path.extension() {
            Some(ext) if ext == "glb" || ext == "gltf" => Ok(path.into()),
            _ => Ok(Path::new(stem).with_extension("glb")),
        }
    }
}
