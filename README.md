# Ledgera

Ledgera is a desktop journal manager built with Tauri v2, React, TypeScript, and hledger-compatible plain text files.

## Development

Install dependencies once with `npm install`.

Run the frontend only with `npm run dev`.

Run the full Tauri development app, including Vite and the Rust shell, with `npm run tauri:dev`.

Create a production desktop bundle with `npm run tauri:build`.

## Example journal

A sample hledger journal is available at `examples/sample.journal`. In the app, open `Settings` and set the journal path to that file to populate the dashboard and transaction outline.

## Build-size notes

The Tauri build config enables `removeUnusedCommands` and `src-tauri/Cargo.toml` contains stable Rust profile settings from the Tauri v2 app-size guide:

- incremental dev builds;
- release link-time optimization;
- single codegen unit for better optimization;
- size-oriented optimization level;
- aborting panics;
- stripped release symbols.
