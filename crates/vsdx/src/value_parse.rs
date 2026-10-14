//! Value parsing and classification shared by the reader and the writer
//! (task P2-14, review finding 3).
//!
//! [`crate::read::value`] parses the `V` attribute of a `Cell` and
//! [`crate::write::value`] decides whether an existing spelling already
//! says the same thing; both classify an undeclared value by its shape and
//! parse color literals, so those rules live here once. No behavior change.

use ferrogram_model::Color;

/// What a value of unknown declared kind looks like.
pub(crate) enum Shaped {
    /// A number literal.
    Number(f64),
    /// A `#RGB` or `#RRGGBB` literal.
    Color(Color),
    /// Anything else, kept as written.
    Str(String),
}

/// Classifies a value with no declared kind by its own shape: a number
/// literal first (whatever `f64` parses, `inf` and `NaN` included), then a
/// color literal, then a plain string.
pub(crate) fn shape(v: &str) -> Shaped {
    if let Ok(number) = v.trim().parse::<f64>() {
        return Shaped::Number(number);
    }
    if let Some(color) = parse_color(v) {
        return Shaped::Color(color);
    }
    Shaped::Str(v.to_owned())
}

/// The color of a `#RGB` or `#RRGGBB` literal (any case), or `None`.
pub(crate) fn parse_color(text: &str) -> Option<Color> {
    let digits = text.strip_prefix('#')?.as_bytes();
    match digits {
        [r1, r2, g1, g2, b1, b2] => {
            Some(Color::new(hex(*r1, *r2)?, hex(*g1, *g2)?, hex(*b1, *b2)?))
        }
        [r, g, b] => Some(Color::new(hex(*r, *r)?, hex(*g, *g)?, hex(*b, *b)?)),
        _ => None,
    }
}

/// The value of one hexadecimal digit pair.
fn hex(high: u8, low: u8) -> Option<u8> {
    Some(digit(high)? * 16 + digit(low)?)
}

/// The value of one hexadecimal digit.
fn digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_parse_like_the_reader() {
        assert_eq!(parse_color("#4d4d4d"), Some(Color::new(77, 77, 77)));
        assert_eq!(parse_color("#abc"), Some(Color::new(170, 187, 204)));
        assert_eq!(parse_color("#gggggg"), None);
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("4d4d4d"), None);
    }

    #[test]
    fn color_literals_have_exactly_three_or_six_digits() {
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("#1234567"), None);
        assert_eq!(parse_color("123456"), None);
        assert_eq!(parse_color("#"), None);
        assert_eq!(parse_color("#zzzzzz"), None);
        assert_eq!(parse_color("#000000"), Some(Color::new(0, 0, 0)));
    }

    #[test]
    fn shapes_classify_numbers_colors_and_strings() {
        assert!(matches!(shape("2.5"), Shaped::Number(n) if n == 2.5));
        assert!(matches!(shape(" inf"), Shaped::Number(n) if n.is_infinite()));
        assert!(matches!(shape("#4d4d4d"), Shaped::Color(_)));
        assert!(matches!(shape("Themed"), Shaped::Str(text) if text == "Themed"));
    }
}
