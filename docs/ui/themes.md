# Themes

ADR 0063 records the decision, and the token files are
`ui-spec/themes/panel-classic.yaml` and `ui-spec/themes/panel-modern.yaml`.

## Structure

```yaml
schema_version: 1
id: panel-modern
name: Panel Modern (white)
fonts:   {role: {family, size_px, transform?}}
colors:  {token: '#rrggbb'}
metrics: {token: px | {width, height, ...}}
contextual_tab_sets: {set: colour family}   # Panel Modern only
```

## Mapping to egui

| Token | egui |
|---|---|
| `chrome.background` | `Visuals::panel_fill` for the chrome panels |
| `chrome.text` | `Visuals::override_text_color` |
| `chrome.text_disabled` | the weak text colour for disabled widgets |
| `button.hover.fill`, `button.hover.border` | `widgets.hovered.weak_bg_fill` and `bg_stroke` |
| `button.pressed.fill`, `button.pressed.border` | `widgets.active` and the selected-state fill |
| `menu.background`, `menu.border` | `Visuals::window_fill` and `window_stroke` for menus and popups |
| `tooltip.background`, `tooltip.border` | the tooltip frame |
| `field.background` | `Visuals::extreme_bg_color` (text edits, combos) |
| `canvas.background`, `page.*`, `ruler.*` | read by the canvas (`crates/ui/src/canvas.rs`) |
| everything else | read by the widget that draws it |

## Fonts

- The UI font for every role is Selawik, bundled in `crates/text/fonts`
  under the OFL. A 12 px size matches the 9 pt UI text of desktop apps
  at 96 DPI.
- The Panel Modern pane title uses Selawik Light at 22 px.
- Panel Modern tab captions are upper-cased at draw time
  (`transform: uppercase`); the label data stays in title case.

## Provisional tokens

Lines marked `# provisional` are design guesses that still need a
visual review: hover colours, pressed borders and tooltip borders.
Change them freely, then regenerate the golden PNGs.
