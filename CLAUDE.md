# depick — CLAUDE.md

Selective dependency updates with inline release context for npm and bun projects.
Interactive TUI built in Rust with ratatui + crossterm.

## Build & run

```bash
cargo build --release
./target/release/depick

# Dev build (faster compile, slower binary)
cargo build
cargo run

# Check without building
cargo check

# Lint (must pass clean)
cargo clippy -- -D warnings

# Run in a specific project
./target/release/depick --dir ~/code/my-project
./target/release/depick --pm bun
./target/release/depick --target wanted
```

## Architecture

```
src/
├── main.rs          # Entry point: CLI parsing, preload fetch, TUI setup, event loop, keyboard routing
├── app.rs           # AppState, Phase, TargetMode, WriteMode — all mutable runtime state
│
├── domain/          # Pure business logic. No I/O, no ratatui, no std::process
│   ├── dependency.rs    # DependencyItem, DependencyKind, install_command, planned_change
│   ├── risk.rs          # RiskLevel enum + classify_risk(current, target) -> RiskLevel
│   └── release.rs       # ReleaseContext, ReleaseContextState, text normalization helpers
│
├── infra/           # All I/O and external calls
│   ├── pm.rs            # PM detection (bun.lock check), npm_outdated, bun_outdated, package.json reader
│   ├── metadata.rs      # Async fetch from registry.npmjs.org
│   └── release.rs       # GitHub release resolver, repo URL normalization, compare URL generation,
│                        # fetch_all_release_contexts (concurrent bulk preloader)
│
└── ui/              # Rendering only. Reads AppState, never mutates it
    ├── theme.rs         # All colors, styles, and md_renderer(). Primary: #2151BF. Single source of truth
    ├── header.rs        # Top summary card: project, PM, modes, counts
    ├── table.rs         # Package list with › focus marker, ●/○ selection dots, TableState scroll
    ├── details.rs       # render_notes (right panel), render_detail_bar (full-width strip), render_modal
    ├── footer.rs        # Prompt line and keybindings bar
    ├── review.rs        # Plan screen shown before apply
    └── apply.rs         # Live apply output and done screen
```

## Module rules

- `domain/` never imports from `infra/` or `ui/`
- `ui/` never imports from `infra/`
- `main.rs` is the only place that wires all three layers together
- All state mutations happen in `main.rs` event handlers or `app.rs` methods
- UI components are pure functions: `fn render(f, area, state)`

## Key data types

**`AppState`** (`app.rs`) — the single source of truth for the TUI:
- `items: Vec<DependencyItem>` — all outdated packages
- `selected_row: usize` — cursor position
- `release_contexts: ReleaseContextStore` — HashMap keyed by `ReleaseCacheKey`, fully populated before the TUI opens
- `phase: Phase` — `Ready | Reviewing | Applying | Done`
- `target_mode: TargetMode` — `Latest | Wanted`
- `write_mode: WriteMode` — `PreserveRange | Exact`
- `show_notes_modal: bool` — whether the full release notes modal is open
- `modal_scroll: u16` — scroll offset inside the modal
- `md_renderer: Renderer` — `the-other-tui-markdown` renderer, built once at startup

**`DependencyItem`** (`domain/dependency.rs`):
- `install_command(&pm, &write_mode) -> String` — generates the exact command to run
- `planned_change() -> String` — display string like `^4.67.0 → ^4.76.0`

**`RiskLevel`** (`domain/risk.rs`):
- `Patch | Minor | Major | ZeroMinor | Unknown`
- `classify_risk(current, target)` — pure function, no I/O
- `auto_select()` — returns true only for Patch (default selection behavior)

**`ReleaseContextState`** (`domain/release.rs`):
- `Loaded(Box<ReleaseContext>) | Failed(String)`
- All entries are populated by `fetch_all_release_contexts` before the TUI opens
- `Box<ReleaseContext>` keeps the enum small (avoids large_enum_variant lint)

## Phase flow

```
Ready → (x or enter with selections) → Reviewing → (enter) → Applying → Done
         (b from Reviewing goes back to Ready)
```

## Startup flow (before TUI opens)

```
main()
  │
  ├─ parse CLI args
  ├─ detect package manager (bun.lock / bun.lockb)
  ├─ run npm/bun outdated → Vec<DependencyItem>
  ├─ print "fetching release notes  0/N" to stderr
  ├─ rt.block_on(fetch_all_release_contexts(...))
  │     └─ stream::iter(items).buffer_unordered(concurrency)
  │           ├─ fetch_metadata (registry.npmjs.org)
  │           └─ resolve_release_context (GitHub releases API)
  ├─ print "fetching release notes  N/N  done" to stderr
  ├─ AppState::new(...)  ← md_renderer built here
  ├─ state.release_contexts = preloaded store
  └─ enable_raw_mode() → TUI opens with all data ready
```

Concurrency strategy in `fetch_all_release_contexts`:
- ≤ 20 packages → all requests fly concurrently
- > 20 packages → `buffer_unordered(8)` caps in-flight connections

## Data flow for outdated packages

1. `infra/pm::detect(dir)` — checks for `bun.lock` / `bun.lockb`
2. `infra/pm::npm_outdated(dir, target_mode)` — runs `npm outdated --json`, parses JSON
3. For Bun repos: tries npm first, falls back to `infra/pm::bun_outdated` (text parsing)
4. `infra/pm::read_declared_ranges(dir)` — reads `package.json` for `^` / `~` operators
5. `domain/risk::classify_risk` — computes risk per package
6. Packages sorted: Major → ZeroMinor → Minor → Patch, then alphabetical

## Release notes fetch flow

Runs fully before the TUI opens via `infra/release::fetch_all_release_contexts`:
1. `infra/metadata::fetch_metadata(client, name)` — hits `registry.npmjs.org/{name}/latest`
2. `infra/release::normalize_repo(url)` — normalizes git+https / git@ / github: URLs
3. `infra/release::find_github_release(client, repo, pkg, target)` — tries GitHub releases API
4. Falls back to releases page URL + guessed compare URL
5. `domain/release::build_preview(text, 400)` — strips markdown noise, clips to 400 chars
6. Result stored as `ReleaseContextState::Loaded(Box::new(ctx))` or `::Failed(msg)`

## Layout

```
╔ header (full width) ════════════════════════════════════════════╗
║  table (55%)                 ║  ╭ pkg  ^old → ^new ──────────╮ ║
║  › ● astro      6.0.8  patch ║  │  ## Release title           │ ║
║    ● dayjs     1.11.20 patch ║  │  body preview (markdown)    │ ║
║    ○ wrangler   4.76.0 minor ║  │  … N more lines  [m] expand │ ║
║    …                         ║  ╰──────────[m] expand─────────╯ ║
╠ name · change · risk · kind (full width) ═══════════════════════╣
║ command  bun add pkg@^version                                    ║
║ releases https://github.com/…/releases                          ║
╠ footer prompt ══════════════════════════════════════════════════╣
╚ [↑↓] move  [space] toggle  [m] expand  [x] apply  [q] quit ════╝
```

`[m]` opens a full-screen modal overlay (90% × 85%) with the complete release notes,
full Markdown rendering, and `↑`/`↓`/`PgUp`/`PgDn` scroll.

## Markdown rendering

All release note text is rendered with `the-other-tui-markdown` via a custom depick-themed
`Renderer` built once in `AppState::new()` and stored as `state.md_renderer`.

The renderer is constructed in `ui/theme::md_renderer()` — the single source of truth for
Markdown element styles. Never call `into_text(...)` with the default theme; always use
`into_text_with_renderer(src, &state.md_renderer)`.

## Theme

All colors defined in `src/ui/theme.rs`. Never use raw `Color::Rgb` values outside that file.

| Constant | Hex | Usage |
|----------|-----|-------|
| `PRIMARY` | `#2151BF` | Title, focus marker, active key hints, commands, links |
| `TEXT` | `#EAECF1` | Default readable text |
| `MUTED` | `#7B818C` | Labels, secondary info, borders |
| `SUCCESS` | `#2EA043` | Selected dot, patch risk, done state |
| `WARNING` | `#D98E04` | Minor risk, inline code |
| `ZERO_MINOR` | `#D96E00` | 0.x minor risk |
| `DANGER` | `#C23B22` | Major risk, failed apply |

## Keybindings (Ready phase)

| Key | Action |
|-----|--------|
| `↑` / `↓` or `k` / `j` | Move cursor |
| `space` | Toggle selection |
| `a` | Select all |
| `i` | Invert selection |
| `t` | Toggle target: latest ↔ wanted |
| `w` | Toggle write mode: preserve-range ↔ exact |
| `o` | Open release page in browser |
| `m` | Open full release notes modal (when loaded) |
| `x` / `enter` | Go to review screen (requires ≥1 selected) |
| `q` / `esc` | Quit |

## Keybindings (modal open)

| Key | Action |
|-----|--------|
| `↑` / `k` | Scroll up 1 line |
| `↓` / `j` | Scroll down 1 line |
| `PgUp` | Scroll up 10 lines |
| `PgDn` | Scroll down 10 lines |
| `m` / `q` / `esc` | Close modal |

## Dependencies (direct)

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI framework |
| `crossterm` | Cross-platform terminal backend |
| `tokio` | Async runtime (preload fetch only) |
| `reqwest` | HTTP client — npm registry + GitHub API |
| `futures` | `buffer_unordered` for concurrent preload |
| `the-other-tui-markdown` | Markdown → styled ratatui `Text` |
| `serde` + `serde_json` | JSON parsing (npm outdated, registry) |
| `clap` | CLI argument parsing |
| `anyhow` | Error handling |
| `thiserror` | Typed errors |
| `semver` | Version comparison (scaffolding for future use) |
| `dirs` | XDG cache dir (scaffolding for future disk cache) |

## Known dead code (intentional scaffolding)

All items below carry `#[allow(dead_code)]` and must not be removed — they are
reserved for upcoming features documented in the roadmap.

| Symbol | Location | Future use |
|--------|----------|------------|
| `Phase::Scanning` | `app.rs` | Async scan with progress indicator |
| `ActivePane::Details` | `app.rs` | Split-pane focus mode |
| `AppState::active_pane` | `app.rs` | Split-pane focus state |
| `AppState::error_message` | `app.rs` | In-TUI error display |
| `AppState::release_context_state()` | `app.rs` | Convenience accessor for UI |
| `LinkConfidence::label()` | `domain/release.rs` | Confidence badge in details pane |
| `PackageMetadata::name` | `infra/metadata.rs` | Symmetry; may be used by disk cache |
| `PRIMARY_DIM` | `ui/theme.rs` | Inactive border styling |
| `semver` crate | `Cargo.toml` | Smarter version comparison |
| `dirs` crate | `Cargo.toml` | Disk cache for release notes |

## Roadmap

- [ ] Disk cache for release notes (avoid re-fetching on restart) — `dirs` + `serde_json`
- [ ] Filter view: patch only / dev only / selected only (`/` key)
- [ ] `r` key to rescan without quitting
- [ ] Expanded link hints in modal (numbered URL references via `RendererBuilder::with_link`)
- [ ] pnpm and yarn support
- [ ] Monorepo / workspace support
- [ ] Non-interactive CI mode: `depick --json`
- [ ] Publish to Homebrew tap
- [ ] Replace `tokio` full feature set with `rt` only to reduce binary size