//! Mapping theme parts and the document colour table (task P3-02,
//! ADR 0019).
//!
//! A theme part (`/visio/theme/theme{N}.xml`) is a DrawingML theme: a
//! colour scheme (`a:clrScheme`), a font scheme (`a:fontScheme`) and an
//! effects scheme (`a:fmtScheme`). Phase 3 maps the colours and the fonts;
//! the effects and everything else stay preserved (the part itself is
//! never rewritten: it is `preserved-only`, `docs/vsdx-inventory.md`). A
//! colour scheme that is missing one of its slots cannot be typed and
//! degrades to no scheme (`Theme::colors` is `None`).
//!
//! The document colour table (`Colors`/`ColorEntry`, indices from 24)
//! maps to `Document::color_table`; the `Colors` element itself stays in
//! the document's preserved content as before (ADR 0012).

use ferrogram_model::{Color, ColorTableEntry, Theme, ThemeColors, ThemeFonts, XmlNode};

use crate::Error;
use crate::read::node::{attribute, elements, is_namespace_declaration};

/// Maps the root `a:theme` element of a theme part into a [`Theme`].
pub(crate) fn map_theme(root: &XmlNode) -> Result<Theme, Error> {
    let mut theme = Theme::new(attribute(root, "name").unwrap_or_default());
    for attribute in root
        .attributes
        .iter()
        .filter(|attribute| !is_namespace_declaration(attribute))
    {
        if attribute.name.local != "name" {
            theme.preserved.attributes.push(attribute.clone());
        }
    }
    for element in elements(root) {
        match element.name.local.as_str() {
            "themeElements" => map_theme_elements(element, &mut theme)?,
            _ => theme.preserved.children.push(element.clone()),
        }
    }
    Ok(theme)
}

/// Maps the `a:themeElements` children: the colour and font schemes, with
/// everything else kept verbatim.
fn map_theme_elements(node: &XmlNode, theme: &mut Theme) -> Result<(), Error> {
    for element in elements(node) {
        match element.name.local.as_str() {
            "clrScheme" => theme.colors = map_color_scheme(element),
            "fontScheme" => theme.fonts = Some(map_font_scheme(element)),
            _ => theme.preserved.children.push(element.clone()),
        }
    }
    Ok(())
}

/// Maps one `a:clrScheme` element, or `None` when a slot is missing or its
/// colour does not parse (a partial scheme cannot be typed).
fn map_color_scheme(node: &XmlNode) -> Option<ThemeColors> {
    let accents = [
        slot(node, "accent1")?,
        slot(node, "accent2")?,
        slot(node, "accent3")?,
        slot(node, "accent4")?,
        slot(node, "accent5")?,
        slot(node, "accent6")?,
    ];
    Some(ThemeColors {
        name: attribute(node, "name").unwrap_or_default().to_owned(),
        dk1: slot(node, "dk1")?,
        lt1: slot(node, "lt1")?,
        dk2: slot(node, "dk2")?,
        lt2: slot(node, "lt2")?,
        accents,
        hlink: slot(node, "hlink")?,
        fol_hlink: slot(node, "folHlink")?,
    })
}

/// The colour of one scheme slot: `a:srgbClr` or `a:sysClr` (whose
/// `lastClr` carries the resolved RGB), or `None`.
fn slot(node: &XmlNode, name: &str) -> Option<Color> {
    let slot = elements(node).find(|element| element.name.local == name)?;
    for child in elements(slot) {
        match child.name.local.as_str() {
            "srgbClr" => {
                if let Some(color) = attribute(child, "val").and_then(hex_color) {
                    return Some(color);
                }
            }
            "sysClr" => {
                if let Some(color) = attribute(child, "lastClr").and_then(hex_color) {
                    return Some(color);
                }
            }
            _ => {}
        }
    }
    None
}

/// Maps one `a:fontScheme` element (the major and minor latin, ea and cs
/// fonts; the script fonts stay preserved with the part).
fn map_font_scheme(node: &XmlNode) -> ThemeFonts {
    ThemeFonts {
        name: attribute(node, "name").unwrap_or_default().to_owned(),
        major_latin: script_font(node, "majorFont", "latin"),
        minor_latin: script_font(node, "minorFont", "latin"),
        major_ea: script_font(node, "majorFont", "ea"),
        minor_ea: script_font(node, "minorFont", "ea"),
        major_cs: script_font(node, "majorFont", "cs"),
        minor_cs: script_font(node, "minorFont", "cs"),
    }
}

/// The `typeface` of one script font of one `majorFont`/`minorFont` block,
/// or the empty string.
fn script_font(node: &XmlNode, block: &str, script: &str) -> String {
    elements(node)
        .find(|element| element.name.local == block)
        .and_then(|block| {
            elements(block)
                .find(|element| element.name.local == script)
                .and_then(|script| attribute(script, "typeface"))
        })
        .unwrap_or_default()
        .to_owned()
}

/// The colour of a `#RGB`/`#RRGGBB` literal or a bare `RRGGBB` hex value
/// (the spelling `a:srgbClr/@val` uses).
fn hex_color(text: &str) -> Option<Color> {
    if text.starts_with('#') {
        return crate::value_parse::parse_color(text);
    }
    let mut with_hash = String::with_capacity(text.len() + 1);
    with_hash.push('#');
    with_hash.push_str(text);
    crate::value_parse::parse_color(&with_hash)
}

/// Maps the `ColorEntry` children of a `Colors` element into `table`
/// (indices from 24 in the files we have seen). An entry with no `IX` or
/// no `RGB` cannot be named and is skipped (it stays in the preserved
/// element); a present value that does not parse is
/// [`Error::InvalidPackage`] naming the part.
pub(crate) fn map_colors(
    node: &XmlNode,
    table: &mut Vec<ColorTableEntry>,
    part: &str,
) -> Result<(), Error> {
    for element in elements(node) {
        if element.name.local != "ColorEntry" {
            continue;
        }
        let (Some(index), Some(rgb)) = (attribute(element, "IX"), attribute(element, "RGB")) else {
            continue;
        };
        let index = index.trim().parse::<i32>().map_err(|_| {
            Error::InvalidPackage(format!("{part}: attribute `IX`: not an integer"))
        })?;
        let color = hex_color(rgb).ok_or_else(|| {
            Error::InvalidPackage(format!("{part}: attribute `RGB`: not a color"))
        })?;
        table.push(ColorTableEntry { index, color });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ferrogram_model::{QName, XmlAttribute, XmlChild};

    use super::*;

    /// Builds a qualified name from `prefix:local` or `local`.
    fn qname(name: &str) -> QName {
        match name.split_once(':') {
            Some((prefix, local)) => QName {
                prefix: Some(prefix.to_owned()),
                uri: None,
                local: local.to_owned(),
            },
            None => QName {
                prefix: None,
                uri: None,
                local: name.to_owned(),
            },
        }
    }

    /// An element with attributes and children.
    fn element(name: &str, attrs: &[(&str, &str)], children: Vec<XmlChild>) -> XmlNode {
        XmlNode {
            name: qname(name),
            attributes: attrs
                .iter()
                .map(|(local, value)| XmlAttribute {
                    name: qname(local),
                    value: (*value).to_owned(),
                })
                .collect(),
            self_closing: children.is_empty(),
            children,
        }
    }

    /// An element child.
    fn child(node: XmlNode) -> XmlChild {
        XmlChild::Element(node)
    }

    /// An `a:clrScheme` with all twelve slots filled.
    fn color_scheme() -> XmlNode {
        let slot = |name: &str, value: &str| {
            child(element(
                name,
                &[],
                vec![child(element("srgbClr", &[("val", value)], Vec::new()))],
            ))
        };
        element(
            "clrScheme",
            &[("name", "Office")],
            vec![
                slot("dk1", "000000"),
                slot("lt1", "FEFFFF"),
                slot("dk2", "44546A"),
                slot("lt2", "E7E6E6"),
                slot("accent1", "5B9BD5"),
                slot("accent2", "ED7D31"),
                slot("accent3", "A5A5A5"),
                slot("accent4", "FFC000"),
                slot("accent5", "4672C4"),
                slot("accent6", "70AD47"),
                slot("hlink", "0563C1"),
                slot("folHlink", "954F72"),
            ],
        )
    }

    /// An `a:fontScheme` with major and minor fonts.
    fn font_scheme() -> XmlNode {
        let block = |name: &str, latin: &str| {
            child(element(
                name,
                &[],
                vec![
                    child(element("latin", &[("typeface", latin)], Vec::new())),
                    child(element("ea", &[("typeface", "")], Vec::new())),
                    child(element("cs", &[("typeface", "CS Font")], Vec::new())),
                ],
            ))
        };
        element(
            "fontScheme",
            &[("name", "Office")],
            vec![
                block("majorFont", "Calibri Light"),
                block("minorFont", "Calibri"),
            ],
        )
    }

    #[test]
    fn a_theme_maps_its_schemes_and_keeps_the_rest() {
        let root = element(
            "a:theme",
            &[("name", "Office Theme"), ("extra", "kept")],
            vec![
                child(element(
                    "a:themeElements",
                    &[],
                    vec![
                        child(color_scheme()),
                        child(font_scheme()),
                        child(element("a:fmtScheme", &[("name", "Office")], Vec::new())),
                    ],
                )),
                child(element("a:objectDefaults", &[], Vec::new())),
            ],
        );
        let theme = map_theme(&root).expect("maps");
        assert_eq!(theme.name, "Office Theme");
        let colors = theme.colors.expect("colors");
        assert_eq!(colors.name, "Office");
        assert_eq!(colors.dk1, Color::new(0, 0, 0));
        assert_eq!(colors.lt1, Color::new(254, 255, 255));
        assert_eq!(colors.dk2, Color::new(68, 84, 106));
        assert_eq!(colors.accents[0], Color::new(91, 155, 213));
        assert_eq!(colors.accents[5], Color::new(112, 173, 71));
        assert_eq!(colors.hlink, Color::new(5, 99, 193));
        assert_eq!(colors.fol_hlink, Color::new(149, 79, 114));
        let fonts = theme.fonts.expect("fonts");
        assert_eq!(fonts.name, "Office");
        assert_eq!(fonts.major_latin, "Calibri Light");
        assert_eq!(fonts.minor_latin, "Calibri");
        assert_eq!(fonts.major_ea, "");
        assert_eq!(fonts.minor_cs, "CS Font");
        // The effects scheme and the unmapped root content stay preserved.
        assert_eq!(theme.preserved.children.len(), 2);
        assert_eq!(theme.preserved.attributes.len(), 1);
        assert_eq!(theme.preserved.attributes[0].value, "kept");
    }

    #[test]
    fn a_partial_color_scheme_degrades_to_none() {
        let mut scheme = color_scheme();
        scheme.children.pop(); // drop folHlink
        let theme = map_theme(&element("a:theme", &[], vec![child(scheme)])).expect("maps");
        assert_eq!(theme.colors, None);
    }

    #[test]
    fn system_colors_read_their_last_resolved_value() {
        let root = element(
            "a:theme",
            &[],
            vec![child(element(
                "clrScheme",
                &[],
                vec![child(element(
                    "dk1",
                    &[],
                    vec![child(element(
                        "sysClr",
                        &[("val", "windowText"), ("lastClr", "000000")],
                        Vec::new(),
                    ))],
                ))],
            ))],
        );
        let theme = map_theme(&root).expect("maps");
        assert_eq!(theme.colors, None); // the other slots are missing
        let scheme = elements(&root)
            .find(|e| e.name.local == "clrScheme")
            .expect("scheme");
        assert_eq!(slot(scheme, "dk1"), Some(Color::new(0, 0, 0)));
        assert_eq!(slot(scheme, "missing"), None);
    }

    #[test]
    fn color_entries_map_by_index() {
        let node = element(
            "Colors",
            &[],
            vec![
                child(element(
                    "ColorEntry",
                    &[("IX", "24"), ("RGB", "#7F7F7F")],
                    Vec::new(),
                )),
                child(element(
                    "ColorEntry",
                    &[("IX", "25"), ("RGB", "FF0000")],
                    Vec::new(),
                )),
                // No IX: skipped (kept preserved with the element).
                child(element("ColorEntry", &[("RGB", "#000000")], Vec::new())),
            ],
        );
        let mut table = Vec::new();
        map_colors(&node, &mut table, "part").expect("maps");
        assert_eq!(
            table,
            vec![
                ColorTableEntry {
                    index: 24,
                    color: Color::new(127, 127, 127)
                },
                ColorTableEntry {
                    index: 25,
                    color: Color::new(255, 0, 0)
                },
            ]
        );
    }

    #[test]
    fn a_bad_color_entry_is_an_error_naming_the_part() {
        let node = element(
            "Colors",
            &[],
            vec![child(element(
                "ColorEntry",
                &[("IX", "x"), ("RGB", "#000000")],
                Vec::new(),
            ))],
        );
        let error = map_colors(&node, &mut Vec::new(), "/visio/document.xml").expect_err("fails");
        assert!(error.to_string().contains("/visio/document.xml"), "{error}");
        let node = element(
            "Colors",
            &[],
            vec![child(element(
                "ColorEntry",
                &[("IX", "24"), ("RGB", "#zz")],
                Vec::new(),
            ))],
        );
        let error = map_colors(&node, &mut Vec::new(), "part").expect_err("fails");
        assert!(error.to_string().contains("RGB"), "{error}");
    }

    #[test]
    fn hex_colors_accept_both_spellings() {
        assert_eq!(hex_color("#7F7F7F"), Some(Color::new(127, 127, 127)));
        assert_eq!(hex_color("ff0000"), Some(Color::new(255, 0, 0)));
        assert_eq!(hex_color("#abc"), Some(Color::new(170, 187, 204)));
        assert_eq!(hex_color("nope"), None);
        assert_eq!(hex_color(""), None);
    }
}
