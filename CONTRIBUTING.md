# Contributing to OpenRemote

Thank you for your interest in contributing to OpenRemote!

## Before You Start

Read [AGENTS.md](AGENTS.md) for the full engineering standards and architecture guidelines.

## Code of Conduct

Be respectful. Be constructive. Keep discussions focused on technical merit.

## How to Contribute

### Bug Reports

1. Search existing issues to avoid duplicates.
2. Include OS version, OpenRemote version, and reproduction steps.
3. For security vulnerabilities, see [SECURITY.md](SECURITY.md) — do not open a public issue.

### Feature Requests

1. Check the [milestone plan](docs/milestones/mvp-plan.md) — it may already be planned.
2. Open a discussion issue before writing code for significant changes.
3. Describe the use case, not just the implementation.

### Pull Requests

1. Fork the repository.
2. Create a focused branch: `git checkout -b feature/descriptive-name`.
3. Make small, reviewable changes.
4. Follow the coding standards in AGENTS.md.
5. Run all checks before submitting:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

6. Update documentation if your change affects documented behavior.
7. Write a clear PR description explaining what changed and why.
8. Link any related issues.

### Security-Sensitive Changes

Changes to cryptographic code, authentication, authorization, IPC, or session management require extra scrutiny. Tag your PR with `security-sensitive` and expect detailed review.

## Development Setup

See [README.md](README.md) for prerequisites and build instructions.

## Coding Standards

- **Rust**: Follow the conventions in AGENTS.md §6.
- **Kotlin**: Follow the conventions in AGENTS.md §6.
- **TypeScript/React**: Follow the conventions in AGENTS.md §6.
- **General**: Prefer readable, maintainable code over clever code.

## Dependency Policy

- Prefer established, maintained dependencies.
- Evaluate licenses before adding new dependencies.
- Document why a dependency was chosen.
- Do not add unnecessary dependencies.

## License

By contributing, you agree your contributions are licensed under Apache-2.0.
