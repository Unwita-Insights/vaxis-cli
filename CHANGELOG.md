# Changelog

Notable changes to `vaxis-cli`. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows the table in [`docs/vaxis-release-guide.md`](docs/vaxis-release-guide.md).

This file starts with the release below. For anything earlier, see the
[git history](https://github.com/Unwita-Insights/vaxis-cli/commits/main) and
[GitHub releases](https://github.com/Unwita-Insights/vaxis-cli/releases).

## [Unreleased]

### Removed

- **BREAKING — `vaxis apps share` is gone.** The subcommand and both its flags
  (`--revoke`, `--json`) no longer exist; invoking it now fails with clap's
  "unrecognized subcommand" error. It had shipped in every release since `v0.1.1`.

  **Why:** app-wide sharing is retired. One app link exposed *every* diagram in the
  app, so the server made link creation a hard `410 APP_SHARE_DISABLED` — leaving the
  command with no working create path. All it still did was surface or revoke a link
  minted before that cutover.

  **If you share diagrams from the CLI**, use the per-diagram command instead, and
  share the **root** diagram — its link also unlocks the sub-diagrams it drills into:

  ```bash
  vaxis diagrams share <rootDiagramId> --json
  ```

  **If you still have a legacy app-wide link**, revoke it from the web app's share
  dialog. The backend keeps serving `GET|DELETE /api/applications/:id/share` for that
  cleanup, so existing links neither break nor disappear on their own — only the CLI's
  access to them is removed.

  **If you have scripts calling `vaxis apps share`**, they will now exit non-zero.
  Drop the call if it was creating links (it has been failing with a `410` regardless),
  or move revocation to the web app.

### Added

- [AI agent quick start guide](docs/vaxis-agent-quick-start.md) — install, skill setup,
  a first prompt to send an agent, and the `CLAUDE.md`/`AGENTS.md` snippet that keeps
  diagrams in step with the code. Linked from the README.

### Changed

- `GET /api/diagrams/{id}/chat` is no longer part of the CLI-coupled backend surface.
  `diagrams show` already read `current_mermaid` straight off the diagram response; the
  contract and docs had not caught up. No user-visible behavior change.
- Documentation now consistently points at per-diagram sharing: README quickstart, the
  command tables, the core skill, `docs/test-commands.sh`, and the API reference.
