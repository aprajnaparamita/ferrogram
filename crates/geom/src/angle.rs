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

    /// Creates an angle of `radians` radians.
    ///
    /// The conversion is [`f64::to_degrees`] (`180 / pi`, rounded in `f64`).
    pub fn from_radians(radians: f64) -> Self {
        Self {
            degrees: radians.to_degrees(),
        }
    }

    /// The angle in degrees.
    pub const fn degrees(self) -> f64 {
        self.degrees
    }

    /// The angle in radians.
    ///
    /// The conversion is [`f64::to_radians`] (`pi / 180`, rounded in `f64`).
    pub fn radians(self) -> f64 {
        self.degrees.to_radians()
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

    #[test]
    fn radians_convert_to_degrees() {
        assert_eq!(Angle::from_radians(core::f64::consts::PI).degrees(), 180.0);
        assert_eq!(Angle::from_degrees(180.0).radians(), core::f64::consts::PI);
    }

    #[test]
    fn radians_round_trip() {
        for &r in &[0.0, 1.0, -2.5, 0.25] {
            let back = Angle::from_radians(r).radians();
            assert!(
                (back - r).abs() <= 1.0e-12 * r.abs().max(1.0),
                "{back} != {r}"
            );
        }
    }
}
