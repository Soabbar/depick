# depick

> Selective dependency updates with inline release context — npm and bun.

[![CI](https://github.com/sofianeabbar/depick/actions/workflows/ci.yml/badge.svg)](https://github.com/sofianeabbar/depick/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/depick.svg)](https://crates.io/crates/depick)


```
╔ depick ── selective dependency updates ════════════════════════════════════════╗
│ Project: my-app   PM: bun   Target: latest   Write: preserve-range            │
│ Outdated: 8   Selected: 2                                                      │
╠ table (55%) ══════════════════╦ release notes (45%) ═══════════════════════════╣
│ › ● astro      6.0.8   patch  ║ ╭ wrangler  ^4.67.0 → ^4.76.0 ─────────────╮ │
│   ● dayjs     1.11.20  patch  ║ │                                           │ │
│   ○ wrangler   4.76.0  minor  ║ │  ## wrangler 4.76.0                       │ │
│   ○ maplibre   5.21.0  minor  ║ │                                           │ │
│   ○ lucide    0.577.0  minor  ║ │  Fixes                                    │ │
│   ○ tailwind    4.2.2  patch  ║ │  • fix deployment config                  │ │
│   ○ @astrojs/… 5.0.1   patch  ║ │  • improve local dev HMR                  │ │
│   ○ @astrojs/… 13.1.3  patch  ║ │                                           │ │
│                               ║ │  … 4 more lines  [m] expand               │ │
│                               ║ ╰────────────────────────────[m] expand─────╯ │
╠ wrangler  ·  ^4.67.0 → ^4.76.0  ·  minor  ·  dev ═════════════════════════════╣
║ command  bun add wrangler@^4.76.0                                              ║
║ releases https://github.com/cloudflare/workers-sdk/releases                   ║
╠ footer ════════════════════════════════════════════════════════════════════════╣
╚ [↑↓] move  [space] toggle  [m] expand  [x] apply  [q] quit ═══════════════════╝
```

## Why depick?

Most updaters tell you what's outdated. depick tells you what changed, how risky it is, and lets you apply exactly what you want... all in one terminal session.
Works with npm and bun. No broken interactive mode. No two-step workflow.

---

## Install

```bash
cargo install depick
```

Or build from source:

```bash
git clone https://github.com/sofianeabbar/depick
cd depick
cargo build --release
./target/release/depick
```

---

## Usage

```bash
# Auto-detect package manager (checks for bun.lock / bun.lockb)
depick

# Force a specific package manager
depick --pm bun
depick --pm npm

# Update only within your declared semver range
depick --target wanted

# Run in a specific directory
depick --dir ~/code/my-project
```

---

## How it works

1. **Detect** — reads your lockfile to pick npm or bun
2. **Scan** — runs `npm outdated --json` (reliable JSON), falls back to `bun outdated` text parsing
3. **Classify** — assigns a risk level to each package based on semver delta
4. **Preload** — fetches all release notes concurrently from npm registry + GitHub before the TUI opens; ≤ 20 packages run fully concurrent, larger lists are batched at 8 in-flight to stay within GitHub's rate limits
5. **Render** — opens the TUI with everything already loaded; no waiting while browsing
6. **Apply** — runs `bun add pkg@^version` or `npm install pkg@^version` for each selected package

---

## Risk classification

| Label | Meaning | Default selection |
|-------|---------|-------------------|
| `patch` | Same major.minor, patch changed | ✓ selected |
| `minor` | Same major, minor bumped | unselected |
| `0.x minor` | 0.x package, minor bump — may be breaking | unselected |
| `major` | Major version changed — review carefully | unselected |

---

## Write modes

Toggle with `w`:

| Mode | Example | Description |
|------|---------|-------------|
| `preserve-range` *(default)* | `^4.2.0 → ^4.6.1` | Keeps your `^` or `~` operator |
| `exact` | `4.2.0 → 4.6.1` | Pins to the bare version |

---

## Keys

### Main screen

| Key | Action |
|-----|--------|
| `↑` / `↓` or `j` / `k` | Move cursor |
| `space` | Toggle package selection |
| `a` | Select all |
| `i` | Invert selection |
| `t` | Toggle target: latest ↔ wanted |
| `w` | Toggle write mode: preserve-range ↔ exact |
| `o` | Open release page in browser |
| `m` | Expand full release notes (when loaded) |
| `x` or `enter` | Go to review screen (requires ≥ 1 selected) |
| `q` / `esc` | Quit |

### Release notes modal (`m`)

| Key | Action |
|-----|--------|
| `↑` / `k` | Scroll up 1 line |
| `↓` / `j` | Scroll down 1 line |
| `PgUp` | Scroll up 10 lines |
| `PgDn` | Scroll down 10 lines |
| `m` / `q` / `esc` | Close |

### Review screen

| Key | Action |
|-----|--------|
| `enter` | Apply selected updates |
| `b` | Back to package list |
| `q` | Quit |

---

## Release notes

depick fetches release context from the npm registry and GitHub for every outdated package, then renders the body as full Markdown using [`the-other-tui-markdown`](https://crates.io/crates/the-other-tui-markdown) with a custom theme:

- Headings, **bold**, *italic*, `inline code`, bullet lists, blockquotes, GFM alerts, tables
- Side panel shows a clipped preview with `… N more lines [m] expand`
- `[m]` opens a full-screen modal (90 × 85 % of the terminal) with the complete notes and scroll

---

## Roadmap

- [ ] Disk cache for release notes (skip re-fetch on restart)
- [ ] Filter view: patch only / dev only / selected only (`/` key)
- [ ] `r` key to rescan without quitting
- [ ] Numbered link hints in the modal (`o` to open)
- [ ] pnpm and yarn support
- [ ] Monorepo / workspace support
- [ ] Non-interactive CI mode: `depick --json`
- [ ] Homebrew tap

---

## Contributing

```bash
git clone https://github.com/sofianeabbar/depick
cd depick
cargo build
cargo clippy -- -D warnings   # must pass clean
cargo test
```

The codebase follows a strict three-layer architecture — domain / infra / ui — where no layer imports upward. See [`CLAUDE.md`](CLAUDE.md) for the full architecture guide.

---

## License

## License

This project is dual-licensed under **MIT** or **Apache-2.0**. Users may choose either license when using or modifying depick.

### MIT License
The `LICENSE-MIT` file contains the full MIT License text. This license is simple and permissive — suitable for most uses.

### Apache-2.0 License
The `LICENSE-APACHE` file contains the full Apache-2.0 License text. This license includes explicit patent grants and is appropriate for projects with patent concerns.

Both licenses are provided for your convenience. depick itself may be distributed under either license, and you may choose the one that best fits your needs.

---

See [`LICENSE-MIT`](LICENSE-MIT) or [`LICENSE-APACHE`](LICENSE-APACHE) for the full license texts.