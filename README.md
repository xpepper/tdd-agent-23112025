# Autonomous Multi-Agent TDD Machine

`tdd-cli` orchestrates three agents (Tester → Implementor → Refactorer)
to evolve a kata repository in strict red–green–refactor loops. The
tool keeps all state in git, writes per-step plans/logs under `.tdd/`,
and enforces the safety gate defined in
`.specify/memory/constitution.md`.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Commands](#commands)
- [Configuration](#configuration)
- [LLM Providers](#llm-providers)
- [Working with Existing Projects](#working-with-existing-projects)
- [Logs and Plans](#logs-and-plans)
- [Development Workflow](#development-workflow)
- [Crate Structure](#crate-structure)
- [Troubleshooting](#troubleshooting)

## Prerequisites

- Rust stable toolchain (1.75+) and Cargo
- `git` available in `PATH`
- LLM provider access:
  - **OpenAI**: API key with access to GPT models
  - **GitHub Copilot**: Personal access token with Copilot scope

## Quick Start

### New Project

```bash
# 1. Clone or create a new directory
mkdir my-kata && cd my-kata

# 2. Initialize the TDD workspace
cargo run -p tdd-cli -- init

# 3. Edit kata.md to describe your kata
vim kata.md

# 4. (Optional) Wire a provisioning script
#   - Uncomment workspace.bootstrap in tdd.yaml
#   - Point it at ./scripts/bootstrap.sh (or similar)
#   - Run `cargo run -p tdd-cli -- provision` to verify it succeeds

#   Example bootstrap block
#   bootstrap:
#     command: ["/bin/sh", "./scripts/bootstrap.sh"]
#     working_dir: "."
#     skip_files:
#       - ".tdd/state/bootstrap.skip"

# 5. Set your API key
export OPENAI_API_KEY="your-key-here"

# 6. Run autonomous TDD steps
cargo run -p tdd-cli -- run --steps 3

# 7. Check status
cargo run -p tdd-cli -- status
```

### Existing Rust Project

```bash
# 1. Navigate to your existing project
cd your-rust-project

# 2. Initialize TDD workspace (non-destructive)
cargo run -p tdd-cli -- init

# 2b. (Optional) Configure workspace.bootstrap and run provisioning
cargo run -p tdd-cli -- provision --force

# 3. Configure and run
export OPENAI_API_KEY="your-key-here"
cargo run -p tdd-cli -- run --steps 1

If provisioning only needs to run once (e.g., heavy tool installs), create a
skip marker so later `init` runs complete instantly:

```bash
touch .tdd/state/bootstrap.skip
```
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    tdd-cli                          │
│  ┌──────────┐  ┌──────────┐    ┌────────────────┐   │
│  │   init   │  │   run    │    │    status      │   │
│  └─────┬────┘  └────┬─────┘    └───────┬────────┘   │
│        │            │                  │            │
└────────┼────────────┼──────────────────┼────────────┘
         │            │                  │
         ▼            ▼                  ▼
┌─────────────────────────────────────────────────────┐
│                   tdd-core                          │
│  ┌──────────────┐  ┌────────────┐  ┌────────────┐   │
│  │ Orchestrator │  │   Config   │  │  Logging   │   │
│  └──────┬───────┘  └────────────┘  └────────────┘   │
│         │                                           │
└─────────┼───────────────────────────────────────────┘
          │
          ▼
┌────────────────────────────────────────────────────┐
│                  tdd-agents                        │
│  ┌────────┐  ┌─────────────┐  ┌────────────┐       │
│  │ Tester │  │ Implementor │  │ Refactorer │       │
│  └───┬────┘  └──────┬──────┘  └──────┬─────┘       │
│      │              │                │             │
└──────┼──────────────┼────────────────┼─────────────┘
       │              │                │
       ▼              ▼                ▼
┌─────────────────────────────────────────────────────┐
│                   tdd-llm                           │
│  ┌──────────────┐         ┌──────────────────┐      │
│  │ OpenAI Client│         │ Copilot Client   │      │
│  └──────────────┘         └──────────────────┘      │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│                   tdd-exec                          │
│  ┌────────┐  ┌───────────┐  ┌──────────────┐        │
│  │  VCS   │  │  Runner   │  │  Filesystem  │        │
│  └────────┘  └───────────┘  └──────────────┘        │
└─────────────────────────────────────────────────────┘
```

## Commands

### `init`

Initialize a TDD workspace. Safe to run in existing projects.

```bash
cargo run -p tdd-cli -- init [--config tdd.yaml]
```

**Behavior:**
- Creates `tdd.yaml`, `kata.md`, `.tdd/plan/`, `.tdd/logs/`, `.tdd/state/`
- Initializes git repository if needed
- Detects existing Rust projects and preserves all files
- Executes the configured `workspace.bootstrap` command once and records its telemetry
- Validates existing configurations

### `run`

Execute multiple TDD steps in sequence.

```bash
cargo run -p tdd-cli -- run --steps N [--config tdd.yaml]
```

**Behavior:**
- Runs N cycles of Tester → Implementor → Refactorer
- Creates git commit after each successful step
- Respects `max_steps` limit in configuration
- Stops on unrecoverable errors

### `step`

Execute a single TDD step (alias for `run --steps 1`).

```bash
cargo run -p tdd-cli -- step [--config tdd.yaml]
```

### `status`

Display current progress and diagnostics.

```bash
`cargo run -p tdd-cli -- status [--config tdd.yaml]`

**Output:**
- Next role to execute
- Current step number
- Last commit summary
- Bootstrap summary (skipped? log file? state path?)
- CI command results (fmt/check/test exit codes)

### `provision`

Run (or re-run) the provisioning/bootstrap script without touching any other artifacts.

```bash
cargo run -p tdd-cli -- provision [--config tdd.yaml] [--force]
```

**Behavior:**
- Loads `workspace.bootstrap` from `tdd.yaml`
- Skips execution when any `skip_files` markers (for example `.tdd/state/bootstrap.skip`) exist unless `--force` is provided
- Streams the same telemetry as `init`, writing `.tdd/logs/bootstrap-*.json` and `.tdd/state/bootstrap.json`

### `doctor`

Diagnose environment prerequisites before running the automation.

```bash
cargo run -p tdd-cli -- doctor [--config tdd.yaml]
```

**Behavior:**
- Verifies git cleanliness and required binaries (`cargo`, configured CI commands)
- Ensures LLM API tokens referenced by `api_key_env` are loaded
- Surfaces bootstrap health (latest state file, skip markers) so you know whether provisioning ran
- *Note*: The current implementation still prints a placeholder message—full diagnostics arrive in a later milestone.
- CI command results (fmt/check/test exit codes)

## Configuration

Edit `tdd.yaml` to customize behavior:

```yaml
workspace:
  kata_file: "kata.md"
  plan_dir: ".tdd/plan"
  log_dir: ".tdd/logs"
  max_steps: 10
  max_attempts_per_agent: 2
  # Optional provisioning hook (runs before first tester step)
  # bootstrap:
  #   command: ["/bin/sh", "./scripts/bootstrap.sh"]
  #   working_dir: "."
  #   skip_files:
  #     - ".tdd/state/bootstrap.skip"

roles:
  tester:
    model: "gpt-4o-mini"
    temperature: 0.1
  implementor:
    model: "gpt-4o-mini"
    temperature: 0.2
  refactorer:
    model: "gpt-4o-mini"
    temperature: 0.15

llm:
  provider: "openai"  # or "github_copilot"
  base_url: "https://api.openai.com/v1"
  api_key_env: "OPENAI_API_KEY"

ci:
  fmt: ["cargo", "fmt"]
  check: ["cargo", "clippy", "-D", "warnings"]
  test: ["cargo", "test"]

commit_author:
  name: "Autonomous TDD Machine"
  email: "tdd-machine@example.com"
```

## LLM Providers

### OpenAI (Default)

```yaml
llm:
  provider: "openai"
  base_url: "https://api.openai.com/v1"
  api_key_env: "OPENAI_API_KEY"
```

```bash
export OPENAI_API_KEY="sk-..."
```

### GitHub Copilot

```yaml
llm:
  provider: "github_copilot"
  base_url: "https://api.githubcopilot.com/v1"
  api_key_env: "GITHUB_COPILOT_TOKEN"
  api_version: "2023-12-01"
```

```bash
export GITHUB_COPILOT_TOKEN="ghp_..."
```

**Note:** Requires a personal access token with `read:org` and `copilot` scopes.

## Working with Existing Projects

The TDD machine integrates safely with existing Rust projects:

1. **Detection**: Automatically detects `Cargo.toml` and `src/`
2. **Preservation**: Never overwrites existing project files
3. **Git History**: Respects and extends existing commit history
4. **Baseline Check**: Runs existing tests before first TDD step
   - Aborts if baseline tests fail
   - Ensures autonomous changes don't break working code

```bash
cd my-existing-kata
cargo run -p tdd-cli -- init  # Non-destructive
cargo run -p tdd-cli -- run --steps 1
```

## Logs and Plans

### Plans (`.tdd/plan/step-XYZ-role.md`)

Human-readable markdown showing each agent's reasoning:
- Context summary
- Proposed changes
- Rationale

### Logs (`.tdd/logs/step-XYZ-role.json`)

Machine-readable JSON capturing:
- `step_index`: Step number
- `role`: tester/implementor/refactorer
- `provider`: LLM provider used
- `files_changed`: List of modified files
- `commit_id`: Resulting git SHA
- `runner`: CI command results (exit codes, stdout, stderr)

### Bootstrap telemetry

- `.tdd/logs/bootstrap-*.json` captures each provisioning run (command, working directory, stdout/stderr, skip reason).
- `.tdd/state/bootstrap.json` stores the latest summary so `status`/`doctor` can report whether provisioning succeeded.
- Touch `.tdd/state/bootstrap.skip` (or any configured skip marker) to instruct the CLI to skip bootstrap unless `--force` is provided.

## Development Workflow

This project follows the **TDD Agent Constitution**:

### Principles

1. **Readable, Intent-Revealing Code**: Clear naming, no side effects
2. **Consistent Quality**: No dead code, follow existing patterns
3. **Test-Driven Development**: Tests first, then implementation
4. **Small, Focused Commits**: Each commit tells one story
5. **Pre-Commit Safety Gate**: All checks must pass before commit

### Safety Gate

Before every commit:
```bash
cargo fmt
cargo clippy -D warnings
cargo test --all
```

All three must succeed.

For the full constitution, see
`.specify/memory/constitution.md`.

## Crate Structure

```
crates/
├── tdd-cli/          # CLI interface (clap, main entry point)
├── tdd-core/         # Domain models, orchestrator, config
├── tdd-agents/       # Agent implementations (tester, implementor, refactorer)
├── tdd-exec/         # Execution utilities (git, runner, filesystem)
├── tdd-llm/          # LLM client abstractions and providers
└── tdd-fixtures/     # Test fixtures and utilities
```

### Key Types

- **`TddConfig`** (`tdd-core`): Workspace configuration from YAML
- **`Orchestrator`** (`tdd-core`): Coordinates agent execution
- **`Agent`** (`tdd-agents`): Trait for tester/implementor/refactorer
- **`LlmClient`** (`tdd-llm`): LLM provider abstraction
- **`Vcs`** (`tdd-exec`): Version control operations
- **`Runner`** (`tdd-exec`): CI command execution

## Troubleshooting

### "Baseline test check failed"

**Cause:** Existing tests are failing before TDD loop starts.

**Solution:**
```bash
cargo test  # Fix failing tests manually
cargo run -p tdd-cli -- run --steps 1  # Retry
```

### "Missing API key"

**Cause:** LLM provider API key not set.

**Solution:**
```bash
# For OpenAI
export OPENAI_API_KEY="your-key"

# For GitHub Copilot
export GITHUB_COPILOT_TOKEN="your-token"
```

### "Failed to parse YAML config"

**Cause:** Invalid `tdd.yaml` syntax or missing required fields.

**Solution:**
```bash
cargo run -p tdd-cli -- init  # Regenerate default config
# Or fix syntax errors manually
```

### "Workspace already reached max_steps"

**Cause:** Hit the `max_steps` limit in configuration.

**Solution:**
- Increase `workspace.max_steps` in `tdd.yaml`, or
- Remove old plan files from `.tdd/plan/` to reset counter

---

**License:** See repository license
**Contributing:** Follow the TDD constitution in `.specify/memory/constitution.md`
