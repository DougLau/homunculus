// ring.rs     Ring module
//
// Copyright (c) 2022-2023  Douglas Lau
//
use crate::mesh::Smoothing;
use glam::{Affine3A, Mat3A, Vec2, Vec3, Vec3A};
use std::f32::consts::PI;
use std::ops::Add;

/// Angular degrees
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub(crate) struct Degrees(pub u16);

/// Ring point
///
/// A point on a [Ring] with distance from the central axis.  An optional label
/// can be declared for a [branch] point.
///
/// [branch]: struct.Husk.html#method.branch
/// [ring]: struct.Ring.html
#[derive(Clone, Debug)]
pub struct RingPoint {
    /// Distance from axis
    pub distance: f32,

    /// Label for branch points
    pub label: Option<String>,
}

/// Empty ring points
const EMPTY_RING: &[RingPoint] = &[RingPoint {
    distance: 0.0,
    label: None,
}];

/// Ring around a [Husk]
///
/// Points are distributed evenly around the ring.
///
/// [husk]: struct.Husk.html
#[derive(Clone, Debug, Default)]
pub struct Ring {
    /// Ring ID
    pub(crate) id: usize,

    /// Axis vector
    pub(crate) axis: Option<Vec3>,

    /// Global transform
    pub(crate) xform: Affine3A,

    /// Ring points
    points: Vec<RingPoint>,

    /// Scale factor
    scale: Option<f32>,

    /// Edge smoothing
    smoothing: Option<Smoothing>,
}

/// Edge between two vertices
#[derive(Clone, Copy, Debug)]
pub struct Edge(pub usize, pub usize);

/// Branch data
#[derive(Debug, Default)]
pub struct Branch {
    /// Internal connection vertices (non-edge)
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

impl Default for RingPoint {
    fn default() -> Self {
        RingPoint::from(1.0)
    }
}

impl From<f32> for RingPoint {
    fn from(distance: f32) -> Self {
        RingPoint {
            distance,
            label: None,
        }
    }
}

impl From<&str> for RingPoint {
    fn from(label: &str) -> Self {
        RingPoint {
            distance: 1.0,
            label: Some(label.to_string()),
        }
    }
}

impl From<(f32, &str)> for RingPoint {
    fn from(val: (f32, &str)) -> Self {
        RingPoint {
            distance: val.0,
            label: Some(val.1.to_string()),
        }
    }
}

impl Ring {
    /// Create a new branch ring
    pub(crate) fn with_branch(branch: &Branch, axis: Vec3) -> Self {
        let center = branch.center();
        let xform = Affine3A::from_translation(center);
        let count = branch.edge_vertex_count();
        Ring {
            id: 0,
            axis: Some(axis),
            xform,
            points: vec![RingPoint::default(); count],
            scale: None,
            smoothing: None,
        }
    }

    /// Create a ring updated with another ring
    pub(crate) fn with_ring(&self, ring: &Self) -> Self {
        let points = if ring.points.is_empty() {
            self.points.clone()
        } else {
            ring.points.clone()
        };
        Ring {
            id: self.id,
            axis: ring.axis.or(self.axis),
            xform: self.xform,
            points,
            scale: ring.scale.or(self.scale),
            smoothing: ring.smoothing.or(self.smoothing),
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

    /// Set smooth edges
    pub fn smooth(mut self) -> Self {
        self.smoothing = Some(Smoothing::Smooth);
        self
    }

    /// Set flat edges
    pub fn flat(mut self) -> Self {
        self.smoothing = Some(Smoothing::Flat);
        self
    }

    /// Get the ring axis (or default value)
    pub(crate) fn axis_or_default(&self) -> Vec3 {
        self.axis.unwrap_or_else(|| Vec3::new(0.0, 1.0, 0.0))
    }

    /// Get the ring scale (or default value)
    pub(crate) fn scale_or_default(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }

    /// Get the edge smoothing (or default value)
    pub(crate) fn smoothing_or_default(&self) -> Smoothing {
        self.smoothing.unwrap_or(Smoothing::Flat)
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
    ///
    /// # Panics
    ///
    /// This function will panic if the distance is negative, infinite, or NaN.
    ///
    /// [branch]: struct.Husk.html#method.branch
    pub fn point<P: Into<RingPoint>>(mut self, pt: P) -> Self {
        let pt = pt.into();
        assert!(pt.distance.is_finite());
        assert!(pt.distance.is_sign_positive());
        self.points.push(pt);
        self
    }

    /// Get an iterator of ring points
    pub fn points(&self) -> impl Iterator<Item = &RingPoint> {
        if self.points.is_empty() {
            EMPTY_RING.iter()
        } else {
            self.points[..].iter()
        }
    }

    /// Get half step in degrees
    pub(crate) fn half_step(&self) -> Degrees {
        let deg = 180 / self.points.len();
        Degrees(deg as u16)
    }

    /// Calculate the angle of a point
    pub(crate) fn angle(&self, i: usize) -> f32 {
        2.0 * PI * i as f32 / self.points.len() as f32
    }

    /// Translate a transform from axis
    pub(crate) fn transform_translate(&mut self) {
        self.xform.translation += self
            .xform
            .matrix3
            .mul_vec3a(Vec3A::from(self.axis_or_default()));
    }

    /// Rotate a transform from axis
    pub(crate) fn transform_rotate(&mut self) {
        if let Some(axis) = self.axis {
            let length = axis.length();
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
            // adjust axis after rotation applied
            self.axis = Some(Vec3::new(0.0, length, 0.0));
        }
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

    /// Get edge vertices sorted by common end-points
    pub fn edge_vids(&self, edge: usize) -> Vec<usize> {
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
    pub fn center(&self) -> Vec3 {
        let len = self.internal.len() as f32;
        self.internal.iter().fold(Vec3::ZERO, |a, b| a + *b) / len
    }

    /// Get an iterator of branch edges
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
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
