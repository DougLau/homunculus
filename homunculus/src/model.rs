// model.rs     Model module
//
// Copyright (c) 2022  Douglas Lau
//
use crate::error::{Error, Result};
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

/// Ring around surface of a model
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
///
/// It can be serialized or deserialized using any [serde] compatible data
/// format.
///
/// After deserializing, a [Model] can be created using `TryFrom`:
///
/// ```rust,no_run
/// # use std::fs::File;
/// # use homunculus::{Model, ModelDef};
/// let file = File::open("model.hom").unwrap();
/// let def: ModelDef = muon_rs::from_reader(file).unwrap();
/// let model = Model::try_from(&def).unwrap();
/// ```
///
/// [model]: struct.Model.html
/// [serde]: https://serde.rs/
#[derive(Debug, Deserialize, Serialize)]
pub struct ModelDef {
    /// Vec of all rings
    ring: Vec<RingDef>,
}

/// A 3D model
pub struct Model {
    /// Mesh builder
    builder: MeshBuilder,

    /// Current ring ID
    ring_id: usize,

    /// Current ring
    ring: Option<Ring>,

    /// All points on mesh
    points: Vec<Point>,

    /// Branches (label to edge vertices mapping)
    branches: HashMap<String, Vec<[usize; 2]>>,
}

impl TryFrom<&RingDef> for Ring {
    type Error = Error;

    fn try_from(def: &RingDef) -> Result<Self> {
        let mut ring = Ring::new();
        *ring.axis_mut() = def.axis()?;
        *ring.scale_mut() = def.scale;
        *ring.smoothing_mut() = def.smoothing()?;
        ring.point_defs = def.point_defs()?;
        Ok(ring)
    }
}

impl Default for Ring {
    fn default() -> Self {
        Self::new()
    }
}

impl Ring {
    /// Create a new ring
    pub fn new() -> Self {
        Ring {
            id: 0,
            center: Vec3::new(0.0, 0.0, 0.0),
            axis: None,
            point_defs: vec![],
            scale: None,
            smoothing: None,
        }
    }

    /// Get the ring axis (or default value)
    pub fn axis(&self) -> Vec3 {
        self.axis.unwrap_or_else(|| Vec3::new(0.0, 1.0, 0.0))
    }

    /// Get mutable ring axis
    pub fn axis_mut(&mut self) -> &mut Option<Vec3> {
        &mut self.axis
    }

    /// Get the ring scale (or default value)
    pub fn scale(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }

    /// Set mutable scale
    pub fn scale_mut(&mut self) -> &mut Option<f32> {
        &mut self.scale
    }

    /// Get the edge smoothing (or default value)
    pub fn smoothing(&self) -> Smoothing {
        self.smoothing.unwrap_or(Smoothing::Smooth)
    }

    /// Set mutable edge smoothing
    pub fn smoothing_mut(&mut self) -> &mut Option<Smoothing> {
        &mut self.smoothing
    }

    /// Update with another ring
    fn update_with(mut self, ring: &Self) -> Self {
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
        self.center += self.axis();
        self
    }

    /// Add a point
    pub fn add_point(&mut self, point: f32) {
        self.point_defs.push(PtDef::Distance(point));
    }

    /// Add a branch point
    pub fn add_branch_point(&mut self, branch: &str) {
        self.point_defs.push(PtDef::Branch(branch.into()));
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

impl FromStr for PtDef {
    type Err = Error;

    fn from_str(code: &str) -> Result<Self> {
        match code.parse::<f32>() {
            Ok(pt) => Ok(PtDef::Distance(pt)),
            Err(_) => {
                if code.chars().all(char::is_alphanumeric) {
                    Ok(PtDef::Branch(code.into()))
                } else {
                    Err(Error::InvalidBranchLabel(code.into()))
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
                let xyz: Vec<_> = axis.split(' ').collect();
                if xyz.len() == 3 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        xyz[0].parse::<f32>(),
                        xyz[1].parse::<f32>(),
                        xyz[2].parse::<f32>(),
                    ) {
                        return Ok(Some(Vec3::new(x, y, z)));
                    }
                }
                Err(Error::InvalidAxis(axis.into()))
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
                    .map_err(|_| Error::InvalidRepeatCount(code.into()))?;
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
                .map_err(|_| Error::InvalidPointDef(code.into()))?;
            defs.push(def);
        }
        Ok(defs)
    }

    /// Get edge smoothing
    fn smoothing(&self) -> Result<Option<Smoothing>> {
        match self.smoothing.as_deref() {
            Some("Sharp") => Ok(Some(Smoothing::Sharp)),
            Some("Smooth") => Ok(Some(Smoothing::Smooth)),
            Some(s) => Err(Error::InvalidSmoothing(s.into())),
            None => Ok(None),
        }
    }
}

impl TryFrom<&ModelDef> for Model {
    type Error = Error;

    fn try_from(def: &ModelDef) -> Result<Self> {
        let mut model = Model::new();
        for ring in &def.ring {
            if let Some(branch) = &ring.branch {
                model.add_branch(branch, ring.axis()?)?;
            }
            model.add_ring(ring.try_into()?)?;
        }
        Ok(model)
    }
}

impl Default for Model {
    fn default() -> Self {
        Model::new()
    }
}

impl Model {
    /// Create a new 3D model
    pub fn new() -> Model {
        let builder = Mesh::builder();
        let points = vec![];
        let branches = HashMap::new();
        Model {
            builder,
            ring_id: 0,
            ring: None,
            points,
            branches,
        }
    }

    /// Get the current ring ID
    fn ring_id(&self) -> usize {
        self.ring_id
    }

    /// Push one point
    fn push_pt(&mut self, order_deg: usize, pt_type: PtType) {
        let ring_id = self.ring_id();
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
                    self.push_pt(order_deg, PtType::Vertex(vid));
                    let rot = Quat::from_axis_angle(axis, angle)
                        * orthonormal_zero(axis);
                    let dist = dist * ring.scale();
                    let vtx = ring.center + rot * dist;
                    self.builder.push_vtx(vtx);
                }
                PtDef::Branch(branch) => {
                    self.push_pt(order_deg, PtType::Branch(branch.into()))
                }
            }
        }
    }

    /// Add a ring
    pub fn add_ring(&mut self, ring: Ring) -> Result<()> {
        let pring = self.ring.take();
        let mut ring = match &pring {
            Some(pr) => pr.clone().update_with(&ring),
            None => ring,
        };
        ring.id = self.ring_id();
        self.ring = Some(ring.clone());
        self.add_ring_points(&ring);
        if let Some(pring) = &pring {
            self.make_band(pring, &ring)?;
        }
        self.ring_id += 1;
        Ok(())
    }

    /// Add a cap
    fn add_cap(&mut self) -> Result<()> {
        let mut ring = self.ring.take().unwrap();
        let mut pts = self.ring_points(&ring, 0);
        let first = pts.pop().ok_or(Error::InvalidRing(ring.id))?;
        let vid = self.builder.vertices();
        let vtx = ring.center;
        self.builder.push_vtx(vtx);
        ring.id = self.ring_id();
        self.ring = Some(ring);
        self.push_pt(0, PtType::Vertex(vid));
        let cpt = self.points.last().unwrap().clone();
        let ring = self.ring.take().unwrap();
        let mut ppt = first.clone();
        while let Some(pt) = pts.pop() {
            self.add_face([&ppt, &pt, &cpt], ring.smoothing())?;
            ppt = pt;
        }
        self.add_face([&ppt, &first, &cpt], ring.smoothing())?;
        self.ring_id += 1;
        Ok(())
    }

    /// Add a branch base ring
    pub fn add_branch(
        &mut self,
        branch: &str,
        axis: Option<Vec3>,
    ) -> Result<()> {
        self.add_cap()?;
        let vertices = self.branch_vertices(branch);
        if vertices.is_empty() {
            return Err(Error::UnknownBranchLabel(branch.into()));
        }
        let id = self.ring_id();
        let len = vertices.len();
        let center = vertices
            .iter()
            .map(|i| self.builder.vertex(*i))
            .fold(Vec3::ZERO, |a, b| a + b)
            / len as f32;
        let axis = axis.unwrap_or_else(|| self.branch_axis(branch, center));
        let ring = Ring {
            id,
            center,
            axis: Some(axis),
            point_defs: vec![PtDef::Distance(1.0); len],
            ..Default::default()
        };
        self.ring = Some(ring);
        for (order_deg, vid) in self.branch_angles(branch, axis, center) {
            self.push_pt(order_deg, PtType::Vertex(vid));
        }
        self.ring_id += 1;
        Ok(())
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
    fn make_band(&mut self, ring0: &Ring, ring1: &Ring) -> Result<()> {
        if ring0.id == ring1.id {
            return Err(Error::InvalidRing(ring0.id));
        }
        // get points for each ring
        let mut pts0 = self.ring_points(ring0, ring1.half_step());
        let mut pts1 = self.ring_points(ring1, ring0.half_step());
        let first0 = pts0.pop().ok_or(Error::InvalidRing(ring0.id))?;
        let first1 = pts1.pop().ok_or(Error::InvalidRing(ring1.id))?;
        pts0.append(&mut pts1);
        pts0.sort();
        pts0.reverse();
        let mut band = pts0;
        let (mut pt0, mut pt1) = (first0.clone(), first1.clone());
        // create faces of band as a triangle strip
        while let Some(pt) = band.pop() {
            self.add_face([&pt1, &pt0, &pt], ring0.smoothing())?;
            if pt.ring_id == ring0.id {
                pt0 = pt;
            } else {
                pt1 = pt;
            }
        }
        // connect with first vertices on band
        self.add_face([&pt1, &pt0, &first1], ring0.smoothing())?;
        self.add_face([&first0, &first1, &pt0], ring0.smoothing())
    }

    /// Add a triangle face
    fn add_face(
        &mut self,
        pts: [&Point; 3],
        smoothing: Smoothing,
    ) -> Result<()> {
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
                let edges = self
                    .branches
                    .get_mut(b)
                    .ok_or_else(|| Error::UnknownBranchLabel(b.into()))?;
                edges.push([*v0, *v1]);
            }
            (PtType::Vertex(_v), PtType::Branch(b0), PtType::Branch(b1))
            | (PtType::Branch(b0), PtType::Vertex(_v), PtType::Branch(b1))
            | (PtType::Branch(b0), PtType::Branch(b1), PtType::Vertex(_v)) => {
                // A single vertex and two branch points:
                // - both points must be for the same branch
                // - no edges need to be added
                if b0 != b1 {
                    return Err(Error::InvalidBranches(b0.into(), b1.into()));
                }
            }
            (PtType::Branch(b0), PtType::Branch(b1), PtType::Branch(b2)) => {
                // Three adjacent branch points:
                // - all points must be for the same branch
                if b0 != b1 {
                    return Err(Error::InvalidBranches(b0.into(), b1.into()));
                }
                if b0 != b2 {
                    return Err(Error::InvalidBranches(b0.into(), b2.into()));
                }
            }
        }
        Ok(())
    }

    /// Write model as glTF
    pub fn write_gltf<W: Write>(mut self, writer: W) -> Result<()> {
        self.add_cap()?;
        let mesh = self.builder.build();
        gltf::export(writer, &mesh)?;
        Ok(())
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
