# Vaxis Skill Distribution Plan

## Goal

Let users install a small Vaxis discovery skill for Claude, Codex, and other compatible agents. When a diagram request is made, the discovery skill loads the complete, version-matched instructions from the installed Vaxis CLI.

## User Installation

```bash
npm install -g @unwita-insights/vaxis
vaxis install --skills
```

`vaxis install --skills` is a planned command and must be implemented.

## How It Works

1. `vaxis install --skills` copies a small discovery `SKILL.md` into the selected agent's skill directory.
2. The agent discovers the skill when a new session starts.
3. For a relevant diagram request, the discovery skill tells the agent to run:

   ```bash
   vaxis skills get core
   ```

4. The CLI prints the complete Vaxis instructions embedded in the installed binary.
5. The agent follows those instructions to create Vaxis-compatible Mermaid.
6. In each new relevant session, the agent loads the core instructions again.

No internet download is required to load the core skill.

## Planned Repository Structure

```text
skills/
└── vaxis/
    └── SKILL.md          # Small discovery skill

skill-data/
└── core/
    └── SKILL.md          # Complete Vaxis instructions
```

Move the current long `skills/SKILL.md` to `skill-data/core/SKILL.md`.

## Discovery Skill

Create `skills/vaxis/SKILL.md`:

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

Add YAML frontmatter to `skill-data/core/SKILL.md`:

```yaml
---
name: vaxis-core
description: Complete Vaxis CLI workflow and Vaxis-compatible Mermaid authoring instructions.
---
```

The remaining full instructions can stay unchanged.

## CLI Commands to Implement

```bash
vaxis install --skills
vaxis skills list
vaxis skills get core
vaxis skills path core
```

- `install --skills`: Detect supported agents, ask for project or global scope, and install the discovery skill.
- `skills list`: List the skills bundled with the CLI.
- `skills get core`: Print the complete embedded Vaxis skill.
- `skills path core`: Show the bundled or extracted skill location.

## Skill Installation Locations

Project scope:

```text
.agents/skills/vaxis/SKILL.md
.claude/skills/vaxis/SKILL.md
```

User/global scope:

```text
~/.agents/skills/vaxis/SKILL.md
~/.claude/skills/vaxis/SKILL.md
~/.codex/skills/vaxis/SKILL.md
```

The installer should report every installed path and avoid silently overwriting a modified skill.

## Rust Embedding

Embed the full skill into the Vaxis binary:

```rust
const CORE_SKILL: &str =
    include_str!("../skill-data/core/SKILL.md");
```

This keeps the core instructions aligned with the installed CLI version.
