//! The metrics parameter grammar (spec §5): period, basis, kind.
//! Pure logic — no database, no HTTP — so it lives in domain with unit tests.
//!
//! Grammar (case-insensitive, surrounding whitespace ignored):
//!   period = YYYY | YYYY-Qn (n = 1..4) | cumulative | ttm
//!   basis  = gross | net
//!   kind   = capital | consumable | all
//!
//! Calendar quarters, Q1 = Jan–Mar. `ttm` is the trailing 12 calendar months
//! from today; because "today" must be the DATABASE's today (one clock, the
//! same one v_order_facts dates come from), ttm carries no date range here —
//! the SQL applies `ordered_on >= CURRENT_DATE - interval '12 months'`.

use chrono::NaiveDate;

/// Error strings are user-facing 422 messages — plain language, no jargon.
const PERIOD_MSG: &str =
    "period must be a year (e.g. 2025), a quarter (e.g. 2025-Q2), 'cumulative', or 'ttm'";
const BASIS_MSG: &str = "basis must be 'gross' or 'net'";
const KIND_MSG: &str = "kind must be 'capital', 'consumable', or 'all'";

/// Years outside this window are treated as garbage, not as empty periods —
/// a typo like 20255 should be a 422, not a silent empty result.
const YEAR_MIN: i32 = 1970;
const YEAR_MAX: i32 = 2100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Quarter { year: i32, quarter: u32 },
    Year(i32),
    Cumulative,
    Ttm,
}

impl Period {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let s = raw.trim().to_ascii_lowercase();
        match s.as_str() {
            "cumulative" => return Ok(Period::Cumulative),
            "ttm" => return Ok(Period::Ttm),
            _ => {}
        }
        if let Some((year_part, q_part)) = s.split_once("-q") {
            let year = parse_year(year_part)?;
            let quarter: u32 = q_part.parse().map_err(|_| PERIOD_MSG.to_string())?;
            if !(1..=4).contains(&quarter) {
                return Err(PERIOD_MSG.to_string());
            }
            return Ok(Period::Quarter { year, quarter });
        }
        Ok(Period::Year(parse_year(&s)?))
    }

    /// Half-open [start, end) date range for quarter/year periods; None for
    /// cumulative (no bounds) and ttm (bounded in SQL by CURRENT_DATE).
    pub fn date_range(&self) -> Option<(NaiveDate, NaiveDate)> {
        match *self {
            Period::Quarter { year, quarter } => {
                let start_month = (quarter - 1) * 3 + 1;
                let start = NaiveDate::from_ymd_opt(year, start_month, 1)?;
                let end = if quarter == 4 {
                    NaiveDate::from_ymd_opt(year + 1, 1, 1)?
                } else {
                    NaiveDate::from_ymd_opt(year, start_month + 3, 1)?
                };
                Some((start, end))
            }
            Period::Year(year) => Some((
                NaiveDate::from_ymd_opt(year, 1, 1)?,
                NaiveDate::from_ymd_opt(year + 1, 1, 1)?,
            )),
            Period::Cumulative | Period::Ttm => None,
        }
    }

    /// Quota proration divisor (spec §5.1): quarter = annual quota / 4;
    /// year and ttm = the full annual quota; cumulative = None (attainment
    /// is undefined across multi-year history).
    pub fn quota_divisor(&self) -> Option<i64> {
        match self {
            Period::Quarter { .. } => Some(4),
            Period::Year(_) | Period::Ttm => Some(1),
            Period::Cumulative => None,
        }
    }

    /// True for the periods served by the materialized-rollup read path
    /// (whole calendar quarters); cumulative/ttm need day precision and read
    /// v_order_facts directly.
    pub fn uses_rollups(&self) -> bool {
        matches!(self, Period::Quarter { .. } | Period::Year(_))
    }
}

fn parse_year(s: &str) -> Result<i32, String> {
    if s.len() != 4 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PERIOD_MSG.to_string());
    }
    let year: i32 = s.parse().map_err(|_| PERIOD_MSG.to_string())?;
    if !(YEAR_MIN..=YEAR_MAX).contains(&year) {
        return Err(PERIOD_MSG.to_string());
    }
    Ok(year)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    Gross,
    Net,
}

impl Basis {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "gross" => Ok(Basis::Gross),
            "net" => Ok(Basis::Net),
            _ => Err(BASIS_MSG.to_string()),
        }
    }

    /// The bind-parameter value for `CASE WHEN $n = 'gross' ...` ranking
    /// expressions — queries stay static and sqlx-compile-checked.
    pub fn as_str(&self) -> &'static str {
        match self {
            Basis::Gross => "gross",
            Basis::Net => "net",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindFilter {
    Capital,
    Consumable,
    All,
}

impl KindFilter {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "capital" => Ok(KindFilter::Capital),
            "consumable" => Ok(KindFilter::Consumable),
            "all" => Ok(KindFilter::All),
            _ => Err(KIND_MSG.to_string()),
        }
    }

    /// Bind-parameter value for the null-folded filter
    /// `($n = 'all' OR kind::text = $n)`. Parts/service revenue appears only
    /// under 'all' (spec §5) — exactly what that predicate yields.
    pub fn as_str(&self) -> &'static str {
        match self {
            KindFilter::Capital => "capital",
            KindFilter::Consumable => "consumable",
            KindFilter::All => "all",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    #[test]
    fn parses_years_quarters_keywords() {
        assert_eq!(Period::parse("2025").unwrap(), Period::Year(2025));
        assert_eq!(
            Period::parse("2025-Q1").unwrap(),
            Period::Quarter {
                year: 2025,
                quarter: 1
            }
        );
        assert_eq!(
            Period::parse(" 2024-q4 ").unwrap(),
            Period::Quarter {
                year: 2024,
                quarter: 4
            }
        );
        assert_eq!(Period::parse("cumulative").unwrap(), Period::Cumulative);
        assert_eq!(Period::parse("TTM").unwrap(), Period::Ttm);
    }

    #[test]
    fn rejects_garbage_periods() {
        for bad in [
            "",
            "vibes",
            "2025-Q5",
            "2025-Q0",
            "2025-Qx",
            "25",
            "20255",
            "1899",
            "2200",
            "2025-",
            "-Q1",
            "2025Q1",
            "2025-Q1-Q2",
            "cumulativ",
        ] {
            let err = Period::parse(bad).expect_err(bad);
            assert!(err.contains("period must be"), "{bad} -> {err}");
        }
    }

    #[test]
    fn quarter_ranges_are_calendar_and_half_open() {
        assert_eq!(
            Period::parse("2025-Q1").unwrap().date_range(),
            Some((d(2025, 1, 1), d(2025, 4, 1)))
        );
        assert_eq!(
            Period::parse("2025-Q2").unwrap().date_range(),
            Some((d(2025, 4, 1), d(2025, 7, 1)))
        );
        assert_eq!(
            Period::parse("2025-Q3").unwrap().date_range(),
            Some((d(2025, 7, 1), d(2025, 10, 1)))
        );
        // Q4 crosses the year boundary.
        assert_eq!(
            Period::parse("2025-Q4").unwrap().date_range(),
            Some((d(2025, 10, 1), d(2026, 1, 1)))
        );
        assert_eq!(
            Period::parse("2024").unwrap().date_range(),
            Some((d(2024, 1, 1), d(2025, 1, 1)))
        );
        assert_eq!(Period::parse("cumulative").unwrap().date_range(), None);
        assert_eq!(Period::parse("ttm").unwrap().date_range(), None);
    }

    #[test]
    fn quota_divisors_match_spec() {
        assert_eq!(Period::parse("2025-Q2").unwrap().quota_divisor(), Some(4));
        assert_eq!(Period::parse("2025").unwrap().quota_divisor(), Some(1));
        assert_eq!(Period::parse("ttm").unwrap().quota_divisor(), Some(1));
        assert_eq!(Period::parse("cumulative").unwrap().quota_divisor(), None);
    }

    #[test]
    fn rollup_path_selection() {
        assert!(Period::parse("2025").unwrap().uses_rollups());
        assert!(Period::parse("2025-Q3").unwrap().uses_rollups());
        assert!(!Period::parse("cumulative").unwrap().uses_rollups());
        assert!(!Period::parse("ttm").unwrap().uses_rollups());
    }

    #[test]
    fn basis_and_kind_parse_and_reject() {
        assert_eq!(Basis::parse("gross").unwrap(), Basis::Gross);
        assert_eq!(Basis::parse(" NET ").unwrap(), Basis::Net);
        assert!(Basis::parse("vibes").unwrap_err().contains("basis must be"));
        assert_eq!(KindFilter::parse("capital").unwrap(), KindFilter::Capital);
        assert_eq!(
            KindFilter::parse("consumable").unwrap(),
            KindFilter::Consumable
        );
        assert_eq!(KindFilter::parse("All").unwrap(), KindFilter::All);
        assert!(KindFilter::parse("parts")
            .unwrap_err()
            .contains("kind must be"));
    }
}
