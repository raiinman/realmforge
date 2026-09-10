// SPDX-License-Identifier: AGPL-3.0-only

//! Welcome email and email verification with locale support.
//!
//! Emails match the captured Battle.net format exactly: dark-themed table
//! layout with logo → divider → body → footer → bottom bar. Tavern branding
//! replaces Blizzard/Battle.net.
//!
//! The Tavern emblem is embedded as a CID inline image (not remote-hosted):
//! email clients block remote images by default, so a `multipart/related`
//! message carries the logo bytes alongside the `alternative` text/HTML body.
//! Two glyph variants are embedded — black-on-light and white-on-dark — and
//! the HTML swaps between them under `prefers-color-scheme`, matching how the
//! rest of the message already themes itself. Sources:
//! `static/email-logo-light.png` and `static/email-logo-dark.png` (derived from
//! `docs/assets/tavern-logo-monochrome.svg`).

use lettre::message::header::{ContentDisposition, ContentId, ContentType};
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Content-IDs of the two embedded emblem parts (referenced as `cid:` in HTML).
const LOGO_LIGHT_CID: &str = "tavern-logo-light"; // black glyph, light theme
const LOGO_DARK_CID: &str = "tavern-logo-dark"; // white glyph, dark theme

/// Embedded emblem bytes (email-sized, 192px, transparent background).
const LOGO_LIGHT_PNG: &[u8] = include_bytes!("../static/email-logo-light.png");
const LOGO_DARK_PNG: &[u8] = include_bytes!("../static/email-logo-dark.png");

/// The PNG `Content-Type` for the embedded emblem parts. Infallible for the
/// literal media type; panics are unreachable.
fn png_content_type() -> ContentType {
    ContentType::parse("image/png")
        .unwrap_or_else(|_| unreachable!("\"image/png\" is a fixed valid MIME type"))
}
/// Send a welcome email localized to the account's locale.
#[allow(clippy::too_many_arguments)]
pub async fn send_welcome_email(
    signing_key_pem: &str,
    smtp_host: &str,
    smtp_port: u16,
    smtp_from: &str,
    to_email: &str,
    account_id: i64,
    account_server_url: &str,
    locale: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let from: Mailbox = smtp_from.parse()?;
    let to: Mailbox = format!("<{to_email}>").parse()?;

    let verify_ticket = tavern_core::encrypted_ticket::seal(account_id, signing_key_pem)
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "ticket encryption failed");
            uuid::Uuid::new_v4().simple().to_string()
        });

    let verify_url = format!("{account_server_url}/overview?ticket={verify_ticket}");
    let (subject, body_html, footer_html) =
        localized_welcome(locale, to_email, &verify_url, account_server_url, None);
    let plain = localized_plain(locale, to_email, &verify_url, account_server_url);

    let html = wrap_email(
        &subject,
        &body_html,
        &footer_html,
        account_server_url,
        None,
        locale,
    );

    // Both emblem variants ride along as inline parts; the HTML decides which
    // one is visible via a `prefers-color-scheme` media query (see wrap_email).
    let logo_light = SinglePart::builder()
        .header(png_content_type())
        .header(ContentDisposition::inline())
        .header(ContentId::from(format!("<{LOGO_LIGHT_CID}>")))
        .body(LOGO_LIGHT_PNG.to_vec());
    let logo_dark = SinglePart::builder()
        .header(png_content_type())
        .header(ContentDisposition::inline())
        .header(ContentId::from(format!("<{LOGO_DARK_CID}>")))
        .body(LOGO_DARK_PNG.to_vec());

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .multipart(
            MultiPart::related()
                .multipart(MultiPart::alternative_plain_html(plain, html))
                .singlepart(logo_light)
                .singlepart(logo_dark),
        )?;

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host)
        .port(smtp_port)
        .build();

    mailer.send(email).await?;
    tracing::info!(account_id, to_email, locale, "welcome email sent");
    Ok(())
}

// ---------------------------------------------------------------------------
// Reusable header / footer
// ---------------------------------------------------------------------------

const EMAIL_STYLES: &str = "font-family:'Noto Sans','Open Sans',Frutiger,'Frutiger \
    Linotype',Univers,'Helvetica Neue',Helvetica,Arial,'Gill Sans','Gill Sans \
    MT','Myriad Pro',Myriad,'DejaVu Sans Condensed','Liberation Sans','Nimbus \
    Sans L','Malgun Gothic','Microsoft YaHei',AppleSDGothicNeo,AppleGothic, \
    Dotum,'Microsoft JhengHei','Hiragino Kaku Gothic Pro','Hiragino Kaku Gothic \
    ProN W3',Osaka,sans-serif";

/// The header `<tr>` block: centered emblem, optional security bar, divider.
fn header_html(battletag: Option<&str>, locale: &str) -> String {
    let security_bar = match battletag {
        Some(tag) => security_note(tag, locale),
        None => String::new(),
    };
    // Two emblem <img>s reference the embedded CID parts; visibility is toggled
    // by the `.logo-light` / `.logo-dark` rules in wrap_email's <style>.
    let logo = format!(
        r#"<img class="logo-light" src="cid:{light}" width="120" height="120"
             alt="Tavern" style="width:120px;height:120px;border:0;line-height:1"/>
<img class="logo-dark" src="cid:{dark}" width="120" height="120"
       alt="Tavern" style="width:120px;height:120px;border:0;line-height:1"/>"#,
        light = LOGO_LIGHT_CID,
        dark = LOGO_DARK_CID,
    );
    format!(
        r#"<!-- Logo -->
<table width="600" border="0" cellpadding="0" cellspacing="0" align="center"
       style="width:600px;min-width:600px;border-spacing:0;border-collapse:collapse;
              margin:0 auto;word-wrap:break-word;word-break:break-word;
              background:var(--bg)">
  <tr><td style="padding:32px 0 8px;text-align:center;background:var(--bg)">
    {logo}
  </td></tr>
</table>

{security_bar}

<!-- Divider -->
<table width="600" border="0" cellpadding="0" cellspacing="0" align="center"
       style="width:600px;min-width:600px;border-spacing:0;border-collapse:collapse;
              margin:0 auto;word-wrap:break-word;word-break:break-word;
              background:var(--bg)">
  <tr><td style="padding:0;background:var(--bg)">
    <table width="100%" cellspacing="0" cellpadding="0" border="0">
      <tr><td style="padding:22px 0">
        <hr style="border:none;border-top:1px solid var(--divider);margin:0">
      </td></tr>
    </table>
  </td></tr>
</table>"#,
    )
}

/// Security note showing the user's BattleTag (not shown in signup emails).
fn security_note(battletag: &str, locale: &str) -> String {
    let text = match locale {
        "deDE" => format!(
            "Hallo! Falls Ihre BattleTag nicht <b>{battletag}</b> lautet, \
             klicken Sie auf keine Links in dieser E-Mail!"
        ),
        "koKR" => format!(
            "안녕하세요! 회원님의 BattleTag가 <b>{battletag}</b> 이(가) 아닐 경우 \
             이메일 내 링크를 클릭하지 마세요!"
        ),
        _ => format!(
            "Hello! If your BattleTag is not <b>{battletag}</b>, \
             do not click any links in this email!"
        ),
    };
    format!(
        r#"<table width="600" border="0" cellpadding="0" cellspacing="0" align="center"
       style="width:600px;min-width:600px;border-spacing:0;border-collapse:collapse;
              margin:0 auto;word-wrap:break-word;word-break:break-word;
              background:var(--bg)">
  <tr>
    <td style="padding:0 40px 0 43px;font-size:14px;line-height:22px;
               color:var(--text-secondary);text-align:left;
               background:var(--bg);font-family:inherit">
      &#x1f512; {text}
    </td>
  </tr>
</table>"#
    )
}

fn footer_html(server_url: &str, locale: &str, _email: &str) -> String {
    let signoff = match locale {
        "deDE" => "Vielen Dank,<br><b>Tavern</b>",
        "koKR" => "감사합니다,<br><b>Tavern</b>",
        _ => "Thank you,<br><b>Tavern</b>",
    };
    let support = match locale {
        "deDE" => "Support",
        "koKR" => "고객지원",
        _ => "Support",
    };
    format!(
        r#"<!-- Signoff -->
<table width="600" border="0" cellpadding="0" cellspacing="0" align="center"
       style="width:600px;min-width:600px;border-spacing:0;border-collapse:collapse;
              margin:0 auto;word-wrap:break-word;word-break:break-word;
              background:var(--bg)">
  <tr>
    <td style="padding:0 40px 40px;font-size:14px;line-height:24px;color:#d5d7dd;color:var(--text);
               text-align:left;background:var(--bg);{EMAIL_STYLES}">
      {signoff}
    </td>
  </tr>
</table>

<!-- Bottom bar -->
<table width="600" border="0" cellpadding="0" cellspacing="0" align="center"
       style="width:600px;min-width:600px;border-spacing:0;border-collapse:collapse;
              margin:0 auto;word-wrap:break-word;word-break:break-word;
              background:var(--footer-bg)">
  <tr>
    <td style="padding:24px 40px;text-align:center;font-size:11px;
               line-height:18px;color:#b0b8c8;color:var(--footer-text);
               background:var(--footer-bg);
               {EMAIL_STYLES}">
      Tavern &middot; <a href="https://wowemulation.dev/" style="color:#b0b8c8;color:var(--footer-text);text-decoration:underline">WoW Emulation</a> &middot; Digital Preservation<br>
      <br>
      <a href="https://github.com/wowemulation-dev/tavern" style="color:#b0b8c8;color:var(--footer-text);text-decoration:underline">Tavern</a> is free software under the AGPL 3.0 license.<br>
      <a href="{server_url}" style="color:#b0b8c8;color:var(--footer-text);text-decoration:none">
        Tavern Account Management</a> &middot;
      <a href="{server_url}" style="color:#b0b8c8;color:var(--footer-text);text-decoration:none">
        {support}</a>
    </td>
  </tr>
</table>"#,
    )
}

/// Wrap body + footer in the full Battle.net-style email template.
fn wrap_email(
    subject: &str,
    body: &str,
    footer: &str,
    _server_url: &str,
    battletag: Option<&str>,
    locale: &str,
) -> String {
    format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<meta name="supported-color-schemes" content="light dark">
<title>{subject}</title>
<style>
  :root {{
    --bg: #dfe0e6;
    --text: #1a1a2e;
    --text-secondary: #4b5563;
    --footer-bg: #ffffff;
    --footer-text: #4b5563;
    --divider: #d1d5db;
    --link: #e04800;
    --logo: #e04800;
  }}
  /* Emblem: black glyph on light (default), white glyph on dark. Exactly one
     <img> is visible at a time. Clients that ignore <style> (none mainstream)
     would show both; the fallback for image-blocking clients is the alt text. */
  img.logo-light {{
    display: block; margin: 0 auto;
  }}
  img.logo-dark {{
    display: none;
  }}
  @media (prefers-color-scheme: dark) {{
    :root {{
      --bg: #15171e;
      --text: #d5d7dd;
      --text-secondary: #b0b8c8;
      --footer-bg: #111318;
      --footer-text: #b0b8c8;
      --divider: #2a3a52;
      --link: #ff8c42;
      --logo: #ff5202;
    }}
    img.logo-light {{
      display: none;
    }}
    img.logo-dark {{
      display: block; margin: 0 auto;
    }}
  }}
</style></head>
<body style="background:var(--bg);margin:0;padding:0">
<center style="margin:0;padding:0">
{header}

<!-- Body -->
<table width="600" border="0" cellpadding="0" cellspacing="0" align="center"
       style="width:600px;min-width:600px;border-spacing:0;border-collapse:collapse;
              margin:0 auto;word-wrap:break-word;word-break:break-word;
              background:var(--bg)">
  <tr>
    <td style="padding:0 40px 40px;font-size:14px;line-height:24px;
               color:var(--text);text-align:left;background:var(--bg);
               {EMAIL_STYLES}">
      {body}
    </td>
  </tr>
</table>

{footer}
</center>
</body>
</html>"#,
        header = header_html(battletag, locale),
    )
}

// ---------------------------------------------------------------------------
// Localized content
// ---------------------------------------------------------------------------

fn localized_welcome(
    locale: &str,
    to_email: &str,
    verify_url: &str,
    server_url: &str,
    battletag: Option<&str>,
) -> (String, String, String) {
    match locale {
        "deDE" => de_body(to_email, verify_url, server_url, battletag),
        "koKR" => ko_body(to_email, verify_url, server_url, battletag),
        _ => en_body(to_email, verify_url, server_url, battletag),
    }
}

fn localized_plain(locale: &str, to_email: &str, verify_url: &str, server_url: &str) -> String {
    match locale {
        "deDE" => format!(
            "Hallo,\n\nVielen Dank, dass Sie ein Tavern-Konto erstellt haben!\n\n\
             Das folgende Konto wurde erstellt:\n\n{to_email}\n\n\
             Bestätigen Sie Ihre E-Mail-Adresse:\n\n{verify_url}\n\n\
             Kontoverwaltung: {server_url}\n\n\
             Falls Sie dieses Konto nicht erstellt haben, ignorieren Sie diese E-Mail.\n\n\
             Vielen Dank,\nTavern"
        ),
        "koKR" => format!(
            "안녕하세요,\n\nTavern 회원으로 가입해 주셔서 감사합니다!\n\n\
             다음과 같은 계정이 생성되었습니다:\n\n{to_email}\n\n\
             이메일 인증: {verify_url}\n\n\
             계정 관리: {server_url}\n\n\
             본인이 생성하지 않은 경우 이 메일을 무시하십시오.\n\n\
             감사합니다,\nTavern"
        ),
        _ => format!(
            "Hello,\n\nThank you for creating a Tavern account!\n\n\
             The following account has been created:\n\n{to_email}\n\n\
             Please verify your email address:\n\n{verify_url}\n\n\
             Account management: {server_url}\n\n\
             If you did not create this account, you can ignore this email.\n\n\
             Thank you,\nTavern"
        ),
    }
}

fn en_body(
    to_email: &str,
    verify_url: &str,
    _server_url: &str,
    _battletag: Option<&str>,
) -> (String, String, String) {
    let subject = "Tavern Account - Verify Your Email".into();
    let body = format!(
        "Hello,<br><br>\
         Thank you for creating a <b>Tavern</b> account!<br><br>\
         The following <b>Tavern</b> account has been created:<br><br>\
         <b>{to_email}</b><br><br>\
         Please verify your email address by visiting the link below.<br><br>\
         <a href=\"{verify_url}\" style=\"color:#ff5202!important;\
            text-decoration:underline!important\">{verify_url}</a><br><br>\
         You can manage your account settings and security information \
         at any time by visiting the Tavern Account Management page."
    );
    let footer = footer_html(_server_url, "enUS", "");
    (subject, body, footer)
}

fn de_body(
    to_email: &str,
    verify_url: &str,
    server_url: &str,
    _battletag: Option<&str>,
) -> (String, String, String) {
    let subject = "Tavern Konto - E-Mail-Adresse bestätigen".into();
    let body = format!(
        "Hallo,<br><br>\
         Vielen Dank, dass Sie ein <b>Tavern</b>-Konto erstellt haben!<br><br>\
         Das folgende <b>Tavern</b>-Konto wurde erstellt:<br><br>\
         <b>{to_email}</b><br><br>\
         Bitte bestätigen Sie Ihre E-Mail-Adresse, indem Sie den \
         untenstehenden Link besuchen.<br><br>\
         <a href=\"{verify_url}\" style=\"color:#ff5202!important;\
            text-decoration:underline!important\">{verify_url}</a><br><br>\
         Sie können Ihre Kontoeinstellungen und Sicherheitsinformationen \
         jederzeit auf der Tavern Kontoverwaltungsseite verwalten."
    );
    let footer = footer_html(server_url, "deDE", "");
    (subject, body, footer)
}

fn ko_body(
    to_email: &str,
    verify_url: &str,
    server_url: &str,
    _battletag: Option<&str>,
) -> (String, String, String) {
    let subject = "Tavern 계정 - 이메일 인증".into();
    let body = format!(
        "안녕하세요,<br><br>\
         <b>Tavern</b> 회원으로 가입해 주셔서 감사합니다!<br><br>\
         다음과 같은 <b>Tavern</b> 계정을 만드셨습니다:<br><br>\
         <b>{to_email}</b><br><br>\
         아래 링크를 클릭하여 이메일 주소를 인증해 주세요.<br><br>\
         <a href=\"{verify_url}\" style=\"color:#ff5202!important;\
            text-decoration:underline!important\">{verify_url}</a><br><br>\
         <a href=\"{server_url}\" style=\"color:#ff5202!important;\
            text-decoration:underline!important\">\
         Tavern 계정 관리</a> 페이지에서 언제든지 계정을 관리할 수 있습니다."
    );
    let footer = footer_html(server_url, "koKR", "");
    (subject, body, footer)
}

// ---------------------------------------------------------------------------
// Email verification
// ---------------------------------------------------------------------------

/// Mark an account's email as verified.
pub async fn verify_email(pool: &sqlx::PgPool, account_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE accounts SET email_verified = true, updated_at = NOW() WHERE id = $1",
        account_id
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailbox_parsing_works() {
        let m: Mailbox = "noreply@wowemu.dev".parse().unwrap();
        assert_eq!(m.to_string(), "noreply@wowemu.dev");
    }

    #[test]
    fn email_renders() {
        let (subject, body, footer) = en_body(
            "user@example.com",
            "https://example.com/overview?ticket=x",
            "https://example.com",
            None,
        );
        assert!(subject.contains("Tavern"));
        assert!(body.contains("user@example.com"));
        let html = wrap_email(
            &subject,
            &body,
            &footer,
            "https://example.com",
            None,
            "enUS",
        );
        // Header carries both embedded emblem variants.
        assert!(html.contains("cid:tavern-logo-light"), "missing light cid");
        assert!(html.contains("cid:tavern-logo-dark"), "missing dark cid");
        assert!(html.contains("img.logo-light"), "missing light toggle rule");
        assert!(html.contains("img.logo-dark"), "missing dark toggle rule");
        assert!(html.contains("Thank you for creating"));
    }

    #[test]
    fn embedded_logo_bytes_are_nonempty_png() {
        // Guard against accidentally blank/alpha-stripped assets.
        assert!(LOGO_LIGHT_PNG.len() > 1000);
        assert!(LOGO_DARK_PNG.len() > 1000);
        assert_eq!(
            &LOGO_LIGHT_PNG[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
        assert_eq!(
            &LOGO_DARK_PNG[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
    }
}
