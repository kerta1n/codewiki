use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::config;

// ---------------------------------------------------------------------------
// Embedded content
// ---------------------------------------------------------------------------

const SKILL_MD: &str = include_str!("../skills/session/SKILL.md");

const PLUGIN_JSON: &str = r#"{
  "name": "codewiki",
  "description": "Maintain a living wiki of the current codebase",
  "version": "0.0.2",
  "author": {
    "name": "kerta1n"
  },
  "repository": "https://github.com/kerta1n/codewiki"
}
"#;

const MARKETPLACE_JSON: &str = r#"{
  "name": "codewiki-local",
  "owner": {
    "name": "local"
  },
  "plugins": [
    {
      "name": "codewiki",
      "source": "./codewiki",
      "description": "Maintain a living wiki of the current codebase"
    }
  ]
}
"#;

const CODEX_AGENTS_SECTION: &str = r#"

## CodeWiki — Codebase Knowledge

You have a compiled wiki of this codebase at `./llm-docs/`. Use it.

### Session start — MANDATORY

Before doing any work, check the wiki state:

```bash
cw status
```

If the wiki is stale or uncompiled, update it:
- Read changed source files
- Update the corresponding wiki articles in `./llm-docs/`
- Run `cw index` then `cw meta update`

If no wiki exists, run `cw init` and compile from scratch.

### During work

When you need to understand how a module works, read the wiki article first:
```bash
cat llm-docs/modules/<name>.md
```

### Session end — MANDATORY

Before finishing any task that involved code changes:

1. If you fixed a bug, create `llm-docs/learnings/<slug>.md`
2. If you made a design decision, create `llm-docs/decisions/<slug>.md`
3. Update any wiki articles affected by your code changes
4. Run `cw index` then `cw meta update`

### Article format

All wiki articles use YAML frontmatter:

```yaml
---
title: Module Name
type: module
source_files:
  - path/to/file
tags: [relevant, tags]
---
```

### Rules

- Check wiki before working. Update wiki before finishing. No exceptions.
- Write for a future agent with zero context.
- Include `source_files` in frontmatter so `cw status` can detect staleness.
"#;

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn claude_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
}

fn claude_plugins_local() -> PathBuf {
    claude_home().join("plugins").join("local")
}

fn codex_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
}

// ---------------------------------------------------------------------------
// Claude Code setup
// ---------------------------------------------------------------------------

pub fn setup_claude_code() -> Result<()> {
    let local = claude_plugins_local();
    let marketplace_dir = local.join(".claude-plugin");
    let plugin_dir = local.join("codewiki");
    let plugin_manifest_dir = plugin_dir.join(".claude-plugin");
    let skill_dir = plugin_dir.join("skills").join("session");
    let skill_path = skill_dir.join("SKILL.md");

    let is_update = skill_path.exists();

    std::fs::create_dir_all(&marketplace_dir)
        .with_context(|| format!("Failed to create {}", marketplace_dir.display()))?;
    std::fs::create_dir_all(&plugin_manifest_dir)
        .with_context(|| format!("Failed to create {}", plugin_manifest_dir.display()))?;
    std::fs::create_dir_all(&skill_dir)
        .with_context(|| format!("Failed to create {}", skill_dir.display()))?;

    std::fs::write(marketplace_dir.join("marketplace.json"), MARKETPLACE_JSON)
        .with_context(|| "Failed to write marketplace.json")?;
    std::fs::write(plugin_manifest_dir.join("plugin.json"), PLUGIN_JSON)
        .with_context(|| "Failed to write plugin.json")?;
    std::fs::write(&skill_path, SKILL_MD)
        .with_context(|| format!("Failed to write {}", skill_path.display()))?;

    if is_update {
        println!("Updated codewiki plugin at {}", plugin_dir.display());
        println!();
        println!("Run /reload-plugins in Claude Code to pick up changes.");
    } else {
        println!("Installed codewiki plugin to {}", plugin_dir.display());
        println!();
        println!("To activate in Claude Code, run:");
        println!("  /plugin marketplace add {}", local.display());
        println!("  /plugin install codewiki@codewiki-local");
        println!();
        println!("Then invoke with: /codewiki:session");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Codex setup
// ---------------------------------------------------------------------------

pub fn setup_codex() -> Result<()> {
    let home = codex_home();
    let agents_path = home.join("AGENTS.md");

    let existing = std::fs::read_to_string(&agents_path).unwrap_or_default();

    if existing.contains("## CodeWiki") {
        println!("CodeWiki already installed in {}", agents_path.display());
        return Ok(());
    }

    std::fs::create_dir_all(&home)
        .with_context(|| format!("Failed to create {}", home.display()))?;

    let content = format!("{}\n{}", existing.trim_end(), CODEX_AGENTS_SECTION);
    std::fs::write(&agents_path, content)
        .with_context(|| format!("Failed to write {}", agents_path.display()))?;

    println!("Installed codewiki instructions to {}", agents_path.display());

    Ok(())
}

// ---------------------------------------------------------------------------
// QMD setup
// ---------------------------------------------------------------------------

pub fn setup_qmd() -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let wiki_path = config::wiki_path(&repo_path)?;

    if !wiki_path.exists() {
        println!("No wiki found at {}.", wiki_path.display());
        println!("Run `cw init` first.");
        return Ok(());
    }

    let qmd_check = std::process::Command::new("qmd")
        .arg("--help")
        .output();

    if qmd_check.is_err() || !qmd_check.unwrap().status.success() {
        println!("qmd not found in PATH.");
        println!("Install QMD first: https://github.com/tobi/qmd");
        return Ok(());
    }

    let project = config::project_name(&repo_path)?;
    let collection_name = format!("codewiki-{}", project);

    let output = std::process::Command::new("qmd")
        .args(["collection", "add", &wiki_path.to_string_lossy(), "--name", &collection_name])
        .output()
        .context("Failed to run qmd collection add")?;

    if output.status.success() {
        println!("Added {} collection to QMD: {}", collection_name, wiki_path.display());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already exists") {
            println!("QMD collection '{}' already exists.", collection_name);
        } else {
            println!("QMD collection add output: {}", String::from_utf8_lossy(&output.stdout));
            if !stderr.is_empty() {
                println!("stderr: {}", stderr);
            }
        }
    }

    println!("Indexing wiki articles...");
    let embed = std::process::Command::new("qmd")
        .arg("embed")
        .output()
        .context("Failed to run qmd embed")?;

    if embed.status.success() {
        println!("QMD indexing complete.");
    } else {
        println!("QMD embed warning: {}", String::from_utf8_lossy(&embed.stderr));
    }

    println!();
    println!("You can now search your wiki:");
    println!("  qmd query \"how does auth work\" -c {}", collection_name);

    Ok(())
}

// ---------------------------------------------------------------------------
// Uninstall helpers
// ---------------------------------------------------------------------------

pub fn uninstall_claude_code() -> Result<()> {
    let local = claude_plugins_local();
    if local.exists() {
        std::fs::remove_dir_all(&local)?;
        println!("Removed codewiki plugin.");
        println!();
        println!("Also run in Claude Code:");
        println!("  /plugin uninstall codewiki@codewiki-local");
    } else {
        println!("Nothing to remove.");
    }
    Ok(())
}

pub fn uninstall_codex() -> Result<()> {
    let agents_path = codex_home().join("AGENTS.md");
    if let Ok(content) = std::fs::read_to_string(&agents_path) {
        if content.contains("## CodeWiki") {
            let cleaned = remove_section(&content, "## CodeWiki");
            std::fs::write(&agents_path, cleaned.trim().to_string() + "\n")?;
            println!("Removed codewiki from Codex AGENTS.md.");
        } else {
            println!("Nothing to remove.");
        }
    } else {
        println!("Nothing to remove.");
    }
    Ok(())
}

fn remove_section(content: &str, header: &str) -> String {
    let mut result = String::new();
    let mut skipping = false;

    for line in content.lines() {
        if line.starts_with(header) {
            skipping = true;
            continue;
        }
        if skipping && line.starts_with("## ") {
            skipping = false;
        }
        if !skipping {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}
