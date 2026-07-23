# Vaxis Skill Distribution Plan

## Goal

Let users install a small Vaxis discovery skill for Claude, Codex, and other compatible agents. When a diagram request is made, the discovery skill loads the complete, version-matched instructions from the installed Vaxis CLI.

## User Installation

```bash
npm install -g @unwita-insights/vaxis
vaxis install --skills
```

The CLI implements this command and bundles both skill files at build time.

## How It Works

1. `vaxis install --skills` copies a small discovery `SKILL.md` into the selected agent's skill directory.
2. The skill becomes available to supported agents on subsequent sessions or reloads, according to each host's refresh behavior.
3. For a relevant diagram request, the discovery skill tells the agent to run:

   ```bash
   vaxis skills get core
   ```

4. The CLI prints the complete Vaxis instructions embedded in the installed binary.
5. The agent follows those instructions to create Vaxis-compatible Mermaid.
6. In each subsequent relevant session, the agent loads the core instructions again.

No internet download is required to load the core skill.

## Repository Structure

```text
skills/
`-- vaxis/
    `-- SKILL.md          # Small discovery skill

skill-data/
`-- core/
    `-- SKILL.md          # Complete Vaxis instructions
```

The complete instructions live at `skill-data/core/SKILL.md`.

## Discovery Skill

The discovery skill lives at `skills/vaxis/SKILL.md`:

```md
---
name: vaxis
description: Use Vaxis for requests to create, update, inspect, explain, or share architecture diagrams, Mermaid diagrams, flowcharts, workflows, sequence diagrams, ER diagrams, state diagrams, roadmaps, and visual system designs.
---

# Vaxis

Before running any Vaxis command or generating Mermaid, run:

    vaxis skills get core

Read the complete output and follow all returned instructions for the remainder of the task.

Do not generate or modify a diagram before loading the core instructions.
```

## Full Skill Change

The full skill at `skill-data/core/SKILL.md` has this YAML frontmatter:

```yaml
---
name: vaxis-core
description: Complete Vaxis CLI workflow and Vaxis-compatible Mermaid authoring instructions.
---
```

The remaining full instructions can stay unchanged.

## CLI Commands

```bash
vaxis install --skills [--agent <agent>] [--project | --global] [--yes] [--force]
vaxis skills list [--json]
vaxis skills get core
vaxis skills path core
vaxis skills preview core
```

- `install --skills`: Install the discovery skill. It is interactive by default and asks for the target agents and project or global scope.
- `install --skills --agent <agent>`: Select a supported host explicitly. Allow the flag to be repeated for multiple agents.
- `install --skills --project` / `--global`: Select the installation scope without prompting.
- `install --skills --yes`: Accept safe defaults for non-interactive use.
- `install --skills --force`: Replace a modified installed discovery skill after creating a backup.
- `skills list`: List only the skills bundled with the installed CLI. `--json` provides stable structured output.
- `skills get core`: Print the embedded `SKILL.md` exactly, without decoration or network access.
- `skills path core`: Print `embedded:<binary-version>/core` when served from the binary, or the effective filesystem path if the skill is extracted.
- `skills preview core`: Display the bundled skill for human inspection without installing or activating it. It currently returns the same raw content as `get`; the separate command preserves a future human-facing preview surface.

When flags provide all required choices, installation must be non-interactive and suitable for scripts and CI. `--json` always disables prompts; missing selections return a structured error and a non-zero exit status.

JSON failures use:

```json
{"error": "stable_error_code", "message": "Human-readable details"}
```

## Skill Installation Locations

Project scope:

```text
.agents/skills/vaxis/SKILL.md
.claude/skills/vaxis/SKILL.md
```

Codex uses the shared `.agents/skills/vaxis/SKILL.md` project path, so selecting both
`--agent agents` and `--agent codex` at project scope intentionally writes one deduplicated
file.

User/global scope:

```text
~/.agents/skills/vaxis/SKILL.md
~/.claude/skills/vaxis/SKILL.md
~/.codex/skills/vaxis/SKILL.md
```

## Agent Path Mapping

Define supported hosts in one canonical mapping table in code instead of scattering path strings across commands. Each entry should contain:

- stable agent identifier used by `--agent`
- display name
- project-scope path
- global-scope path
- refresh or restart guidance

All detection, prompting, installation, and help output should use this table so adding a future host requires one mapping change.

## Installation and Upgrade Policy

The installer should report every installed path and use deterministic overwrite behavior:

1. Calculate and store the SHA-256 checksum of the discovery skill installed by Vaxis.
2. If the destination is absent, install it.
3. If its content matches the new bundled discovery skill, report it as unchanged.
4. If its content matches the checksum from the previous Vaxis installation, replace it as a safe managed upgrade.
5. If it differs from both, treat it as user-modified and do not overwrite it by default.
6. In an interactive terminal, ask before replacing a modified file.
7. In non-interactive mode, fail with a clear message unless `--force` is supplied.
8. With `--force`, copy the existing file to a timestamped `.bak` file before replacement.

The command should return a non-zero exit status for unresolved conflicts and include per-target results in `--json` output.

## Rust Embedding

Embed the full skill into the Vaxis binary:

```rust
const CORE_SKILL: &str =
    include_str!("../../skill-data/core/SKILL.md");
```

This keeps the core instructions aligned with the installed CLI version.

## Build and Packaging Validation

Compilation already fails if `include_str!` cannot find the core file. Add further build or test checks that verify:

- the core and discovery skills contain valid YAML frontmatter
- required `name` and `description` fields are present
- `vaxis skills get core` exactly matches the embedded source file
- `vaxis skills list --json` exposes every bundled skill once
- every mapped agent path produces the expected project and global destination
- packaged release binaries return the expected core skill

Release checks should fail before publishing if any validation fails.

## Security and Trust

Skills are prompt-like instructions and must be treated as executable guidance:

- `vaxis skills preview core` should let users inspect bundled content before installation.
- The installer should identify Vaxis-bundled skills as trusted, version-locked package content.
- Do not install skill content fetched from arbitrary URLs as part of `install --skills`.
- If third-party skills are supported later, require an explicit source, preview step, checksum or signature verification, and an allowlist or trust confirmation.
- Never overwrite user-modified skills silently.
