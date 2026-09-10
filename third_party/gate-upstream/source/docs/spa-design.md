# Account Management SPA Design

> **Status:** Partial implementation. Reflects the M10 dashboard SPA and the
> email theme from `email.rs`. The visual language is derived from the
> dark-themed Battle.net-style email template and applied to the account
> management web application. `static/style.css` and `static/spa.js` are
> extracted and served (Steps 1–3); responsive sidebar collapse and
> per-page loading/error/empty states (Steps 4–5) remain.

## Design Goals

- **Coherent brand.** The SPA shares the same visual language as the welcome
  email: dark-first color scheme, accent-driven primary actions, the same
  system font stack, and a footer that mirrors the email bottom bar.
- **Server-rendered shell, client-rendered pages.** The HTML shell (sidebar,
  top bar, footer) is served by `askama`; all page content is rendered by
  vanilla JavaScript from `/api/*` JSON endpoints. No JS framework.
- **Session-first.** The SPA bootstraps via `POST /api/`, reads the
  `SESSIONID` cookie, and uses `XSRF-TOKEN` / `X-XSRF-TOKEN` double-submit
  for protected endpoints.
- **Progressive enhancement.** The SPA works without JavaScript for the
  login and password pages (server-rendered forms). The dashboard requires
  JS; if JS is unavailable, the server redirects to the login page.
- **Responsive.** The layout adapts from a two-column sidebar+content on
  desktop to a single-column stack with a collapsible nav on mobile.
- **Locale-aware.** The template renders locale-appropriate text from the
  account's stored locale (`accounts.locale`). The email locale set matches
  the SUI locale (same pool of 16 locale strings).

## Visual Design System

### Color Palette

Derived from `email.rs` CSS variables and the template `<style>` blocks.
One source of truth, extracted into a shared CSS file.

| Token | Light | Dark | Usage |
| --- | --- | --- | --- |
| `--bg` | `#dfe0e6` | `#15171e` | Page background, input backgrounds |
| `--card` | `#ffffff` | `#1a1c23` | Card/section backgrounds, sidebar |
| `--text` | `#1a1a2e` | `#d5d7dd` | Body text, headings |
| `--text-secondary` | `#4b5563` | `#b0b8c8` | Labels, hints, footnotes |
| `--footer-bg` | `#ffffff` | `#111318` | Footer background |
| `--footer-text` | `#4b5563` | `#b0b8c8` | Footer text, links |
| `--divider` | `#d1d5db` | `#2a3a52` | Borders, horizontal rules |
| `--link` | `#e04800` | `#ff8c42` | Inline text links |
| `--accent` | `#e04800` | `#ff5202` | Primary buttons, brand text, active state |
| `--badge-ok` | `#d1fae5` | `#064e3b` | Enabled/verified badge background |
| `--badge-ok-text` | `#065f46` | `#6ee7b7` | Enabled/verified badge text |
| `--badge-warn` | `#fef3c7` | `#78350f` | Disabled/unverified badge background |
| `--badge-warn-text` | `#92400e` | `#fcd34d` | Disabled/unverified badge text |
| `--error` | `#ff6b6b` | `#f87171` | Error text and icons |
| `--error-bg` | `#fef2f2` | `#1c1010` | Error callout background |

Dark mode is the default for the email; the SPA follows the browser's
`prefers-color-scheme` media query and defaults to dark in the email.

### Typography

```css
font-family: -apple-system, BlinkMacSystemFont, "Segoe UI",
  "Noto Sans", "Open Sans", Frutiger, "Frutiger Linotype", Univers,
  "Helvetica Neue", Helvetica, Arial, "Gill Sans", "Gill Sans MT",
  "Myriad Pro", Myriad, "DejaVu Sans Condensed", "Liberation Sans",
  "Nimbus Sans L", "Malgun Gothic", "Microsoft YaHei",
  AppleSDGothicNeo, AppleGothic, Dotum, "Microsoft JhengHei",
  "Hiragino Kaku Gothic Pro", "Hiragino Kaku Gothic ProN W3",
  Osaka, sans-serif;
```

The full stack matches the email's `EMAIL_STYLES` constant and covers
CJK locales (Korean, Japanese, Chinese, Thai) with appropriate font
fallbacks.

### Spacing Scale

| Token | Value | Usage |
| --- | --- | --- |
| `--space-xs` | 4px | Inner padding on badges, small icons |
| `--space-sm` | 8px | Gaps between inline elements |
| `--space-md` | 16px | Card padding, form element margins |
| `--space-lg` | 24px | Section spacing, sidebar item padding |
| `--space-xl` | 32px | Page section spacing, layout gaps |
| `--space-2xl` | 48px | Major layout sections |

### Border Radius

| Token | Value | Usage |
| --- | --- | --- |
| `--radius-sm` | 3px | Badges, small decorative elements |
| `--radius-md` | 4px | Inputs, buttons, cards |
| `--radius-lg` | 8px | Modal dialogs, card groups |

### Shadows

Used sparingly, matching the flat design of the source
`account.battle.net`.

| Token | Value | Usage |
| --- | --- | --- |
| `--shadow-sm` | `0 1px 2px rgba(0,0,0,0.1)` | Cards, sidebar items |
| `--shadow-md` | `0 2px 8px rgba(0,0,0,0.15)` | Dropdowns, modals |

## Layout

### Page Shell

```text
+----------------------------------------------------------+
|  Top Bar                                                  |
|  [brand]                           [battletag] [Log out] |
+----------------------------------------------------------+
|             |                                              |
|  Sidebar    |  Content Area                                |
|  [Overview] |  +----------------------------------------+ |
|  [Games]    |  | Section header                         | |
|  [Security] |  | Key-value item ................ Value  | |
|  [Details]  |  | Key-value item ................ Value  | |
|  [Privacy]  |  +----------------------------------------+ |
|             |  +----------------------------------------+ |
|             |  | Section header                         | |
|             |  | ...                                    | |
|             |  +----------------------------------------+ |
|             |                                              |
+----------------------------------------------------------+
|  Footer                                                   |
|  Tavern · WoW Emulation · Digital Preservation           |
|  Tavern is free software under the AGPL 3.0 license.      |
|  Tavern Account Management · Support                      |
+----------------------------------------------------------+
```

- **Top bar:** fixed height (48px), background `var(--card)`, bottom border
  `var(--divider)`. Contains brand text in `var(--accent)` on the left,
  account info and logout on the right.
- **Sidebar:** fixed width (200px desktop, collapses to hamburger on mobile).
  Each item is a `<a>` tag with `data-page` attribute for JS routing.
  Active item has background `var(--card)` and `var(--text)` color.
- **Content area:** flex-grows to fill remaining width. Composed of stacked
  `.section` cards, each containing a heading and key-value rows.
- **Footer:** matches the email bottom bar exactly: same text, same layout,
  same typography (11px, 18px line-height). Links to the project homepage,
  the GitHub repository, and the account management page.

### Responsive Breakpoints

| Breakpoint | Layout |
| --- | --- |
| >= 768px | Two-column: sidebar + content |
| < 768px | Single-column: sidebar collapses to hamburger toggle |

Below 768px the sidebar becomes a slide-out overlay triggered by a
hamburger icon in the top bar. The content area takes full width.

## Component Library

### Card (`.section`)

```text
+----------------------------------+
| Section Header                   |  <- h2 with bottom border
+----------------------------------+
| Label .............. Value       |  <- .kv row
| Label ...... Badge [Enabled]    |  <- .kv row with badge
| Label .............. Value       |
+----------------------------------+
```

- Background `var(--card)`, border-radius `var(--radius-lg)`, padding
  `var(--space-lg)`.
- Section header has bottom border `var(--divider)` and padding-bottom
  `var(--space-md)`.
- Key-value rows (`div.kv`) have flexible left/right layout with
  `justify-content: space-between` and bottom border `var(--divider)`.
- The last `div.kv` in a section has no bottom border.

### Key-Value Row (`.kv`)

```text
<span class="label">Label text</span>
<span class="value">Value or badge</span>
```

- Label: `var(--text-secondary)`, font-size `0.9rem`.
- Value: `var(--text)` for plain text, or a `.badge` component for
  boolean/status values.
- Empty values render an em-dash (`&mdash;`) in `var(--text-secondary)`.

### Badge (`.badge`)

```text
<span class="badge badge-ok">Enabled</span>
<span class="badge badge-warn">Disabled</span>
```

- Inline-block, padding `2px 10px`, border-radius `var(--radius-sm)`.
- `badge-ok`: green background (`var(--badge-ok)`) and text
  (`var(--badge-ok-text)`).
- `badge-warn`: amber background (`var(--badge-warn)`) and text
  (`var(--badge-warn-text)`).

### Button

| Variant | Background | Text | Hover |
| --- | --- | --- | --- |
| Primary (`.btn-primary`) | `var(--accent)` | White | `filter: brightness(1.1)` |
| Secondary (`.btn-secondary`) | None, border `var(--divider)` | `var(--text-secondary)` | `color: var(--text)` |
| Danger (`.btn-danger`) | `var(--error)` | White | `filter: brightness(1.1)` |

- Height: 40px, padding `0.75rem 1.5rem`, border-radius `var(--radius-md)`.
- Disabled state: opacity 0.5, cursor default.

### Input

```text
<input type="text|email|password|tel" class="input" />
```

- Full width within its container, padding `0.625rem`, border `1px solid
  var(--divider)`, border-radius `var(--radius-md)`.
- Background `var(--bg)`, text `var(--text)`.
- Focus: outline `2px solid var(--accent)` with `outline-offset: 1px`.
- Read-only: opacity 0.7.

### Loading State

```text
<div class="loading">
  <span class="spinner"></span>
  <p>Loading...</p>
</div>
```

- Centered text block, padding `2rem`, color `var(--text-secondary)`.
- Optional CSS-only spinner (rotating ring using `border-color` animation).

### Error State

```text
<div class="error">
  <p>Failed to load.</p>
</div>
```

- Centered text block, padding `2rem`, color `var(--error)`.

### Empty State

```text
<div class="empty">
  <p>No game accounts.</p>
</div>
```

- Centered text block, padding `2rem`, color `var(--text-secondary)`.

## Page Inventory

Each page maps to one or more `/api/*` endpoints. The SPA fetches on
navigation and renders into `#main-content`.

### Overview

| Section | Data Source | Fields |
| --- | --- | --- |
| Account Details | `/api/user` + `/api/details` | Email, BattleTag, First name, Country |
| Security Status | `/api/overview` | Email verified (badge), Authenticator (badge) |
| Quick Links | Static | Change password, Manage security |

The BattleTag from `/api/user` or `/api/details` populates the top-bar
`#header-battletag`. If neither resolves, the top bar shows nothing.

### Games & Subs

| Section | Data Source | Fields |
| --- | --- | --- |
| Game Accounts | `/api/games-and-subs` | Game account name/handle, Expansion level |
| Classic Games | `/api/classic-games` | Game name (if present) |

If no game accounts exist, show an empty state.

### Security

| Section | Data Source | Fields |
| --- | --- | --- |
| Authentication | `/api/security` | Authenticator (badge), SMS Protect (badge), Mobile alerts (badge) |
| Password | Static form | Old password, New password, Confirm button |

The password change form (`/api/security/password`) is a `.section` with
three inputs (old, new, confirm). On success, show a success toast (or
inline message) and clear the form. On error, show the error inline.

### Account Details

| Section | Data Source | Fields |
| --- | --- | --- |
| Personal Information | `/api/details` | Email, BattleTag, First name, Last name, Birth date, Country |
| Address | `/api/details/address` | Street, City, State, Postal code |

Read-only display for MVP. Edit mode (PUT back to the API) is deferred.

### Privacy

| Section | Data Source | Fields |
| --- | --- | --- |
| Communication Settings | `/api/privacy` | Marketing emails, Third-party data sharing, Personalized recommendations (all badges) |

All fields are read-only badges for MVP.

## Data Flow

### Bootstrap Sequence

```text
Browser                  Account Server
  |                            |
  |--- GET /overview -------->|
  |<-- HTML shell ------------|  (includes XSRF-TOKEN cookie)
  |                            |
  |--- POST /api/ ----------->|  (SESSIONID cookie sent automatically)
  |<-- {authenticated, ...} --|
  |                            |
  |--- GET /api/user -------->|  (X-XSRF-TOKEN header)
  |<-- {accountId, battleTag} |
  |                            |
  |--- GET /api/details ----->|  (X-XSRF-TOKEN header)
  |<-- {email, firstName, ...}|
  |                            |
  |--- GET /api/overview ---->|  (X-XSRF-TOKEN header)
  |<-- {game_accounts, ...}   |
  |                            |
  [Render Overview page]
```

1. Browser requests `GET /overview`. Server validates `SESSIONID` cookie,
   returns the HTML shell with a fresh `XSRF-TOKEN` cookie.
2. SPA calls `POST /api/` (no XSRF header per the capture). Returns
   `{authenticated, accountId, loginUri, logoutUri, accountCompletion}`.
3. If not authenticated, redirect to `/login/en/`.
4. If authenticated, fetch data for the default page (Overview).
5. All subsequent `/api/*` calls include `X-XSRF-TOKEN: <cookie-value>`.

### Navigation

```text
User clicks "Games & Subs" in sidebar
  |
  v
JS: link.click handler fires
  |
  +-> Set .active class on clicked link
  +-> Call loadPage("games")
  +-> Show .loading state in #main-content
  +-> Fetch /api/games-and-subs + /api/classic-games
  +-> Render page content
  +-> Replace #main-content innerHTML
```

### Error Handling

- **API returns 400/403:** Redirect to `/login/en/` (session expired).
- **API returns 5xx:** Show `.error` state with "Failed to load [page]."
  Allow retry by clicking the sidebar link again.
- **Network error:** Show `.error` state with "Network error. Please try
  again."
- **Empty data:** Show `.empty` state with contextual message.

### Logout

```text
User clicks "Log out"
  |
  v
POST /api/logout (no body)
  |
  v
Server deletes session row, clears SESSIONID cookie
  |
  v
Client redirects to /login/en/
```

## Static Assets

### File Layout

```text
crates/tavern-account/static/
  style.css       # Shared CSS: variables, typography, layout,
                  #   cards, badges, forms, buttons, loading/error/empty
  spa.js          # SPA JavaScript: bootstrap, routing, page renders,
                  #   API client, XSRF handling, logout, helpers
  srp6a.js        # Existing SRP crypto module (unchanged)
```

### CSS Organization

`style.css` is a single file (no build step). Sections:

1. **CSS Custom Properties** — the full color palette, spacing scale,
   border-radius, shadows.
2. **Reset** — `* { margin: 0; padding: 0; box-sizing: border-box; }`
3. **Body** — background, font-family, text color, min-height, flex column.
4. **Top bar** — `.top-bar`
5. **Layout** — `.layout` (flex container), `.sidebar`, `.content`
6. **Sidebar** — `.sidebar a`, `.sidebar a.active`, `.sidebar a:hover`
7. **Section card** — `.section`, `.section h2`
8. **Key-value row** — `.kv`, `.kv .label`
9. **Badge** — `.badge`, `.badge-ok`, `.badge-warn`
10. **Forms** — `.form-group`, `input`, `button`, `.btn-primary`,
   `.btn-secondary`, `.btn-danger`
11. **States** — `.loading`, `.error`, `.empty`
12. **Footer** — `.footer`
13. **Responsive** — `@media (max-width: 767px)` rules

### JavaScript Organization

`spa.js` is a single vanilla JS module. No bundler, no framework.

Functions:

- `getXsrfToken()` — read `XSRF-TOKEN` cookie
- `api(path, opts)` — fetch wrapper that sets `X-XSRF-TOKEN`, handles
  400/403
- `bootstrap()` — `POST /api/`, check authentication
- `loadPage(page)` — show loading, dispatch to `render*` function
- `renderOverview(main)` — fetch `/api/user`, `/api/details`, `/api/overview`
- `renderGames(main)` — fetch `/api/games-and-subs`, `/api/classic-games`
- `renderSecurity(main)` — fetch `/api/security`, render password change form
- `renderDetails(main)` — fetch `/api/details`
- `renderPrivacy(main)` — fetch `/api/privacy`
- `kv(label, value)` — helper to render a key-value row
- `badge(cond, yes, no)` — helper to render a badge
- `changePassword(event)` — handle password change form submission

Event listeners:

- Sidebar link clicks → `loadPage()`
- Logout button click → `POST /api/logout` then redirect
- Password form submit → `changePassword()`
- `Enter` key on password field → trigger login

## Implementation Plan

### Step 1: Extract shared CSS

Move the CSS variables and common styles from `landing.html`, `login.html`,
`dashboard.html`, and `creation.html` into `static/style.css`. Each template
links to `/static/style.css` instead of duplicating the `<style>` block. The
email template (`email.rs`) keeps its own inline styles for email client
compatibility.

### Step 2: Refactor SPA JavaScript

Move the JavaScript from `dashboard.html` into `static/spa.js`. The template
references it via `<script src="/static/spa.js">`. Add the missing render
functions and error handling.

### Step 3: Apply consistent component styling

Ensure every page uses the same card, badge, button, and form components from
`style.css`. The login form, creation wizard, and dashboard all share the
same visual tokens.

### Step 4: Add responsive sidebar

Implement the hamburger-toggle sidebar collapse for mobile. The sidebar
slides in from the left on small screens; tapping outside dismisses it.

### Step 5: Add loading/error/empty states

Every `render*` function wraps fetches in try/catch with appropriate state
display. Each page section handles the case where its API call fails or
returns empty.

### Step 6: Verify cross-browser and cross-locale

Test CJK locale rendering (Korean, Japanese, Chinese) with the full system
font stack. Test light/dark mode on macOS, Windows, Linux. Test mobile
viewport.

## Appendix: Theme Alignment Map

| Email Element | SPA Equivalent | Notes |
| --- | --- | --- |
| Logo (Tavern emblem, two theme PNGs) | Top-bar emblem + wordmark (`/static/tavern-logo-*.png`) | Same black-on-light / white-on-dark variants under `prefers-color-scheme` |
| Dark background (`#15171e`) | Page background (`--bg` dark) | Same value |
| Body text (`#d5d7dd`) | Text color (`--text` dark) | Same value |
| Footer text/layout | Page footer | Same 11px, same links, same AGPL mention |
| Security bar (lock icon + BattleTag) | N/A (in-app, not needed) | Only in transactional emails |
| Divider (`#2a3a52`) | Card borders, HRs (`--divider`) | Same value |
| Link color (`#ff8c42`) | Inline links (`--link` dark) | Same value |
| Accent orange (`#ff5202`) | Buttons, active states (`--accent`) | Same value |
