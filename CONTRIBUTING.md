# Contributing to Portkeeper

Thanks for helping! A few guidelines to keep things smooth.

## How it works here

- **Anyone can open issues and pull requests.** Bug reports, ideas and fixes are all welcome.
- **Only the maintainer merges and closes.** Every pull request needs a review from @MarcAparisiTFG (see `.github/CODEOWNERS`), and `main` only changes through reviewed pull requests.
- **Licensing of contributions.** Portkeeper is under the PolyForm Noncommercial License 1.0.0. By opening a pull request you agree that your contribution is licensed under it, and that the maintainer may also include it in versions licensed commercially.

## Development setup

1. Install Rust (`https://rustup.rs`), Node.js 18+ and the Tauri prerequisites for your OS: https://tauri.app/start/prerequisites/
2. `npm install`
3. `npm run dev` starts the app with the web view reloading on changes to `src/`. Rust changes trigger a rebuild.

## Before opening a pull request

```bash
cd src-tauri
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same checks on macOS, Windows and Linux.

## Reporting a platform bug

Please attach the output of `portkeeper --dump` (the binary lives in `src-tauri/target/<profile>/`), your OS version and how Docker is installed (Docker Desktop, OrbStack, Colima…).

## Scope

Portkeeper is deliberately small: listening ports + containers, and quick actions on them. Features that keep it a one-glance tool are welcome; a full container manager is out of scope.
