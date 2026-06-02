# CLI Login — How It Works & Why Some Feel Different

## The problem this doc solves

When you run `gh auth login` (GitHub) or `claude login`, your browser opens a page on **their real website** and you log in normally. Everything feels clean and professional.

But when we first built `vaxis login`, the browser opened `http://localhost:54321` — your own computer's address. That feels wrong, like something a developer hacked together.

This doc explains **why that happens**, what the difference is, and which approach is right for vaxis.

---

## Real-world analogy first

Imagine you ordered something online and need to verify your identity.

**Localhost callback approach** (what we built first):
> The shop says: "Give us your home address. We'll send a verification letter there."
> You wait at home, letter arrives, you confirm it.
> The address (`localhost`) is your own computer — only works if you're at home.

**Device flow / polling approach** (GitHub, Claude, Supabase CLI):
> The shop says: "Here's a 6-digit code: **ABCD-1234**. Go to our website and enter it."
> You go to their website, log in, type the code.
> Meanwhile the shop keeps checking: "Did they enter the code yet? Not yet... not yet... YES!"
> Your app gets confirmed.

Same end result. Very different experience.

---

## The two approaches, side by side

### Approach 1 — Localhost Callback (what we built)

```
You run:  vaxis login

1. vaxis starts a mini web server on YOUR computer (localhost:54321)
2. Browser opens → http://localhost:3000/api/auth/signin/google
                     ?callbackUrl=http://localhost:54321/callback
3. You log in with Google
4. Google/Better Auth redirects you to → http://localhost:54321/callback?token=...
5. The mini server on your computer catches it
6. vaxis saves the token
```

**What the user sees in the browser:**
```
http://localhost:54321/callback?token=abc123...
Login successful! You can close this tab.
```

**The problem:**
- The browser shows `localhost` — looks unofficial, untrustworthy
- Chrome sometimes blocks `localhost` redirects with security warnings
- Doesn't work if user is on a remote machine (SSH, cloud)
- Port 54321 might already be in use on someone's computer

---

### Approach 2 — Device Flow / Polling (GitHub, Claude, Supabase style)

```
You run:  vaxis login

1. vaxis calls your server → gets a random code (e.g. "XKCD-9821")
2. Browser opens → https://vaxis.app/cli-auth?state=XKCD-9821
3. You log in with Google normally on YOUR domain
4. Your server marks "XKCD-9821 = done, token = abc123"
5. Meanwhile vaxis is asking your server every 2 seconds:
   "Is XKCD-9821 done yet?" → "No" → "No" → "YES → here's the token"
6. vaxis saves the token
```

**What the user sees in the browser:**
```
https://vaxis.app/cli-auth
← your real domain, your real login page
```

**What the user sees in the terminal:**
```
Opening browser for login...
Waiting for you to complete login...  ⠋
✓ Logged in as Mani (mani@example.com)
```

---

## Why GitHub and Claude use Approach 2

| | Localhost Callback | Device Flow (polling) |
|---|---|---|
| Browser shows | `localhost:54321` | `github.com`, `claude.ai` |
| Feels like | Developer hack | Professional product |
| Works on remote machines | No | Yes |
| Port conflicts possible | Yes | No |
| Chrome security warnings | Sometimes | Never |
| Trust level for users | Low | High |
| Extra server work needed | No | Yes |

GitHub, Claude, and Supabase CLI all chose Approach 2 because they're shipping products to non-technical users. Showing `localhost` would scare people.

---

## Why Better Auth "doesn't work like this"

This is the key misunderstanding to clear up:

**Better Auth is a library. GitHub/Claude is a product.**

Better Auth gives you tools to build authentication into your web app. It handles Google login, sessions, email login — all for web browsers visiting your website.

It does NOT have a built-in "CLI device flow" because that's not a web browser feature — it's a CLI feature. No auth library (Better Auth, Supabase Auth, Firebase Auth, NextAuth) gives you this for free.

```
Better Auth can do:
  ✓ Google login on your website
  ✓ Email + password login
  ✓ Magic links
  ✓ Session management
  ✗ CLI device flow  ← not built-in, you build it yourself

GitHub CLI device flow:
  → GitHub's own engineers custom-built this
  → It runs on GitHub's servers
  → It's not part of any auth library
```

---

## Why Supabase CLI "works like Claude"

Supabase CLI (`supabase login`) uses device flow — but this was **custom-built by Supabase's engineering team** specifically for their CLI tool. It's not something the Supabase Auth library (which you use in your app) gives you automatically.

**Proof:** If you use Supabase Auth in your own app and build a CLI for it, you'd still have to build the device flow endpoints yourself. The Supabase CLI's login is Supabase's internal custom implementation.

```
Supabase Auth (the library)     →  handles web app auth
Supabase CLI login flow         →  custom-built by Supabase team, not part of the library
```

Same situation with Better Auth:

```
Better Auth (the library)       →  handles web app auth
vaxis login device flow         →  we need to build this ourselves
```

---

## What this means for vaxis

There are 3 paths forward:

### Path 1 — Keep localhost (current, works but looks rough)
- `vaxis login` opens `localhost:54321` in browser
- Works fine for developers
- Not suitable for non-technical users

### Path 2 — Build device flow on top of Better Auth (proper)
You add two endpoints to your Better Auth server:

```
GET  /api/cli/start           → generates a state code, returns it
GET  /api/cli/poll?state=XXX  → CLI calls this every 2s, returns token when ready
```

And one page on your frontend:
```
/cli-auth?state=XXX           → user logs in here with Google via Better Auth
                                 then server marks state as complete
```

Result: `vaxis login` opens `https://your-real-domain.com/cli-auth` — exactly like GitHub/Claude.

### Path 3 — Use Google OAuth directly (no server at all)
Skip Better Auth entirely for the CLI. The CLI talks directly to Google using PKCE flow (industry standard for CLI/native apps). No server needed, no localhost hack.

```
vaxis login → browser opens → accounts.google.com → logs in → CLI gets token
```

This is documented in: `docs/rust-cli-google-auth.md`

---

## Summary for non-programmers

| Question | Answer |
|---|---|
| Why did our first version use localhost? | It's the quick/easy approach — browser redirects to your own computer |
| Why does GitHub not use localhost? | They built a "polling" system — browser stays on their website, app waits for confirmation |
| Does Better Auth support polling? | Not built-in — needs custom code on the server |
| Does Supabase Auth support polling? | Not built-in — Supabase built it themselves for their own CLI |
| What's the right approach for vaxis? | Path 2 (device flow) for production, or Path 3 (direct Google) for simplicity |

---

## The one-line version

> **Localhost callback** = browser comes to your computer.
> **Device flow** = your computer waits while you use the browser normally.
> Neither auth library gives you device flow for free — you build it, or skip the server entirely.
