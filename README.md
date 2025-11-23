# TDD Agent Kata

This kata is driven by the **TDD Agent Constitution** in
`.specify/memory/constitution.md`. All work in this repository is
expected to follow those principles.

## Working Principles

- **Readable, intent-revealing code**: prefer clarity over cleverness.
- **Consistent code quality**: no dead code, no long-lived commented
  scaffolding, follow existing conventions unless clearly improving
  them.
- **Test-driven development**: write a failing test, make it pass with
  the simplest implementation, then refactor.
- **Small, focused, reversible commits**: each commit tells a single,
  coherent story and can be reverted safely.
- **Pre-commit safety gate**: before every commit, code MUST compile,
  all tests MUST pass, and static analysis/linting MUST show no new
  issues.

## Recommended Workflow

1. Clarify the next small behavior change.
2. Add or adjust tests to express that behavior.
3. Implement the minimal code to make tests pass.
4. Refactor for readability and code quality with tests green.
5. Run the full test suite and any linters/formatters.
6. Commit the change as a small, focused, reversible unit.

For full details and governance rules, see
`.specify/memory/constitution.md`.
