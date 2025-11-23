# Implementation Plan: Autonomous Multi-Agent TDD Machine

**Branch**: `001-tdd-agent-machine` | **Date**: 2025-11-23 | **Spec**: `specs/001-tdd-agent-machine/spec.md`
**Input**: Feature specification from `/specs/001-tdd-agent-machine/spec.md` and `initial-requirements.md`

## Summary

Build a Rust cargo workspace that provides a CLI (`tdd-cli`) orchestrating three agents (tester, implementor, refactorer) over a strict red–green–refactor loop for N steps. The tool runs locally, persists all state in git via conventional commits, consumes a kata description from Markdown, calls role-specific LLMs through an OpenAI-compatible API, and stores per-step plans and logs for auditability.

## Technical Context

**Language/Version**: Rust (stable, latest stable toolchain)
**Primary Dependencies**: `clap`, `serde`, `serde_yaml`, `serde_json`, `tokio`, `reqwest`, `git2`, `tempfile`, `anyhow`, `thiserror`, `walkdir`, `ignore`, `which`, `duct` or `tokio::process`
**Storage**: Local filesystem + git repository (via `git2`)
**Testing**: `cargo test`, `cargo clippy -D warnings` for linting, `cargo fmt` for formatting compliance
**Target Platform**: Developer workstations (Linux/macOS), local CLI usage
**Project Type**: Single backend workspace with multiple Rust crates (no GUI)
**Performance Goals**: Reasonable per-step latency for local development; dominated by LLM latency rather than orchestration overhead
**Constraints**: Rust-only in v1; no remote execution or containers; must run fully offline except for LLM HTTP calls
**Scale/Scope**: Single-developer usage on kata-sized repositories; small to medium codebases and test suites

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The plan demonstrates how it will:
- Keep code readable and intent-revealing via small, focused crates with clear module boundaries and descriptive types.
- Preserve or improve code quality using strong typing, explicit error handling (`anyhow`/`thiserror`), and consistent use of Rust idioms.
- Apply test-driven development by introducing unit tests for orchestration, git/process utilities, and agents, and adding e2e tests via `tdd-fixtures`.
- Be implemented in small, focused, reversible commits aligned with the workspace crates and user stories (init, run, status, config).
- Satisfy the pre-commit safety gate by wiring `cargo fmt`, `cargo clippy -D warnings`, and `cargo test` into the development and CI workflow and treating failures as blockers for commits.

## Project Structure

### Documentation (this feature)

```text
specs/001-tdd-agent-machine/
├── spec.md              # Feature specification (this feature)
├── plan.md              # Implementation plan (this file)
├── research.md          # (future) Design research, LLM and git patterns
├── data-model.md        # (future) Rust types and data relationships
├── quickstart.md        # (future) How to run init/run/step/status
└── contracts/           # (future) CLI/JSON schemas if needed
```

### Source Code (repository root)

```text
Cargo.toml                 # Workspace manifest
crates/
  tdd-cli/                 # binary, CLI entrypoint
    src/
      main.rs
  tdd-core/                # domain model, orchestrator, traits, commit policy
    src/
      lib.rs
      orchestrator.rs
      roles.rs
      step.rs
      commit_policy.rs
  tdd-agents/              # role implementations that call the LLMs
    src/
      lib.rs
      tester.rs
      implementor.rs
      refactorer.rs
  tdd-exec/                # test runner, git, fs, process utilities
    src/
      lib.rs
      runner.rs
      vcs.rs
      fs.rs
  tdd-llm/                 # client and adapters for OpenAI-compatible providers
    src/
      lib.rs
      client.rs
      config.rs
  tdd-fixtures/            # sample katas and tests for e2e validation (dev-dependency)
    src/
      lib.rs
    tests/
      string_calculator.rs
      bowling.rs

.tdd/
  logs/                    # per-step JSON logs
  plan/                    # per-step plan markdown files

kata.md                    # kata description (created by init)
tdd.yaml                   # main configuration file (created by init)
```

**Structure Decision**: Use a single cargo workspace with dedicated crates per concern (CLI, core domain/orchestrator, agents, execution helpers, LLM adapter, fixtures). Shared types (roles, step context/result) live in `tdd-core`. Side-effecting concerns (git, process execution) are isolated in `tdd-exec`. LLM interactions are isolated in `tdd-llm` with trait-based abstraction for testability.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|----------------------------------------|
| Multiple crates in single workspace | Separates CLI, domain, agents, execution, and LLM concerns for clarity and testability | Single-crate design would mix CLI, orchestration, git, and HTTP concerns, reducing readability and making tests harder |
| Dedicated fixtures crate | Allows isolated e2e tests and reusable kata examples | Embedding fixtures in `tdd-core` or `tdd-cli` would couple test data with runtime code and bloat main crates |

## Implementation Phases

### Phase 1: Workspace and Configuration Skeleton

- Create workspace `Cargo.toml` with member crates: `tdd-cli`, `tdd-core`, `tdd-agents`, `tdd-exec`, `tdd-llm`, `tdd-fixtures`.
- Scaffold each crate with minimal `lib.rs`/`main.rs` and placeholder modules.
- Define shared configuration structures in `tdd-core` or `tdd-llm` matching `tdd.yaml` (paths, steps, roles, LLM, CI, commit author).
- Add `tdd.yaml` template and `kata.md` placeholder under repo root.
- Add `.gitignore`, `rust-toolchain.toml`, and basic `clippy`/`fmt` config.

### Phase 2: Execution and Git Utilities (`tdd-exec`)

- Implement `Runner` trait and a concrete Rust runner that wraps `cargo fmt`, `cargo clippy`, and `cargo test` via `duct` or `tokio::process`.
- Implement `Vcs` trait using `git2` to init repo if needed, read last commit message, compute last diff, list files (via `walkdir`/`ignore`), stage all, and commit with author info.
- Add unit tests for `Runner` (command building and success/failure mapping) and `Vcs` (init, commit, state introspection using temporary directories).

### Phase 3: LLM Client (`tdd-llm`)

- Define `LlmClient` trait (`chat(messages: Vec<Message>) -> anyhow::Result<String>`) and `Message` type.
- Implement an OpenAI-compatible HTTP client using `reqwest`, honoring per-role model/temperature and shared base URL/API key env var.
- Add a mock implementation for tests and for `tdd-fixtures`.

### Phase 4: Core Domain and Orchestrator (`tdd-core`)

- Define `Role`, `StepContext`, `StepResult`, `Agent` trait, and `Orchestrator` trait as specified.
- Implement orchestrator that:
  - Determines starting role (Tester when repo is uninitialized).
  - Builds `StepContext` from `tdd-exec` and `kata.md`.
  - Calls `Agent::plan` and writes `.tdd/plan/step-N-role.md`.
  - Calls `Agent::edit` and applies file changes via `tdd-agents` helpers.
  - Invokes `Runner::fmt`, `Runner::check`, `Runner::test` in order, with retries for Implementor/Refactorer up to `max_attempts_per_agent`.
  - On success, delegates to `Vcs` to create a conventional commit message and rotate role.
- Encode conventional commit policy in a small `CommitPolicy` helper.

### Phase 5: Agents and File Editing (`tdd-agents`)

- Define concrete agent types implementing `Agent` for Tester, Implementor, Refactorer.
- Embed role-specific system prompts as Rust string constants, incorporating `StepContext` and selected file snippets.
- Implement JSON edit-plan parsing and file-application logic (upsert files, no patch files) in a helper module.
- Ensure Tester only touches tests, Implementor focuses on minimal behavior changes, Refactorer avoids changing test assertions.

### Phase 6: CLI (`tdd-cli`)

- Implement `tdd-cli` using `clap` with subcommands: `init`, `run --steps N`, `step`, `status`, `doctor`.
- `init`: create workspace scaffold if absent, `kata.md`, `tdd.yaml`, `.tdd/` directories, and initialize git.
- `run`: load config, create orchestrator, execute N steps.
- `step`: execute a single step (current role only) for debugging.
- `status`: inspect logs and git to report current role, step index, last commit summary.
- `doctor`: verify external tools (`git`, `cargo`, `rustfmt`, `clippy`) and configuration.

### Phase 7: Fixtures and E2E Tests (`tdd-fixtures`)

- Add at least one kata (e.g., String Calculator) as Markdown plus starter tests.
- Implement e2e tests that:
  - Use a temporary workspace.
  - Initialize via `tdd-cli init` equivalent helpers.
  - Run a small number of steps using a mocked `LlmClient`.
  - Assert on commit sequence, logs, and status behavior.

### Phase 8: Documentation and Polish

- Expand `README.md` with detailed usage examples (`init`, `run`, `step`, `status`, `doctor`), config reference, and architecture overview per crate.
- Ensure all public functions and types have Rustdoc comments.
- Run `cargo fmt`, `cargo clippy -D warnings`, and `cargo test` as part of the final validation.
