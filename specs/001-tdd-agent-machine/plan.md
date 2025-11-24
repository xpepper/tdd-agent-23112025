# Implementation Plan: Autonomous Multi-Agent TDD Machine

**Branch**: `001-tdd-agent-machine` | **Date**: 2025-11-24 | **Spec**: `specs/001-tdd-agent-machine/spec.md`
**Input**: Feature specification from `/specs/001-tdd-agent-machine/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Build a Rust-based CLI (`tdd-cli`) that initializes kata workspaces, then
orchestrates Tester → Implementor → Refactorer agents through configurable
red–green–refactor loops while enforcing git-based safety gates and emitting
structured artifacts under `.tdd/`. This plan also adds support for
GitHub Copilot provided LLM models authenticated with a personal GitHub
token so teams can use Copilot’s hosted endpoints alongside existing
OpenAI-compatible providers.

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: Rust 1.75+ (stable toolchain per `rust-toolchain.toml`)
**Primary Dependencies**: `tokio`, `clap` for CLI, `anyhow`, workspace crates
(`tdd-core`, `tdd-agents`, `tdd-exec`, `tdd-llm`, `tdd-fixtures`), git2 for
tests; upcoming Copilot LLM integration requires GitHub API client support.
**Storage**: Local git repository + filesystem artifacts (`.tdd/plan`, `.tdd/logs`).
**Testing**: `cargo test` (unit, integration, fixtures) with mock LLM clients.
**Target Platform**: Local developer machines (macOS/Linux) running Rust CLI.
**Project Type**: Multi-crate Rust workspace with CLI front-end.
**Performance Goals**: Single TDD step completes in <10 minutes with default
LLM latency; CLI commands return structured status quickly.
**Constraints**: Must operate offline except for LLM calls; enforce pre-commit
safety gate; Copilot LLM support must handle GitHub token auth securely and
avoid storing tokens in repo.
**Scale/Scope**: Single-developer kata repositories with sequential agent
execution; steps typically <=50 files touched.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The plan MUST demonstrate how it will:
- Keep code readable via explicit orchestration modules and
  configuration-driven behavior; any Copilot LLM adapters will be factored
  behind `tdd-llm` traits for clarity.
- Preserve code quality by extending existing crates rather than adding
  ad-hoc scripts; deprecate no behavior and avoid unused abstractions.
- Apply TDD: add failing tests for CLI commands, orchestrator logging, and
  Copilot provider selection before implementing behavior.
- Land changes as role-scoped commits (init, run, status, Copilot auth)
  so each is reversible.
- Uphold the safety gate by running `cargo fmt`, `cargo clippy -D warnings`,
  and `cargo test --all` before every commit.

**Post-Phase-1 review**: Research and design artifacts lock down provider
abstractions, Copilot token handling, and logging changes without introducing
new dependencies or skipping tests, so the gate remains satisfied.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```text
crates/
├── tdd-cli/          # CLI entrypoints, Clap commands, user UX
├── tdd-core/         # Shared domain models (steps, config, logs)
├── tdd-agents/       # Role orchestrators and planner logic
├── tdd-exec/         # Git/process utilities, CI runners
├── tdd-llm/          # LLM client abstractions + providers
└── tdd-fixtures/     # Test helpers & mock LLM implementations

.specify/             # Specs, plans, constitution, automation scripts
specs/001-tdd-agent-machine/
├── spec.md
├── plan.md
└── (Phase outputs written by /speckit.plan)

root files: Cargo.{toml,lock}, kata.md (prompt), tdd.yaml (user config),
README.md.
```

**Structure Decision**: Use the existing multi-crate workspace. CLI behavior
builds in `crates/tdd-cli`, orchestrator logic lives in `tdd-agents` and
`tdd-core`, LLM provider work (including GitHub Copilot support) lands in
`tdd-llm`, while git+CI helpers remain in `tdd-exec`. Specs and docs stay
under `specs/001-tdd-agent-machine/` per template.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
