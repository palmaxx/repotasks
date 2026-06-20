# Production Readiness Assessment & Open-Source Roadmap

This document provides a comprehensive production readiness assessment for releasing **RepoTasks** as a high-quality, secure, and maintainable open-source project on GitHub. It outlines current code quality, security considerations, documentation gaps, and release pipelines, followed by a concrete action plan.

---

## What to Do Next (First Steps)

To get RepoTasks ready for its open-source debut, execute these tasks in order. These "Quick Wins" establish legal, community, and security baselines before writing complex CI/CD pipelines.

1. **Add a License File**
   - Create a `LICENSE` file (typically MIT or Apache 2.0) at the repository root.
   - Update `license` fields in [package.json](file:///Users/massimilianodepalma/Dev/repotasks/package.json) and [Cargo.toml](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/Cargo.toml).
2. **Define a Content Security Policy (CSP)**
   - Replace `"csp": null` with a strict CSP policy in [tauri.conf.json](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/tauri.conf.json) to secure the app from potential injection vulnerabilities.
3. **Update Manifest Metadata**
   - Change `authors = ["you"]` in [Cargo.toml](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/Cargo.toml) to your name and email.
   - Set the `identifier` in [tauri.conf.json](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/tauri.conf.json) to a reverse-domain format you own, such as `io.github.palmaxx.repotasks`.
   - Add description, repository URL, bugs, and homepage fields to [package.json](file:///Users/massimilianodepalma/Dev/repotasks/package.json).
4. **Set Up Formatting & Linting**
   - Add Prettier and ESLint (or Biome) to enforce code style in the frontend.
   - Add an npm script or Git pre-commit hooks to run `cargo fmt`, `cargo clippy`, and frontend formatting.
5. **Create the Initial Changelog**
   - Add an initial `CHANGELOG.md` file noting the pre-release state (`0.1.0`) and features built so far.

---

## Production Readiness Scorecard

| Category | Status | Action Items | Priority |
| :--- | :---: | :--- | :---: |
| **Legal & Compliance** | 🔴 Needs Work | Add `LICENSE`, update package manifest licenses. | High |
| **Security & Permissions** | 🟡 Action Required | Add Content Security Policy (CSP), validate path imports. | High |
| **Code Quality (Backend)** | 🟢 Solid | Refactor error handling (`Result<T, String>` to custom error enums). | Medium |
| **Code Quality (Frontend)** | 🟡 Action Required | Refactor "god components," set up linting/formatting, add Vitest. | Medium |
| **Metadata & Manifests** | 🟡 Action Required | Fill in description, authors, repository URLs, change app ID. | High |
| **Release & CI/CD** | 🔴 Missing | Create GitHub Actions release workflow for cross-compilation. | High |
| **Community & Health** | 🔴 Missing | Add `CONTRIBUTING.md`, issue templates, security policy. | Medium |
| **Gitignore & Cleanliness** | 🟢 Solid | Minor expansions for editor/OS files. | Low |

---

## Detailed Assessment

### 1. Code Quality & Architecture

#### Rust Backend (`src-tauri`)
* **State of the Code**: The Rust backend is clean and well-structured, separating CLI/API concerns ([commands.rs](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/src/commands.rs)), model representations ([models.rs](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/src/models.rs)), index state persistence ([store.rs](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/src/store.rs)), and parsing logic ([notes.rs](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/src/notes.rs)).
* **Error Handling**: Currently, commands use `.map_err(|e| e.to_string())` returning `Result<T, String>`. For production, it is highly recommended to build a custom error enum using the `thiserror` crate:
  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum Error {
      #[error("IO error: {0}")]
      Io(#[from] std::io::Error),
      #[error("Serialization error: {0}")]
      Serde(#[from] serde_json::Error),
      #[error("Project not found: {0}")]
      ProjectNotFound(String),
      // ...
  }
  // Implement serde::Serialize for Error to return clean JSON errors to TypeScript
  ```
* **Unit Testing**: Unit tests in [notes.rs](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/src/notes.rs#L173-L278) and [commands.rs](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/src/commands.rs#L237-L271) cover parsing, updates, deletes, and provisioning. This is excellent. We should extend these to cover edge cases, such as invalid UTF-8 files or deep-directory path resolutions.

#### TypeScript Frontend (`src`)
* **State of the Code**: Frontend uses React 19 and Vite 7.
* **Component Architecture**: [Board.tsx](file:///Users/massimilianodepalma/Dev/repotasks/src/Board.tsx) is currently 509 lines long and acts as a "god component" managing state, event handlers, task lists, specific entry rows, modals/menus, and SVGs. To improve maintainability and invite open-source contributions:
  * Extract SVG icons into an `Icon` component file or adopt `lucide-react`.
  * Move `EntryRow` and `TaskList` into their own files under a `src/components` directory.
  * Extract helper functions (like `formatTimestamp`, `toDateKey`, `filterEntries`, `getStats`) into a `src/utils/` directory.
* **Linting & Formatting**: There is no ESLint or Prettier configuration in the project. Setting this up is crucial to maintain a consistent style guideline for external contributors.
* **Testing**: No frontend tests exist. We recommend adding **Vitest** + **React Testing Library** to write tests for component behaviors, searching, statistics calculation, and error state displays.

---

### 2. Security Audit

Tauri runs with native system privileges. Thus, security is a top priority:

* **Content Security Policy (CSP)**:
  Currently, [tauri.conf.json](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/tauri.conf.json#L38-L40) sets `"csp": null`. In production, this must be configured to prevent Cross-Site Scripting (XSS) and remote injection vulnerabilities. A recommended policy:
  ```json
  "security": {
    "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; ipc-src 'self';"
  }
  ```
* **Path Validation & Directory Traversal**:
  In [commands.rs](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/src/commands.rs#L67), `import_project` imports any path provided by the frontend. In a desktop utility, users control which directories they import. However, we should verify that path traversal attempts (like importing directories with `..` segments or referencing files outside bounds) are handled safely or sanitized using standard Rust path canonicalization (`std::fs::canonicalize`).

---

### 3. Metadata & Manifests

To release the app as an official open-source binary, update the project metadata.

* **[Cargo.toml](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/Cargo.toml)**:
  * Update `authors = ["you"]` to your actual details.
  * Define `license = "MIT"` (or choice).
  * Add `repository = "https://github.com/yourusername/repotasks"`.
  * Ensure the package name and binary names reflect the official naming convention.
* **[package.json](file:///Users/massimilianodepalma/Dev/repotasks/package.json)**:
  * Remove `"private": true` if you plan to publish any components to npm (otherwise keep it private to avoid accidental publishing of the desktop bundle, but fill in details).
  * Fill in `"description"`, `"author"`, `"license"`, `"repository"`, `"bugs"`, and `"homepage"` fields.
* **[tauri.conf.json](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/tauri.conf.json)**:
  * Set a unique bundle identifier: `"identifier": "io.github.palmaxx.repotasks"`.
  * Check bundle details, like setting proper app descriptions and ensuring icons in `src-tauri/icons` are fully updated and generic ones are replaced.

---

### 4. License & Community Health

Open-source users and contributors require clear rules of engagement and documentation.

* **License**:
  * Create a `LICENSE` file at the root. The MIT License is recommended for utility desktop software if you want maximum adoption. Dual MIT/Apache 2.0 is the default in the Rust community.
* **Changelogs**:
  * Create a `CHANGELOG.md` at the root. Use the **Conventional Commits** standard (e.g. `feat:`, `fix:`, `docs:`) to document progress.
  * Use a tool like **Changesets** or **Release Please** to automate generating changelogs from commits when creating releases.
* **Community Documents**:
  * `CONTRIBUTING.md`: Document building instructions, prerequisites, formatting standards, and PR workflows.
  * `SECURITY.md`: Clarify how to report vulnerabilities privately.
  * `.github/ISSUE_TEMPLATE/`: Create standard bug report and feature request templates to keep GitHub issues organized.

---

### 5. CI/CD & Release Pipeline

Tauri applications compile to platform-specific native binaries. Automating builds is vital.

* **GitHub Actions Workflow**:
  Create a `.github/workflows/release.yml` workflow that triggers on tag pushes (e.g., `v*.*.*`). It should:
  * Set up Rust, Node.js, and pnpm.
  * Use `tauri-apps/tauri-action` to build binaries for macOS (`.dmg`, `.app`), Windows (`.msi`, `.exe`), and Linux (`.deb`, `.AppImage`).
  * Upload the compiled artifacts and draft a GitHub Release with the correct changelog.
* **Code Signing & Notarization**:
  * **macOS**: Unsigned apps prompt a security block. To distribute safely, you must sign and notarize the app using an Apple Developer ID certificate (`$APPLE_CERTIFICATE` and `$APPLE_API_KEY`).
  * **Windows**: Unsigned installers trigger Windows SmartScreen warnings. Buying an EV/OV code signing certificate removes these warnings, though many small open-source projects rely on unsigned binaries or self-signed certificates with installation instructions.
* **Auto-Updater**:
  Tauri provides a built-in auto-update system. If configured, you can point [tauri.conf.json](file:///Users/massimilianodepalma/Dev/repotasks/src-tauri/tauri.conf.json) to a remote JSON endpoint (such as a GitHub Gist or an automated GitHub release endpoint) to check for updates.

---

### 6. Gitignore & Cleanliness

* **[Root .gitignore](file:///Users/massimilianodepalma/Dev/repotasks/.gitignore)**:
  Add OS-specific and editor configuration entries:
  ```gitignore
  # OS files
  .DS_Store
  Thumbs.db
  
  # IDE / Editors
  .idea/
  .vscode/*
  !.vscode/extensions.json
  
  # Local logs
  *.log
  npm-debug.log*
  yarn-debug.log*
  yarn-error.log*
  pnpm-debug.log*
  
  # Tauri artifacts
  src-tauri/target/
  src-tauri/gen/
  ```

---

### 7. Feature Roadmap (Pre- & Post-Release)

To make RepoTasks a compelling open-source project, consider these features:

#### Pre-Release (Must-Haves)
* **Custom Editor Commands**: Currently, `open_in_editor` uses the default OS opener. Developers will want to configure their editor command (e.g., `code`, `cursor`, `vim`, `subl`).
* **Configurable Global Shortcut**: Currently hardcoded as `Ctrl+Alt+Space`. Provide a settings panel or read a local config file to customize this key combination.
* **Tray Configurations**: Allow turning off "minimize/hide to tray on close" for users who prefer standard taskbar window behaviors.

#### Post-Release (Good-to-Haves)
* **Markdown Rendering**: Render notes with syntax highlighting and checklists on the Board UI rather than plain text.
* **Global Search**: Add a global query option to search across *all* imported project files at once.
* **Startup on Boot**: Integrate native app autostart capabilities (e.g., via `tauri-plugin-autostart`) so RepoTasks launches quietly into the tray when the computer boots.
