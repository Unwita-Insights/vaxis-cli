# npm Account, Token & GitHub Setup Guide

A complete walkthrough for first-time setup: create an npm account, generate a publish token, and wire it into GitHub so the CI release workflow can publish automatically.

---

## Overview

```
You (one-time setup)                 CI (every release)
─────────────────────                ──────────────────
1. Create npm account          →
2. Generate npm token          →     uses NPM_TOKEN secret
3. Add token to GitHub secret  →     to run: npm publish
```

You do steps 1–3 once. After that, every `git tag v*` push triggers the workflow and publishes to npm automatically — no manual action needed.

---

## Step 1 — Create an npm Account

1. Go to **https://www.npmjs.com**
2. Click **Sign Up** (top right)
3. Fill in:
   - **Username** — this becomes your public npm identity (e.g. `unwita`)
   - **Email** — use your work email
   - **Password** — strong password
4. Click **Create an Account**
5. Check your email — click the **verification link** npm sends you

> Your account is now active. The username you chose (`unwita`) will appear as the publisher on every package you release.

---

## Step 2 — Enable Two-Factor Authentication (required for publishing)

npm requires 2FA on accounts that publish packages.

1. Log in at **https://www.npmjs.com**
2. Click your avatar (top right) → **Account Settings**
3. Scroll to **Two-Factor Authentication**
4. Click **Enable 2FA**
5. Choose **Authentication app** (recommended — works with Google Authenticator, 1Password, Authy)
6. Scan the QR code with your authenticator app
7. Enter the 6-digit code to confirm
8. Save the **recovery codes** — store them somewhere safe

> Without 2FA, npm will reject publish attempts from CI tokens.

---

## Step 3 — Generate an npm Access Token

This is the token your GitHub Actions workflow uses to publish packages.

### 3.1 Open Token Settings

1. Log in at **https://www.npmjs.com**
2. Click your avatar → **Access Tokens**
3. Click **Generate New Token** → choose **Granular Access Token** (recommended over Classic)

### 3.2 Configure the Token

Fill in the form:

| Field | Value |
|---|---|
| **Token name** | `github-actions-vaxis-cli` |
| **Expiration** | `No expiration` (or set 1 year and rotate annually) |
| **Allowed IP ranges** | Leave blank (GitHub Actions IPs change) |
| **Packages and scopes** | Select **Read and write** |
| **Only select packages** | Pick `vaxis-cli` from the list (or leave as "All packages" if the package doesn't exist yet) |
| **Organizations** | Leave as default |

> **Granular token** is scoped to publish only — it cannot delete packages, change settings, or access your account. Safer than a Classic token which has full account access.

### 3.3 Generate and Copy

1. Click **Generate Token**
2. **Copy the token immediately** — npm only shows it once
3. It looks like: `npm_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`

> If you close the page without copying, you must delete the token and generate a new one.

---

## Step 4 — Add the Token to GitHub

### 4.1 Open Repository Secrets

1. Go to your GitHub repository: `https://github.com/Unwita-Insights/vaxis-cli`
2. Click **Settings** (top nav, near the right)
3. In the left sidebar: **Secrets and variables** → **Actions**
4. Click **New repository secret**

### 4.2 Create the Secret

| Field | Value |
|---|---|
| **Name** | `NPM_TOKEN` |
| **Secret** | Paste the token you copied in Step 3.3 |

Click **Add secret**.

> The secret is now encrypted and stored by GitHub. No one — including you — can read it back. You can only replace or delete it.

### 4.3 Verify It's There

You should now see `NPM_TOKEN` listed under **Repository secrets**. The value shows as `••••••••` — that's correct.

---

## Step 5 — How the CI Workflow Uses It

Your workflow at `.github/workflows/npm-publish.yml` already uses the secret correctly:

```yaml
- name: Stamp version and publish
  run: |
    VERSION="${GITHUB_REF_NAME#v}"
    cd npm
    npm version "$VERSION" --no-git-tag-version --allow-same-version
    npm publish --access public
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}   # ← secret injected here
```

When a tag like `v0.1.6` is pushed:
1. GitHub injects `NPM_TOKEN` into the workflow environment as `NODE_AUTH_TOKEN`
2. The `setup-node` action writes a `.npmrc` that reads `NODE_AUTH_TOKEN` automatically
3. `npm publish` authenticates using that token and publishes the package

> You never need to log in to npm manually in CI. The token handles everything.

---

## Step 6 — Test the Full Flow

After completing steps 1–4, trigger a test release:

```bash
# From the vaxis-cli repo
git tag v0.1.6
git push origin v0.1.6
```

Then watch the workflow:

```bash
gh run list --repo Unwita-Insights/vaxis-cli --limit 3
```

A successful run looks like:

```
✓  Build darwin-arm64     completed   1m5s
✓  Build linux-x64        completed   1m35s
✓  Build windows-x64      completed   3m50s
✓  Publish to npm         completed   11s
```

Verify the package is live:

```bash
npm view vaxis-cli version
# → 0.1.6
```

---

## Token Rotation (Annual Maintenance)

If you set an expiry on your token, rotate it before it expires:

1. npm → **Access Tokens** → **Generate New Token** (same settings as Step 3)
2. Copy the new token
3. GitHub → **Settings** → **Secrets and variables** → **Actions**
4. Click **Update** next to `NPM_TOKEN`
5. Paste the new token → **Update secret**
6. Delete the old token from npm → **Access Tokens** → trash icon

> Rotating before expiry avoids a CI outage. Set a calendar reminder 2 weeks before expiry.

---

## Troubleshooting

### `npm error code E403 — Forbidden`
- The token doesn't have publish permission → regenerate with **Read and write** scope
- The package name is taken by another user → choose a different name

### `npm error code ENEEDAUTH`
- The `NPM_TOKEN` secret is missing from GitHub → go back to Step 4
- The secret name is wrong — must be exactly `NPM_TOKEN` (uppercase)

### `npm error code E404 — Not found` (on first publish)
- First publish of a new package requires `--access public` → already set in the workflow

### `npm error 402 Payment Required`
- Scoped packages (e.g. `@unwita/vaxis-cli`) require a paid npm org for public publish
- Either use `--access public` explicitly or switch to an unscoped name like `vaxis-cli`

### Token published but CI says `401 Unauthorized`
- Token may have expired → rotate it (see Token Rotation section above)
- Token was revoked → generate a new one and update the GitHub secret

### `npm warn allow-scripts` on user install
- This is an npm 9+ security warning about the `postinstall` script — it is not an error
- Users run `npm install -g vaxis-cli --foreground-scripts` to allow the script
- Long-term fix: migrate to `optionalDependencies` approach (see `vaxis-npm-packaging.md`)

---

## Quick Reference

| Task | Where |
|---|---|
| Create npm account | https://www.npmjs.com/signup |
| Manage tokens | https://www.npmjs.com → Avatar → Access Tokens |
| Add GitHub secret | github.com/Unwita-Insights/vaxis-cli → Settings → Secrets → Actions |
| Watch CI runs | `gh run list --repo Unwita-Insights/vaxis-cli` |
| Check published package | https://www.npmjs.com/package/vaxis-cli |
| Verify installed version | `npm view vaxis-cli version` |
