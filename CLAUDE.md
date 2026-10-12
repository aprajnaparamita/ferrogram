@AGENTS.md

# Project summary

Ferrogram: clean-room, pure-Rust, Visio-compatible diagramming app. Alpha MVP goal: open, render, edit and save .vsdx diagrams with a ribbon UI generated from Microsoft's published control-identifier data (switchable Visio 2016 / Microsoft 365 layouts). Read-only .vsd import via `crates/vsd-binary` (MPL-2.0, libvisio-derived). See `KICKOFF.md` for the full plan and `docs/STATUS.md` for where we are.

Claude Code sessions: the sub-agent role files in `.claude/agents/` double as role descriptions for workers in other harnesses. The master session does not spawn sub-agents; the user launches each task session from `docs/tasks/<task-id>.md`.
