# Phase 1: Data Model

## Entity: TDD Configuration (`tdd-core::config::TddConfig`)

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| `workspace.kata_file` | Path | Markdown file consumed by agents | Must exist/readable before running steps |
| `workspace.plan_dir` | Path | Directory where `.tdd/plan/*` files are written | Auto-created; must be relative to repo |
| `workspace.log_dir` | Path | Directory for per-step logs `.tdd/logs/*` | Auto-created; must be relative to repo |
| `workspace.max_steps` | u32 | Upper bound passed to `run --steps` | Defaults to 10; coerced to >0 |
| `workspace.max_attempts_per_agent` | u32 | Retry count for failing agent executions | Defaults to 2; coerced to >0 |
| `roles.{tester,implementor,refactorer}.model` | String | Provider-specific model name | Non-empty |
| `roles.{...}.temperature` | f32 | Sampler temperature | 0.0 ≤ temperature ≤ 2.0 |
| `llm.provider` | Enum (`openai`, `github_copilot`) | Selects which client implementation to instantiate | Required; unknown value rejects config |
| `llm.base_url` | String/URL | Root endpoint for provider | Non-empty; HTTPS recommended |
| `llm.api_version` | Option<String> | Required for providers that demand version headers (Copilot) | If `provider == github_copilot`, default `2023-12-01` |
| `llm.api_key_env` | String | Env var name storing provider token | Non-empty; env lookup performed at runtime |
| `ci.fmt/check/test` | Vec<String> | Command invocations run sequentially | Each list ≥1 entry |
| `commit_author.{name,email}` | Strings | Signature used for auto-commits | Non-empty, valid email format |

### Relationships
- `llm.provider` drives which implementation of `tdd-llm::LlmClient` is created.
- `roles.*` feeds into `LlmClientSettings.role_models` to choose model/temperature per agent.
- `workspace.*` links CLI options to orchestrator/planner directories.

### Notes
- Introducing `llm.provider` is backwards-compatible by defaulting to `openai` when omitted.
- When `provider == github_copilot`, the CLI enforces presence of a personal GitHub token in the configured env var and sends the `X-GitHub-Api-Version` header using `llm.api_version`.

## Entity: Step Context (`tdd-core::step::StepContext`)

| Field | Type | Description |
|-------|------|-------------|
| `role` | Enum (`Tester`, `Implementor`, `Refactorer`) | Current agent |
| `step_index` | u32 | 1-based sequence number |
| `kata_description` | String | Contents of `kata.md` |
| `git_last_commit_msg` | String | Most recent commit message |
| `git_last_diff` | String | Diff between HEAD and previous commit |
| `repo_snapshot_paths` | Vec<String> | Sorted list of tracked files for context |

### Relationships
- Produced before each agent run and passed to `tdd-agents` plus the selected `LlmClient`.

### Validation Rules
- Requires readable `kata_file`; errors bubble up as `StepContextError::ReadKata`.
- Snapshot enumeration excludes `.git/` and uses forward slashes for portability.

## Entity: Step Result (`tdd-core::step::StepResult`)

| Field | Type | Description |
|-------|------|-------------|
| `files_changed` | Vec<String> | Relative paths touched by the agent |
| `commit_message` | String | Conventional commit summary |
| `notes` | String | Additional agent-produced commentary |

### Relationships
- Consumed by `tdd-core::logging::StepLogEntry` and git commit creation.
- Source for `.tdd/plan/step-XYZ-role.md` summaries.

## Entity: Step Log Entry (`tdd-core::logging::StepLogEntry`)

| Field | Type | Description |
|-------|------|-------------|
| `step_index` | u32 | Matches orchestrator counter |
| `role` | Role | Role that produced this log |
| `plan_path` | String | Relative path to plan markdown |
| `files_changed` | Vec<String> | Paths written during the step |
| `commit_id` | String | Resulting git SHA |
| `commit_message` | String | Commit subject |
| `notes` | String | Any executor notes |
| `runner` | `RunnerLog` | Captured exit codes/stdout/stderr for fmt/check/test |
| `provider` *(new)* | String | Name of LLM provider used for the step |

### Relationships
- Written under `.tdd/logs/step-XYZ-role.json` by `StepLogger`.
- `tdd-cli status` reads the latest entry to report health.

### Validation Rules
- `provider` is required once multi-provider support ships; defaults to `openai` for past logs.
- Log writer truncates stdout/stderr when >2KB to keep files readable (implementation detail to add during execution phase).

## Entity: LLM Provider Settings (`tdd-llm::config::LlmClientSettings` + provider-specific structs)

| Field | Type | Description |
|-------|------|-------------|
| `provider` | Enum (`OpenAi`, `GitHubCopilot`) | Derived from `TddConfig.llm.provider` |
| `base_url` | String | HTTP base used for API calls |
| `api_key_env` | String | Env var storing secret |  |
| `api_version` | Option<String> | Required for Copilot to populate `X-GitHub-Api-Version` |
| `role_models` | HashMap<Role, RoleModelConfig> | Model + temperature per role |

### GitHub Copilot Extension
- Additional header `X-GitHub-Api-Version` (value from `api_version`).
- Personal token must include Copilot access; CLI validates env var presence before constructing the client.

## State Transitions
1. **Initialization**: `tdd-cli init` populates config defaults, ensuring directories exist and git repo is initialized.
2. **Run Cycle**: Orchestrator loads `TddConfig`, builds `StepContext`, selects `LlmClient` based on `llm.provider`, and iterates Tester → Implementor → Refactorer.
3. **Logging**: After each successful role, `StepResult` + runner outcomes produce a `StepLogEntry` persisted to `.tdd/logs` with the provider flag.
4. **Status Inspection**: `tdd-cli status` reads the latest log and returns next role info plus provider metadata, enabling users to verify whether Copilot or OpenAI handled the previous step.
