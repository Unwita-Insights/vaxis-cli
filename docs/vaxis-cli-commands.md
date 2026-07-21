# Vaxis CLI — Command Reference

All commands support `--json` flag for machine-readable output (used by Claude AI).

---

## Auth

| Command | Status | Description |
|---------|--------|-------------|
| `vaxis login` | ✅ Built | Opens browser for Google OAuth. Polls until complete. Saves token to `~/.config/vaxis/config.toml` |
| `vaxis logout` | ✅ Built | Clears saved user token. Preserves server URL config |
| `vaxis me --json` | ✅ Built | Shows logged-in user name and email. Returns `{"error":"not_authenticated"}` in JSON mode if not logged in |

---

## Config

| Command | Status | Description |
|---------|--------|-------------|
| `vaxis config set-url <url>` | ✅ Built | Saves the Vaxis server URL to config file. Allows `vaxis login` to work without setting `VAXIS_AUTH_URL` env var every time |
| `vaxis config show` | ✅ Built | Shows current config: server URL and logged-in user |

---

## Apps

| Command | Status | Description |
|---------|--------|-------------|
| `vaxis apps list --json` | ✅ Built | Lists all applications owned by the logged-in user. Returns array of `{ id, name, description, created_at }` |
| `vaxis apps create <name> --json` | ✅ Built | Creates a new application (project). Optional `--description` flag. Returns created app with ID |
| `vaxis apps update [id] --name --description` | ✅ Built | Updates app name or description. Omit ID for interactive picker. Omit flags for pre-filled interactive input |
| `vaxis apps delete [id] --force` | ✅ Built | Deletes an application and all its diagrams. Omit ID for interactive picker. `--force` skips confirmation |
| `vaxis apps share <id> [--revoke] --json` | ✅ Built | **Legacy read/revoke only** — app-wide share creation is retired (server returns 410). Reports a still-live legacy app link and, with `--revoke`, turns it off. Share individual diagrams with `vaxis diagrams share` instead |

---

## Diagrams

| Command | Status | Description |
|---------|--------|-------------|
| `vaxis diagrams list <appId> --json` | ✅ Built | Lists all diagrams in an application. Shows which are root vs child. Returns `{ id, name, parent_diagram_id }` per diagram |
| `vaxis diagrams create <appId> <name> --json` | ✅ Built | Creates a new empty diagram inside an application. Returns `{ id, name }` |
| `vaxis diagrams generate <id> --prompt "..." [--intent auto\|edit\|replace\|drill\|detail\|simplify\|ask] [--session <id>] --json` | ✅ Built | Sends prompt to server AI. May return an edit (`{ mermaid, drills[] }`) OR a no-op — an Ask `answer`, a `notice`, a `mode_mismatch`, or a delete `actions` confirmation (`unchanged` / no drills). Use `--mermaid` instead when Claude is the AI |
| `vaxis diagrams generate <id> --mermaid "..." --json` | ✅ Built | Saves Claude-provided Mermaid directly — server skips AI but still parses drill annotations and creates child diagrams. Returns `{ diagram_id, mermaid, drills[] }` |
| `vaxis diagrams ask <id> --prompt "..." [--session <id>] --json` | ✅ Built | Asks a question about a diagram — server AI answers in prose, makes no edit |
| `vaxis diagrams share <id> [--rotate] [--revoke] --json` | ✅ Built | Get/create the per-diagram public link (covers the diagram + its drill subtree). Plain call returns the existing link; `--rotate` mints a new one (invalidates the old); `--revoke` turns sharing off |
| `vaxis diagrams sessions list\|create\|rename <id> ... --json` | ✅ Built | Manage server-AI chat sessions for a diagram |
| `vaxis diagrams show <id> --json` | ✅ Built | Shows diagram metadata + current Mermaid (`current_mermaid` from the diagram record) + child node map. Claude reads this before every generate call |
| `vaxis diagrams tree <id> --json` | ✅ Built | Shows the full diagram hierarchy from root down to all descendants as a nested tree. Claude uses this to find the right diagram to update |
| `vaxis diagrams undo <id>` | ✅ Built | Removes the last AI-generated user+assistant message pair from chat history. Safe to call before retrying a bad generation |
| `vaxis diagrams rename <id> <name>` | ✅ Built | Renames a diagram. Does not affect content or child diagrams |
| `vaxis diagrams delete [id] --force` | ✅ Built | Deletes a diagram and all its child diagrams recursively. Omit ID for interactive picker. `--force` skips confirmation |
| `vaxis diagrams format` | ✅ Built | Returns the full Mermaid format reference: editable vs image-fallback diagram types, a working example for each, node ID rules, drill annotation syntax, and limits. Runs without auth. Claude calls this at the start of complex sessions |
| `vaxis diagrams import <id> --mermaid "..."` | ✅ Built | Saves user-provided raw Mermaid directly to a diagram without calling AI. Used when the user pastes Mermaid from another tool or doc |

---

## Status Summary

| Area | Built | Needed | Total |
|------|-------|--------|-------|
| Auth | 3 | 0 | 3 |
| Config | 2 | 0 | 2 |
| Apps | 5 | 0 | 5 |
| Diagrams | 13 | 0 | 13 |
| **Total** | **23** | **0** | **23** |

---

## Global Flags

| Flag | Applies to | Description |
|------|-----------|-------------|
| `--json` | All commands | Outputs raw JSON instead of colored text. Required for Claude AI to parse output and make decisions |
| `--force` | delete commands | Skips confirmation prompt. Used by Claude in scripting mode |
| `--help` | All commands | Shows command usage and available flags |
| `--version` | Root | Shows CLI version |
