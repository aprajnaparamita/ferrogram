//! Test-only fixture trees for the `extract` dispatcher (P1-08 goal 3).
//!
//! [`reference_tree`] builds a small but *complete* `reference/`
//! fixture: every extractor of the [`crate::extract::run`] dispatcher
//! can consume it, so `cargo xtask extract --check` can be exercised
//! end to end without the real downloads. The two Fluent workbooks are
//! written as minimal `.xlsx` packages (a handful of ZIP entries with
//! inline XML) with the `zip` dev-dependency.
//!
//! Everything here is original test data (AGENTS.md section 4): no
//! Microsoft text, no corpus material.

use std::path::{Path, PathBuf};

/// The VisUICmds page fixture: a tiny `| Constant | Value |` table.
const VISUICMDS_MD: &str = "\
# VisUICmds enumeration (fixture)

| Constant | Value |
| --- | --- |
| **visCmdABarAutoHeight** | 1684 |
| **visCmdFileSave** | 1004 |
| **visCmdZoomIn** | 1216 |
";

/// One enumeration page fixture (`| Constant | Value | Description |`).
fn enum_md(rows: &[(&str, i64)]) -> String {
    let mut out = String::from("# Enumeration (fixture)\n\n| Constant | Value |\n| --- | --- |\n");
    for (name, value) in rows {
        out.push_str(&format!("| **{name}** | {value} |\n"));
    }
    out
}

/// The keyboard-shortcuts page fixture. Two actions map onto the
/// hand-written action table, one is unmapped and dropped.
const SHORTCUTS_HTML: &str = r#"<!DOCTYPE html>
<html><body>
<h2>Frequently used shortcuts</h2>
<table class="banded">
  <thead><tr><th><p>To do this</p></th><th><p>Press</p></th></tr></thead>
  <tbody>
    <tr><td><p>Undo a recent action</p></td><td><p>Ctrl+Z</p></td></tr>
    <tr><td><p>Group the selected shapes.</p></td><td><p>Ctrl+G or Ctrl+Shift+G</p></td></tr>
    <tr><td><p>Not a command we know.</p></td><td><p>F12</p></td></tr>
  </tbody>
</table>
</body></html>
"#;

/// The embedded XSD fixture (the same miniature the vsdx-schema tests
/// parse in-memory).
const FIXTURE_XSD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema">
    <xsd:annotation>
        <xsd:documentation>
            FIXTURE-ANNOTATION-MARKER: this line must never appear in the output.
            Copyright (c) Fixture Authors. All rights reserved.
        </xsd:documentation>
    </xsd:annotation>
    <xsd:element name="VisioDocument" type="VisioDocument_Type" />
    <xsd:complexType name="VisioDocument_Type">
        <xsd:sequence>
            <xsd:element name="Pages" type="Pages_Type" minOccurs="0" maxOccurs="1" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="Pages_Type">
        <xsd:sequence>
            <xsd:element name="Page" type="Page_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="Page_Type">
        <xsd:all>
            <xsd:element name="Cell" type="Cell_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:all>
        <xsd:attribute name="ID" type="xsd:unsignedInt" use="required" />
        <xsd:attribute name="Name" type="xsd:string" />
    </xsd:complexType>
    <xsd:complexType name="Cell_Type">
        <xsd:sequence>
            <xsd:element name="RefBy" type="RefBy_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:sequence>
        <xsd:attribute name="N" type="xsd:string" use="required" />
    </xsd:complexType>
    <xsd:complexType name="RefBy_Type">
        <xsd:attribute name="T" type="xsd:string" use="required" />
    </xsd:complexType>
</xsd:schema>
"#;

/// A doc page for one schema type.
const FIXTURE_PAGE_TYPE: &str = "\
# Cell_Type complexType (Visio XML)\n\n## Type information\n\n\
||Value |\n|:-----|:-----|\n|**Extension base** <br/> |xsd:anyType  <br/> |\n";

/// A doc page for one element occurrence.
const FIXTURE_PAGE_ELEMENT: &str = "\
# RefBy element (Cell_Type complexType) (Visio XML)\n\n## Element information\n\n\
||Value |\n|:-----|:-----|\n|**Element type** <br/> |[RefBy_Type](refby_type-complextypevisio-xml.md) <br/> |\n";

/// A ShapeSheet cell page fixture. The summary drives type inference
/// ("coordinate" -> `Number`/`Inches`).
fn cell_page(cell: &str, section: &str, cell_const: &str) -> String {
    format!(
        "\
# {cell} Cell ({section} Section)\n\nThe x coordinate of the shape's pin, in inches.\n\n\
||Value |\n|:-----|:-----|\n\
|**Cell name:** <br/> |{cell} <br/> |\n\
|**Section index:** <br/> |**visSectionFirstComponent** <br/> |\n\
|**Cell index:** <br/> |**{cell_const}** <br/> |\n\
|**Row index:** <br/> |**visRowXForm1D** <br/> |\n"
    )
}

/// A ShapeSheet section page fixture.
fn section_page(section: &str) -> String {
    format!(
        "\
# {section} Section\n\nThe {section} section of a shape.\n\n\
||Value |\n|:-----|:-----|\n|**Section index:** <br/> |**visSectionFirstComponent** <br/> |\n"
    )
}

/// The Fluent workbook fixture rows (the same miniature the fluent
/// tests feed to `assemble_layout`), header included. `extra` rows are
/// appended to the 365 workbook only.
fn workbook_rows<'a>(extra: &[Vec<&'a str>]) -> Vec<Vec<&'a str>> {
    let mut rows: Vec<Vec<&str>> = vec![
        vec![
            "Control Name",
            "Control Type",
            "Tab Set",
            "Tab",
            "Group/Context Menu Name",
            "Parent Control",
            "Policy ID",
        ],
        vec!["TabHome", "tab", "None (Core Tab)", "", "", "", "1"],
        vec![
            "GroupClipboard",
            "group",
            "None (Core Tab)",
            "TabHome",
            "",
            "",
            "2",
        ],
        vec![
            "FileSave",
            "button",
            "None (Core Tab)",
            "TabHome",
            "GroupClipboard",
            "",
            "3",
        ],
        vec![
            "ZoomIn",
            "button",
            "None (Core Tab)",
            "TabHome",
            "GroupClipboard",
            "",
            "4",
        ],
        vec!["TabInfo", "tab", "None (Backstage View)", "", "", "", "5"],
        vec![
            "FileSaveBackstage",
            "button",
            "None (Backstage View)",
            "TabInfo",
            "",
            "",
            "6",
        ],
        vec![
            "FileSave",
            "button",
            "None (Quick Access Toolbar)",
            "Quick Access Toolbar",
            "",
            "",
            "7",
        ],
        vec![
            "ContextMenuShape",
            "contextMenu",
            "None (Context Menu)",
            "None (Context Menu)",
            "",
            "",
            "8",
        ],
    ];
    rows.extend(extra.iter().map(|r| r.iter().map(|s| *s).collect()));
    rows
}

/// Builds a complete fixture `reference/` tree in a unique temp
/// directory. Every call gets its own directory: tests run in
/// parallel and must not rewrite each other's fixture files.
pub fn reference_tree(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("ferrogram-p108-{}-{name}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write(&root, "learn/visuicmds/visuicmds.md", VISUICMDS_MD);
    write(
        &root,
        "learn/enums/visio.vissectionindices.md",
        &enum_md(&[("visSectionFirstComponent", 0), ("visSectionObject", 1)]),
    );
    write(
        &root,
        "learn/enums/visio.visrowindices.md",
        &enum_md(&[("visRowXForm1D", 0), ("visRowXFormOut", 1)]),
    );
    write(
        &root,
        "learn/enums/visio.viscellindices.md",
        &enum_md(&[
            ("visCellXFormPinX", 0),
            ("visCellXFormPinY", 1),
            ("visCellXFormHeight", 2),
        ]),
    );
    write(
        &root,
        "learn/enums/visio.visunitcodes.md",
        &enum_md(&[("visNumber", 0), ("visInches", 1)]),
    );
    write(
        &root,
        "support/shortcuts/keyboard-shortcuts-visio.html",
        SHORTCUTS_HTML,
    );

    let visio = "ms-vsdx/office-developer-client-docs/docs/visio";
    write(
        &root,
        &format!("{visio}/schema-mapvisio-xml.md"),
        &format!("# Schema map (fixture)\n\n```XML\n{FIXTURE_XSD}\n```\n"),
    );
    write(
        &root,
        &format!("{visio}/cell_type-complextypevisio-xml.md"),
        FIXTURE_PAGE_TYPE,
    );
    write(
        &root,
        &format!("{visio}/refby-element-cell_type-complextypevisio-xml.md"),
        FIXTURE_PAGE_ELEMENT,
    );
    write(
        &root,
        &format!("{visio}/pinx-cell-shape-transform-section.md"),
        &cell_page("PinX", "Shape Transform", "visCellXFormPinX"),
    );
    write(
        &root,
        &format!("{visio}/height-cell-shape-transform-section.md"),
        &cell_page("Height", "Shape Transform", "visCellXFormHeight"),
    );
    write(
        &root,
        &format!("{visio}/shape-transform-section.md"),
        &section_page("Shape Transform"),
    );

    let sheets = [
        (
            "fluent-ui-ids/Office 2016/visiocontrols.xlsx",
            workbook_rows(&[]),
        ),
        (
            "fluent-ui-ids/Microsoft 365/Current Channel/visiocontrols.xlsx",
            workbook_rows(&[vec![
                "RedoButton",
                "button",
                "None (Core Tab)",
                "TabHome",
                "GroupClipboard",
                "",
                "9",
            ]]),
        ),
    ];
    for (rel, rows) in sheets {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("fixture dir");
        }
        let xml = workbook_xml(&rows);
        std::fs::write(&path, xml).expect("fixture xlsx");
    }
    root
}

/// Writes one fixture file (parents included).
fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture dir");
    }
    std::fs::write(path, text).expect("fixture file");
}

/// Builds a minimal `.xlsx` package (ZIP, stored entries) holding one
/// worksheet with the given rows and an inline shared-string table.
pub fn workbook_xml(rows: &[Vec<&str>]) -> Vec<u8> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut shared: Vec<&str> = Vec::new();
    let mut sheet = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
         <sheetData>",
    );
    for (ri, row) in rows.iter().enumerate() {
        sheet.push_str(&format!("<row r=\"{}\">", ri + 1));
        for (ci, value) in row.iter().enumerate() {
            let idx = match shared.iter().position(|s| s == value) {
                Some(i) => i,
                None => {
                    shared.push(value);
                    shared.len() - 1
                }
            };
            sheet.push_str(&format!(
                "<c r=\"{}{}\" t=\"s\"><v>{idx}</v></c>",
                column_name(ci),
                ri + 1
            ));
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");

    let mut sst = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"0\" uniqueCount=\"0\">",
    );
    for s in &shared {
        sst.push_str(&format!("<si><t>{s}</t></si>"));
    }
    sst.push_str("</sst>");

    let files: [(&str, String); 6] = [
        (
            "[Content_Types].xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
             <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
             <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
             <Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
             <Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\
             <Override PartName=\"/xl/sharedStrings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml\"/>\
             </Types>"
                .to_owned(),
        ),
        (
            "_rels/.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
             <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>\
             </Relationships>"
                .to_owned(),
        ),
        (
            "xl/workbook.xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
             xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
             <sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"
                .to_owned(),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
             <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/>\
             <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings\" Target=\"sharedStrings.xml\"/>\
             </Relationships>"
                .to_owned(),
        ),
        ("xl/worksheets/sheet1.xml", sheet),
        ("xl/sharedStrings.xml", sst),
    ];

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, body) in files {
            zip.start_file(name, options).expect("zip entry");
            zip.write_all(body.as_bytes()).expect("zip bytes");
        }
        zip.finish().expect("zip finish");
    }
    cursor.into_inner()
}

/// Spreadsheet column name for a 0-based column index (`A`, `B`, ...).
fn column_name(mut index: usize) -> String {
    let mut out = String::new();
    loop {
        out.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    out
}
