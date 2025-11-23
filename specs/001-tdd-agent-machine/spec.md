# Feature Specification: Autonomous Multi-Agent TDD Machine

**Feature Branch**: `001-tdd-agent-machine`
**Created**: 2025-11-23
**Status**: Draft
**Input**: User description from `initial-requirements.md`

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run a multi-step TDD loop (Priority: P1)

As a developer practicing code katas on my local machine,
I want to run an autonomous TDD loop for a configurable
number of steps so that three agents (tester, implementor,
refactorer) can iteratively evolve the kata while keeping
tests passing and changes traceable in git.

**Why this priority**: This is the core value of the tool:
automated red–green–refactor cycles driven by multiple
agents. Without this, the product does not meet its primary
objective.

**Independent Test**: On a fresh workspace with a kata
description configured, a user can run a command to execute
N TDD steps and observe alternating commits from tester,
implementor and refactorer, with tests passing after
implementor and refactorer steps and detailed commit
messages recorded.

**Acceptance Scenarios**:

1. **Given** an initialized Rust kata repo with `tdd.yaml`
   configured and a kata description file, **When** the user
   runs the CLI with a command to execute 3 steps,
   **Then** the tool produces a sequence of commits that
   follow the red–green–refactor loop
   (tester → implementor → refactorer), and all tests are
   passing after the second and third commits.
2. **Given** an existing repo with prior commits from the
   tool, **When** the user runs the CLI for additional
   steps, **Then** the tool resumes from the current state,
   continues the role rotation, and appends new commits
   without breaking previous behavior.

---

### User Story 2 - Initialize a kata workspace (Priority: P1)

As a developer starting a new kata, I want a single command
to initialize a Rust workspace, test scaffolding, config,
and git repository so that the TDD machine can start running
without manual project setup.

**Why this priority**: A reliable, standardized starting
point for katas is required before any autonomous TDD loop
can run. Manual setup is error-prone and undermines the
benefit of automation.

**Independent Test**: On an empty directory, running the
initialization command creates a compilable Rust workspace,
default configuration files, kata description placeholder,
and a git repository ready for the first tester step.

**Acceptance Scenarios**:

1. **Given** an empty folder with no git repository,
   **When** the user runs the initialization command,
   **Then** a Rust workspace, kata description file,
   configuration file, test folder, and git repository are
   created, and a status command reports that the system is
   ready for TDD steps.
2. **Given** a folder already initialized by the tool,
   **When** the user reruns the initialization command,
   **Then** the tool detects the existing setup and either
   reports it as already initialized or safely updates
   non-destructive configuration files without corrupting
   the repo.

---

### User Story 3 - Inspect status and diagnostics (Priority: P2)

As a developer running the TDD machine, I want to see the
current role, step counter, last commit summary, and recent
logs so that I can understand what the agents have done and
why, and troubleshoot any failures.

**Why this priority**: Visibility into agent activity and
health is essential for trust and debuggability, but not as
foundational as basic initialization and TDD loop
execution.

**Independent Test**: After running several steps, the user
can query the status and log outputs to understand the last
role executed, step index, most recent commit context, and
any failing checks.

**Acceptance Scenarios**:

1. **Given** a repo where multiple TDD steps have completed,
   **When** the user runs the status command,
   **Then** they see which role will run next, the current
   step number, and a short summary of the last commit and
   test results.
2. **Given** that a recent step failed due to formatting,
   checking, or tests, **When** the user inspects logs,
   **Then** they can see structured information for that
   step including which role ran, commands executed, and
   captured output.

---

### User Story 4 - Configure models and CI commands (Priority: P2)

As a team integrating the TDD machine into our environment,
I want to configure which LLMs, API endpoints, and CI
commands are used so that the tool can align with our
infrastructure and policies without code changes.

**Why this priority**: Pluggable models and configurable
commands make the tool adaptable across environments and
LLM providers, enabling broader use without forks.

**Independent Test**: Changing values in a configuration
file (for models, base URL, and commands) changes which
external services and local commands the tool invokes,
without modifying source code.

**Acceptance Scenarios**:

1. **Given** a valid configuration file specifying different
   models per role and a custom base URL,
   **When** the user runs TDD steps,
   **Then** requests are routed to the configured
   OpenAI-compatible endpoint and model names according to
   the role.
2. **Given** that the configuration specifies alternative
   test, check, or format commands,
   **When** the tool runs a step,
   **Then** it executes those commands in the configured
   order and records their outputs in logs.

---

### User Story 5 - Use the tool in an existing kata (Priority: P3)

As a developer with an existing Rust kata repository,
I want to point the TDD machine at my current project so
that agents can start from the existing code and tests
rather than starting from scratch.

**Why this priority**: This expands applicability beyond new
projects, but is secondary to getting the greenfield flow
reliable.

**Independent Test**: In a repo that already contains Rust
code and tests, the user can add configuration and run TDD
steps without losing existing history or behavior.

**Acceptance Scenarios**:

1. **Given** an existing Rust project with tests,
   **When** the user adds a configuration file and runs the
   TDD machine, **Then** the tool initializes its own
   metadata directories, reuses the existing git history,
   and proceeds with TDD steps without overwriting existing
   files unnecessarily.
2. **Given** an existing project where tests currently fail,
   **When** the user attempts to run TDD steps,
   **Then** the tool detects the failing baseline and
   reports that initial stabilization is required before the
   autonomous loop can proceed.

### Edge Cases

- What happens when the configured steps or attempts per
  agent are very small (e.g., 0 or 1)?
- How does the system behave when formatting, checking, or
  tests hang or take excessively long?
- What happens if the configured LLM endpoint is
  unreachable, slow, or returns an error?
- How does the system respond when git operations fail
  (e.g., due to conflicts, missing author info, or
  permissions)?
- What happens if the workspace contains very large files or
  many files beyond a reasonable limit for context?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a command-line
  interface that allows users to initialize a kata
  workspace, run a configurable number of TDD steps, run a
  single step, inspect status, and perform environment
  checks.
- **FR-002**: The system MUST orchestrate three roles
  (tester, implementor, refactorer) in a strict red–green–
  refactor loop for a configurable number of steps.
- **FR-003**: The system MUST maintain all working state in
  a local git repository, including initializing git when
  absent and creating commits as steps succeed.
- **FR-004**: Each role MUST be able to read the last commit
  message, last diff, and a snapshot of files in the
  repository to inform its decisions.
- **FR-005**: The system MUST consume a kata description
  from a Markdown file and make its content available as
  context for all roles.
- **FR-006**: Implementation MUST support automated testing
  and be covered by unit and/or integration tests as
  appropriate, especially for orchestration and git/process
  utilities.
- **FR-007**: Behavior MUST be decomposed so that it can be
  delivered in small, independently testable increments,
  such as initialization, single-step execution, and
  multi-step runs.
- **FR-008**: For each step, the system MUST run formatting,
  compile checks, and tests in a defined order and only
  create a commit when these checks succeed.
- **FR-009**: The system MUST respect a maximum number of
  attempts per agent for failing steps, stopping and
  reporting failure when the limit is reached.
- **FR-010**: The system MUST persist per-step plans and
  execution logs to dedicated files so that users can audit
  what each role decided and did.
- **FR-011**: The system MUST allow configuration of
  role-specific LLM settings (model, temperature) and a
  shared OpenAI-compatible base URL and authentication
  mechanism.
- **FR-012**: The system MUST allow configuration of
  commands for formatting, checking, and testing so that it
  can be adapted to different environments.
- **FR-013**: The system MUST enforce a conventional commit
  structure that includes context, rationale, diff summary,
  and verification details tailored to the role.
- **FR-014**: The system MUST provide a way to reset or
  recover from a failed step without leaving the repository
  in a broken state.

*Example of marking unclear requirements:*

- **FR-015**: The system MUST support additional programming
  languages beyond Rust in the future
  [NEEDS CLARIFICATION: which languages and in what
  priority for future versions?]

### Key Entities *(include if feature involves data)*

- **TDD Configuration**: Represents user-controlled
  settings such as kata description path, number of steps,
  maximum attempts per agent, role models, LLM endpoint,
  CI commands, and commit author identity.
- **TDD Step**: Represents a single iteration of the
  red–green–refactor loop, including the role executed, step
  index, plan location, logs, outcome, and associated
  commit.
- **Repo Snapshot**: Represents a summary of the working
  tree at a point in time, including last commit message,
  last diff, and a list of files considered for context.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a fresh directory, a user can
  initialize a kata and run at least 3 TDD steps in a row
  with no manual intervention, resulting in a sequence of
  passing commits that follow the Tester → Implementor →
  Refactorer pattern.
- **SC-002**: For a typical kata of moderate size,
  running a single TDD step completes within a reasonable
  time window acceptable for local development
  (e.g., under several minutes, depending mainly on external
  LLM latency).
- **SC-003**: At least 90% of orchestrator and execution
  paths (including success and common failure modes) are
  covered by automated tests.
- **SC-004**: In user tests or internal usage, developers
  report that they can understand what happened in each
  step, based on status output, per-step logs, and commit
  messages, without needing to inspect internal
  implementation details.
