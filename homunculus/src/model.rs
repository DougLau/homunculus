// model.rs     Model module
//
// Copyright (c) 2022  Douglas Lau
//
use crate::gltf;
use crate::mesh::{Face, Mesh, MeshBuilder};
use glam::{Quat, Vec3};
use serde_derive::Deserialize;
use std::f32::consts::PI;
use std::io::Write;
use std::str::FromStr;

/// A point on a model surface
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Point {
    /// Degrees around ring (must be first for `Ord`)
    order_deg: usize,

    /// Ring number
    ring_number: usize,

    /// Vertex number
    vertex: Option<usize>,
}

/// Point definition
#[derive(Clone, Debug)]
enum PtDef {
    /// Point limits around axis
    Limits(f32, f32),

    /// Branch label
    Branch(String),
}

/// Ring around surface of model
#[derive(Clone, Debug)]
struct Ring {
    /// Ring number
    number: usize,

    /// Center point
    center: Vec3,

    /// Axis vector
    axis: Vec3,

    /// Point definitions
    point_defs: Vec<PtDef>,

    /// Scale factor
    scale: f32,
}

/// Ring configuration
#[derive(Debug, Deserialize)]
pub struct RingCfg {
    /// Ring label
    label: Option<String>,

    /// Axis vector
    axis: Option<String>,

    /// Point limits
    points: Vec<String>,

    /// Scale factor
    scale: Option<f32>,
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

impl Default for Ring {
    fn default() -> Self {
        Ring {
            number: 0,
            center: Vec3::new(0.0, 0.0, 0.0),
            axis: Vec3::new(0.0, 1.0, 0.0),
            point_defs: vec![],
            scale: 1.0,
        }
    }
}

impl Ring {
    /// Update ring from a configuration
    fn with_config(&mut self, cfg: &RingCfg) {
        if let Some(axis) = cfg.parse_axis() {
            self.axis = axis;
        }
        if !cfg.points.is_empty() {
            self.point_defs = cfg.point_defs();
        }
        if let Some(scale) = cfg.scale {
            self.scale = scale;
        }
    }

    /// Get half step in degrees
    fn half_step(&self) -> usize {
        180 / self.point_defs.len()
    }

    /// Calculate the degrees around ring
    fn order_deg(&self, i: usize) -> usize {
        360 * i / self.point_defs.len()
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
                Err(_) => {
                    if code == "." {
                        Ok(PtDef::Limits(1.0, 1.0))
                    } else {
                        Ok(PtDef::Branch(code.into()))
                    }
                }
            },
            2 => match (codes[0].parse::<f32>(), codes[1].parse::<f32>()) {
                (Ok(near), Ok(far)) => {
                    if near > far {
                        Err("Near > far")
                    } else {
                        Ok(PtDef::Limits(near, far))
                    }
                }
                _ => Err("Invalid points"),
            },
            _ => Err("Invalid points"),
        }
    }
}

impl RingCfg {
    /// Parse an axis vector
    fn parse_axis(&self) -> Option<Vec3> {
        self.axis.as_ref().map(|axis| {
            let xyz: Vec<_> = axis.split(" ").collect();
            if xyz.len() == 3 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    xyz[0].parse::<f32>(),
                    xyz[1].parse::<f32>(),
                    xyz[2].parse::<f32>(),
                ) {
                    return Vec3::new(x, y, z);
                }
            }
            panic!("Invalid axis: {axis}");
        })
    }

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
            defs.push(code.parse().expect("Invalid point code"));
        }
        defs
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
    fn push_point(&mut self, order_deg: usize, ring_number: usize) {
        let vertex = Some(self.builder.vertices());
        self.points.push(Point {
            order_deg,
            ring_number,
            vertex,
        });
    }

    /// Push one hole
    fn push_hole(&mut self, order_deg: usize, ring_number: usize) {
        self.points.push(Point {
            order_deg,
            ring_number,
            vertex: None,
        });
    }

    /// Add a ring
    fn add_ring(&mut self, ring: Ring) {
        let axis = ring.axis.normalize();
        for (i, ptd) in ring.point_defs.iter().enumerate() {
            let order_deg = ring.order_deg(i);
            match ptd {
                PtDef::Limits(near, _far) => {
                    self.push_point(order_deg, ring.number);
                    let angle = ring.angle(i);
                    let rot = Quat::from_axis_angle(axis, angle);
                    let dist = near * ring.scale; // FIXME: use far
                    let pt = ring.center + rot * Vec3::new(dist, 0.0, 0.0);
                    self.builder.push_vtx(pt);
                }
                PtDef::Branch(_label) => self.push_hole(order_deg, ring.number),
            }
        }
    }

    /// Get the points for one ring
    fn ring_points(&self, ring: &Ring, hs_other: usize) -> Vec<Point> {
        let mut pts = vec![];
        for point in &self.points {
            if point.ring_number == ring.number {
                let mut pt = point.clone();
                // adjust degrees by half step of other ring
                pt.order_deg = (pt.order_deg + hs_other) % 360;
                pts.push(pt);
            }
        }
        pts.sort();
        pts.reverse();
        pts
    }

    /// Make a band of faces between two rings
    fn make_band(&mut self, ring0: &Ring, ring1: &Ring) {
        assert_ne!(ring0.number, ring1.number);
        // get points for each ring
        let mut pts0 = self.ring_points(ring0, ring1.half_step());
        let mut pts1 = self.ring_points(ring1, ring0.half_step());
        let first0 = pts0.pop().unwrap().vertex;
        let first1 = pts1.pop().unwrap().vertex;
        pts0.append(&mut pts1);
        pts0.sort();
        pts0.reverse();
        let mut band = pts0;
        let (mut vtx0, mut vtx1) = (first0, first1);
        // create faces of band as a triangle strip
        while let Some(pt) = band.pop() {
            self.add_face([vtx0, vtx1, pt.vertex]);
            if pt.ring_number == ring0.number {
                vtx0 = pt.vertex;
            } else {
                vtx1 = pt.vertex;
            }
        }
        // connect with first vertices on band
        self.add_face([vtx0, vtx1, first1]);
        self.add_face([first1, first0, vtx0]);
    }

    /// Add a triangle face
    fn add_face(&mut self, vtx: [Option<usize>; 3]) {
        // if any vertices are None, there is a hole
        if let (Some(v0), Some(v1), Some(v2)) = (vtx[0], vtx[1], vtx[2]) {
            self.builder.push_face(Face::new([v0, v1, v2]));
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
        let mut ring = Ring::default();
        let mut pring = ring.clone();
        for cfg in &self.ring {
            ring.with_config(&cfg);
            model.add_ring(ring.clone());
            // FIXME: link rings
            if ring.number > 0 {
                model.make_band(&ring, &pring);
            }
            pring = ring.clone();
            ring.number += 1;
            ring.center += ring.axis;
        }
        model.builder.build()
    }
}
