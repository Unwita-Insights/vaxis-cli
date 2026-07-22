# Diagram parity semantic rubric

Apply this rubric to both native `--prompt` and external-assistant `--mermaid` outputs.
Score each category from 0 to 2 for a maximum of 10. Record a short justification for
every score so model comparisons remain auditable.

## 1. Required concept coverage

- **2:** Every `required_labels` concept is present with the requested role.
- **1:** Most concepts are present, but one is missing or materially misrepresented.
- **0:** Several required concepts are absent or the diagram addresses a different subject.

## 2. Domain fidelity

- **2:** Labels use accurate domain vocabulary with no invented software terminology in a
  non-software diagram and no vague placeholders in a software diagram.
- **1:** The domain is recognizable but contains one generic, misleading, or padded label.
- **0:** The output mixes domains or repeatedly invents inappropriate component names.

## 3. Richness fidelity

- **2:** Detail matches intent: bare requests remain small, named integrations expose
  relevant real capabilities, and broad requests form a coherent system.
- **1:** The output is useful but slightly over-expanded or under-specified.
- **0:** A trivial request is padded heavily, or a requested real integration is collapsed
  into a generic box.

## 4. Hierarchy and drill semantics

- **2:** Composite nodes drill where useful, atomic leaves do not, and seeded children
  contain only meaningful internals with at least three real nodes.
- **1:** Hierarchy is mostly correct with one missed composite or unnecessary drill.
- **0:** The root is an unreadable flat graph, atomic leaves drill, or children repeat the
  parent rather than exposing internals.

## 5. Edit preservation

- **2:** All untouched nodes, edges, IDs, shapes, type, and direction are preserved.
- **1:** Requested content is correct but one nonessential untouched property changes.
- **0:** Existing content disappears, is renamed, or is reshaped without authorization.

## Passing guidance

- A result should score at least **8/10** overall.
- Categories 1 and 5 are critical: a score of 0 in either is an automatic failure.
- Structural evaluator failures remain separate; a high semantic score cannot excuse
  invalid Mermaid, missing required shapes, excessive fan-out, or broken drills.
