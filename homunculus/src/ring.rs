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

    /// Ring points
    points: Vec<RingPoint>,

    /// Scale factor
    scale: Option<f32>,

    /// Edge smoothing
    smoothing: Option<Smoothing>,
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
    pub(crate) fn with_branch(id: usize, axis: Vec3, pts: usize) -> Self {
        Ring {
            id,
            axis: Some(axis),
            points: vec![RingPoint::default(); pts],
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
    pub(crate) fn transform_translate(&self, xform: &mut Affine3A) {
        xform.translation +=
            xform.matrix3.mul_vec3a(Vec3A::from(self.axis_or_default()));
    }

    /// Rotate a transform from axis
    pub(crate) fn transform_rotate(&mut self, xform: &mut Affine3A) {
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
