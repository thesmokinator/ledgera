# Ledgera

A desktop app for managing [hledger](https://hledger.org) journals - built with Tauri, React, and Rust.

![Screenshot](screenshots/001.png)

## Features

- **Browse & filter** - monthly transactions, scheduled entries, accounts overview with time-range filters
- **Edit, create & delete** - full CRUD on journal entries with date, status, code, description, and postings
- **Investment mode** - enter quantity, commodity, and unit price; the balancing cash posting is calculated automatically using hledger's `@` syntax
- **Split journal support** - flat, nested, and glob-based include structures are auto-detected
- **Logs** - structured event log (errors, warnings, mutations), visible for power users, with one-click copy
- **Structured errors** - backend-driven error codes with localised messages and technical details

## Requirements

- Node.js and npm
- Rust toolchain
- [hledger](https://hledger.org/install.html) installed locally or configured in the app settings

## Getting started

```bash
npm install
npm run tauri:dev        # desktop app (dev mode)
npm run dev              # frontend only (browser)
npm run tauri:build      # production build
```

## Settings

Open the **Settings** tab to configure:

| Setting | Description |
|---------|-------------|
| Journal path | Path to your main `.journal` file |
| hledger path | Custom executable path (auto-detected by default) |
| Default commodity | e.g. `EUR`, `€`, `USD` |
| Theme | System / Dark / Light |
| Power user | Show raw journal entries, line numbers, and the Logs section |

## Sample journals

The `examples/` directory contains ready-to-use journals:

- `examples/sample.journal` - single-file journal with past-month and current-month transactions, including an investment example
- `examples/split-flat/` - month-based split (`include YYYY-MM.journal`) with past-month and current-month files
- `examples/split-glob/` - year/month glob split (`include YYYY/*.journal`) with past-month and current-month files
- `examples/custom-hledger-path/` - custom CLI setup with simple past-month and current-month smoke-test entries

## Project structure

| Directory | Purpose |
|-----------|---------|
| `src/` | React frontend - routes, components, i18n |
| `src-tauri/` | Rust backend - journal parsing, mutation, logging |
| `examples/` | Sample hledger journal layouts |
| `screenshots/` | Screenshots |

## License

MIT
