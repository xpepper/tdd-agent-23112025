# Quickstart: Autonomous Multi-Agent TDD Machine

## 1. Prerequisites
- Rust stable toolchain (`rustup default stable`) with `cargo`, `rustfmt`, and `clippy`.
- Git installed and globally configured.
- Access to at least one LLM provider:
  - **OpenAI-compatible** endpoint with API key, or
  - **GitHub Copilot Models** access and a personal token (`read:org`, `copilot` scopes).

## 2. Initialize the workspace
```bash
cargo run -p tdd-cli -- init
```
This creates `kata.md`, `tdd.yaml`, `.tdd/plan`, `.tdd/logs`, and ensures a git repo exists.

## 3. Configure `tdd.yaml`
Key sections to edit:
- `workspace`: confirm `kata_file`, `plan_dir`, and `log_dir` paths.
- `roles`: choose model and temperature for tester/implementor/refactorer.
- `llm`:
  ```yaml
  llm:
    provider: github_copilot   # or openai
    base_url: https://api.githubcopilot.com/v1
    api_version: 2023-12-01    # required for Copilot
    api_key_env: GITHUB_COPILOT_TOKEN
  ```
- `ci`: commands executed after each agent edit (e.g., `cargo fmt`, `cargo clippy -D warnings`, `cargo test --all`).
- `commit_author`: bot-friendly name/email used for automatic commits.

Export your token before running:
```bash
export GITHUB_COPILOT_TOKEN="ghp_your_token_here"
```

## 4. Run agents
- Execute three full cycles: `cargo run -p tdd-cli -- run --steps 3`
- Execute a single cycle: `cargo run -p tdd-cli -- step`
- Limit steps with guard rails: `cargo run -p tdd-cli -- run --steps 12 --max-attempts 1`

Every successful role writes:
- Plan markdown files under `.tdd/plan/step-XYZ-role.md`
- JSON logs (with provider name) under `.tdd/logs/step-XYZ-role.json`
- Conventional commits in git history

## 5. Inspect status & doctor
- `cargo run -p tdd-cli -- status` → shows next role, latest step log summary, and CI exit codes.
- `cargo run -p tdd-cli -- doctor` → verifies git cleanliness, CI binaries, and that the configured token/env vars are available.

## 6. When something fails
1. Read `.tdd/logs/step-XYZ-role.json` for the role that failed.
2. Check `runner.check.stderr` or `runner.test.stderr` for compiler/test issues.
3. Fix the underlying issue manually if needed, rerun `cargo fmt && cargo test --all`, then call `tdd-cli run --steps 1` to resume.

## 7. Developing with Copilot provider
- Copilot requests reuse the same prompts but add the `X-GitHub-Api-Version` header.
- Rotate your GH token regularly; the CLI never stores it on disk.
- Mix providers by editing `tdd.yaml` to switch back to OpenAI when necessary.
