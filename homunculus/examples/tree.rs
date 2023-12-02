// tree example
use anyhow::Result;
use argh::FromArgs;
use glam::Vec3;
use homunculus::{Husk, Ring};
use std::fs::File;

/// Command-line arguments
#[derive(FromArgs, PartialEq, Debug)]
struct Args {
    /// random seed
    #[argh(positional)]
    seed: Option<u64>,
}

#[derive(Debug)]
struct Branch {
    label: String,
    scale: f32,
}

fn make_ring(label: Option<&str>) -> Ring {
    let mut ring = Ring::default().axis(Vec3::new(0.0, 1.0, 0.0));
    let b = fastrand::usize(..6);
    for i in 0..6 {
        if let Some(label) = label {
            if i == b {
                ring = ring.spoke(label);
            } else {
                ring = ring.spoke(1.0);
            }
        } else {
            ring = ring.spoke(1.0);
        }
    }
    ring
}

fn make_branch(husk: &mut Husk, mut scale: f32) -> Result<Vec<Branch>> {
    let mut branches = Vec::new();
    let mut i = 0;
    while scale > 0.05 {
        let ring;
        let sc = scale * 0.5;
        if i % 3 == 1 && fastrand::f32() > scale && sc > 0.05 {
            let label = format!("B{}", fastrand::u16(..10000));
            ring = make_ring(Some(&label));
            branches.push(Branch { label, scale: sc });
        } else {
            ring = make_ring(None);
        }
        let x = fastrand::f32() * 0.01 - (0.01 * 0.5);
        let z = fastrand::f32() * 0.04 - (0.04 * 0.5);
        let axis = Vec3::new(x, scale, z);
        husk.ring(ring.axis(axis).scale(scale))?;
        scale *= 0.96;
        i += 1;
    }
    Ok(branches)
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();
    if let Some(seed) = args.seed {
        fastrand::seed(seed);
    }
    let mut husk = Husk::new();
    let mut branches = make_branch(&mut husk, 1.0)?;
    while !branches.is_empty() {
        let branch = branches.pop().unwrap();
        let r = husk.branch(branch.label)?;
        husk.ring(r)?;
        branches.extend(make_branch(&mut husk, branch.scale)?);
    }
    let file = File::create("tree.glb")?;
    husk.write_gltf(file)?;
    Ok(())
}
