// husk.rs     Husk module
//
// Copyright (c) 2022-2023  Douglas Lau
//
use crate::error::{Error, Result};
use crate::gltf;
use crate::mesh::{Face, Mesh, MeshBuilder, Smoothing};
use crate::ring::{Branch, Degrees, Ring};
use glam::{Quat, Vec3};
use std::collections::HashMap;
use std::io::Write;

/// Point type
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum Pt {
    /// Vertex number
    Vertex(usize),

    /// Branch label
    Branch(String),
}

/// A point on a husk
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Point {
    /// Degrees around ring (must be first for `Ord`)
    order_deg: Degrees,

    /// Ring ID
    ring_id: usize,

    /// Point type
    pt_type: Pt,
}

/// Shell of a 3D model
///
/// A husk is a series of [Ring]s, possibly branching.
///
/// ```rust
/// # use homunculus::{Error, Husk, Ring};
/// # fn main() -> Result<(), Error> {
/// let mut pyramid = Husk::new();
/// let base = Ring::default().point(1.0).point(1.0).point(1.0);
/// pyramid.ring(base)?;
/// pyramid.ring(Ring::default().point(0.0))?;
/// # Ok(())
/// # }
/// ```
///
/// [ring]: struct.Ring.html
pub struct Husk {
    /// Mesh builder
    builder: MeshBuilder,

    /// Current ring ID
    ring_id: usize,

    /// Current ring
    ring: Option<Ring>,

    /// All points on mesh
    points: Vec<Point>,

    /// Mapping of labels to branches
    branches: HashMap<String, Branch>,
}

impl Default for Husk {
    fn default() -> Self {
        Husk::new()
    }
}

impl Husk {
    /// Create a new husk
    pub fn new() -> Self {
        Husk {
            builder: Mesh::builder(),
            ring_id: 0,
            ring: None,
            points: Vec::new(),
            branches: HashMap::new(),
        }
    }

    /// Add internal branch vertex
    fn add_branch_vertex(&mut self, label: &str, pos: Vec3) {
        if !self.branches.contains_key(label) {
            self.branches.insert(label.to_string(), Branch::default());
        }
        if let Some(branch) = self.branches.get_mut(label) {
            branch.push_internal(pos);
        }
    }

    /// Push one point
    fn push_pt(&mut self, order_deg: Degrees, pt_type: Pt) {
        let ring_id = self.ring_id;
        self.points.push(Point {
            order_deg,
            ring_id,
            pt_type,
        });
    }

    /// Add points for a ring
    fn add_ring_points(&mut self, ring: &Ring) {
        for (i, rpt) in ring.points().enumerate() {
            let angle = ring.angle(i);
            let order_deg = Degrees::from(angle);
            let rot = Quat::from_rotation_y(angle);
            let pos = rot
                * Vec3::new(rpt.distance * ring.scale_or_default(), 0.0, 0.0);
            let pos = ring.xform.transform_point3(pos);
            match &rpt.label {
                None => {
                    let vid = self.builder.push_vtx(pos);
                    self.push_pt(order_deg, Pt::Vertex(vid));
                }
                Some(label) => {
                    self.add_branch_vertex(label, pos);
                    self.push_pt(order_deg, Pt::Branch(label.into()))
                }
            }
        }
    }

    /// Add a ring to the current branch
    ///
    /// All unset properties are copied from the previous ring:
    /// - axis
    /// - scale
    /// - smoothing
    /// - points
    pub fn ring(&mut self, ring: Ring) -> Result<()> {
        let pring = self.ring.take();
        let mut ring = match &pring {
            Some(pr) => pr.with_ring(&ring),
            None => ring,
        };
        ring.id = self.ring_id;
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
        if pts.is_empty() {
            return Ok(());
        }
        // add cap center point
        let pos = ring.xform.transform_point3(Vec3::ZERO);
        let vid = self.builder.push_vtx(pos);
        ring.id = self.ring_id;
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
        let branch = self.take_branch(label)?;
        let mut ring = Ring::with_branch(&branch, &self.builder);
        if let Some(axis) = axis {
            ring = ring.axis(axis);
        }
        ring.id = self.ring_id;
        for (order_deg, vid) in self.edge_angles(&branch, &ring) {
            self.push_pt(order_deg, Pt::Vertex(vid));
        }
        self.ring = Some(ring);
        self.ring_id += 1;
        Ok(())
    }

    /// Take a branch by label
    fn take_branch(&mut self, label: &str) -> Result<Branch> {
        self.branches
            .remove(label)
            .ok_or_else(|| Error::UnknownBranchLabel(label.into()))
    }

    /// Calculate edge angles for a branch base
    fn edge_angles(
        &self,
        branch: &Branch,
        ring: &Ring,
    ) -> Vec<(Degrees, usize)> {
        let inverse = ring.xform.inverse();
        let zero_deg = Vec3::new(1.0, 0.0, 0.0);
        // Step 1: find "first" edge vertex (closest to 0 degrees)
        let mut edge = 0;
        let mut angle = f32::MAX;
        for (i, ed) in branch.edges().enumerate() {
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
        if pt1 != first1 {
            self.add_face([&pt1, &pt0, &first1], ring0.smoothing_or_default())?;
        }
        if pt0 != first0 {
            self.add_face(
                [&first0, &first1, &pt0],
                ring0.smoothing_or_default(),
            )?;
        }
        Ok(())
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
            (Pt::Branch(lbl), Pt::Vertex(v0), Pt::Vertex(v1))
            | (Pt::Vertex(v1), Pt::Branch(lbl), Pt::Vertex(v0))
            | (Pt::Vertex(v0), Pt::Vertex(v1), Pt::Branch(lbl)) => {
                let branch = self
                    .branches
                    .get_mut(lbl)
                    .ok_or_else(|| Error::UnknownBranchLabel(lbl.into()))?;
                branch.push_edge(*v0, *v1);
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

    /// Write husk as [glTF] `.glb`
    ///
    /// ```rust,no_run
    /// # use homunculus::Husk;
    /// # use std::fs::File;
    /// let mut husk = Husk::new();
    /// // add rings …
    /// let file = File::create("husk.glb").unwrap();
    /// husk.write_gltf(file).unwrap();
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
