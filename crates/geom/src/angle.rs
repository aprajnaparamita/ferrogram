//! Angles. Canonical unit: radians (ADR 0011).

/// An angle. The canonical unit is radians, so an [`Angle`] stores radians.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Angle {
    radians: f64,
}

impl Angle {
    /// Creates an angle of `degrees` degrees.
    ///
    /// The conversion is [`f64::to_radians`] (`pi / 180`, rounded in `f64`).
    pub fn from_degrees(degrees: f64) -> Self {
        Self {
            radians: degrees.to_radians(),
        }
    }

    /// Creates an angle of `radians` radians.
    ///
    /// Radians are the canonical unit (ADR 0011), so the value is stored as
    /// written, with no conversion.
    pub const fn from_radians(radians: f64) -> Self {
        Self { radians }
    }

    /// The angle in degrees.
    ///
    /// The conversion is [`f64::to_degrees`] (`180 / pi`, rounded in `f64`).
    pub fn degrees(self) -> f64 {
        self.radians.to_degrees()
    }

    /// The angle in radians (the canonical unit).
    pub const fn radians(self) -> f64 {
        self.radians
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relative tolerance for `f64` conversions.
    const EPS: f64 = 1.0e-12;

    fn assert_approx(got: f64, want: f64) {
        let tol = EPS * want.abs().max(1.0);
        assert!(
            (got - want).abs() <= tol,
            "got {got}, want {want} (tol {tol})"
        );
    }

    #[test]
    fn radians_are_stored_as_written() {
        for &r in &[0.0, 1.0, -2.5, 0.25, core::f64::consts::PI] {
            assert_eq!(Angle::from_radians(r).radians(), r);
        }
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(Angle::default().radians(), 0.0);
    }

    #[test]
    fn degrees_convert_to_radians() {
        assert_approx(Angle::from_degrees(180.0).radians(), core::f64::consts::PI);
        assert_approx(
            Angle::from_degrees(90.0).radians(),
            core::f64::consts::FRAC_PI_2,
        );
    }

    #[test]
    fn radians_convert_to_degrees() {
        assert_approx(Angle::from_radians(core::f64::consts::PI).degrees(), 180.0);
    }

    #[test]
    fn degrees_round_trip() {
        for &d in &[0.0, 45.0, -2.5, 360.0] {
            assert_approx(Angle::from_degrees(d).degrees(), d);
        }
    }
}
