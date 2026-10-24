# 0026: The Phase 3 CLI

- Status: accepted
- Date: 2026-10-14
- Task: P3-01

## Context

KICKOFF's Phase 3 adds CLI commands `inspect`, `render` and `export` to
`apps/larrygram-cli` (L7). The CLI is the first consumer of the reader and
the renderer and the test harness for the phase's exit criterion ("every
corpus file renders without panics"). AGENTS.md section 6 requires that
every user action is a command with an id in one registry
(`crates/commands`, Phase 4); the CLI dispatches through library calls now
and must move onto the registry without churn.

## Decision

### Commands

```
larrygram-cli inspect <file> [--json] [--page N]
larrygram-cli render  <file> --out <path> [--page N] [--format png|svg]
                      [--scale PPI] [--background #RRGGBB|none] [--strict]
larrygram-cli export  <file> --out <dir> [--format png|svg] [--scale PPI]
                      [--strict]
```

- `inspect` prints a document summary to stdout: kind, pages (name, size in
  inches, shape count), masters, styles, themes, and the rendering warning
  count of each page. `--json` prints the same as one JSON object
  (serde_json, stable key order); without it, a compact text form.
  `--page N` limits the summary to one page (1-based).
- `render` renders one page (default: the first) to `--out`. Format is
  taken from the output extension, overridable with `--format`.
- `export` renders every page of the document into `--out` as
  `page-<N>.<ext>`, skipping background-only pages' names from clashing by
  numbering in document order.
- `--scale` is pixels per inch (default 96, ADR 0025).
- `--background` overrides the page background for `render` (`none` =
  transparent).
- `--strict` makes warnings exit with the warnings code (ADR 0021).

### Argument parsing: `lexopt`

`lexopt` (MIT OR Apache-2.0) is a small, pure-Rust parser. `clap` was
rejected: a large dependency tree for three subcommands, and its derive
help text is not ours (AGENTS.md section 4: write our own descriptions).
Help and usage text are written by us (`--help` prints it; an unknown or
missing argument prints usage to stderr).

### Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | usage error (bad arguments, `--help` also exits 0) |
| 2 | input error (file not found, unreadable package, parse failure) |
| 3 | render or output error |
| 4 | warnings occurred and `--strict` is set |

Errors print one line to stderr: `error: <message>`. Warnings print one
line each to stderr (ADR 0021). Nothing writes to stdout except `inspect`
output.

### Programmatic shape and the Phase 4 move

Parsing produces a `Command` value; `run` executes it through library calls
(`larrygram_vsdx::read_document`, `larrygram_render::build_scene`,
`render_png`/`render_svg`). The `Command` enum is the seam: in Phase 4 each
variant maps to a registry command id (`inspect` → `file.inspect`,
`render` → `file.render`, `export` → `file.export`) with the same
arguments, dispatched through `crates/commands`; the parser, the exit codes
and the printed output do not change. Programmatic calls (JSON control
channel, MCP) call the registry, never the CLI's `main` (AGENTS.md
section 6).

## Consequences

- The phase's corpus test can shell out to the CLI or call `run` in
  process; both use the same code path.
- Two crates' worth of dependency (`lexopt`, `serde_json`) in an app crate;
  nothing in L0..L6 gains a dependency (layers stay green).
- Help text is ours and small; no generated completions in the alpha.

## Alternatives considered

- **`clap`.** Rejected: heavy tree, generated help text; revisit if the
  CLI grows past a handful of options.
- **Subcommand `run-command` now.** Rejected: it needs the Phase 4
  registry; the name stays reserved in the crate description.
- **Warnings as JSON on stdout.** Rejected: stdout is the data channel for
  `inspect --json`; diagnostics go to stderr.
- **One `--output-format` for all commands.** Rejected: `render` writes one
  file whose extension already names the format.
