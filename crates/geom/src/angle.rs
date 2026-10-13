//! Angles. Canonical unit: degrees (ADR 0011).

/// An angle. The canonical unit is degrees, so an [`Angle`] stores degrees.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Angle {
    degrees: f64,
}

impl Angle {
    /// Creates an angle of `degrees` degrees.
    pub const fn from_degrees(degrees: f64) -> Self {
        Self { degrees }
    }

    /// The angle in degrees.
    pub const fn degrees(self) -> f64 {
        self.degrees
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees_round_trip() {
        let a = Angle::from_degrees(45.0);
        assert_eq!(a.degrees(), 45.0);
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(Angle::default().degrees(), 0.0);
    }
}
