# <task-id>: <title>

- Role: <researcher | architect | implementer | test-engineer | reviewer | coverage-auditor | cleanroom-auditor> (read `.claude/agents/<role>.md`)
- Phase: <n>
- Depends on: <task ids merged to main first, or "none">
- Parallel-safe with: <task ids>
- Suggested model tier: <small | medium | large>

## Launch

Paste into a fresh session (any harness), from `/Users/dara/ferrogram`:

> You are working on Ferrogram. Read `AGENTS.md`, then `docs/tasks/<task-id>.md`, and do that task. Follow AGENTS.md section 2 for the worktree and branch. Stop when the Result section is filled and the gate is green.

## Owns

Files/crates this task may edit. Nothing else.

## Inputs

## Goal

## Interfaces

Types, functions, schemas this task provides or consumes.

## Acceptance criteria

- [ ] ...
- [ ] `cargo xtask gate` green in the worktree.

## Result

_Filled in by the worker at hand-off: what changed, deviations, new dependencies, gate summary, open issues._
