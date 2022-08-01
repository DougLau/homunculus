// model.rs     Model module
//
// Copyright (c) 2022  Douglas Lau
//
use crate::gltf;
use crate::mesh::{Face, Mesh, MeshBuilder};
use glam::{Quat, Vec3};
use serde_derive::Deserialize;
use std::collections::VecDeque;
use std::f32::consts::PI;
use std::io::Write;
use std::str::FromStr;

/// A point on a model surface
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Point {
    /// Ring angle (must be first for PartialOrd)
    angle: f32,

    /// Ring number
    ring_number: usize,

    /// Vertex number
    vertex: Option<usize>,
}

/// Point definition
#[derive(Clone)]
enum PtDef {
    /// Point limits around axis
    Limits(f32, f32),

    /// Branch label
    Branch(String),
}

/// Ring on surface of model
#[derive(Clone, Default)]
struct Ring {
    /// Ring number
    number: usize,

    /// Center point
    center: Vec3,

    /// Scale factor
    scale: f32,

    /// Point definitions
    point_defs: Vec<PtDef>,

    /// Bone vector
    bone: Vec3,
}

/// Ring configuration
#[derive(Deserialize)]
pub struct RingCfg {
    /// Ring label
    label: Option<String>,

    /// Scale factor
    scale: Option<f32>,

    /// Point limits
    points: Vec<String>,

    /// Bone vector
    bone: Option<String>,
}

/// Model configuration
#[derive(Deserialize)]
pub struct Model {
    /// Vec of all rings
    ring: Vec<RingCfg>,
}

/// Model mesh builder
struct ModelBuilder {
    /// Mesh builder
    builder: MeshBuilder,

    /// All points on mesh
    points: Vec<Point>,
}

impl Ring {
    fn new() -> Self {
        Ring {
            number: 0,
            center: Vec3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            point_defs: vec![],
            bone: Vec3::new(0.0, 1.0, 0.0),
        }
    }

    /// Update ring from a configuration
    fn with_config(&mut self, cfg: &RingCfg) {
        if let Some(scale) = cfg.scale {
            self.scale = scale;
        }
        if !cfg.points.is_empty() {
            self.point_defs = cfg.point_defs();
        }
        if let Some(bone) = cfg.parse_bone() {
            self.bone = bone;
        }
    }

    /// Calculate the angle of a point
    fn angle(&self, i: usize) -> f32 {
        let count = self.point_defs.len() as f32;
        i as f32 / count * PI * 2.0
    }
}

/// Parse a point count
fn parse_count(code: &str) -> usize {
    code.parse().expect("Invalid count")
}

impl FromStr for PtDef {
    type Err = &'static str;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        let codes: Vec<&str> = code.split("..").collect();
        let len = codes.len();
        match len {
            1 => match code.parse::<f32>() {
                Ok(pt) => Ok(PtDef::Limits(pt, pt)),
                Err(_) => Ok(PtDef::Branch(code.into())),
            },
            2 => match (codes[0].parse::<f32>(), codes[1].parse::<f32>()) {
                (Ok(near), Ok(far)) => {
                    if near > far {
                        Err("Near > far: {code}")
                    } else {
                        Ok(PtDef::Limits(near, far))
                    }
                }
                _ => Err("Invalid points: {code}"),
            },
            _ => Err("Invalid points: {code}"),
        }
    }
}

impl RingCfg {
    /// Get point definitions
    fn point_defs(&self) -> Vec<PtDef> {
        let mut defs = vec![];
        let mut repeat = false;
        for code in &self.points {
            if repeat {
                let count = parse_count(code);
                let ptd =
                    defs.last().cloned().unwrap_or(PtDef::Limits(1.0, 1.0));
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
            defs.push(code.parse().unwrap());
        }
        defs
    }

    /// Parse a bone vector
    fn parse_bone(&self) -> Option<Vec3> {
        self.bone.as_ref().map(|bone| {
            let xyz: Vec<_> = bone.split(" ").collect();
            if xyz.len() == 3 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    xyz[0].parse::<f32>(),
                    xyz[1].parse::<f32>(),
                    xyz[2].parse::<f32>(),
                ) {
                    return Vec3::new(x, y, z);
                }
            }
            panic!("Invalid bone: {bone}");
        })
    }
}

impl ModelBuilder {
    /// Create a new model mesh builder
    fn new() -> ModelBuilder {
        let builder = Mesh::builder();
        let points = vec![];
        ModelBuilder { builder, points }
    }

    /// Push one point
    fn push_point(&mut self, angle: f32, ring_number: usize) {
        let vertex = Some(self.builder.vertices());
        self.points.push(Point {
            angle,
            ring_number,
            vertex,
        });
    }

    /// Push one hole
    fn push_hole(&mut self, angle: f32, ring_number: usize) {
        self.points.push(Point {
            angle,
            ring_number,
            vertex: None,
        });
    }

    /// Add a ring
    fn add_ring(&mut self, ring: Ring) {
        let bone = ring.bone.normalize();
        for (i, ptd) in ring.point_defs.iter().enumerate() {
            let angle = ring.angle(i);
            match ptd {
                PtDef::Limits(near, _far) => {
                    self.push_point(angle, ring.number);
                    let rot = Quat::from_axis_angle(bone, angle);
                    let dist = near * ring.scale; // FIXME: use far
                    let pt = ring.center + rot * Vec3::new(dist, 0.0, 0.0);
                    self.builder.push_vtx(pt);
                }
                PtDef::Branch(_label) => self.push_hole(angle, ring.number),
            }
        }
    }

    /// Make a band around the model
    fn make_band(&mut self, ring0: usize, ring1: usize) {
        let mut band = VecDeque::new();
        for point in &self.points {
            if point.ring_number == ring0 || point.ring_number == ring1 {
                band.push_back(point);
            }
        }
        band.make_contiguous()
            .sort_by(|a, b| a.partial_cmp(b).unwrap());
        let ipt = band.pop_front().unwrap();
        let jpt = band.pop_front().unwrap();
        let mut ivtx = ipt.vertex;
        let mut jvtx = jpt.vertex;
        assert!(ivtx != jvtx);
        if jpt.ring_number > ipt.ring_number {
            (ivtx, jvtx) = (jvtx, ivtx);
        }
        let (avtx, bvtx) = (ivtx, jvtx);
        while let Some(pt) = band.pop_front() {
            if let (Some(i), Some(j), Some(p)) = (ivtx, jvtx, pt.vertex) {
                self.builder.push_face(Face::new([i, j, p]));
            }
            if pt.ring_number == ring1 {
                ivtx = pt.vertex;
            } else {
                jvtx = pt.vertex;
            }
        }
        // Connect with first vertices
        if let (Some(i), Some(j), Some(b)) = (ivtx, jvtx, bvtx) {
            self.builder.push_face(Face::new([i, j, b]));
        }
        if let (Some(b), Some(a), Some(i)) = (bvtx, avtx, ivtx) {
            self.builder.push_face(Face::new([b, a, i]));
        }
    }
}

impl Model {
    /// Write model as glTF
    pub fn write_gltf<W: Write>(&self, writer: W) -> std::io::Result<()> {
        let mesh = self.build();
        Ok(gltf::export(writer, &mesh)?)
    }

    /// Build a mesh from the configuration
    fn build(&self) -> Mesh {
        let mut model = ModelBuilder::new();
        let mut ring = Ring::new();
        for cfg in &self.ring {
            ring.with_config(&cfg);
            model.add_ring(ring.clone());
            if ring.number > 0 {
                model.make_band(ring.number - 1, ring.number);
            }
            ring.number += 1;
            ring.center += ring.bone;
        }
        model.builder.build()
    }
}
