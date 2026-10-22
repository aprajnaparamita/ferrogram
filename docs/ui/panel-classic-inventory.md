# Panel Classic layout inventory

Generated from `ui-spec/layouts/panel-classic.yaml`; do not edit by hand.
Underlined letters are access keys; ——— is a separator; 🖼 marks an item that draws an icon;
*new* marks a command id that Phase 7 adds.

## Menu bar: Drawing menu bar

### <u>F</u>ile

- <u>N</u>ew (▸) 🖼
  - <u>G</u>etting Started... → `file.create.new.drawing`
  - <u>N</u>ew Drawing `Ctrl+N` → `file.new.blank.drawing` 🖼
  - N<u>e</u>w Drawing (Metric) → `file.new.blank.drawing.metric` 🖼
  - N<u>e</u>w Drawing (US Units) → `file.new.blank.drawing.us` 🖼
  - New Drawing from <u>T</u>emplate... → `file.new` 🖼
  - ———
  - *[list: template_categories]*
- <u>O</u>pen... `Ctrl+O` → `file.open` 🖼
- <u>C</u>lose → `file.close`
- ———
- <u>S</u>ave `Ctrl+S` → `file.save` 🖼
- S<u>a</u>ve As... → `file.save_as`
- Save as Web Pa<u>g</u>e... → `file.save.as.web.page` 🖼
- Pu<u>b</u>lish as PDF or XPS... → `file.save.as.fixed.format.dlg` *(hidden)*
- <u>R</u>emove Hidden Information... → `help.rhi.dlg`
- ———
- Ch<u>e</u>ck In → `file.checkin` *(hidden)*
- Ch<u>e</u>ck Out → `file.checkout` *(hidden)*
- Discard Chec<u>k</u> Out → `file.undo.checkout` *(hidden)*
- ———
- Shap<u>e</u>s (▸) 🖼
  - <u>M</u>y Shapes (▸)
    - *[list: my_shapes]*
    - ———
    - <u>O</u>rganize My Shapes → `file.shapes.organize_my_shapes`
  - ———
  - *[list: stencil_categories]*
  - ———
  - <u>O</u>pen Stencil... → `file.open.stencil` 🖼
  - <u>N</u>ew Stencil → `file.new.blank.stencil` 🖼
  - N<u>e</u>w Stencil (Metric) → `file.new.blank.stencil.metric` 🖼
  - N<u>e</u>w Stencil (US Units) → `file.new.blank.stencil.us` 🖼
  - Show <u>D</u>ocument Stencil → `view.window.show.master.objects`
- ———
- Page Set<u>u</u>p... → `tools.options.page.setup`
- Print Pre<u>v</u>iew → `file.print_preview` 🖼
- <u>P</u>rint... `Ctrl+P` → `file.print` 🖼
- ———
- Sen<u>d</u> To (▸)
  - <u>M</u>ail Recipient (as Attachment)... → `file.send.as.mail` 🖼
- Propert<u>i</u>es → `file.summary.info.dlg`
- ———
- *[list: recent_files]*
- ———
- E<u>x</u>it → `file.exit`

### <u>E</u>dit

- <u>U</u>ndo `Ctrl+Z` → `edit.undo` 🖼 *(hidden)*
- <u>R</u>edo `Ctrl+Y` → `edit.redo` 🖼 *(hidden)*
- ———
- Cu<u>t</u> `Ctrl+X` → `edit.cut` 🖼
- <u>C</u>opy `Ctrl+C` → `edit.copy` 🖼
- <u>P</u>aste `Ctrl+V` → `edit.paste` 🖼
- Pa<u>s</u>te Special... `Ctrl+Alt+V` → `edit.paste.special`
- Paste as <u>H</u>yperlink → `tools.inet.paste.as.hyperlink`
- ———
- Cle<u>a</u>r `Delete` → `edit.clear`
- Se<u>l</u>ect All `Ctrl+A` → `edit.select_all`
- Select <u>b</u>y Type... → `edit.select.special`
- Dupl<u>i</u>cate `Ctrl+D` → `edit.duplicate`
- <u>D</u>elete Pages... → `page.delete`
- ———
- <u>F</u>ind... `Ctrl+F` → `edit.find` 🖼
- R<u>e</u>place... → `edit.replace`
- ———
- Recon<u>v</u>ert → `edit.reconvert` *new*
- ———
- <u>G</u>o To (▸)
  - <u>F</u>oreground → `tools.options.edit.drawing`
  - <u>B</u>ackground → `tools.options.edit.background`
  - ———
  - *[list: pages]*
  - ———
  - <u>P</u>age... → `tools.options.go.to.drawing`
- ———
- Lin<u>k</u>s... → `edit.links`
- <u>O</u>bject... → `edit.open.object`

### <u>V</u>iew

- Shapes Wi<u>n</u>dow → `shape.shapes.window` 🖼
- <u>P</u>an & Zoom Window → `view.pan.zoom` 🖼
- Shap<u>e</u> Data Window → `tools.cust.prop` 🖼
- <u>S</u>ize & Position Window → `arrange.size.pos` 🖼
- <u>D</u>rawing Explorer Window → `shape.drawing.explorer` 🖼
- Mast<u>e</u>r Explorer Window → `shape.master.explorer` 🖼 *(hidden)*
- E<u>x</u>ternal Data Window → `tools.external.data.window`
- ———
- Tas<u>k</u> Pane `Ctrl+F1` → `tools.mso.custom.item`
- <u>T</u>oolbars (▸)
  - Standard (check) → `view.toolbar.standard` *new*
  - Formatting (check) → `view.toolbar.formatting` *new*
  - Action → `view.toolbar.action` *new*
  - Data → `view.toolbar.data` *new*
  - Developer → `view.toolbar.developer` *new*
  - Drawing → `view.toolbar.drawing` *new*
  - Format Shape → `format.shape.pane`
  - Format Text → `view.toolbar.format_text` *new*
  - Ink → `tools.ink.tool`
  - Layout & Routing → `view.toolbar.layout_and_routing` *new*
  - Picture → `view.toolbar.picture` *new*
  - Reviewing → `view.toolbar.reviewing` *new*
  - Snap & Glue → `tools.snap_glue_setup`
  - Stencil → `view.toolbar.stencil` *new*
  - View → `view.toolbar.view` *new*
  - Web → `tools.inet.visio.on.the.web`
  - ———
  - <u>C</u>ustomize... → `view.toolbar.customize` *new*
- <u>V</u>iew Direction (▸) → `view.direction.toggle` *(hidden)*
  - <u>L</u>eft-to-Right → `view.left.to.right`
  - <u>R</u>ight-to-Left → `view.right.to.left`
- ———
- <u>R</u>ulers (check) → `view.rulers`
- <u>G</u>rid (check) → `view.toggle_grid`
- G<u>u</u>ides (check) → `view.toggle_guides`
- <u>C</u>onnection Points (check) → `view.connections`
- Page <u>B</u>reaks (check) → `view.page.breaks`
- ———
- <u>H</u>eader and Footer... → `file.header.footer`
- M<u>a</u>rkup → `view.markup` *new* 🖼
- <u>L</u>ayer Properties... → `layer.setup.dlg` 🖼
- ———
- <u>F</u>ull Screen `F5` → `view.full_screen_mode` 🖼
- <u>Z</u>oom (▸)
  - <u>L</u>ast Zoom → `view.zoom.last`
  - <u>P</u>age Width → `view.zoom.page.width`
  - <u>W</u>hole Page → `view.fit.in.window`
  - ———
  - <u>4</u>00% → `view.view400`
  - <u>2</u>00% → `view.view200`
  - 15<u>0</u>% → `view.view150`
  - <u>1</u>00% → `view.view100`
  - <u>7</u>5% → `view.view75`
  - <u>5</u>0% → `view.view50`
  - ———
  - <u>Z</u>oom... → `view.custom`

### <u>I</u>nsert

- <u>N</u>ew Page... → `tools.options.new.page` 🖼
- ———
- <u>F</u>ield... → `edit.insert.field`
- <u>S</u>ymbol... → `tools.mso.insert.symbol.dlg`
- Shape Screen<u>T</u>ip... → `shape.comment.dlg`
- Co<u>m</u>ment → `insert.comment` 🖼
- ———
- <u>P</u>icture (▸)
  - <u>C</u>lip Art... → `tools.mso.media.gallery` 🖼
  - <u>F</u>rom File... → `file.import` 🖼
  - ———
  - C<u>h</u>art → `insert.microsoft.graph` 🖼
  - <u>D</u>ata Map... → `insert.data.map`
  - <u>E</u>quation... → `tools.mso.insert.equation` 🖼
  - <u>W</u>ord Art... → `insert.word.art` 🖼
- Te<u>x</u>t Box → `insert.text.box` 🖼
- Te<u>x</u>t Box (▸) → `insert.text.box`
  - <u>H</u>orizontal → `insert.text.box` 🖼
  - <u>V</u>ertical → `insert.vert.text.box` 🖼
- CA<u>D</u> Drawing... → `insert.auto.cad.add.on`
- <u>C</u>ontrol... → `insert.control.dlg` 🖼
- ———
- <u>O</u>bject... → `edit.insert.object`
- <u>H</u>yperlinks... `Ctrl+K` → `insert.hyper.link` 🖼

### F<u>o</u>rmat

- <u>T</u>ext... → `format.all.text.props` 🖼
- <u>L</u>ine... → `format.line` 🖼
- <u>F</u>ill... → `format.fill` 🖼
- Shado<u>w</u>... → `format.shadow` 🖼
- <u>C</u>orner Rounding... → `format.corners` 🖼
- ———
- The<u>m</u>e... → `format.theme` *new* 🖼
- ———
- P<u>i</u>cture... → `insert.image.properties.dlg` 🖼
- I<u>n</u>k... → `format.ink.dlg`
- ———
- <u>P</u>rotection... → `format.protection`
- <u>B</u>ehavior... → `format.behavior`
- Sp<u>e</u>cial... → `format.special` *(hidden)*
- ———
- <u>S</u>tyle... → `format.style` *(hidden)*
- <u>D</u>efine Styles... → `format.define.styles` *(hidden)*
- ———
- Laye<u>r</u>... → `layer.dlg`

### <u>T</u>ools

- <u>S</u>pelling (▸)
  - <u>S</u>pelling... `F7` → `tools.spelling` 🖼
  - Spelling <u>O</u>ptions... → `review.spelling.options.dlg`
- ———
- Doc<u>u</u>ment Management... → `view.task.pane.document.management`
- Trac<u>k</u> Markup → `format.set.add.markup` 🖼
- ———
- Snap & <u>G</u>lue... → `tools.snap_glue_setup`
- <u>R</u>uler & Grid... → `view.ruler.grid.dlg`
- Color <u>P</u>alette... → `tools.options.color.palette.dlg`
- Shap<u>e</u>Studio... → `shape.studio.add.on` *(hidden)*
- ———
- <u>M</u>acro (▸)
  - <u>M</u>acros... `Alt+F8` → `tools.macro.dlg` 🖼
  - <u>R</u>ecord New Macro... → `developer.record.new.macro` 🖼
  - Stop <u>R</u>ecording → `tools.stop.recording.macro` 🖼
  - ———
  - <u>V</u>isual Basic Editor `Alt+F11` → `tools.run.vbe` 🖼
  - ———
  - *[list: macros]*
- A<u>d</u>d-Ons (▸) → `developer.run.addon.menu`
  - <u>R</u>un Add-On... → `developer.run.add.on.dlg`
  - ———
  - *[list: add_ons]*
- <u>A</u>utoCorrect Options... → `tools.mso.auto.correct.dlg` 🖼
- <u>T</u>rust Center... → `app.trust.center.dlg`
- <u>C</u>ustomize... → `view.toolbars.dlg`
- <u>O</u>ptions... → `tools.options.preferences`

### <u>D</u>ata

- <u>S</u>hape Data... → `format.cust.prop.edit` 🖼
- ———
- <u>L</u>ink Data to Shapes... → `data.link.data.to.shapes` 🖼
- <u>A</u>utomatically Link... → `data.automatically.link` 🖼
- <u>R</u>efresh Data... → `data.refresh` 🖼
- ———
- <u>D</u>isplay Data on Shapes... → `view.task.pane.data.graphic` 🖼
- ———
- Insert <u>P</u>ivotDiagram... → `data.insert_pivotdiagram` *new*
- I<u>n</u>sert Data Solutions (▸)
  - <u>O</u>rganization Chart... → `data.insert_data_solutions.organization_chart` *new*
  - <u>T</u>imeline... → `data.insert_data_solutions.timeline` *new*
  - <u>C</u>alendar... → `data.insert_data_solutions.calendar` *new*
  - <u>G</u>antt Chart... → `data.insert_data_solutions.gantt_chart` *new*
- ———
- Repor<u>t</u>s... → `tools.inventory`

### <u>S</u>hape

- ———
- <u>G</u>rouping (▸)
  - <u>G</u>roup `Shift+Ctrl+G` → `arrange.group` 🖼
  - <u>U</u>ngroup `Shift+Ctrl+U` → `arrange.ungroup` 🖼
  - ———
  - <u>A</u>dd to Group → `arrange.object.add.to.group`
  - <u>R</u>emove from Group → `arrange.object.remove.from.group`
  - <u>C</u>onvert to Group → `arrange.object.convert.to.group`
- <u>O</u>rder (▸)
  - Bring to <u>F</u>ront → `arrange.bring_to_front` 🖼
  - Send to <u>B</u>ack → `arrange.send_to_back` 🖼
  - Bring F<u>o</u>rward → `arrange.bring_forward` 🖼
  - Send B<u>a</u>ckward → `arrange.send_backward` 🖼
- <u>R</u>otate or Flip (▸)
  - Rotate <u>L</u>eft `Ctrl+L` → `arrange.object.rotate90` 🖼
  - <u>R</u>otate Right `Ctrl+R` → `arrange.rotate90.clockwise` 🖼
  - Ro<u>t</u>ate Text → `text.rotate90` 🖼
  - ———
  - Flip <u>H</u>orizontal `Ctrl+H` → `arrange.flip_horizontal` 🖼
  - Flip <u>V</u>ertical `Ctrl+J` → `arrange.flip_vertical` 🖼
- Operatio<u>n</u>s (▸)
  - Upd<u>a</u>te Alignment Box → `edit.recalc.object.wh`
  - <u>R</u>everse Ends → `arrange.object.swap.ends`
  - ———
  - <u>U</u>nion → `arrange.object.union` 🖼
  - <u>C</u>ombine → `arrange.object.combine` 🖼
  - <u>F</u>ragment → `arrange.object.fragment` 🖼
  - <u>I</u>ntersect → `arrange.intersect` 🖼
  - <u>S</u>ubtract → `arrange.subtract` 🖼
  - ———
  - <u>J</u>oin → `arrange.join` 🖼
  - <u>T</u>rim → `arrange.trim` 🖼
  - <u>O</u>ffset... → `arrange.offset.dlg` 🖼
  - ———
  - Fit Curv<u>e</u>... → `tools.dlg.custom.fit`
- ———
- <u>A</u>lign Shapes... → `arrange.align_objects` 🖼
- <u>D</u>istribute Shapes... → `arrange.object.distribute.dlg` 🖼
- <u>C</u>onnect Shapes → `arrange.object.connect.objects` 🖼
- Configure <u>L</u>ayout... → `tools.layout.shapes.dlg` 🖼
- R<u>e</u>-layout Shapes → `arrange.relayout.shapes`
- Center Dra<u>w</u>ing → `view.center.drawing`
- ———
- Ac<u>t</u>ions → `shape.actions` *(hidden)*

### <u>W</u>indow

- <u>N</u>ew Window → `view.window.new.window` 🖼
- <u>T</u>ile → `view.tile_all` 🖼
- <u>C</u>ascade → `view.cascade_all` 🖼
- ———
- <u>S</u>how ShapeSheet → `view.window.show.shape.sheet` 🖼
- ———
- *[list: windows]*

### <u>H</u>elp

- Larrygram <u>H</u>elp → `help.contents` 🖼 *(hidden)*
- <u>D</u>eveloper Reference → `developer.prog.ref.help` 🖼
- ———
- <u>S</u>ample Diagrams → `help.sample_diagrams`
- ———
- <u>A</u>bout Larrygram → `help.about.visio` *(hidden)*
- ———
- *[list: hyperlinks]*

## Menu bar: ShapeSheet menu bar (shown in `shapesheet_window`)

### <u>F</u>ile

- <u>N</u>ew (▸) 🖼
  - <u>G</u>etting Started... → `file.create.new.drawing`
  - <u>N</u>ew Drawing `Ctrl+N` → `file.new.blank.drawing` 🖼
  - N<u>e</u>w Drawing (Metric) → `file.new.blank.drawing.metric` 🖼
  - N<u>e</u>w Drawing (US Units) → `file.new.blank.drawing.us` 🖼
  - New Drawing from <u>T</u>emplate... → `file.new` 🖼
  - ———
  - *[list: template_categories]*
- <u>O</u>pen... `Ctrl+O` → `file.open` 🖼
- <u>C</u>lose → `file.close` 🖼
- ———
- <u>S</u>ave `Ctrl+S` → `file.save` 🖼
- S<u>a</u>ve As... → `file.save_as`
- Save as Web Pa<u>g</u>e... → `file.save.as.web.page` 🖼
- ———
- Ch<u>e</u>ck In → `file.checkin`
- Ch<u>e</u>ck Out → `file.checkout`
- Discard Chec<u>k</u> Out → `file.undo.checkout`
- ———
- Shap<u>e</u>s (▸)
  - <u>M</u>y Shapes (▸)
    - *[list: my_shapes]*
    - ———
    - <u>O</u>rganize My Shapes → `file.shapes.organize_my_shapes`
  - ———
  - *[list: stencil_categories]*
  - ———
  - <u>O</u>pen Stencil... → `file.open.stencil` 🖼
  - <u>N</u>ew Stencil → `file.new.blank.stencil` 🖼
  - N<u>e</u>w Stencil (Metric) → `file.new.blank.stencil.metric` 🖼
  - N<u>e</u>w Stencil (US Units) → `file.new.blank.stencil.us` 🖼
  - Show <u>D</u>ocument Stencil → `view.window.show.master.objects`
- Propert<u>i</u>es → `file.summary.info.dlg`
- ———
- *[list: recent_files]*
- ———
- E<u>x</u>it → `file.exit`

### <u>E</u>dit

- <u>U</u>ndo `Ctrl+Z` → `edit.undo` 🖼
- <u>R</u>edo `Ctrl+Y` → `edit.redo` 🖼
- ———
- Cu<u>t</u> `Ctrl+X` → `edit.cut` 🖼
- <u>C</u>opy `Ctrl+C` → `edit.copy` 🖼
- <u>P</u>aste `Ctrl+V` → `edit.paste` 🖼
- Cle<u>a</u>r `Delete` → `edit.clear`
- ———
- D<u>e</u>lete Section → `edit.sw.delete.section`
- <u>D</u>elete Row → `edit.sw.delete.row`
- E<u>x</u>pand Row → `edit.sw.expand.row`
- ———
- C<u>h</u>ange Row Type → `edit.sw.change.row.type.dlg`
- Acti<u>o</u>n... → `edit.action` *new*

### <u>V</u>iew

- <u>V</u>alues → `edit.sw.show.values`
- <u>F</u>ormulas → `edit.sw.show.formulas`
- ———
- Sectio<u>n</u>s... → `view.sections` *new*
- ———
- <u>S</u>tyle Explorer Window → `shape.drawing.explorer` 🖼
- Fo<u>r</u>mula Tracing Window → `view.formula_tracing_window` *new* 🖼
- ———
- <u>T</u>oolbars (▸)
  - Standard → `view.toolbar.standard` *new*
  - Formatting → `view.toolbar.formatting` *new*
  - Action → `view.toolbar.action` *new*
  - Data → `view.toolbar.data` *new*
  - Developer → `view.toolbar.developer` *new*
  - Drawing → `view.toolbar.drawing` *new*
  - Format Shape → `format.shape.pane`
  - Format Text → `view.toolbar.format_text` *new*
  - Ink → `tools.ink.tool`
  - Layout & Routing → `view.toolbar.layout_and_routing` *new*
  - Picture → `view.toolbar.picture` *new*
  - Reviewing → `view.toolbar.reviewing` *new*
  - Snap & Glue → `tools.snap_glue_setup`
  - Stencil → `view.toolbar.stencil` *new*
  - View → `view.toolbar.view` *new*
  - Web → `tools.inet.visio.on.the.web`
  - ———
  - <u>C</u>ustomize... → `view.toolbar.customize` *new*

### <u>I</u>nsert

- <u>S</u>ection... → `insert.section` *new*
- <u>R</u>ow → `insert.row` *new*
- Row <u>A</u>fter → `insert.row_after` *new*
- ———
- <u>N</u>ame... → `insert.name` *new*
- <u>F</u>unction... → `view.ss.window.paste.function`
- ———
- <u>H</u>yperlinks... `Ctrl+K` → `insert.hyper.link` 🖼

### F<u>o</u>rmat

- <u>T</u>ext... → `format.all.text.props` 🖼
- <u>L</u>ine... → `format.line` 🖼
- <u>F</u>ill... → `format.fill` 🖼
- ———
- Pict<u>u</u>re... → `insert.image.properties.dlg` 🖼
- I<u>n</u>k... → `format.ink.dlg`
- ———
- <u>P</u>rotection... → `format.protection`
- <u>B</u>ehavior... → `format.behavior`
- Sp<u>e</u>cial... → `format.special`
- ———
- <u>S</u>tyle... → `format.style`
- ———
- Laye<u>r</u>... → `layer.dlg`

### <u>T</u>ools

- ———
- <u>M</u>acro (▸)
  - <u>M</u>acros... `Alt+F8` → `tools.macro.dlg` 🖼
  - <u>R</u>ecord New Macro... → `developer.record.new.macro` 🖼
  - Stop <u>R</u>ecording → `tools.stop.recording.macro` 🖼
  - ———
  - <u>V</u>isual Basic Editor `Alt+F11` → `tools.run.vbe` 🖼
  - ———
  - *[list: macros]*
- Add-o<u>n</u>s (▸) → `developer.run.addon.menu`
  - <u>R</u>un Add-On... → `developer.run.add.on.dlg`
  - ———
  - *[list: add_ons]*
- ———
- <u>F</u>ormula Tracing (▸)
  - ShapeSheet Fo<u>r</u>mula Tracing Window → `tools.formula_tracing.shapesheet_formula_tracing_window` *new* 🖼
  - Trace <u>D</u>ependents → `tools.formula_tracing.trace_dependents` *new* 🖼
  - Trace <u>P</u>recedents → `tools.formula_tracing.trace_precedents` *new* 🖼
  - ———
  - <u>B</u>ack → `tools.formula_tracing.back` *new* 🖼
  - <u>F</u>orward → `tools.formula_tracing.forward` *new* 🖼
- ———
- C<u>u</u>stomize... → `view.toolbars.dlg`
- <u>O</u>ptions... → `tools.options.preferences`

### <u>D</u>ata

- Shape <u>D</u>ata... → `format.cust.prop.edit` 🖼

### <u>S</u>hape

- ———
- Rotate <u>L</u>eft `Ctrl+L` → `arrange.object.rotate90` 🖼
- <u>R</u>otate Right `Ctrl+R` → `arrange.rotate90.clockwise` 🖼
- Flip <u>V</u>ertical `Ctrl+J` → `arrange.flip_vertical` 🖼
- Flip <u>H</u>orizontal `Ctrl+H` → `arrange.flip_horizontal` 🖼
- Ro<u>t</u>ate Text → `text.rotate90` 🖼
- ———
- <u>U</u>pdate Alignment Box → `edit.recalc.object.wh`
- R<u>e</u>verse Ends → `arrange.object.swap.ends`

### <u>W</u>indow

- <u>N</u>ew Window → `view.window.new.window` 🖼
- <u>T</u>ile → `view.tile_all` 🖼
- <u>C</u>ascade → `view.cascade_all` 🖼
- ———
- *[list: windows]*

### <u>H</u>elp

- Larrygram <u>H</u>elp → `help.contents` 🖼
- <u>D</u>eveloper Reference → `developer.prog.ref.help` 🖼
- ———
- <u>S</u>ample Diagrams → `help.sample_diagrams`
- ———
- <u>A</u>bout Larrygram → `help.about.visio`
- ———
- *[list: hyperlinks]*

## Menu bar: Stencil menu bar (shown in `stencil_edit`)

### <u>F</u>ile

- <u>N</u>ew (▸) 🖼
  - <u>G</u>etting Started... → `file.create.new.drawing`
  - <u>N</u>ew Drawing `Ctrl+N` → `file.new.blank.drawing` 🖼
  - N<u>e</u>w Drawing (Metric) → `file.new.blank.drawing.metric` 🖼
  - N<u>e</u>w Drawing (US Units) → `file.new.blank.drawing.us` 🖼
  - New Drawing from <u>T</u>emplate... → `file.new` 🖼
  - ———
  - *[list: template_categories]*
- <u>O</u>pen... `Ctrl+O` → `file.open` 🖼
- <u>C</u>lose → `file.close` 🖼
- ———
- <u>S</u>ave `Ctrl+S` → `file.save` 🖼
- S<u>a</u>ve As... → `file.save_as`
- Save as Web Pa<u>g</u>e... → `file.save.as.web.page` 🖼
- ———
- Ch<u>e</u>ck In → `file.checkin`
- Ch<u>e</u>ck Out → `file.checkout`
- Discard Chec<u>k</u> Out → `file.undo.checkout`
- ———
- Shap<u>e</u>s (▸)
  - <u>M</u>y Shapes (▸)
    - *[list: my_shapes]*
    - ———
    - <u>O</u>rganize My Shapes → `file.shapes.organize_my_shapes`
  - ———
  - *[list: stencil_categories]*
  - ———
  - <u>O</u>pen Stencil... → `file.open.stencil` 🖼
  - <u>N</u>ew Stencil → `file.new.blank.stencil` 🖼
  - N<u>e</u>w Stencil (Metric) → `file.new.blank.stencil.metric` 🖼
  - N<u>e</u>w Stencil (US Units) → `file.new.blank.stencil.us` 🖼
  - Show <u>D</u>ocument Stencil → `view.window.show.master.objects`
- Propert<u>i</u>es → `file.summary.info.dlg`
- ———
- *[list: recent_files]*
- ———
- E<u>x</u>it → `file.exit`

### <u>E</u>dit

- <u>U</u>ndo `Ctrl+Z` → `edit.undo` 🖼
- <u>R</u>edo `Ctrl+Y` → `edit.redo` 🖼
- ———
- Cu<u>t</u> `Ctrl+X` → `edit.cut` 🖼
- <u>C</u>opy `Ctrl+C` → `edit.copy` 🖼
- <u>P</u>aste `Ctrl+V` → `edit.paste` 🖼
- Paste Sh<u>o</u>rtcut → `edit.paste_shortcut` *new*
- ———
- Cle<u>a</u>r `Delete` → `edit.clear`
- Se<u>l</u>ect All `Ctrl+A` → `edit.select_all`
- <u>D</u>uplicate `Ctrl+D` → `edit.duplicate`
- Create <u>S</u>hortcut → `edit.create_shortcut` *new*

### <u>V</u>iew

- Icons <u>a</u>nd Names → `shape.sten.icon.and.name` 🖼
- Names <u>U</u>nder Icons → `shape.sten.names.under.icons` 🖼
- <u>I</u>cons Only → `shape.sten.icon.only` 🖼
- <u>N</u>ames Only → `shape.sten.name.only` 🖼
- I<u>c</u>ons and Details → `shape.sten.icon.and.detail` 🖼
- ———
- <u>T</u>oolbars (▸)
  - Standard → `view.toolbar.standard` *new*
  - Formatting → `view.toolbar.formatting` *new*
  - Action → `view.toolbar.action` *new*
  - Data → `view.toolbar.data` *new*
  - Developer → `view.toolbar.developer` *new*
  - Drawing → `view.toolbar.drawing` *new*
  - Format Shape → `format.shape.pane`
  - Format Text → `view.toolbar.format_text` *new*
  - Ink → `tools.ink.tool`
  - Layout & Routing → `view.toolbar.layout_and_routing` *new*
  - Picture → `view.toolbar.picture` *new*
  - Reviewing → `view.toolbar.reviewing` *new*
  - Snap & Glue → `tools.snap_glue_setup`
  - Stencil → `view.toolbar.stencil` *new*
  - View → `view.toolbar.view` *new*
  - Web → `tools.inet.visio.on.the.web`
  - ———
  - <u>C</u>ustomize... → `view.toolbar.customize` *new*
- <u>D</u>rawing Explorer Window → `shape.drawing.explorer` 🖼

### <u>T</u>ools

- ———
- <u>M</u>acro (▸)
  - <u>M</u>acros... `Alt+F8` → `tools.macro.dlg` 🖼
  - <u>R</u>ecord New Macro... → `developer.record.new.macro` 🖼
  - Stop <u>R</u>ecording → `tools.stop.recording.macro` 🖼
  - ———
  - <u>V</u>isual Basic Editor `Alt+F11` → `tools.run.vbe` 🖼
  - ———
  - *[list: macros]*
- Add-o<u>n</u>s (▸) → `developer.run.addon.menu`
  - <u>R</u>un Add-On... → `developer.run.add.on.dlg`
  - ———
  - *[list: add_ons]*
- ———
- C<u>u</u>stomize... → `view.toolbars.dlg`
- <u>O</u>ptions... → `tools.options.preferences`

### <u>M</u>aster

- <u>N</u>ew Master... → `shape.sten.new.master`
- ———
- <u>M</u>aster Properties... → `shape.master.properties`
- <u>E</u>dit Master Shape → `shape.edit_master_shape` *new* 🖼
- ———
- Ed<u>i</u>t Icon Image → `shape.edit_icon_image` *new*
- <u>U</u>pdate Icon Image → `shape.update_icon_image` *new*

### <u>W</u>indow

- <u>N</u>ew Window → `view.window.new.window` 🖼
- <u>T</u>ile → `view.tile_all` 🖼
- <u>C</u>ascade → `view.cascade_all` 🖼
- ———
- <u>S</u>how Drawing Page... → `view.show_drawing_page` *new*
- ———
- *[list: windows]*

### <u>H</u>elp

- Larrygram <u>H</u>elp → `help.contents` 🖼
- <u>D</u>eveloper Reference → `developer.prog.ref.help` 🖼
- ———
- <u>S</u>ample Diagrams → `help.sample_diagrams`
- ———
- <u>A</u>bout Larrygram → `help.about.visio`
- ———
- *[list: hyperlinks]*

## Toolbars

### Standard (docked top, row 0; shown by default)

- New (split) → `file.new.blank.drawing` 🖼
  - Getting Started... → `file.create.new.drawing`
  - *[list: template_categories]*
- Open `Ctrl+O` → `file.open` 🖼
- Save `Ctrl+S` → `file.save` 🖼
- Mail Recipient (as Attachment)... → `file.send.as.mail` 🖼
- ———
- Print Page → `file.print.page` 🖼
- Print Preview `Ctrl+F2` → `file.print_preview` 🖼
- ———
- Spelling `F7` → `tools.spelling` 🖼
- ———
- Cut `Ctrl+X` → `edit.cut` 🖼
- Copy Drawing `Ctrl+C` → `edit.copy` 🖼
- Paste `Ctrl+V` → `edit.paste` 🖼
- Delete `Delete` → `edit.clear` 🖼
- Format Painter `Shift+Ctrl+P` → `tools.format_painter` 🖼
- ———
- Undo (split) → `edit.undo` 🖼
- Redo (split) → `edit.redo` 🖼
- ———
- Shapes (▸) 🖼
  - My Shapes (▸)
    - *[list: my_shapes]*
    - ———
    - Organize My Shapes → `file.shapes.organize_my_shapes`
  - ———
  - *[list: stencil_categories]*
  - ———
  - Open Stencil... → `file.open.stencil` 🖼
  - New Stencil → `file.new.blank.stencil` 🖼
  - New Stencil (Metric) → `file.new.blank.stencil.metric` 🖼
  - New Stencil (US Units) → `file.new.blank.stencil.us` 🖼
  - Show Document Stencil → `view.window.show.master.objects`
- ———
- Pointer Tool (split) → `tools.pointer` 🖼
  - Area Select → `tools.area.select` 🖼
  - Lasso Select → `tools.lasso.select.visio` 🖼
  - ———
  - Multiple Select → `tools.pointer_tool.multiple_select` *new* 🖼
- Connector Tool (split, remembers last) → `tools.connector` 🖼
  - Connector Tool → `tools.connector` 🖼
  - Connection Point Tool → `tools.connection.point` 🖼
- Text Tool (split, remembers last) → `tools.text` 🖼
  - Text Tool → `tools.text` 🖼
  - Text Block Tool → `tools.text_block` 🖼
- Drawing Tools → `shape.drawing.tools` 🖼
- ———
- AutoConnect (check) → `connector.auto_connect` 🖼
- Zoom (combo) → `shape.draw.zoom`
- ———
- Larrygram Help → `help.contents` 🖼

### Formatting (docked top, row 1; shown by default)

- Font (combo) → `text.font`
- Font Size (combo) → `text.font.size`
- ———
- Bold `Ctrl+B` → `format.text.bold` 🖼
- Italic `Ctrl+I` → `format.text.italic` 🖼
- Underline `Ctrl+U` → `format.text.underline` 🖼
- ———
- Align Left (check) `Shift+Ctrl+L` → `format.text.align_left` 🖼
- Align Center `Shift+Ctrl+C` → `format.text.align_center` 🖼
- Align Right `Shift+Ctrl+R` → `format.text.align_right` 🖼
- Change Text Direction → `format.change_text_direction` *new* 🖼 *(hidden)*
- Left-to-Right `Left Ctrl + Shift` → `text.direction.left.to.right` 🖼 *(hidden)*
- Right-to-Left `Right Ctrl + Shift` → `text.direction.right.to.left` 🖼 *(hidden)*
- ———
- Bullets → `text.bullets` 🖼
- Decrease Indent → `format.decrease.indent` 🖼
- Increase Indent → `format.increase.indent` 🖼
- ———
- Text Color (split) → `text.font.color` 🖼
  - Use Default → `format.text_color.use_default` *new*
  - ———
  - **Theme Colors**
  - *[list: theme_colors]*
  - *[list: theme_tints]*
  - ———
  - **Standard Colors**
  - *[list: standard_colors]*
  - ———
  - **Recent Colors**
  - *[list: recent_colors]*
  - More Text Colors... → `text.font.color`
- Line Color (split) → `format.set.line.color` 🖼
  - Use Default → `format.line_color.use_default` *new*
  - ———
  - No Line → `format.line_color.no_line` *new*
  - ———
  - **Theme Colors**
  - *[list: theme_colors]*
  - *[list: theme_tints]*
  - ———
  - **Standard Colors**
  - *[list: standard_colors]*
  - ———
  - **Recent Colors**
  - *[list: recent_colors]*
  - More Line Colors... → `format.line`
- Fill Color (split) → `format.set.fill.color` 🖼
  - Use Default → `format.fill_color.use_default` *new*
  - ———
  - No Fill → `format.fill_color.no_fill` *new*
  - ———
  - **Theme Colors**
  - *[list: theme_colors]*
  - *[list: theme_tints]*
  - ———
  - **Standard Colors**
  - *[list: standard_colors]*
  - ———
  - **Recent Colors**
  - *[list: recent_colors]*
  - More Fill Colors... → `format.fill`
- Theme → `format.theme` *new* 🖼
- ———
- Line Weight (split) → `format.set.line.weight` 🖼
  - No Line → `format.set.line.weight`
  - *[list: line_weights]*
  - ———
  - More Line Weights... `Shift+F3` → `format.line`
- Line Pattern (split) → `format.set.line.pattern` 🖼
  - No Line → `format.line_pattern.no_line` *new*
  - *[list: line_patterns]*
  - ———
  - More Line Patterns... `Shift+F3` → `format.line`
- Line Ends (split) → `format.set.line.ends` 🖼
  - No Line Ends → `format.line_ends.no_line_ends` *new*
  - *[list: line_ends]*
  - ———
  - More Line Ends... `Shift+F3` → `format.line`

### Action (docked top; hidden by default)

- Align Shapes... (split) → `arrange.align_objects` 🖼
  - Align Left → `arrange.align.left` 🖼
  - Align Center → `arrange.align.center` 🖼
  - Align Right → `arrange.align.right` 🖼
  - Align Top → `arrange.align.top` 🖼
  - Align Middle → `arrange.align.middle` 🖼
  - Align Bottom → `arrange.align.bottom` 🖼
- Distribute Shapes... (split) → `arrange.object.distribute.dlg` 🖼
  - Distribute Horizontal Spacing → `arrange.distribute_shapes.distribute_horizontal_spacing` *new* 🖼
  - Distribute Center → `arrange.distribute_shapes.distribute_center` *new* 🖼
  - Distribute Vertical Spacing → `arrange.distribute_shapes.distribute_vertical_spacing` *new* 🖼
  - Distribute Middle → `arrange.distribute_shapes.distribute_middle` *new* 🖼
- Connect Shapes → `arrange.object.connect.objects` 🖼
- Lay Out Shapes... → `tools.layout.shapes.dlg` 🖼
- ———
- Flip Horizontal `Ctrl+H` → `arrange.flip_horizontal` 🖼
- Flip Vertical `Ctrl+J` → `arrange.flip_vertical` 🖼
- Rotate Right `Ctrl+R` → `arrange.rotate90.clockwise` 🖼
- Rotate Left `Ctrl+L` → `arrange.object.rotate90` 🖼
- Rotate Text 90° → `text.rotate90` 🖼
- ———
- Bring to Front `Shift+Ctrl+F` → `arrange.bring_to_front` 🖼
- Send to Back `Shift+Ctrl+B` → `arrange.send_to_back` 🖼
- Group `Shift+Ctrl+G` → `arrange.group` 🖼
- Ungroup `Shift+Ctrl+U` → `arrange.ungroup` 🖼
- ———
- Shape Data... → `format.cust.prop.edit` 🖼

### Data (docked top; hidden by default)

- Shape Data → `format.cust.prop.edit` 🖼
- ———
- Link Data to Shapes → `data.link.data.to.shapes` 🖼
- Automatically Link → `data.automatically.link` 🖼
- Refresh All → `data.refresh_all` *new* 🖼
- ———
- Display Data on Shapes → `view.task.pane.data.graphic` 🖼

### Developer (docked top; hidden by default)

- Run Macro `Alt+F8` → `tools.macro.dlg` 🖼
- Record New Macro... → `developer.record.new.macro` 🖼
- Stop Recording → `tools.stop.recording.macro` 🖼
- Visual Basic Editor `Alt+F11` → `tools.run.vbe` 🖼
- Insert Control → `insert.control.dlg` 🖼
- Show ShapeSheet → `view.window.show.shape.sheet` 🖼
- Design Mode → `tools.design.mode` 🖼

### Drawing (docked floating; hidden by default)

- Rectangle Tool `Ctrl+8` → `shape.rectangle` 🖼
- Ellipse Tool `Ctrl+9` → `shape.ellipse` 🖼
- Line Tool `Ctrl+6` → `shape.line` 🖼
- Arc Tool `Ctrl+7` → `shape.arc` 🖼
- Freeform Tool `Ctrl+5` → `shape.spline` 🖼
- Pencil Tool `Ctrl+4` → `tools.pencil` 🖼

### Format Shape (docked top; hidden by default)

- Layer (dropdown) → `shape.layer.toolbar`
- Corner Rounding (split) → `format.set.line.corner.style` 🖼
  - *[list: corner_roundings]*
  - ———
  - More Corner Roundings... → `format.corners`
- Transparency (split) → `shape.transparency.dlg` 🖼
  - No Transparency → `format.transparency.no_transparency` *new*
  - *[list: transparency_levels]*
  - ———
  - More Transparency Levels... → `shape.transparency.dlg`
- Fill Pattern (split) → `format.set.fill.pattern` 🖼
  - No Fill → `format.set.fill.pattern`
  - *[list: fill_patterns]*
  - ———
  - More Fill Patterns... `F3` → `format.fill`
- Shadow Color (split) → `format.set.fill.shadow` 🖼
  - No Shadow → `format.set.fill.shadow`
  - *[list: shadow_styles]*
  - More Shadow Settings... → `format.shadow`
  - ———
  - Use Default → `format.set.fill.shadow`
  - ———
  - **Theme Colors**
  - *[list: theme_colors]*
  - *[list: theme_tints]*
  - ———
  - **Standard Colors**
  - *[list: standard_colors]*
  - ———
  - **Recent Colors**
  - *[list: recent_colors]*
  - More Shadow Colors... → `format.set.fill.shadow`

### Format Text (docked top; hidden by default)

- ———
- Increase Font Size `Shift+Ctrl+.` → `format.text.increase_size` 🖼
- Decrease Font Size `Shift+Ctrl+,` → `format.text.decrease_size` 🖼
- ———
- Strikethrough → `text.strike.through` 🖼
- Small Caps `Shift+Ctrl+K` → `format.text.small_caps` 🖼
- Superscript `Shift+Ctrl++` → `text.super.script` 🖼
- Subscript `Ctrl++` → `format.text.subscript` 🖼
- ———
- Align Bottom `Shift+Ctrl+V` → `format.text.align_bottom` 🖼
- Align Middle `Shift+Ctrl+M` → `format.text.align_middle` 🖼
- Align Top `Shift+Ctrl+T` → `format.text.align_top` 🖼
- ———
- Decrease Paragraph Spacing → `format.decrease.para.spacing` 🖼
- Increase Paragraph Spacing → `format.increase.para.spacing` 🖼

### Ink (docked floating; hidden by default)

- Ink Pen 1 → `tools.ink.stock.pen0` 🖼
- Ink Pen 2 → `tools.ink.stock.pen1` 🖼
- Ink Pen 3 → `tools.ink.ink_pen_3` *new* 🖼
- Ink Pen 4 → `tools.ink.ink_pen_4` *new* 🖼
- Ink Pen 5 → `tools.ink.ink_pen_5` *new* 🖼
- ———
- Eraser → `tools.ink.eraser` 🖼
- Close Ink Shape → `tools.ink.close_ink_shape` *new* 🖼
- ———
- Ink Color (▸) 🖼
  - *[list: ink_colors]*
- Ink Thickness (▸) 🖼
  - *[list: pen_widths]*

### Layout & Routing (docked top; hidden by default)

- Routing Style (palette) → `format.set.dyn.conn.routing.style`
- Straight Lines → `format.set.dyn.conn.appearance.straight` 🖼
- Curved Lines → `format.set.dyn.conn.appearance.curved` 🖼
- ———
- Reset Connector → `connector.dyn.conn.reroute` 🖼
- ———
- Reroute Freely → `format.set.dyn.conn.reroute.freely` 🖼
- Reroute on Crossover → `format.set.dyn.conn.reroute.on.crossover` 🖼
- Never Reroute → `format.set.dyn.conn.reroute.never` 🖼
- ———
- Line Jump Style (split, remembers last) → `connector.line_jump_style` *new* 🖼
  - Page Default → `connector.line_jump_style.page_default` *new* 🖼
  - Arc → `connector.line_jump_style.arc` *new* 🖼
  - Gap → `connector.line_jump_style.gap` *new* 🖼
  - Square → `connector.line_jump_style.square` *new* 🖼
  - 2 Sides → `connector.line_jump_style.sides_2` *new* 🖼
  - 3 Sides → `connector.line_jump_style.sides_3` *new* 🖼
  - 4 Sides → `connector.line_jump_style.sides_4` *new* 🖼
  - 5 Sides → `connector.line_jump_style.sides_5` *new* 🖼
  - 6 Sides → `connector.line_jump_style.sides_6` *new* 🖼
  - 7 Sides → `connector.line_jump_style.sides_7` *new* 🖼
- Add Line Jumps To (split, remembers last) → `connector.add_line_jumps_to` *new* 🖼
  - No Lines → `connector.add_line_jumps_to.no_lines` *new* 🖼
  - Horizontal Lines → `connector.add_line_jumps_to.horizontal_lines` *new* 🖼
  - Vertical Lines → `connector.add_line_jumps_to.vertical_lines` *new* 🖼
  - Last Routed Line → `connector.add_line_jumps_to.last_routed_line` *new* 🖼
  - Last Displayed Line → `connector.add_line_jumps_to.last_displayed_line` *new* 🖼
  - First Displayed Line → `connector.add_line_jumps_to.first_displayed_line` *new* 🖼
- Move Other Shapes Away on Drop → `format.set.page.plow` 🖼

### Picture (docked floating; hidden by default)

- Insert Picture → `file.import` 🖼
- ———
- Crop Tool `Shift+Ctrl+2` → `tools.crop` 🖼
- Rotate Left `Ctrl+L` → `arrange.object.rotate90` 🖼
- Line Weight (split) → `format.set.line.weight` 🖼
  - No Line → `format.set.line.weight`
  - *[list: line_weights]*
  - ———
  - More Line Weights... `Shift+F3` → `format.line`
- Format Picture → `insert.image.properties.dlg` 🖼
- Transparency (split) → `shape.transparency.dlg` 🖼
  - No Transparency → `format.transparency.no_transparency` *new*
  - *[list: transparency_levels]*
  - ———
  - More Transparency Levels... → `shape.transparency.dlg`

### Reviewing (docked top; hidden by default)

- Show/Hide Markup → `review.show_hide_markup` *new* 🖼
- ———
- Reviewers (▸) → `review.reviewer.visibility.all`
  - Show All → `review.reviewer.visibility.all`
  - Hide All → `review.reviewers.hide_all` *new*
  - ———
  - *[list: reviewers]*
- ———
- Insert Comment (split) → `insert.comment` 🖼
  - Insert Comment → `insert.comment` 🖼
  - Edit Comment → `review.insert_comment.edit_comment` *new* 🖼
  - Delete Comment → `edit.delete.comment` 🖼
- Ink Tool → `tools.ink.review.pen` 🖼
- ———
- Previous Markup → `view.previous.markup` 🖼
- Next Markup → `view.next.markup` 🖼
- Delete Markup (split) → `review.delete_markup` *new* 🖼
  - Delete Markup → `review.delete_markup.delete_markup` *new* 🖼
  - Delete Markup Overlays on Current Page (▸)
    - All on Current Page → `review.delete_markup.delete_markup_overlays_on_current_page.all_on_current_page` *new*
    - ———
    - *[list: markup_overlays]*
  - Delete All Markup Overlays in Document → `review.delete_markup.delete_all_markup_overlays_in_document` *new*
- ———
- Track Markup → `format.set.add.markup` 🖼
- ———
- Reviewing Pane → `view.task.pane.reviewer` 🖼

### Snap & Glue (docked top; hidden by default)

- Toggle Snap `Shift+F9` → `shape.draw.snap` 🖼
- Toggle Glue `F9` → `shape.draw.glue` 🖼
- Snap to Dynamic Grid → `connector.dynamic.grid` 🖼
- Snap to Drawing Aids → `tools.tool.snap.lines` 🖼
- ———
- Snap to Ruler Subdivisions → `view.rul.sub` 🖼
- Snap to Grid → `view.grid` 🖼
- Snap to Alignment Box → `arrange.align.box` 🖼
- Snap to Shape Intersections → `shape.intersect` 🖼
- Glue to Shape Geometry → `shape.geo` 🖼
- Glue to Guides → `view.guides` 🖼
- Glue to Shape Handles → `shape.hand` 🖼
- Glue to Shape Vertices → `shape.vert` 🖼
- Glue to Connection Points → `connector.conn.points` 🖼

### Stencil (docked top; hidden by default)

- New Stencil → `file.new.blank.stencil` 🖼
- Show Document Stencil → `view.window.show.master.objects` 🖼
- ———
- Icons and Names → `shape.sten.icon.and.name` 🖼
- Names Under Icons → `shape.sten.names.under.icons` 🖼
- Icons Only → `shape.sten.icon.only` 🖼
- Names Only → `shape.sten.name.only` 🖼
- Icons and Details → `shape.sten.icon.and.detail` 🖼

### View (docked top; hidden by default)

- Rulers (check) → `view.rulers` 🖼
- Grid (check) → `view.toggle_grid` 🖼
- Guides (check) → `view.toggle_guides` 🖼
- Connection Points (check) → `view.connections` 🖼
- Page Breaks (check) → `view.page.breaks` 🖼
- Shapes Window → `shape.shapes.window` 🖼
- ———
- Pan & Zoom Window → `view.pan.zoom` 🖼
- Shape Data Window → `tools.cust.prop` 🖼
- Size & Position Window → `arrange.size.pos` 🖼
- External Data Window → `tools.external.data.window` 🖼
- Drawing Explorer Window → `shape.drawing.explorer` 🖼
- Master Explorer Window → `shape.master.explorer` 🖼
- ———
- Layer Properties → `layer.setup.dlg` 🖼

### Web (docked top; hidden by default)

- Insert Hyperlink `Ctrl+K` → `insert.hyper.link` 🖼
- Back `Alt+Left` → `tools.inet.go.back` 🖼
- Forward `Alt+Right` → `tools.inet.go.forward` 🖼

### ShapeSheet Standard (docked top; shown by default; only in `shapesheet_window`)

- New (split) → `file.new.blank.drawing` 🖼
  - Getting Started... → `file.create.new.drawing`
  - *[list: template_categories]*
- Open `Ctrl+O` → `file.open` 🖼
- Save `Ctrl+S` → `file.save` 🖼
- E-mail → `file.send.as.mail` 🖼
- ———
- Cut `Ctrl+X` → `edit.cut` 🖼
- Copy `Ctrl+C` → `edit.copy` 🖼
- Paste `Ctrl+V` → `edit.paste` 🖼
- ———
- Undo (split) → `edit.undo`
- Redo (split) → `edit.redo`
- ———
- Shapes (▸) 🖼
  - My Shapes (▸)
    - *[list: my_shapes]*
    - ———
    - Organize My Shapes → `file.shapes.organize_my_shapes`
  - ———
  - *[list: stencil_categories]*
  - ———
  - Open Stencil... → `file.open.stencil` 🖼
  - New Stencil → `file.new.blank.stencil` 🖼
  - New Stencil (Metric) → `file.new.blank.stencil.metric` 🖼
  - New Stencil (US Units) → `file.new.blank.stencil.us` 🖼
  - Show Document Stencil → `view.window.show.master.objects`
- ———
- New Window → `view.window.new.window` 🖼
- Tile Windows `Shift+F7` → `view.tile_all` 🖼
- Cascade Windows `Alt+F7` → `view.cascade_all` 🖼
- ———
- Larrygram Help → `help.contents` 🖼

### ShapeSheet Formula Bar (docked top; shown by default; only in `shapesheet_window`)

- Cancel Change → `edit.sw.cancel` 🖼
- Accept → `edit.sw.accept` 🖼

### ShapeSheet Formula Tracing (docked top; shown by default; only in `shapesheet_window`)

- ShapeSheet Formula Tracing Window → `view.show.formula.tracing.window` 🖼
- ———
- Trace Dependents → `tools.formula_tracing.trace_dependents` *new* 🖼
- Trace Precedents → `tools.formula_tracing.trace_precedents` *new* 🖼
- ———
- Back → `tools.formula_tracing.back` *new* 🖼
- Forward → `tools.formula_tracing.forward` *new* 🖼

### Stencil Standard (docked top; shown by default; only in `stencil_edit`)

- New (split) → `file.new.blank.drawing` 🖼
  - Getting Started... → `file.create.new.drawing`
  - *[list: template_categories]*
- Open `Ctrl+O` → `file.open` 🖼
- Save `Ctrl+S` → `file.save` 🖼
- E-mail → `file.send.as.mail` 🖼
- ———
- Cut `Ctrl+X` → `edit.cut` 🖼
- Copy `Ctrl+C` → `edit.copy` 🖼
- Paste `Ctrl+V` → `edit.paste` 🖼
- ———
- Undo (split) → `edit.undo`
- Redo (split) → `edit.redo`
- ———
- Shapes (▸) 🖼
  - My Shapes (▸)
    - *[list: my_shapes]*
    - ———
    - Organize My Shapes → `file.shapes.organize_my_shapes`
  - ———
  - *[list: stencil_categories]*
  - ———
  - Open Stencil... → `file.open.stencil` 🖼
  - New Stencil → `file.new.blank.stencil` 🖼
  - New Stencil (Metric) → `file.new.blank.stencil.metric` 🖼
  - New Stencil (US Units) → `file.new.blank.stencil.us` 🖼
  - Show Document Stencil → `view.window.show.master.objects`
- ———
- New Window → `view.window.new.window` 🖼
- Tile Windows `Shift+F7` → `view.tile_all` 🖼
- Cascade Windows `Alt+F7` → `view.cascade_all` 🖼
- ———
- Larrygram Help → `help.contents` 🖼

### Stencil Format (docked top; shown by default; only in `stencil_edit`)

- New Stencil → `file.new.blank.stencil` 🖼
- ———
- Icons and Names → `shape.sten.icon.and.name` 🖼
- Names Under Icons → `shape.sten.names.under.icons` 🖼
- Icons Only → `shape.sten.icon.only` 🖼
- Names Only → `shape.sten.name.only` 🖼
- Icons and Details → `shape.sten.icon.and.detail` 🖼

## Context menus

### Shape: right-click a shape

- *[list: hyperlinks]*
- ———
- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- ———
- F<u>o</u>rmat (▸)
  - <u>T</u>ext... → `format.all.text.props` 🖼
  - <u>L</u>ine... → `format.line` 🖼
  - <u>F</u>ill... → `format.fill` 🖼
  - ———
  - <u>A</u>llow Themes → `tools.allow.themes`
  - Re<u>m</u>ove Theme → `edit.remove.theme.from.sel`
  - ———
  - Pict<u>u</u>re... → `insert.image.properties.dlg` 🖼
  - I<u>n</u>k... → `format.ink.dlg`
  - ———
  - <u>P</u>rotection... → `format.protection`
  - <u>B</u>ehavior... → `format.behavior`
  - Sp<u>e</u>cial... → `format.special`
  - ———
  - <u>S</u>tyle... → `format.style`
  - ———
  - Laye<u>r</u>... → `layer.dlg`
- <u>D</u>ata (▸)
  - <u>S</u>hape Data... → `format.cust.prop.edit` 🖼
  - ———
  - <u>E</u>dit Data Graphic... → `shape.data.edit_data_graphic` *new*
  - <u>R</u>emove Data Graphic → `edit.remove.data.graphic.from.sel`
  - ———
  - *[list: data_recordset_links]*
- Shap<u>e</u> (▸)
  - ———
  - <u>G</u>roup → `arrange.group` 🖼
  - <u>U</u>ngroup → `arrange.ungroup` 🖼
  - ———
  - Bring to <u>F</u>ront → `arrange.bring_to_front` 🖼
  - Send to <u>B</u>ack → `arrange.send_to_back` 🖼
  - Br<u>i</u>ng Forward → `arrange.bring_forward` 🖼
  - <u>S</u>end Backward → `arrange.send_backward` 🖼
  - ———
  - Flip <u>H</u>orizontal → `arrange.flip_horizontal` 🖼
  - Flip <u>V</u>ertical → `arrange.flip_vertical` 🖼
  - ———
  - Ro<u>t</u>ate Text → `text.rotate90` 🖼
  - Find Si<u>m</u>ilar Shapes → `shape.shape.find_similar_shapes` *new*
- ———
- R<u>i</u>ght-Angle Connector → `connector.right_angle` 🖼
- Strai<u>g</u>ht Connector → `connector.straight` 🖼
- Cur<u>v</u>ed Connector → `connector.curved` 🖼
- ———
- Reset Co<u>n</u>nector → `connector.dyn.conn.reroute` 🖼
- Convert <u>I</u>nk to Text → `shape.convert_ink_to_text` *new*
- Convert Ink to Geo<u>m</u>etry → `shape.convert_ink_to_geometry` *new*
- ———
- <u>H</u>elp → `help.contents` 🖼

### Page tab: right-click a page tab

- <u>I</u>nsert Page... → `tools.options.new.page`
- <u>D</u>elete Page → `page.delete` 🖼
- <u>R</u>ename Page → `page.rename`
- Reorder <u>P</u>ages... `Ctrl+Alt+P` → `page.reorder_pages` *new*

### Text editing: right-click inside text being edited

- ———
- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- ———
- Forma<u>t</u> Text... → `format.all.text.props` 🖼
- Insert <u>F</u>ield... → `edit.insert.field`
- Bullet<u>s</u> → `text.bullets` 🖼
- ———
- Recon<u>v</u>ert → `text.reconvert` *new*
- <u>C</u>hange Text Direction → `view.direction.toggle` 🖼
- H<u>o</u>rizontal in Vertical... → `text.horizontal_in_vertical` *new*
- ———
- Text <u>R</u>uler → `text.edit.ruler`

### Embedded object: right-click an embedded object

- *[list: hyperlinks]*
- ———
- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- ———
- <u>F</u>ormat (▸)
  - <u>T</u>ext... → `format.all.text.props` 🖼
  - <u>L</u>ine... → `format.line` 🖼
  - <u>F</u>ill... → `format.fill` 🖼
  - ———
  - <u>A</u>llow Themes → `tools.allow.themes`
  - Re<u>m</u>ove Theme → `edit.remove.theme.from.sel`
  - ———
  - Pict<u>u</u>re... → `insert.image.properties.dlg` 🖼
  - I<u>n</u>k... → `format.ink.dlg`
  - ———
  - <u>P</u>rotection... → `format.protection`
  - <u>B</u>ehavior... → `format.behavior`
  - Sp<u>e</u>cial... → `format.special`
  - ———
  - <u>S</u>tyle... → `format.style`
  - ———
  - Laye<u>r</u>... → `layer.dlg`
- <u>D</u>ata (▸)
  - <u>S</u>hape Data... → `format.cust.prop.edit` 🖼
  - ———
  - <u>E</u>dit Data Graphic... → `shape.data.edit_data_graphic` *new*
  - <u>R</u>emove Data Graphic → `edit.remove.data.graphic.from.sel`
  - ———
  - *[list: data_recordset_links]*
- Shap<u>e</u> (▸)
  - ———
  - <u>G</u>roup → `arrange.group` 🖼
  - <u>U</u>ngroup → `arrange.ungroup` 🖼
  - ———
  - Bring to <u>F</u>ront → `arrange.bring_to_front` 🖼
  - Send to <u>B</u>ack → `arrange.send_to_back` 🖼
  - Br<u>i</u>ng Forward → `arrange.bring_forward` 🖼
  - <u>S</u>end Backward → `arrange.send_backward` 🖼
  - ———
  - Flip <u>H</u>orizontal → `arrange.flip_horizontal` 🖼
  - Flip <u>V</u>ertical → `arrange.flip_vertical` 🖼
  - ———
  - Ro<u>t</u>ate Text → `text.rotate90` 🖼
  - Find Si<u>m</u>ilar Shapes → `shape.shape.find_similar_shapes` *new*
- ———
- <u>O</u>bject... → `edit.open.object`

### Hyperlink: right-click a hyperlink in text

- <u>O</u>pen → `insert.hyperlink.open` *new*
- Ope<u>n</u> in New Window → `insert.hyperlink.open_in_new_window` *new*
- ———
- <u>C</u>opy Hyperlink → `insert.hyperlink.copy_hyperlink` *new*
- <u>A</u>dd to Favorites → `insert.hyperlink.add_to_favorites` *new*
- <u>E</u>dit Hyperlink → `insert.hyperlink.edit_hyperlink` *new*

### Connection point: right-click a connection point with the connection point tool

- <u>I</u>nward → `edit.connection_point.inward` *new*
- <u>O</u>utward → `edit.connection_point.outward` *new*
- In<u>w</u>ard & Outward → `edit.connection_point.inward_and_outward` *new*

### Comment: right-click a comment mark

- <u>E</u>dit Comment → `review.edit_comment` *new* 🖼
- <u>D</u>elete Comment → `edit.delete.comment` 🖼

### Full screen: right-click in full-screen view

- <u>P</u>revious Page → `page.previous` 🖼
- <u>N</u>ext Page → `page.next` 🖼
- ———
- <u>G</u>o To (▸)
  - <u>F</u>oreground → `tools.options.edit.drawing`
  - <u>B</u>ackground → `tools.options.edit.background`
  - ———
  - *[list: pages]*
  - ———
  - <u>P</u>age... → `tools.options.go.to.drawing`
- <u>C</u>lose → `view.full_screen_mode`

### ShapeSheet: right-click in a ShapeSheet window

- <u>V</u>alues → `edit.sw.show.values`
- <u>F</u>ormulas → `edit.sw.show.formulas`
- ———
- View Sectio<u>n</u>s... → `edit.sw.show.sections.dlg`
- In<u>s</u>ert Section... → `edit.sw.add.section.dlg`
- D<u>e</u>lete Section → `edit.sw.delete.section`
- ———
- Inse<u>r</u>t Row → `edit.sw.insert.row`
- Insert Row <u>A</u>fter → `edit.sw.insert.row.after`
- <u>D</u>elete Row → `edit.sw.delete.row`
- E<u>x</u>pand Row → `edit.sw.expand.row`
- <u>C</u>hange Row Type → `edit.sw.change.row.type.dlg`

### Master editing: right-click the page of a master being edited

- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- Paste Sh<u>o</u>rtcut → `edit.paste.shortcut`
- ———
- Add to <u>M</u>y Shapes (▸)
  - *[list: my_shapes]*
  - ———
  - Add To <u>N</u>ew Stencil... → `shape.master.add_to_my_shapes.add_to_new_stencil` *new*
  - Add To <u>E</u>xisting Stencil... → `shape.master.add_to_my_shapes.add_to_existing_stencil` *new*
  - ———
  - <u>O</u>rganize My Shapes → `file.shapes.organize_my_shapes`
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
- <u>H</u>elp → `help.contents` 🖼

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
- <u>D</u>rawing Explorer Window → `shape.drawing.explorer` 🖼
- ———
- Co<u>l</u>lapse All Groups → `shape.stencil.collapse_all_groups` *new* 🖼
- E<u>x</u>pand All Groups → `shape.stencil.expand_all_groups` *new* 🖼
- ———
- <u>F</u>loat Window → `shape.stencil.float_window` *new*
- <u>C</u>lose → `shape.stencil.close` *new* 🖼

### Docked window title: right-click the title of a docked window

- <u>F</u>loat Window → `view.pane.float_window` *new*
- Auto<u>H</u>ide → `view.pane.autohide` *new*
- ———
- <u>C</u>lose → `view.pane.close` *new* 🖼

### Shape Data window: right-click in the Shape Data window

- <u>F</u>loat Window → `view.pane.float_window` *new*
- Auto<u>H</u>ide → `view.pane.autohide` *new*
- Auto<u>F</u>it → `view.pane.autofit` *new*
- ———
- <u>L</u>ink Data to Shapes... → `data.link.data.to.shapes` 🖼
- Di<u>s</u>play Data on Shapes... → `view.task.pane.data.graphic` 🖼
- <u>D</u>efine Shape Data... → `view.pane.define_shape_data` *new* 🖼
- Shape Da<u>t</u>a Sets... → `view.pane.shape_data_sets` *new*
- ———
- <u>C</u>lose → `view.pane.close` *new* 🖼

### Shapes window: right-click the Shapes window title

- <u>F</u>loat Window → `view.pane.float_window` *new*
- Auto<u>H</u>ide → `view.pane.autohide` *new*
- ———
- Icons and Na<u>m</u>es → `shape.sten.icon.and.name` 🖼
- Names <u>U</u>nder Icons → `shape.sten.names.under.icons` 🖼
- Ic<u>o</u>ns Only → `shape.sten.icon.only` 🖼
- <u>N</u>ames Only → `shape.sten.name.only` 🖼
- Icons and De<u>t</u>ails → `shape.sten.icon.and.detail` 🖼
- ———
- Se<u>a</u>rch Options... → `view.pane.search_options` *new*
- ———
- Clos<u>e</u> All Stencils → `view.pane.close_all_stencils` *new*
- Hide <u>W</u>indow → `view.pane.hide_window` *new* 🖼

### Size & Position window: right-click in the Size & Position window

- <u>F</u>loat Window → `view.pane.float_window` *new*
- Auto<u>H</u>ide → `view.pane.autohide` *new*
- Auto<u>F</u>it → `view.pane.autofit` *new*
- ———
- <u>C</u>lose → `view.pane.close` *new* 🖼

### External Data list: right-click a row in the External Data window

- Select <u>A</u>ll → `edit.select_all`
- ———
- Link to <u>S</u>elected Shapes → `data.external.link_to_selected_shapes` *new*
- <u>U</u>nlink → `data.external.unlink` *new*
- <u>L</u>inked Shapes → `data.external.linked_shapes` *new*
- Automatically Lin<u>k</u>... → `data.automatically.link`
- ———
- Re<u>f</u>resh Data... → `data.refresh`
- <u>C</u>onfigure Refresh... → `data.external.configure_refresh` *new*
- ———
- C<u>o</u>lumn Settings... → `data.external.column_settings` *new*
- A<u>r</u>range by → `data.external.arrange_by` *new*
- <u>D</u>ata Source (▸)
  - <u>A</u>dd... → `data.external.data_source.add` *new*
  - <u>R</u>ename → `data.external.data_source.rename` *new*
  - R<u>e</u>move → `data.external.data_source.remove` *new* 🖼
  - <u>P</u>roperties... → `data.external.data_source.properties` *new*
- ———
- <u>H</u>elp → `help.contents` 🖼

### External Data tab: right-click a tab in the External Data window

- <u>A</u>dd... → `data.external.add` *new*
- <u>R</u>ename → `data.external.rename` *new*
- R<u>e</u>move → `data.external.remove` *new* 🖼
- <u>P</u>roperties... → `data.external.properties` *new*
- ———
- <u>H</u>elp → `help.contents` 🖼

### Drawing Explorer: document: right-click the document in the Drawing Explorer

- Propert<u>i</u>es → `file.summary.info.dlg`
- ———
- <u>H</u>eader and Footer... → `file.header.footer`
- ———
- <u>C</u>olor Palette... → `tools.options.color.palette.dlg`
- <u>P</u>rotect Document... → `view.explorer.protect_document` *new*
- ———
- <u>S</u>how ShapeSheet → `view.show.shape.sheet.document` 🖼

### Drawing Explorer: pages: right-click the Pages folder

- <u>I</u>nsert Page... → `tools.options.new.page`
- <u>R</u>eorder Pages... → `view.explorer.reorder_pages` *new*

### Drawing Explorer: page: right-click a page

- Page Set<u>u</u>p... → `tools.options.page.setup`
- <u>R</u>uler & Grid... → `view.explorer.ruler_and_grid` *new*
- <u>D</u>elete Page → `page.delete` 🖼
- ———
- <u>S</u>how ShapeSheet → `view.show.shape.sheet.page` 🖼

### Drawing Explorer: layers: right-click the Layers folder

- <u>L</u>ayer Properties... → `layer.setup.dlg` 🖼

### Drawing Explorer: layer: right-click a layer

- <u>V</u>isible → `view.explorer.visible` *new*
- <u>A</u>ctive → `view.explorer.active` *new*
- <u>D</u>elete Layer → `view.explorer.delete_layer` *new* 🖼

### Drawing Explorer: shape: right-click a shape

- <u>S</u>hape Data... → `format.cust.prop.edit` 🖼
- <u>L</u>ayer... → `layer.dlg`
- <u>D</u>elete Shape → `view.explorer.delete_shape` *new* 🖼
- ———
- <u>S</u>how ShapeSheet → `view.show.shape.sheet.shape` 🖼

### Drawing Explorer: masters: right-click the Masters folder

- <u>N</u>ew Master... → `shape.sten.new.master`
- Show <u>D</u>ocument Stencil → `view.window.show.master.objects`

### Drawing Explorer: master: right-click a master

- Cu<u>t</u> → `edit.cut` 🖼
- <u>C</u>opy → `edit.copy` 🖼
- <u>P</u>aste → `edit.paste` 🖼
- Paste Sh<u>o</u>rtcut → `edit.paste.shortcut`
- ———
- De<u>l</u>ete → `view.explorer.delete` *new* 🖼
- <u>D</u>uplicate → `edit.duplicate`
- Create <u>S</u>hortcut → `view.explorer.create_shortcut` *new*
- ———
- <u>M</u>aster Properties... → `shape.master.properties`
- <u>E</u>dit Master Shape → `view.explorer.edit_master_shape` *new*
- Ed<u>i</u>t Icon Image → `view.explorer.edit_icon_image` *new*

### Drawing Explorer: styles: right-click the Styles folder

- <u>D</u>efine Styles... → `format.define.styles`

### Drawing Explorer: style: right-click a style

- D<u>e</u>fine Styles... → `format.define.styles`
- <u>D</u>elete Style → `view.explorer.delete_style` *new* 🖼
- ———
- <u>S</u>how ShapeSheet → `view.explorer.show_shapesheet` *new* 🖼

### Drawing Explorer: patterns: right-click a pattern folder

- <u>N</u>ew Pattern... → `view.explorer.new_pattern` *new*

### Master Explorer: document: right-click the document in the Master Explorer

- Propert<u>i</u>es → `file.summary.info.dlg`
- ———
- <u>S</u>how ShapeSheet → `view.master_explorer.show_shapesheet` *new* 🖼

### Master Explorer: masters: right-click a master

- Page Set<u>u</u>p... → `view.master_explorer.page_setup` *new*
- <u>R</u>uler & Grid... → `view.master_explorer.ruler_and_grid` *new*
- ———
- <u>S</u>how ShapeSheet → `view.master_explorer.show_shapesheet` *new* 🖼

