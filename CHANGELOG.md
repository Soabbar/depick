# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-03-21

Initial release.

### Added

#### Core TUI

- Interactive terminal UI built with `ratatui` + `crossterm`
- Three-zone layout:
  - **Header** — project name, package manager, target mode, write mode, outdated/selected counts
  - **Body** — scrollable package table (55 %) + release notes panel (45 %) side-by-side
  - **Detail bar** — focused package name · version change · risk · kind, install command, release link
  - **Footer** — context-sensitive prompt and keybinding bar
- Full keyboard navigation: `↑`/`↓`, `j`/`k`, `space`, `a`, `i`, `t`, `w`, `o`, `x`, `m`, `q`/`Esc`

#### Package list

- Detects outdated packages via `npm outdated --json` (reliable JSON output)
- Falls back to `bun outdated` text-table parsing for Bun-native repos
- Auto-detects package manager from lockfile presence (`bun.lock` / `bun.lockb`)
- Reads `package.json` to preserve declared semver operators (`^`, `~`)
- Sorted by risk severity (Major → ZeroMinor → Minor → Patch), then alphabetically
- Scrolling table — cursor always stays in view regardless of list length

#### Risk classification

| Label | Meaning |
|-------|---------|
| `patch` | Same major.minor — low risk, pre-selected by default |
| `minor` | Same major, minor bumped |
| `0.x minor` | 0.x package minor bump — potentially breaking |
| `major` | Major version changed — starts unselected |

#### Release notes

- All release contexts fetched **concurrently at startup** before the TUI opens, using `futures::stream::buffer_unordered`
- Strategy: ≤ 20 packages → fully concurrent; > 20 packages → batched at 8 in-flight to respect GitHub API rate limits
- Inline progress counter printed to stderr during fetch (`fetching release notes 3/11`)
- Fetches npm registry metadata → resolves GitHub repository URL → hits GitHub Releases API
- Falls back to releases page URL + guessed compare URL when no exact release tag is found
- Release body rendered as styled Markdown via [`the-other-tui-markdown`](https://crates.io/crates/the-other-tui-markdown) with a custom depick theme
- Side panel shows: release title (as `## heading`), preview text clipped to available height, `… N more lines [m] expand` overflow hint
- `[m]` key opens a full-screen modal with the complete release notes and line-by-line / page scroll
- `[o]` key opens the release URL in the system browser

#### Write modes

- **preserve-range** (default) — re-uses the semver operator from `package.json` (`^4.2.0 → ^4.6.1`)
- **exact** — pins to the bare version (`4.2.0 → 4.6.1`)
- Toggle with `w`

#### Target modes

- **latest** (default) — update to the newest published version
- **wanted** — update only within the declared semver range
- Toggle with `t`

#### Review + apply flow

```
Ready → [x / enter] → Reviewing → [enter] → Applying → Done
                          ↑
                         [b] back
```

- Review screen lists every planned change with command preview and risk summary
- Apply step runs `bun add pkg@^version` or `npm install pkg@^version` per selection
- Live apply log shown during install; done screen summarises results

#### CLI flags

| Flag | Description |
|------|-------------|
| `--dir <path>` | Run against a specific project directory |
| `--pm npm\|bun` | Override package manager detection |
| `--target latest\|wanted` | Set initial target mode |

#### Theme

Single source of truth in `src/ui/theme.rs`. All Markdown rendered with a matching palette:

| Constant | Hex | Usage |
|----------|-----|-------|
| `PRIMARY` | `#2151BF` | Titles, focus marker, key hints, commands, links |
| `TEXT` | `#EAECF1` | Default prose |
| `MUTED` | `#7B818C` | Labels, borders, secondary info |
| `SUCCESS` | `#2EA043` | Selected dot, patch risk |
| `WARNING` | `#D98E04` | Minor risk, inline code |
| `ZERO_MINOR` | `#D96E00` | 0.x minor risk |
| `DANGER` | `#C23B22` | Major risk, errors |

#### Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI framework |
| `crossterm` | Cross-platform terminal backend |
| `tokio` | Async runtime (preload fetch only) |
| `reqwest` | HTTP — npm registry + GitHub API |
| `futures` | `buffer_unordered` concurrent fetch |
| `the-other-tui-markdown` | Markdown → styled ratatui `Text` |
| `serde` + `serde_json` | JSON parsing |
| `clap` | CLI argument parsing |
| `anyhow` | Error handling |
| `semver` | Version comparison (scaffolding) |
| `dirs` | XDG cache dir (scaffolding) |

---

## [Unreleased]

### Planned

- Disk cache for release notes (skip re-fetch on restart) — `dirs` crate already present
- Filter view: patch only / dev only / selected only (`/` key)
- `r` key to rescan without quitting
- Expanded release notes with link hints (numbered URL references)
- pnpm and yarn support
- Monorepo / workspace support
- Non-interactive CI mode: `depick --json`
- Homebrew tap
- Replace `tokio` full with `rt-multi-thread` only to reduce binary size

[0.1.0]: https://github.com/sofianeabbar/depick/releases/tag/v0.1.0
[Unreleased]: https://github.com/sofianeabbar/depick/compare/v0.1.0...HEAD
