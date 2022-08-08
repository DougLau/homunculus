// model.rs     Model module
//
// Copyright (c) 2022  Douglas Lau
//
use crate::gltf;
use crate::mesh::{Face, Mesh, MeshBuilder, Smoothing};
use crate::plane::Plane;
use glam::{Quat, Vec3};
use serde_derive::{Deserialize, Serialize};
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
    /// Distance from axis
    Distance(f32),

    /// Branch label
    Branch(String),
}

/// Ring around surface of model
#[derive(Clone, Debug)]
pub struct Ring {
    /// Ring ID
    id: usize,

    /// Center point
    center: Vec3,

    /// Axis vector
    axis: Option<Vec3>,

    /// Point definitions
    point_defs: Vec<PtDef>,

    /// Scale factor
    scale: Option<f32>,

    /// Edge smoothing
    smoothing: Option<Smoothing>,
}

/// Ring configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct RingCfg {
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

/// Model configuration
#[derive(Deserialize, Serialize)]
pub struct ModelCfg {
    /// Vec of all rings
    ring: Vec<RingCfg>,
}

/// 3D Model
pub struct Model {
    /// Mesh builder
    builder: MeshBuilder,

    /// Current ring
    ring: Ring,

    /// Previous ring
    pring: Option<Ring>,

    /// All points on mesh
    points: Vec<Point>,

    /// Branches (label to edge vertices mapping)
    branches: HashMap<String, Vec<[usize; 2]>>,
}

impl Default for Ring {
    fn default() -> Self {
        Ring {
            id: 0,
            center: Vec3::new(0.0, 0.0, 0.0),
            axis: None,
            point_defs: vec![],
            scale: None,
            smoothing: None,
        }
    }
}

impl TryFrom<&RingCfg> for Ring {
    type Error = &'static str;

    fn try_from(cfg: &RingCfg) -> Result<Self, Self::Error> {
        let mut ring = Ring::new();
        ring.axis = cfg.axis();
        ring.point_defs = cfg.point_defs();
        ring.scale = cfg.scale;
        ring.smoothing = cfg.smoothing();
        Ok(ring)
    }
}

impl Ring {
    /// Create a new ring
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the ring axis
    fn axis(&self) -> Vec3 {
        self.axis.unwrap_or_else(|| Vec3::new(0.0, 1.0, 0.0))
    }

    /// Get the ring scale
    fn scale(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }

    /// Get the ring smoothing
    fn smoothing(&self) -> Smoothing {
        self.smoothing.unwrap_or(Smoothing::Smooth)
    }

    /// Update with another ring
    fn update_with(&mut self, ring: &Self) {
        if ring.axis.is_some() {
            self.axis = ring.axis;
        }
        if !ring.point_defs.is_empty() {
            self.point_defs = ring.point_defs.clone();
        }
        if ring.scale.is_some() {
            self.scale = ring.scale;
        }
        if ring.smoothing.is_some() {
            self.smoothing = ring.smoothing;
        }
    }

    /// Get half step in degrees
    fn half_step(&self) -> usize {
        180 / self.point_defs.len()
    }

    /// Calculate the angle of a point
    fn angle(&self, i: usize) -> f32 {
        2.0 * PI * i as f32 / self.point_defs.len() as f32
    }
}

/// Parse a point count
fn parse_count(code: &str) -> usize {
    code.parse().expect("Invalid count")
}

impl FromStr for PtDef {
    type Err = &'static str;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        match code.parse::<f32>() {
            Ok(pt) => Ok(PtDef::Distance(pt)),
            Err(_) => {
                if code == "." {
                    Ok(PtDef::Distance(1.0))
                } else {
                    Ok(PtDef::Branch(code.into()))
                }
            }
        }
    }
}

impl RingCfg {
    /// Parse axis vector
    fn axis(&self) -> Option<Vec3> {
        self.axis.as_ref().map(|axis| {
            let xyz: Vec<_> = axis.split(' ').collect();
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
            defs.push(code.parse().expect("Invalid point code"));
        }
        defs
    }

    /// Get edge smoothing
    fn smoothing(&self) -> Option<Smoothing> {
        match self.smoothing.as_deref() {
            Some("flat") => Some(Smoothing::Sharp),
            Some("smooth") => Some(Smoothing::Smooth),
            _ => None,
        }
    }
}

impl TryFrom<&ModelCfg> for Model {
    type Error = &'static str;

    fn try_from(cfg: &ModelCfg) -> Result<Self, Self::Error> {
        let mut model = Model::new();
        for ring in &cfg.ring {
            if let Some(branch) = &ring.branch {
                model.add_branch(branch, ring.axis());
            }
            model.add_ring(&ring.try_into().unwrap());
        }
        Ok(model)
    }
}

impl Model {
    /// Create a new 3D model
    pub fn new() -> Model {
        let builder = Mesh::builder();
        let ring = Ring::new();
        let points = vec![];
        let branches = HashMap::new();
        Model {
            builder,
            ring,
            pring: None,
            points,
            branches,
        }
    }

    /// Push one point
    fn push_pt(&mut self, order_deg: usize, ring_id: usize, pt_type: PtType) {
        self.points.push(Point {
            order_deg,
            ring_id,
            pt_type,
        });
    }

    /// Add points for a ring
    fn add_ring_points(&mut self, ring: &Ring) {
        let axis = ring.axis().normalize();
        for (i, ptd) in ring.point_defs.iter().enumerate() {
            let angle = ring.angle(i);
            let order_deg = angle.to_degrees() as usize;
            match ptd {
                PtDef::Distance(dist) => {
                    let vid = self.builder.vertices();
                    self.push_pt(order_deg, ring.id, PtType::Vertex(vid));
                    let rot = Quat::from_axis_angle(axis, angle)
                        * orthonormal_zero(axis);
                    let dist = dist * ring.scale();
                    let vtx = ring.center + rot * dist;
                    self.builder.push_vtx(vtx);
                }
                PtDef::Branch(branch) => self.push_pt(
                    order_deg,
                    ring.id,
                    PtType::Branch(branch.into()),
                ),
            }
        }
    }

    /// Add a ring
    pub fn add_ring(&mut self, aring: &Ring) {
        let mut ring = self.ring.clone();
        ring.update_with(aring);
        self.add_ring_points(&ring);
        if let Some(pring) = self.pring.take() {
            self.make_band(&ring, &pring);
        }
        self.pring = Some(ring.clone());
        ring.id += 1;
        ring.center += ring.axis();
        self.ring = ring;
    }

    /// Add a branch base ring
    pub fn add_branch(&mut self, branch: &str, axis: Option<Vec3>) {
        // FIXME: add cap to previous ring
        let vertices = self.branch_vertices(branch);
        if vertices.is_empty() {
            panic!("Unknown branch");
        }
        let id = self.ring.id;
        let len = vertices.len();
        let center = vertices
            .iter()
            .map(|i| self.builder.vertex(*i))
            .fold(Vec3::ZERO, |a, b| a + b)
            / len as f32;
        let axis = axis
            .unwrap_or_else(|| self.branch_axis(branch, center));
        let pring = Ring {
            id,
            center,
            axis: Some(axis),
            point_defs: vec![PtDef::Distance(1.0); len],
            ..Default::default()
        };
        for (order_deg, vid) in self.branch_angles(branch, axis, center) {
            self.push_pt(order_deg, id, PtType::Vertex(vid));
        }
        self.ring.axis = Some(axis);
        self.ring.center = center + axis.normalize() / 2.0;
        self.ring.id += 1;
        self.pring = Some(pring);
    }

    /// Get all vertices on a branch base
    fn branch_vertices(&self, branch: &str) -> Vec<usize> {
        match self.branches.get(branch) {
            Some(edges) => {
                let mut vertices = edges
                    .iter()
                    .flat_map(|e| [e[0], e[1]].into_iter())
                    .collect::<Vec<usize>>();
                vertices.sort();
                vertices.dedup();
                vertices
            }
            None => vec![],
        }
    }

    /// Calculate axis for a branch base
    fn branch_axis(&self, branch: &str, center: Vec3) -> Vec3 {
        match self.branches.get(branch) {
            Some(edges) => {
                let mut norm = Vec3::ZERO;
                for edge in edges {
                    let v0 = self.builder.vertex(edge[0]);
                    let v1 = self.builder.vertex(edge[1]);
                    norm += (center - v0).cross(center - v1);
                }
                norm.normalize()
            }
            None => Vec3::new(0.0, 1.0, 0.0),
        }
    }

    /// Calculate angles for a branch base
    fn branch_angles(
        &self,
        branch: &str,
        axis: Vec3,
        center: Vec3,
    ) -> Vec<(usize, usize)> {
        match self.branches.get(branch) {
            Some(edges) => {
                let plane = Plane::new(axis, center);
                let zero_deg = center + orthonormal_zero(axis);
                // Step 1: find "first" edge vertex (closest to 0 degrees)
                let mut edge = 0;
                let mut angle = f32::MAX;
                for (i, ed) in edges.iter().enumerate() {
                    let vid = ed[0];
                    let vtx = plane.project_point(self.builder.vertex(vid));
                    let ang = (zero_deg - center).angle_between(vtx - center);
                    if ang < angle {
                        angle = ang;
                        edge = i;
                    }
                }
                // Step 2: sort edge vertices by common end-points
                let vids = edge_vids(edges, edge);
                // Step 3: make vec of (order_deg, vid)
                let mut angle = 0.0;
                let mut pvtx = zero_deg;
                let mut angles = vec![];
                for vid in vids {
                    let vtx = plane.project_point(self.builder.vertex(vid));
                    let ang = (pvtx - center).angle_between(vtx - center);
                    angle += ang;
                    let order_deg = angle.to_degrees() as usize;
                    angles.push((order_deg, vid));
                    pvtx = vtx;
                }
                angles
            }
            None => vec![],
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
        let (mut pt0, mut pt1) = (first0.clone(), first1.clone());
        // create faces of band as a triangle strip
        while let Some(pt) = band.pop() {
            self.add_face([&pt0, &pt1, &pt], ring0.smoothing());
            if pt.ring_id == ring0.id {
                pt0 = pt;
            } else {
                pt1 = pt;
            }
        }
        // connect with first vertices on band
        self.add_face([&pt0, &pt1, &first1], ring0.smoothing());
        self.add_face([&first1, &first0, &pt0], ring0.smoothing());
    }

    /// Add a triangle face
    fn add_face(&mut self, pts: [&Point; 3], smoothing: Smoothing) {
        match (&pts[0].pt_type, &pts[1].pt_type, &pts[2].pt_type) {
            (PtType::Vertex(v0), PtType::Vertex(v1), PtType::Vertex(v2)) => {
                let face = Face::new([*v0, *v1, *v2], smoothing);
                self.builder.push_face(face);
            }
            (PtType::Branch(b), PtType::Vertex(v0), PtType::Vertex(v1))
            | (PtType::Vertex(v1), PtType::Branch(b), PtType::Vertex(v0))
            | (PtType::Vertex(v0), PtType::Vertex(v1), PtType::Branch(b)) => {
                if !self.branches.contains_key(b) {
                    self.branches.insert(b.clone(), vec![]);
                }
                let edges = self.branches.get_mut(b).unwrap();
                edges.push([*v0, *v1]);
            }
            (PtType::Vertex(_v), PtType::Branch(b0), PtType::Branch(b1))
            | (PtType::Branch(b0), PtType::Vertex(_v), PtType::Branch(b1))
            | (PtType::Branch(b0), PtType::Branch(b1), PtType::Vertex(_v)) => {
                // A single vertex and two branch points:
                // - both points must be for the same branch
                // - no edges need to be added
                assert_eq!(b0, b1);
            }
            (PtType::Branch(b0), PtType::Branch(b1), PtType::Branch(b2)) => {
                // Three adjacent branch points:
                // - all points must be for the same branch
                assert_eq!(b0, b1);
                assert_eq!(b1, b2);
            }
        }
    }

    /// Write model as glTF
    pub fn write_gltf<W: Write>(self, writer: W) -> std::io::Result<()> {
        // FIXME: add cap to last ring
        let mesh = self.builder.build();
        gltf::export(writer, &mesh)
    }
}

/// Get the orthonormal to a vector at zero degrees
///
/// We don't use `Vec3::any_orthonormal_vector` since the returned vector may
/// change in a future glam update.
fn orthonormal_zero(v: Vec3) -> Vec3 {
    if v.x.abs() > v.y.abs() {
        Vec3::new(-v.z, 0.0, v.x)
    } else {
        Vec3::new(0.0, v.z, -v.y)
    }
    .normalize()
}

/// Get edge vertices sorted by common end-points
fn edge_vids(edges: &[[usize; 2]], edge: usize) -> Vec<usize> {
    let mut edges = edges.to_vec();
    if edge > 0 {
        edges.swap(0, edge);
    }
    let mut vid = edges[0][1];
    for i in 1..edges.len() {
        for j in (i + 1)..edges.len() {
            if vid == edges[j][0] {
                edges.swap(i, j);
            }
        }
        vid = edges[i][1];
    }
    edges.into_iter().map(|e| e[0]).collect()
}
