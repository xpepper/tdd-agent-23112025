# Tasks: Autonomous Multi-Agent TDD Machine

**Input**: Design documents from `/specs/001-tdd-agent-machine/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Every user story lists dedicated test tasks. Follow TDD—add and run those tests before implementing the related behavior.

**Organization**: Tasks are grouped by phase so each user story remains independently implementable, testable, and releasable.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish the Rust workspace skeleton, documentation, and repo-level tooling expected by the plan.

- [X] T001 Ensure `Cargo.toml` declares the multi-crate workspace (`crates/tdd-cli`, `tdd-core`, `tdd-agents`, `tdd-exec`, `tdd-llm`, `tdd-fixtures`) per plan.md.
- [X] T002 [P] Scaffold crate directories and placeholder `lib.rs` files under `crates/` so each crate builds pre-feature.
- [X] T003 [P] Configure repo-level tooling files (`.gitignore`, `rust-toolchain.toml`, `rustfmt.toml`, `Clippy.toml`) to enforce consistent linting/formatting.
- [X] T004 [P] Add base developer artifacts (`README.md`, `kata.md`, starter `tdd.yaml`) mirroring quickstart.md so contributors can run the CLI.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that every user story depends on; no user story work may start until these complete.

- [X] T005 Implement `TddConfig` parsing/validation in `crates/tdd-core/src/config.rs` covering workspace, roles, LLM, CI, and commit author fields from data-model.md.
- [X] T006 [P] Implement the process runner abstraction in `crates/tdd-exec/src/runner.rs` to execute configurable `fmt/check/test` commands.
- [X] T007 [P] Implement git repository helpers (`RepoState`, `GitVcs`) in `crates/tdd-exec/src/vcs.rs` for init/state/stage/commit flows.
- [X] T008 [P] Implement filesystem snapshot utilities in `crates/tdd-exec/src/fs.rs` so StepContext can enumerate tracked files.
- [X] T009 [P] Define `LlmClient`, `Message`, and settings structs in `crates/tdd-llm/src/client.rs` and `crates/tdd-llm/src/config.rs`, enabling pluggable providers per research.md.

**Checkpoint**: Configuration, CI runners, git adapters, and LLM abstractions are in place; user stories can now start.

---

## Phase 3: User Story 2 - Initialize a kata workspace (Priority: P1) 🎯 MVP

**Goal**: `tdd-cli init` produces a ready-to-use Rust kata workspace with `kata.md`, `tdd.yaml`, `.tdd/*`, and git scaffolding.

**Independent Test**: Running `tdd-cli init` in an empty directory creates compilable workspace assets and can be run twice without corruption.

### Tests for User Story 2

- [X] T010 [P] [US2] Add `crates/tdd-cli/tests/init_tests.rs` covering empty-dir init success and idempotent re-runs per `/init` contract.

### Implementation for User Story 2

- [X] T011 [P] [US2] Wire CLI argument parsing/subcommands (init, run, step, status) in `crates/tdd-cli/src/main.rs` using `clap`.
- [X] T012 [US2] Implement workspace detection and git initialization logic in `crates/tdd-cli/src/init.rs`, reusing `GitVcs` for existing repos.
- [X] T013 [US2] Create `.tdd/plan`, `.tdd/logs`, default `kata.md`, and `tdd.yaml` scaffolds inside `crates/tdd-cli/src/init.rs`.
- [X] T014 [US2] Align `/init` request/response examples inside `specs/001-tdd-agent-machine/contracts/openapi.yaml` with actual CLI behavior.
- [X] T015 [US2] Update `specs/001-tdd-agent-machine/quickstart.md` to document the init workflow and prerequisites (toolchain, git, tokens).

**Checkpoint**: CLI init command & docs verified; workspace can be created safely.

---

## Phase 4: User Story 1 - Run a multi-step TDD loop (Priority: P1)

**Goal**: Execute N red–green–refactor steps using Tester, Implementor, Refactorer agents, committing only when CI passes.

**Independent Test**: On an initialized workspace, `tdd-cli run --steps 3` creates three commits (tester→implementor→refactorer) with tests passing after green/refactor steps.

### Tests for User Story 1

- [X] T016 [P] [US1] Create integration coverage in `crates/tdd-cli/tests/run_loop_tests.rs` to assert role order, commit count, and CI success.
- [X] T017 [P] [US1] Add orchestrator unit tests in `crates/tdd-core/src/orchestrator.rs` validating role rotation, retries, and empty-repo rules.

### Implementation for User Story 1

- [X] T018 [US1] Implement `Role`, `StepContext`, and builder helpers in `crates/tdd-core/src/step.rs` using data-model.md definitions.
- [X] T019 [US1] Persist plans/logs via `PlanWriter` & `StepLogger` in `crates/tdd-core/src/orchestrator.rs` and `crates/tdd-core/src/logging.rs`.
- [X] T020 [US1] Implement `DefaultOrchestrator::next` to call agents, run CI (fmt/check/test), enforce attempt limits, and commit via `GitVcs`.
- [X] T021 [US1] Implement `CommitPolicy` formatting in `crates/tdd-core/src/commit_policy.rs` aligning with FR-013.
- [X] T022 [US1] Build `TesterAgent`, `ImplementorAgent`, `RefactorerAgent`, and edit-plan helpers across `crates/tdd-agents/src/*.rs`, enforcing per-role constraints.
- [X] T023 [US1] Integrate LLM clients into CLI execution in `crates/tdd-cli/src/executor.rs`, loading role models from config and emitting StepResult metadata.
- [X] T024 [US1] Provide mock/fake LLM fixtures in `crates/tdd-fixtures/` for deterministic CLI tests.

**Checkpoint**: Multi-step orchestration works end-to-end with mocked LLMs and commits are produced automatically.

---

## Phase 5: User Story 3 - Inspect status and diagnostics (Priority: P2)

**Goal**: Surface current role, step number, last commit summary, and recent CI logs via CLI/contract endpoints.

**Independent Test**: After running steps, `tdd-cli status` (and `/status` contract) shows the next role, step index, and last runner outcomes; `/logs/{step}` retrieves structured JSON.

### Tests for User Story 3

- [X] T025 [P] [US3] Add `crates/tdd-cli/tests/status_tests.rs` verifying CLI status output and log retrieval for both passing and failing steps.

### Implementation for User Story 3

- [X] T026 [US3] Extend `StepLogEntry` in `crates/tdd-core/src/logging.rs` with provider + runner metadata per data-model.md.
- [X] T027 [US3] Implement log reader helpers in `crates/tdd-core/src/logging.rs` to fetch latest entries for status/reporting.
- [X] T028 [US3] Add `status` command plumbing in `crates/tdd-cli/src/main.rs` (or `status.rs`) that prints next role/step plus CI summaries, mapping to `/status`.
- [X] T029 [US3] Document diagnostics usage in `specs/001-tdd-agent-machine/quickstart.md` and adjust `/status` + `/logs/{step}` sections in contracts/openapi.yaml.

**Checkpoint**: Users can audit recent steps and see what will run next without inspecting code.

---

## Phase 6: User Story 4 - Configure models and CI commands (Priority: P2)

**Goal**: Allow runtime configuration of role-specific LLM models, base URLs, API versions, and CI commands, including GitHub Copilot support.

**Independent Test**: Editing `tdd.yaml` to switch providers or CI commands changes actual runtime behavior without code changes.

### Tests for User Story 4

- [X] T030 [P] [US4] Expand `crates/tdd-core/tests/config_tests.rs` to cover provider enum parsing, default API versions, and invalid command arrays.
- [X] T031 [P] [US4] Add integration coverage in `crates/tdd-cli/tests/run_loop_tests.rs` asserting that overriding CI commands in `tdd.yaml` changes which binaries runner executes.

### Implementation for User Story 4

- [X] T032 [US4] Extend `TddConfig` in `crates/tdd-core/src/config.rs` with `llm.provider`, `api_version`, and role-specific settings.
- [X] T033 [US4] Implement provider factory plus GitHub Copilot client in `crates/tdd-llm/src/providers/{openai,github}.rs`, applying research.md header/token rules.
- [X] T034 [US4] Update CLI executor in `crates/tdd-cli/src/executor.rs` to instantiate the correct LLM client, read env vars, and pass provider metadata into logs.
- [X] T035 [US4] Update template config emitted by `crates/tdd-cli/src/init.rs` (and `tdd.yaml`) with annotated Copilot examples per quickstart.md.
- [X] T036 [US4] Ensure `/init` + `/steps` schemas in `specs/001-tdd-agent-machine/contracts/openapi.yaml` describe the new configuration knobs.

**Checkpoint**: Teams can swap providers/commands confidently; logs show which provider handled each step.

---

## Phase 7: User Story 5 - Use the tool in an existing kata (Priority: P3)

**Goal**: Run the TDD machine inside a non-empty Rust repo without overwriting files, enforcing baseline test checks.

**Independent Test**: In a repo with existing tests, `tdd-cli init` reuses current files/history and `tdd-cli run` refuses to proceed until baseline tests pass.

### Tests for User Story 5

- [X] T037 [P] [US5] Create `crates/tdd-cli/tests/existing_repo_tests.rs` case ensuring init preserves existing files/history and baseline tests pass.
- [X] T038 [P] [US5] Add failing-baseline scenario in the same test suite verifying the CLI blocks execution and surfaces stdout/stderr clearly.

### Implementation for User Story 5

- [X] T039 [US5] Enhance `crates/tdd-cli/src/init.rs` to detect existing `Cargo.toml`/`src` trees and skip destructive writes while still creating `.tdd/*` metadata.
- [X] T040 [US5] Implement baseline test guard in `crates/tdd-cli/src/executor.rs` that runs configured test commands before orchestrating agents.
- [X] T041 [US5] Ensure `crates/tdd-exec/src/vcs.rs` leaves existing history untouched (stage only new artifacts, commit with configured author).
- [X] T042 [US5] Document existing-repo expectations and troubleshooting inside `specs/001-tdd-agent-machine/quickstart.md` and README.

**Checkpoint**: Existing projects can adopt the tool safely with clear failure messaging.

---

## Phase N: Polish & Cross-Cutting Concerns

**Purpose**: Final hardening, docs, and resilience improvements after all user stories land.

- [ ] T043 [P] Add comprehensive Rustdoc for public APIs across `crates/tdd-*` to satisfy long-term polish goals.
- [ ] T044 Improve user-facing error enums using `thiserror` in `crates/tdd-cli`, `tdd-core`, and `tdd-exec`.
- [ ] T045 [P] Add edge-case tests (LLM timeouts, git failures, CI command errors) across `crates/tdd-exec/tests` and `crates/tdd-cli/tests`.
- [ ] T046 [P] Harden `.tdd` directory creation/cleanup logic in `crates/tdd-cli` and `crates/tdd-core`, ensuring permissions checks.
- [ ] T047 Validate the documented workflow by executing steps from `specs/001-tdd-agent-machine/quickstart.md` end-to-end and updating docs with lessons learned.

---

## Dependencies & Execution Order

1. **Phase 1 → Phase 2**: Scaffold workspace before adding config/runner/git infrastructure.
2. **Phase 2 → Phase 3**: Init CLI depends on config + git helpers.
3. **Phase 3 → Phase 4**: Running steps requires a workspace created by init.
4. **Phase 4 → Phase 5**: Status/logging consume orchestrator outputs.
5. **Phase 4 → Phase 6**: Configurability builds atop working loop + config structs.
6. **Phase 3 & 4 → Phase 7**: Existing-repo flow needs init + run behaviors.
7. **Phase N**: Polish after all functional stories.

Dependency graph (story order): `Setup → Foundational → US2 → US1 → US3 → US4 → US5 → Polish`.

---

## Parallel Execution Examples

- **US2**: T011 (CLI wiring) and T013 (filesystem scaffolding) can run in parallel after T010 lands because they touch different modules.
- **US1**: T022 (agents) and T023 (CLI executor wiring) can proceed simultaneously once T018–T020 define contexts/orchestrator.
- **US3**: T026 (log schema) and T028 (CLI status command) may run in parallel—the status command consumes only the finalized reader contract.
- **US4**: T033 (provider factory) and T035 (template config updates) are independent once T032 defines config fields.
- **US5**: T039 (init detection) and T040 (baseline guard) target separate files, so different contributors can implement them concurrently after tests T037–T038 exist.

---

## Implementation Strategy

1. **MVP (US2 + US1)**: Complete Setup + Foundational, deliver US2 (init), then US1 (run loop). Validate via T016–T017 integration tests before proceeding.
2. **Incremental Delivery**: Ship US3 (status) once logs exist, enabling observability. Next, add US4 (config flexibility), then US5 (existing repo) to widen adoption.
3. **Polish & Hardening**: After feature stories, execute Phase N tasks (Rustdoc, error ergonomics, edge-case tests, `.tdd` resilience, quickstart validation) ahead of release.

Each phase remains independently testable—pause after any checkpoint to demo or release.
