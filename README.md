# Ledgera

Ledgera is a desktop app for managing hledger journals.

It lets you:

- browse transactions in a simple UI;
- switch between monthly transactions and scheduled ones;
- navigate month by month;
- explore accounts and their transactions over a time range;
- edit, create, and delete journal entries;
- work with split journals;
- use a custom `hledger` executable path when needed.

## Requirements

- Node.js and npm
- Rust toolchain
- hledger installed locally or configured in the app settings

## Run locally

Install dependencies:

```bash
npm install
```

Start the desktop app in development mode:

```bash
npm run tauri:dev
```

If you only want the frontend in the browser, run:

```bash
npm run dev
```

## Build

Create a production build with:

```bash
npm run tauri:build
```

## Configure the app

Open `Settings` and set:

- the journal file path;
- the `hledger` executable path, if it is not on the standard PATH;
- the theme preference.

## Sample journals

The repository includes sample journals under `examples/`.

Useful samples include:

- `examples/sample.journal` for a simple single-file journal;
- `examples/split-flat/` for a month-based split journal;
- `examples/split-glob/` for a year/month glob split journal;
- `examples/custom-hledger-path/` for a journal paired with a custom CLI setup.

## Project structure

- `src/` contains the React frontend;
- `src-tauri/` contains the Rust backend and Tauri commands;
- `examples/` contains sample journal layouts.

## Notes

Ledgera is designed around plain-text accounting workflows.
The backend owns the journal parsing and mutation logic, while the frontend
focuses on navigation, forms, and presentation.
