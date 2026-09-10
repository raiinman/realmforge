// SPDX-License-Identifier: AGPL-3.0-only

//! Locale and region mapping per Blizzard's Battle.net deployment model.
//!
//! Five regions with per-region default and optional locales. Country codes
//! (ISO 3166-1 alpha-3) map to their default region and locale.

/// A Battle.net region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    US,
    EU,
    KR,
    TW,
    CN,
}

impl Region {
    /// All supported locales for this region. The first is the default.
    pub fn locales(self) -> &'static [&'static str] {
        match self {
            Region::US => &["enUS", "esMX", "ptBR"],
            Region::EU => &[
                "enGB", "deDE", "frFR", "esES", "itIT", "ruRU", "plPL", "ptPT",
            ],
            Region::KR => &["koKR", "jaJP", "thTH"],
            Region::TW => &["zhTW"],
            Region::CN => &["zhCN"],
        }
    }

    pub fn default_locale(self) -> &'static str {
        self.locales()[0]
    }
}

/// Map a country code to its default region. Accepts ISO 3166-1 alpha-2
/// (e.g. `US`, `KR`) or alpha-3 (e.g. `USA`, `KOR`).
pub fn country_to_region(code: &str) -> Region {
    match code {
        // US (Americas, Oceania) — alpha-2
        "US" | "CA" | "MX" | "BR" | "AR" | "CL" | "CO" | "PE" | "AU" | "NZ"
        // alpha-3
        | "USA" | "CAN" | "MEX" | "BRA" | "ARG" | "CHL" | "COL" | "PER"
        | "AUS" | "NZL" => Region::US,
        // EU (Europe, Russia, Middle East, Africa) — alpha-2
        "GB" | "DE" | "FR" | "ES" | "IT" | "RU" | "PL" | "PT"
        | "AT" | "BE" | "BG" | "HR" | "CY" | "CZ" | "DK" | "EE"
        | "FI" | "GR" | "HU" | "IE" | "LV" | "LT" | "LU" | "MT"
        | "NL" | "NO" | "RO" | "SK" | "SI" | "SE" | "CH" | "UA"
        | "IL" | "TR" | "SA" | "AE" | "ZA"
        // alpha-3
        | "GBR" | "DEU" | "FRA" | "ESP" | "ITA" | "RUS" | "POL" | "PRT"
        | "AUT" | "BEL" | "BGR" | "HRV" | "CYP" | "CZE" | "DNK" | "EST"
        | "FIN" | "GRC" | "HUN" | "IRL" | "LVA" | "LTU" | "LUX" | "MLT"
        | "NLD" | "NOR" | "ROU" | "SVK" | "SVN" | "SWE" | "CHE" | "UKR"
        | "ISR" | "TUR" | "SAU" | "ARE" | "ZAF" => Region::EU,
        // KR (Korea, Japan, Southeast Asia) — alpha-2
        "KR" | "JP" | "TH" | "SG" | "MY" | "ID" | "PH" | "VN"
        // alpha-3
        | "KOR" | "JPN" | "THA" | "SGP" | "MYS" | "IDN" | "PHL" | "VNM" => Region::KR,
        // TW (Taiwan, Hong Kong, Macau) — alpha-2
        "TW" | "HK" | "MO"
        // alpha-3
        | "TWN" | "HKG" | "MAC" => Region::TW,
        // CN (China) — alpha-2
        "CN"
        // alpha-3
        | "CHN" => Region::CN,
        _ => Region::US,
    }
}

/// Country code to default locale. Accepts alpha-2 or alpha-3.
/// Maps individual countries to their preferred locale within their region.
pub fn country_to_locale(code: &str) -> &str {
    match code {
        // US region — per-country preferences
        "US" | "USA" => "enUS",
        "MX" | "MEX" => "esMX",
        "BR" | "BRA" => "ptBR",
        // EU region — per-country preferences
        "GB" | "GBR" => "enGB",
        "DE" | "DEU" => "deDE",
        "FR" | "FRA" => "frFR",
        "ES" | "ESP" => "esES",
        "IT" | "ITA" => "itIT",
        "RU" | "RUS" => "ruRU",
        "PL" | "POL" => "plPL",
        "PT" | "PRT" => "ptPT",
        // KR region — per-country preferences
        "KR" | "KOR" => "koKR",
        "JP" | "JPN" => "jaJP",
        "TH" | "THA" => "thTH",
        // TW/CN — per-country preferences
        "TW" | "TWN" | "HK" | "HKG" | "MO" | "MAC" => "zhTW",
        "CN" | "CHN" => "zhCN",
        // Fall back to region default for other countries
        other => country_to_region(other).default_locale(),
    }
}

/// Check whether a locale string is valid (supported by any region).
pub fn is_valid_locale(locale: &str) -> bool {
    [
        Region::US.locales(),
        Region::EU.locales(),
        Region::KR.locales(),
        Region::TW.locales(),
        Region::CN.locales(),
    ]
    .iter()
    .any(|locales| locales.contains(&locale))
}

/// All supported locales as a flat list.
pub fn all_locales() -> Vec<&'static str> {
    vec![
        "enUS", "esMX", "ptBR", "enGB", "deDE", "frFR", "esES", "itIT", "ruRU", "plPL", "ptPT",
        "koKR", "jaJP", "thTH", "zhTW", "zhCN",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_mapping() {
        assert_eq!(country_to_region("USA"), Region::US);
        assert_eq!(country_to_region("DEU"), Region::EU);
        assert_eq!(country_to_region("KOR"), Region::KR);
        assert_eq!(country_to_region("TWN"), Region::TW);
        assert_eq!(country_to_region("CHN"), Region::CN);
        assert_eq!(country_to_region("AUS"), Region::US); // Oceania → US
        assert_eq!(country_to_region("JPN"), Region::KR); // Asia → KR
        assert_eq!(country_to_region("HKG"), Region::TW);
    }

    #[test]
    fn default_locales() {
        assert_eq!(Region::US.default_locale(), "enUS");
        assert_eq!(Region::EU.default_locale(), "enGB");
        assert_eq!(Region::KR.default_locale(), "koKR");
        assert_eq!(Region::TW.default_locale(), "zhTW");
        assert_eq!(Region::CN.default_locale(), "zhCN");
    }

    #[test]
    fn country_to_locale_works() {
        // Per-country preferences
        assert_eq!(country_to_locale("USA"), "enUS");
        assert_eq!(country_to_locale("US"), "enUS");
        assert_eq!(country_to_locale("DEU"), "deDE");
        assert_eq!(country_to_locale("DE"), "deDE");
        assert_eq!(country_to_locale("GBR"), "enGB");
        assert_eq!(country_to_locale("KOR"), "koKR");
        assert_eq!(country_to_locale("JPN"), "jaJP");
        assert_eq!(country_to_locale("THA"), "thTH");
        assert_eq!(country_to_locale("HKG"), "zhTW");
        assert_eq!(country_to_locale("CHN"), "zhCN");
        // Unknown countries fall back to region default
        assert_eq!(country_to_locale("AUS"), "enUS"); // AU → US region → enUS
        assert_eq!(country_to_locale("NLD"), "enGB"); // NL → EU region → enGB
        assert_eq!(country_to_locale("XYZ"), "enUS"); // unknown → US → enUS
    }

    #[test]
    fn all_locales_are_valid() {
        for l in all_locales() {
            assert!(is_valid_locale(l), "{l} should be valid");
        }
        assert!(!is_valid_locale("xxXX"));
    }
}
