// model.rs     Model definitions
//
// Copyright (c) 2022-2023  Douglas Lau
//
use anyhow::{anyhow, bail, Error};
use glam::Vec3;
use homunculus::{Husk, Ring};
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;

type Result<T> = std::result::Result<T, Error>;

/// Point definition
#[derive(Clone, Debug)]
enum PtDef {
    /// Distance from axis
    Distance(f32),

    /// Branch label (FIXME: add distance as well)
    Branch(String),
}

/// Ring definition
#[derive(Debug, Deserialize, Serialize)]
pub struct RingDef {
    /// Ring branch label
    branch: Option<String>,

    /// Axis vector
    axis: Option<String>,

    /// Point limits
    points: Vec<String>,

    /// Scale factor
    scale: Option<f32>,

    /// Smoothing setting
    smoothing: Option<String>,
}

/// Definition of a 3D model
#[derive(Debug, Deserialize, Serialize)]
pub struct ModelDef {
    /// Vec of all rings
    ring: Vec<RingDef>,
}

impl TryFrom<&RingDef> for Ring {
    type Error = Error;

    fn try_from(def: &RingDef) -> Result<Self> {
        let mut ring = Ring::default();
        if let Some(axis) = def.axis()? {
            ring = ring.axis(axis);
        }
        if let Some(scale) = def.scale {
            ring = ring.scale(scale);
        }
        match def.smooth()? {
            Some(true) => ring = ring.smooth(),
            Some(false) => ring = ring.flat(),
            None => (),
        }
        for pt in def.point_defs()? {
            ring = match pt {
                PtDef::Distance(d) => ring.spoke(d),
                PtDef::Branch(b) => ring.spoke(b.as_ref()),
            };
        }
        Ok(ring)
    }
}

impl FromStr for PtDef {
    type Err = Error;

    fn from_str(code: &str) -> Result<Self> {
        match code.parse::<f32>() {
            Ok(dist) => Ok(PtDef::Distance(dist)),
            Err(_) => {
                if code.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    Ok(PtDef::Branch(code.into()))
                } else {
                    bail!("Invalid branch label: {code}")
                }
            }
        }
    }
}

impl RingDef {
    /// Parse axis vector
    fn axis(&self) -> Result<Option<Vec3>> {
        match &self.axis {
            Some(axis) => {
                let mut xyz = axis.splitn(3, ' ');
                if let (Some(x), Some(y), Some(z)) =
                    (xyz.next(), xyz.next(), xyz.next())
                {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (x.parse::<f32>(), y.parse::<f32>(), z.parse::<f32>())
                    {
                        return Ok(Some(Vec3::new(x, y, z)));
                    }
                }
                bail!("Invalid axis: {axis}")
            }
            None => Ok(None),
        }
    }

    /// Get point definitions
    fn point_defs(&self) -> Result<Vec<PtDef>> {
        let mut defs = vec![];
        let mut repeat = false;
        for code in &self.points {
            if repeat {
                let count = code
                    .parse()
                    .map_err(|_| anyhow!("Invalid repeat count: {code}"))?;
                let ptd = defs.last().cloned().unwrap_or(PtDef::Distance(1.0));
                for _ in 1..count {
                    defs.push(ptd.clone());
                }
                repeat = false;
                continue;
            }
            if code == "*" {
                repeat = true;
                continue;
            }
            let def = code
                .parse()
                .map_err(|_| anyhow!("Invalid point def: {code}"))?;
            defs.push(def);
        }
        Ok(defs)
    }

    /// Get edge smoothing
    fn smooth(&self) -> Result<Option<bool>> {
        match self.smoothing.as_deref() {
            Some("Flat") => Ok(Some(false)),
            Some("Smooth") => Ok(Some(true)),
            Some(s) => bail!("Invalid smoothing: {s}"),
            None => Ok(None),
        }
    }
}

impl TryFrom<&ModelDef> for Husk {
    type Error = Error;

    fn try_from(def: &ModelDef) -> Result<Self> {
        let mut husk = Husk::new();
        for ring in &def.ring {
            if let Some(label) = &ring.branch {
                husk.branch(label, ring.axis()?)?;
            }
            husk.ring(ring.try_into()?)?;
        }
        Ok(husk)
    }
}
