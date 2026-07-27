# Vaxis + AI Agent Quick Start

Vaxis lets Claude, Codex, GLM, and other AI coding agents create and maintain
shareable architecture diagrams for your project.

## 1. Install and sign in

```bash
npm install -g @unwita-insights/vaxis@latest
vaxis login
vaxis me
```

## 2. Install the Vaxis skill

### Option A: Install from skills.sh

Use this option for Claude, Codex, GLM, or another agent that supports Agent
Skills:

```bash
npx skills add https://github.com/Unwita-Insights/vaxis-cli --skill vaxis
```

Choose your agent and either project or global scope when prompted. Restart or
reload the agent session after installation.

Vaxis skill page:
<https://www.skills.sh/unwita-insights/vaxis-cli/vaxis>

### Option B: Install using Vaxis

Run the interactive installer:

```bash
vaxis install --skills
```

Or install explicitly for Claude or Codex:

```bash
vaxis install --skills --agent claude --global --yes
vaxis install --skills --agent codex --global --yes
```

The installed file is a small discovery skill. It tells the agent to load the
complete instructions that match the installed CLI version:

```bash
vaxis skills get core
```

## 3. Give your agent this first prompt

Copy and send:

```text
Use the installed Vaxis skill for this project.

First, explain briefly what Vaxis does and run `vaxis skills get core` to load
its complete instructions. Then inspect this codebase, identify its main
components and relationships, and create a clear root architecture diagram in
Vaxis. Add useful drill-down diagrams for important services or modules. Give
me the Vaxis link when finished.
```

The agent will use commands such as:

```bash
vaxis apps list --json
vaxis apps create "<project-name>" --description "<description>" --json
vaxis diagrams create <appId> "Root Architecture" --json
vaxis diagrams generate <diagramId> --mermaid "<mermaid>" --json
vaxis diagrams show <diagramId> --json
vaxis diagrams tree <diagramId> --json
vaxis diagrams share <diagramId> --json
```

Normally, you do not need to run these diagram commands yourself—the agent uses
them after loading the skill.

## 4. Keep architecture and code growing together

Add the following text to your repository's agent instruction file:

- Claude Code: `CLAUDE.md`
- Codex: `AGENTS.md`
- GLM or another agent: its equivalent project instruction file

```text
## Vaxis architecture maintenance

Use Vaxis as the architecture source for this repository. Before creating,
reading, or updating diagrams, run `vaxis skills get core` and follow the
returned instructions.

Keep the codebase and Vaxis diagrams synchronized:

- Before implementing a change that affects architecture, inspect the existing
  Vaxis root diagram and relevant drill-down diagrams.
- If the architecture is designed in Vaxis first, implement the code to match
  the approved diagrams.
- If the code changes first, update the affected Vaxis diagrams in the same
  task.
- Add, update, or remove nodes, connections, and drill-down diagrams whenever
  services, modules, data stores, integrations, or major workflows change.
- Preserve accurate existing diagram content that is unrelated to the change.
- At the end of an architecture-impacting task, verify the diagram tree and
  provide the updated Vaxis link.
```

This makes architecture maintenance part of normal development instead of a
separate documentation task.

