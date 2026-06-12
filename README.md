# RepoTasks

RepoTasks is a desktop application built with Tauri, React, and TypeScript. It provides a simple, repo-aware quick-capture interface for managing your notes and todos directly within your projects.

## Features

- **Repo-Aware Notes & Todos**: Add notes and todos to your repositories. Data is stored directly within each project as a `NOTES.md` file, meaning your tasks stay with your code and on your disk.
- **Quick Add**: Use the global shortcut `Ctrl+Alt+Space` to instantly bring up a Quick Add capture window from anywhere, type your thought or task, pick the target repo, and get back to work.
- **Main Board**: A dedicated board view to manage all your imported repositories. Pin your favorites, filter tasks, and view quick stats (open tasks, completed tasks, notes).
- **System Tray Integration**: Runs quietly in the background with a system tray icon, ensuring quick access to the board and capture views without keeping windows open.
- **Quick Actions**: One-click actions to open a project's folder, edit the `NOTES.md` file directly in your preferred editor, or stop tracking a project without deleting its notes.

## Tech Stack

- **Frontend**: React (v19) + TypeScript, built with Vite.
- **Backend/Desktop**: Tauri (v2) using Rust.
- **System Integration**: Tauri plugins for global shortcuts, tray icons, dialogs, and native file openers.

## Getting Started

### Prerequisites

Ensure you have [Node.js](https://nodejs.org/), [pnpm](https://pnpm.io/), and [Rust](https://www.rust-lang.org/) installed, along with the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

### Installation

1. Clone the repository and navigate into it:
   ```bash
   git clone https://github.com/palmaxx/repotasks.git
   cd repotasks
   ```

2. Install dependencies:
   ```bash
   pnpm install
   ```

3. Run the development server:
   ```bash
   pnpm tauri dev
   ```

4. Build for production:
   ```bash
   pnpm tauri build
   ```

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
