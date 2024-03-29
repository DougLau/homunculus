// husk.rs     Husk module
//
// Copyright (c) 2022-2023  Douglas Lau
//
use crate::error::{Error, Result};
use crate::gltf;
use crate::mesh::{Face, Mesh, MeshBuilder};
use crate::ring::{Branch, Degrees, Point, Pt, Ring, Shading};
use glam::Vec3;
use std::collections::HashMap;
use std::io::Write;

/// Outer shell of a 3D model
///
/// A husk is a series of [Ring]s, possibly branching.
///
/// ```rust
/// # use homunculus::{Error, Husk, Ring};
/// # fn main() -> Result<(), Error> {
/// let mut pyramid = Husk::new();
/// let base = Ring::default().spoke(1.0).spoke(1.0).spoke(1.0);
/// pyramid.ring(base)?;
/// pyramid.ring(Ring::default().spoke(0.0))?;
/// # Ok(())
/// # }
/// ```
///
/// [ring]: struct.Ring.html
pub struct Husk {
    /// Mesh builder
    builder: MeshBuilder,

    /// Current surface
    surface: u16,

    /// Current ring
    ring: Option<Ring>,

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
            surface: 0,
            ring: None,
            branches: HashMap::new(),
        }
    }

    /// Push internal branch point
    fn push_branch_internal(&mut self, label: &str, pos: Vec3) {
        if !self.branches.contains_key(label) {
            self.branches.insert(label.to_string(), Branch::default());
        }
        if let Some(branch) = self.branches.get_mut(label) {
            branch.push_internal(pos);
        }
    }

    /// Push branch edge
    fn push_branch_edge(&mut self, label: &str, v0: usize, v1: usize) {
        if !self.branches.contains_key(label) {
            self.branches.insert(label.to_string(), Branch::default());
        }
        if let Some(branch) = self.branches.get_mut(label) {
            branch.push_edge(v0, v1);
        }
    }

    /// Add branch points for a ring
    fn add_branch_points(&mut self, ring: &Ring) {
        for point in ring.points() {
            if let Pt::Branch(label, pos) = &point.pt {
                self.push_branch_internal(label, *pos);
            }
        }
    }

    /// Add a ring to the current branch
    ///
    /// All unset properties are copied from the previous ring:
    /// - spacing
    /// - scale
    /// - shading
    /// - spokes
    pub fn ring(&mut self, ring: Ring) -> Result<()> {
        let pring = self.ring.take();
        let mut ring = match &pring {
            Some(pr) => pr.with_ring(&ring),
            None => ring,
        };
        if ring.points().len() == 0 {
            ring.make_points(&mut self.builder);
            self.add_branch_points(&ring);
        }
        if let Some(pring) = &pring {
            self.make_band(pring, &ring)?;
        }
        self.ring = Some(ring);
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
    fn cap_ring(&mut self, ring: Ring) -> Result<()> {
        let mut pts = ring.points_offset(Degrees(0));
        // unwrap note: ring will always have at least one point
        let last = pts.pop().unwrap();
        if pts.len() < 2 {
            return Ok(());
        }
        // add hub point
        let (order, pos) = ring.make_hub();
        let vid = self.builder.push_vtx(pos);
        let hub = Point::new(Pt::Vertex(vid), order);
        let mut prev = last.clone();
        for pt in pts.drain(..) {
            self.add_face([&pt, &prev, &hub])?;
            prev = pt;
            if ring.shading_or_default() == Shading::Flat {
                self.surface += 1;
            }
        }
        self.add_face([&last, &prev, &hub])?;
        if ring.shading_or_default() == Shading::Flat {
            self.surface += 1;
        }
        Ok(())
    }

    /// End the current branch and get the `label` branch
    ///
    /// The `label` must match one or more [Spoke]s from earlier rings.
    ///
    /// [spoke]: struct.Spoke.html
    pub fn branch(&mut self, label: impl AsRef<str>) -> Result<Ring> {
        self.cap()?;
        let branch = self.take_branch(label.as_ref())?;
        Ok(Ring::with_branch(branch, &self.builder))
    }

    /// Take a branch by label
    fn take_branch(&mut self, label: &str) -> Result<Branch> {
        self.branches
            .remove(label)
            .ok_or_else(|| Error::UnknownBranchLabel(label.to_string()))
    }

    /// Make a band of faces between two rings
    fn make_band(&mut self, ring0: &Ring, ring1: &Ring) -> Result<()> {
        if ring0.shading_or_default() != Shading::Smooth {
            self.surface += 1;
        }
        // get points for each ring
        let mut pts0 = ring0.points_offset(ring1.half_step());
        let mut pts1 = ring1.points_offset(ring0.half_step());
        // unwrap note: ring will always have at least one point
        let first0 = pts0.pop().unwrap();
        let first1 = pts1.pop().unwrap();
        let (mut pt0, mut pt1) = (first0.clone(), first1.clone());
        let mut band = Vec::with_capacity(pts0.len() + pts1.len());
        band.extend_from_slice(&pts0[..]);
        band.append(&mut pts1);
        band.sort_by(|a, b| b.order.partial_cmp(&a.order).unwrap());
        // create faces of band as a triangle strip
        while let Some(pt) = band.pop() {
            self.add_face([&pt1, &pt0, &pt])?;
            if pts0.contains(&pt) {
                pt0 = pt;
            } else {
                pt1 = pt;
            }
            if ring0.shading_or_default() == Shading::Flat {
                self.surface += 1;
            }
        }
        // connect with first vertices on band
        if pt1 != first1 {
            self.add_face([&pt1, &pt0, &first1])?;
            if ring0.shading_or_default() == Shading::Flat {
                self.surface += 1;
            }
        }
        if pt0 != first0 {
            self.add_face([&first0, &first1, &pt0])?;
            if ring0.shading_or_default() == Shading::Flat {
                self.surface += 1;
            }
        }
        Ok(())
    }

    /// Add a triangle face
    fn add_face(&mut self, pts: [&Point; 3]) -> Result<()> {
        match (&pts[0].pt, &pts[1].pt, &pts[2].pt) {
            (Pt::Vertex(v0), Pt::Vertex(v1), Pt::Vertex(v2)) => {
                let face = Face::new([*v0, *v1, *v2], self.surface);
                self.builder.push_face(face);
            }
            (Pt::Branch(lbl, _), Pt::Vertex(v0), Pt::Vertex(v1))
            | (Pt::Vertex(v1), Pt::Branch(lbl, _), Pt::Vertex(v0))
            | (Pt::Vertex(v0), Pt::Vertex(v1), Pt::Branch(lbl, _)) => {
                self.push_branch_edge(lbl, *v0, *v1);
            }
            (Pt::Vertex(_v), Pt::Branch(b0, _), Pt::Branch(b1, _))
            | (Pt::Branch(b0, _), Pt::Vertex(_v), Pt::Branch(b1, _))
            | (Pt::Branch(b0, _), Pt::Branch(b1, _), Pt::Vertex(_v)) => {
                // A single vertex and two branch points:
                // - both points must be for the same branch
                // - no edges need to be added
                if b0 != b1 {
                    return Err(Error::InvalidBranches(format!(
                        "{b0} != {b1}"
                    )));
                }
            }
            (Pt::Branch(b0, _), Pt::Branch(b1, _), Pt::Branch(b2, _)) => {
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
    /// # use homunculus::{Error, Husk};
    /// # use std::fs::File;
    /// # fn main() -> Result<(), Error> {
    /// let mut husk = Husk::new();
    /// // add rings …
    /// let file = File::create("husk.glb")?;
    /// husk.write_gltf(file)?;
    /// # Ok(())
    /// # }
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
