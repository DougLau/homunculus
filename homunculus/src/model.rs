// model.rs     Model module
//
// Copyright (c) 2022-2023  Douglas Lau
//
use crate::error::{Error, Result};
use crate::gltf;
use crate::mesh::{Face, Mesh, MeshBuilder, Smoothing};
use glam::{Affine3A, Mat3A, Quat, Vec2, Vec3, Vec3A};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::io::Write;
use std::ops::Add;

/// Angular degrees
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Degrees(u16);

/// Ring point
///
/// A point on a ring with distance from the central axis.  An optional label
/// can be declared for a [branch] point.
///
/// [branch]: struct.Model.html#method.branch
#[derive(Clone, Debug)]
pub struct RingPoint {
    /// Distance from axis
    distance: f32,

    /// Branch label
    branch: Option<String>,
}

/// Point type
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum Pt {
    /// Vertex number
    Vertex(usize),

    /// Branch label
    Branch(String),
}

/// A point on model surface
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Point {
    /// Degrees around ring (must be first for `Ord`)
    order_deg: Degrees,

    /// Ring ID
    ring_id: usize,

    /// Point type
    pt_type: Pt,
}

/// Ring around a [Model] hull
///
/// [model]: struct.Model.html
#[derive(Clone, Debug, Default)]
pub struct Ring {
    /// Ring ID
    id: usize,

    /// Axis vector
    axis: Option<Vec3>,

    /// Ring points
    points: Vec<RingPoint>,

    /// Scale factor
    scale: Option<f32>,

    /// Edge smoothing
    smoothing: Option<Smoothing>,
}

/// Model edge between two vertices
#[derive(Clone, Debug)]
struct Edge(usize, usize);

/// Branch data
#[derive(Debug)]
struct Branch {
    /// Internal connection vertices (non-edge)
    internal: Vec<Vec3>,

    /// Edges at base of branch
    edges: Vec<Edge>,
}

/// A 3D model
///
/// A series of [Ring]s defines the hull of a model.
///
/// ```rust
/// # use homunculus::{Model, Ring};
/// let mut model = Model::new();
/// model.ring(Ring::default().point(2.0).point(1.0).point(1.5));
/// model.ring(Ring::default().point(1.5).point(1.1).point(1.2));
/// ```
///
/// [ring]: struct.Ring.html
pub struct Model {
    /// Mesh builder
    builder: MeshBuilder,

    /// Current ring ID
    ring_id: usize,

    /// Global transform for current ring
    xform: Affine3A,

    /// Current ring
    ring: Option<Ring>,

    /// All points on mesh
    points: Vec<Point>,

    /// Mapping of labels to branches
    branches: HashMap<String, Branch>,
}

impl From<f32> for Degrees {
    fn from(angle: f32) -> Self {
        let deg = angle.to_degrees().rem_euclid(360.0);
        Degrees(deg.round() as u16)
    }
}

impl Add for Degrees {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Degrees(self.0 + rhs.0 % 360)
    }
}

impl Default for RingPoint {
    fn default() -> Self {
        RingPoint::from(1.0)
    }
}

impl From<f32> for RingPoint {
    fn from(distance: f32) -> Self {
        RingPoint {
            distance,
            branch: None,
        }
    }
}

impl From<&str> for RingPoint {
    fn from(branch: &str) -> Self {
        RingPoint {
            distance: 1.0,
            branch: Some(branch.to_string()),
        }
    }
}

impl From<(f32, &str)> for RingPoint {
    fn from(val: (f32, &str)) -> Self {
        RingPoint {
            distance: val.0,
            branch: Some(val.1.to_string()),
        }
    }
}

impl Ring {
    /// Create a new branch ring
    fn with_branch(id: usize, axis: Vec3, pts: usize) -> Self {
        Ring {
            id,
            axis: Some(axis),
            points: vec![RingPoint::default(); pts],
            scale: None,
            smoothing: None,
        }
    }

    /// Set ring axis
    pub fn axis(mut self, axis: Option<Vec3>) -> Self {
        self.axis = axis;
        self
    }

    /// Set ring scale
    pub fn scale(mut self, scale: Option<f32>) -> Self {
        self.scale = scale;
        self
    }

    /// Set edge smoothing
    pub fn smoothing(mut self, smoothing: Option<Smoothing>) -> Self {
        self.smoothing = smoothing;
        self
    }

    /// Get the ring axis (or default value)
    fn axis_or_default(&self) -> Vec3 {
        self.axis.unwrap_or_else(|| Vec3::new(0.0, 1.0, 0.0))
    }

    /// Get the ring scale (or default value)
    fn scale_or_default(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }

    /// Get the edge smoothing (or default value)
    fn smoothing_or_default(&self) -> Smoothing {
        self.smoothing.unwrap_or(Smoothing::Smooth)
    }

    /// Update with another ring
    fn update_with(mut self, ring: &Self) -> Self {
        if ring.axis.is_some() {
            self.axis = ring.axis;
        }
        if !ring.points.is_empty() {
            self.points = ring.points.clone();
        }
        if ring.scale.is_some() {
            self.scale = ring.scale;
        }
        if ring.smoothing.is_some() {
            self.smoothing = ring.smoothing;
        }
        self
    }

    /// Add a ring / [branch] point
    ///
    /// ```rust
    /// # use homunculus::Ring;
    /// let ring = Ring::default()
    ///     .point(2.0)
    ///     .point(2.7)
    ///     .point("branch A")
    ///     .point((1.6, "branch A"))
    ///     .point(1.8);
    /// ```
    /// [branch]: struct.Model.html#method.branch
    pub fn point<P: Into<RingPoint>>(mut self, pt: P) -> Self {
        self.points.push(pt.into());
        self
    }

    /// Get half step in degrees
    fn half_step(&self) -> Degrees {
        let deg = 180 / self.points.len();
        Degrees(deg as u16)
    }

    /// Calculate the angle of a point
    fn angle(&self, i: usize) -> f32 {
        2.0 * PI * i as f32 / self.points.len() as f32
    }

    /// Translate a transform from axis
    fn transform_translate(&self, xform: &mut Affine3A) {
        xform.translation +=
            xform.matrix3.mul_vec3a(Vec3A::from(self.axis_or_default()));
    }

    /// Rotate a transform from axis
    fn transform_rotate(&mut self, xform: &mut Affine3A) {
        if let Some(axis) = self.axis {
            let length = axis.length();
            let axis = axis.normalize();
            if axis.x != 0.0 {
                // project to XY plane, then rotate around Z axis
                let up = Vec2::new(0.0, 1.0);
                let proj = Vec2::new(axis.x, axis.y);
                let angle = up.angle_between(proj) * proj.length();
                xform.matrix3 *= Mat3A::from_rotation_z(angle);
            }
            if axis.z != 0.0 {
                // project to YZ plane, then rotate around X axis
                let up = Vec2::new(1.0, 0.0);
                let proj = Vec2::new(axis.y, axis.z);
                let angle = up.angle_between(proj) * proj.length();
                xform.matrix3 *= Mat3A::from_rotation_x(angle);
            }
            // adjust axis after rotation applied
            self.axis = Some(Vec3::new(0.0, length, 0.0));
        }
    }
}

impl Branch {
    /// Create a new branch
    fn new() -> Self {
        Branch {
            internal: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Get edge vertices sorted by common end-points
    fn edge_vids(&self, edge: usize) -> Vec<usize> {
        let mut edges = self.edges.to_vec();
        if edge > 0 {
            edges.swap(0, edge);
        }
        let mut vid = edges[0].1;
        for i in 1..edges.len() {
            for j in (i + 1)..edges.len() {
                if vid == edges[j].0 {
                    edges.swap(i, j);
                }
            }
            vid = edges[i].1;
        }
        edges.into_iter().map(|e| e.0).collect()
    }

    /// Get center of internal vertices
    fn center(&self) -> Vec3 {
        let len = self.internal.len() as f32;
        self.internal.iter().fold(Vec3::ZERO, |a, b| a + *b) / len
    }

    /// Get count of vertices on edges
    fn edge_vertex_count(&self) -> usize {
        let mut vertices = self
            .edges
            .iter()
            .flat_map(|e| [e.0, e.1].into_iter())
            .collect::<Vec<usize>>();
        vertices.sort();
        vertices.dedup();
        vertices.len()
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
        Model {
            builder: Mesh::builder(),
            ring_id: 0,
            xform: Affine3A::IDENTITY,
            ring: None,
            points: Vec::new(),
            branches: HashMap::new(),
        }
    }

    /// Get the current ring ID
    fn ring_id(&self) -> usize {
        self.ring_id
    }

    /// Add internal branch vertex
    fn add_branch_vertex(&mut self, branch: &str, pos: Vec3) {
        if !self.branches.contains_key(branch) {
            self.branches.insert(branch.to_string(), Branch::new());
        }
        // unwrap can never panic because of contains_key test
        let branch = self.branches.get_mut(branch).unwrap();
        branch.internal.push(pos);
    }

    /// Push one point
    fn push_pt(&mut self, order_deg: Degrees, pt_type: Pt) {
        let ring_id = self.ring_id();
        self.points.push(Point {
            order_deg,
            ring_id,
            pt_type,
        });
    }

    /// Add points for a ring
    fn add_ring_points(&mut self, ring: &Ring) {
        for (i, ptd) in ring.points.iter().enumerate() {
            let angle = ring.angle(i);
            let order_deg = Degrees::from(angle);
            let rot = Quat::from_rotation_y(angle);
            let pos = rot
                * Vec3::new(ptd.distance * ring.scale_or_default(), 0.0, 0.0);
            let pos = self.xform.transform_point3(pos);
            match &ptd.branch {
                None => {
                    let vid = self.builder.push_vtx(pos);
                    self.push_pt(order_deg, Pt::Vertex(vid));
                }
                Some(branch) => {
                    self.add_branch_vertex(branch, pos);
                    self.push_pt(order_deg, Pt::Branch(branch.into()))
                }
            }
        }
    }

    /// Add a ring
    pub fn ring(&mut self, ring: Ring) -> Result<()> {
        let pring = self.ring.take();
        let mut ring = match &pring {
            Some(pr) => {
                let mut ring = pr.clone().update_with(&ring);
                ring.transform_translate(&mut self.xform);
                ring.transform_rotate(&mut self.xform);
                ring
            }
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

    /// Add a cap face on the current branch
    fn cap(&mut self) -> Result<()> {
        match self.ring.take() {
            Some(ring) => self.cap_ring(ring),
            None => Ok(()),
        }
    }

    /// Add a cap face on the given ring
    fn cap_ring(&mut self, mut ring: Ring) -> Result<()> {
        let mut pts = self.ring_points(&ring, Degrees(0));
        let last = pts.pop().ok_or(Error::InvalidRing(ring.id))?;
        // add cap center point
        let pos = self.xform.transform_point3(Vec3::ZERO);
        let vid = self.builder.push_vtx(pos);
        ring.id = self.ring_id();
        self.push_pt(Degrees(0), Pt::Vertex(vid));
        let center = self.points.last().unwrap().clone();
        let mut prev = last.clone();
        for pt in pts.drain(..) {
            self.add_face([&pt, &prev, &center], ring.smoothing_or_default())?;
            prev = pt;
        }
        self.add_face([&last, &prev, &center], ring.smoothing_or_default())?;
        self.ring_id += 1;
        Ok(())
    }

    /// End the current branch and start the `label` branch
    pub fn branch(
        &mut self,
        label: impl AsRef<str>,
        axis: Option<Vec3>,
    ) -> Result<()> {
        self.cap()?;
        let label = label.as_ref();
        let id = self.ring_id();
        let (center, len) = self.branch_center_vertices(label)?;
        self.xform = Affine3A::from_translation(center);
        // start with base of branch
        let ax = self.branch_axis(label);
        let mut ring = Ring::with_branch(id, ax, len);
        ring.transform_rotate(&mut self.xform);
        if let Some(axis) = axis {
            // modify axis if specified
            ring.axis = Some(axis);
            ring.transform_rotate(&mut self.xform);
        }
        self.ring = Some(ring);
        for (order_deg, vid) in self.branch_angles(label) {
            self.push_pt(order_deg, Pt::Vertex(vid));
        }
        self.ring_id += 1;
        Ok(())
    }

    /// Get center of a branch base
    fn branch_center_vertices(&self, label: &str) -> Result<(Vec3, usize)> {
        match self.branches.get(label) {
            Some(branch) => {
                let center = branch.center();
                let count = branch.edge_vertex_count();
                Ok((center, count))
            }
            None => Err(Error::UnknownBranchLabel(label.into())),
        }
    }

    /// Calculate axis for a branch base
    fn branch_axis(&self, label: &str) -> Vec3 {
        let center = self.xform.transform_point3(Vec3::ZERO);
        match self.branches.get(label) {
            Some(branch) => {
                let mut norm = Vec3::ZERO;
                for edge in &branch.edges {
                    let v0 = self.builder.vertex(edge.0);
                    let v1 = self.builder.vertex(edge.1);
                    norm += (v0 - center).cross(v1 - center);
                }
                norm.normalize()
            }
            None => Vec3::new(0.0, 1.0, 0.0),
        }
    }

    /// Calculate angles for a branch base
    fn branch_angles(&self, label: &str) -> Vec<(Degrees, usize)> {
        match self.branches.get(label) {
            Some(branch) => self.edge_angles(branch),
            None => Vec::new(),
        }
    }

    /// Calculate edge angles for a branch base
    fn edge_angles(&self, branch: &Branch) -> Vec<(Degrees, usize)> {
        let inverse = self.xform.inverse();
        let zero_deg = Vec3::new(1.0, 0.0, 0.0);
        // Step 1: find "first" edge vertex (closest to 0 degrees)
        let mut edge = 0;
        let mut angle = f32::MAX;
        for (i, ed) in branch.edges.iter().enumerate() {
            let vid = ed.0;
            let pos = inverse.transform_point3(self.builder.vertex(vid));
            let pos = Vec3::new(pos.x, 0.0, pos.z);
            let ang = zero_deg.angle_between(pos);
            if ang < angle {
                angle = ang;
                edge = i;
            }
        }
        // Step 2: sort edge vertices by common end-points
        let vids = branch.edge_vids(edge);
        // Step 3: make vec of (order_deg, vid)
        let mut angle = 0.0;
        let mut ppos = zero_deg;
        let mut angles = Vec::with_capacity(vids.len());
        for vid in vids {
            let pos = inverse.transform_point3(self.builder.vertex(vid));
            let pos = Vec3::new(pos.x, 0.0, pos.z);
            let ang = ppos.angle_between(pos);
            angle += ang;
            let order_deg = Degrees::from(angle);
            angles.push((order_deg, vid));
            ppos = pos;
        }
        angles
    }

    /// Get the points for one ring
    fn ring_points(&self, ring: &Ring, hs_other: Degrees) -> Vec<Point> {
        let mut pts = Vec::new();
        for point in &self.points {
            if point.ring_id == ring.id {
                let mut pt = point.clone();
                // adjust degrees by half step of other ring
                pt.order_deg = pt.order_deg + hs_other;
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
            self.add_face([&pt1, &pt0, &pt], ring0.smoothing_or_default())?;
            if pt.ring_id == ring0.id {
                pt0 = pt;
            } else {
                pt1 = pt;
            }
        }
        // connect with first vertices on band
        self.add_face([&pt1, &pt0, &first1], ring0.smoothing_or_default())?;
        self.add_face([&first0, &first1, &pt0], ring0.smoothing_or_default())
    }

    /// Add a triangle face
    fn add_face(
        &mut self,
        pts: [&Point; 3],
        smoothing: Smoothing,
    ) -> Result<()> {
        match (&pts[0].pt_type, &pts[1].pt_type, &pts[2].pt_type) {
            (Pt::Vertex(v0), Pt::Vertex(v1), Pt::Vertex(v2)) => {
                let face = Face::new([*v0, *v1, *v2], smoothing);
                self.builder.push_face(face);
            }
            (Pt::Branch(b), Pt::Vertex(v0), Pt::Vertex(v1))
            | (Pt::Vertex(v1), Pt::Branch(b), Pt::Vertex(v0))
            | (Pt::Vertex(v0), Pt::Vertex(v1), Pt::Branch(b)) => {
                let branch = self
                    .branches
                    .get_mut(b)
                    .ok_or_else(|| Error::UnknownBranchLabel(b.into()))?;
                branch.edges.push(Edge(*v0, *v1));
            }
            (Pt::Vertex(_v), Pt::Branch(b0), Pt::Branch(b1))
            | (Pt::Branch(b0), Pt::Vertex(_v), Pt::Branch(b1))
            | (Pt::Branch(b0), Pt::Branch(b1), Pt::Vertex(_v)) => {
                // A single vertex and two branch points:
                // - both points must be for the same branch
                // - no edges need to be added
                if b0 != b1 {
                    return Err(Error::InvalidBranches(format!(
                        "{b0} != {b1}"
                    )));
                }
            }
            (Pt::Branch(b0), Pt::Branch(b1), Pt::Branch(b2)) => {
                // Three adjacent branch points:
                // - all points must be for the same branch
                if b0 != b1 {
                    return Err(Error::InvalidBranches(format!(
                        "{b0} != {b1}"
                    )));
                }
                if b0 != b2 {
                    return Err(Error::InvalidBranches(format!(
                        "{b0} != {b2}"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Write model as [glTF] `.glb`
    ///
    /// ```rust,no_run
    /// # use homunculus::Model;
    /// # use std::fs::File;
    /// let mut model = Model::new();
    /// // add rings …
    /// let file = File::create("model.glb").unwrap();
    /// model.write_gltf(file).unwrap();
    /// ```
    ///
    /// [gltf]: https://en.wikipedia.org/wiki/GlTF
    pub fn write_gltf<W: Write>(mut self, writer: W) -> Result<()> {
        self.cap()?;
        let mesh = self.builder.build();
        gltf::export(writer, &mesh)?;
        Ok(())
    }
}
