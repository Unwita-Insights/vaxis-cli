# Diagram parity evaluations

This directory contains the fixed input catalog for comparing Vaxis native `--prompt`
generation with external-assistant `--mermaid` generation.

`diagram-parity-cases.json` is deliberately model-independent. Each case defines a prompt
and deterministic expectations that can be applied to Mermaid captured from either path.
The catalog is validated by Rust tests in `src/parity_eval.rs`.

`semantic-rubric.md` defines the scored human/model review for concept coverage, domain
fidelity, richness, hierarchy, and edit preservation. Keep those scores separate from the
deterministic structural failures.

`visual-rubric.md` defines common screenshot scoring. Screenshot capture itself belongs to
the Vaxis web renderer; this repository stores the scoring contract and recorded metadata.

## Current milestone

Milestone 1 provides:

- Ten representative cases.
- Expected direction, minimum structure, required concepts, shape requirements, fan-out,
  and drill requirements.
- Deterministic Mermaid metrics and expectation failures.

It does not yet call the Vaxis API, invoke an external model, assess rendered screenshots,
or publish baseline scores. Those steps must use recorded model/rules versions and stable
credentials so results remain attributable.

## Running the checks

```bash
cargo test parity_eval
vaxis diagrams evaluate --captures evals/fixtures/structural-smoke.json --json
```

The command exits with status `2` when any recorded capture fails its deterministic
expectations. Use `--output report.json` to write a versioned report instead of printing it.
It runs offline and does not require authentication.

## Capture contract for the next milestone

Each captured result should record:

- Case ID and generation path (`prompt` or `mermaid`)
- Mermaid output
- Model identifier
- prompt/rules version
- Timestamp
- Viewport and theme when rendering is evaluated
- Deterministic metrics and expectation failures

Do not overwrite earlier captures when a prompt or model changes; create a new versioned
baseline so improvements and regressions remain comparable.
