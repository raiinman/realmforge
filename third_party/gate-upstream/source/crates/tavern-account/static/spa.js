// SPDX-License-Identifier: AGPL-3.0-only
//
// Tavern account management SPA — vanilla JavaScript, no framework.
//
// Bootstrap: POST /api/ → check auth, then render pages client-side
// from /api/* JSON endpoints. Session via SESSIONID cookie; protected
// endpoints use XSRF-TOKEN / X-XSRF-TOKEN double-submit.
//
// Reference: docs/spa-design.md



// ---------------------------------------------------------------------------
// API client
// ---------------------------------------------------------------------------

const API_BASE = "/api";

/**
 * Read the XSRF-TOKEN cookie value for the X-XSRF-TOKEN header.
 */
function getXsrfToken() {
  const m = document.cookie.match(/(?:^|;\s*)XSRF-TOKEN=([^;]*)/);
  return m ? m[1] : "";
}

/**
 * Fetch wrapper. Sets X-XSRF-TOKEN automatically. On 400/403, redirects
 * to the login page (session expired).
 */
async function api(path, opts) {
  const o = opts || {};
  o.headers = o.headers || {};
  if (!o.headers["X-XSRF-TOKEN"]) {
    o.headers["X-XSRF-TOKEN"] = getXsrfToken();
  }
  const resp = await fetch(API_BASE + path, o);
  if (!resp.ok) {
    if (resp.status === 400 || resp.status === 403) {
      window.location.href = "/login/en/";
      throw new Error("Session expired");
    }
    throw new Error(path + ": HTTP " + resp.status);
  }
// Handlers may return an empty 200 (e.g. PUTs that write without a body).
const text = await resp.text();
return text ? JSON.parse(text) : null;
}

/**
 * Show a transient toast notification (auto-dismisses after 3.5s).
 * type: "success" | "error".
 */
function showToast(message, type) {
  let container = document.getElementById("toast-container");
  if (!container) {
    container = document.createElement("div");
    container.id = "toast-container";
    document.body.appendChild(container);
  }
  const toast = document.createElement("div");
  toast.className = "toast toast-" + (type === "error" ? "error" : "success");
  toast.textContent = message;
  container.appendChild(toast);
  setTimeout(() => {
    toast.classList.add("toast-hide");
    setTimeout(() => toast.remove(), 300);
  }, 3500);
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

/**
 * POST /api/ — check authentication. Redirects to login if
 * unauthenticated.
 */
async function bootstrap() {
  try {
    const data = await api("/", { method: "POST" });
    if (!data.authenticated) {
      window.location.href = "/login/en/";
      return false;
    }
    return true;
  } catch {
    window.location.href = "/login/en/";
    return false;
  }
}

// ---------------------------------------------------------------------------
// Page navigation
// ---------------------------------------------------------------------------

/**
 * Load a page by name. Shows loading state, fetches data, renders.
 */
async function loadPage(page) {
  const main = document.getElementById("main-content");
  main.innerHTML =
    '<div class="loading"><span class="spinner"></span> Loading\u2026</div>';

  try {
    switch (page) {
      case "overview":
        await renderOverview(main);
        break;
      case "games":
        await renderGames(main);
        break;
      case "security":
        await renderSecurity(main);
        break;
      case "details":
        await renderDetails(main);
        break;
      case "privacy":
        await renderPrivacy(main);
        break;
      default:
        main.innerHTML = '<div class="error">Unknown page.</div>';
    }
  } catch (e) {
    console.error("loadPage error:", e);
    main.innerHTML =
      '<div class="error"><p>Failed to load page.</p><p style="font-size:0.875rem;margin-top:0.5rem">Please try again.</p></div>';
  }
}

// ---------------------------------------------------------------------------
// Page renders
// ---------------------------------------------------------------------------

async function renderOverview(main) {
  const [overview, details, user] = await Promise.all([
    api("/overview").catch(() => null),
    api("/details").catch(() => null),
    api("/user").catch(() => null),
  ]);

  const bt =
    user?.battleTag?.name && user.battleTag.code
      ? user.battleTag.name + "#" + user.battleTag.code
      : details?.battleTag || "";
  const headerBt = document.getElementById("header-battletag");
  if (headerBt) headerBt.textContent = bt;

  let html = "";
  html +=
    '<h1 class="page-title" style="font-size:1.5rem;margin-bottom:1.5rem;color:var(--accent)">' +
    escapeHtml(bt || "Account") +
    "</h1>";

  // Unverified email banner (matches account.overview.email.unverified.*).
  const sec = overview?.accountSecurityStatus;
  if (sec && sec.emailVerified === false) {
    const email = details?.email || "your email address";
    html +=
      '<div class="banner" style="border:1px solid var(--warning,#f39c12);border-radius:0.5rem;padding:0.75rem 1rem;margin-bottom:1.5rem;background:rgba(243,156,18,0.08)">' +
      "<p style='margin:0 0 0.25rem'><strong>The email address (" +
      escapeHtml(email) +
      ") is unverified.</strong></p>" +
      "<p style='margin:0 0 0.5rem;color:var(--text-secondary);font-size:0.9rem'>" +
      "Verifying your email address adds additional security and lets you recover your account if you ever can't access it.</p>" +
      "<button id='resend-verification' class='btn btn-secondary' style='font-size:0.85rem;padding:0.35rem 0.75rem'>Resend Verification Email</button>" +
      "</div>";
  }

  html += '<div class="section"><h2>Account Details</h2>';
  html +=
    kv("Email", details?.email) +
    kv("BattleTag", details?.battleTag) +
    kv("First name", details?.firstName) +
    kv("Country", details?.countryCodeAlpha3);
  html += "</div>";

  if (overview?.accountSecurityStatus) {
    const s = overview.accountSecurityStatus;
    html += '<div class="section"><h2>Security</h2>';
    html +=
      kv("Email verified", badge(s.emailVerified, "Verified", "Not verified")) +
      kv(
        "Authenticator",
        badge(s.authenticatorAttached, "Attached", "Not set up"),
      );
    html += "</div>";
  }

  main.innerHTML = html;
  const resendBtn = document.getElementById("resend-verification");
  if (resendBtn) {
    resendBtn.addEventListener("click", async () => {
      try {
        await api("/email/verification", { method: "POST" });
        resendBtn.textContent = "Verification email sent.";
        resendBtn.disabled = true;
        showToast("Verification email sent.", "success");
      } catch {
        showToast("Failed to send verification email.", "error");
      }
    });
  }
}

async function renderGames(main) {
  const [games, classic] = await Promise.all([
    api("/games-and-subs").catch(() => null),
    api("/classic-games").catch(() => null),
  ]);

  let html =
    '<h1 class="page-title" style="font-size:1.5rem;margin-bottom:1.5rem;color:var(--accent)">Games &amp; Subscriptions</h1>';

  html += '<div class="section"><h2>Game Accounts</h2>';
  const items = games?.gameAccounts || [];
  if (items.length === 0) {
    html += '<p class="empty">No game accounts.</p>';
  }
  for (const g of items) {
    const status = g.gameAccountStatus || "";
    const region = g.gameAccountRegion || "";
    const statusColor =
      status === "Good"
        ? "var(--success, #2ecc71)"
        : status === "Trial"
          ? "var(--warning, #f39c12)"
          : "var(--error, #e74c3c)";
    const sub =
      g.accountSubscriptionView?.subscriptionStatus === "ACTIVE"
        ? '<span style="color:var(--success, #2ecc71)">Subscription active</span>'
        : '<span style="color:var(--text-secondary)">No subscription</span>';
    html +=
      '<div style="display:flex;align-items:center;gap:1rem;padding:0.75rem 0;border-bottom:1px solid var(--border,#333)">' +
      "<div style='flex:1'><strong>" +
      escapeHtml(g.gameAccountName || "") +
      "</strong><br/><span style='color:var(--text-secondary);font-size:0.85rem'>" +
      escapeHtml(g.localizedGameName || "") +
      "</span></div><span style='color:var(--text-secondary);font-size:0.85rem'>" +
      escapeHtml(region) +
      "</span> <span style='color:" +
      statusColor +
      ";font-size:0.85rem'>" +
      escapeHtml(status) +
      "</span> <span style='font-size:0.85rem'>" +
      sub +
      "</span></div>";
  }
  html += "</div>";

  if (classic?.classicGames?.length > 0) {
    html += '<div class="section"><h2>Classic Games</h2>';
    for (const g of classic.classicGames) {
      html += kv(escapeHtml(g.localizedGameName || ""), "");
    }
    html += "</div>";
  }
  main.innerHTML = html;
}

async function renderSecurity(main) {
  // Fetch security info and render the page with password change form.
  let sec = null;
  try {
    sec = await api("/security");
  } catch {
    main.innerHTML =
      '<div class="error"><p>Failed to load security information.</p></div>';
    return;
  }

  let html =
    '<h1 class="page-title" style="font-size:1.5rem;margin-bottom:1.5rem;color:var(--accent)">Security</h1>' +
    '<div class="section"><h2>Authentication</h2>';
  html +=
    kv(
      "Authenticator",
      badge(sec.blizzardAuthenticatorAttached, "Attached", "Not set up"),
    ) +
    kv(
      "SMS Protect",
      badge(sec.smsProtectActive, "Active", "Not set up"),
    ) +
    kv(
      "Mobile alerts",
      badge(sec.mobileAlertsEnabled, "Enabled", "Disabled"),
    );
  html += "</div>";

  // Password change form.
  html += '<div class="section"><h2>Change Password</h2>';
  html += '<form id="password-form" onsubmit="handlePasswordChange(event)">';
  html +=
    '<div class="form-group"><label for="pw-old">Current password</label>' +
    '<input type="password" id="pw-old" name="old_password" required autocomplete="current-password"></div>';
  html +=
    '<div class="form-group"><label for="pw-new">New password</label>' +
    '<input type="password" id="pw-new" name="new_password" minlength="8" required autocomplete="new-password"></div>';
  html +=
    '<div class="form-group"><label for="pw-confirm">Confirm new password</label>' +
    '<input type="password" id="pw-confirm" name="confirm" minlength="8" required autocomplete="new-password"></div>';
  html += '<div id="pw-error" class="form-error hidden"></div>';
  html += '<div id="pw-success" class="hidden" style="color:var(--badge-ok-text);font-size:0.875rem;margin-bottom:var(--space-md)">Password changed successfully.</div>';
  html +=
    '<button type="submit" class="btn btn-primary">Change Password</button>';
  html += "</form>";
  html += "</div>";

  main.innerHTML = html;
}

async function renderDetails(main) {
  let det = null;
  try {
    det = await api("/details");
  } catch {
    main.innerHTML =
      '<div class="error"><p>Failed to load account details.</p></div>';
    return;
  }

  let html =
    '<h1 class="page-title" style="font-size:1.5rem;margin-bottom:1.5rem;color:var(--accent)">Account Details</h1>' +
    '<div class="section"><h2>Personal Information</h2>';
  html +=
    kv("Email", det.email) +
    kv("BattleTag", det.battleTag) +
    kv("First name", det.firstName) +
    kv("Last name", det.lastName) +
    kv("Birth date", det.birthDate) +
    kv("Country", det.countryCodeAlpha3);
  html += "</div>";

  main.innerHTML = html;
}

async function renderPrivacy(main) {
  let priv = null, comm = null;
  try {
    const [p, c] = await Promise.all([
      api("/privacy").catch(() => null),
      api("/communication-preferences").catch(() => null),
    ]);
    priv = p;
    comm = c;
  } catch {
    main.innerHTML =
      '<div class="error"><p>Failed to load privacy settings.</p></div>';
    return;
  }

  let html =
    '<h1 class="page-title" style="font-size:1.5rem;margin-bottom:1.5rem;color:var(--accent)">Privacy</h1>';

  // Social / chat / voice settings (PUT /api/privacy/social).
  const socialFields = [
    ["enableTextChat", "Text chat"],
    ["enablePrivateTextChat", "Private text chat"],
    ["onlyAllowFriendWhispers", "Only allow friend whispers"],
    ["enableVoiceChat", "Voice chat"],
    ["enableVoiceChatSpeak", "Voice chat speak"],
    ["enableFriendsManagement", "Friends management"],
    ["enableGroups", "Groups"],
    ["enableRealId", "Real ID"],
    ["showRealId", "Show Real ID"],
    ["enableFriendsOfFriends", "Friends of friends"],
  ];
  html += '<div class="section"><h2>Social &amp; Chat</h2>';
  for (const [key, label] of socialFields) {
    const val = priv && typeof priv[key] === "boolean" ? priv[key] : false;
    html +=
      '<label style="display:flex;align-items:center;justify-content:space-between;padding:0.5rem 0;border-bottom:1px solid var(--border,#333)">' +
      "<span>" +
      escapeHtml(label) +
      "</span> <input type='checkbox' data-privacy-social='" +
      key +
      "' " +
      (val ? "checked" : "") +
      " /></label>";
  }
  html += "</div>";

  // Data sharing (PUT /api/privacy/data).
  const shareVal =
    priv && typeof priv.enableThirdPartySharing === "boolean"
      ? priv.enableThirdPartySharing
      : false;
  html += '<div class="section"><h2>Data Sharing</h2>';
  html +=
    '<label style="display:flex;align-items:center;justify-content:space-between;padding:0.5rem 0;border-bottom:1px solid var(--border,#333)">' +
    "<span>Share data with third parties</span> <input type='checkbox' id='privacy-third-party' " +
    (shareVal ? "checked" : "") +
    " /></label>";
  html += "</div>";

  // Communication preferences (read-only view).
  if (comm) {
    const commFields = [
      ["receiveTargetedAds", "Targeted ads"],
      ["receiveBlizzardOffers", "Blizzard offers"],
      ["enableBlizzardPersonalizedProductRecommendations", "Personalized recommendations"],
      ["worldOfWarcraftCommunications", "World of Warcraft comms"],
    ];
    html += '<div class="section"><h2>Communication Preferences</h2>';
    for (const [key, label] of commFields) {
      const v = typeof comm[key] === "boolean" ? comm[key] : false;
      html += kv(label, badge(v, "Enabled", "Disabled"));
    }
    html += "</div>";
  }

  main.innerHTML = html;

  // Build the social payload from the current checkboxes and PUT on change.
  const buildSocial = () => {
    const body = {};
    document
      .querySelectorAll("[data-privacy-social]")
      .forEach((cb) => {
        body[cb.dataset.privacySocial] = cb.checked;
      });
    return body;
  };
  document
    .querySelectorAll("[data-privacy-social]")
    .forEach((cb) => {
      cb.addEventListener("change", async () => {
        try {
          await api("/privacy/social", {
            method: "PUT",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(buildSocial()),
          });
          showToast("Settings updated.", "success");
        } catch {
          cb.checked = !cb.checked;
          showToast("Failed to save setting.", "error");
        }
      });
    });
  const share = document.getElementById("privacy-third-party");
  if (share) {
    share.addEventListener("change", async () => {
      try {
        await api("/privacy/data", {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ thirdPartySharing: share.checked }),
        });
        showToast("Settings updated.", "success");
      } catch {
        share.checked = !share.checked;
        showToast("Failed to save setting.", "error");
      }
    });
  }
}

// ---------------------------------------------------------------------------
// Password change
// ---------------------------------------------------------------------------

async function handlePasswordChange(event) {
  event.preventDefault();

  const oldPw = document.getElementById("pw-old").value;
  const newPw = document.getElementById("pw-new").value;
  const confirm = document.getElementById("pw-confirm").value;
  const errEl = document.getElementById("pw-error");
  const successEl = document.getElementById("pw-success");
  const btn = event.target.querySelector("button[type=submit]");

  errEl.classList.add("hidden");
  successEl.classList.add("hidden");

  if (newPw !== confirm) {
    errEl.textContent = "New passwords do not match.";
    errEl.classList.remove("hidden");
    return;
  }

  if (newPw.length < 8) {
    errEl.textContent = "New password must be at least 8 characters.";
    errEl.classList.remove("hidden");
    return;
  }

  btn.disabled = true;
  btn.textContent = "Changing\u2026";

  try {
    const resp = await fetch(API_BASE + "/security/password", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-XSRF-TOKEN": getXsrfToken(),
      },
      body: JSON.stringify({
        old_password: oldPw,
        new_password: newPw,
      }),
    });

    if (!resp.ok) {
      if (resp.status === 401) {
        errEl.textContent = "Current password is incorrect.";
      } else if (resp.status === 400) {
        errEl.textContent = "Invalid request.";
      } else {
        errEl.textContent = "Server error. Please try again.";
      }
      errEl.classList.remove("hidden");
      btn.disabled = false;
      btn.textContent = "Change Password";
      return;
    }

    successEl.classList.remove("hidden");
    document.getElementById("pw-old").value = "";
    document.getElementById("pw-new").value = "";
    document.getElementById("pw-confirm").value = "";
  } catch {
    errEl.textContent = "Network error. Please try again.";
    errEl.classList.remove("hidden");
  }

  btn.disabled = false;
  btn.textContent = "Change Password";
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Render a key-value row (div.kv).
 */
function kv(label, value) {
  const v =
    value !== null && value !== undefined && value !== ""
      ? value
      : '<span style="color:var(--text-secondary)">&mdash;</span>';
  return (
    '<div class="kv"><span class="label">' +
    escapeHtml(label) +
    '</span><span class="value">' +
    v +
    "</span></div>"
  );
}

/**
 * Render a boolean badge.
 */
function badge(cond, yesText, noText) {
  const cls = cond ? "badge badge-ok" : "badge badge-warn";
  return '<span class="' + cls + '">' + (cond ? yesText : noText) + "</span>";
}

/**
 * Basic HTML escaping for user-provided strings.
 */
function escapeHtml(str) {
  if (!str) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

// ---------------------------------------------------------------------------
// Mobile sidebar
// ---------------------------------------------------------------------------

function initSidebar() {
  const hamburger = document.getElementById("hamburger");
  const sidebar = document.querySelector(".sidebar");
  const overlay = document.getElementById("sidebar-overlay");

  if (!hamburger || !sidebar || !overlay) return;

  hamburger.addEventListener("click", () => {
    sidebar.classList.add("open");
    overlay.classList.add("active");
  });

  overlay.addEventListener("click", () => {
    sidebar.classList.remove("open");
    overlay.classList.remove("active");
  });

  // Close sidebar when a nav link is clicked (mobile).
  sidebar.querySelectorAll("a").forEach((link) => {
    link.addEventListener("click", () => {
      sidebar.classList.remove("open");
      overlay.classList.remove("active");
    });
  });
}

// ---------------------------------------------------------------------------
// Event wiring
// ---------------------------------------------------------------------------

function initEvents() {
  // Sidebar navigation.
  document.querySelectorAll(".sidebar a").forEach((link) => {
    link.addEventListener("click", (e) => {
      e.preventDefault();
      document
        .querySelectorAll(".sidebar a")
        .forEach((l) => l.classList.remove("active"));
      link.classList.add("active");
      loadPage(link.dataset.page);
    });
  });

  // Logout button.
  const logoutBtn = document.getElementById("logout-btn");
  if (logoutBtn) {
    logoutBtn.addEventListener("click", async () => {
      try {
        await fetch(API_BASE + "/logout", { method: "POST" });
      } catch {
        // Proceed even if the request fails.
      }
      window.location.href = "/login/en/";
    });
  }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

(async () => {
  initSidebar();
  initEvents();
  const ok = await bootstrap();
  if (ok) loadPage("overview");
})();
