# 0004: Multi-agent workflow on a shared local repository

- Status: accepted
- Date: 2026-10-12
- Task: P0

## Context

Work is done by several agents, on different models and harnesses (Claude Code, pi). The user launches each session with a task prompt. All share one local repository. The main drive is small; an external drive is available at /Volumes/Dara.

## Decision

- The master agent plans phases, writes task files `docs/tasks/<task-id>.md`, reviews and merges. It does not spawn sub-agents; the user launches each task session, pointing it at its task file.
- Task ids: `P<phase>-<nn>`. Each task names a role (a file in `.claude/agents/`, readable by any harness), owned crates/files, inputs, outputs, interfaces and acceptance criteria.
- Each task runs in a git worktree at `/Volumes/Dara/ferrogram/worktrees/<task-id>` on branch `agent/<task-id>`, with `CARGO_TARGET_DIR=/Volumes/Dara/ferrogram/target/<task-id>`.
- Workers never commit to `main`. They rebase on `main`, fill the task's Result section, and stop. The master reviews (with reviewer/auditor tasks for phase gates) and merges.
- Tasks that may run in parallel own disjoint crates/files; the master merges them sequentially and re-runs the gate after each merge.
- `docs/STATUS.md` is the task board; only the master edits it on `main`.

## Consequences

- Task files must be self-contained: a fresh agent on any model must be able to start from AGENTS.md + the task file alone.
- Review is a bottleneck by design.

## Alternatives considered

- In-session sub-agents only: excludes non-Claude harnesses and model choice per task.
- Full clones per agent: more disk, slower; worktrees share the object store.
