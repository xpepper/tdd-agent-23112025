# Phase 0 Research

## Topic: Multi-provider LLM abstraction

- Decision: Extend `tdd-llm` with a `ProviderKind` enum and factory so the CLI can choose between the existing OpenAI-compatible client and a new GitHub Copilot client based on `tdd.yaml`.
- Rationale: A typed provider switch keeps logic centralized, avoids scattering `if copilot` branches across crates, and lets future providers (Azure OpenAI, local models) plug into the same trait.
- Alternatives considered: Infer provider solely from the configured base URL (fragile: Copilot API is OpenAI-compatible but needs extra headers) or fork the CLI per provider (splits behavior and complicates UX).

## Topic: GitHub Copilot API usage

- Decision: Target the GitHub Models chat completion endpoint (`POST https://api.githubcopilot.com/v1/chat/completions`) with OpenAI-compatible payloads plus required headers (`Authorization: Bearer <token>`, `X-GitHub-Api-Version: 2023-12-01`).
- Rationale: GitHub documents the Models API as OpenAI v1-compatible, so keeping the same payload structure minimizes code divergence while still complying with GitHub's versioned header policy.
- Alternatives considered: Proxy Copilot via Azure OpenAI (adds latency and loses personal-token support) or treat Copilot as a generic REST provider (would force a second request schema despite near-identical semantics).

## Topic: GitHub token handling

- Decision: Require users to supply a personal access token (with Copilot access) via an environment variable referenced in `tdd.yaml`, load it at runtime, and never persist it to disk or logs.
- Rationale: Environment variables align with existing `api_key_env` handling, keep secrets out of tracked files, and let developers rotate tokens without code changes.
- Alternatives considered: Embedding the token directly in YAML (unsafe, leaks secrets in git) or prompting interactively (breaks non-interactive automation and CI usage).

## Topic: Provider-specific observability

- Decision: Augment `.tdd/logs/step-XYZ-role.json` with the `provider` name and truncated response metadata while continuing to omit sensitive tokens.
- Rationale: Users need to know which provider generated a response to debug drift or latency differences, and the log format already captures per-role metadata without storing raw prompts.
- Alternatives considered: Recording full prompts/responses (risks leaking secrets) or omitting provider info entirely (harder to correlate behavior with cost/latency).
