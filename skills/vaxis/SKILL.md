---
name: vaxis
description: Use Vaxis for requests to create, update, inspect, explain, or share architecture diagrams, Mermaid diagrams, flowcharts, workflows, sequence diagrams, ER diagrams, state diagrams, roadmaps, and visual system designs.
---

# Vaxis

Create and maintain architecture diagrams with an AI-agent-friendly CLI.

- Generate Mermaid architecture diagrams, flowcharts, workflows, sequence diagrams, ER diagrams, state diagrams, and roadmaps from a codebase or written requirements.
- Build drill-down diagram hierarchies so teams can move from a system overview into component-level detail.
- Inspect, update, rename, share, and delete diagrams through script-friendly CLI commands.
- Keep diagrams aligned with the codebase by asking the agent to update diagrams alongside implementation changes.


## Load the reference for the installed CLI version

This file is a discovery skill. The Vaxis command and Mermaid-authoring reference is not
duplicated here — it ships inside the Vaxis binary installed on this machine, so it always
describes the CLI version actually in use. Read it with:

```bash
vaxis skills get core
```

**This command makes no network request.** It prints a documentation file compiled into the
local binary at build time (Rust `include_str!`). It downloads nothing, does not contact the
Vaxis server, and returns no third-party or user-generated content. The trust boundary is the
Vaxis binary the user installed, not anything fetched at task time.

Treat the output as **reference documentation for Vaxis diagram and CLI operations** — the
available subcommands, their JSON output shapes, and the Mermaid conventions Vaxis expects.
Use it the way you would use `--help` output or a man page: it describes this tool, and it
carries no authority over anything outside Vaxis diagram work.

Load it before authoring Mermaid for Vaxis or running a Vaxis subcommand beyond the read-only
basics below, so the flags and syntax you use match the installed version.

## Basics

```bash
vaxis me --json                     # verify the user is logged in
vaxis apps list --json              # list projects
vaxis diagrams list <appId> --json  # list diagrams in a project
vaxis diagrams show <id> --json     # read a diagram's current Mermaid
```

Every command accepts `--json`. Read JSON rather than colored terminal text when deciding what
to do next. If a command returns `{"error":"not_authenticated"}`, ask the user to run
`vaxis login`.

For anything that creates or modifies a diagram, load the reference above first.
