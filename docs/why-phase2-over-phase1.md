# Why We Moved from Phase 1 to Phase 2

## The one-line reason

Phase 1 showed `localhost` in the user's browser. Phase 2 shows your real domain. That single difference changes how professional and trustworthy the login feels.

---

## What Phase 1 looked like

When a user ran `vaxis login` in Phase 1:

```
Terminal:
  Opening browser for Google login...
  Waiting for Google to redirect back...

Browser address bar:
  http://localhost:54321/callback?token=abc123...
  ──────────────────────────────────────────────
  "Login successful! You can close this tab."
```

The browser showed `localhost:54321` — your own computer's address. To a non-technical user this looks like a broken page, a security risk, or something unofficial.

---

## What Phase 2 looks like

```
Terminal:
  Opening browser for login...
  ⠙ Waiting for you to complete login in the browser...

Browser address bar:
  https://vaxis.dev/cli-auth?state=f3a9...
  ─────────────────────────────────────────
  [Vaxis logo]
  Connect your terminal
  [Continue with Google button]
```

The user sees your real domain, your real design, your real login flow. After they log in:

```
Browser:
  ✓ You're logged in
  Return to your terminal. You can close this tab.

Terminal:
  ✓ Logged in as Mani (mani@example.com)
```

---

## Side-by-side comparison

| | Phase 1 (localhost) | Phase 2 (polling) |
|---|---|---|
| Browser shows | `localhost:54321` | `vaxis.dev/cli-auth` |
| Feels like | Developer tool | Real product |
| Works on remote server/SSH | No | Yes |
| Port conflicts possible | Yes | No |
| Chrome security warnings | Sometimes | Never |
| User trust | Low | High |

---

## Why both frontend AND backend changes were needed

This is the key question. Phase 1 only needed CLI code (the `tiny_http` local server). Phase 2 requires both frontend and backend because of how the polling flow works.

### The handshake problem

In Phase 1, the handshake was between:
```
Google → your computer (localhost)
```
The CLI was the middleman — it ran a local server, caught Google's redirect, and extracted the token. No external service needed.

In Phase 2, the handshake is between:
```
Browser (on vaxis.dev) → your server → CLI (polling)
```
The CLI never receives a redirect. Instead, the browser tells the server "login is done", and the CLI asks the server "is login done yet?" The server is the middleman now.

This is why both frontend and backend had to be built:

### Why the backend needed changes

The backend is the **shared memory** between the browser and the CLI. Without it, the browser and CLI have no way to talk to each other.

Three new endpoints were added to `apps/api`:

```
POST /api/cli/start
  → CLI calls this first
  → Creates a unique "state" code (like a ticket number)
  → Returns the browser URL with that code embedded

GET /api/cli/poll?state=UUID
  → CLI calls this every 2 seconds
  → Asks: "Is login done for ticket UUID?"
  → Returns: pending / complete / expired

POST /api/cli/complete
  → Browser calls this after the user logs in
  → Says: "Ticket UUID is done — here's the session token"
```

And a new database table (`cli_auth_state`) to store the ticket state between calls.

**Without these backend changes**, the CLI has no way to know when login is done. The browser and CLI are on completely different processes, different machines potentially. The server is the only thing both can talk to.

### Why the frontend needed changes

The existing `LoginPage.tsx` (at `/login`) redirects the user to `/` after login. That's correct for web app users.

But for CLI users, after login we need to:
1. Call `/api/cli/complete` to mark the state as done
2. Show "You can close this tab" — not redirect to the app

If we used the existing `/login` page, after Google OAuth the user would end up on the dashboard (`/`), and the CLI state would never be marked complete. The CLI would keep polling forever and time out.

The new `CliAuthPage.tsx` (at `/cli-auth?state=UUID`) is purpose-built for this exact situation:
- It reads the `state` from the URL
- After login, it calls `/api/cli/complete` with that state
- It shows the "close this tab" message instead of redirecting

---

## What other providers were compared

### GitHub CLI (`gh auth login`)

**Flow:** Device Authorization Grant (RFC 8628)
- CLI displays a one-time code: `ABCD-1234`
- User visits `github.com/login/device` and types the code
- CLI polls GitHub's API until the code is confirmed
- No localhost, no redirect — the CLI and browser are completely decoupled

**Why we didn't do this:**
- Requires a dedicated "device code" page on GitHub's side
- Requires implementing RFC 8628 properly
- Adds UX friction: user has to manually type a code
- More complex to implement correctly

**What we borrowed:** The core idea — CLI polls a server, browser tells the server when done.

---

### Claude CLI (`claude login`)

**Flow:** Similar polling approach to what we built
- Browser opens Anthropic's hosted domain
- User logs in normally
- CLI receives the token via polling or redirect to a known endpoint
- No localhost shown to user

**Why we borrowed this:** This is the closest match to what we built. Clean UX, real domain shown, polling in background.

---

### Supabase CLI (`supabase login`)

**Flow:** Custom polling built on top of Supabase Auth
- Opens `supabase.com` in browser
- CLI polls Supabase's API for confirmation
- Same general pattern as Phase 2

**Key insight:** Supabase Auth (the library) does NOT provide this out of the box. The Supabase team built the polling endpoints themselves — on top of their own auth system. This is exactly what we did: built polling on top of Better Auth.

**Verdict:** Confirmed that any auth library requires this custom polling layer for CLI use. There is no library that gives you CLI device flow for free.

---

### Stripe CLI (`stripe login`)

**Flow:** Localhost callback (same as our Phase 1)
- Opens browser → Stripe's dashboard
- Stripe redirects to `localhost:PORT/callback`
- CLI catches the token

**What this tells us:** Even major production tools (Stripe) use localhost callback. It's not wrong — it's just a tradeoff between simplicity and polish. Stripe targets developers who understand localhost. Vaxis may target a broader audience.

---

### Fly.io CLI (`fly auth login`)

**Flow:** Localhost callback
- Same pattern as Stripe
- Opens browser → fly.io → redirects back to localhost

**Verdict:** Another developer-focused tool comfortable with localhost.

---

### Railway CLI (`railway login`)

**Flow:** Localhost callback
- Opens browser → railway.app → redirects back to localhost

**Verdict:** Same pattern.

---

## Summary: when each approach is right

| Approach | Used by | Right when |
|----------|---------|-----------|
| Localhost callback (Phase 1) | Stripe, Fly, Railway | Developer-only tools, speed matters more than polish |
| Browser polling (Phase 2) | Claude, Supabase CLI | Products used by non-developers, polish matters |
| Device code (RFC 8628) | GitHub | Large platforms, need to work on any device including TV/IoT |
| Direct API (email+password) | Any | CLI is used internally/scripted, no browser available |

---

## The decision

Vaxis is being built as a product, not just a developer tool. The audience may include designers, product managers, and non-technical users. Showing `localhost:54321` would create confusion and reduce trust.

Phase 2 adds ~3 hours of implementation work for a significantly better first impression. For a product that people will use daily, that tradeoff is worth it.
