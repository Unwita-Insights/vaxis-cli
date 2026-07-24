# Vaxis CLI — Command Reference

This is the maintained, user-facing command index for the Vaxis CLI. Run
`vaxis <command> --help` for the authoritative flags supported by the installed version.

All commands accept the global `--json` flag. Use it for scripts and agent-driven decisions.

## Install and skills

| Command | Description |
|---|---|
| `vaxis install --skills` | Interactively install the small Vaxis discovery skill for supported agents. |
| `vaxis install --skills --agent <agents\|claude\|codex> --project --yes --json` | Install non-interactively in the current project. Repeat `--agent` to target multiple hosts. |
| `vaxis install --skills --agent <agent> --global --yes --json` | Install the discovery skill for the current user. |
| `vaxis install --skills ... --force` | Back up and replace a user-modified installed discovery skill. |
| `vaxis skills list --json` | List skills bundled with the installed CLI. |
| `vaxis skills get core` | Print the exact embedded, version-matched core `SKILL.md`. |
| `vaxis skills path core --json` | Show the embedded source identifier for the core skill. |
| `vaxis skills preview core` | Display the bundled core skill for inspection. |

`--json` disables installer prompts. Codex uses `.agents/skills/vaxis/SKILL.md` for both
project and global scope.

## Authentication

| Command | Description |
|---|---|
| `vaxis login` | Open the browser-based Google login flow and store the CLI session. |
| `vaxis me --json` | Show the stored user profile or return `not_authenticated`. |
| `vaxis logout` | Clear the stored user session. |

## Configuration

| Command | Description |
|---|---|
| `vaxis config set-url <url>` | Save a custom Vaxis server URL. |
| `vaxis config set-mode <mermaid\|prompt>` | Choose whether the driving agent or Vaxis server AI generates diagrams. |
| `vaxis config show --json` | Show the effective server URL and saved generation mode. |

`VAXIS_AUTH_URL` overrides the stored server URL for the current process.

## Applications

| Command | Description |
|---|---|
| `vaxis apps list --json` | List applications owned by the logged-in user. |
| `vaxis apps create <name> [--description <text>] --json` | Create an application and return its ID. |
| `vaxis apps update [id] [--name <name>] [--description <text>] --json` | Update an application; omit the ID for the interactive picker. |
| `vaxis apps delete [id] [--force] --json` | Delete an application and its diagrams; `--force` skips confirmation. |
| `vaxis apps share <id> [--revoke] --json` | Inspect or revoke a legacy app-wide link. New app-wide links are retired. |

Use `vaxis diagrams share`, not `apps share`, for current sharing.

## Diagrams

| Command | Description |
|---|---|
| `vaxis diagrams list <appId> --json` | List diagrams in an application. |
| `vaxis diagrams create <appId> <name> --json` | Create an empty diagram and return its ID. |
| `vaxis diagrams generate <id> --prompt <text> [--intent <intent>] [--session <id>] --json` | Ask Vaxis server AI to generate, edit, drill, simplify, or answer. |
| `vaxis diagrams generate <id> --mermaid <source> [direction options] --json` | Save agent-authored Mermaid directly and process Vaxis drill annotations. |
| `vaxis diagrams ask <id> --prompt <question> [--session <id>] --json` | Ask about a diagram without editing it. |
| `vaxis diagrams sessions list <id> --json` | List AI chat sessions for a diagram. |
| `vaxis diagrams sessions create <id> [--title <title>] --json` | Start a new AI chat session. |
| `vaxis diagrams sessions rename <id> <sessionId> <title> --json` | Rename an AI chat session. |
| `vaxis diagrams share <id> [--rotate\|--revoke] --json` | Get/create, rotate, or revoke the diagram's public link. |
| `vaxis diagrams show <id> --json` | Show metadata, `current_mermaid`, children, and ancestry. Read before editing. |
| `vaxis diagrams tree <id> --json` | Show the full root-to-children diagram hierarchy. |
| `vaxis diagrams undo <id> --json` | Remove the last AI generation turn before retrying. |
| `vaxis diagrams rename <id> <name> --json` | Rename a diagram without changing its content. |
| `vaxis diagrams delete [id] [--app-id <appId>] [--force] --json` | Delete a diagram and all descendants. |
| `vaxis diagrams format --json` | Return the offline Mermaid authoring contract, supported types, rules, and limits. |
| `vaxis diagrams rules-check --json` | Compare the embedded authoring contract with the connected server. |
| `vaxis diagrams evaluate --captures <file> [--output <file>] --json` | Evaluate recorded direct/native Mermaid outputs against the parity catalog. |
| `vaxis diagrams import <id> --mermaid <source> --json` | Save raw Mermaid directly without calling AI. |

### Generate options

`--prompt` and `--mermaid` conflict.

| Option | Applies to | Description |
|---|---|---|
| `--intent auto\|edit\|replace\|drill\|detail\|simplify\|ask` | `--prompt` | Set the server-AI intent. Default behavior is `auto`. |
| `--session <id>` | Prompt/ask | Target an existing AI chat session. |
| `--direction-policy preserve\|auto` | `--mermaid` | Choose whether direct Mermaid direction is preserved or automatically selected. |
| `--explicit-direction lr\|tb` | `--mermaid` | Force direct Mermaid direction. |
| `--fresh-generation` | `--mermaid` | Mark new direct Mermaid as eligible for automatic direction selection. |
| `--viewport-width <n> --viewport-height <n>` | `--mermaid` | Supply the canvas dimensions used for direction decisions. |

## Global flags

| Flag | Description |
|---|---|
| `--json` | Request machine-readable output. For skill installation, it also disables prompts. |
| `--help` | Show command or subcommand help. |
| `--version` | Show the CLI version. |

Command-specific flags such as `--force`, `--yes`, `--rotate`, and `--revoke` are documented
with their commands above.

## Maintenance rule

When a CLI command, subcommand, argument, flag, or output contract changes, update this file
in the same pull request. Validate it against `vaxis --help` and the relevant nested
`--help` output.
