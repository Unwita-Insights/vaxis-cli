Your installed Vaxis CLI provides these commands.

### Account

```powershell
vaxis login
vaxis me --json
vaxis logout
```

### Configuration

```powershell
vaxis config show --json
vaxis config set-url http://localhost:3000
vaxis config set-mode mermaid
vaxis config set-mode prompt
```

- `mermaid`: we write Mermaid directly.
- `prompt`: Vaxis server AI generates it.

### Applications

```powershell
vaxis apps list --json
vaxis apps create "Application Name" --json
vaxis apps update <app-id> --name "New Name" --json
vaxis apps update <app-id> --description "Description" --json
vaxis apps delete <app-id> --force
```

### Diagrams

```powershell
vaxis diagrams list <app-id> --json
vaxis diagrams create <app-id> "Diagram Name" --json
vaxis diagrams show <diagram-id> --json
vaxis diagrams tree <diagram-id> --json
vaxis diagrams rename <diagram-id> "New Name" --json
vaxis diagrams delete <diagram-id> --force
```

### Generate using direct Mermaid

```powershell
vaxis diagrams generate <diagram-id> `
  --mermaid "flowchart TB
    user[User] --> api[API]
    api --> db[(Database)]" `
  --json
```

Additional direct-Mermaid options include:

```powershell
--direction-policy preserve
--direction-policy auto
--explicit-direction lr
--explicit-direction tb
--fresh-generation
--viewport-width 1440 --viewport-height 900
```

### Generate using server AI

```powershell
vaxis diagrams generate <diagram-id> `
  --prompt "Create a payment architecture" `
  --intent replace `
  --json
```

Supported intents:

```text
auto, edit, replace, drill, detail, simplify, ask
```

### Ask without modifying

```powershell
vaxis diagrams ask <diagram-id> `
  --prompt "Which services access the database?" `
  --json
```

### Import existing Mermaid

```powershell
vaxis diagrams import <diagram-id> `
  --mermaid "flowchart LR
    A --> B" `
  --json
```

### History and sessions

```powershell
vaxis diagrams undo <diagram-id> --json

vaxis diagrams sessions list <diagram-id> --json
vaxis diagrams sessions create <diagram-id> --title "Architecture Updates" --json
vaxis diagrams sessions rename <diagram-id> <session-id> "New Title" --json
```

### Sharing

```powershell
vaxis diagrams share <diagram-id> --json
vaxis diagrams share <diagram-id> --rotate --json
vaxis diagrams share <diagram-id> --revoke --json
```

### Format and validation

```powershell
vaxis diagrams format --json
vaxis diagrams rules-check --json
vaxis diagrams evaluate --captures <captures.json> --json
vaxis diagrams evaluate --captures <captures.json> --output report.json
```

`format` and `rules-check` take no other flags. `evaluate` writes to stdout unless
`-o, --output <path>` is given.

### Skills and installation

```powershell
vaxis skills list --json
vaxis skills get core
vaxis skills path core
vaxis skills preview core

vaxis install --skills
vaxis install --skills --agent codex --global
vaxis install --skills --agent claude --project
vaxis install --skills --agent agents --global --yes
vaxis install --skills --agent claude --global --force
```

`skills get core` prints the authoritative reference embedded in this binary — no network
request. `--agent` accepts `claude`, `codex`, and `agents` (repeatable); Codex and `agents`
share the `.agents/skills/` path. `--yes` accepts safe defaults without prompting, `--force`
backs up and replaces a discovery skill you have edited, and `--json` reports the result
machine-readably.

Use `--json` whenever output will be read programmatically. On this Windows machine, if PowerShell blocks `vaxis.ps1`, use `vaxis.cmd` instead.
