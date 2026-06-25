# RepoTasks

RepoTasks is a small desktop app for repo-aware notes and todos. It stores project notes in each repository as `NOTES.md`, so the tasks live beside the code instead of in a separate service.

## Status

RepoTasks is currently preparing its first `0.1.0` release. Until the first GitHub Release is published, install from source.

## Features

- Repo-aware notes and todos stored in a local `NOTES.md` file.
- Quick capture window with the global shortcut `Ctrl+Alt+Space`.
- Board view for imported projects, pinned repos, filtering, and task counts.
- System tray access for the board and capture window.
- Quick actions to open a project folder, open `NOTES.md`, or remove a tracked project.
- Git sync checks for uncommitted notes, unpushed notes, remote changes, and conflict risk.
- Optional CLI helper named `rtasks` for adding projects, writing notes, toggling todos, and running note sync actions from a terminal.

## How Data Is Stored

RepoTasks writes notes and todos to a `NOTES.md` file inside each tracked project. It also writes a small `.repotasks.json` metadata file in imported repositories so the desktop app and CLI can resolve the project consistently.

RepoTasks does not require an account or hosted backend.

## Desktop App

After the first release, download the installer or package for your OS from GitHub Releases.

The desktop app is the main supported interface. It manages tracked repositories, edits notes, shows sync status, and exposes Git actions from the UI.

## CLI Helper

The CLI is built from the same repository as the desktop app, but it is not automatically installed into your terminal `PATH` by the desktop app installer.

For releases, `rtasks` should be treated as a separate GitHub Release asset:

- `rtasks-windows-x64.exe`
- `rtasks-macos-arm64`
- `rtasks-macos-x64`
- `rtasks-linux-x64`

To use it, put the binary somewhere on your `PATH`.

Common commands:

```bash
rtasks add .
rtasks list
rtasks note "Follow up on release checklist"
rtasks note "Ship v0.1.0" --todo
rtasks view
rtasks toggle 12
rtasks status
rtasks pull
rtasks push
```

If you run a command inside a tracked repository, `rtasks` uses that repo. You can also pass a project folder name or path to commands that accept one.

## Development

### Prerequisites

- Node.js
- pnpm
- Rust
- Tauri v2 system prerequisites for your OS
- Git on `PATH` for Git sync features

### Install

```bash
git clone https://github.com/palmaxx/repotasks.git
cd repotasks
pnpm install
```

### Run

```bash
pnpm tauri dev
```

### Check

```bash
pnpm lint
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

### Build

Build the desktop app:

```bash
pnpm tauri build
```

Build only the CLI:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release --bin rtasks
```

The CLI binary is written to `src-tauri/target/release/rtasks` on macOS/Linux and `src-tauri/target/release/rtasks.exe` on Windows.

## Release Checklist

For a complete first GitHub release:

- Pass `pnpm lint`, `pnpm build`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
- Build desktop packages for supported platforms.
- Build `rtasks` binaries for supported platforms.
- Publish a `v0.1.0` tag.
- Create a GitHub Release with desktop app packages and separate `rtasks` assets.

## Tech Stack

- Tauri 2
- Rust
- React 19
- TypeScript
- Vite

## License

MIT
