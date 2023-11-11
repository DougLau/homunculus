// ring.rs     Ring module
//
// Copyright (c) 2022-2023  Douglas Lau
//
use crate::mesh::MeshBuilder;
use glam::{Affine3A, Mat3A, Quat, Vec2, Vec3, Vec3A};
use std::f32::consts::PI;
use std::ops::Add;

/// Angular degrees
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Degrees(pub u16);

/// Ring spoke
///
/// A spoke on a [ring] with distance from the central axis.  An optional
/// `label` can be declared for a [branch].
///
/// ```rust
/// # use homunculus::Spoke;
/// let spoke_a = Spoke::from(1.23);
/// let spoke_b = Spoke::from("branch");
/// let spoke_c = Spoke::from((2.5, "branch B"));
/// ```
/// [branch]: struct.Husk.html#method.branch
/// [ring]: struct.Ring.html#method.spoke
#[derive(Clone, Debug)]
pub struct Spoke {
    /// Distance from axis
    pub distance: f32,

    /// Label for branch points
    pub label: Option<String>,
}

/// Empty ring spokes
const EMPTY_RING: &[Spoke] = &[Spoke {
    distance: 0.0,
    label: None,
}];

/// Point type
#[derive(Clone, Debug, PartialEq)]
pub enum Pt {
    /// Vertex index
    Vertex(usize),

    /// Branch label
    Branch(String, Vec3),
}

/// A point on a ring
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    /// Point type
    pub pt: Pt,

    /// Degrees around ring
    pub order: Degrees,
}

/// Ring around a [Husk]
///
/// Points are distributed evenly around the ring.
///
/// [husk]: struct.Husk.html
#[derive(Clone, Debug, Default)]
pub struct Ring {
    /// Spacing to next ring
    spacing: Option<f32>,

    /// Spoke scale factor
    scale: Option<f32>,

    /// Vertex normal smoothing
    smoothing: Option<f32>,

    /// Spokes from center to ring
    spokes: Vec<Spoke>,

    /// Local-to-global transform
    xform: Affine3A,

    /// Points on ring
    points: Vec<Point>,
}

/// Edge between two vertices
#[derive(Clone, Copy, Debug)]
pub struct Edge(pub usize, pub usize);

/// Branch data
#[derive(Debug, Default)]
pub struct Branch {
    /// Internal connection points (non-edge)
    internal: Vec<Vec3>,

    /// Edges at base of branch
    edges: Vec<Edge>,
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

impl Default for Spoke {
    fn default() -> Self {
        Spoke::from(1.0)
    }
}

impl From<f32> for Spoke {
    fn from(distance: f32) -> Self {
        Spoke {
            distance,
            label: None,
        }
    }
}

impl From<&str> for Spoke {
    fn from(label: &str) -> Self {
        Spoke {
            distance: 1.0,
            label: Some(label.to_string()),
        }
    }
}

impl From<(f32, &str)> for Spoke {
    fn from(val: (f32, &str)) -> Self {
        Spoke {
            distance: val.0,
            label: Some(val.1.to_string()),
        }
    }
}

impl Ring {
    /// Create a new ring from a branch
    pub(crate) fn with_branch(branch: Branch, builder: &MeshBuilder) -> Self {
        let center = branch.center();
        let axis = branch.axis(builder, center);
        let xform = Affine3A::from_translation(center);
        let count = branch.edges.len();
        let mut ring = Ring {
            spacing: None,
            xform,
            scale: None,
            smoothing: None,
            spokes: vec![Spoke::default(); count],
            points: Vec::new(),
        };
        ring.transform_rotate(axis);
        for (order, vid) in branch.edge_angles(&ring, builder) {
            ring.push_pt(order, Pt::Vertex(vid));
        }
        ring
    }

    /// Create a ring updated with another ring
    pub(crate) fn with_ring(&self, ring: &Self) -> Self {
        let spacing = ring.spacing.or(self.spacing);
        let spokes = if ring.spokes.is_empty() {
            self.spokes.clone()
        } else {
            ring.spokes.clone()
        };
        let mut ring = Ring {
            spacing,
            xform: self.xform * ring.xform,
            scale: ring.scale.or(self.scale),
            smoothing: ring.smoothing.or(self.smoothing),
            spokes,
            points: Vec::new(),
        };
        ring.transform_translate();
        ring
    }

    /// Set ring axis
    ///
    /// Spacing between rings is determined by its length.
    ///
    /// # Panics
    ///
    /// This function will panic if any axis component is infinite or NaN.
    pub fn axis(mut self, axis: Vec3) -> Self {
        assert!(axis.x.is_finite());
        assert!(axis.y.is_finite());
        assert!(axis.z.is_finite());
        self.transform_rotate(axis);
        self
    }

    /// Set ring scale
    ///
    /// Spoke distances are scaled by this factor.
    ///
    /// # Panics
    ///
    /// This function will panic if the scale is negative, infinite, or NaN.
    pub fn scale(mut self, scale: f32) -> Self {
        assert!(scale.is_finite());
        assert!(scale.is_sign_positive());
        self.scale = Some(scale);
        self
    }

    /// Set normal smoothing factor
    ///
    /// Ranges from `0.0` (flat) to `1.0` (smooth)
    ///
    /// # Panics
    ///
    /// This function will panic if smoothing is negative, infinite, or NaN.
    pub fn smoothing(mut self, smoothing: f32) -> Self {
        assert!(smoothing.is_finite());
        assert!(smoothing.is_sign_positive());
        self.smoothing = Some(smoothing.min(1.0));
        self
    }

    /// Get the ring scale (or default value)
    fn scale_or_default(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }

    /// Get the normal smoothing factor (or default value)
    pub(crate) fn smoothing_or_default(&self) -> f32 {
        self.smoothing.unwrap_or(0.0)
    }

    /// Add a spoke
    ///
    /// A `label` is used for [branch] points.
    ///
    /// ```rust
    /// # use homunculus::Ring;
    /// let ring = Ring::default()
    ///     .spoke(2.0)
    ///     .spoke(2.7)
    ///     .spoke("branch A")
    ///     .spoke((1.6, "branch A"))
    ///     .spoke(1.8);
    /// ```
    ///
    /// # Panics
    ///
    /// This function will panic if the distance is negative, infinite, or NaN.
    ///
    /// [branch]: struct.Husk.html#method.branch
    pub fn spoke<S: Into<Spoke>>(mut self, spoke: S) -> Self {
        let spoke = spoke.into();
        assert!(spoke.distance.is_finite());
        assert!(spoke.distance.is_sign_positive());
        self.spokes.push(spoke);
        self
    }

    /// Get an iterator of spokes
    pub(crate) fn spokes(&self) -> impl Iterator<Item = &Spoke> {
        if self.spokes.is_empty() {
            EMPTY_RING.iter()
        } else {
            self.spokes[..].iter()
        }
    }

    /// Get half step in degrees
    pub(crate) fn half_step(&self) -> Degrees {
        let deg = 180 / self.spokes.len();
        Degrees(deg as u16)
    }

    /// Calculate the angle of a spoke
    pub(crate) fn angle(&self, i: usize) -> f32 {
        2.0 * PI * i as f32 / self.spokes.len() as f32
    }

    /// Translate a transform from axis
    fn transform_translate(&mut self) {
        let spacing = self.spacing.unwrap_or(1.0);
        let axis = Vec3A::new(0.0, spacing, 0.0);
        self.xform.translation += self.xform.matrix3.mul_vec3a(axis);
    }

    /// Rotate a transform from axis
    fn transform_rotate(&mut self, axis: Vec3) {
        self.spacing = Some(axis.length());
        let axis = axis.normalize();
        if axis.x != 0.0 {
            // project to XY plane, then rotate around Z axis
            let up = Vec2::new(0.0, 1.0);
            let proj = Vec2::new(axis.x, axis.y);
            let angle = up.angle_between(proj) * proj.length();
            self.xform.matrix3 *= Mat3A::from_rotation_z(angle);
        }
        if axis.z != 0.0 {
            // project to YZ plane, then rotate around X axis
            let up = Vec2::new(1.0, 0.0);
            let proj = Vec2::new(axis.y, axis.z);
            let angle = up.angle_between(proj) * proj.length();
            self.xform.matrix3 *= Mat3A::from_rotation_x(angle);
        }
    }

    /// Make a point for the given spoke
    fn make_point(&self, i: usize, spoke: &Spoke) -> (Degrees, Vec3) {
        let angle = self.angle(i);
        let order = Degrees::from(angle);
        let rot = Quat::from_rotation_y(angle);
        let distance = spoke.distance * self.scale_or_default();
        let pos = rot * Vec3::new(distance, 0.0, 0.0);
        let pos = self.xform.transform_point3(pos);
        (order, pos)
    }

    /// Make hub point
    pub(crate) fn make_hub(&self) -> (Degrees, Vec3) {
        let pos = self.xform.transform_point3(Vec3::ZERO);
        (Degrees(0), pos)
    }

    /// Make ring points
    pub(crate) fn make_points(&mut self, builder: &mut MeshBuilder) {
        let mut points = Vec::with_capacity(self.spokes.len());
        for (i, spoke) in self.spokes().enumerate() {
            let (order, pos) = self.make_point(i, spoke);
            match &spoke.label {
                None => {
                    let vid = builder.push_vtx(pos);
                    let point = Point {
                        order,
                        pt: Pt::Vertex(vid),
                    };
                    points.push(point);
                }
                Some(label) => {
                    let point = Point {
                        order,
                        pt: Pt::Branch(label.into(), pos),
                    };
                    points.push(point);
                }
            }
        }
        self.points = points;
    }

    /// Push point on ring
    pub(crate) fn push_pt(&mut self, order: Degrees, pt: Pt) {
        self.points.push(Point { order, pt });
    }

    /// Get iterator of points on ring
    pub(crate) fn points(&self) -> impl Iterator<Item = &Point> {
        self.points.iter()
    }

    /// Get points offset by a fixed angle (in descending order)
    pub(crate) fn points_offset(&self, hs_other: Degrees) -> Vec<Point> {
        let mut pts = Vec::new();
        for point in self.points() {
            let mut point = point.clone();
            // adjust degrees by half step of other ring
            point.order = point.order + hs_other;
            pts.push(point);
        }
        pts.sort_by(|a, b| b.order.partial_cmp(&a.order).unwrap());
        pts
    }
}

impl Branch {
    /// Push an edge
    pub fn push_edge(&mut self, v0: usize, v1: usize) {
        self.edges.push(Edge(v0, v1));
    }

    /// Push an internal point
    pub fn push_internal(&mut self, pos: Vec3) {
        self.internal.push(pos);
    }

    /// Calculate branch base axis
    fn axis(&self, builder: &MeshBuilder, center: Vec3) -> Vec3 {
        let mut norm = Vec3::ZERO;
        for edge in self.edges() {
            let v0 = builder.vertex(edge.0);
            let v1 = builder.vertex(edge.1);
            norm += (v0 - center).cross(v1 - center);
        }
        norm.normalize()
    }

    /// Get edge vertices sorted by common end-points
    fn edge_vids(self, edge: usize) -> impl ExactSizeIterator<Item = usize> {
        let mut edges = self.edges;
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
        edges.into_iter().map(|e| e.0)
    }

    /// Get center of internal points
    fn center(&self) -> Vec3 {
        let len = self.internal.len() as f32;
        self.internal.iter().fold(Vec3::ZERO, |a, b| a + *b) / len
    }

    /// Get an iterator of branch edges
    fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
    }

    /// Calculate edge angles for a branch base
    fn edge_angles(
        self,
        ring: &Ring,
        builder: &MeshBuilder,
    ) -> Vec<(Degrees, usize)> {
        let inverse = ring.xform.inverse();
        let zero_deg = Vec3::new(1.0, 0.0, 0.0);
        // Step 1: find "first" edge vertex (closest to 0 degrees)
        let mut edge = 0;
        let mut angle = f32::MAX;
        for (i, ed) in self.edges().enumerate() {
            let vid = ed.0;
            let pos = inverse.transform_point3(builder.vertex(vid));
            let pos = Vec3::new(pos.x, 0.0, pos.z);
            let ang = zero_deg.angle_between(pos);
            if ang < angle {
                angle = ang;
                edge = i;
            }
        }
        // Step 2: sort edge vertices by common end-points
        let vids = self.edge_vids(edge);
        // Step 3: make vec of (order, vid)
        let mut angle = 0.0;
        let mut ppos = zero_deg;
        let mut angles = Vec::with_capacity(vids.len());
        for vid in vids {
            let pos = inverse.transform_point3(builder.vertex(vid));
            let pos = Vec3::new(pos.x, 0.0, pos.z);
            let ang = ppos.angle_between(pos);
            angle += ang;
            let order = Degrees::from(angle);
            angles.push((order, vid));
            ppos = pos;
        }
        angles
    }
}
