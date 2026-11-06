# Development & Git Conventions

This document outlines the conventions followed in `mcp-websocket-proxy` to maintain a clean Git history, robust code quality, and predictable release management.

---

## 1. Commit Messages

We adhere to the [Conventional Commits (v1.0.0)](https://www.conventionalcommits.org/) specification.

### General Format

```
<type>(<optional scope>): <short description in imperative mood>

[optional body explaining the "why", not the "how"]

[optional footer(s): Closes #123, BREAKING CHANGE: ...]
```

### Allowed Types

| Type | Purpose |
| :--- | :--- |
| `feat` | New feature for the user |
| `fix` | Bug fix |
| `perf` | Performance improvement (RAM, CPU, I/O) |
| `refactor` | Internal code refactoring without functional change or fix |
| `docs` | Documentation only (README, guides, comments) |
| `test` | Adding or updating tests |
| `ci` | Modifications to GitHub Actions workflows or CI scripts |
| `build` | Changes to build systems, Cargo, external dependencies |
| `chore` | Maintenance tasks (tooling, formatting, .gitignore, etc.) |

### Examples

- `feat(server): add support for custom websocket ping-pong intervals`
- `fix(framing): handle incomplete json-rpc frames split across chunk boundaries`
- `perf(io): reuse allocation buffer when forwarding stdout to websocket`
- `docs: add game engine websocket integration example to readme`
- `ci: configure cargo-dist for multi-target binary builds`

### Rules

- Use the imperative, present tense for the commit subject (`add`, `fix`, `handle`, not `added` or `adds`).
- Header line length must not exceed 72 characters.
- Do not end the header line with a period.
- Avoid vague commits like `wip`, `update`, or `fix bug`.
- Signal breaking changes with a `!` after type/scope or include a `BREAKING CHANGE:` footer.

---

## 2. Branches and Workflow

### Main Branches

- `main`: Production-ready, stable branch. Any merge into `main` must pass all tests and compile with zero warnings.

### Working Branch Naming

- `feat/<short-name>`: New feature (`feat/reconnect-handling`)
- `fix/<short-name>`: Bug fix (`fix/stdio-pipe-deadlock`)
- `perf/<short-name>`: Performance enhancement (`perf/zero-copy-buffer`)
- `chore/<short-name>`: Maintenance / dependency updates (`chore/bump-tokio-version`)

### Merge Policy

- **Squash and Merge** is preferred for pull requests to maintain an atomic, linear history on `main`.
- **Rebase and Merge** may be used when a PR contains multiple distinct, well-crafted conventional commits.

---

## 3. Rust Code Standards

### Formatting and Linting

Before every commit or push, run the standard suite locally:

```bash
# Check code formatting
cargo fmt --all -- --check

# Strict static analysis
cargo clippy --all-targets --all-features -- -D warnings

# Execute test suite
cargo test --all-targets
```

### Architecture Guidelines

1. **Memory Management**:
   - Avoid redundant allocations on JSON-RPC payloads. Prefer borrowing, streaming, or direct buffer sharing.
   - Use asynchronous line/frame streams without unbounded accumulation.
2. **Error Handling**:
   - Use `thiserror` for structured domain errors.
   - Never use `unwrap()` or `expect()` in runtime execution paths (restricted to unit tests or mathematically proven invariants).
3. **Safety**:
   - Zero `unsafe` code unless an exceptional, heavily documented performance requirement dictates it.

---

## 4. Versioning and Releases

This project follows [Semantic Versioning (SemVer 2.0.0)](https://semver.org/):
- `MAJOR`: Breaking changes in CLI arguments, protocol handling, or proxy behavior.
- `MINOR`: Backward-compatible new features.
- `PATCH`: Backward-compatible bug fixes.

Releases are triggered by pushing a git tag matching `v*` (e.g. `v0.1.0`), which launches the automated multi-platform release pipeline powered by `cargo-dist`.
