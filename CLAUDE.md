@AGENTS.md

# Project summary

Larrygram: clean-room, pure-Rust, Visio-compatible diagramming app. Alpha MVP goal: open, render, edit and save .vsdx diagrams with a command bar UI generated from Microsoft's published control-identifier data (switchable Panel (2016 data) and Panel (365 data) layouts). Read-only .vsd import via `crates/vsd-binary` (MPL-2.0, libvisio-derived). See `KICKOFF.md` for the full plan and `docs/STATUS.md` for where we are.

Claude Code sessions: the sub-agent role files in `.claude/agents/` double as role descriptions for workers in other harnesses. The master session does not spawn sub-agents; the user launches each task session from `docs/tasks/<task-id>.md`.
