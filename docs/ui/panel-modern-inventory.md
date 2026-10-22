# Panel Modern layout inventory

Generated from `ui-spec/layouts/panel-modern.yaml`; do not edit by hand.
Slots are `[column,row]` inside the group; L/M/S is large, medium (icon and label) or small
(icon only); 🖼 marks an item that draws an icon; *new* marks a command id that Phase 7 adds.

## Quick Access Toolbar

- Save → `file.save` 🖼
- Undo (split) → `edit.undo` 🖼 *(label changes)*
- Redo → `edit.redo.or.repeat` *(label changes)*
- Customize Quick Access Toolbar (dropdown) → `view.qat.customize` *new*

## File view

- Info (page)
- New (page) → `file.new`
- Open (page) → `file.open`
- Save (command) → `file.save`
- Save As (page) → `file.save_as`
- Print (page) → `file.print`
- Export (page)
- Close (command) → `file.close`
- Options (command) → `app.office.center.options`

## Tab: Home

### Clipboard (layout `stack`)

- [0,0] L Paste (split) → `edit.paste` 🖼
  - Paste → `edit.paste` 🖼
  - Paste Special... → `edit.paste.special.dialog` 🖼
  - Paste as Hyperlink → `tools.inet.paste.as.hyperlink` 🖼
  - Duplicate → `edit.paste.duplicate` 🖼
- [1,0] M Cut → `edit.cut` 🖼
- [1,1] M Copy → `edit.copy` 🖼
- [1,2] M Format Painter → `tools.format_painter` 🖼

### Font (layout `rows`)

- [0,0] Font (combo) → `text.font`
- [1,0] Font Size (combo) → `text.font.size`
- [2,0] S Increase Font Size → `text.font.size.increase` 🖼
- [3,0] S Decrease Font Size → `text.font.size.decrease` 🖼
- [0,1] S Bold → `format.text.bold` 🖼
- [1,1] S Italic → `format.text.italic` 🖼
- [2,1] S Underline → `format.text.underline` 🖼
- [3,1] S Strikethrough → `tools.strikethrough` 🖼
- [4,1] S Change Case... (dropdown) → `tools.change.case.dialog.classic` 🖼
  - Normal → `text.normal.case` 🖼
  - All Caps → `format.text.all_caps` 🖼
  - Capitalize Each Word → `text.initial.caps`
  - Small Caps → `format.text.small_caps` 🖼
- [5,1] S Font Color (split) *[list: colors]* → `text.font.color` 🖼
- launcher: Font → `format.all.text.props`

### Paragraph (layout `rows`)

- [0,0] S Align Top → `format.text.align_top` 🖼
- [1,0] S Align Middle → `format.text.align_middle` 🖼
- [2,0] S Align Bottom → `format.text.align_bottom` 🖼
- [3,0] S Bullets → `text.bullets` 🖼
- [4,0] S Rotate Text → `text.rotate90` 🖼
- [0,1] S Align Left → `format.text.align_left` 🖼
- [1,1] S Center → `format.text.align_center` 🖼
- [2,1] S Align Right → `format.text.align_right` 🖼
- [3,1] S Justify → `format.text.align_justify` 🖼
- [4,1] S Decrease Indent → `format.decrease.indent` 🖼
- [5,1] S Increase Indent → `format.increase.indent` 🖼
- S Left-to-Right → `text.direction.left.to.right` 🖼 *(hidden)*
- S Right-to-Left → `text.direction.right.to.left` 🖼 *(hidden)*
- S Text Direction → `text.direction.vertical` 🖼 *(hidden)*
- launcher: Paragraph → `format.paragraph`

### Tools (layout `stack`)

- [0,0] M Pointer Tool → `tools.pointer` 🖼
- [0,1] M Connector → `tools.connector` 🖼
- [0,2] M Text → `tools.text` 🖼
- [1,0] S Rectangle (split) → `shape.drawing.tools`
  - Rectangle → `tools.rectangle` 🖼
  - Ellipse → `tools.ellipse` 🖼
  - Line → `format.line` 🖼
  - Freeform → `tools.freeform` 🖼
  - Arc → `shape.arc` 🖼
  - Pencil → `tools.pencil` 🖼
- [1,1] S Connection Point → `tools.connection.point` 🖼
- [1,2] S Text Block → `tools.text_block` 🖼

### Shape Styles (layout `stack`)

- [0,0] L Quick Styles (gallery) *[list: shape_styles]* → `shape.styles` 🖼
- [1,0] M Fill (split) *[list: colors]* → `format.set.fill.color` 🖼
  - Fill Options... → `format.fill`
- [1,1] M Line (split) *[list: colors]* → `format.set.line.color` 🖼
  - Weight (▸)
    - *[list: line_weights]*
  - Dashes (▸)
    - *[list: line_patterns]*
  - Arrows (▸)
    - *[list: line_ends]*
    - Arrows Options... → `tools.arrows.more` 🖼
  - Line Options... → `format.line`
- [1,2] M Effects (dropdown) *[list: effects]* → `shape.effects` 🖼
  - Shadow (▸)
    - *[list: shadow_presets]*
    - Shadow Options... → `tools.shadow.options.dialog` 🖼
  - Reflection (▸)
    - *[list: reflection_presets]*
    - Reflection Options... → `tools.reflections.more.options` 🖼
  - Glow (▸)
    - *[list: glow_presets]*
    - Glow Options... → `tools.glows.more.options` 🖼
  - Soft Edges (▸)
    - *[list: soft_edge_presets]*
    - Soft Edges Options... → `tools.soft.edges.more.options` 🖼
  - Bevel (▸)
    - *[list: bevel_presets]*
  - 3-D Rotation (▸)
    - *[list: rotation_3d_presets]*
    - 3-D Rotation Options... → `tools.rotation_3d_options` 🖼
- launcher: Format Object... → `format.shape.pane` *new*

### Arrange (layout `stack`)

- [0,0] L Align (dropdown) *[list: align]* → `arrange.align.box` 🖼
- [1,0] L Position (dropdown) *[list: position]* → `arrange.position` 🖼
  - Rotate Shapes (▸)
    - Rotate Right 90° → `arrange.rotate90.clockwise` 🖼
    - Rotate Left 90° → `arrange.object.rotate90` 🖼
    - Flip Vertical → `arrange.flip_vertical` 🖼
    - Flip Horizontal → `arrange.flip_horizontal` 🖼
  - Rotate Diagram (▸)
    - *[list: diagram_rotations]*
- [2,0] M Bring to Front (split) → `arrange.bring_to_front` 🖼
  - Bring Forward → `arrange.bring_forward` 🖼
  - Bring to Front → `arrange.bring_to_front` 🖼
- [2,1] M Send to Back (split) → `arrange.send_to_back` 🖼
- [2,2] M Group (dropdown) → `arrange.group` 🖼
  - Group → `arrange.group` 🖼
  - Ungroup → `arrange.ungroup` 🖼
  - Add to Group → `tools.objects.add.to`
  - Remove from Group → `tools.objects.remove.from`
  - Convert to Group → `tools.objects.convert.to`

### Editing (layout `stack`)

- [0,0] L Change Shape (gallery) *[list: shapes_for_change_shape]* → `edit.replace.shape` 🖼
- [1,0] M Find (dropdown) → `edit.find` 🖼
  - Find... → `tools.find.dialog` 🖼
  - Replace... → `edit.replace.dialog` 🖼
- [1,1] M Layers (dropdown) → `tools.layers` 🖼
  - Assign to Layer... → `layer.dialog` 🖼
  - Layer Properties... → `layer.properties.dialog` 🖼
- [1,2] M Select (dropdown) → `edit.select.special` 🖼
  - Select All → `edit.select_all` 🖼
  - Select by Type... → `edit.select.by.type.dialog`
  - Area Select → `tools.area.select` 🖼
  - Lasso Select → `tools.lasso.select.visio` 🖼

## Tab: Insert

### Pages (layout `stack`)

- [0,0] L New Page (split) → `tools.blank.page.insert` 🖼
  - Blank Page → `tools.blank.page.insert.visio` 🖼
  - Background Page... → `insert.new.background.page` 🖼
  - Duplicate This Page → `edit.duplicate.page` 🖼

### Illustrations (layout `stack`)

- [0,0] L Pictures... → `tools.picture.insert.from.file` 🖼
- [1,0] L Online Pictures... → `insert.clip.art` 🖼
- [2,0] L CAD Drawing... → `tools.cad.drawing` 🖼
- L Chart... → `tools.chart.insert` 🖼 *(hidden)*

### Diagram Parts (layout `stack`)

- [0,0] L Container (dropdown) *[list: container]* → `insert.add.to.new.container` 🖼
- [1,0] L Callout (dropdown) *[list: callout]* → `shape.associate.callout` 🖼
- [2,0] L Connector → `tools.quick.connector` 🖼

### Links (layout `stack`)

- [0,0] L Hyperlink... → `tools.hyperlink.insert` 🖼

### Text (layout `stack`)

- [0,0] L Text Box (split) → `insert.text.box` 🖼
  - Horizontal Text Box → `insert.text.box` 🖼
  - Vertical Text Box → `insert.vert.text.box` 🖼
- [1,0] L ScreenTip... → `tools.screen.tip` 🖼
- [2,0] L Object... → `tools.ole.objectct.insert` 🖼
- [3,0] L Field... → `edit.insert.field` 🖼
- [4,0] L Symbol (dropdown) → `tools.mso.insert.symbol` 🖼

## Tab: Design

### Page Setup (layout `stack`)

- [0,0] L Orientation (dropdown) → `format.set.page.orientation` 🖼
  - *[options: Portrait, Landscape]*
- [1,0] L Size (dropdown) → `page.size.dlg` 🖼
  - *[options: Letter, Tabloid, Legal, Statement, Executive, A3, A4, A5, B4 (JIS), B5 (JIS)]*
  - Fit to Drawing → `page.size.to.fit.drawing` 🖼
  - More Page Sizes... → `tools.more.page.sizes` 🖼
- [2,0] L Auto Size → `page.auto.size.toggle` 🖼
- launcher: Page Setup... → `tools.options.page.setup`

### Themes (layout `stack`)

- [0,0] L Themes (dropdown) *[list: themes]* → `tools.themes.gallery.visio.legacy` 🖼
  - Apply Theme to New Shapes → `format.apply.theme.to.new.shapes.toggle`

### Variants (layout `stack`)

- [0,0] L Variants (gallery) *[list: theme_variants]* → `tools.theme.variants` 🖼

### Backgrounds (layout `stack`)

- [0,0] L Backgrounds (dropdown) *[list: backgrounds]* → `tools.backgrounds` 🖼
  - Background Color (▸)
    - *[list: colors]*
- [1,0] L Borders & Titles (dropdown) *[list: borders_and_titles]* → `tools.borders.and.titles` 🖼
- launcher: Page Setup → `tools.options.page.setup`

### Layout (layout `stack`)

- [0,0] L Re-Layout Page (dropdown) → `page.relayout` 🖼
  - *[options: Top To Bottom, Bottom To Top, Left to Right, Right to Left, Down then Right, Down then Left, Right then Down, Left then Down, Radial, Circular]*
  - More Layout Options... → `arrange.layout.more.options` 🖼
- [1,0] L Connectors (dropdown) → `connector.style` 🖼
  - Right Angle → `connector.right_angle` 🖼
  - Straight Lines → `connector.straight.lines` 🖼
  - Curved Lines → `connector.curved.lines` 🖼
  - Show Line Jumps → `connector.show.line.jumps`
- launcher: Layout & Routing... → `page.setup.layout_and_routing` *new*

## Tab: Data

### External Data (layout `stack`)

- [0,0] L Link Data to Shapes → `data.link.data.to.shapes` 🖼
- [1,0] L Automatically Link... → `data.automatically.link` 🖼
- [2,0] L Refresh All (split) → `data.refresh` 🖼

### Display Data (layout `stack`)

- [0,0] L Data Graphics (dropdown) *[list: data_graphics]* → `data.graphics.gallery.visio` 🖼
  - Create New Data Graphic... → `data.graphic.create.new` 🖼
  - Edit Data Graphic... → `data.graphic.edit` 🖼
  - Apply after Linking Data to Shapes → `format.apply.data.graphic`
- [1,0] L Insert Legend (dropdown) → `insert.legend.horizontal` 🖼
  - *[options: Horizontal, Vertical]*

### Show/Hide (layout `stack`, 2 rows)

- [0,0] Shape Data Window (check) → `shape.data.window` 🖼
- [0,1] External Data Window (check) → `tools.external.data.window` 🖼

## Tab: Process

### Subprocess (layout `stack`)

- [0,0] L Create New → `tools.subprocess.create.new` 🖼
- [1,0] L Link to Existing (dropdown) → `tools.subprocess.link.to.existing` 🖼
- [2,0] L Create from Selection → `tools.subprocess.create.from.selection` 🖼

### Diagram Validation (layout `stack`, 2 rows)

- [0,0] L Check Diagram (split) → `arrange.diagram.check` 🖼
  - Check Diagram → `arrange.diagram.check` 🖼
  - Rules to Check (▸)
    - *[list: validation_rule_sets]*
  - Import Rules From (▸)
    - *[list: open_documents]*
- [1,0] M Ignore This Issue (split) → `review.ignore.this.issue` 🖼
- [1,1] Issues Window (check) → `review.validation.issues.window.toggle`

## Tab: Review

### Proofing (layout `stack`)

- [0,0] L Spelling... → `review.spelling.change` 🖼
- [1,0] L Research... → `view.task.pane.research` 🖼
- [2,0] L Thesaurus... → `help.research.thesaurus` 🖼

### Language (layout `stack`)

- [0,0] L Translate → `help.research.translate` 🖼
- [1,0] L Language (dropdown) → `tools.language.commands` 🖼
  - Set Proofing Language... → `format.set.language.dlg` 🖼
  - Language Preferences... → `tools.language.preferences.dlg`

### Comments (layout `stack`)

- [0,0] L New Comment → `review.new.comment` 🖼
- [1,0] L Comments Pane (split) → `view.show.comments` 🖼
  - Comments Pane → `tools.commenting` 🖼
  - Reveal Tags → `view.show.tags`
- [2,0] L Ink → `tools.ink.tool` 🖼

### Reports (layout `stack`)

- [0,0] L Shape Reports → `shape.reports` 🖼

## Tab: View

### Views (layout `stack`)

- [0,0] L Presentation Mode → `view.full_screen_mode`

### Show (layout `stack`, 2 rows)

- [0,0] Ruler (check) → `view.rulers` 🖼
- [0,1] Grid (check) → `view.toggle_grid` 🖼
- [1,0] Page Breaks (check) → `view.show.page.breaks` 🖼
- [1,1] Guides (check) → `view.guides` 🖼
- [2,0] L Task Panes (dropdown) → `view.task.panes.menu.visio` 🖼
  - Shapes → `shape.shapes.window` 🖼
  - Shape Data → `shape.data.window.show.hide` 🖼
  - Pan & Zoom → `view.pan.zoom` 🖼
  - Size & Position → `arrange.size.and.position.window` 🖼
- launcher: Ruler & Grid... → `view.ruler.grid.dlg`

### Zoom (layout `stack`)

- [0,0] M Zoom... → `view.zoom.dialog` 🖼
- [0,1] M Fit to Window → `view.zoom.fit` 🖼
- [0,2] M Page Width → `view.zoom.page.width` 🖼

### Visual Aids (layout `stack`)

- [0,0] Dynamic Grid (check) → `connector.dynamic.grid` 🖼
- [0,1] AutoConnect (check) → `connector.auto_connect` 🖼
- [0,2] Connection Points (check) → `tools.connection.points` 🖼
- launcher: Snap & Glue... → `tools.snap_glue_setup`

### Window (layout `stack`)

- [0,0] L New Window → `view.window.new.window` 🖼
- [1,0] L Arrange All → `tools.windows.arrange.all` 🖼
- [2,0] L Cascade → `tools.windows.cascade` 🖼
- [3,0] L Switch Windows (dropdown) → `view.window.switch.windows.menu.word` 🖼
  - *[list: windows]*

### Macros (layout `stack`)

- [0,0] L Macros → `tools.macro.play` 🖼
- [1,0] L Add-Ons (dropdown) → `insert.add.ons` 🖼
  - Run Add-On... → `developer.run_add_on` *new*
  - *[list: add_ons]*

## Contextual tab: Picture Tools / Format (shown in `picture_selected`)

### Adjust (layout `stack`)

- [0,0] L Brightness (dropdown) *[list: brightness]* → `tools.picture.brightness` 🖼
  - Picture Corrections Options... → `tools.picture.corrections.dialog` 🖼
- [1,0] L Contrast (dropdown) → `tools.picture.contrast` 🖼
- [2,0] L AutoBalance → `tools.picture.auto.balance` 🖼
- [3,0] L Compress Picture → `tools.picture.compress` 🖼
- launcher: Picture... → `insert.image.properties.dlg`

### Picture Styles (layout `stack`)

- [0,0] L Line (split) *[list: colors]* → `format.set.line.color` 🖼

### Arrange (layout `stack`)

- [0,0] L Bring to Front (split) → `arrange.bring_to_front` 🖼
- [1,0] L Send to Back (split) → `arrange.send_to_back` 🖼
- [2,0] L Rotate (dropdown) → `arrange.rotate.object` 🖼
- [3,0] L Crop Tool → `tools.crop` 🖼

## Contextual tab: Container Tools / Format (shown in `container_selected`)

### Size (layout `stack`, 2 rows)

- [0,0] L Margins (dropdown) *[list: margins]* → `shape.container.margins` 🖼
- [1,0] M Fit to Contents → `view.fit.container.to.contents` 🖼
- [1,1] M Automatic Resize (dropdown) → `tools.automatic.resize` 🖼

### Container Styles (layout `stack`)

- [0,0] L Heading Style (dropdown) → `shape.container.heading.styles` 🖼

### Membership (layout `stack`)

- [0,0] L Lock Container → `shape.lock.container` 🖼
- [1,0] L Select Contents → `edit.select.contents` 🖼
- [2,0] L Disband Container → `arrange.disband.container` 🖼

## Contextual tab: ShapeSheet Tools / Design (shown in `shapesheet_window`)

### View (layout `stack`, 2 rows)

- [0,0] L Formulas → `edit.sw.show.formulas` 🖼
- [1,0] L Values → `edit.sw.show.values` 🖼
- [2,0] M Sections → `edit.sw.show.sections.dlg` 🖼
- [2,1] Style Explorer (check) → `format.style.explorer` 🖼

### Sections (layout `stack`, 2 rows)

- [0,0] M Insert → `shape.sheet.insert.section` 🖼
- [0,1] M Delete → `shape.sheet.delete.section` 🖼

### Rows (layout `stack`, 2 rows)

- [0,0] M Insert → `shape.sheet.insert.row` 🖼
- [0,1] M Delete → `shape.sheet.delete.row` 🖼

### Editing (layout `stack`)

- [0,0] L Edit Formula → `shape.sheet.edit.formula` 🖼
- [1,0] M Shape Data → `format.cust.prop.edit` 🖼
- [1,1] M Hyperlink... → `tools.hyperlink.insert` 🖼
- [1,2] M Shape Action → `shape.sheet.shape.action` 🖼

### Formula Tracing (layout `stack`)

- [0,0] M Show Window → `view.show.formula.tracing.window` 🖼
- [0,1] M Trace Precedents → `tools.trace.precedent.cells` 🖼
- [0,2] M Trace Dependents → `tools.trace.dependent.cells` 🖼
- [1,0] L Back → `tools.trace.back` 🖼
- [2,0] L Forward → `tools.trace.forward` 🖼

## Context menus

### Shape: right-click a shape

- ———
- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- ———
- <u>I</u>nsert Shape Before → `shape.insert_shape_before` *new* *(hidden)*
- I<u>n</u>sert Shape After → `shape.insert_shape_after` *new* *(hidden)*
- ———
- <u>G</u>roup (▸)
  - <u>G</u>roup `Shift+Ctrl+G` → `arrange.group` 🖼
  - <u>U</u>ngroup `Shift+Ctrl+U` → `arrange.ungroup` 🖼
  - ———
  - <u>O</u>bject... → `edit.open.object` *(hidden)*
- Contain<u>e</u>r (▸)
  - <u>F</u>it to Contents → `view.fit.container.to.contents` *(hidden)*
  - <u>L</u>ock Container → `shape.lock.container` *(hidden)*
  - <u>S</u>elect Contents → `edit.select.container.members` *(hidden)*
  - <u>D</u>isband Container → `arrange.disband.container` *(hidden)*
  - ———
  - <u>A</u>dd to New Container → `insert.add.to.new.container`
  - Add to <u>U</u>nderlying Container → `insert.add.member.to.container`
- ———
- *[list: hyperlinks]* *(hidden)*
- <u>A</u>dd Comment → `insert.comment` 🖼
- Edi<u>t</u> Text `F2` → `text.begin` 🖼
- ———
- <u>D</u>ata (▸)
  - <u>S</u>hape Data... → `tools.cust.prop` 🖼
  - <u>D</u>efine Shape Data... → `shape.data.define_shape_data` *new*
  - ———
  - <u>E</u>dit Data Graphic... → `shape.data.edit_data_graphic` *new*
  - <u>R</u>emove Data Graphic → `edit.remove.data.graphic.from.sel`
  - *[list: data_recordset_links]* *(hidden)*
- Format <u>S</u>hape `F3` → `format.shape.pane` *new*
- ———
- R<u>i</u>ght-Angle Connector → `connector.right_angle` 🖼
- Strai<u>g</u>ht Connector → `connector.straight` 🖼
- Cur<u>v</u>ed Connector → `connector.curved` 🖼
- ———
- Reset Co<u>n</u>nector → `connector.dyn.conn.reroute` 🖼 *(hidden)*
- Convert <u>I</u>nk to Text → `shape.convert_ink_to_text` *new* *(hidden)*
- Convert Ink to Geo<u>m</u>etry → `shape.convert_ink_to_geometry` *new* *(hidden)*

### Page: right-click an empty part of the page

- ———
- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy `Ctrl+C` → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- P<u>a</u>ste Special... `Ctrl+Alt+V` → `edit.paste.special` 🖼
- ———
- *[list: hyperlinks]* *(hidden)*
- <u>A</u>dd Comment → `insert.comment` 🖼

### Text editing: right-click inside text being edited

- ———
- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- ———
- Fo<u>n</u>t... → `format.all.text.props` 🖼
- Para<u>g</u>raph... → `format.paragraph`
- Bullet<u>s</u> → `text.bullets` 🖼
- Insert <u>F</u>ield... → `edit.insert.field`
- ———
- Recon<u>v</u>ert → `text.reconvert` *new*
- <u>C</u>hange Text Direction → `view.direction.toggle` 🖼 *(hidden)*
- H<u>o</u>rizontal in Vertical... → `text.horizontal_in_vertical` *new* *(hidden)*
- ———
- Text <u>R</u>uler → `text.edit.ruler`

### Page tab: right-click a page tab

- <u>I</u>nsert → `tools.options.new.page`
- <u>D</u>elete → `page.delete` 🖼
- <u>R</u>ename → `page.rename`
- D<u>u</u>plicate → `edit.duplicate.page`
- Page <u>S</u>etup... → `tools.options.page.setup`
- ———
- Reorder <u>P</u>ages... `Ctrl+Alt+P` → `page.reorder_pages` *new*

### Page navigation: right-click the page navigation buttons

- *[list: pages]* *(hidden)*
- ———
- <u>G</u>o to Page... → `tools.options.go.to.drawing`

### Hyperlink: right-click a hyperlink in text

- <u>O</u>pen → `insert.hyperlink.open` *new*
- Ope<u>n</u> in New Window → `insert.hyperlink.open_in_new_window` *new*
- ———
- <u>C</u>opy Hyperlink → `insert.hyperlink.copy_hyperlink` *new*
- <u>A</u>dd to Favorites → `insert.hyperlink.add_to_favorites` *new*
- <u>E</u>dit Hyperlink → `insert.hyperlink.edit_hyperlink` *new*

### Connection point: right-click a connection point

- <u>I</u>nward → `edit.connection_point.inward` *new*
- <u>O</u>utward → `edit.connection_point.outward` *new*
- In<u>w</u>ard & Outward → `edit.connection_point.inward_and_outward` *new*

### Full screen: right-click in full-screen view

- <u>P</u>revious Page → `page.previous` 🖼
- <u>N</u>ext Page → `page.next` 🖼
- ———
- <u>G</u>o To (▸)
  - <u>F</u>oreground → `tools.options.edit.drawing`
  - <u>B</u>ackground → `tools.options.edit.background`
  - ———
  - *[list: pages]* *(hidden)*
  -  → `view.go_to.unnamed` *new* *(hidden)*
  - ———
  - <u>P</u>age... → `tools.options.go.to.drawing`
- <u>C</u>lose → `view.full_screen_mode`

### ShapeSheet: right-click in a ShapeSheet window

- <u>V</u>alues → `edit.sw.show.values`
- <u>F</u>ormulas → `edit.sw.show.formulas`
- ———
- View Sectio<u>n</u>s... → `edit.sw.show.sections.dlg`
- In<u>s</u>ert Section... → `edit.sw.add.section.dlg`
- D<u>e</u>lete or Inherit Section → `edit.delete_or_inherit_section` *new*
- ———
- Inse<u>r</u>t Row → `edit.sw.insert.row`
- Insert Row <u>A</u>fter → `edit.sw.insert.row.after`
- <u>D</u>elete Row → `edit.sw.delete.row`
- E<u>x</u>pand Row → `edit.sw.expand.row`
- <u>C</u>hange Row Type → `edit.sw.change.row.type.dlg`

### Issues list: right-click an issue in the Issues window

- Ig<u>n</u>ore This Issue → `review.ignore.this.issue` 🖼
- Ignore R<u>u</u>le → `review.ignore.validation.rule`
- <u>S</u>top Ignoring This Issue → `review.issues.stop_ignoring_this_issue` *new*
- S<u>t</u>op Ignoring Rule → `review.issues.stop_ignoring_rule` *new*
- S<u>h</u>ow Ignored Issues → `view.show.ignored.issues.toggle`
- ———
- A<u>r</u>range by (▸)
  - <u>R</u>ule → `review.validation.issues.arrange.by.rule` *(hidden)*
  - <u>C</u>ategory → `review.validation.issues.arrange.by.category` *(hidden)*
  - <u>P</u>age → `tools.options.go.to.drawing` *(hidden)*
  - Ig<u>n</u>ored → `review.validation.issues.arrange.by.ignored` *(hidden)*
  - ———
  - <u>O</u>riginal Order → `review.validation.issues.arrange.original.order` *(hidden)*

### Embedded object: right-click an embedded object

- *[list: hyperlinks]* *(hidden)*
- ———
- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- P<u>a</u>ste Special... `Ctrl+Alt+V` → `edit.paste.special` 🖼
- ———
- *[list: placeholder]*
- ———
- <u>D</u>ata (▸)
  - <u>S</u>hape Data... → `tools.cust.prop` 🖼
  - <u>D</u>efine Shape Data... → `shape.data.define_shape_data` *new*
  - ———
  - <u>E</u>dit Data Graphic... → `shape.data.edit_data_graphic` *new*
  - <u>R</u>emove Data Graphic → `edit.remove.data.graphic.from.sel`
  - *[list: data_recordset_links]* *(hidden)*
- Fo<u>r</u>mat (▸)
  - <u>T</u>ext... → `format.all.text.props` 🖼
- Format <u>S</u>hape `F3` → `format.shape.pane` *new*
- ———
- <u>O</u>bject... → `edit.open.object`

### Master editing: right-click the page of a master being edited

- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- Paste Sh<u>o</u>rtcut → `edit.paste.shortcut`
- ———
- Add to <u>M</u>y Shapes (▸)
  - *[list: my_shapes]* *(hidden)*
  - ———
  - Add To <u>N</u>ew Stencil... → `shape.master.add_to_my_shapes.add_to_new_stencil` *new*
  - Add To <u>E</u>xisting Stencil... → `shape.master.add_to_my_shapes.add_to_existing_stencil` *new*
  - ———
  - <u>O</u>rganize My Shapes → `file.shapes.organize_my_shapes` *new*
- Add to <u>Q</u>uick Shapes → `shape.master.add_to_quick_shapes` *new*
- Remove from <u>Q</u>uick Shapes → `shape.master.remove_from_quick_shapes` *new*
- ———
- <u>N</u>ew Master... → `shape.sten.new.master`
- De<u>l</u>ete Master → `shape.master.delete_master` *new* 🖼
- <u>R</u>ename Master → `shape.master.rename_master` *new*
- <u>E</u>dit Master (▸)
  - <u>P</u>roperties... → `shape.master.edit_master.properties` *new*
  - ———
  - <u>E</u>dit Master Shape → `shape.master.edit_master.edit_master_shape` *new* 🖼
  - Ed<u>i</u>t Icon Image → `shape.master.edit_master.edit_icon_image` *new*
- ———
- <u>H</u>elp → `help.contents` 🖼 *(hidden)*

### Stencil editing: right-click a master in an editable stencil

- <u>E</u>dit Stencil → `shape.stencil.edit_stencil` *new* 🖼
- <u>S</u>ave → `file.save` 🖼
- S<u>a</u>ve As... → `file.save_as`
- Propert<u>i</u>es... → `shape.stencil.properties` *new* 🖼
- ———
- <u>V</u>iew (▸)
  - Icons and Na<u>m</u>es → `shape.sten.icon.and.name` 🖼
  - Names <u>U</u>nder Icons → `shape.sten.names.under.icons` 🖼
  - Ic<u>o</u>ns Only → `shape.sten.icon.only` 🖼
  - <u>N</u>ames Only → `shape.sten.name.only` 🖼
  - Icons and De<u>t</u>ails → `shape.sten.icon.and.detail` 🖼
- <u>O</u>rder (▸)
  - Move <u>U</u>p → `shape.stencil.order.move_up` *new*
  - Move <u>D</u>own → `shape.stencil.order.move_down` *new*
- <u>R</u>eset Stencil → `shape.stencil.reset_stencil` *new*
- <u>D</u>rawing Explorer Window → `shape.drawing.explorer` 🖼
- ———
- Co<u>l</u>lapse All Groups → `shape.stencil.collapse_all_groups` *new* 🖼
- E<u>x</u>pand All Groups → `shape.stencil.expand_all_groups` *new* 🖼
- ———
- <u>F</u>loat Window → `shape.stencil.float_window` *new*
- <u>C</u>lose → `shape.stencil.close` *new* 🖼

