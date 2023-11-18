// pyramid example
use anyhow::Result;
use homunculus::{Husk, Ring, Shading};
use std::fs::File;

fn main() -> Result<()> {
    let mut husk = Husk::new();
    let base = Ring::default()
        .shading(Shading::Flat)
        .spoke(1.0)
        .spoke(1.0)
        .spoke(1.0)
        .spoke(1.0);
    husk.ring(base)?;
    husk.ring(Ring::default().spoke(0.0))?;
    let file = File::create("pyramid.glb")?;
    husk.write_gltf(file)?;
    Ok(())
}
