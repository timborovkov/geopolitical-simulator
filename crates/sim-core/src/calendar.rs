//! Civil dates for source records; campaign ticks remain days since 2000-01-01.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CivilDate {
    year: u16,
    month: u8,
    day: u8,
}
fn month_days(year: u16, month: u8) -> Option<u8> {
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ]
    .get(usize::from(month.checked_sub(1)?))
    .copied()
}
impl CivilDate {
    pub fn year(self) -> u16 {
        self.year
    }
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, String> {
        if year == 0
            || year > 9999
            || day == 0
            || month_days(year, month).is_none_or(|limit| day > limit)
        {
            return Err("Invalid Gregorian civil date".into());
        }
        Ok(Self { year, month, day })
    }
    /// A February 29 anniversary falls on February 28 in a non-leap year.
    pub fn anniversary(self, years: u16) -> Result<Self, String> {
        let year = self
            .year
            .checked_add(years)
            .filter(|year| *year <= 9999)
            .ok_or("Civil date year overflow")?;
        Self::new(
            year,
            self.month,
            self.day
                .min(month_days(year, self.month).expect("validated month")),
        )
    }
    pub fn campaign_day(self) -> Result<u32, String> {
        if !(2000..=2100).contains(&self.year) {
            return Err("Date must fall within the 2000–2100 campaign range".into());
        }
        let before_year = |year: u32| {
            let y = year - 1;
            365 * y + y / 4 - y / 100 + y / 400
        };
        let elapsed = before_year(u32::from(self.year)) - before_year(2000);
        let months: u32 = (1..self.month)
            .map(|month| u32::from(month_days(self.year, month).expect("validated month")))
            .sum();
        let day = elapsed + months + u32::from(self.day) - 1;
        if day > crate::MAX_DAY {
            return Err("Date exceeds 2100-01-01".into());
        }
        Ok(day)
    }
}
impl std::str::FromStr for CivilDate {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, String> {
        if input.len() != 10
            || input.as_bytes()[4] != b'-'
            || input.as_bytes()[7] != b'-'
            || !input
                .bytes()
                .enumerate()
                .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        {
            return Err("Use YYYY-MM-DD".into());
        }
        Self::new(
            input[..4].parse().map_err(|_| "Invalid year")?,
            input[5..7].parse().map_err(|_| "Invalid month")?,
            input[8..].parse().map_err(|_| "Invalid day")?,
        )
    }
}
impl std::fmt::Display for CivilDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
impl Serialize for CivilDate {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for CivilDate {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
