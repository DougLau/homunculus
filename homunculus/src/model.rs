// model.rs     Model module
//
// Copyright (c) 2022  Douglas Lau
//
use crate::gltf;
use crate::mesh::{Face, Mesh, MeshBuilder};
use glam::{Quat, Vec3};
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::io::Write;
use std::str::FromStr;

/// Point type
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum PtType {
    /// Vertex number
    Vertex(usize),

    /// Branch label
    Branch(String),
}

/// A point on model surface
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Point {
    /// Degrees around ring (must be first for `Ord`)
    order_deg: usize,

    /// Ring ID
    ring_id: usize,

    /// Point type
    pt_type: PtType,
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
    /// Ring ID
    id: usize,

    /// Center point
    center: Vec3,

    /// Axis vector
    axis: Option<Vec3>,

    /// Point definitions
    point_defs: Vec<PtDef>,

    /// Scale factor
    scale: f32,
}

/// Ring configuration
#[derive(Debug, Deserialize)]
pub struct RingCfg {
    /// Ring branch label
    branch: Option<String>,

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

    /// Branches (label to vertices mapping)
    branches: HashMap<String, Vec<usize>>,
}

impl Default for Ring {
    fn default() -> Self {
        Ring {
            id: 0,
            center: Vec3::new(0.0, 0.0, 0.0),
            axis: None,
            point_defs: vec![],
            scale: 1.0,
        }
    }
}

impl Ring {
    /// Get the ring axis
    fn axis(&self) -> Vec3 {
        self.axis.unwrap_or(Vec3::new(0.0, 1.0, 0.0))
    }

    /// Update ring from a configuration
    fn with_config(&mut self, cfg: &RingCfg) {
        if cfg.branch.is_some() {
            // clear previous axis on new branches
            self.axis = None;
        }
        if let Some(axis) = cfg.parse_axis() {
            self.axis = Some(axis);
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
        let branches = HashMap::new();
        ModelBuilder {
            builder,
            points,
            branches,
        }
    }

    /// Push one point
    fn push_point(&mut self, order_deg: usize, ring_id: usize, vtx: usize) {
        let pt_type = PtType::Vertex(vtx);
        self.points.push(Point {
            order_deg,
            ring_id,
            pt_type,
        });
    }

    /// Push one hole
    fn push_hole(&mut self, order_deg: usize, ring_id: usize, branch: String) {
        if !self.branches.contains_key(&branch) {
            self.branches.insert(branch.clone(), vec![]);
        }
        let pt_type = PtType::Branch(branch);
        self.points.push(Point {
            order_deg,
            ring_id,
            pt_type,
        });
    }

    /// Add a ring
    fn add_ring(&mut self, ring: Ring) {
        let axis = ring.axis().normalize();
        for (i, ptd) in ring.point_defs.iter().enumerate() {
            let order_deg = ring.order_deg(i);
            match ptd {
                PtDef::Limits(near, _far) => {
                    let vtx = self.builder.vertices();
                    self.push_point(order_deg, ring.id, vtx);
                    let angle = ring.angle(i);
                    let rot = Quat::from_axis_angle(axis, angle);
                    let dist = near * ring.scale; // FIXME: use far
                    let pt = ring.center + rot * Vec3::new(dist, 0.0, 0.0);
                    self.builder.push_vtx(pt);
                }
                PtDef::Branch(branch) => {
                    self.push_hole(order_deg, ring.id, branch.clone())
                }
            }
        }
    }

    /// Add a branch base ring
    fn add_branch(&mut self, branch: &str, ring: &Ring) -> Option<Ring> {
        let mut center = Vec3::new(0.0, 0.0, 0.0);
        let points = self.branch_points(branch);
        if points.is_empty() {
            return None;
        }
        for idx in &points {
            center += self.builder.vertex(*idx);
        }
        let len = points.len();
        center /= len as f32;
        dbg!(&center);
        let axis = ring.axis.unwrap_or_else(|| {
            self.branch_axis(branch, center)
        });
        dbg!(&axis);
        let mut pring = Ring::default();
        pring.id = ring.id;
        pring.center = center;
        pring.axis = Some(axis);
        pring.point_defs = vec![PtDef::Limits(1.0, 1.0); len];
        pring.scale = ring.scale;
        self.push_point(10, ring.id, 19); // 0.1745
        self.push_point(70, ring.id, 14); // 1.2217
        self.push_point(130, ring.id, 15); // 2.2689
        self.push_point(190, ring.id, 20); // 3.3161
        self.push_point(250, ring.id, 24); // 4.3633
        self.push_point(310, ring.id, 23); // 5.4105
        for pt in points {
            // FIXME: calculate `order_deg`
            let vtx = (self.builder.vertex(pt) - center).normalize();
            let rot = Quat::from_axis_angle(axis, 0.0);
            let zero = rot * Vec3::new(1.0, 0.0, 0.0);
            dbg!(pt);
            dbg!(vtx.angle_between(zero));
        }
        Some(pring)
    }

    /// Get all points on a branch base
    fn branch_points(&self, branch: &str) -> Vec<usize> {
        match self.branches.get(branch) {
            Some(vtx) => {
                let mut vtx = vtx.clone();
                vtx.sort();
                vtx.dedup();
                vtx
            }
            None => vec![],
        }
    }

    /// Calculate axis for a branch base
    fn branch_axis(&self, branch: &str, center: Vec3) -> Vec3 {
        match self.branches.get(branch) {
            Some(vtx) => {
                let mut norm = Vec3::default();
                // FIXME: this doesn't work with two Branch points
                for v in vtx.chunks_exact(2) {
                    let v0 = self.builder.vertex(v[0]);
                    let v1 = self.builder.vertex(v[1]);
                    let trin = (center - v0).cross(center - v1).normalize();
                    norm += trin;
                }
                norm
            }
            None => Vec3::new(0.0, 1.0, 0.0),
        }
    }

    /// Get the points for one ring
    fn ring_points(&self, ring: &Ring, hs_other: usize) -> Vec<Point> {
        let mut pts = vec![];
        for point in &self.points {
            if point.ring_id == ring.id {
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
        assert_ne!(ring0.id, ring1.id);
        // get points for each ring
        let mut pts0 = self.ring_points(ring0, ring1.half_step());
        let mut pts1 = self.ring_points(ring1, ring0.half_step());
        let first0 = pts0.pop().unwrap();
        let first1 = pts1.pop().unwrap();
        pts0.append(&mut pts1);
        pts0.sort();
        pts0.reverse();
        let mut band = pts0;
        let (mut vtx0, mut vtx1) = (first0.clone(), first1.clone());
        // create faces of band as a triangle strip
        while let Some(pt) = band.pop() {
            self.add_face([&vtx0, &vtx1, &pt]);
            if pt.ring_id == ring0.id {
                vtx0 = pt;
            } else {
                vtx1 = pt;
            }
        }
        // connect with first vertices on band
        self.add_face([&vtx0, &vtx1, &first1]);
        self.add_face([&first1, &first0, &vtx0]);
    }

    /// Add a triangle face
    fn add_face(&mut self, pts: [&Point; 3]) {
        match (&pts[0].pt_type, &pts[1].pt_type, &pts[2].pt_type) {
            (PtType::Vertex(v0), PtType::Vertex(v1), PtType::Vertex(v2)) => {
                self.builder.push_face(Face::new([*v0, *v1, *v2]));
            }
            (PtType::Branch(b), PtType::Vertex(v0), PtType::Vertex(v1))
            | (PtType::Vertex(v1), PtType::Branch(b), PtType::Vertex(v0))
            | (PtType::Vertex(v0), PtType::Vertex(v1), PtType::Branch(b)) => {
                if let Some(vtx) = self.branches.get_mut(b) {
                    vtx.push(*v0);
                    vtx.push(*v1);
                }
            }
            (PtType::Vertex(v), PtType::Branch(b0), PtType::Branch(b1))
            | (PtType::Branch(b0), PtType::Vertex(v), PtType::Branch(b1))
            | (PtType::Branch(b0), PtType::Branch(b1), PtType::Vertex(v)) => {
                assert_eq!(b0, b1);
                dbg!((b0, v));
                todo!();
            }
            (PtType::Branch(b0), PtType::Branch(b1), PtType::Branch(b2)) => {
                assert_eq!(b0, b1);
                assert_eq!(b1, b2);
                todo!();
            }
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
        let mut pring = None;
        for cfg in &self.ring {
            ring.with_config(&cfg);
            if let Some(branch) = &cfg.branch {
                pring = model.add_branch(branch, &ring);
                match (ring.axis, &pring) {
                    (None, Some(pring)) => ring.axis = Some(pring.axis()),
                    _ => (),
                }
                if let Some(pring) = &pring {
                    ring.center = pring.center + pring.axis().normalize() / 2.0;
                }
                ring.id += 1;
            }
            model.add_ring(ring.clone());
            if let Some(pring) = &pring {
                model.make_band(&ring, pring);
            }
            pring = Some(ring.clone());
            ring.id += 1;
            ring.center += ring.axis();
        }
        model.builder.build()
    }
}
