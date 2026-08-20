# Vaxis Architecture Version Control Plan

## Accepted implementation direction (supersedes earlier multi-file examples below)

The shipped format is one portable architecture document plus one sync manifest:

```text
architecture/
├── architecture.vaxis.mmd
└── vaxis.yaml
```

`architecture.vaxis.mmd` contains ordinary root Mermaid and recursive
`%% vaxis:drill` / `%% vaxis:drill-line` payloads. Importing this one file into Vaxis
reconstructs the complete drill hierarchy. The CLI obtains it from
`GET /api/diagrams/:rootId/export/mermaid`; `vaxis.yaml` records the root ID, file, schema,
and synchronized hash.

Two flows are supported. After reviewing the canvas, export from the current `scene_json`
tree. In hands-free use, the user may approve file creation immediately after successful
generation/import; unopened levels use `current_mermaid` plus `child_nodes`, so opening Vaxis
is not required. In both cases, Claude, Codex, or another interactive agent asks the same
structured Yes/No version-control question before writing repository files.

The older per-diagram `diagrams/*.mmd` and generated README layouts retained later in this
historical planning document are rejected alternatives, not the implementation contract.

## Status

- In progress on `feature/architecture-version-control`
- Target product name: **Vaxis Architecture as Code**
- Working CLI name: **repository sync**
- Initial format: Mermaid (`.mmd`) plus a Vaxis manifest

Implemented in the first CLI slice:

- `vaxis diagrams sync init`
- `vaxis diagrams sync status`
- `vaxis diagrams sync pull` with `--dry-run`
- Versioned YAML manifest and one `.mmd` file per drill-tree diagram
- Content hashing, drift/conflict classification, symlink-safe paths, and rollback-safe staged writes
- AI-assistant onboarding and consent instructions

`sync push` remains pending until the backend provides expected-revision enforcement. A
client-only check cannot close the race between reading a remote diagram and overwriting it.

### Backend prerequisite discovered during implementation

Live API validation found current visual root diagrams whose `GET /api/diagrams/:id`
response contains `current_mermaid: null` even though `scene_json`, `scene_version`, and drill
children exist. The CLI refuses these diagrams with `mermaid_unavailable`; it never creates a
blank `.mmd` file. Before general release, the backend must either persist/reconstruct Mermaid
for every editable diagram or expose a stable, versionable scene snapshot with a documented
round-trip import contract. Mermaid remains the preferred Git format because raw scene JSON is
larger and produces noisier reviews.

## Summary

Vaxis users should be able to store their architecture beside their application code, review architecture changes in pull requests, recover earlier versions through Git, and synchronize those files with the collaborative diagrams hosted by Vaxis.

The repository is the durable source of truth. Vaxis remains the visual collaboration, AI-editing, drill-down, and sharing environment. Synchronization is explicit so that a web edit, CLI edit, and Git branch cannot silently overwrite one another.

The complete design contains four user workflows. Version 0.5.16 implements the first three;
the fourth remains blocked on a backend concurrency contract:

1. Link and export a Vaxis diagram tree into a repository.
2. Show whether local and remote architecture are in sync.
3. Pull remote Vaxis changes into repository files.
4. Push reviewed repository changes back to Vaxis.

AI assistants such as Claude Code, Codex, and other skill-compatible agents should also offer this workflow at the appropriate point in diagram creation. They must ask before creating architecture files in the user's repository.

## Problem

Today, Vaxis persists diagram content remotely. A team can collaborate and share diagrams, but its architecture history is separate from the history of the code it describes. This creates several gaps:

- Architecture changes cannot naturally participate in code review.
- A release tag does not capture the architecture for that release.
- Teams cannot use normal Git diffs, branches, approvals, or rollback.
- CI cannot easily verify that checked-in architecture is valid and synchronized.
- A remote-only diagram may become disconnected from the implementation repository.

Exporting one Mermaid file helps, but it is not sufficient for Vaxis because a root diagram can drill into multiple child diagrams. The complete versioned artifact must preserve the whole diagram tree and its Vaxis identity.

## Goals

- Store human-readable Mermaid in Git.
- Preserve the complete Vaxis drill hierarchy.
- Make architecture changes understandable in normal Git diffs.
- Detect local changes, remote changes, and true conflicts before writing.
- Never silently overwrite a newer local or remote version.
- Support humans, AI assistants, and CI through stable JSON output.
- Keep credentials and sharing secrets out of the repository.
- Work without requiring Vaxis to perform Git commits or GitHub operations.
- Reuse the existing Vaxis diagram, tree, lint, and import/generate capabilities where safe.

## Non-goals for the first release

- Automatically committing or pushing Git changes.
- Creating pull requests.
- A GitHub App or webhook integration.
- Continuously watching files in the background.
- Automatically merging conflicting Mermaid.
- Versioning chat history, Excalidraw scene JSON, or AI sessions.
- Supporting PlantUML, D2, or arbitrary diagram formats.
- Reconstructing architecture automatically from source code.

## Product principles

### Git is the durable source of truth

Once a diagram is linked, committed repository files represent the reviewable architecture. Vaxis may still be edited directly, but those changes must be pulled and committed before they become part of the repository's history.

### Synchronization is explicit

The CLI should not unexpectedly modify a working tree after unrelated commands. Users invoke `pull` or `push`, inspect the result, and decide whether to commit it.

### No silent data loss

If both local and remote content changed after the last successful synchronization, the CLI reports a conflict and writes neither side over the other.

### Mermaid remains readable

The committed `.mmd` files should contain valid Mermaid without generated wrappers, credentials, timestamps, or other diff noise.

### Vaxis identity lives in a manifest

Stable application, diagram, parent, and drill-node mappings belong in one small manifest rather than in Mermaid comments.

### Repository linking requires consent

Creating local architecture files changes the user's codebase. An AI assistant must explain the proposed repository location and ask for confirmation before initializing version control. Normal diagram creation must not silently enable repository synchronization.

## AI-assistant onboarding flow

Vaxis is commonly driven through Claude Code, Codex, or another AI assistant. The bundled Vaxis instructions should teach every supported assistant when and how to offer architecture version control.

### When the assistant should offer it

Offer repository version control when all of the following are true:

- The assistant has just created or substantially updated a meaningful root architecture.
- The session is operating inside a detected source-code repository.
- No Vaxis sync manifest is already present for that repository.
- The user has not already declined the offer during the current project/session.
- The session is interactive rather than automated/CI.

The assistant should also start the workflow immediately when the user explicitly asks to version, export, track, commit, or keep the architecture in Git.

### Recommended question

After completing the diagram and reporting its result, ask one concise question:

> Do you want to version-control this architecture with your project? I can create Mermaid files and a Vaxis manifest in `architecture/` so changes can be reviewed and committed with Git.

If the interface supports structured choices, offer:

- **Enable version control** — preview the files and initialize repository sync.
- **Not now** — leave the diagram only in Vaxis and do not ask again during the current session.

Use the host's structured question tool whenever available (Claude Code's
`AskUserQuestion`, Codex's question/request-input tool, or an equivalent). Use the header
`Git history`, the question `Do you want to version-control this architecture diagram with
your project?`, and explicit **Yes, create files** / **No, not now** choices.

The assistant should not imply that enabling this feature automatically creates a Git commit or pushes anything to GitHub.

### What happens after acceptance

1. Determine the root diagram ID from the current conversation or diagram result; never guess it.
2. Check whether `architecture/vaxis.yaml` or another configured sync manifest already exists.
3. Preview the directory, files, diagram count, and root diagram that will be linked.
4. Ask for final confirmation if the preview differs materially from the original offer.
5. Run the repository initialization command.
6. Report the created files and recommend reviewing `git diff` before committing.
7. Show `git add architecture/`, a suggested commit command, and `git push` as instructions.
   Do not execute Git writes unless separately and explicitly authorized; initialization
   consent covers repository architecture files, not committing or pushing them.

The intended command is:

```text
vaxis diagrams sync init <rootDiagramId> [--dir architecture] [--json]
```

`vaxis diagrams init` does not exist today and should not be documented as an existing command. The `sync` namespace distinguishes repository linking from ordinary diagram creation.

### What happens after rejection

- Make no repository changes.
- Continue using Vaxis normally.
- Do not repeat the offer after every diagram edit.
- Offer it again only in a later session or when the user explicitly requests version control.

### Existing linked repositories

When a manifest already exists, the assistant should not ask whether to enable version control. It should check synchronization state when relevant and describe the intended action:

- Before publishing local architecture edits: check status and explain that revision-safe
  `sync push` is not available yet.
- After known Vaxis/web edits: check status, then propose `sync pull`.
- When both sides changed: report the conflict and ask which version to preserve; never force a direction.

### Automated and CI usage

Automated sessions must never display the onboarding question. Repository sync runs only when explicitly configured through command arguments or workflow files. CI should normally use read-only validation such as `sync status --check` and must not push remote changes unless the workflow explicitly authorizes it.

## Proposed repository layout

Default layout:

```text
architecture/
├── vaxis.yaml
├── README.md
└── diagrams/
    ├── system.mmd
    ├── authentication.mmd
    ├── payments.mmd
    └── notifications.mmd
```

The directory should be configurable, but the initial CLI can default to `architecture/`.

### Why separate `.mmd` files

- GitHub renders `.mmd` and `.mermaid` files directly.
- Each subsystem receives a focused diff.
- File ownership can follow team or CODEOWNERS boundaries.
- A root diagram does not need to embed the complete content of every drill child.
- Renames and additions remain visible as normal file operations.

## Manifest format

Proposed `architecture/vaxis.yaml`:

```yaml
schema_version: 1
application_id: app_123
root_diagram: system

diagrams:
  system:
    id: diag_root
    name: System Architecture
    file: diagrams/system.mmd
    synced_hash: sha256:012345
    remote_revision: 42

  authentication:
    id: diag_auth
    name: Authentication
    file: diagrams/authentication.mmd
    parent: system
    parent_node: auth
    synced_hash: sha256:abcdef
    remote_revision: 8
```

### Manifest requirements

- `schema_version` enables future migrations.
- Diagram keys are stable local aliases and must be unique.
- Paths are relative to the manifest directory.
- Paths must not escape the architecture directory.
- `parent` and `parent_node` reconstruct the drill tree.
- `synced_hash` is calculated from normalized Mermaid.
- `remote_revision` is an opaque server-controlled revision.
- Entries should have deterministic ordering to reduce diff noise.

### Data that must never be stored

- Authentication tokens
- Share tokens or edit tokens
- User names or email addresses
- Chat session IDs
- Server request IDs
- Absolute machine-specific paths
- Volatile synchronization timestamps
- Complete `scene_json`

The configured Vaxis host should continue to come from existing CLI configuration. A future optional `server` field may be useful for self-hosted projects, but it must never contain credentials.

## CLI design

The command family should be repository-oriented rather than overloading individual diagram import/export commands.

### Initialize a repository link

```text
vaxis diagrams sync init <rootDiagramId> [--dir architecture] [--json]
```

Behavior:

1. Verify authentication.
2. Find the root and complete diagram tree.
3. Fetch current Mermaid for every diagram.
4. Validate and normalize file names.
5. Refuse to overwrite an existing manifest unless explicitly requested.
6. Stage all `.mmd` files and the manifest, refuse existing initialization targets, then
   replace the set with backup-based best-effort rollback and report incomplete restoration.
7. Create a short README describing pull, status, push, and Git usage.
8. Print created files and any diagrams with missing Mermaid.

Initialization must not create a Git commit.

### Show synchronization state

```text
vaxis diagrams sync status [--dir architecture] [--json]
```

Per-diagram states:

- `in_sync`
- `local_changed`
- `remote_changed`
- `conflict`
- `local_missing`
- `remote_missing`
- `untracked_local`
- `invalid_local`
- `remote_added`
- `mermaid_unavailable`

The command is read-only. Normal human usage returns zero after reporting states; `--check`
exits with status 2 unless every diagram is `in_sync`, making drift enforceable in CI.

### Pull Vaxis changes

```text
vaxis diagrams sync pull [--dir architecture] [--dry-run] [--json]
```

Behavior:

- Update files only when the remote changed and the local file did not.
- Apply the validated portable remote tree as one file, including drill additions and removals.
- Refuse conflicts.
- Write every individual file through a temporary file followed by a rename. The complete
  multi-file operation is not transactional; retain backups and report paths if rollback is
  incomplete.
- Update hashes and revisions only after all selected writes succeed.
- Provide a deterministic change summary suitable for `git diff`.

### Push repository changes

```text
vaxis diagrams sync push [--dir architecture] [--dry-run] [--json]
```

Behavior:

- Lint every changed Mermaid file before making a remote write.
- Verify that drill markers agree with manifest relationships.
- Push only locally changed diagrams whose remote revision is unchanged.
- Refuse conflicts and invalid or unavailable remote content.
- Update manifest hashes and revisions after confirmed server success.
- Never use `--force` implicitly.

The CLI should show a complete plan before the first remote mutation. To prevent partial tree updates, the preferred backend implementation is a transactional batch sync endpoint. If the first version uses existing per-diagram endpoints, it must report partial success precisely and must not claim the whole sync succeeded.

### Optional follow-up commands

```text
vaxis diagrams sync render --ref <commit|tag|branch>
vaxis diagrams sync diff
vaxis diagrams sync unlink
vaxis diagrams sync resolve <alias> --use-local
vaxis diagrams sync resolve <alias> --use-remote
```

These should follow after the basic state model is proven.

### Historical Git rendering

`sync render --ref` is the planned read-only historical visualization workflow. It should:

1. Read the manifest and `.mmd` files directly from the requested Git commit, tag, or branch
   without changing the user's checkout.
2. Validate every file and drill relationship.
3. Create a separate Vaxis snapshot tree named with the Git ref.
4. Allocate new Vaxis diagram IDs and keep the old-to-new mapping in memory.
5. Preserve the current working Vaxis tree and the historical manifest unchanged.
6. Return the new snapshot link to Claude, Codex, or the invoking user.

This command is not implemented in v0.5.16. Until it exists, agents must not invoke it or
claim a complete historical tree was recreated. `diagrams import --file` can render one old
`.mmd` file, while full drill-tree reconstruction requires this future workflow.

## Synchronization algorithm

For each diagram, compare three values:

- `base`: `synced_hash` recorded after the last successful synchronization
- `local`: hash of normalized local Mermaid
- `remote`: hash or revision-derived content fetched from Vaxis

| Local vs. base | Remote vs. base | State | Safe action |
|---|---|---|---|
| Same | Same | In sync | None |
| Changed | Same | Local changed | Push |
| Same | Changed | Remote changed | Pull |
| Changed | Changed, same content | Converged | Refresh metadata |
| Changed | Changed, different content | Conflict | User resolution required |

### Normalization

Hash normalization should be conservative:

- Normalize CRLF and CR to LF.
- Ensure exactly one final newline.
- Preserve all other whitespace and ordering.

Do not reformat Mermaid automatically during synchronization. Aggressive normalization could hide meaningful edits or create unexpected diffs.

### Revision semantics

The backend should expose a monotonically increasing `content_revision` for every diagram. It increases only when versioned diagram content or its relevant hierarchy mapping changes. Chat messages, sharing settings, and view activity must not change it.

Push requests should use optimistic concurrency:

```http
If-Match: "diagram-revision-42"
```

or an equivalent JSON field:

```json
{
  "expected_revision": 42,
  "mermaid": "flowchart TB..."
}
```

The server returns `409 Conflict` or `412 Precondition Failed` when the expected revision is stale.

## Diagram-tree behavior

### Adding a drill child locally

For the first release, require both:

1. A valid drill marker in the parent `.mmd` file.
2. A corresponding manifest entry and child `.mmd` file.

A later command can simplify this:

```text
vaxis diagrams sync add authentication --parent system --node auth
```

### Removing a drill child

Remote deletion is destructive and should not occur as an incidental effect of editing a marker. The first release should report the detached or missing relationship and require an explicit diagram deletion/detach operation.

### Renaming

- Renaming a local alias or file does not rename the remote diagram automatically.
- Changing the manifest `name` can be treated as an intentional remote rename during push.
- The stable remote diagram ID prevents a file rename from creating a duplicate diagram.

## Backend/API work

### Minimum viable backend changes

1. Add `content_revision` to diagram responses.
2. Return current Mermaid, parent mapping, and revision consistently.
3. Accept an expected revision on import/update.
4. Reject stale writes atomically.
5. Ensure tree responses expose stable `parent_node_id` mappings.

### Preferred batch API

```text
GET  /api/diagrams/:rootId/snapshot
POST /api/diagrams/:rootId/sync
```

Snapshot response:

```json
{
  "application_id": "app_123",
  "root_id": "diag_root",
  "diagrams": [
    {
      "id": "diag_root",
      "name": "System Architecture",
      "parent_diagram_id": null,
      "parent_node_id": null,
      "content_revision": 42,
      "current_mermaid": "flowchart TB..."
    }
  ]
}
```

The batch sync endpoint should validate every expected revision and every hierarchy change before committing any mutation. The whole request succeeds or fails.

## Safety and security

- Resolve and validate every output path before writing.
- Reject absolute manifest paths and `..` traversal.
- Reject symlinked files or parents that escape the managed directory.
- Never follow a manifest-provided server URL for authentication without explicit configuration.
- Never print or persist auth/share tokens in sync output.
- Use staged writes, backups, and best-effort rollback. Report any original that cannot be
  restored and retain its backup for manual recovery.
- Initialization never overwrites an existing target. Pull temporarily backs up every file
  it replaces; explicit force/resolve operations may introduce longer-lived backups later.
- Require explicit confirmation before applying a pull that may contain drill removals.
- Bound snapshot size, diagram count, Mermaid size, and recursion depth.

## JSON contract

Every sync command must support stable machine-readable output. Example status result:

```json
{
  "root_diagram_id": "diag_root",
  "manifest": "architecture/vaxis.yaml",
  "summary": {
    "in_sync": 2,
    "local_changed": 1,
    "remote_changed": 0,
    "conflicts": 0
  },
  "diagrams": [
    {
      "alias": "payments",
      "id": "diag_payments",
      "file": "architecture/diagrams/payments.mmd",
      "state": "local_changed"
    }
  ]
}
```

Error objects should use stable codes such as:

- `manifest_not_found`
- `manifest_invalid`
- `path_outside_sync_root`
- `local_mermaid_invalid`
- `remote_revision_conflict`
- `remote_diagram_missing`
- `partial_sync`

## Implementation plan

### Phase 0: Validate the contract — completed for v0.5.16 scope

- Confirm repository-as-source-of-truth positioning.
- Use the selected `diagrams sync ...` command family.
- Use the selected YAML manifest format.
- Add backend content revision semantics to the cross-repository API contract.
- Prototype snapshots against a diagram tree with populated drill children.
- Define the exact assistant onboarding prompt, consent rules, and suppression behavior.

Deliverable: approved CLI/API specification with example fixtures.

### Phase 1: Local model and status — implemented in v0.5.16

- Add manifest structs and schema validation.
- Add safe path resolution and deterministic file naming.
- Add conservative Mermaid hashing/normalization.
- Add a remote snapshot client.
- Implement `sync status` and `--json` output.
- Add unit tests for all state combinations.

Deliverable: read-only drift detection.

### Phase 2: Initialize and pull — implemented in v0.5.16

- Implement `sync init` for the complete tree.
- Implement staged file and manifest writes with backup-based best-effort rollback.
- Implement `sync pull` and `--dry-run`.
- Handle drill additions and removals as part of the complete portable tree.
- Add Windows replacement tests plus platform-gated symlink-containment tests.

Deliverable: Vaxis-to-Git versioning workflow.

### Phase 3: Revision-safe push

- Add backend expected-revision enforcement.
- Implement push planning and Mermaid preflight validation.
- Implement `sync push` for changed diagrams.
- Add transactional batch sync if feasible; otherwise add precise partial-failure recovery.
- Update the manifest only after confirmed writes.

Deliverable: bidirectional conflict-safe synchronization.

### Phase 4: CI and review workflow

- Add command-level coverage for the implemented `sync status --check` JSON and exit-code
  contract; helper tests alone do not cover process exit behavior.
- Document GitHub Actions usage.
- Provide an example workflow that validates Mermaid and detects drift.
- Document CODEOWNERS and PR review recommendations.
- Consider optional rendered SVG artifacts, but keep `.mmd` canonical.
- Update the bundled instructions for Claude Code, Codex, and other supported assistants so interactive sessions offer repository linking once at the appropriate point.

Deliverable: enforceable architecture review in CI.

### Phase 5: Advanced collaboration

- Add explicit conflict-resolution commands.
- Add local child creation helpers.
- Add snapshot history or release annotations in Vaxis.
- Evaluate GitHub App, PR previews, and webhooks based on adoption.

Deliverable: deeper Git hosting integration without changing the canonical file model.

## Testing strategy

### Unit tests

- Manifest parse, validation, and deterministic serialization
- Hash normalization across line endings
- Every synchronization state
- File-name collision handling
- Path traversal and symlink rejection
- Revision conflict response mapping
- Drill/manifest relationship validation

### Integration tests

- Initialize a root with multiple child diagrams.
- Pull a remote-only edit.
- Push a local-only edit.
- Detect simultaneous local and remote edits.
- Recover cleanly from one failed write.
- Add a remote child and export it.
- Preview remote tree changes before applying drill removals.
- Run commands non-interactively with `--json`.

### End-to-end tests

- Initialize, commit, edit in Vaxis, pull, and review `git diff`.
- Branch from a committed architecture, edit locally, and push safely.
- Reject a push after another user changes the remote diagram.
- Restore an older Git version and intentionally push it as a new remote revision.
- Verify GitHub renders committed `.mmd` files.

## Documentation and release updates

Implementation will require coordinated updates to:

- `src/cli.rs`
- `src/commands/diagrams.rs` or a new `src/commands/sync.rs`
- `skill-data/core/SKILL.md`
- `docs/vaxis-cli-commands.md`
- `docs/vaxis-cli-guide.md`
- `docs/vaxis-api-and-frontend-reference.md`
- The backend repository's API contract documentation
- `README.md`
- `CHANGELOG.md`

The Rust crate and npm package versions must remain in lockstep for release.

## Success measures

- Percentage of active Vaxis projects linked to a repository
- Successful sync operations versus conflicts and failures
- Number of architecture changes reviewed in pull requests
- Percentage of linked projects that remain in sync
- Time from a remote architecture edit to a committed repository update
- Incidents of overwritten work; the target is zero

## Open decisions

1. Should commands be `vaxis diagrams sync ...` or `vaxis repo ...`?
2. Is the repository always authoritative, or should init offer a Vaxis-authoritative mode?
3. Should new drill children be created declaratively from the manifest during push?
4. Can the backend provide transactional whole-tree sync for the push release?
5. Should remote deletion require a separate command rather than a push option?
6. Should the manifest include an optional custom server identifier for self-hosted Vaxis?
7. Should CI drift checks contact Vaxis, or should offline linting be the default?
9. Should a user's "not now" preference last only for the session, for the repository, or in Vaxis account settings?

## Recommendation

Version 0.5.16 proceeds with read-only status, Vaxis-to-Git export, and explicit pull. It uses
`.mmd` as the canonical diagram format and a versioned manifest for identity and hierarchy.
Add push only after server-enforced optimistic concurrency is available. Avoid automatic Git
operations and automatic deletion.

This positions Vaxis as a collaborative architecture workspace that fits into existing engineering governance instead of competing with it: Vaxis provides visualization and AI-assisted editing, while Git provides history, review, branching, release alignment, and rollback.

## Related approaches

- GitHub renders committed `.mmd` and `.mermaid` files: <https://docs.github.com/en/repositories/working-with-files/using-files/working-with-non-code-files>
- Mermaid promotes versioning diagram text in Git: <https://mermaid.ai/web/products/code/>
- Structurizr recommends local version-controlled DSL/JSON followed by server publication: <https://docs.structurizr.com/getting-started/local-server>
- PlantUML documents text-file, Git, and CI workflows: <https://plantuml.com/en/starting>
- D2 provides a comparable text-to-diagram workflow: <https://www.d2lang.com/tour/intro/>
