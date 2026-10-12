# P1-03 review (master)

- Branch reviewed: `agent/P1-03` at `ff42c29`
- Verdict: **request changes** (small). The extractors, the diff, the
  parity matrix and the tests are good. The fixes are dependency
  alignment and a check that the output loads.
- Fix on the same branch (`agent/P1-03`, same worktree). First run
  `git rebase main`. `main` has moved: P1-06 (ui-spec loader) and
  P1-07 (oracle) are merged, so expect a `Cargo.lock` conflict.
  Regenerate the lock file instead of hand-merging it. Re-run the
  full gate. Append a "Review fixes" subsection to the Result in
  `docs/tasks/P1-03.md`.

## Major

**M1. Use the same `jsonschema` and `serde-saphyr` versions as `crates/ui-spec`.**
The branch adds `jsonschema = "0.18"` and `serde-saphyr = "0.0.16"`.
`main` already has `jsonschema` 0.58 (with `default-features = false`)
and `serde-saphyr` 1.3 in `crates/ui-spec`. Two versions of each are
now in `Cargo.lock`. `jsonschema` 0.18 with default features also
pulls in `reqwest` for remote `$ref`s, which we never use. The P1-02
review asked for a current `jsonschema`, not 0.18.

Fix:
- `xtask/Cargo.toml`: `jsonschema = { version = "0.58", default-features = false }`
  and `serde-saphyr = "1.3"`. Port `xtask/src/extract/schema.rs` to
  the 0.58 API (`jsonschema::draft7::new` / `validator_for`, and
  `Validator::iter_errors`).
- Check the result: `cargo tree -i jsonschema` and
  `cargo tree -i serde-saphyr` each show one version, and
  `cargo tree -i reqwest` finds nothing.

**M2. The committed layouts must load with the runtime loader.**
`crates/ui-spec` (`ferrogram_ui_spec::Loader`) is now on `main`. Add
an xtask test that does all of the following:
- copy the committed `ui-spec/panel-2016.yaml` and `panel-365.yaml`
  into a temp workspace;
- add an empty `commands.yaml` (`[]`) and an empty `shortcuts.yaml`
  (`[]`);
- call `Loader::load` for both `LayoutKind`s;
- assert success and the control counts from your Result.

`xtask` may depend on `ferrogram-ui-spec` (a path dev-dependency is
fine; xtask is outside the layer rules). This proves the extractor and
the loader agree on the format before P1-04 starts. If the loader
rejects something, report it under "Open issues" with the exact
error. Do not edit `crates/ui-spec`.

## Minor

- **m1.** Every `label` equals its Fluent id (e.g. `label: SaveDrawing`),
  because `visiocontrols.xlsx` has no display labels. That is fine for
  Phase 1, but say so explicitly in the Result and in a one-line note
  at the top of `docs/parity/controls.md`. Human labels come later,
  from our own command labels in `commands.yaml`.
- **m2.** Shorten the `fluentId` description in `ui-spec/schema.json`.
  Keep one sentence on the allowed characters plus the example
  `_3DRotationGallery`. Move the history ("a P1-03 correction...") to
  your Result.
- **m3.** The Result says "the schema's `oneOf` allows the transition
  from `null`". The schema root is now `anyOf` and `command` is
  `anyOf [commandId, null]` (`ddf98e7`). Correct the wording.

## Master does at merge

- Update the `FluentId` doc comment in `crates/ui-spec/src/lib.rs`
  (line 81) to the new pattern `^[A-Za-z_][A-Za-z0-9_]*$`.
