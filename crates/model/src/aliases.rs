//! Name aliases between `.vsdx` spellings and the ShapeSheet table.
//!
//! The model keeps cell and section names exactly as the `.vsdx` file spells
//! them (ADR 0011): the file is the source of truth and a write must emit the
//! same spelling back. The ShapeSheet table (`ui-spec/shapesheet.toml`,
//! ADR 0008) spells some of those names differently, so the rungs of the
//! lookup ([`crate::CellLookup`]) that read names *from the table* translate
//! the file spelling first with [`table_name`]. Local, master and style cells
//! keep matching by file spelling; only table lookups translate.
//!
//! The table lists the spellings the test corpus uses (task P2-06); each entry
//! maps one file spelling to the table spelling of the same cell or section.
//! A name with no entry is its own translation.

/// The ShapeSheet-table spelling of a cell or section name as a `.vsdx` file
/// spells it.
///
/// The identity function when there is no alias: a name the table spells the
/// same way (or does not know at all) comes back unchanged.
pub fn table_name(file_name: &str) -> &str {
    match file_name {
        // Cells.
        "BulletFontSize" => "BulletSize",
        "BulletStr" => "BulletString",
        "ColorTrans" => "Transparency",
        "DblUnderline" => "DoubleULine",
        "DefaultTabStop" => "DefaultTabstop",
        "DirX" => "DirX/A",
        "DirY" => "DirY/B",
        "FontScale" => "Scale",
        "HorzAlign" => "HAlign",
        "Letterspace" => "Spacing",
        "UICat" => "UICategory",
        "UICod" => "UICode",
        "UIFmt" => "UIFormat",
        "XCon" => "XBehavior",
        "XDyn" => "XDynamics",
        "YCon" => "YBehavior",
        "YDyn" => "YDynamics",
        // Sections.
        "Connection" => "Connection Points",
        "Control" => "Controls",
        "Field" => "Text Fields",
        "FillGradient" => "Fill Gradient",
        "Layer" => "Layers",
        "LineGradient" => "Line Gradient",
        "Property" => "Shape Data",
        "User" => "User-defined Cells",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_names_are_their_own_translation() {
        assert_eq!(table_name("PinX"), "PinX");
        assert_eq!(table_name(""), "");
        assert_eq!(table_name("Geometry"), "Geometry");
    }

    #[test]
    fn cell_spellings_translate_to_the_table() {
        for (file, table) in [
            ("BulletFontSize", "BulletSize"),
            ("BulletStr", "BulletString"),
            ("ColorTrans", "Transparency"),
            ("DblUnderline", "DoubleULine"),
            ("DefaultTabStop", "DefaultTabstop"),
            ("DirX", "DirX/A"),
            ("DirY", "DirY/B"),
            ("FontScale", "Scale"),
            ("HorzAlign", "HAlign"),
            ("Letterspace", "Spacing"),
            ("UICat", "UICategory"),
            ("UICod", "UICode"),
            ("UIFmt", "UIFormat"),
            ("XCon", "XBehavior"),
            ("XDyn", "XDynamics"),
            ("YCon", "YBehavior"),
            ("YDyn", "YDynamics"),
        ] {
            assert_eq!(table_name(file), table, "alias for {file}");
        }
    }

    #[test]
    fn section_spellings_translate_to_the_table() {
        for (file, table) in [
            ("Connection", "Connection Points"),
            ("Control", "Controls"),
            ("Field", "Text Fields"),
            ("FillGradient", "Fill Gradient"),
            ("Layer", "Layers"),
            ("LineGradient", "Line Gradient"),
            ("Property", "Shape Data"),
            ("User", "User-defined Cells"),
        ] {
            assert_eq!(table_name(file), table, "alias for {file}");
        }
    }

    #[test]
    fn translations_are_idempotent() {
        // Every table spelling is a fixed point: translating twice must not
        // move a name again.
        for file in [
            "BulletFontSize",
            "BulletStr",
            "ColorTrans",
            "Connection",
            "Control",
            "DblUnderline",
            "DefaultTabStop",
            "DirX",
            "DirY",
            "Field",
            "FillGradient",
            "FontScale",
            "HorzAlign",
            "Layer",
            "Letterspace",
            "LineGradient",
            "Property",
            "UICat",
            "UICod",
            "UIFmt",
            "User",
            "XCon",
            "XDyn",
            "YCon",
            "YDyn",
        ] {
            let once = table_name(file);
            assert_eq!(table_name(once), once, "alias for {file}");
        }
    }
}
