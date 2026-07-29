# Vaxis CLI — Comprehensive Use Case Catalog

Categories: Authentication → Account → Projects → Diagram Creation → Viewing → Editing →
Adding/Deleting Components → AI Generation & Suggestions → Drill-down Diagrams →
Validation & Limits → Undo & Versioning → Collaboration → Errors & Recovery →
Export & Sharing → Advanced Workflows → **Code to Diagram & Commit-Based Updates** → Future/Missing Capabilities

---

## 1. Authentication

### UC-01 · First command while not logged in
**Scenario:** User installs the CLI and immediately tries to use it without logging in.
**Trigger:** `vaxis apps list`
**Expected behavior:** CLI detects no stored token, tells user to run `vaxis login`, exits with error. Does NOT attempt a network call.
**Edge cases:** Same guard must fire for every command that touches the API; `vaxis --help`, `vaxis login`, `vaxis config`, `vaxis skills`, and `vaxis diagrams format` must NOT require auth.

---

### UC-02 · Successful login via browser device-flow
**Scenario:** User is not logged in and initiates authentication.
**Trigger:** `vaxis login`
**Expected behavior:** CLI opens browser to the auth URL, polls for completion, saves token, confirms which account was authenticated.
**Edge cases:** Browser fails to open → URL is printed for manual copy. Poll timeout after ~5 min → clear timeout message. User clicks the link in a different browser profile than expected.

---

### UC-03 · Login when already logged in (account switch)
**Scenario:** User is already logged in as Account A but wants to switch to Account B.
**Trigger:** `vaxis login` (while already authenticated)
**Expected behavior:** Initiates fresh login flow; after completion, stored credentials are replaced with the new account. Does NOT warn that a previous session existed.
**Edge cases:** If the user abandons the new login halfway, the old credentials should remain intact.

---

### UC-04 · Verify which account is active before starting work
**Scenario:** User wants to confirm they're using the right account before designing.
**Trigger:** `vaxis me --json`
**Expected behavior:** Returns `{ "name": "...", "email": "..." }`. In human mode, shows formatted profile.
**Edge cases:** Not logged in → `{"error": "not_authenticated"}`. AI agents run this as the first step in every session.

---

### UC-05 · Session token expires mid-session
**Scenario:** User was authenticated; server-side token was revoked or expired while they were working.
**Trigger:** Any API command after expiry.
**Expected behavior:** CLI catches the 401 response, tells user the session expired and to log in again. Does not crash or hang.
**Edge cases:** `vaxis me` reads local config only and will NOT detect server-side expiry — only the next real API call catches it.

---

### UC-06 · Logout — clear credentials
**Scenario:** User wants to sign out (security concern, shared machine, account switch).
**Trigger:** `vaxis logout`
**Expected behavior:** Stored token and user info cleared from config. Does NOT clear `auth_url` or `generation_mode`. Subsequent commands hit the not-logged-in guard.
**Edge cases:** Running `logout` when already logged out → idempotent, no error.

---

### UC-07 · Login behind a corporate proxy / VPN
**Scenario:** User's environment routes traffic through a proxy that may block the browser callback.
**Trigger:** `vaxis login`
**Expected behavior:** CLI should still print the auth URL even if it can't open the browser. The device-flow poll should still work if the proxy allows outbound HTTPS to the Vaxis server.
**Edge cases (future):** Proxy-aware configuration. Today the CLI has no proxy settings.

---

## 2. Account & Configuration

### UC-08 · Check current server configuration
**Scenario:** User or AI agent needs to know the effective server URL and generation mode before any work.
**Trigger:** `vaxis config show --json`
**Expected behavior:** Returns `{ "auth_url": "...", "generation_mode": "mermaid"|"prompt"|null }`. Effective URL reflects env var override if set.
**Edge cases:** `generation_mode: null` = not yet set; agents should treat null as `mermaid`.

---

### UC-09 · Point CLI at a local dev or staging server
**Scenario:** Developer testing changes against `http://localhost:3000`.
**Trigger:** `vaxis config set-url http://localhost:3000`
**Expected behavior:** URL stored (trailing slash stripped). All subsequent requests use it. `config show` reflects the new URL.
**Edge cases:** `VAXIS_AUTH_URL` environment variable always overrides the stored value — env wins.

---

## 3. Project Management

### UC-10 · List all projects — no projects yet (first-time user)
**Scenario:** User runs the CLI for the first time; no projects exist.
**Trigger:** `vaxis apps list --json` → returns `[]`
**Expected behavior:** Empty array in JSON; human mode shows hint to create one.
**Edge cases:** AI agents see an empty list and guide into creation rather than asking "what project?"

---

### UC-11 · List all projects — multiple projects exist
**Scenario:** User has an existing portfolio of projects.
**Trigger:** `vaxis apps list --json`
**Expected behavior:** Array of objects with `id`, `name`, `description`, `created_at`.
**Edge cases:** Long project names should not truncate IDs. AI fuzzy-matches user's description to an existing project name before creating a new one.

---

### UC-12 · Create a new project (no matching project exists)
**Scenario:** User starts designing a genuinely new system.
**Trigger:** `vaxis apps create "Inventory Service" --description "..." --json`
**Expected behavior:** Returns new app object with ID. Save ID for diagram operations.
**Edge cases:** Description is optional. Duplicate names are allowed by the backend — AI must check before creating.

---

### UC-13 · Create a project but a matching project already exists
**Scenario:** User says "design a payment system" and a "Payment System" project already exists.
**Trigger:** AI calls `vaxis apps list --json` first, finds a match.
**Expected behavior:** AI presents the match and asks: "I found 'Payment System' — continue that or start fresh?" Does NOT silently create a duplicate.
**Edge cases:** Multiple partial matches (e.g., "Payment Gateway v1" and "Payment Service") → show all matches and ask the user to pick one.

---

### UC-14 · Multiple projects match the user's description
**Scenario:** User says "update the auth diagram" and three projects contain "auth" in their name.
**Trigger:** AI calls `vaxis apps list --json`, finds multiple fuzzy matches.
**Expected behavior:** AI presents all matching projects in plain language and asks the user to pick. Does NOT guess.
**Edge cases:** If none match, ask the user whether to create a new project or try a different name.

---

### UC-15 · Rename a project
**Scenario:** User wants to rename "Payment System" to "Payments Platform v2".
**Trigger:** `vaxis apps update <id> --name "Payments Platform v2" --json`
OR interactive: `vaxis apps update` → picker → inline edit fields
**Expected behavior:** Name updated. All diagrams inside are unchanged.
**Edge cases:** Interactive mode pre-fills current name in the input field for easy in-place editing. Cancel exits cleanly.

---

### UC-16 · Update a project's description
**Scenario:** User wants to add or correct the project description.
**Trigger:** `vaxis apps update <id> --description "Stripe-based checkout" --json`
**Expected behavior:** Only description changed; name unchanged.
**Edge cases:** In interactive mode, if user makes no changes → "No changes made." without API call.

---

### UC-17 · Search or filter projects by name (future capability)
**Scenario:** User has 30+ projects and wants to find ones related to "auth".
**Trigger:** `vaxis apps list --filter "auth"` (not yet implemented)
**Expected behavior:** Returns only projects whose name or description contains the filter string.
**Edge cases:** Case-insensitive match. "No results" message when filter matches nothing.

---

### UC-18 · Move a diagram to a different project (future capability)
**Scenario:** User created a diagram under the wrong project.
**Trigger:** Not currently supported; no `diagrams move` command.
**Expected behavior (desired):** `vaxis diagrams move <diagramId> --to-app <targetAppId>`
**Edge cases:** If the diagram has children, all children move too. Shared links may need to be rotated.

---

## 4. Diagram Creation

### UC-19 · Create the first diagram in a project
**Scenario:** Project exists but has no diagrams yet.
**Trigger:** `vaxis diagrams create <appId> "Architecture Overview" --json`
**Expected behavior:** New empty diagram created and its ID returned. `current_mermaid` is null — no content yet.
**Edge cases:** The diagram MUST be populated via `generate` or `import` before it has any content.

---

### UC-20 · Create multiple root diagrams in one project
**Scenario:** Project has both an architecture diagram and a separate data-model diagram.
**Trigger:** Two separate `vaxis diagrams create` calls.
**Expected behavior:** Both root diagrams coexist. `diagrams list` shows both. Each has its own drill tree.
**Edge cases:** `tree` returns the subtree rooted at the given diagram; use any diagram ID in the app.

---

### UC-21 · Create a diagram from a named template (future capability)
**Scenario:** User wants to start from a standard "microservices starter" diagram.
**Trigger:** Not currently supported; no template command.
**Expected behavior (desired):** `vaxis diagrams create <appId> "My Service" --template microservices` → pre-populated Mermaid from template.
**Edge cases:** Templates are server-defined and version-locked. CLI would need to list available templates.

---

### UC-22 · Import user-provided Mermaid directly (bypass AI)
**Scenario:** User has a Mermaid diagram from another tool (Mermaid Live, draw.io export) and wants to save it to Vaxis.
**Trigger:** `vaxis diagrams import <diagramId> --mermaid "graph TD\n..." --json`
**Expected behavior:** Mermaid saved directly; no AI call. No drill processing. Returns `{"ok": true, "diagram_id": "..."}`.
**Edge cases:** `%% vaxis:drill` markers inside the imported Mermaid are NOT processed — use `generate --mermaid` for drill support.

---

### UC-23 · Import Mermaid from a file (future capability)
**Scenario:** User has a `.mmd` file and wants to import it.
**Trigger:** Not currently supported; no `--file` flag on `import`.
**Expected behavior (desired):** `vaxis diagrams import <diagramId> --file ./architecture.mmd --json`
**Edge cases:** File encoding, line endings, BOM characters must be handled. Large files (>50 nodes) should warn about limits.

---

## 5. Viewing Diagrams

### UC-24 · Read a diagram's current content before editing
**Scenario:** AI agent must read `current_mermaid` before modifying a diagram (never overwrite blindly).
**Trigger:** `vaxis diagrams show <diagramId> --json`
**Expected behavior:** Returns full diagram object including `current_mermaid`, `child_nodes`, `ancestry`. `scene_json` and `chat_messages` stripped (noise).
**Edge cases:** Fresh diagram: `current_mermaid` is null or empty — agent must handle this (can't edit what doesn't exist).

---

### UC-25 · Navigate the full diagram tree for a project
**Scenario:** User or AI needs to see the complete hierarchy of a design.
**Trigger:** `vaxis diagrams tree <anyDiagramId> --json`
**Expected behavior:** Tree from root, with all children and their `node_id`s. `scene_json` stripped recursively.
**Edge cases:** Any diagram ID in the app works — CLI navigates to root automatically. Empty tree (root has no drills) is valid. Human mode shows `├──`/`└──` tree with `[nodeId]` tags.

---

### UC-26 · List all diagrams in a project to find the right one
**Scenario:** User doesn't know which diagram to work on; wants to see them all.
**Trigger:** `vaxis diagrams list <appId> --json`
**Expected behavior:** Array of diagram objects (without `scene_json`). Human mode shows count "(root only)".
**Edge cases:** Distinguish root diagrams from children in the list (`parent_diagram_id: null` vs set).

---

### UC-27 · Find a diagram by name when you don't know the ID
**Scenario:** User says "show me the auth diagram" — AI needs to find the ID.
**Trigger:** AI calls `vaxis diagrams tree <rootId> --json`, searches for child by `name` or `node_id`.
**Expected behavior:** AI matches user's description to a diagram name in the tree; uses the matched `id`.
**Edge cases:** Multiple children with similar names (e.g., "Auth Service" and "Auth Gateway") → AI presents the list and asks which.

---

### UC-28 · View diagram history / version log (future capability)
**Scenario:** User wants to see what changes were made and when.
**Trigger:** Not currently supported; no `diagrams history` command.
**Expected behavior (desired):** `vaxis diagrams history <diagramId> --json` → list of `{version_id, created_at, changed_by, summary}`.
**Edge cases:** Pagination for diagrams with many versions.

---

## 6. Generating Diagrams (AI)

### UC-29 · Generate a diagram from scratch — AI writes Mermaid (primary flow)
**Scenario:** New diagram, Claude generates the Mermaid and sends it. This is the main product flow.
**Trigger:** `vaxis diagrams generate <diagramId> --mermaid "<mermaid>" --json`
**Expected behavior:** Diagram stored; drill children auto-created for any `%% vaxis:drill` markers. Returns `{ mermaid, drills[] }`.
**Edge cases:** Drill markers must be at column 0 AFTER the complete diagram. Indented markers silently ignored. Node IDs in drill markers must exist in the main diagram.

---

### UC-30 · Generate from a thin/vague description — clarification needed before drawing
**Scenario:** User says "design my app" with no component detail.
**Trigger:** AI should ask clarifying questions BEFORE calling generate.
**Expected behavior:** AI asks 2–4 focused questions (main components, connections, datastores, external services), then generates from the answers.
**Edge cases:** Don't over-ask — if the description is already specific enough, draw it and offer to refine. One round of questions, then proceed.

---

### UC-31 · Generate a specific diagram type (sequence, ER, state machine)
**Scenario:** User says "draw the authentication flow as a sequence diagram."
**Trigger:** `vaxis diagrams generate <diagramId> --mermaid "sequenceDiagram\n    ..." --json`
**Expected behavior:** Diagram stored as that type. `show` returns the correct Mermaid.
**Edge cases:** Sequence, class, ER, and state diagrams cannot have drill children — drill markers in these are ignored. Preserve diagram type on subsequent edits unless user asks to convert.

---

### UC-32 · Generate using Vaxis server AI (prompt mode)
**Scenario:** User has `generation_mode = "prompt"` — server AI generates.
**Trigger:** `vaxis diagrams generate <diagramId> --prompt "Design a Stripe integration" --json`
**Expected behavior:** Server AI generates; CLI checks `unchanged` field before reporting success. If `unchanged: true`, surfaces `answer`/`notice` — does NOT claim "Generated."
**Edge cases:** Server AI subject to rate limits (429). Large prompts may be truncated. Server AI may route questions to Ask even with `--prompt` (when `intent: auto` and input reads as a question).

---

### UC-33 · Generate fails silently — server returns unchanged but user expected an edit
**Scenario:** User says "add a Redis cache" but server returns `unchanged: true` (misclassified as question).
**Trigger:** `vaxis diagrams generate <diagramId> --prompt "add Redis cache" --json`
**Expected behavior:** CLI detects `unchanged: true`, does NOT print "Generated". AI should report the notice/answer and offer to retry with `--intent edit`.
**Edge cases:** Use `--intent edit` to force an edit turn; use `--mermaid` path to bypass the ambiguity entirely.

---

### UC-34 · First generate on a diagram (default mode: AI writes Mermaid)
**Scenario:** User's first `diagrams generate` call on a new diagram.
**Trigger:** `vaxis diagrams generate <diagramId> --mermaid "<mermaid>" --json`
**Expected behavior:** The AI assistant (Claude/Codex/etc.) always writes the Mermaid and passes it via `--mermaid`. This is the default and only recommended mode for end users. The skill handles this automatically with no mode selection needed from the user.
**Edge cases:** If no Mermaid is supplied and no `--prompt` is given, the CLI returns an error. The generation mode is an internal configuration detail — end users never need to select or change it.

---

## 7. Editing Existing Diagrams

### UC-35 · Add a new component to an existing diagram
**Scenario:** User says "add a Redis cache between the API and the database."
**Trigger (AI workflow):**
1. `vaxis diagrams show <diagramId> --json` → read `current_mermaid`
2. AI adds `cache[(Redis Cache)]` and edges while preserving ALL existing nodes
3. `vaxis diagrams generate <diagramId> --mermaid "<full updated mermaid>" --json`

**Expected behavior:** Updated diagram has all prior nodes + new cache node.
**Edge cases:** Any dropped node = silent data loss. New composite component should get a `%% vaxis:drill` marker. Near the 50-node limit: flag before adding more.

---

### UC-36 · Remove a specific component from a diagram
**Scenario:** User says "remove the notification service — we don't need it."
**Trigger (AI workflow):**
1. `vaxis diagrams show <diagramId> --json`
2. AI removes target node AND all its edges; carries everything else unchanged
3. `vaxis diagrams generate <diagramId> --mermaid "<updated without notify>" --json`

**Expected behavior:** Removed node gone; all other nodes/edges identical.
**Edge cases:** If the removed node had a drill child, that child diagram is now orphaned. Also remove `%% vaxis:drill <nodeId>` from the Mermaid. Offer to delete the orphaned child.

---

### UC-37 · Modify a specific component (rename, re-wire)
**Scenario:** User says "rename 'API Gateway' to 'GraphQL Gateway' and add a REST adapter next to it."
**Trigger (AI workflow):**
1. Read `current_mermaid`
2. Change the label for the target node; add new node + edges; carry all others unchanged
3. Re-generate

**Expected behavior:** Only the specified node/edges differ.
**Edge cases:** Do NOT change the node's shape (cylinder stays cylinder). Do NOT change untouched node IDs. Do NOT change the flowchart direction.

---

### UC-38 · Modify only a specific subgraph / region of the diagram
**Scenario:** User says "update only the Backend subgraph — add a worker pool."
**Trigger (AI workflow):** Read diagram → apply changes only inside the `Backend` subgraph → carry the rest exactly.
**Expected behavior:** Only the target subgraph is different; Frontend and Data subgraphs are identical.
**Edge cases:** Subgraph boundaries must be preserved. New nodes in the subgraph should appear inside the right `subgraph ... end` block.

---

### UC-39 · Change diagram direction (TB to LR or vice versa)
**Scenario:** User says "flip this to left-right — it's a pipeline, not a hierarchy."
**Trigger (AI workflow):** Change `graph TD` to `graph LR` (or `flowchart TB` to `flowchart LR`), carry all nodes/edges unchanged.
**Expected behavior:** Direction changed; everything else identical.
**Edge cases:** Explicit user request for direction change always wins. AI must not change direction on its own initiative.

---

### UC-40 · Convert diagram type (flowchart to sequence or ER)
**Scenario:** User says "convert this to a sequence diagram."
**Trigger (AI):** Rewrite the Mermaid in the new diagram type's syntax; carry the same logical components.
**Expected behavior:** New type stored; `show` returns the updated type keyword.
**Edge cases:** Drill children only work on flowcharts — if converting away from flowchart, warn that children become inaccessible. Preserve existing diagram type on all other edits.

---

### UC-41 · Small edit on a large diagram (40+ nodes)
**Scenario:** User wants to add one node to a diagram near the limit.
**Trigger (AI):** Read full `current_mermaid`, make the minimal change, re-send the complete Mermaid.
**Expected behavior:** One new node added; all 40+ existing nodes intact.
**Edge cases:** If adding the node would exceed 50 nodes, warn the user. Suggest restructuring into a new drill child instead. There is no diff/patch endpoint — the full Mermaid must be resent.

---

### UC-42 · Whole-canvas redesign ("turn this into a hospital management system")
**Scenario:** User wants to completely replace the existing content with a different design.
**Trigger:** `vaxis diagrams generate <diagramId> --mermaid "<new design>" --json`
OR: `vaxis diagrams generate <diagramId> --prompt "redesign as hospital system" --intent replace --json`
**Expected behavior:** Prior content replaced entirely.
**Edge cases:** Old child diagrams become orphaned (not auto-deleted). AI should warn: "This will replace all existing content. Old diagrams for [auth, payment] will become orphaned. Continue?"

---

## 8. AI Suggestions & Review

### UC-43 · Ask AI to review the existing diagram for issues
**Scenario:** User says "review my design and tell me what's missing."
**Trigger (AI):**
1. `vaxis diagrams show <diagramId> --json` → read current state
2. `vaxis diagrams tree <rootId> --json` → see full hierarchy and empty children
3. AI evaluates: single points of failure, missing error paths, isolated nodes, empty children, overcrowded nodes
4. Responds in prose; does NOT modify the diagram

**Expected behavior:** Prose feedback with specific gap identification. Offers to fix each gap.
**Edge cases:** AI must never modify the diagram during a review unless user explicitly says "fix it."

---

### UC-44 · AI suggests improvements — user selects which to apply
**Scenario:** AI proposes: "I suggest (a) adding an error path, (b) adding a retry mechanism, (c) adding an audit log. Which would you like to apply?"
**Trigger:** User's choice (e.g., "apply a and c")
**Expected behavior (current):** AI re-generates diagram with only the selected suggestions incorporated.
**Expected behavior (future):** CLI could support presenting diffs of each suggestion independently.
**Edge cases:** User might say "apply all" → generate once with all changes. User might say "none" → no change.

---

### UC-45 · AI proposes a significant redesign — user wants preview before applying
**Scenario:** AI wants to reorganize 6 subgraphs. User says "show me what it would look like first."
**Trigger:** No `generate` call yet; AI presents the proposed Mermaid code for human review.
**Expected behavior (current):** AI shows the proposed Mermaid in the chat for user to review. User says "yes" → then AI calls `generate`.
**Expected behavior (future):** A `--dry-run` flag on `generate` that returns the proposed Mermaid without persisting it.
**Edge cases:** User sees the proposal and says "too many changes — just add the error path." AI must respect that and re-plan.

---

### UC-46 · Ask a question about the diagram without modifying it
**Scenario:** User says "what talks to the database?" or "what does the auth service do?"
**Trigger:** `vaxis diagrams ask <diagramId> --prompt "What talks to the database?" --json`
**Expected behavior:** Returns prose answer in `answer` field. Diagram unchanged. `unchanged: true` always.
**Edge cases:** Surface the `answer` field directly to user — not Mermaid, not raw JSON. If no answer returned, show a clear message.

---

### UC-47 · User disagrees with AI-generated changes — undo and retry with correction
**Scenario:** AI added 5 nodes when user only wanted 1.
**Trigger:**
1. User: "That's too much — undo it."
2. `vaxis diagrams undo <diagramId> --json`
3. User specifies exactly what they want
4. `vaxis diagrams generate <diagramId> --mermaid "<corrected>" --json`

**Expected behavior:** Undo removes last AI turn; retry applies corrected version.
**Edge cases:** Never call `generate` again without undoing first (compound bad states). If undo fails, offer `import` to overwrite directly.

---

### UC-48 · AI identifies a node as a single point of failure
**Scenario:** During a review, AI notices the entire system routes through one node with no redundancy.
**Trigger:** Part of UC-43 (diagram review).
**Expected behavior:** AI flags the issue in prose: "Your auth service has no fallback — if it goes down, every user is locked out. Want me to add a circuit breaker?"
**Edge cases:** This is a recommendation, not an automatic edit. AI must wait for user approval.

---

## 9. Drill-down Diagrams

### UC-49 · Create a drill-down child from an existing parent node
**Scenario:** Parent diagram has `api[API Gateway]` and user wants to "drill into the API."
**Trigger (AI workflow):**
1. Read parent `current_mermaid`
2. Add `%% vaxis:drill api` at column 0 after complete diagram
3. `vaxis diagrams generate <parentId> --mermaid "<updated parent>" --json`
4. Drill child is auto-created by CLI from the `drills[]` response

**Expected behavior:** Parent diagram updated; child diagram created (empty or seeded). `tree` shows the new child.
**Edge cases:** Node ID must exist in the parent diagram. Drill only works on flowchart type. Child is named from the node's label (e.g., "API Gateway"), not the node ID ("api").

---

### UC-50 · Fill an empty drill child diagram
**Scenario:** Parent has a drill child that was created but is empty (`current_mermaid` null).
**Trigger (AI workflow):**
1. `vaxis diagrams tree <rootId> --json` → find child ID
2. `vaxis diagrams show <childId> --json` → confirms empty
3. `vaxis diagrams generate <childId> --mermaid "<internals>" --json`

**Expected behavior:** Child now has content. Tree shows it as non-empty.
**Edge cases:** Child must NOT repeat parent node IDs. Child cannot have its own drill children (one level deep only).

---

### UC-51 · Navigate to a child diagram by its subsystem name
**Scenario:** User says "show me the payment service internals" — AI must find the correct diagram ID.
**Trigger (AI):**
1. `vaxis diagrams tree <rootId> --json`
2. Find child with matching `name` or `node_id`
3. `vaxis diagrams show <childId> --json`

**Expected behavior:** AI finds the right child and shows its content.
**Edge cases:** User refers to a subsystem by label ("the payments flow") not node ID ("pay"). AI must match by name, not node ID alone.

---

### UC-52 · Seed a child diagram's content in the same generate call
**Scenario:** AI creates a drill AND provides its initial content in one call.
**Trigger (in Mermaid):**
```
graph TD
    api[API Gateway]
%% vaxis:drill api
flowchart TB
    rate[Rate Limiter]
    router[Route Handler]
    rate --> router
```
**Expected behavior:** Parent and child populated in one round-trip. CLI processes the seeded content for the child.
**Edge cases:** Seeded child must have at least 3 nodes (server minimum). Bare marker (no content) is always valid and creates an empty child.

---

### UC-53 · Delete a child diagram but keep the parent reference
**Scenario:** User wants to "reset" the payment child diagram and start the internals over.
**Trigger:** `vaxis diagrams delete <childId> --force --json`
**Expected behavior:** Child deleted. Parent diagram still has `%% vaxis:drill payment` in its Mermaid. The node reference in the parent's `child_nodes` map becomes stale.
**Edge cases (future):** A `--reset` option on delete that clears the child but re-creates an empty one. Today, user must re-run `generate` on the parent to recreate the child.

---

### UC-54 · What's left to design? — find empty children
**Scenario:** User says "what should I design next?"
**Trigger (AI):**
1. `vaxis diagrams tree <rootId> --json` → find all children
2. `vaxis diagrams show <each_child_id> --json` → check `current_mermaid`
3. Summarize: "Payment and Auth are detailed. Order Service is empty."

**Expected behavior:** AI presents a clear summary of what's done vs. what's not, and offers to tackle the next empty one.
**Edge cases:** Some children may have been renamed or deleted externally. Check `show` on each child before reporting.

---

## 10. Validation & Limits

### UC-55 · Mermaid syntax error in generated output
**Scenario:** AI writes invalid Mermaid (unclosed subgraph, illegal character in node ID, etc.).
**Trigger:** `vaxis diagrams generate <diagramId> --mermaid "<invalid mermaid>" --json`
**Expected behavior:** CLI's Mermaid linter catches the error before sending. Returns structured error; diagram unchanged.
**Edge cases (future):** Preflight linter should catch: unclosed subgraphs, spaces in node IDs, forbidden shape syntax, drill marker between nodes (not at end), unknown node IDs in drill markers.

---

### UC-56 · Drill marker in wrong position (between nodes)
**Scenario:** AI places `%% vaxis:drill auth` between two diagram nodes instead of after the complete diagram.
**Expected behavior:** Preflight linter catches this and returns `{"error":"mermaid_lint_failed","issues":[...]}`. Diagram unchanged.
**Edge cases:** Today this silently mangles the diagram — linter is the defense.

---

### UC-57 · Node limit reached (50 nodes)
**Scenario:** Diagram is at 48 nodes; user asks to "add 5 more services."
**Expected behavior:** AI calculates the count before generating and warns: "Adding 5 nodes would exceed the 50-node limit. Consider splitting the diagram into child diagrams."
**Edge cases (future):** CLI could enforce this at the preflight step. Today it's AI-side discipline.

---

### UC-58 · Edge limit reached (60 edges)
**Scenario:** Diagram is densely connected; adding more edges exceeds the 60-edge limit.
**Expected behavior:** AI warns before generating: "This diagram already has 58 edges. Adding more may reach the limit. Restructure using intermediate hubs or subgraphs."
**Edge cases:** Like node limit — AI-side guard today; preflight enforcement is future work.

---

### UC-59 · Fan-out cap exceeded (>4 connections per node)
**Scenario:** AI wires a hub node to 7 other services.
**Expected behavior:** AI self-checks before generating: a hub with 5+ connections should route extras through a bus/gateway/queue. Flags any hub with >4 total connections (in + out) and restructures.
**Edge cases:** This is an authoring rule, not a hard server error. AI must apply it proactively.

---

### UC-60 · Attempted drill on a non-flowchart diagram type
**Scenario:** AI accidentally adds `%% vaxis:drill` to a sequence diagram.
**Expected behavior:** Preflight linter should warn or error. Server silently ignores it.
**Edge cases:** Drills only work on flowchart types. Sequence, ER, class, state diagrams cannot have children.

---

### UC-61 · Invalid diagram type submitted
**Scenario:** User submits Mermaid with an unsupported diagram type keyword.
**Expected behavior:** Linter warns. Server may store it as an image-fallback type.
**Edge cases:** Image-fallback types (gantt, pie, journey, timeline, etc.) are valid Mermaid but not editable in Vaxis — they render as static images. Warn user.

---

## 11. Undo & Versioning

### UC-62 · Undo the last AI generation turn
**Scenario:** Last `generate` output was wrong; user wants to roll it back.
**Trigger:** `vaxis diagrams undo <diagramId> --json`
**Expected behavior:** Last `assistant` message removed from chat history. Diagram reverts to previous state. Returns `{"ok": true, "diagram_id": "..."}`.
**Edge cases:** Nothing to undo (first generate): server may return 404 or no-op — handle gracefully. Must undo BEFORE retrying with a corrected prompt or Mermaid.

---

### UC-63 · Multiple undo — reverting several steps
**Scenario:** User wants to roll back 3 consecutive bad generates.
**Trigger:** Three consecutive `vaxis diagrams undo <diagramId> --json` calls.
**Expected behavior:** Each undo removes one turn. After 3 undos, diagram is 3 turns back.
**Edge cases (future):** A `--steps N` option on undo. Today: call undo once per turn to reverse.

---

### UC-64 · Undo after sharing — share link still valid but shows old version
**Scenario:** User shared a diagram, then made more changes, then undid the changes.
**Expected behavior:** Share link still valid. The linked diagram now shows the rolled-back content.
**Edge cases:** Share link is not affected by undo — same `token` returns whatever `current_mermaid` is at the time of viewing.

---

### UC-65 · View history of all changes to a diagram (future capability)
**Scenario:** User wants an audit trail — who changed what, when.
**Trigger:** Not currently supported; no `diagrams history` command.
**Expected behavior (desired):** `vaxis diagrams history <diagramId>` → list of versions with timestamps and change summaries.
**Edge cases:** Chat messages partially represent history but are not a formal version log. Sessions add another dimension.

---

### UC-66 · Restore a specific earlier version (future capability)
**Scenario:** User wants to revert to a specific earlier version, not just undo the last turn.
**Trigger:** Not currently supported.
**Expected behavior (desired):** `vaxis diagrams restore <diagramId> --version <versionId>`
**Edge cases:** After restore, the current version is overwritten. Shared links would reflect the restored content. Children from the restored version may no longer exist.

---

### UC-67 · Compare current version to a previous version (future capability)
**Scenario:** User wants to see what changed between two versions.
**Trigger:** Not currently supported.
**Expected behavior (desired):** `vaxis diagrams diff <diagramId> --from <versionId>` → structured diff (added nodes, removed nodes, changed edges).
**Edge cases:** Visual diff in the web app vs. CLI text diff for agents.

---

## 12. Collaboration

### UC-68 · Share a diagram with a read-only view link
**Scenario:** Design session complete; user wants a link to share with the team.
**Trigger:** `vaxis diagrams share <rootDiagramId> --json`
**Expected behavior:** Returns `url` (view-only) and `edit_url` (collaborative edit). Always share the ROOT diagram — one link covers root + all drill children.
**Edge cases:** Non-destructive: plain `share` returns existing link if already shared. Does NOT rotate on plain call.

---

### UC-69 · Share link already exists — confirm it's the same link
**Scenario:** AI calls `share` again in a subsequent session to give the user the link.
**Expected behavior:** Returns existing `url` + `rotated: false`. Does not mint a new link.
**Edge cases:** Never call `--rotate` just to fetch a link — it breaks the old one.

---

### UC-70 · Rotate a share link (invalidate compromised link)
**Scenario:** User shared a link and it leaked; they need a new one.
**Trigger:** `vaxis diagrams share <diagramId> --rotate --json`
**Expected behavior:** New link minted; old link immediately invalid. `rotated: true` in response.
**Edge cases:** Warn the user: "This breaks the old link. People you shared it with will need the new one."

---

### UC-71 · Revoke sharing entirely (make diagram private)
**Scenario:** User no longer wants the diagram to be publicly accessible.
**Trigger:** `vaxis diagrams share <diagramId> --revoke --json`
**Expected behavior:** `{ "ok": true, "shared": false }`. Anyone with the old link gets a 404.
**Edge cases:** Cannot be undone without re-sharing (which mints a new link).

---

### UC-72 · Multiple users editing the same diagram (collaboration conflict)
**Scenario:** User A and User B both try to generate/update the same diagram simultaneously.
**Expected behavior (desired):** Last-write-wins today (no conflict detection). Future: the server should detect concurrent edits and return a conflict error.
**Edge cases (future):** CLI could detect a stale diagram state (version mismatch) and warn: "This diagram was modified by someone else since you last read it. Reload first."

---

### UC-73 · User does not have permission to modify a project (future capability)
**Scenario:** Project belongs to another user; current user has view-only access.
**Expected behavior:** `generate`, `delete`, `rename`, etc. return 403 Forbidden.
**Edge cases (current):** No permission model in the current CLI — all authenticated users have full access. Future: `{"error": "forbidden"}` with a clear message.

---

### UC-74 · Collaborative real-time editing via the edit link
**Scenario:** User shares the `edit_url` with a colleague for a live co-editing session.
**Trigger:** Share command returns `edit_url` / `edit_token`.
**Expected behavior:** Collaborator opens `edit_url` in the browser. CLI currently provides the link but has no further role in the session.
**Edge cases (future):** CLI could detect if the edit link exists and surface it alongside the view link in `share --json`.

---

## 13. Errors & Recovery

### UC-75 · Network error — server unreachable
**Scenario:** Server is down or URL is misconfigured.
**Trigger:** Any API command.
**Expected behavior:** JSON: `{"error": "network_error"}`. Human: `✗ Could not reach server.`
**Recovery:** Suggest `vaxis config show` to verify URL. Check `VAXIS_AUTH_URL` env var.

---

### UC-76 · Server timeout during long AI generation
**Scenario:** Server AI takes too long to generate for a large/complex prompt.
**Trigger:** `vaxis diagrams generate <id> --prompt "..." --json` (server AI path)
**Expected behavior:** Request times out after a reasonable threshold. CLI reports `{"error": "timeout"}` or the server's timeout response.
**Edge cases:** Diagram may be in a partial state. Offer undo before retry.

---

### UC-77 · Rate limiting (429 — too many requests)
**Scenario:** User or AI is calling the generate endpoint too rapidly.
**Trigger:** Generate command when server AI quota is exceeded.
**Expected behavior:** CLI surfaces the server's rate-limit message. Human: `⚠ Rate limited: <message>`.
**Edge cases:** `--mermaid` path bypasses server AI and is not rate-limited for the AI generation step.

---

### UC-78 · Diagram or app not found (stale ID)
**Scenario:** User passes an ID that was deleted or never existed.
**Expected behavior:** `{"error": "not_found"}`. Recovery: `apps list` → `diagrams list` to rediscover correct IDs.
**Edge cases:** IDs from a previous session may have been deleted. Never hardcode IDs across sessions — always rediscover.

---

### UC-79 · Partial failure — generate succeeded but child creation failed
**Scenario:** `generate --mermaid` processed drills but one `POST /children` call failed (network hiccup).
**Expected behavior:** The response's `drills[]` contains the attempted children. CLI reports which children were created and which failed.
**Edge cases:** CLI makes individual `POST /children` calls per drill; a failure on one doesn't abort the others. Missing children should be reported clearly.

---

### UC-80 · CLI version out of date with backend API
**Scenario:** User has an old CLI and the backend has changed a route or field.
**Trigger:** Any command that hits a changed endpoint.
**Expected behavior (current):** Vague `Unexpected status` or JSON parse failures. `vaxis diagrams rules-check --json` detects schema drift.
**Edge cases (future):** A `/api/version` endpoint that CLI queries on startup and warns if it's incompatible.

---

### UC-81 · Output piped to jq or head — broken pipe
**Scenario:** `vaxis apps list --json | head -1` — downstream process exits before CLI finishes writing.
**Expected behavior:** CLI exits 0 cleanly; no panic output.
**Required functionality:** Panic hook catches "BrokenPipe"/"os error 32" and exits 0.

---

### UC-82 · User refers to a project or diagram with an unclear name
**Scenario:** User says "update the diagram" with no context.
**Expected behavior:** AI asks one focused clarifying question ("Which project?" or "Which diagram?"), then proceeds. Never asks two clarifying questions in a row.
**Edge cases:** AI checks conversation context first before fetching — if IDs were established earlier, reuse them.

---

### UC-83 · Requested project or diagram does not exist
**Scenario:** User says "continue my e-commerce project" but no such project exists.
**Trigger:** AI calls `vaxis apps list --json` → no match found.
**Expected behavior:** AI reports: "I don't see an e-commerce project. Would you like to create one, or did you mean one of these: [Payment System, Store Backend]?"
**Edge cases:** Never silently create a new project when the user seems to expect one to exist.

---

## 14. Export & Sharing

### UC-84 · Export diagram as Mermaid code
**Scenario:** User wants the raw Mermaid syntax to use in another tool.
**Trigger (AI):** `vaxis diagrams show <diagramId> --json` → extract `current_mermaid` field → show to user in a code block.
**Expected behavior:** AI shows the raw Mermaid only when user explicitly asks for it. Otherwise describes the diagram in prose.
**Edge cases:** `show` already returns `current_mermaid` — no separate export command needed for Mermaid.

---

### UC-85 · Export diagram as image (PNG/SVG) (future capability)
**Scenario:** User wants a static image for a slide deck or documentation.
**Trigger:** Not currently supported; no `export` command.
**Expected behavior (desired):** `vaxis diagrams export <diagramId> --format png --output ./arch.png`
**Edge cases:** Image rendering requires the web app's renderer (ELK layout + Excalidraw). May be a web app feature, not a CLI feature.

---

### UC-86 · Export all diagrams in a project (future capability)
**Scenario:** User wants to back up or migrate an entire project's diagrams.
**Trigger:** Not currently supported.
**Expected behavior (desired):** `vaxis apps export <appId> --format json --output ./project.json` → all diagrams with their Mermaid content.
**Edge cases:** Large projects with many children. Would need to walk the tree and fetch each child.

---

### UC-87 · Get the Mermaid format reference for authoring guidance
**Scenario:** AI agent needs to know shape rules, limits, supported types, before generating.
**Trigger:** `vaxis diagrams format --json`
**Expected behavior:** Full structured JSON spec. No network call; embedded in binary.
**Edge cases:** Offline environments: always succeeds. CLI version-locked — matches the AI's authoring contract.

---

## 15. Advanced Workflows

### UC-88 · Continue from previously unfinished work
**Scenario:** User returns to a design session from days ago.
**Trigger (AI workflow):**
1. `vaxis apps list --json` → fuzzy-match "payment" to an existing project
2. Confirm with user: "I found 'Payment System' with 3 diagrams — continue that?"
3. `vaxis diagrams tree <rootId> --json` → understand current structure
4. For each child, `show` to check populated vs empty
5. Present summary: "Root done. Auth done. Order is still empty."

**Edge cases:** AI reuses IDs from earlier in the conversation rather than re-fetching when context is clear.

---

### UC-89 · CLI used in CI pipeline (fully scripted / non-interactive)
**Scenario:** CI script runs `vaxis diagrams generate` as part of a documentation pipeline.
**Requirements:** All commands must accept `--json` and required flags explicitly (no interactive prompts in CI). Any missing flag → structured `invalid_arguments` error, not a prompt.
**Edge cases:** `--json` disables all `dialoguer` pickers and confirms. `--force` disables confirm prompts. `VAXIS_AUTH_URL` env var sets the server.

---

### UC-90 · AI session skill loaded at start (`vaxis skills get core`)
**Scenario:** AI assistant loads the version-locked behavioral contract at the start of each session.
**Trigger:** `vaxis skills get core`
**Expected behavior:** Prints the full `skill-data/core/SKILL.md` embedded in the binary. No network call.
**Edge cases:** Only the `core` skill is bundled. The embedded version always matches the installed CLI's commands and JSON schemas.

---

### UC-91 · Install the Vaxis discovery skill for Claude Code
**Scenario:** User sets up the CLI and wants Claude to use it automatically.
**Trigger:** `vaxis install --skills` (interactive)
**Expected behavior:** Prompts for agent + scope, copies `skills/vaxis/SKILL.md` to the agent's skill directory, confirms installation.
**Edge cases:** `--force` backs up a user-modified skill before overwriting. `--yes` accepts safe defaults. Checksum-managed: if skill is unchanged, reports "already up to date."

---

### UC-92 · Install skill for Codex (scripted for CI)
**Scenario:** Automated onboarding script installs the skill for Codex in the project.
**Trigger:** `vaxis install --skills --agent codex --project --yes --json`
**Expected behavior:** `[{"agent":"codex","path":".agents/skills/vaxis/SKILL.md","status":"installed",...}]`
**Edge cases:** Codex and Agents use the shared `.agents/skills/vaxis/SKILL.md` path at both project and global scopes.

---

### UC-93 · Cross-diagram consistency review (future capability)
**Scenario:** User has 5 diagrams in a project and wants to ensure consistent naming across all of them (e.g., the same database node called `postgres` in every diagram that references it).
**Trigger:** Not currently supported.
**Expected behavior (desired):** `vaxis apps check <appId>` → finds naming inconsistencies across diagram trees.
**Edge cases:** Drill children reuse the same physical component as the parent — consistency is important.

---

### UC-94 · Batch operations across multiple diagrams (future capability)
**Scenario:** User wants to rename a node from `api` to `gateway` in all diagrams in the project.
**Trigger:** Not currently supported.
**Expected behavior (desired):** `vaxis apps refactor <appId> --rename-node api=gateway`
**Edge cases:** Node IDs may be different across diagrams. Drill references would need to be updated. High risk of corruption if not done carefully.

---

### UC-95 · AI session summary at the end — "what did we create?"
**Scenario:** At the end of a design session, user asks "what did we build today?"
**Trigger (AI):**
1. `vaxis diagrams tree <rootId> --json` → list full hierarchy
2. Count populated vs empty children
3. Summarize in plain English + show the share link

**Expected behavior:** "We designed the Payment System with 3 subsystem diagrams (Payment Service, Auth Service, Order Service). Auth Service is still empty. View your design: https://..."
**Edge cases:** AI must NOT dump raw Mermaid or JSON. Plain prose + link only.

---

### UC-96 · Rename a chat session for better organization
**Scenario:** User's default "Main" session accumulated many questions; they want to rename it.
**Trigger:** `vaxis diagrams sessions rename <diagramId> <sessionId> "Post-launch review" --json`
**Expected behavior:** `{"ok": true}`
**Edge cases:** 404 if session doesn't belong to that diagram.

---

### UC-97 · Use a specific chat session for a focused conversation thread
**Scenario:** User has a "Refactor pass" session and wants AI questions routed into it.
**Trigger:** `vaxis diagrams ask <diagramId> --prompt "..." --session <sessionId> --json`
OR: `vaxis diagrams generate <diagramId> --prompt "..." --session <sessionId> --json`
**Expected behavior:** Question/edit routed into the specified session's context, not the default/active session.
**Edge cases:** Mixing sessions may confuse the AI's conversation context. Sessions are per-diagram only.

---

### UC-98 · Warn user before a change that would break existing sharing (future capability)
**Scenario:** User says "delete the auth service from the root diagram." But the root diagram is shared via a public link.
**Expected behavior (desired):** AI warns: "This diagram is currently shared publicly. Removing the auth service will immediately affect anyone viewing the link. Continue?"
**Edge cases (current):** No such warning exists. The CLI has no knowledge of whether sharing is active before the edit — would require a `GET /share` check before every `generate`.

---

### UC-99 · Rename a diagram
**Scenario:** User wants to rename a diagram from "Architecture Overview" to "System Architecture v2" — the diagram entity itself, not its chat session.
**Trigger:** `vaxis diagrams rename <diagramId> "System Architecture v2" --json`
**Expected behavior:** `{"ok": true, "diagram_id": "..."}`. The diagram's `name` field is updated; its `current_mermaid`, children, and share links are unaffected.
**Edge cases:** This is distinct from project rename (`apps update`), node relabeling in an edit, and session rename (`sessions rename`). In human mode, confirm the new name. In `--json` mode, `--id` and the name argument are both required.

---

### UC-100 · Delete a root diagram (not the whole project)
**Scenario:** User wants to remove a single root diagram from a project that has multiple root diagrams. The project itself should remain.
**Trigger:** `vaxis diagrams delete <rootDiagramId> --force --json`
**Expected behavior:** `{"ok": true}`. The diagram and all its drill children are deleted. The project and any other root diagrams in it are untouched.
**Edge cases:** Diagram delete removes only this diagram and its drill children — the project and other root diagrams are untouched. If the deleted diagram had a share link, the link becomes invalid. Without `--force`, an interactive confirm fires in human mode: "Delete diagram '<name>' and all its children? This cannot be undone." In `--json` mode, `--force` is required.

---

### UC-101 · List and create chat sessions for a diagram
**Scenario:** User wants to see all chat sessions on a diagram (to pick one to route future asks/generates into), or create a new clean-slate session for a focused conversation thread.
**Trigger (list):** `vaxis diagrams sessions list <diagramId> --json`
**Expected behavior (list):** Array of session objects — `id`, `name`, `created_at`, `is_active`. Active session is the one that new `ask`/`generate --prompt` calls land in by default.
**Trigger (create):** `vaxis diagrams sessions create <diagramId> "Refactor pass" --json`
**Expected behavior (create):** New session created; returned object includes `id` for use with `--session` flag in subsequent calls.
**Edge cases:** An empty session list means no conversation history exists yet — the first `ask` or `generate --prompt` call creates the default session automatically. Explicitly creating a session lets the user partition conversation history by topic or milestone.

---

### UC-102 · Check for schema drift between CLI and backend (`rules-check`)
**Scenario:** User suspects their installed CLI version is out of sync with the backend's diagram-authoring rules (shape mappings, node limits, etc.).
**Trigger:** `vaxis diagrams rules-check --json`
**Expected behavior:** Fetches the canonical rules contract from `GET /api/diagrams/rules` (requires auth) and compares it to the rules embedded in the CLI binary. Returns `{"ok": true}` if in sync, or a structured diff of diverging fields if drift is detected.
**Edge cases:** No network → `{"error": "network_error"}`. 401 → session expired. If the backend has added new shape rules that the CLI's embedded spec doesn't know about, the diff surfaces exactly which fields differ — useful during CLI upgrades or backend deployments. AI agents can call this at session start as a version health check.

---

## 16. Code to Diagram & Commit-Based Updates

These use cases cover the **Code to Diagram** capability: the AI agent reads the codebase
using its file system tools, synthesizes Mermaid, and uses the CLI to create or update
diagrams. The CLI itself never reads files — all code reading happens in the agent.

The key interaction patterns are:
- **On-demand:** user explicitly asks to analyze code or update a diagram after a change.
- **Standing instruction (trigger-based):** user sets a persistent rule ("whenever a new commit is available, update the VAXIS diagram") and the skill handles everything autonomously — no further prompts needed.

---

### UC-103 · Generate an initial architecture diagram from project code
**Scenario:** User says "analyze this project and create an architecture diagram." No Vaxis diagram exists yet.
**Trigger (AI behavior):**
1. Read structural signals first — manifests (`package.json`, `Cargo.toml`, `go.mod`, `pom.xml`), Docker/compose configs, entry points (`main.rs`, `index.ts`, `cmd/`, etc.) — not every file.
2. Identify major components: services, modules, datastores, external APIs, cross-service connections.
3. Synthesize a Mermaid flowchart following all diagram rules (shape mapping, fan-out cap, drills for composite services).
4. `vaxis apps create` + `vaxis diagrams create` if no project exists yet, then `vaxis diagrams generate --mermaid`.
5. Offer to drill into each major service; ask user to validate: "Does this match how the system actually works?"
**Expected behavior:** A top-level architecture diagram reflecting the real project structure. Drill children auto-created for composite services.
**Edge cases:** Very small project (1–3 files) → simple flat diagram, no drills. Frontend + backend → two subgraphs or two drill children. Multiple languages in one repo → include all; label nodes with language if relevant.

---

### UC-104 · Update an existing diagram after code changes (on-demand)
**Scenario:** User says "I added a new notification service — update the diagram."
**Trigger (AI behavior):**
1. `vaxis diagrams show <diagramId> --json` → read `current_mermaid`.
2. Read the new service directory or changed files.
3. Identify what's new, what it connects to, whether it needs a drill.
4. Add new node + edges; carry all existing nodes unchanged.
5. `vaxis diagrams generate <diagramId> --mermaid "<updated>" --json`.
**Expected behavior:** Diagram updated with the new service; all prior nodes intact.
**Edge cases:** New service already partially visible as a placeholder → update that node rather than duplicating. New service replaces an old one → ask user before removing the old node.

---

### UC-105 · Detect drift between the diagram and the current codebase
**Scenario:** User says "the diagram might be out of date — check what's changed in the code."
**Trigger (AI behavior):**
1. `vaxis diagrams show <diagramId> --json` → extract all node IDs and labels.
2. Re-analyze the codebase (manifest + entry points) to re-derive the component list.
3. Compare: find nodes in the diagram not in code (stale); find components in code not in diagram (missing).
4. Report the drift in plain language BEFORE making any changes.
**Expected behavior:** "The diagram has `authService` but I don't see that module in the code anymore. The code has `oauthProvider` which isn't in the diagram. Want me to update the diagram?"
**Edge cases:** Diagram was intentionally simplified → some components legitimately absent; don't flag everything. Rename detected (old name in diagram, new name in code) → propose as rename, not delete + add. No drift → "The diagram matches the current code."

---

### UC-106 · Commit-triggered autonomous diagram update (standing instruction)
**Scenario:** User gives a standing instruction: "Whenever a new commit is available, update the VAXIS diagram." From that point on, the skill handles updates autonomously — the user does not need to specify what changed.
**Trigger (standing instruction pattern):** Fires each time the skill detects a new commit in the current repo, or when the user says "a new commit just landed" / "I just pushed."
**Skill behavior (full autonomous logic):**

Step 1 — Identify what changed:
- `git log --oneline -5` → see recent commits
- `git diff HEAD~1 HEAD --name-only` → list changed files
- `git diff HEAD~1 HEAD` → read the actual diff

Step 2 — Classify the change:
- **Architectural (diagram update needed):** new file in `services/`, `packages/`, `apps/`, `cmd/`; deleted service entry point; new import from one service to another; new HTTP route or gRPC method; new database, cache, or queue added in config; new directory under `services/` or `apps/`.
- **Implementation-only (no update needed):** function body changes, bug fixes, test file changes (`*.test.ts`, `*_spec.rb`, `tests/`), documentation, comments, build config, formatting.

Step 3 — Identify which diagram to update:
- `vaxis apps list --json` → find the project
- `vaxis diagrams tree <rootId> --json` → see all diagrams
- Match the changed file path to the diagram that represents that service (e.g., changes in `services/payment/` → find the "Payment Service" diagram)

Step 4 — Read before touching:
- `vaxis diagrams show <diagramId> --json` → extract `current_mermaid`; note all existing node IDs and edges

Step 5 — Plan edits:
- New component → add node with proper shape + edges + `%% vaxis:drill` if composite
- Removed component → remove node, all its edges, and any `%% vaxis:drill` marker for it
- Renamed component → update label; keep node ID
- New dependency → add edge (check fan-out cap: ≤4 connections per node)

Step 6 — Generate:
- `vaxis diagrams generate <diagramId> --mermaid "<full updated mermaid>" --json`
- Preserve EVERY existing node not touched by the commit

Step 7 — Report:
- "Commit [abc1234] added `NotificationService` to the architecture diagram. Connected to `APIGateway` and `MessageQueue`. No other diagrams needed updating."

**Expected behavior:** Diagram kept in sync with every architectural change in the repo, automatically, with no additional prompts after the initial standing instruction.
**Edge cases:**
- Implementation-only commit → no diagram update; optionally note: "Commit [abc] had no architectural changes — diagram unchanged."
- Ambiguous change → ask once: "I see changes to `lib/sharedUtils.ts`. Should I update the diagram?"
- Changes affect multiple diagrams → update each one and report all.
- No Vaxis diagram exists yet → "No diagram found for this project. Would you like me to create one from the current codebase?"

---

### UC-107 · CI/CD pipeline integration — diagram update per PR
**Scenario:** Every PR or merge to `main` automatically generates or updates the architecture diagram as part of the documentation pipeline.
**How it works today (current):** CI script invokes Claude Code (or another AI agent) as a step; the agent runs the full Code-to-Diagram workflow (UC-104 or UC-106) and uses `vaxis diagrams generate --mermaid --json` to push the result. The `VAXIS_AUTH_URL` env var points to the Vaxis server; `--json` disables all interactive prompts.
**Expected behavior:** Diagram updated automatically on every architectural PR. CI step exits 0 on success; non-zero on error.
**Edge cases:** Secrets must never appear in CI logs — the CLI never reads `.env`. The AI step runs outside CI (pre-commit hook, a separate CI job, or a post-merge GitHub Action). Token auth is via `VAXIS_AUTH_URL` + stored credential or a CI secret.

---

### UC-108 · Architectural diff between two git branches
**Scenario:** User says "show me what changed architecturally between `main` and `feature/new-auth`."
**Trigger (AI behavior):**
1. Read the codebase on `main`; synthesize architecture.
2. Read the codebase on `feature/new-auth` (checkout or `git show <branch>:<file>`); synthesize architecture.
3. Diff the two component lists structurally: added nodes, removed nodes, changed edges.
4. Present the structural diff in plain language. Optionally create two Vaxis diagrams for visual comparison.
**Expected behavior:** "Branch `feature/new-auth` adds `OAuthProvider`, removes `LegacyAuth`, and adds an edge from `api` to `SessionStore`."
**Edge cases (future):** Shared code changes that don't affect architecture → ignored in the diff. Only structural (node/edge) changes reported.

---

### UC-109 · "Keep the diagram up to date as I code" — watch mode (roadmap)
**Scenario:** User wants the diagram to automatically reflect code changes without any explicit trigger.
**Expected behavior (future):** `vaxis sync --watch` → monitors file changes; re-runs UC-104 logic on change; updates diagram.
**Edge cases:** Debounce: don't re-analyze on every keystroke — trigger after a pause in activity. Only re-generate if the change is meaningful (a new file in `services/` = trigger; a function edit inside an existing class = skip).
**Current workaround:** Use UC-106 (commit-triggered update) as a close substitute — each git commit triggers the skill to check for architectural changes.

---

### UC-110 · Add newly detected code components to the diagram
**Scenario:** User says "add anything new you find in the code to the diagram."
**Trigger (AI behavior):**
1. Read the existing diagram's `current_mermaid`.
2. Analyze the codebase for components not represented in the diagram.
3. For each new component: determine its type (service, database, external API), its connections to existing nodes.
4. Generate updated Mermaid adding new nodes + edges; preserve all existing content.
5. `vaxis diagrams generate --mermaid`.
**Expected behavior:** "Added `EmailService` (connects to `api` and `NotificationQueue`) and `emailQueue [(Redis)]`."
**Edge cases:** Many new components → ask which to add first; don't dump 15 new nodes at once. Unclear connections → add the node but ask: "I found `WebhookHandler` but I'm not sure what calls it — shall I connect it to the API Gateway?"

---

### UC-111 · Remove stale diagram nodes after code deletion
**Scenario:** User says "clean up the diagram — remove anything that's been deleted from the codebase."
**Trigger (AI behavior):**
1. Read the existing diagram.
2. For each node, check whether a corresponding module/service/component exists in the code.
3. Identify stale nodes; confirm with user before removing.
4. Generate updated Mermaid with stale nodes and their edges removed.
**Expected behavior:** "I found 3 components in the diagram that don't appear in the code: `legacyAuth`, `oldPaymentGateway`, `v1ApiRouter`. Remove all three?"
**Edge cases:** Component was renamed (not deleted) → detect via fuzzy match and propose rename instead of removal. Component exists in an unmerged branch → note uncertainty rather than removing. Stale node was an intentional placeholder → if user confirms, keep it.

---

### UC-112 · Scoped analysis — diagram a specific directory or service
**Scenario:** User says "diagram just the `apps/api` service" or "analyze only the auth module."
**Trigger (AI behavior):** Scope all file reading to the specified directory. Treat it as the root of the analysis.
**Expected behavior:** Diagram reflects only the code in that scope — not the entire repo. External dependencies (libraries imported from outside the scope) shown as labeled leaf nodes, not recursed into.
**Edge cases:** Directory doesn't exist → report clearly and ask for the correct path. Specified directory imports heavily from a shared library → include it as "External: Shared Library" node.

---

### UC-113 · End-to-end feature or code flow diagram
**Scenario:** User says "diagram the checkout feature from UI click to database write" or "trace what happens when `processPayment()` is called."
**Trigger (AI behavior):**
1. Find the feature entry point (route, function, event handler).
2. Trace the execution path: controller → service → external API → database.
3. Generate a flowchart (for layered view) or sequence diagram (for call-order view).
**Expected behavior:** Precise feature diagram; not the whole system. Node labels match real function/class names from the code.
**Edge cases:** Flow has async branches → show as parallel paths or `alt` blocks. Call chain is too deep (10+ levels) → cut at service boundaries; offer to drill into each service separately. Function name ambiguous → ask which one.

---

### UC-114 · Safety: skip secrets, build artifacts, and test files during code analysis
**Scenario:** Agent reads the project but must never expose credentials, read compiled output, or let test infrastructure pollute the production architecture diagram.
**Skill behavior (always-on rules):**
- **Never read:** `.env`, `*secret*`, `*credential*`, `*private*`, `*key*`, `.pem`, `.p12`, `*password*`, `config/secrets.*`
- **Never read:** `node_modules/`, `dist/`, `build/`, `target/release/`, `__pycache__/`, `.next/`, `vendor/`, `.git/`
- **Never read by default:** `*.test.ts`, `*_spec.rb`, `tests/`, `__tests__/`, `spec/`
- **Never include:** connection strings, API keys, or any credential value in node labels or edges
**Expected behavior:** Diagram generated from source code only. No credentials appear anywhere. Secrets note: "Database connection details read from environment — not included in diagram."
**Edge cases:** User explicitly asks to analyze `dist/` → explain why that's not advisable; offer source instead. Secret file is the only place a database connection is defined → diagram the database node using its role ("PostgreSQL"), not the connection string.

---

### UC-115 · Validate: confirm the generated diagram matches the actual system
**Scenario:** After the agent generates a diagram from code, some connections may be wrong (dynamic dispatch, runtime config, service discovery). User should validate before treating it as authoritative.
**Skill behavior:** After every code-to-diagram generation, add a validation prompt: "I've generated this from the code. Does it match how the system actually works? Let me know if I missed any connections or if any are wrong."
**Expected behavior:** User reviews and corrects: "add a connection from `api` to `AnalyticsService`" or "the `db` is actually PostgreSQL not MySQL." AI applies corrections via UC-104 (on-demand update).
**Edge cases:** Code analysis cannot capture runtime behavior perfectly (dynamic registration, feature flags, environment-specific routing). Always offer validation — it's standard practice, not a failure.

---

### UC-116 · Auto-detect project type and choose the right diagram shape
**Scenario:** Agent opens the project and must infer what kind of diagram is most appropriate without being told.
**Trigger (AI behavior):**
- REST API project → `flowchart TB` showing routes, middleware, handlers, database
- Event-driven / message queue system → `flowchart LR` showing producers, queues, consumers
- Library or SDK → class diagram or dependency diagram
- Frontend SPA → component hierarchy + data flow (state management, API calls)
- Database-only or migration project → ER diagram from schema files
- Microservices monorepo → top-level architecture with one drill per service
**Expected behavior:** Skill picks the most appropriate diagram type before generating. If ambiguous, asks user once: "This looks like a REST API — shall I draw the service architecture or the request flow?"
**Edge cases:** Project could fit multiple types → ask user to choose; present 2–3 options. Type detection confident but user wants a different type → honor the request.

---

### UC-117 · Analyze a monorepo and generate a service topology diagram
**Scenario:** User says "diagram this monorepo — show all services and how they relate."
**Trigger (AI behavior):**
1. Detect monorepo structure: look for `apps/`, `packages/`, `services/`, `libs/`, Nx/Turborepo/Lerna config.
2. Enumerate all apps/services.
3. For each service: identify what it exposes (HTTP port, message queue topic, gRPC service) and what it calls (other services, external APIs, databases).
4. Generate a top-level topology diagram with one node per service; drills into each service.
**Expected behavior:** Clean service topology with inter-service connections (labeled with protocol: HTTP, gRPC, Kafka, etc.). One drill child per service offering to detail its internals.
**Edge cases:** 20+ services → group by domain/team first; drill per domain, then per service within domain. Services communicate via a shared message bus → show the bus as a central cylinder node. Services only documented in README, not wired in code → include but label as "documented only — not verified in code."

---

### UC-118 · Handle a large repository that cannot be read in its entirety
**Scenario:** Repo has 500+ files across 40 directories.
**Trigger (AI behavior):**
1. Start from the top-level manifest and entry points only.
2. Identify the most important directories based on naming conventions (`src/`, `lib/`, `services/`, `api/`, `core/`).
3. Read only the top-level structure of each identified directory (not every file).
4. Generate a high-level diagram from manifest + entry points only; offer to drill into specific areas.
5. Report: "This is a large codebase. I've analyzed the top-level structure. Want me to dive deeper into any specific service?"
**Expected behavior:** High-level diagram from structural analysis only. No deep file reading until the user narrows scope.
**Edge cases:** Entry points are unclear → look for CI/CD configs (`Dockerfile`, `docker-compose.yml`, `k8s/*.yaml`) to infer what runs. Manifest missing → ask user "What's the main entry point for this project?"

---

### UC-119 · Update requested but code and diagram have diverged significantly
**Scenario:** The diagram is 6 months old and the codebase has been heavily refactored. 40% of nodes no longer exist.
**Trigger (AI behavior):**
1. Run a full drift analysis (UC-105).
2. Report the scale: "The diagram has 12 components that no longer exist in the code, and I found 8 new components. This is a significant drift — shall I regenerate the diagram from scratch, or merge the changes incrementally?"
3. Wait for user decision.
**Expected behavior:** User chooses: "regenerate from scratch" → treat as UC-103 (full analysis from code); "merge" → apply changes via UC-110 (add new) + UC-111 (remove stale).
**Edge cases:** User says "keep anything that still exists" → carefully preserve valid nodes, remove stale, add new in a single generate call.

---

## Summary — Coverage Grid

| Category | Use Cases | Notes |
|----------|-----------|-------|
| Authentication | UC-01 to UC-07 | Includes future: proxy support |
| Account & Config | UC-08 to UC-09 | |
| Project Management | UC-10 to UC-18 | Includes future: search, move |
| Diagram Creation | UC-19 to UC-23 | Includes future: duplicate, template, file import |
| Viewing Diagrams | UC-24 to UC-28 | Includes future: history |
| Generating Diagrams | UC-29 to UC-34 | |
| Editing Existing | UC-35 to UC-42 | |
| AI Suggestions & Review | UC-43 to UC-48 | |
| Drill-down Diagrams | UC-49 to UC-54 | |
| Validation & Limits | UC-55 to UC-61 | |
| Undo & Versioning | UC-62 to UC-67 | Includes future: history, restore, diff |
| Collaboration | UC-68 to UC-74 | Includes future: permissions, conflict |
| Errors & Recovery | UC-75 to UC-83 | |
| Export & Sharing | UC-84 to UC-87 | Includes future: image, bulk export |
| Advanced Workflows | UC-88 to UC-102 | Includes future: consistency check, batch, diagram rename, root delete, session list/create, rules-check |
| Code to Diagram & Commit-Based Updates | UC-103 to UC-119 | Includes future: watch mode (UC-109), branch diff (UC-108) |

**Total: 119 use cases.**
~85 exist today (current or partial), ~34 are future/missing capabilities.
