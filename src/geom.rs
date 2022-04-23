use core::ops::{AddAssign, Mul, Sub};
use serde_derive::{Deserialize, Serialize};

/// Vector of 3 components
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Vec3(pub [f32; 3]);

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.0[0] += rhs.x();
        self.0[1] += rhs.y();
        self.0[2] += rhs.z();
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self([self.x() - rhs.x(), self.y() - rhs.y(), self.z() - rhs.z()])
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self([self.x() * rhs, self.y() * rhs, self.z() * rhs])
    }
}

impl Vec3 {
    /// Get the X component
    pub const fn x(self) -> f32 {
        self.0[0]
    }

    /// Get the Y component
    pub const fn y(self) -> f32 {
        self.0[1]
    }

    /// Get the Z component
    pub const fn z(self) -> f32 {
        self.0[2]
    }

    /// Get minimum of two `Vec3`s
    pub fn min(self, rhs: Self) -> Self {
        let x = self.x().min(rhs.x());
        let y = self.y().min(rhs.y());
        let z = self.z().min(rhs.z());
        Vec3([x, y, z])
    }

    /// Get maximum of two `Vec3`s
    pub fn max(self, rhs: Self) -> Self {
        let x = self.x().max(rhs.x());
        let y = self.y().max(rhs.y());
        let z = self.z().max(rhs.z());
        Vec3([x, y, z])
    }

    /// Calculate vector magnitude
    pub fn magnitude(self) -> f32 {
        (self.x() * self.x() + self.y() * self.y() + self.z() * self.z()).sqrt()
    }

    /// Calculate cross product of two `Vec3`s
    #[must_use]
    pub fn cross(self, rhs: Vec3) -> Vec3 {
        let x = self.y() * rhs.z() - self.z() * rhs.y();
        let y = self.z() * rhs.x() - self.x() * rhs.z();
        let z = self.x() * rhs.y() - self.y() * rhs.x();
        Vec3([x, y, z])
    }

    /// Calculate doc product of two `Vec3`s
    pub fn dot(self, rhs: Vec3) -> f32 {
        self.x() * rhs.x() + self.y() * rhs.y() + self.z() * rhs.z()
    }

    /// Calculate normalized vector
    #[must_use]
    pub fn normalize(self) -> Vec3 {
        let mag = self.magnitude();
        if mag != 0.0 {
            self * (1.0 / mag)
        } else {
            self
        }
    }

    /// Calculate angle between two `Vec3`s
    pub fn angle(self, rhs: Vec3) -> f32 {
        let mag = self.magnitude() * rhs.magnitude();
        if mag != 0.0 { self.dot(rhs) / mag } else { 0.0 }
            .min(1.0)
            .max(-1.0)
            .acos()
    }
}
