# Vaxis — Complete Use Cases

All use cases are written from the user's perspective. The user speaks naturally to Claude.
Claude uses Vaxis behind the scenes. The user never sees CLI commands.

---

## Domain 1: Software System Design

### UC-01 — Design a system from scratch
**Who:** Developer, Tech Lead, Architect  
**Says:** *"Design a school admission system with student registration, parent portal, payment, and admin dashboard"*  
**Gets:**
- Top-level architecture diagram showing all services and connections
- Separate child diagram for each major subsystem (auto-generated)
- Can drill deeper into any subsystem in follow-up turns

---

### UC-02 — Add a new component to existing system
**Who:** Developer mid-project  
**Says:** *"Add a notification service to my admission system"*  
**Gets:**
- Root diagram updated with new node connected to relevant services
- New child diagram created for notification service internals
- Existing diagrams untouched

---

### UC-03 — Drill into a specific subsystem
**Who:** Developer who needs more detail  
**Says:** *"Show me the internals of the payment service"*  
**Gets:**
- Child diagram for payment service expanded with full internal flow
- Gateway, transaction handling, webhook, receipts — all detailed
- Parent system architecture unchanged

---

### UC-04 — Update a specific child diagram
**Who:** Developer refining one component  
**Says:** *"Add a refund flow to the payment service"*  
**Gets:**
- Only the Payment Service child diagram updated
- Refund request → validation → Stripe API → notification flow added
- Root architecture and other children untouched

---

### UC-05 — Design any type of software diagram
**Who:** Developer, Backend architect, DevOps engineer, Data engineer  
**Says:**
- *"Design a microservices architecture for an e-commerce platform on AWS"*
- *"Design the database schema for a blog with users, posts, tags, and comments"*
- *"Show the CI/CD pipeline and infrastructure for our SaaS on Cloudflare"*
- *"Map the order lifecycle — pending, paid, shipped, delivered, returned"*
- *"Show the API call sequence from login to dashboard load"*

**Gets:**
- Diagram type matched to the request (architecture, ER, sequence, state machine, flow, infrastructure)
- Tech-stack labels honored throughout (AWS services, specific DBs, frameworks, cloud providers)
- Child diagrams auto-created for major components
- If the prompt is too vague: Claude asks 2–3 clarifying questions before generating anything

---

### UC-06 — Analyze existing app and create architecture diagrams
**Who:** Developer or team who built a working application but never documented its architecture  
**Says:** *"We have a running app but no diagrams — can you create the architecture for it?"* or *"Here's our tech stack and services — generate the architecture diagrams"*  
**What Claude does:**
1. Asks the user to describe what exists: services, databases, APIs, tech stack, key flows
2. Analyzes what was provided — maps services, identifies connections, spots data flows
3. Creates a Vaxis project for the application
4. Generates the root architecture diagram reflecting the actual running system
5. Creates child diagrams for each major service or subsystem
6. Flags any gaps: "I notice you mentioned a Redis cache but didn't describe how it connects — is it used for session storage or queue?"

**Gets:**
- A full architecture diagram set that matches the real application — not a greenfield design
- Gaps and unclear connections flagged and clarified before generating
- The team now has documentation they can maintain going forward

---

## Domain 2: Iteration & Refinement

### UC-07 — Undo and retry
**Who:** Anyone dissatisfied with last generation  
**Says:** *"That doesn't look right — undo and use Stripe instead of a generic gateway"*  
**Gets:**
- Last AI turn removed from diagram
- Fresh generation with corrected instruction
- Prior diagram state restored before new attempt

---

### UC-08 — Continue from where we left off
**Who:** User returning to a previous session  
**Says:** *"What have we designed so far for the admission system?"*  
**Gets:**
- Summary of all diagrams in the project
- What's in each diagram and what was discussed in prior AI turns
- Suggestion for what's missing or incomplete

---

### UC-09 — Refine iteratively in one session
**Who:** Architect building step by step  
**Says (turn 1):** *"Design the high-level architecture for a fintech app"*  
**Says (turn 2):** *"Now add fraud detection between the payment service and the bank"*  
**Says (turn 3):** *"Make the fraud detection a separate service with ML scoring"*  
**Gets:**
- Each turn builds on the previous — chat history carries context
- Diagram evolves incrementally without losing prior nodes
- Final result is the accumulated architecture from all turns

---

### UC-10 — Match existing project before creating
**Who:** User who may have started this before  
**Says:** *"Design a payment system"*  
**Claude finds:** "Payment Gateway System" already exists  
**Claude asks:** *"I found 'Payment Gateway System' with 3 diagrams — continue that or start fresh?"*  
**Gets:**
- No duplicate projects created
- If continuing: picks up exactly where they left off
- If fresh: new project created cleanly

---

### UC-11 — Show all my projects
**Who:** Returning user, team lead  
**Says:** *"What projects do I have?"* or *"Show me everything I've designed so far"*  
**Gets:**
- List of all projects with names and descriptions
- Diagram count per project
- Suggestion to pick one and continue, or start a new one

---

### UC-12 — Simplify or add more detail
**Who:** Anyone who got a result that's too complex or too shallow  
**Says:** *"This is too complex — simplify the top-level diagram"* or *"I need more detail in the auth flow"*  
**Gets:**
- For simplify: non-essential nodes collapsed or merged, diagram tightened
- For more detail: current diagram expanded with additional components and flows
- Other diagrams untouched

---

### UC-13 — Rename or update a project
**Who:** Developer who named a project hastily  
**Says:** *"Rename this project to 'Payment Gateway v2'"* or *"Update the description"*  
**Gets:**
- Project name or description updated
- All diagrams inside remain unchanged
- Confirmation shown with the new name

---

## Domain 3: Non-Software Domains

### UC-14 — Business process design
**Who:** Business analyst, operations manager  
**Says:** *"Map out the HR onboarding process for a new employee joining a 500-person company"*  
**Gets:**
- End-to-end onboarding flowchart (offer letter → IT setup → orientation → buddy assignment)
- Decision nodes for contractor vs full-time, remote vs in-office
- Each phase as a drillable child (IT setup internals, background check flow, etc.)

---

### UC-15 — Product planning
**Who:** Product manager  
**Says:** *"Create a product roadmap diagram for our mobile app — Q1 through Q4"*  
**Gets:**
- Top-level timeline diagram with quarterly milestones
- Each quarter drilled into: features, dependencies, owners
- Launch criteria as a separate child diagram

---

### UC-16 — User journey mapping
**Who:** UX designer, product manager  
**Says:** *"Map the user journey for a customer buying insurance online"*  
**Gets:**
- End-to-end journey: awareness → compare → quote → apply → pay → policy issued
- Emotion/friction notes as decision nodes
- Each major step drilled into detailed interactions

---

### UC-17 — System integration design
**Who:** Enterprise architect  
**Says:** *"Design the integration architecture between our CRM, ERP, and billing system"*  
**Gets:**
- Integration hub diagram with all three systems + middleware
- Data flow arrows with labels (sync vs async, REST vs event)
- Each integration point as a child diagram with transformation logic

---

## Domain 4: End-to-End Flows

These use cases cover how Claude handles full sessions, multi-turn conversations, and context reuse.

---

### UC-18 — Full project from a single prompt
**Who:** Developer, Architect  
**Says:** *"Design a school admission system with student registration, parent portal, payment integration, and admin dashboard"*  
**What Claude does:**
1. Checks apps list — no matching project found
2. Creates application "School Admission System"
3. Creates root diagram "System Architecture"
4. Generates root diagram with all 4 subsystems as nodes
5. For each drill block returned: creates child diagram automatically
6. Reports: "I created the School Admission System with 4 subsystem diagrams. Want me to expand any of them?"

**Gets:**
- One prompt → full project with root + all child diagrams created
- No follow-up needed unless user wants to go deeper
- Claude reports all created diagram names in plain English

---

### UC-19 — Resume a prior session
**Who:** User returning after hours or days  
**Says:** *"Let's continue the payment system we were designing"*  
**What Claude does:**
1. Runs apps list — finds "Payment Gateway System" (fuzzy match)
2. Confirms: "I found 'Payment Gateway System' with 3 diagrams — shall I continue that?"
3. On confirmation: loads the full diagram tree
4. Reads root diagram current state
5. Summarizes: "Here's where we left off — root has 3 services, Payment Service has a child diagram, Auth Service is empty. What would you like to do next?"

**Gets:**
- Claude picks up exactly where the session ended
- No need to re-explain the project
- Claude shows a clear summary of what exists and what's incomplete

---

### UC-20 — Context reuse mid-conversation
**Who:** Developer mid-session  
**Says:** *"Now add a refund flow to the payment service"*  
**What Claude does:**
1. Recognizes "payment service" maps to an existing child diagram from earlier in the conversation
2. Does NOT create a new diagram or a new project
3. Reads current Mermaid from the Payment Service diagram
4. Confirms: "I found the Payment Service diagram — I'll update it with the refund flow"
5. Generates the update on that child diagram only

**Gets:**
- Claude reuses the correct existing diagram without asking "which diagram?"
- Existing nodes in the Payment Service are preserved
- Only the refund flow is added — nothing else changes

---

### UC-21 — Drill diagram reuse when user refers to a subsystem
**Who:** Developer who drilled into a subsystem earlier  
**Says:** *"Make the auth flow more detailed — add JWT refresh and session expiry"*  
**What Claude does:**
1. Recognizes "auth flow" from prior context — maps to the Auth Service child diagram
2. Uses diagram tree to confirm the correct diagram ID
3. Reads existing content of the auth child diagram
4. Confirms: "I found the Auth Service diagram — I'll add JWT refresh and session expiry to it"
5. Generates the update on the auth child diagram only

**Gets:**
- Claude finds the right diagram in the hierarchy without the user specifying an ID
- Existing auth flow structure is preserved, new detail added on top
- Root diagram and other children are untouched

---

### UC-22 — Multi-turn build within one session
**Who:** Architect building step by step  
**Says (turn 1):** *"Design a high-level architecture for a fintech app"*  
**Says (turn 2):** *"Now add fraud detection between payment and the bank"*  
**Says (turn 3):** *"Make fraud detection its own service with ML scoring"*  
**What Claude does:**
- Turn 1: Creates app + root diagram, generates initial architecture
- Turn 2: Reads existing root, generates update — adds fraud detection node, all other nodes preserved
- Turn 3: Reads current root again, updates fraud detection to a separate service, creates a child diagram for its ML internals

**Gets:**
- Each turn builds on the previous — no nodes lost between turns
- Claude reads the current diagram before every generate call
- Final result reflects all 3 instructions accumulated cleanly

---

### UC-23 — Undo in the middle of a multi-turn session
**Who:** Developer not happy with the last generation  
**Says:** *"That fraud detection diagram doesn't look right — undo and redo it with a rule engine instead of ML"*  
**What Claude does:**
1. Recognizes undo intent
2. Runs undo on the fraud detection diagram — removes last AI turn
3. Confirms: "Undone. I'll regenerate the Fraud Detection diagram using a rule engine approach"
4. Generates with the corrected instruction

**Gets:**
- The bad generation is cleanly removed before the new one runs
- No accumulation of bad + good content
- Rest of the project (root diagram, other children) completely untouched

---

### UC-24 — Vague mid-session update that needs clarification
**Who:** Developer who gives an incomplete instruction  
**Says:** *"Update the diagram"* (no further context)  
**What Claude does:**
1. Recognizes the instruction is ambiguous
2. Checks available diagrams from earlier context or runs diagrams list
3. Asks once: "Which diagram would you like to update? I can see Root Architecture, Payment Service, Auth Service, and Notification Service"
4. User picks one, Claude reads it, then asks: "What changes would you like to make?"
5. Proceeds with generate after getting the actual instruction

**Gets:**
- Claude never guesses which diagram to update
- One clarifying question — not multiple
- After clarification, the flow proceeds without further interruption

---

### UC-25 — Convert natural language into a Vaxis diagram
**Who:** Developer, Architect, anyone describing a system in words  
**Says:** *"I have a payment system with an API gateway, an auth service, a payment processor, and a Postgres database — create a diagram for this"*  
**What Claude does:**
1. Takes the user's natural language description
2. Calls `vaxis diagrams generate` — the AI converts the description into Mermaid diagram code
3. The Mermaid is saved to the Vaxis diagram and becomes a visual canvas on the web app
4. Reports back in plain English: "I created the diagram — it shows your API Gateway at the top, connecting to Auth Service and Payment Processor, both backed by a shared Postgres database"

**Gets:**
- Natural language → Mermaid → stored Vaxis diagram (visual canvas on web app)
- No Mermaid code shown to the user — Claude describes what was created in plain English
- User can open the Vaxis web app to see and interact with the visual diagram

---

### UC-26 — Read and explain a diagram in plain English
**Who:** Developer, team member, or stakeholder who wants to understand an existing diagram  
**Says:** *"What does the payment service diagram look like?"* or *"Explain the current architecture to me"*  
**What Claude does:**
1. Calls `vaxis diagrams show --json` to fetch the diagram
2. Reads the `current_mermaid` field — gets the raw Mermaid code
3. Interprets the Mermaid and translates it into a plain English description
4. Does NOT dump raw Mermaid at the user

**Gets:**
- Clear human-readable explanation: "Your Payment Service has 4 components: a request entry point, a validation layer, a Stripe integration, and a notification trigger on success"
- User understands the diagram without needing to read any code
- If the user explicitly asks for the raw diagram, Claude can show the Mermaid — otherwise it always explains in plain English

---

### UC-27 — Import user-provided Mermaid into Vaxis
**Who:** Developer who already has a Mermaid diagram from another tool, docs, or a teammate  
**Says:** *"I have this Mermaid — can you add it to Vaxis?"* (pastes raw Mermaid code)  
**What Claude does:**
1. Receives the raw Mermaid code from the user
2. Checks apps list — finds or creates the right project
3. Creates a new diagram in the project
4. Saves the provided Mermaid directly as the diagram content — no AI generation needed
5. Confirms: "Done — I've added your diagram to the Payment System project. You can view it in the Vaxis web app"

**Gets:**
- User-provided Mermaid imported into Vaxis as a real diagram in one step
- No regeneration — the Mermaid is used exactly as provided
- Diagram is immediately viewable on the web canvas

---

### UC-28 — Get the Mermaid code of an existing diagram
**Who:** Developer who wants to copy, share, or reuse diagram code  
**Says:** *"Give me the Mermaid for the payment service diagram"* or *"I want to copy this diagram to use in our docs"*  
**What Claude does:**
1. Reads the diagram via `vaxis diagrams show --json`
2. Extracts the `current_mermaid` field
3. Returns the raw Mermaid code clearly formatted for the user to copy

**Gets:**
- The exact Mermaid code for the requested diagram
- Ready to paste into any tool that accepts Mermaid (GitHub, Notion, Confluence, etc.)
- No explanation added unless user asks — just the clean Mermaid block

---

### UC-29 — Delete a diagram or project
**Who:** Developer cleaning up test work or old prototypes  
**Says:** *"Delete that test diagram I created earlier"* or *"Remove the prototype project, we don't need it"*  
**What Claude does:**
1. Confirms which diagram or project the user is referring to (from context or by listing)
2. Asks once to confirm: "Are you sure you want to delete 'Auth Service Prototype'? This will also remove all its child diagrams"
3. On confirmation: runs delete command
4. Reports: "Done — deleted Auth Service Prototype and its 2 child diagrams"

**Gets:**
- The right diagram or project identified before deletion — no guessing
- One confirmation prompt before any destructive action
- Clear report of what was deleted and what was cascaded

---

### UC-30 — Architectural review / "Is this design correct?"
**Who:** Developer or architect who wants a second opinion on their design  
**Says:** *"Does this payment system architecture look correct?"* or *"What am I missing in my admission system?"*  
**What Claude does:**
1. Reads the diagram via `vaxis diagrams show --json` to get the current Mermaid
2. Reads the full diagram tree to understand the overall structure
3. Evaluates the design — checks for missing components, single points of failure, unclear flows, or incomplete connections
4. Responds with specific feedback: what looks solid, what's missing, what could be improved

**Gets:**
- Architectural feedback in plain English — not just a description of what's there
- Specific callouts: "Your payment flow has no error handling path" or "Auth service has no session expiry mechanism"
- Suggestions for what to add or fix next

---

### UC-31 — What should I design next?
**Who:** Developer mid-project who wants guidance on what's missing  
**Says:** *"What's left to design in this system?"* or *"What should I build next for the admission system?"*  
**What Claude does:**
1. Reads the full diagram tree — sees what diagrams exist and which are empty
2. Looks at which child diagrams have content vs. which were created but never generated
3. Suggests the logical next step: "You've designed the root architecture and Payment Service. The Auth Service and Admin Dashboard diagrams exist but are empty — want me to expand those next?"

**Gets:**
- A clear picture of what's done vs. what's incomplete
- A specific suggestion for what to work on next — not a vague answer
- User can say yes and Claude proceeds immediately, or pick a different one

---

### UC-32 — First-time user with no projects
**Who:** Brand new user opening Claude with Vaxis for the first time  
**Says:** *"I want to design a system"* or anything that triggers Vaxis — but they have no projects yet  
**What Claude does:**
1. Runs `vaxis apps list --json` — returns an empty list
2. Recognizes this is the user's first session
3. Welcomes them: "It looks like you're just getting started — you have no projects yet. What would you like to design? Tell me what you have in mind and I'll set everything up"
4. Takes the user's description and runs the full creation flow: app → diagram → generate

**Gets:**
- No confusing empty state — Claude guides them straight into creation
- One natural question ("what would you like to design?") — then Claude handles all the setup
- By the end of the first message, a real project and diagram exist

---

### UC-33 — Get diagram format spec before generating
**Who:** Claude (at the start of any design session) / Developer who wants to know what diagram types Vaxis supports  
**Says:** *"What types of diagrams can you create in Vaxis?"* or Claude recognises it needs the format spec before starting a complex generation  
**What Claude does:**
1. Calls `vaxis diagrams format --json` to fetch the full Mermaid format reference
2. Receives all supported diagram types (flowchart, ER, sequence, state machine, infrastructure, journey, etc.) with a working example for each
3. Receives the rules: node ID format, drill annotation syntax, node limits, unsupported syntax
4. Uses this spec to produce correct Mermaid on the first attempt — no guessing

**Gets:**
- Claude knows exactly what diagram types are available before generating anything
- Correct Mermaid syntax on the first try — no broken diagrams from forgotten rules
- If Claude makes a mistake mid-session, it can call this again to reset its understanding
- User gets a plain-English answer: "I can create architecture diagrams, ER/database schemas, sequence diagrams, state machines, flowcharts, and infrastructure maps — which would you like?"

---

### UC-34 — Add or remove a specific node without rewriting the whole diagram
**Who:** Developer iterating on a large existing diagram  
**Says:** *"Just add a Redis cache between the API and the database"* or *"Remove the Legacy API node — we don't use it anymore"*  
**What Claude does:**
1. Recognises this is a targeted change on an existing diagram — not a full redesign
2. Reads the current diagram via `vaxis diagrams show --json` to understand what exists
3. Uses `vaxis diagrams patch` with only the change — not a full Mermaid rewrite:
   - For add: `{ "add_nodes": [{ "id": "cache", "label": "Redis Cache" }], "add_edges": [...] }`
   - For remove: `{ "remove_nodes": ["legacy_api"] }`
4. The server applies the diff to the existing Mermaid — all other nodes untouched
5. Reports: "Done — added Redis Cache between API and Database. All other nodes unchanged"

**Gets:**
- Only the requested change is applied — nothing else in the diagram moves or gets renamed
- Safe on large diagrams (50+ nodes) where a full rewrite would risk corrupting existing content
- Claude confirms exactly what was added or removed — no surprises

---

## Key Patterns Across All Use Cases

### Pattern 1 — Always check before creating
Claude checks `vaxis apps list` before creating a new project. If a match exists, it asks.
Prevents: duplicate projects, lost prior work.

### Pattern 2 — Read before write
Claude reads `vaxis diagrams show` before any `generate` call.
Prevents: overwriting nodes the user already placed, losing existing structure.

### Pattern 3 — Target the right depth
Claude uses `vaxis diagrams tree` to find the correct diagram level.
Root diagram → whole system changes. Child diagram → specific component changes.

### Pattern 4 — Drills are automatic
When the AI returns drill blocks, Claude creates the child diagrams without asking.
User sees: "Created 3 subsystem diagrams" — not a list of commands run.

### Pattern 5 — Undo is safe
If the user says anything like "that's wrong", "undo", "go back", "try again" —
Claude runs `vaxis diagrams undo` before re-generating. Never generates on top of bad output.

### Pattern 6 — Context reuse over re-creation
When the user refers to a subsystem by name mid-conversation, Claude finds the existing diagram and reuses it. It never creates a new one when one already exists.

### Pattern 7 — Chat history carries context
The server stores the last 3 AI turns per diagram. Claude doesn't need to re-explain the full system on every turn — the AI on the server already has context from prior turns.

### Pattern 8 — One clarifying question, then proceed
If the user's instruction is ambiguous, Claude asks one focused question (which diagram? what change?), then proceeds without further interruption.

---

## Use Case Coverage Matrix

| # | Use Case | Domain |
|---|----------|--------|
| UC-01 | Design a system from scratch | Software Design |
| UC-02 | Add a new component | Software Design |
| UC-03 | Drill into a subsystem | Software Design |
| UC-04 | Update a specific child diagram | Software Design |
| UC-05 | Design any type of software diagram | Software Design |
| UC-06 | Analyze existing app and create architecture diagrams | Software Design |
| UC-07 | Undo and retry | Iteration |
| UC-08 | Continue from where we left off | Iteration |
| UC-09 | Refine iteratively in one session | Iteration |
| UC-10 | Match existing project before creating | Iteration |
| UC-11 | Show all my projects | Iteration |
| UC-12 | Simplify or add more detail | Iteration |
| UC-13 | Rename or update a project | Iteration |
| UC-14 | Business process design | Non-Software |
| UC-15 | Product planning | Non-Software |
| UC-16 | User journey mapping | Non-Software |
| UC-17 | System integration design | Non-Software |
| UC-18 | Full project from a single prompt | End-to-End Flow |
| UC-19 | Resume a prior session | End-to-End Flow |
| UC-20 | Context reuse mid-conversation | End-to-End Flow |
| UC-21 | Drill diagram reuse by subsystem name | End-to-End Flow |
| UC-22 | Multi-turn build within one session | End-to-End Flow |
| UC-23 | Undo in the middle of a session | End-to-End Flow |
| UC-24 | Vague update with clarification | End-to-End Flow |
| UC-25 | Convert natural language into a Vaxis diagram | End-to-End Flow |
| UC-26 | Read and explain a diagram in plain English | End-to-End Flow |
| UC-27 | Import user-provided Mermaid into Vaxis | End-to-End Flow |
| UC-28 | Get the Mermaid code of an existing diagram | End-to-End Flow |
| UC-29 | Delete a diagram or project | End-to-End Flow |
| UC-30 | Architectural review — is this design correct? | End-to-End Flow |
| UC-31 | What should I design next? | End-to-End Flow |
| UC-32 | First-time user with no projects | End-to-End Flow |
| UC-33 | Get diagram format spec before generating | End-to-End Flow |
| UC-34 | Add or remove a specific node without rewriting | End-to-End Flow |
