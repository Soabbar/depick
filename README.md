# depick

> Selective dependency updates with inline release notes for npm and bun — built in Rust.

[![CI](https://github.com/sofianeabbar/depick/actions/workflows/ci.yml/badge.svg)](https://github.com/sofianeabbar/depick/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/depick.svg)](https://crates.io/crates/depick)


![depick demo](assets/demo.gif)

_Fast, lightweight, and terminal-native — built in Rust._

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

1. **Detects your package manager** — automatically picks npm or bun from the project context
2. **Finds outdated dependencies** — builds a focused list of packages worth reviewing
3. **Loads release notes** — pulls changelogs and release context so you can see what changed
4. **Highlights update risk** — marks patch, minor, major, and 0.x bumps at a glance
5. **Applies only what you choose** — updates just the packages you select

---

## Risk classification

| Label | Meaning | Default selection |
|-------|---------|-------------------|
| `patch` | Same major.minor, patch changed | ✓ selected |
| `minor` | Same major, minor bumped | unselected |
| `0.x minor` | 0.x package, minor bump — may be breaking | unselected |
| `major` | Major version changed — review carefully | unselected |

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

---

See [`LICENSE-MIT`](LICENSE-MIT) or [`LICENSE-APACHE`](LICENSE-APACHE) for the full license texts.
