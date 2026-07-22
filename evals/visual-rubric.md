# Diagram parity visual rubric

Render native and direct-Mermaid outputs at the same viewport and theme before scoring.
Score each category from 0 to 2 for a maximum of 10 and record the screenshot paths.

- **Readability:** 2 = labels and paths are immediately readable; 1 = minor crowding;
  0 = substantial overlap, clipping, or illegibility.
- **Hierarchy:** 2 = visual tiers and parent/child emphasis are clear; 1 = understandable
  with effort; 0 = structure appears flat or misleading.
- **Grouping:** 2 = related nodes are consistently contained and colored; 1 = partial or
  inconsistent grouping; 0 = groups are absent, broken, or visually confusing.
- **Flow:** 2 = direction and edge routing support natural reading; 1 = a few avoidable
  crossings; 0 = tangled routing or an unsuitable orientation obscures the flow.
- **Canvas use:** 2 = balanced spacing and useful occupied area; 1 = mildly sparse or
  cramped; 0 = extreme aspect ratio, excessive whitespace, or content outside containers.

A visual result passes at **8/10** or higher with no zero in readability or hierarchy.
Screenshot generation and geometric measurements live in the `vaxis` web repository;
this file is the shared scoring contract consumed by the recorded evaluation report.
