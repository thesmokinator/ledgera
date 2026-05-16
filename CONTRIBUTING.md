# Contributing to Ledgera

Thank you for your interest in contributing! Your help is essential for keeping
this project great.

Please note that this project is released with a
[Contributor Code of Conduct](CODE_OF_CONDUCT.md). By participating you agree to
abide by its terms.

## Issues

Issues are the best way to report bugs or suggest new features. Before opening a
new issue, please search existing ones to avoid duplicates.

When reporting a bug, include:

- Steps to reproduce the problem
- What you expected to happen
- What actually happened
- Your operating system and version
- Your journal layout or split-journal setup, if relevant
- Your hledger executable path, if relevant

When suggesting a feature, describe the use case and why it would be useful.

## Pull Requests

Pull requests are welcome. For non-trivial changes, please open an issue first
to discuss your approach.

### Getting Started

1. Fork and clone the repository
2. Install dependencies:
   ```bash
   npm install
   ```
3. Run the desktop app in development mode:
   ```bash
   npm run tauri:dev
   ```
4. Make your changes and test them
5. Push to your fork and submit a pull request

### Branch Naming

Use a descriptive prefix:

- `feat/` for new features
- `fix/` for bug fixes
- `docs/` for documentation changes

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) standard:

```
<type>: <short summary>
```

**Types:** `feat`, `fix`, `docs`, `test`, `style`, `refactor`, `chore`

Guidelines:

- Use the imperative mood
- Keep the summary under 72 characters
- Do not end the summary with a period
- Add a blank line and a body for complex changes

Examples:

```
feat: add split journal routing
fix: handle custom hledger executable path
docs: improve setup instructions
```

### Code Style

- Write all code, comments, and docstrings in English
- Follow the existing TypeScript, Rust, and CSS conventions
- Add tests for backend behavior and edge cases
- Ensure `npm run build` and `cargo test --manifest-path src-tauri/Cargo.toml` pass

## Resources

- [How to Contribute to Open Source](https://opensource.guide/how-to-contribute/)
- [GitHub Pull Request documentation](https://docs.github.com/en/pull-requests)
