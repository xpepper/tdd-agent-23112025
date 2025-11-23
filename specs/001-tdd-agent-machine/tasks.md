# Tasks: Autonomous Multi-Agent TDD Machine

**Input**: Design documents from `/specs/001-tdd-agent-machine/` and `initial-requirements.md`
**Prerequisites**: plan.md (this feature), spec.md (user stories), research.md, data-model.md, contracts/

**Tests**: Tests are strongly recommended for all new behavior, especially
orchestrator, execution utilities, and agents, following the TDD
constitution. For new behavior, write or update tests first, ensure they
fail against current code, then implement the minimal change to make them
pass before refactoring.

**Organization**: Tasks are grouped by user story to enable independent
implementation and testing of each story, and to support small, focused
commits.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Workspace initialization and shared configuration

- [X] T001 Create cargo workspace manifest in `Cargo.toml` with members `crates/tdd-cli`, `crates/tdd-core`, `crates/tdd-agents`, `crates/tdd-exec`, `crates/tdd-llm`, `crates/tdd-fixtures`
- [X] T002 [P] Scaffold crate `crates/tdd-cli/src/main.rs` with a minimal `clap`-based CLI entrypoint
- [X] T003 [P] Scaffold crate `crates/tdd-core/src/lib.rs` with module declarations for `orchestrator`, `roles`, `step`, `commit_policy`
- [X] T004 [P] Scaffold crate `crates/tdd-agents/src/lib.rs` with module declarations for `tester`, `implementor`, `refactorer`
- [X] T005 [P] Scaffold crate `crates/tdd-exec/src/lib.rs` with module declarations for `runner`, `vcs`, `fs`
- [X] T006 [P] Scaffold crate `crates/tdd-llm/src/lib.rs` with module declarations for `client`, `config`
- [X] T007 [P] Scaffold crate `crates/tdd-fixtures/src/lib.rs` and `crates/tdd-fixtures/tests/` folder
- [X] T008 Add repo-level `.gitignore`, `rust-toolchain.toml`, and baseline `clippy`/`fmt` configuration files
- [X] T009 Create `tdd.yaml` template and `kata.md` placeholder at repository root

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure required before user stories

- [X] T010 Implement configuration structs for `tdd.yaml` in `crates/tdd-core/src/config.rs` (paths, steps, roles, LLM, CI, commit author)
- [X] T011 [P] Implement `Runner` trait and Rust runner in `crates/tdd-exec/src/runner.rs` wrapping `cargo fmt`, `cargo clippy`, and `cargo test`
- [X] T012 [P] Implement `RepoState` and `Vcs` trait in `crates/tdd-exec/src/vcs.rs` using `git2` for init, state reading, staging, and committing
- [X] T013 [P] Implement filesystem helpers in `crates/tdd-exec/src/fs.rs` for listing repo files using `walkdir` and `ignore`
- [X] T014 Add unit tests for `Runner` and `Vcs` in `crates/tdd-exec/tests/runner_vcs_tests.rs` using temporary directories
- [X] T015 Implement `LlmClient` trait and `Message` type in `crates/tdd-llm/src/client.rs`
- [X] T016 [P] Implement OpenAI-compatible HTTP client in `crates/tdd-llm/src/client.rs` using `reqwest` and `tokio`
- [X] T017 [P] Implement configuration parsing for LLM and roles in `crates/tdd-llm/src/config.rs` bound to `tdd.yaml`
- [X] T018 Add mock `LlmClient` implementation for tests in `crates/tdd-llm/src/mock.rs`

**Checkpoint**: Foundation ready – orchestrator and CLI can now build on stable execution, VCS, and LLM layers.

---

## Phase 3: User Story 2 - Initialize a kata workspace (Priority: P1) [US2]

**Goal**: `tdd-cli init` creates a Rust kata scaffold, config, and git repo ready for TDD steps.

**Independent Test**: Running `tdd-cli init` in an empty directory results in a compilable workspace with `kata.md`, `tdd.yaml`, tests folder, and initialized git repo; rerunning is idempotent.

### Implementation for User Story 2

- [X] T019 [US2] Define CLI arguments and subcommands (including `init`) in `crates/tdd-cli/src/main.rs` using `clap`
- [X] T020 [P] [US2] Implement `init` handler in `crates/tdd-cli/src/main.rs` that creates `crates/` structure, `kata.md`, `tdd.yaml`, `.tdd/plan`, `.tdd/logs` if absent
- [X] T021 [P] [US2] Use `tdd-exec::Vcs` in `init` to initialize git repo and create initial commit if repo was empty
- [X] T022 [US2] Implement basic validation of `tdd.yaml` after creation (required fields, defaults) in `crates/tdd-core/src/config.rs`
- [X] T023 [US2] Add integration test in `crates/tdd-cli/tests/init_tests.rs` that runs `tdd-cli init` in a temp directory and asserts generated files and git repo

**Checkpoint**: `tdd-cli init` works end-to-end and prepares the workspace for automated TDD steps.

---

## Phase 4: User Story 1 - Run a multi-step TDD loop (Priority: P1) [US1]

**Goal**: Execute N TDD steps with tester, implementor, and refactorer agents, producing git commits in a red–green–refactor loop.

**Independent Test**: On an initialized workspace with `tdd.yaml` and `kata.md`, running `tdd-cli run --steps 3` yields three commits with roles Tester → Implementor → Refactorer and passing tests after Implementor and Refactorer.

### Core Domain & Orchestrator

- [X] T024 [US1] Define `Role`, `StepContext`, and `StepResult` types in `crates/tdd-core/src/step.rs` per spec
- [X] T025 [P] [US1] Define `Agent` trait and `Orchestrator` trait in `crates/tdd-core/src/orchestrator.rs`
- [X] T026 [US1] Implement role rotation logic and starting-role rules in `crates/tdd-core/src/orchestrator.rs`
- [X] T027 [US1] Implement `StepContext` builder that pulls state from `tdd-exec::Vcs`, `tdd-exec::fs`, and `kata.md`
- [X] T028 [US1] Implement plan persistence to `.tdd/plan/step-N-role.md` in `crates/tdd-core/src/orchestrator.rs`
- [X] T029 [US1] Implement orchestrator `next` method that calls `Agent::plan`, `Agent::edit`, then `Runner::fmt/check/test` with retry logic for Implementor and Refactorer
- [X] T030 [US1] Implement `CommitPolicy` helper in `crates/tdd-core/src/commit_policy.rs` to format conventional commit messages with context, rationale, diff summary, and verification

### Agents & File Editing

- [X] T031 [US1] Implement JSON edit-plan schema and parser in `crates/tdd-agents/src/edit_plan.rs` (upsert file actions)
- [X] T032 [P] [US1] Implement file-application logic in `crates/tdd-agents/src/edit_plan.rs` to write full-file contents to disk, avoiding patch files
- [X] T033 [US1] Implement `TesterAgent` in `crates/tdd-agents/src/tester.rs` using `LlmClient` and enforcing test-only edits
- [X] T034 [US1] Implement `ImplementorAgent` in `crates/tdd-agents/src/implementor.rs` using `LlmClient` and enforcing minimal change to pass tests
- [X] T035 [US1] Implement `RefactorerAgent` in `crates/tdd-agents/src/refactorer.rs` using `LlmClient` and enforcing no test assertion changes
- [X] T036 [US1] Embed role-specific system prompts as constants in the respective agent modules

### CLI Wiring

- [ ] T037 [US1] Implement `run --steps N` subcommand in `crates/tdd-cli/src/main.rs` that loads config, constructs orchestrator, and executes N steps
- [ ] T038 [US1] Implement `step` subcommand in `crates/tdd-cli/src/main.rs` that executes a single orchestrator step
- [ ] T039 [US1] Add integration test in `crates/tdd-cli/tests/run_loop_tests.rs` that uses a mocked `LlmClient` to verify role sequence and commit creation for 3 steps

**Checkpoint**: `tdd-cli run --steps 3` and `tdd-cli step` function with mocked LLMs, producing correct role sequence and commits under passing tests.

---

## Phase 5: User Story 3 - Inspect status and diagnostics (Priority: P2) [US3]

**Goal**: Provide status and diagnostics so users can understand the current role, step index, last commit summary, and recent logs.

**Independent Test**: After running steps, `tdd-cli status` reports the next role, step number, last commit summary, and indicates any failing checks.

### Implementation for User Story 3

- [ ] T040 [US3] Define per-step JSON log schema in `crates/tdd-core/src/logging.rs` (role, step, plan path, runner outputs, commit id)
- [ ] T041 [P] [US3] Implement log writing in orchestrator to `.tdd/logs/step-N-role.json`
- [ ] T042 [US3] Implement `status` subcommand in `crates/tdd-cli/src/main.rs` that reads latest log and git state to print concise status
- [ ] T043 [US3] Add integration test in `crates/tdd-cli/tests/status_tests.rs` verifying `status` output after a few mocked steps

---

## Phase 6: User Story 4 - Configure models and CI commands (Priority: P2) [US4]

**Goal**: Allow configuration of per-role LLM models, base URL, API key env var, and CI commands through `tdd.yaml`.

**Independent Test**: Updating `tdd.yaml` changes which models and commands are used, without recompiling.

### Implementation for User Story 4

- [ ] T044 [US4] Extend configuration structs in `crates/tdd-core/src/config.rs` to match `tdd.yaml` example (roles, LLM, CI, commit author)
- [ ] T045 [P] [US4] Implement configuration loader in `crates/tdd-core/src/config.rs` that validates required fields and supplies defaults
- [ ] T046 [US4] Wire per-role model/temperature and base URL from config into `tdd-llm::client` construction
- [ ] T047 [US4] Wire CI command arrays from config into `tdd-exec::Runner` so fmt/check/test commands can be overridden
- [ ] T048 [US4] Add unit tests in `crates/tdd-core/tests/config_tests.rs` for parsing, validation, and override behavior

---

## Phase 7: User Story 5 - Use the tool in an existing kata (Priority: P3) [US5]

**Goal**: Allow running the TDD machine against an existing Rust kata repository without breaking existing code or history.

**Independent Test**: In a non-empty Rust repo, adding `tdd.yaml` and running `tdd-cli` initializes metadata and runs steps without overwriting existing files.

### Implementation for User Story 5

- [ ] T049 [US5] Adjust `init` logic in `crates/tdd-cli/src/main.rs` to detect existing Rust projects and skip recreating workspace files
- [ ] T050 [US5] Ensure `tdd-exec::Vcs` respects existing git history and only adds new commits for tool actions
- [ ] T051 [US5] Implement baseline test check in orchestrator that aborts if existing tests fail before running autonomous steps
- [ ] T052 [US5] Add integration test in `crates/tdd-cli/tests/existing_repo_tests.rs` with a pre-populated Rust project

---

## Phase N: Polish & Cross-Cutting Concerns

**Purpose**: Cross-story improvements and final hardening

- [ ] T053 [P] Add or refine Rustdoc comments for all public types and functions across crates
- [ ] T054 Improve error messages and error typing using `thiserror` for user-facing failures (CLI, orchestrator, execution)
- [ ] T055 [P] Add additional unit tests for edge cases (timeouts, LLM errors, git failures) across `tdd-exec`, `tdd-llm`, and `tdd-core`
- [ ] T056 Ensure `.tdd` directory handling is robust (creation, permissions, cleanup) in `tdd-core` and `tdd-cli`
- [ ] T057 [P] Update top-level `README.md` with CLI examples, configuration documentation, and architecture overview for all crates
- [ ] T058 Run `cargo fmt`, `cargo clippy -D warnings`, and `cargo test --all`
	as a pre-release safety gate and resolve any issues before initial
	release

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies – must complete before foundational work.
- **Foundational (Phase 2)**: Depends on Setup – blocks all user stories.
- **User Story 2 (Phase 3)**: Depends on Foundational – enables workspace initialization.
- **User Story 1 (Phase 4)**: Depends on Foundational and User Story 2 – requires initialized workspace and core infra.
- **User Story 3 (Phase 5)**: Depends on User Story 1 – requires orchestrator and logs.
- **User Story 4 (Phase 6)**: Depends on Foundational – can be developed in parallel with other stories once config structs exist.
- **User Story 5 (Phase 7)**: Depends on User Story 2 and User Story 1 – requires init and TDD loop behavior.
- **Polish (Final Phase)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **US1 (Run TDD loop)**: Requires working init and foundational infra.
- **US2 (Init workspace)**: Requires foundational infra but no other stories.
- **US3 (Status)**: Requires logs from orchestrator in US1.
- **US4 (Configurable models/CI)**: Requires foundational config and LLM client but not TDD loop behavior.
- **US5 (Existing kata)**: Requires init and TDD loop behavior to be stable.

### Parallel Opportunities

- Setup tasks marked [P] (crate scaffolding) can be implemented in parallel.
- Foundational tasks marked [P] (`Runner`, `Vcs`, fs helpers, LLM client/config) can progress concurrently once the workspace exists.
- Within US1, agent implementations and edit-plan helpers marked [P] can be developed in parallel after the orchestrator traits exist.
- US3 and US4 can partially proceed in parallel once orchestrator and config types are defined.
- Polish tasks marked [P] (docs, extra tests) can be parallelized after core functionality is stable.
