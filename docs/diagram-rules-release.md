# Diagram authoring rules release process

The `vaxis` repository owns server prompting, normalization, and rendering behavior. It is
the upstream authority for diagram authoring semantics. This CLI embeds a versioned offline
snapshot in `src/diagram_format.json` so agents can call `vaxis diagrams format --json`
without authentication or network access.

## Updating rules

1. Change and test the canonical rule behavior in `vaxis`.
2. Increment the canonical contract version. Additive fields may keep the same major
   version; removed or redefined fields require a major version increase.
3. Update `src/diagram_format.json` in this repository from the canonical contract.
4. Update `skills/SKILL.md` and its `vaxis-authoring-rules` marker.
5. Run `cargo test` and `vaxis diagrams format --json`.
6. Capture native and direct outputs with model, rules version, viewport, and theme, then
   run `vaxis diagrams evaluate --captures <file>`.
7. Release the server before or together with a CLI whose embedded contract requires the
   new behavior.

## Offline and compatibility behavior

- `diagrams format` always uses the embedded snapshot and never fails because the server
  is unavailable.
- Evaluation reports preserve the rules version recorded with each capture.
- Consumers ignore unknown additive fields within a supported major version.
- A CLI must not claim parity against an unsupported server contract major version.
- Until automated cross-repository comparison is enabled, the version marker and required
  phrase tests prevent accidental drift inside this repository; release review must still
  compare the embedded snapshot with the canonical `vaxis` artifact.
