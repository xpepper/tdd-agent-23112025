# Autonomous Multi-Agent TDD Machine

`tdd-cli` orchestrates three agents (Tester → Implementor → Refactorer)
to evolve a kata repository in strict red–green–refactor loops. The
tool keeps all state in git, writes per-step plans/logs under `.tdd/`,
and enforces the safety gate defined in
`.specify/memory/constitution.md`.

## Prerequisites

- Rust (stable toolchain) and Cargo installed
- `git` available in `PATH`
- OpenAI-compatible endpoint + API key (or use the mock client in
  tests)

## Quick Start

All commands run from the repository root:

```bash
cargo run -p tdd-cli -- init
cargo run -p tdd-cli -- run --steps 3
cargo run -p tdd-cli -- status
```

1. **Initialize**: `tdd-cli init` scaffolds `kata.md`, `tdd.yaml`, and
   `.tdd/{plan,logs}/`, then initializes git (or reuses the existing
   repo).
2. **Run multiple steps**: `tdd-cli run --steps N` loads `tdd.yaml`,
   spins up the orchestrator, and executes the next `N` Tester →
   Implementor → Refactorer steps, committing after each successful
   cycle.
3. **Run a single step**: `tdd-cli step` is a convenience alias for
   `run --steps 1`.
4. **Inspect status**: `tdd-cli status` reports the next role/step,
   last commit summary, and CI exit codes recorded in
   `.tdd/logs/step-XXX-role.json`.

## Configuration (`tdd.yaml`)

| Section | Required keys | Description |
|---------|---------------|-------------|
| `workspace` | `kata_file`, `plan_dir`, `log_dir`, `max_steps`, `max_attempts_per_agent` | File locations and guard rails for the loop. |
| `roles` | `tester`, `implementor`, `refactorer` each with `model`, optional `temperature` | Per-role LLM settings. |
| `llm` | `base_url`, `api_key_env` | OpenAI-compatible endpoint and env var name for the API key. |
| `ci` | `fmt`, `check`, `test` (arrays) | Commands run after each edit before committing. |
| `commit_author` | `name`, `email` | Identity used when the tool creates commits. |

Update the YAML file to match your environment (e.g., change CI command
arrays or model names). The CLI validates these values on every run.

## Logs and Plans

- Plans: `.tdd/plan/step-XYZ-role.md` capture each agent’s reasoning.
- Logs: `.tdd/logs/step-XYZ-role.json` store files touched, commit id,
  notes, and CI exit codes. `tdd-cli status` reads the latest entry to
  summarize progress.

## Development Workflow & Safety Gate

We follow the **TDD Agent Constitution**:

1. Clarify the next small behavior change.
2. Write or update tests to express it.
3. Make the tests pass with the simplest change.
4. Refactor while keeping the suite green.
5. Run `cargo fmt`, `cargo clippy -D warnings`, and `cargo test --all`.
6. Commit a focused, reversible change.

Readable code, consistent quality, and small commits are mandatory.
Before every commit, the safety gate requires a clean `cargo test`,
`cargo fmt`, and linting run.

For the full constitution, see
`.specify/memory/constitution.md`.
