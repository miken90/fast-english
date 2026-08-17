# AGENTS.md — Fast English

This file provides guidance when working with code in this repository.

## Role & Responsibilities

Analyze user requirements, delegate tasks to appropriate sub-agents, and ensure cohesive delivery of features that meet specifications and architectural standards.

## Workflows

- Primary workflow: `$HOME/AGENTS.md`
- Development rules: `$HOME/AGENTS.md`
- Orchestration protocols: `$HOME/AGENTS.md`
- Documentation management: `$HOME/AGENTS.md`

**IMPORTANT:** Analyze the skills catalog and activate relevant skills during the process.
**IMPORTANT:** Follow development rules in `$HOME/AGENTS.md` strictly.
**IMPORTANT:** Sacrifice grammar for concision in reports. List unresolved questions at end.

## Python Scripts (Skills)

When running Python scripts from skills, use the venv Python interpreter:
- **Linux/macOS:** `skills/.venv/bin/python3 scripts/xxx.py`

**IMPORTANT:** When scripts fail, try to fix them directly.

## [IMPORTANT] Consider Modularization
- If a code file exceeds 200 lines, consider modularizing it
- Check existing modules before creating new
- Use snake_case naming for Rust files (project convention)
- After modularization, continue with main task
- Exception: Markdown, plain text, bash scripts, config files

## Documentation Management

Keep docs in `./docs` folder:
```
./docs
├── project-overview-pdr.md
├── code-standards.md
├── codebase-summary.md
├── design-guidelines.md
├── deployment-guide.md
├── system-architecture.md
└── project-roadmap.md
```

---

## Project-Specific: Fast English (Windows Desktop App)

### Tech Stack
- **Language:** Rust (edition 2021)
- **UI:** WebView2 via `wry` + `tao` (popup + settings windows, one shared event-loop thread)
- **Async:** tokio (multi-thread)
- **Windows APIs:** `windows` crate 0.58, `uiautomation` 0.24
- **System:** tray-icon, global-hotkey, arboard (clipboard)
- **AI:** reqwest + serde_json (OpenAI/Anthropic)

### Architecture
- `src/main.rs` — entry, COM STA init, runtime, event channel
- `src/app.rs` — PopupState state machine, AppEvent, WritingToolKind
- `src/selection/` — UIA text detection (Tier 1-3), monitor polling
- `src/ui/` — WebView popup/settings hosts, IPC bridge, positioning, resize tracking, AI orchestration
- `src/ai/` — AIClient trait, OpenAI/Anthropic clients
- `src/editing/` — text replacement via UIA SetValue / clipboard paste
- `src/settings.rs` + `src/persistence.rs` — config + DPAPI key storage

See `docs/system-architecture.md` (pointer: `docs/ARCHITECTURE.md`) for the full module map and IPC flow.

### [CRITICAL] Build & Run — Windows Only

This is a **Windows-only** application. All build, check, and run commands **MUST** use PowerShell remoting.

#### Why
- Dev environment runs on WSL2 (Linux), but the app targets Windows
- Rust compilation requires Windows SDK, Win32 APIs, COM interfaces
- `cargo check` / `cargo build` will fail on Linux due to `windows`, `uiautomation`, `winreg` crates
- UIA, clipboard, hotkey features are Windows-only at runtime

#### How to Build/Check/Run

**Always use PowerShell** for cargo commands:

```bash
# Check compilation (fast, no binary output)
powershell.exe -Command "cd D:\\WORKSPACES\\PERSONAL\\fast-eng; cargo check 2>&1"

# Build debug
powershell.exe -Command "cd D:\\WORKSPACES\\PERSONAL\\fast-eng; cargo build 2>&1"

# Build release
powershell.exe -Command "cd D:\\WORKSPACES\\PERSONAL\\fast-eng; cargo build --release 2>&1"

# Run debug
powershell.exe -Command "cd D:\\WORKSPACES\\PERSONAL\\fast-eng; cargo run 2>&1"

# Run with logging
powershell.exe -Command "cd D:\\WORKSPACES\\PERSONAL\\fast-eng; \$env:RUST_LOG='fast_english=debug'; cargo run 2>&1"

# Format check
powershell.exe -Command "cd D:\\WORKSPACES\\PERSONAL\\fast-eng; cargo fmt --check 2>&1"

# Clippy lint
powershell.exe -Command "cd D:\\WORKSPACES\\PERSONAL\\fast-eng; cargo clippy 2>&1"
```

**IMPORTANT:** The WSL path `/home/canhnguyen/WORKSPACES/PERSONAL/fast-eng` maps to Windows path `D:\WORKSPACES\PERSONAL\fast-eng`. Always use the Windows path (`D:\\WORKSPACES\\PERSONAL\\fast-eng`) inside PowerShell commands.

**IMPORTANT:** After modifying any `.rs` file, run `cargo check` via PowerShell to verify compilation. Do NOT rely on Linux `cargo check` — it will show false errors from Windows-only dependencies.

#### Other Windows Commands

```bash
# Kill running instance
powershell.exe -Command "Stop-Process -Name FastEnglish -ErrorAction SilentlyContinue"

# Check if running
powershell.exe -Command "Get-Process FastEnglish -ErrorAction SilentlyContinue"
```

### Release Rules
- Repo-specific release process lives in `docs/deployment-guide.md`
- Default release scope for `fast-eng` is portable only; installer release is opt-in and requires explicit user ask
- **Version: `Cargo.toml` is the only place it is written.** There is nothing to sync by hand. It reaches everything else automatically:
  - VERSIONINFO on the `.exe` — `build.rs` passes it to `resources/app.rc` as an rc macro
  - the app manifest — `build.rs` substitutes `{{FE_VERSION_4}}` in `resources/app.manifest.in` and writes the result to `OUT_DIR`; the repo holds only the template
  - the portable zip name and the release assertions — `scripts/build-release.ps1` reads it out of `Cargo.toml`
  - the installer — `scripts/build-release.ps1 -Installer` passes `/DAppVersion` to ISCC; `installer/FastEnglish.iss` errors out if the define is missing rather than falling back to a literal
- Never add a version literal to a resource, manifest, script, or installer file. Bump `Cargo.toml` (and let `Cargo.lock` follow) and rebuild
- A release build fails if the version embedded in the built exe does not match `Cargo.toml`
- Use PowerShell for build, packaging, and release commands

### Code Conventions
- **File naming:** snake_case (Rust standard)
- **State machine:** `PopupState` enum with associated data
- **Event flow:** tokio mpsc `AppEvent` channel → shared `tao` EventLoop → typed IPC (`JsToRust`/`RustToJs`)
- **Platform gating:** `#[cfg(target_os = "windows")]` for Win32-specific code
- **Error handling:** `anyhow::Result` + `tracing` structured logging
- **Unsafe code:** minimal, only for Win32 FFI (DPAPI, COM, SendInput)
- **Formatting:** `rustfmt` standard, 4-space indent

---

## Rule: development-rules

# Development Rules

**IMPORTANT:** Follow **YAGNI** - **KISS** - **DRY** principles always.

## General
- **File Naming**: snake_case for Rust files; kebab-case for docs/scripts
- **File Size**: Keep code files under 200 lines
- Use `gh` for Github, `psql` for Postgres debugging
- **[IMPORTANT]** Follow codebase structure and code standards during implementation
- **[IMPORTANT]** Implement real code, not simulations or mocks
- **[IMPORTANT]** Use PowerShell for all cargo/build/run commands (see Build section)

## Code Quality
- Ensure no syntax errors and code compiles (via `powershell.exe cargo check`)
- Prioritize functionality and readability
- Use try-catch error handling & cover security standards

## Pre-commit/Push Rules
- Run `cargo fmt --check` and `cargo clippy` via PowerShell before commit
- Run tests before push (DO NOT ignore failed tests)
- **DO NOT** commit secrets (dotenv, API keys, credentials)
- Use conventional commit format, no AI references

## Code Implementation
- Write clean, readable, maintainable code
- Follow established architectural patterns
- Handle edge cases and error scenarios
- **DO NOT** create new enhanced files, update existing files directly
---

## Rule: documentation-management

# Project Documentation Management

### Roadmap & Changelog Maintenance
- **Project Roadmap** (`./docs/development-roadmap.md`): Track project phases, milestones, progress
- **Project Changelog** (`./docs/project-changelog.md`): Record changes, features, fixes
- **System Architecture** (`./docs/system-architecture.md`): System design documentation
- **Code Standards** (`./docs/code-standards.md`): Coding conventions and standards

### Update Protocol
1. **Before Updates**: Read current roadmap and changelog status
2. **During Updates**: Maintain version consistency and formatting
3. **After Updates**: Verify links, dates, cross-references
4. **Quality Check**: Ensure updates align with actual progress

### Plans

Save plans in `plans/` directory with timestamp and descriptive name.

**Example:** `plans/251101-1505-authentication-and-profile-implementation/`

#### File Organization

```
plans/
├── 20251101-1505-feature-name/
│   ├── research/
│   ├── reports/
│   ├── plan.md
│   ├── phase-01-setup.md
│   ├── phase-02-implement.md
│   └── phase-03-test.md
└── ...
```

##### Overview Plan (plan.md)
- Keep under 80 lines
- List phases with status/progress
- Link to detailed phase files

##### Phase Files (phase-XX-name.md)
Each should contain: Context Links, Overview, Requirements, Architecture, Related Code Files, Implementation Steps, Todo List, Success Criteria, Risk Assessment, Security Considerations, Next Steps.
---

## Rule: orchestration-protocol

# Orchestration Protocol

## Delegation Context (MANDATORY)

When spawning subagents, **ALWAYS** include:
1. **Work Context Path**: Git root of primary files
2. **Reports Path**: `{work_context}/plans/reports/`
3. **Plans Path**: `{work_context}/plans/`

#### Sequential Chaining
- **Planning → Implementation → Testing → Review**: Feature development
- **Research → Design → Code → Documentation**: New components
- Each agent completes fully before next begins

#### Parallel Execution
- Independent components can run in parallel
- Ensure no file conflicts or shared resource contention
- Plan integration points before parallel execution
---

## Rule: primary-workflow

# Primary Workflow

**IMPORTANT:** Activate relevant skills as needed. Ensure token efficiency.

#### 1. Code Implementation
- Delegate to `planner` agent first for implementation plan
- Use `researcher` agents in parallel for technical research
- **[IMPORTANT]** After modifying code, run compile check via PowerShell: `powershell.exe -Command "cd D:\\WORKSPACES\\PERSONAL\\fast-eng; cargo check 2>&1"`

#### 2. Testing
- Delegate to `tester` agent on the **final code**
- **DO NOT** ignore failing tests
- **DO NOT** use fake data, mocks, tricks to pass builds
- Fix failing tests and re-test until all pass

#### 3. Code Quality
- Delegate to `code-reviewer` agent after tests pass

#### 4. Integration
- Follow the plan from `planner` agent
- Maintain backward compatibility
- Delegate to `docs-manager` agent to update docs

#### 5. Debugging
- Delegate to `debugger` agent for issue analysis
- Fix based on report, then re-test
---

## Rule: team-coordination-rules

# Team Coordination Rules

> Only apply when operating as a teammate within an Agent Team.

## File Ownership (CRITICAL)
- Each teammate MUST own distinct files — no overlapping edits
- Define ownership via glob patterns in task descriptions
- Tester owns test files only

## Git Safety
- Prefer git worktrees for parallel work
- Never force-push from a teammate session
- Commit frequently with descriptive messages

## Report Output
- Save reports to `plans/reports/`
- Naming: `{type}-{date}-{slug}.md`
- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`

## Task Claiming
- Claim lowest-ID unblocked task first
- Set task to `in_progress` before starting work
- If all tasks blocked, notify lead

<!-- HARNESS:BEGIN -->
## Harness

Start with the requested outcome and use the repository as the system of record.
Read `docs/WORKFLOW.md` and only relevant product, design, plan, code, and
validation material.

- Answers, explanations, reviews, diagnoses, plans, and status reports are
  read-only. Inspect only what is needed; change nothing.
- For a bounded change, inspect affected behavior and proof, implement, and
  validate. No control-plane operation is required.
- Use one `docs/plans/active/` file when work spans sessions, coordinates
  contributors, has dependencies, or needs recovery. Move it to
  `docs/plans/completed/` only after validation.
- Before editing, identify repository authority for each new externally
  observable policy. If materially different choices remain open, stop before
  edits; configurable defaults are not authority.
- For architecture, reliability, security, or quality invariant work, read
  `docs/patterns/encoding-invariants.md` and enforce only accepted rules.
- Report reusable agent friction. Change guidance, tools, runbooks, or validation
  for that purpose only when explicitly asked to use `$improve-harness`.
- Also pause when product intent remains ambiguous, recovery is difficult,
  validation is weakened, or authority is insufficient.
- Claim completion only with executable or observable evidence. Report outcome,
  changes, validation, and unresolved risks.

Harness has no task database or orchestration lifecycle. Use repository plans
and behavior-level proof; do not create parallel control-plane state.
<!-- HARNESS:END -->
