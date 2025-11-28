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

## 4. Provision the environment (optional)
If your kata requires installing toolchains, seeding datasets, or running any
other provisioning scripts, add a `workspace.bootstrap` block:

```yaml
workspace:
  kata_file: "kata.md"
  plan_dir: ".tdd/plan"
  log_dir: ".tdd/logs"
  max_steps: 10
  max_attempts_per_agent: 2
  bootstrap:
    command: ["/bin/sh", "./scripts/bootstrap.sh"]
    working_dir: "."
    skip_files:
      - ".tdd/state/bootstrap.skip"
```

- `command`: program + args executed before the first tester run.
- `skip_files`: touching any of these files (for example
  `.tdd/state/bootstrap.skip`) tells the CLI to skip provisioning unless you
  pass `--force`.
- Telemetry is written to `.tdd/logs/bootstrap-*.json` and consolidated state
  lands in `.tdd/state/bootstrap.json`.

Run the bootstrap script manually or when re-provisioning:

```bash
cargo run -p tdd-cli -- provision
# or force a rerun even if skip markers exist
cargo run -p tdd-cli -- provision --force
```

`tdd-cli init` automatically executes the bootstrap command once the block is
configured.

## 5. Run agents
- Execute three full cycles: `cargo run -p tdd-cli -- run --steps 3`
- Execute a single cycle: `cargo run -p tdd-cli -- step`
- Limit steps with guard rails: `cargo run -p tdd-cli -- run --steps 12 --max-attempts 1`

Every successful role writes:
- Plan markdown files under `.tdd/plan/step-XYZ-role.md`
- JSON logs (with provider name) under `.tdd/logs/step-XYZ-role.json`
- Conventional commits in git history

## 6. Inspect status & doctor
- `cargo run -p tdd-cli -- status` → shows next role, latest step log summary, bootstrap outcome, and CI exit codes.
- `cargo run -p tdd-cli -- doctor` → (coming soon) verifies git cleanliness, CI binaries, required env vars, and that `.tdd/state/bootstrap.json` reflects a healthy provisioning run (or highlights missing skip markers). Until diagnostics ship, the command prints a placeholder message.

## 7. When something fails
1. Read `.tdd/logs/step-XYZ-role.json` for the role that failed.
2. Check `runner.check.stderr` or `runner.test.stderr` for compiler/test issues.
3. Fix the underlying issue manually if needed, rerun `cargo fmt && cargo test --all`, then call `tdd-cli run --steps 1` to resume.

## 8. Developing with Copilot provider
- Copilot requests reuse the same prompts but add the `X-GitHub-Api-Version` header.
- Rotate your GH token regularly; the CLI never stores it on disk.
- Mix providers by editing `tdd.yaml` to switch back to OpenAI when necessary.
