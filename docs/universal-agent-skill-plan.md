# Universal Vaxis Agent Skill and Mermaid Validation Plan

## Objective

Make Mermaid diagrams authored through the Vaxis CLI consistently follow the Vaxis-native
authoring contract across Claude Code, Codex, Kimi CLI, OpenCode, GLM running inside a
supported host, and other Agent Skills or MCP-compatible tools.

The solution must not depend on an AI voluntarily finding `skills/SKILL.md` in the
`vaxis-cli` repository. It must distribute a valid portable skill, install it into the
locations each host actually scans, and enforce machine-checkable diagram requirements at
the CLI/API boundary.

## Success criteria

- A normal CLI release contains the complete Vaxis skill package.
- One explicit command installs or updates the skill for all supported local agents.
- Claude Code and Codex discover the skill in a new session without opening this repository.
- Kimi CLI and OpenCode discover the same canonical skill through their supported paths.
- GLM follows the skill when it runs inside Claude Code, OpenCode, or another supported host.
- Fresh software architectures with composite subsystems contain populated drill diagrams,
  unless the caller explicitly opts into staged empty drills.
- Invalid drill markers and structurally invalid seeded drills are rejected before saving.
- Validation failures are JSON-structured and actionable for any model.
- Existing direct-Mermaid requests remain supported through an explicit compatibility path.
- Contract drift and skill installation state are testable in CI and by end users.

## Non-goals

- Guaranteeing behavior in chat products that cannot load local skills, execute the CLI, or
  connect to MCP.
- Making the CLI call Vaxis server AI on the `--mermaid` path.
- Replacing Mermaid or `scene_json` as Vaxis data formats.
- Inferring perfect architecture semantics using only regular expressions.

## Key design decisions

### Skill is guidance; validation is enforcement

The skill teaches an external model how to design a good Vaxis diagram. The CLI validates
the subset of that contract that can be checked deterministically. Neither mechanism is
sufficient alone.

### Install for the host, not the model

Claude, Codex, GLM, and Kimi can be model names, host products, or both. Skill discovery is
owned by the host. For example, GLM running through Claude Code uses Claude Code's skill
location.

### Use the open Agent Skills format

The canonical package will follow the Agent Skills directory and frontmatter specification.
The main `SKILL.md` will stay concise; detailed references and examples will be loaded only
when needed.

### Prefer structured drills for new integrations

The existing `%% vaxis:drill` syntax remains supported, but a structured main-plus-drills
input removes ambiguous parsing and shell quoting from agent workflows.

## Supported host matrix

| Host | Installed location | Discovery strategy |
|---|---|---|
| Codex | `~/.agents/skills/vaxis/` | Canonical cross-agent installation |
| Claude Code | `~/.claude/skills/vaxis/` | Symlink/junction to canonical skill; copy fallback |
| Kimi CLI | `~/.agents/skills/vaxis/` | Kimi scans the generic Agent Skills location |
| OpenCode | `~/.agents/skills/vaxis/` | OpenCode scans the generic Agent Skills location |
| GLM via Claude Code | `~/.claude/skills/vaxis/` | Host is Claude Code |
| GLM via OpenCode | `~/.agents/skills/vaxis/` | Host is OpenCode |
| Other Agent Skills clients | `~/.agents/skills/vaxis/` where supported | Open-format compatibility |
| MCP clients | Vaxis MCP server configuration | Tool discovery plus server validation |

## Phase 1: Create a valid portable skill package

### Work

- Move the canonical skill from `skills/SKILL.md` to `skills/vaxis/SKILL.md`.
- Add required YAML frontmatter:
  - `name: vaxis`
  - a trigger-focused `description`
  - `license`
  - a short compatibility statement
- Keep the main file below 500 lines and focused on:
  - when the skill must activate;
  - authentication and generation-mode checks;
  - create/update/show workflows;
  - populated-drill requirements;
  - validation and recovery steps.
- Split detailed material into:
  - `references/mermaid-authoring.md`
  - `references/drill-hierarchy.md`
  - `references/cli-reference.md`
  - `references/output-contracts.md`
- Update all repository links and tests that reference the old path.
- Preserve the authoring-contract version marker and required mirrored phrases.

### Acceptance criteria

- The package passes the official Agent Skills format validator.
- Claude Code, Codex, Kimi CLI, and OpenCode can parse its metadata.
- The description explicitly triggers for creating, updating, reviewing, or sharing Vaxis
  diagrams and system architectures.
- The main skill requires complete seeded children for fresh composite architectures.

## Phase 2: Package and install the skill

### Commands

```text
vaxis skill install --all
vaxis skill install --target codex
vaxis skill install --target claude
vaxis skill install --target kimi
vaxis skill install --target opencode
vaxis skill status --json
vaxis skill update --all
vaxis skill uninstall --target <target>
vaxis skill print
```

### Work

- Include `skills/vaxis/**` in `npm/package.json`'s published files.
- Embed or bundle the skill with native release artifacts so non-npm installations work.
- Install the canonical skill into `~/.agents/skills/vaxis/`.
- Create `~/.claude/skills/vaxis/` as a symlink/junction to the canonical installation.
- Fall back to a copy when links are unavailable; record the installed version and checksum.
- Make installation explicit. Do not mutate agent configuration during npm `postinstall`.
- Make repeated installation idempotent.
- Refuse to overwrite a locally modified skill without `--force`; report the conflict in
  machine-readable JSON.
- Ensure uninstall removes only Vaxis-managed files and never deletes an entire shared skills
  directory.

### `status --json` contract

```json
{
  "bundled_version": "1.1.0",
  "targets": [
    {
      "target": "codex",
      "path": "~/.agents/skills/vaxis/SKILL.md",
      "installed": true,
      "version": "1.1.0",
      "current": true,
      "modified": false
    }
  ]
}
```

### Acceptance criteria

- The packed npm tarball contains the skill and all referenced resources.
- `install --all` works on Windows, macOS, and Linux.
- `status` detects missing, stale, and locally modified installations.
- A new session in every supported host lists the Vaxis skill.

## Phase 3: Add deterministic Mermaid validation

### Commands

```text
vaxis diagrams validate --mermaid <content> --profile native-parity --json
vaxis diagrams generate <id> --mermaid <content> --json
vaxis diagrams generate <id> --mermaid <content> --allow-empty-drills --json
```

### Required errors

- malformed or indented drill markers;
- drill markers placed before the complete main diagram;
- markers referencing unknown main-diagram node IDs;
- drill blocks on non-flowchart families;
- nested drill markers;
- non-empty seeded drills below the minimum connected-node floor;
- repeated drill markers for the same main node;
- drill child content that contains no renderable diagram;
- directives leaking into the renderable main diagram;
- bare markers in a fresh complete architecture unless `--allow-empty-drills` is present.

### Warnings

- a substantial software architecture has several composite-looking services but no drills;
- a node exceeds the recommended connection cap;
- storage labels use non-storage shapes;
- genuine decision labels do not use decision shapes;
- required subgraph grouping or visible domain vocabulary is unusually weak.

Warnings must not pretend semantic heuristics are certainty. The JSON result must separate
errors from warnings and include stable codes, line numbers where available, and a specific
repair instruction.

### Validation response

```json
{
  "ok": false,
  "profile": "native-parity",
  "errors": [
    {
      "code": "EMPTY_DRILL",
      "node_id": "payments",
      "line": 24,
      "message": "Add a child flowchart with at least 3 connected nodes after this marker."
    }
  ],
  "warnings": []
}
```

### Acceptance criteria

- `generate --mermaid` cannot silently save a structurally invalid hierarchy.
- Existing staged workflows remain available through an explicit flag.
- Tests cover valid populated drills, every required error, intentional empty drills, and
  non-software/non-flowchart diagrams.

## Phase 4: Add structured diagram input

### Proposed command

```text
vaxis diagrams generate <id> --spec diagram.json --json
```

### Proposed schema

```json
{
  "schema_version": "1.0",
  "main_mermaid": "flowchart TB\n...",
  "drills": [
    {
      "node_id": "payments",
      "mermaid": "flowchart TB\n..."
    }
  ],
  "direction_context": {
    "policy": "preserve"
  }
}
```

### Work

- Validate the spec locally before any network request.
- Convert it to the existing backend request during the compatibility phase, or add a
  first-class structured backend contract in lockstep with the Vaxis repository.
- Avoid interpolating large Mermaid payloads into shell arguments; support `--spec-file` and
  stdin.
- Preserve `--mermaid` for compatibility and small manual inputs.

### Acceptance criteria

- Agents no longer need to concatenate main and child diagrams with comment delimiters.
- Windows, macOS, and Linux handle labels and multiline Mermaid without quoting failures.
- The backend and CLI contract lists are updated together if the API changes.

## Phase 5: Add a Vaxis MCP adapter

### Tools

- `vaxis_get_authoring_contract`
- `vaxis_validate_mermaid`
- `vaxis_create_application`
- `vaxis_create_diagram`
- `vaxis_save_diagram`
- `vaxis_show_diagram`
- `vaxis_get_diagram_tree`
- `vaxis_share_diagram` as an explicit, separately authorized action

### Work

- Reuse CLI authentication and base-URL configuration.
- Accept `main_mermaid` and structured `drills[]` in the save tool.
- Put concise mandatory constraints in tool descriptions and return validation failures as
  structured tool errors.
- Do not create public share links implicitly.
- Document setup for Claude Code, Codex, Kimi, OpenCode, Cline, and other MCP clients.

### Acceptance criteria

- An MCP-capable agent can discover and use Vaxis without shell-command construction.
- Invalid Mermaid is rejected through the same validator as the CLI.
- MCP and CLI outputs agree for the same input fixture.

## Phase 6: Cross-agent evaluation

### Test matrix

Run the same prompts through:

- Claude Code with the installed skill;
- Codex with the installed skill;
- Kimi CLI with the installed skill;
- OpenCode using at least one GLM model;
- one host without the skill as a negative control;
- Vaxis server AI as the parity reference.

### Fixtures

- small flowchart that correctly needs no drills;
- layered software architecture with at least four composite services;
- payment or order system requiring populated drill children;
- non-software workflow to catch invented software vocabulary;
- sequence, ER, state, and class diagrams where drills are forbidden;
- malformed markers, unknown IDs, thin drills, nested drills, and long-form edge labels;
- Windows labels containing spaces and punctuation.

### Metrics

- main node and edge counts;
- valid populated drill count;
- empty drill count;
- required visible concepts;
- storage/decision shape correctness;
- maximum node connection count;
- renderer success and absence of directive text;
- child content retrievable through `diagrams show`;
- skill activation and CLI validation outcomes.

### Acceptance threshold

- 100% structural validity across supported agents.
- Zero literal `%% vaxis:drill` text in rendered main diagrams.
- Zero empty drills in complete-generation cases.
- No regression in small diagrams that legitimately need no hierarchy.
- Document semantic differences that remain model-dependent rather than calling them bugs.

## Phase 7: Documentation and release

### Documentation

- Add a short installation section to the root README.
- Explain that models use the skill through their host.
- Document supported and unsupported hosts.
- Document `skill status`, `diagrams validate`, `--allow-empty-drills`, and `--spec`.
- Add migration instructions for users who manually copied the old `skills/SKILL.md`.
- Update the diagram parity plan and release checklist.

### Release gates

- all Rust tests pass;
- Agent Skills validation passes;
- npm pack-content test proves the skill is present;
- install/status/update/uninstall tests pass on all three operating systems;
- direct-Mermaid validation fixtures pass;
- cross-agent evaluation results are recorded;
- CLI/backend contract checks pass;
- release version is updated consistently in Cargo, npm, and CLI version output;
- no production release until the install and rollback procedures are documented.

## Compatibility and rollout

1. Release the valid packaged skill and installer first.
2. Initially report native-parity issues as warnings while collecting real fixtures.
3. Promote unambiguous structural violations to errors.
4. Require explicit `--allow-empty-drills` for bare markers in a major or clearly announced
   compatibility release.
5. Introduce structured `--spec` input and recommend it to agents.
6. Add MCP after the CLI and validator share stable contracts.
7. Keep the previous skill bundle available for rollback, but never overwrite a locally
   modified installation automatically.

## Checklist

### Portable skill

- [ ] Move to `skills/vaxis/SKILL.md`.
- [ ] Add valid Agent Skills frontmatter.
- [ ] Split detailed references from the core workflow.
- [ ] Require populated drills for complete fresh architectures.
- [ ] Validate the package with the Agent Skills reference validator.

### Distribution

- [ ] Include the skill in npm and native release artifacts.
- [ ] Implement `skill install/status/update/uninstall/print`.
- [ ] Support `.agents/skills` and `.claude/skills` safely.
- [ ] Add checksum and local-modification protection.
- [ ] Test installation on Windows, macOS, and Linux.

### Enforcement

- [ ] Add `diagrams validate --profile native-parity`.
- [ ] Reuse validation automatically in `generate --mermaid`.
- [ ] Add stable JSON error and warning codes.
- [ ] Require explicit opt-in for empty staged drills.
- [ ] Add regression fixtures for all structural rules.

### Structured and MCP interfaces

- [ ] Add `--spec`/stdin structured input.
- [ ] Coordinate backend schema changes in both repositories if needed.
- [ ] Build the Vaxis MCP adapter on the shared validator.
- [ ] Keep sharing an explicit authorized action.

### Evaluation and release

- [ ] Run the cross-agent test matrix.
- [ ] Record parity results against Vaxis server AI.
- [ ] Update README, migration guide, and release checklist.
- [ ] Bump all version sources together.
- [ ] Publish only after packaging and installation smoke tests pass.
