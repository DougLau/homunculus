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

    /// Axis vector
    axis: Option<Vec3>,

    /// Point definitions
    point_defs: Vec<PtDef>,

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
    /// Branch connection vertices
    vertices: Vec<Vec3>,

    /// Branch edges
    edges: Vec<Edge>,
}

/// A 3D model
pub struct Model {
    /// Mesh builder
    builder: MeshBuilder,

    /// Global transform
    xform: Affine3A,

    /// Current ring ID
    ring_id: usize,

    /// Current ring
    ring: Option<Ring>,

    /// All points on mesh
    points: Vec<Point>,

    /// Mapping of labels to branches
    branches: HashMap<String, Branch>,
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
            axis: None,
            point_defs: Vec::new(),
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

    /// Get ring scale mutably
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
        self
    }

    /// Add a point
    pub fn add_point(&mut self, dist: f32) {
        self.point_defs.push(PtDef::Distance(dist));
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

    /// Translate a transform from axis
    fn transform_translate(&self, xform: &mut Affine3A) {
        xform.translation += xform.matrix3.mul_vec3a(Vec3A::from(self.axis()));
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
            vertices: Vec::new(),
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

    /// Get center of branch vertices
    fn center(&self) -> Vec3 {
        let len = self.vertices.len() as f32;
        self.vertices.iter().fold(Vec3::ZERO, |a, b| a + *b) / len
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

    /// Add branch vertex
    fn add_branch_vertex(&mut self, branch: &str, pos: Vec3) {
        if !self.branches.contains_key(branch) {
            self.branches.insert(branch.to_string(), Branch::new());
        }
        // unwrap can never panic because of contains_key test
        let branch = self.branches.get_mut(branch).unwrap();
        branch.vertices.push(pos);
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
        for (i, ptd) in ring.point_defs.iter().enumerate() {
            let angle = ring.angle(i);
            let order_deg = angle.to_degrees() as usize;
            let rot = Quat::from_rotation_y(angle);
            match ptd {
                PtDef::Distance(dist) => {
                    let pos = rot * Vec3::new(dist * ring.scale(), 0.0, 0.0);
                    let pos = self.xform.transform_point3(pos);
                    let vid = self.builder.push_vtx(pos);
                    self.push_pt(order_deg, PtType::Vertex(vid));
                }
                PtDef::Branch(branch) => {
                    let pos = rot * Vec3::new(ring.scale(), 0.0, 0.0);
                    let pos = self.xform.transform_point3(pos);
                    self.add_branch_vertex(branch, pos);
                    self.push_pt(order_deg, PtType::Branch(branch.into()))
                }
            }
        }
    }

    /// Add a ring
    pub fn add_ring(&mut self, ring: Ring) -> Result<()> {
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

    /// Add a cap
    fn add_cap(&mut self) -> Result<()> {
        let mut ring = self.ring.take().ok_or(Error::MissingRing())?;
        let mut pts = self.ring_points(&ring, 0);
        let first = pts.pop().ok_or(Error::InvalidRing(ring.id))?;
        let pos = self.xform.transform_point3(Vec3::ZERO);
        let vid = self.builder.push_vtx(pos);
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
        let id = self.ring_id();
        let (center, len) = self.branch_center_vertices(branch)?;
        self.xform = Affine3A::from_translation(center);
        let ax = self.branch_axis(branch);
        let mut ring = Ring {
            id,
            axis: Some(ax),
            point_defs: vec![PtDef::Distance(1.0); len],
            ..Default::default()
        };
        ring.transform_rotate(&mut self.xform);
        if let Some(axis) = axis {
            ring.axis = Some(axis);
            ring.transform_rotate(&mut self.xform);
        }
        self.ring = Some(ring);
        for (order_deg, vid) in self.branch_angles(branch) {
            self.push_pt(order_deg, PtType::Vertex(vid));
        }
        self.ring_id += 1;
        Ok(())
    }

    /// Get center of a branch base
    fn branch_center_vertices(&self, branch: &str) -> Result<(Vec3, usize)> {
        match self.branches.get(branch) {
            Some(branch) => {
                let center = branch.center();
                let count = branch.edge_vertex_count();
                Ok((center, count))
            }
            None => Err(Error::UnknownBranchLabel(branch.into())),
        }
    }

    /// Calculate axis for a branch base
    fn branch_axis(&self, branch: &str) -> Vec3 {
        let center = self.xform.transform_point3(Vec3::ZERO);
        match self.branches.get(branch) {
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
    fn branch_angles(&self, branch: &str) -> Vec<(usize, usize)> {
        match self.branches.get(branch) {
            Some(branch) => self.edge_angles(branch),
            None => Vec::new(),
        }
    }

    /// Calculate edge angles for a branch base
    fn edge_angles(&self, branch: &Branch) -> Vec<(usize, usize)> {
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
            let order_deg = angle.to_degrees() as usize;
            angles.push((order_deg, vid));
            ppos = pos;
        }
        angles
    }

    /// Get the points for one ring
    fn ring_points(&self, ring: &Ring, hs_other: usize) -> Vec<Point> {
        let mut pts = Vec::new();
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
                let branch = self
                    .branches
                    .get_mut(b)
                    .ok_or_else(|| Error::UnknownBranchLabel(b.into()))?;
                branch.edges.push(Edge(*v0, *v1));
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

    /// Write model as [glTF] `.glb`
    ///
    /// ```rust,no_run
    /// # use homunculus::Model;
    /// # use std::fs::File;
    /// let mut model = Model::new();
    /// // add rings …
    /// let file = File::open("model.glb").unwrap();
    /// model.write_gltf(file).unwrap();
    /// ```
    ///
    /// [gltf]: https://en.wikipedia.org/wiki/GlTF
    pub fn write_gltf<W: Write>(mut self, writer: W) -> Result<()> {
        self.add_cap()?;
        let mesh = self.builder.build();
        gltf::export(writer, &mesh)?;
        Ok(())
    }
}
