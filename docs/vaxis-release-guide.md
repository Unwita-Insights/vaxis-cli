# Vaxis CLI — Release Guide

How to ship a new version of `vaxis-cli` to npm and GitHub Releases.

---

## Prerequisites (one-time setup — already done)

| What | Status |
|---|---|
| `NPM_TOKEN` secret in GitHub repo | ✅ Set |
| `npm/package.json` name is `vaxis-cli` | ✅ Done |
| `.github/workflows/npm-publish.yml` | ✅ In repo |
| npm account with publish access | ✅ Done |

---

## Release Steps

### Step 1 — Make your code changes

Edit any files in `src/` as needed.

```bash
# Example: edit a command
code src/commands/diagrams.rs
```

---

### Step 2 — Bump the version

Both files must have the same version number.

**`Cargo.toml`** (line 3):
```toml
version = "0.1.5"   # ← change this
```

**`npm/package.json`** (line 3):
```json
"version": "0.1.5"  // ← change this to match
```

---

### Step 3 — Test locally

```bash
cargo build --release
./target/release/vaxis --help
./target/release/vaxis me --json
```

---

### Step 4 — Commit and push

```bash
git add .
git commit -m "Release v0.1.5: describe what changed"
git push
```

---

### Step 5 — Tag and push (triggers CI)

```bash
git tag v0.1.5
git push --tags
```

That's it. CI takes over from here.

---

## What CI Does Automatically

After `git push --tags`, GitHub Actions starts immediately:

```
Job 1: build-and-release (6 targets in parallel, ~4 min)
  ├── darwin-arm64   → vaxis-darwin-arm64
  ├── darwin-x64     → vaxis-darwin-x64
  ├── linux-x64      → vaxis-linux-x64
  ├── linux-arm64    → vaxis-linux-arm64
  ├── linux-musl-x64 → vaxis-linux-musl-x64
  └── windows-x64    → vaxis-windows-x64.exe
        ↓ all uploaded to GitHub Release v0.1.5

Job 2: publish-npm (runs after Job 1, ~30 sec)
  └── publishes vaxis-cli@0.1.5 to npmjs.com
```

---

## Monitor the Release

Watch it live in terminal:
```bash
gh run list --repo Unwita-Insights/vaxis-cli --limit 3
gh run watch <RUN_ID> --repo Unwita-Insights/vaxis-cli
```

Or open in browser:
```
https://github.com/Unwita-Insights/vaxis-cli/actions
```

---

## After Release

Users upgrade with:
```bash
npm install -g vaxis-cli
```

Check the release on GitHub:
```
https://github.com/Unwita-Insights/vaxis-cli/releases/tag/v0.1.5
```

Check the package on npm:
```
https://www.npmjs.com/package/vaxis-cli
```

---

## Version Naming Convention

| Type | Example | When to use |
|---|---|---|
| **Patch** — bug fix | `v0.1.4` → `v0.1.5` | Fix a crash, wrong URL, bad output |
| **Minor** — new feature | `v0.1.5` → `v0.2.0` | New command, new flag, new behaviour |
| **Major** — breaking change | `v0.2.0` → `v1.0.0` | Changed command syntax, removed flags |

---

## Troubleshooting

### CI failed — how to check
```bash
gh run list --repo Unwita-Insights/vaxis-cli --limit 5
gh run view <RUN_ID> --repo Unwita-Insights/vaxis-cli --log | grep "error"
```

### npm publish failed — re-run just that job
```bash
gh run rerun <RUN_ID> --repo Unwita-Insights/vaxis-cli --failed
```

### Wrong version published — patch it
```bash
# Bump to next patch version, fix the issue, re-release
# npm does not allow republishing the same version number
git tag v0.1.6
git push --tags
```

### Tag pushed by mistake — delete it
```bash
git tag -d v0.1.5            # delete local tag
git push --delete origin v0.1.5  # delete remote tag
# CI will not run for deleted tags
```
