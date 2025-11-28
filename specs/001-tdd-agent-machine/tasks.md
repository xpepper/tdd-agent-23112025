# Tasks: Autonomous Multi-Agent TDD Machine

**Input**: Design documents from `/specs/001-tdd-agent-machine/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Each user story includes explicit test tasks; create/execute them before implementation to honor the constitution’s TDD rule.

**Organization**: Tasks are grouped by phase so every user story is independently implementable, testable, and releasable.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish workspace scaffolding, docs, and repo tooling described in plan.md.

- [X] T001 Ensure `Cargo.toml` defines the multi-crate workspace (`crates/tdd-cli`, `tdd-core`, `tdd-agents`, `tdd-exec`, `tdd-llm`, `tdd-fixtures`) per plan.md.
- [X] T002 [P] Scaffold crate directories plus placeholder `lib.rs`/`main.rs` files so `cargo check` succeeds before feature work.
- [X] T003 [P] Configure repo-level tooling (`.gitignore`, `rust-toolchain.toml`, `rustfmt.toml`, `Clippy.toml`) for consistent linting/formatting.
- [X] T004 [P] Seed developer artifacts (`README.md`, `kata.md`, starter `tdd.yaml`) mirroring quickstart.md instructions.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that all user stories require. No story work may start until this phase is complete.

- [X] T005 Implement `TddConfig` parsing/validation in `crates/tdd-core/src/config.rs` covering workspace, roles, LLM, CI, and commit author fields (data-model.md).
- [X] T006 [P] Implement the process runner abstraction in `crates/tdd-exec/src/runner.rs` to execute configured `fmt/check/test` commands.
- [X] T007 [P] Implement git helpers (`RepoState`, `GitVcs`) in `crates/tdd-exec/src/vcs.rs` for init/status/stage/commit flows.
- [X] T008 [P] Add filesystem snapshot utilities in `crates/tdd-exec/src/fs.rs` so `StepContext` can enumerate tracked files.
- [X] T009 [P] Define `LlmClient` traits plus OpenAI provider structs in `crates/tdd-llm/src/{client,config}.rs` using research.md decisions.
- [X] T010 [P] Implement `CommitPolicy` scaffolding in `crates/tdd-core/src/commit_policy.rs` aligned with FR-013.

**Checkpoint**: Config parsing, CI runner, git adapters, and LLM abstractions exist; user stories can begin.

---

## Phase 3: User Story 2 - Initialize a kata workspace (Priority: P1) 🎯 MVP

**Goal**: `tdd-cli init` (and the new provisioning script flow) create a ready-to-use kata workspace with `.tdd/*`, config, git repo, and optional bootstrap automation.

**Independent Test**: On an empty directory, running `tdd-cli init` (or `tdd-cli provision`) produces a compilable Rust workspace, logs bootstrap output, and running the command twice is idempotent.

### Tests for User Story 2

- [X] T011 [P] [US2] Expand `crates/tdd-cli/tests/init_tests.rs` for empty-dir initialization, idempotent reruns, and provisioning-script success/failure cases.
- [X] T012 [P] [US2] Update `/init` contract examples in `specs/001-tdd-agent-machine/contracts/openapi.yaml` to cover bootstrap metadata (skip markers, state file paths).

### Implementation for User Story 2

- [X] T013 [P] [US2] Wire `clap` subcommands (init, run, step, status, provision) in `crates/tdd-cli/src/main.rs` with shared argument parsing.
- [X] T014 [US2] Implement workspace detection + git initialization logic in `crates/tdd-cli/src/init.rs`, reusing `GitVcs` for existing repos.
- [X] T015 [US2] Create `.tdd/plan`, `.tdd/logs`, `.tdd/state`, default `kata.md`, and `tdd.yaml` scaffolds inside `crates/tdd-cli/src/init.rs` with safe overwrites.
- [X] T016 [US2] Extend `workspace` config with a `bootstrap` block (command, working_dir, skip markers) in `crates/tdd-core/src/config.rs` plus serde validation tests.
- [X] T017 [P] [US2] Implement `BootstrapRunner` in `crates/tdd-exec/src/bootstrap.rs` to resolve paths, honor skip files, stream output, and persist logs.
- [X] T018 [US2] Integrate bootstrap execution into `tdd-cli init` and add `tdd-cli provision --force` handling in `crates/tdd-cli/src/commands`.
- [X] T019 [US2] Persist bootstrap telemetry to `.tdd/logs/bootstrap-*.json` and `.tdd/state/bootstrap.json` via helpers in `crates/tdd-core/src/logging.rs`.
- [X] T020 [US2] Update `specs/001-tdd-agent-machine/quickstart.md`, README, and `tdd.yaml` template to explain provisioning scripts, env vars, and doctor guidance.

**Checkpoint**: CLI initialization + provisioning script automation works and is documented.

---

## Phase 4: User Story 1 - Run a multi-step TDD loop (Priority: P1)

**Goal**: Execute N tester→implementor→refactorer steps, running CI between roles and committing only after success.

**Independent Test**: On an initialized workspace (with or without provisioning), `tdd-cli run --steps 3` produces three commits with tests passing after implementor/refactorer and plans/logs written to `.tdd/`.

### Tests for User Story 1

- [X] T021 [P] [US1] Add `crates/tdd-cli/tests/run_loop_tests.rs` to assert role rotation, commit counts, CI gating, and provisioning log references.
- [ ] T022 [P] [US1] Add orchestrator unit tests in `crates/tdd-core/tests/orchestrator_tests.rs` covering retries, attempt limits, and bootstrap prerequisites.

### Implementation for User Story 1

- [X] T023 [US1] Implement `Role`, `StepContext`, and builder helpers in `crates/tdd-core/src/step.rs` per data-model relationships.
- [X] T024 [US1] Persist plans/logs via `PlanWriter` and `StepLogger` inside `crates/tdd-core/src/orchestrator.rs` and `crates/tdd-core/src/logging.rs`.
- [X] T025 [US1] Implement `DefaultOrchestrator::next` to invoke agents, run CI (`Runner`), enforce attempt limits, and commit via `GitVcs`.
- [X] T026 [US1] Build Tester/Implementor/Refactorer agents in `crates/tdd-agents/src/*.rs`, guaranteeing tester-only test edits.
- [X] T027 [US1] Integrate LLM clients into CLI execution (`crates/tdd-cli/src/executor.rs`) reading role configs and logging provider metadata.
- [X] T028 [US1] Finalize `CommitPolicy` formatting and apply when creating commits through `tdd-exec`.
- [ ] T029 [US1] Provide deterministic LLM fixtures in `crates/tdd-fixtures/` for CLI/orchestrator tests.

**Checkpoint**: Multi-step orchestration works end-to-end with mocks and real CI commands.

---

## Phase 5: User Story 3 - Inspect status and diagnostics (Priority: P2)

**Goal**: Let users inspect next role, step index, last commit summary, CI runner output, and bootstrap state via CLI/contract endpoints.

**Independent Test**: After several steps, `tdd-cli status` and `/logs/{step}` show provider info, runner output, and any bootstrap telemetry; doctor reports bootstrap health.

### Tests for User Story 3

- [ ] T030 [P] [US3] Add `crates/tdd-cli/tests/status_tests.rs` verifying status output includes next role, bootstrap summary, and recent CI results.
- [ ] T031 [P] [US3] Add log-reader tests in `crates/tdd-core/tests/logging_tests.rs` confirming bootstrap + runner metadata round-trip.

### Implementation for User Story 3

- [ ] T032 [US3] Extend `StepLogEntry` with provider, runner, and bootstrap references in `crates/tdd-core/src/logging.rs`.
- [ ] T033 [US3] Implement log reader helpers to fetch latest entries and bootstrap state for status/reporting.
- [ ] T034 [US3] Add `status` command plumbing in `crates/tdd-cli/src/commands/status.rs` mapping to `/status` and surfacing CI + bootstrap health.
- [ ] T035 [US3] Document diagnostics usage and log schema updates in `specs/001-tdd-agent-machine/quickstart.md` plus contracts (`/status`, `/logs/{step}`).

**Checkpoint**: Users can audit recent steps, bootstrap runs, and know what executes next.

---

## Phase 6: User Story 4 - Configure models and CI commands (Priority: P2)

**Goal**: Allow teams to configure role-specific models, OpenAI-compatible endpoints (Deepseek, GLM, Copilot), CI commands, and env vars without code changes.

**Independent Test**: Editing `tdd.yaml` to change provider/commands reroutes LLM requests and CI executions accordingly, with logs reflecting the selections.

### Tests for User Story 4

- [X] T036 [P] [US4] Expand `crates/tdd-core/tests/config_tests.rs` covering provider enums, API version defaults, Deepseek-style base URLs, and CI arrays.
- [ ] T037 [P] [US4] Add integration coverage to `crates/tdd-cli/tests/provider_switch_tests.rs` ensuring overrides change HTTP targets and logs.

### Implementation for User Story 4

- [X] T038 [US4] Extend `TddConfig` with provider + API version fields in `crates/tdd-core/src/config.rs` (OpenAI-compatible + Copilot).
- [X] T039 [US4] Implement provider factory with OpenAI + GitHub clients in `crates/tdd-llm/src/providers/{openai,github}.rs`, applying research.md headers.
- [X] T040 [US4] Update CLI executor (`crates/tdd-cli/src/executor.rs`) to instantiate clients based on config/env vars and log provider metadata.
- [X] T041 [US4] Update template config emitted by `tdd-cli init` (`crates/tdd-cli/src/init.rs`) with Deepseek/GLM/Copilot examples and doc comments.
- [X] T042 [US4] Ensure `/init`, `/steps/run`, and `/status` schemas in `specs/001-tdd-agent-machine/contracts/openapi.yaml` describe configurable providers/commands.

**Checkpoint**: Teams can swap providers/CI commands confidently; logs show the chosen provider per step.

---

## Phase 7: User Story 5 - Use the tool in an existing kata (Priority: P3)

**Goal**: Safely run the TDD machine inside non-empty repos, preserving history and blocking execution when baseline tests fail.

**Independent Test**: In an existing repo with tests, `tdd-cli init` reuses files/history, `tdd-cli run` halts if baseline tests fail, and provisioning scripts respect skip markers.

### Tests for User Story 5

- [X] T043 [P] [US5] Add `crates/tdd-cli/tests/existing_repo_tests.rs` covering init preservation, bootstrap skip logic, and baseline success.
- [X] T044 [P] [US5] Add failing-baseline and bootstrap-error scenarios ensuring CLI surfaces stderr/stdout clearly.

### Implementation for User Story 5

- [ ] T045 [US5] Enhance `crates/tdd-cli/src/init.rs` to detect existing `Cargo.toml`/`src` trees, skip destructive writes, and respect bootstrap skip files.
- [X] T046 [US5] Implement baseline test guard in `crates/tdd-cli/src/executor.rs` to run configured commands before orchestrating agents.
- [ ] T047 [US5] Ensure `crates/tdd-exec/src/vcs.rs` stages only new artifacts (plans/logs/bootstrap state) and preserves existing history.
- [X] T048 [US5] Document existing-repo + provisioning guidance in `specs/001-tdd-agent-machine/quickstart.md` and README.

**Checkpoint**: Existing projects can adopt the tool safely with clear failure messaging and bootstrap automation.

---

## Phase N: Polish & Cross-Cutting Concerns

**Purpose**: Final documentation, resilience, and ergonomics once functional stories land.

- [ ] T049 [P] Add comprehensive Rustdoc for public APIs across `crates/tdd-*` to explain orchestration + bootstrap hooks.
- [ ] T050 Improve user-facing error enums via `thiserror` in `crates/tdd-cli`, `tdd-core`, and `tdd-exec` (map bootstrap failures distinctly).
- [ ] T051 [P] Add edge-case tests for LLM timeouts, git failures, CI command errors, and bootstrap command hangs across `crates/tdd-cli/tests` and `crates/tdd-exec/tests`.
- [ ] T052 [P] Harden `.tdd` directory creation/cleanup logic (permissions, retries) in `crates/tdd-cli` and `crates/tdd-core`.
- [ ] T053 Validate the documented workflow by executing quickstart end-to-end (with and without bootstrap) and updating docs with lessons learned.

---

## Dependencies & Execution Order

1. **Setup → Foundational**: Base repo scaffolding (Phase 1) must exist before shared infrastructure (Phase 2).
2. **Foundational → US2**: Initialization relies on config parsing, git helpers, LLM abstractions, and commit policy.
3. **US2 → US1**: Multi-step runs require initialization/provisioning artifacts.
4. **US1 → US3/US4**: Diagnostics and configurability build on orchestration + logging.
5. **US2 & US1 → US5**: Existing-repo support depends on initialization + run loop behavior.
6. **All User Stories → Polish**: Final hardening waits until functional stories stabilize.

Dependency graph (priority order): `Setup → Foundational → US2 → US1 → US3 → US4 → US5 → Polish`.

---

## Parallel Execution Examples

- **Setup**: T002–T004 touch different files (crate directories vs. tooling vs. docs) and can run concurrently.
- **Foundational**: T006 (runner), T007 (git), and T009 (LLM traits) are independent once config parsing (T005) exists.
- **US2**: T017 (bootstrap runner) and T018 (CLI integration) can proceed in parallel after config changes (T016). Documentation (T020) can start once commands settle.
- **US1**: Agent work (T026) and CLI executor wiring (T027) can run concurrently after orchestrator skeleton (T023–T025) lands.
- **US4**: Provider factory (T039) and template config updates (T041) are independent once config fields exist (T038).
- **US5**: Baseline guard (T046) and docs (T048) can proceed while git adjustments (T047) finish.

---

## Implementation Strategy

1. **MVP (US2 + US1)**: Finish Setup + Foundational, ship provisioning-aware initialization (US2), then deliver the multi-step run loop (US1). Validate via integration tests before moving on.
2. **Incremental Delivery**: Layer in observability (US3), configurability (US4), then existing-repo adoption (US5). Each story remains independently testable per its checkpoints.
3. **Polish & Hardening**: Execute Phase N items—docs, refined errors, edge-case tests, `.tdd` resilience—after functional scope stabilizes.

Each phase is independently releasable; pause after any checkpoint to demo or cut an incremental release.
