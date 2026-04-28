# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                  # debug build
cargo build --release        # optimized build (LTO + strip enabled)
cargo install --path .       # install `cw` binary to ~/.cargo/bin
cw --help                    # verify installation
```

No test suite exists yet. Validate changes by running `cw` subcommands against a test repo. CI runs `cargo build --locked` and `cargo clippy --locked -- -D warnings` on push/PR.

## What This Is

CodeWiki is a Rust CLI (`cw`) that scaffolds and manages structured markdown wikis for codebases. The CLI handles git ops, metadata, and file scaffolding — it is intentionally not smart. The actual reading, understanding, and article-writing is done by an AI agent (Claude Code or Codex) guided by a skill file.

The binary name is `cw`, defined in `Cargo.toml` `[[bin]]`.

## Architecture

```
src/
  main.rs        — CLI entry point (clap derive), all command handlers (init, status, index, meta, path)
  config.rs      — Wiki path resolution: <repo>/llm-docs/, project name from git remote or dir name
  frontmatter.rs — YAML frontmatter parser for wiki articles (title, type, source_files, tags)
  meta.rs        — WikiMeta struct: tracks project name, repo path, last compiled commit/timestamp
  setup.rs       — Agent integration: installs skill to Claude Code, instructions to Codex, collection to QMD

skills/
  session/
    SKILL.md          — Claude Code skill file (embedded via include_str! in setup.rs)

.claude-plugin/
  plugin.json         — Plugin identity (enables --plugin-dir and GitHub marketplace install)
  marketplace.json    — Makes repo self-hostable as a marketplace

.github/workflows/
  ci.yml         — Build + clippy on push/PR
  release.yml    — Cross-platform binary builds on tag push, uploads to GitHub Releases
```

Wiki lives at `<project>/llm-docs/`. Project-local, committable to git.

## Key Design Decisions

- **Staleness detection** works by comparing `source_files` in article frontmatter against `git diff --name-only <last_compiled_commit>..HEAD`. Articles missing `source_files` won't be flagged as stale.
- **Skill is embedded** in the binary via `include_str!`. Editing `skills/session/SKILL.md` requires recompiling to update the installed plugin.
- **No API keys or network calls.** The CLI shells out to `git` and optionally `qmd`. All intelligence comes from the agent running the skill.
- **Index generation** (`cmd_index`) skips files starting with `_` (like `_meta.yaml`, `_architecture.md`, `_index.md` itself). Articles are grouped by `type` frontmatter field.

## CLI Commands

| Command | What it does |
|---------|-------------|
| `cw init` | Scaffold wiki dirs (modules/, concepts/, decisions/, learnings/, queries/) + _meta.yaml |
| `cw status` | Show files changed since last compile, list stale articles |
| `cw index` | Rebuild `_index.md` from article frontmatter |
| `cw meta update` | Write current HEAD commit to `_meta.yaml` |
| `cw path` | Print wiki path for current repo |
| `cw setup claude-code` | Install plugin to `~/.claude/plugins/local/codewiki/` and print activation instructions |
| `cw setup codex` | Append instructions to `~/.codex/AGENTS.md` |
| `cw setup qmd` | Register `./llm-docs/` as QMD search collection |
| `cw uninstall claude-code/codex` | Remove installed integrations |

## Dependencies

clap (derive), serde + serde_yaml, chrono, walkdir, anyhow, dirs. No async runtime — all operations are synchronous.
