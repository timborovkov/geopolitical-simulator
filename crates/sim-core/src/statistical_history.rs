//! Bounded summaries of retired review observations. These are sample means,
//! not annual accounts, continuous time averages, or reconstructed daily data.
use crate::{CivilDate, Money, Observation, date};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatisticalArchive {
    pub years: Vec<AnnualReviewSummary>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnnualReviewSummary {
    pub year: u16,
    pub first_day: crate::CampaignDay,
    pub last_day: crate::CampaignDay,
    pub samples: u32,
    pub treasury_sum: Money,
    pub debt_sum: Money,
    pub approval_sum: i64,
    pub needs_sum: i64,
    pub daily_output_sum: Money,
}
fn year(day: crate::CampaignDay) -> Result<u16, String> {
    if day > crate::CampaignDay::new(crate::MAX_DAY) {
        return Err("Archived review exceeds the campaign calendar".into());
    }
    Ok(date(day).parse::<CivilDate>()?.year())
}
impl AnnualReviewSummary {
    /// Floor each nonnegative sample mean once. `day` is the last sampled
    /// review, not a claim that this was a point observation on that day.
    pub fn mean(&self) -> Option<Observation> {
        self.validate().ok()?;
        let count = i64::from(self.samples);
        Some(Observation {
            day: self.last_day,
            treasury: Money(self.treasury_sum.0 / count),
            debt: Money(self.debt_sum.0 / count),
            approval: (self.approval_sum / count) as i32,
            needs: (self.needs_sum / count) as i32,
            output: Money(self.daily_output_sum.0 / count),
        })
    }
    fn validate(&self) -> Result<(), String> {
        let count = i64::from(self.samples);
        if self.samples == 0
            || self.samples > 366
            || self.last_day > crate::CampaignDay::new(crate::MAX_DAY)
            || self.first_day > self.last_day
            || self
                .last_day
                .elapsed_since(self.first_day)
                .is_none_or(|elapsed| self.samples > elapsed.units() + 1)
            || year(self.first_day)? != self.year
            || year(self.last_day)? != self.year
            || [
                self.treasury_sum.0,
                self.debt_sum.0,
                self.daily_output_sum.0,
            ]
            .iter()
            .any(|value| !(0..=1_000_000_000_000 * count).contains(value))
            || !(0..=100 * count).contains(&self.approval_sum)
            || !(0..=100 * count).contains(&self.needs_sum)
        {
            return Err("Invalid archived review summary".into());
        }
        Ok(())
    }
}
impl StatisticalArchive {
    pub fn validate(
        &self,
        start: crate::CampaignDay,
        recent: &[Observation],
    ) -> Result<(), String> {
        let start = start.units();
        if self.years.len() > 101 || (!self.years.is_empty() && recent.len() != 120) {
            return Err("Archived reviews require the bounded recent review window".into());
        }
        let mut previous = None;
        for summary in &self.years {
            summary.validate()?;
            if summary.first_day < crate::CampaignDay::new(start)
                || previous
                    .is_some_and(|(year, last)| summary.year <= year || summary.first_day <= last)
                || recent
                    .first()
                    .is_none_or(|point| summary.last_day >= point.day)
            {
                return Err("Archived review chronology overlaps recent history".into());
            }
            previous = Some((summary.year, summary.last_day));
        }
        Ok(())
    }
    /// Move one retired observation into its calendar-year sample summary.
    /// Validate and allocate on a copy so refusal leaves the archive unchanged.
    pub fn retire(&mut self, point: &Observation) -> Result<(), String> {
        let point_year = year(point.day)?;
        if [point.treasury.0, point.debt.0, point.output.0]
            .iter()
            .any(|value| !(0..=1_000_000_000_000).contains(value))
            || !(0..=100).contains(&point.approval)
            || !(0..=100).contains(&point.needs)
            || self
                .years
                .last()
                .is_some_and(|summary| point.day <= summary.last_day)
        {
            return Err("Invalid or unordered retired review observation".into());
        }
        let mut summary = match self
            .years
            .last()
            .filter(|summary| summary.year == point_year)
        {
            Some(summary) => summary.clone(),
            None => AnnualReviewSummary {
                year: point_year,
                first_day: point.day,
                last_day: point.day,
                samples: 0,
                treasury_sum: Money(0),
                debt_sum: Money(0),
                approval_sum: 0,
                needs_sum: 0,
                daily_output_sum: Money(0),
            },
        };
        summary.samples = summary
            .samples
            .checked_add(1)
            .ok_or("Review sample count exhausted")?;
        summary.last_day = point.day;
        let add = |left: i64, right: i64| {
            left.checked_add(right)
                .ok_or("Archived review sum overflow")
        };
        summary.treasury_sum = Money(add(summary.treasury_sum.0, point.treasury.0)?);
        summary.debt_sum = Money(add(summary.debt_sum.0, point.debt.0)?);
        summary.daily_output_sum = Money(add(summary.daily_output_sum.0, point.output.0)?);
        summary.approval_sum = add(summary.approval_sum, i64::from(point.approval))?;
        summary.needs_sum = add(summary.needs_sum, i64::from(point.needs))?;
        summary.validate()?;
        if self
            .years
            .last()
            .is_some_and(|last| last.year == point_year)
        {
            *self.years.last_mut().expect("same year") = summary;
        } else {
            if self.years.len() >= 101 {
                return Err("Archived calendar-year bound exceeded".into());
            }
            self.years.push(summary);
        }
        Ok(())
    }
}

/// A released sample summary. Treasury and the private sums are excluded.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedAnnualReview {
    pub year: u16,
    pub first_day: crate::CampaignDay,
    pub last_day: crate::CampaignDay,
    pub samples: u32,
    pub mean: crate::PublishedObservation,
}
impl StatisticalArchive {
    pub fn published(&self) -> Vec<PublishedAnnualReview> {
        self.years
            .iter()
            .filter_map(|summary| {
                let mean = summary.mean()?;
                Some(PublishedAnnualReview {
                    year: summary.year,
                    first_day: summary.first_day,
                    last_day: summary.last_day,
                    samples: summary.samples,
                    mean: crate::PublishedObservation {
                        day: mean.day,
                        debt: mean.debt,
                        approval: mean.approval,
                        needs: mean.needs,
                        daily_output: mean.output,
                    },
                })
            })
            .collect()
    }
}
