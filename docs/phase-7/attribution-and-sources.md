# Rows to add for Phase 7

P7-01 adds these rows in the same commit as the files they describe
(AGENTS.md section 4). Renumber the source if source 21 is already
taken by then.

## `ATTRIBUTION.md`

| Path | Source | License | Notes |
|---|---|---|---|
| `crates/ui/assets/icons/tabler/*.svg` | Tabler Icons 3.49.0, npm package `@tabler/icons`, `icons/outline/` (source 21 in `docs/sources.md`, task P7-01) | MIT (`crates/ui/assets/icons/tabler/LICENSE`) | Unmodified upstream SVG files, 245 icons named in `ui-spec/icons.yaml`. Recoloured at run time only (`currentColor` replaced by a theme token, ADR 0062). |
| `crates/ui/assets/icons/tabler/LICENSE` | Tabler Icons 3.49.0, `LICENSE` (source 21) | MIT | The licence text as distributed in the package (Copyright (c) 2020-2026 Paweł Kuna). |
| `ui-spec/layouts/*.yaml`, `ui-spec/contexts.yaml`, `ui-spec/icons.yaml`, `ui-spec/themes/*.yaml`, `ui-spec/layout-v2.schema.json` | Larrygram layout and theme data, compiled by the project (task P7-01, ADRs 0061–0064) | MIT (project license) | Short UI labels, structure, metrics, colour values and our own icon choices. No third-party images or long-form text. |

## `docs/sources.md`

| # | Source | URL | Retrieved | Version / commit | License / terms | Committed derived data |
|---|---|---|---|---|---|---|
| 21 | Tabler Icons (open icon set) | https://tabler.io/icons and https://registry.npmjs.org/@tabler/icons/-/icons-3.49.0.tgz | 2026-10-23 | 3.49.0 (published 2026-10-05) | MIT | 245 unmodified outline SVGs and the licence file under `crates/ui/assets/icons/tabler/` |
