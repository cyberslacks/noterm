// Freshness/staleness logic ported from Kazam.
// A note is stale when its `modified` date + `review_every` cadence < today.
// Date math is hand-rolled (JDN algorithm) — no chrono dep for this path.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Kazam's current warning window: a note is due soon during the seven days
/// before its review deadline, regardless of cadence length.
pub const DUE_SOON_DAYS: i64 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessStatus {
    Fresh,
    DueSoon { days_until_due: i64 },
    Overdue { days_overdue: i64 },
    Expired { days_past_expiry: i64 },
}

pub struct FreshnessInfo {
    pub updated_days: Option<i64>,
    pub review_days: Option<i64>,
    pub expires_days: Option<i64>,
    pub today_days: i64,
}

impl FreshnessInfo {
    pub fn days_since_update(&self) -> Option<i64> {
        self.updated_days.map(|u| self.today_days - u)
    }

    pub fn status(&self) -> FreshnessStatus {
        if let Some(exp) = self.expires_days {
            if self.today_days > exp {
                return FreshnessStatus::Expired {
                    days_past_expiry: self.today_days - exp,
                };
            }
        }
        let (elapsed, cadence) = match (self.days_since_update(), self.review_days) {
            (Some(e), Some(c)) => (e, c),
            _ => return FreshnessStatus::Fresh,
        };
        let days_until_due = cadence - elapsed;
        if days_until_due < 0 {
            FreshnessStatus::Overdue {
                days_overdue: -days_until_due,
            }
        } else if days_until_due <= DUE_SOON_DAYS {
            FreshnessStatus::DueSoon { days_until_due }
        } else {
            FreshnessStatus::Fresh
        }
    }
}

/// Compute freshness info from note frontmatter fields.
/// Returns `None` when `review_every` is absent — no cadence means no staleness check.
/// `updated` accepts both ISO `YYYY-MM-DD` and RFC3339 timestamps (truncated to date).
pub fn compute(
    updated: Option<&str>,
    review_every: Option<&str>,
    expires: Option<&str>,
) -> Option<FreshnessInfo> {
    review_every?;
    let today = today_iso();
    let today_days = parse_iso_date(&today)?;
    let updated_days = updated.and_then(|s| parse_iso_date(truncate_to_date(s)));
    let review_days = review_every.and_then(parse_duration_days);
    let expires_days = expires.and_then(|s| parse_iso_date(truncate_to_date(s)));
    Some(FreshnessInfo {
        updated_days,
        review_days,
        expires_days,
        today_days,
    })
}

/// A note with freshness metadata, computed for the freshness view panel.
#[derive(Debug, Clone)]
pub struct FreshnessEntry {
    pub path: PathBuf,
    pub relative_path: String,
    pub title: String,
    pub status: FreshnessStatus,
    pub owner: Option<String>,
    pub review_every: Option<String>,
}

/// Scan a list of note paths and return freshness entries for notes that have
/// `review_every` set, sorted worst-first (Expired → Overdue → DueSoon → Fresh).
pub fn scan_paths(paths: &[PathBuf], notes_dir: &Path) -> Vec<FreshnessEntry> {
    let mut entries = Vec::new();

    for path in paths {
        let raw = match std::fs::read_to_string(path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let (fm, _body) = crate::notes::frontmatter::parse(&raw);

        // Only include notes that have review_every configured
        if fm.review_every.is_none() {
            continue;
        }

        let info = compute(
            fm.modified.as_deref(),
            fm.review_every.as_deref(),
            fm.expires.as_deref(),
        );
        let status = info.map(|i| i.status()).unwrap_or(FreshnessStatus::Fresh);

        let relative_path = path
            .strip_prefix(notes_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let title = fm.title.clone().unwrap_or_else(|| relative_path.clone());

        entries.push(FreshnessEntry {
            path: path.clone(),
            relative_path,
            title,
            status,
            owner: fm.owner.clone(),
            review_every: fm.review_every.clone(),
        });
    }

    // Sort: Expired first, then Overdue (most overdue first), then DueSoon, then Fresh
    entries.sort_by(|a, b| {
        fn rank(s: FreshnessStatus) -> (u8, i64) {
            match s {
                FreshnessStatus::Expired { days_past_expiry } => (0, -days_past_expiry),
                FreshnessStatus::Overdue { days_overdue } => (1, -days_overdue),
                FreshnessStatus::DueSoon { days_until_due } => (2, days_until_due),
                FreshnessStatus::Fresh => (3, 0),
            }
        }
        rank(a.status).cmp(&rank(b.status))
    });

    entries
}

/// Today's date as `YYYY-MM-DD` from the system clock.
pub fn today_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    iso_from_days_since_epoch(days)
}

/// Truncate RFC3339 (`2026-05-10T12:00:00Z`) to just the date part.
fn truncate_to_date(s: &str) -> &str {
    if s.len() > 10 && s.as_bytes().get(10) == Some(&b'T') {
        &s[..10]
    } else {
        s
    }
}

/// Parse `YYYY-MM-DD` → days since 1970-01-01. Returns `None` on bad input.
pub fn parse_iso_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let mut parts = s.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_since_epoch(y, m, d))
}

/// Gregorian (y, m, d) → days since 1970-01-01 via JDN formula.
fn days_since_epoch(y: i32, m: u32, d: u32) -> i64 {
    let a = (14 - m as i32) / 12;
    let y = y + 4800 - a;
    let m_adj = m as i32 + 12 * a - 3;
    let jdn = d as i32 + (153 * m_adj + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;
    (jdn - 2440588) as i64
}

/// Days since 1970-01-01 → ISO `YYYY-MM-DD`.
fn iso_from_days_since_epoch(days: i64) -> String {
    let jdn = days + 2440588;
    let a = jdn + 32044;
    let b = (4 * a + 3) / 146_097;
    let c = a - (146_097 * b) / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m_ = (5 * e + 2) / 153;
    let day = e - (153 * m_ + 2) / 5 + 1;
    let month = m_ + 3 - 12 * (m_ / 10);
    let year = 100 * b + d - 4800 + m_ / 10;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Parse duration strings: `7d`, `2w`, `3m`, `1y`, `weekly`, `monthly`, etc.
pub fn parse_duration_days(s: &str) -> Option<i64> {
    let s = s.trim();
    match s.to_ascii_lowercase().as_str() {
        "weekly" => return Some(7),
        "monthly" => return Some(30),
        "quarterly" => return Some(90),
        "yearly" | "annually" => return Some(365),
        _ => {}
    }
    let (num, unit) = s.split_at(s.len().saturating_sub(1));
    let n: i64 = num.trim().parse().ok()?;
    let mult = match unit {
        "d" | "D" => 1,
        "w" | "W" => 7,
        "m" | "M" => 30,
        "y" | "Y" => 365,
        _ => return None,
    };
    Some(n * mult)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_parsing() {
        assert_eq!(parse_duration_days("7d"), Some(7));
        assert_eq!(parse_duration_days("2w"), Some(14));
        assert_eq!(parse_duration_days("3m"), Some(90));
        assert_eq!(parse_duration_days("1y"), Some(365));
        assert_eq!(parse_duration_days("monthly"), Some(30));
        assert_eq!(parse_duration_days("weekly"), Some(7));
        assert_eq!(parse_duration_days("quarterly"), Some(90));
        assert_eq!(parse_duration_days("bogus"), None);
    }

    #[test]
    fn overdue_detection() {
        // Updated 110 days ago, cadence 90d → should be Overdue
        let info = FreshnessInfo {
            updated_days: Some(0),
            review_days: Some(90),
            expires_days: None,
            today_days: 110,
        };
        assert!(matches!(info.status(), FreshnessStatus::Overdue { .. }));
    }

    #[test]
    fn fresh_within_window() {
        let info = FreshnessInfo {
            updated_days: Some(0),
            review_days: Some(90),
            expires_days: None,
            today_days: 20,
        };
        assert!(matches!(info.status(), FreshnessStatus::Fresh));
    }

    #[test]
    fn expired_beats_overdue() {
        let info = FreshnessInfo {
            updated_days: Some(0),
            review_days: Some(30),
            expires_days: Some(50),
            today_days: 100,
        };
        assert!(matches!(info.status(), FreshnessStatus::Expired { .. }));
    }
}
