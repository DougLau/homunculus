// plane.rs     Plane module
//
// Copyright (c) 2022  Douglas Lau
//
use glam::Vec3;

/// Geometric plane
///
/// Stored in Hessian Normal form
pub struct Plane {
    /// Normal vector
    pub normal: Vec3,

    /// Distance to origin
    pub origin_dist: f32,
}

impl Plane {
    /// Create a new plane (with normal and one point on the plane)
    pub fn new(normal: Vec3, point: Vec3) -> Self {
        let normal = normal.normalize();
        let origin_dist = -normal.dot(point);
        Plane {
            normal,
            origin_dist,
        }
    }

    /// Calculate distance to a point
    ///
    /// Negative value returned for negative half-space
    pub fn point_dist(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.origin_dist
    }

    /// Project a point onto the plane
    pub fn project_point(&self, point: Vec3) -> Vec3 {
        point - self.normal * self.point_dist(point)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn up_plane() {
        let p = Plane::new(Vec3::new(0.0, 1.0, 0.0), Vec3::ZERO);
        assert_eq!(p.point_dist(Vec3::new(0.0, 0.0, 0.0)), 0.0);
        assert_eq!(p.point_dist(Vec3::new(1.0, 1.0, 1.0)), 1.0);
        assert_eq!(p.point_dist(Vec3::new(0.0, 1.0, 1.0)), 1.0);
        assert_eq!(p.point_dist(Vec3::new(0.0, 1.0, 0.0)), 1.0);
        assert_eq!(p.point_dist(Vec3::new(0.0, -1.0, 0.0)), -1.0);
    }

    #[test]
    fn angled_plane() {
        let p = Plane::new(Vec3::new(1.0, 1.0, 1.0), Vec3::ZERO);
        assert_eq!(p.point_dist(Vec3::new(0.0, 0.0, 0.0)), 0.0);
        assert_eq!(p.point_dist(Vec3::new(1.0, 0.0, 0.0)), 0.57735026);
        assert_eq!(p.point_dist(Vec3::new(0.0, 1.0, 0.0)), 0.57735026);
        assert_eq!(p.point_dist(Vec3::new(0.0, 0.0, 1.0)), 0.57735026);
        assert_eq!(p.point_dist(Vec3::new(0.0, -1.0, 0.0)), -0.57735026);
    }
}
