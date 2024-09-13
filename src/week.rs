use anyhow::{bail, Result};
use chrono::{Datelike, IsoWeek, NaiveDate, TimeDelta, Weekday};
use core::fmt;

/// an ISO week
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Week {
    /// the weeks between the first week of 1970 until this week
    db_week: i64,
}

impl Week {
    /// Create a new week from its calendar week and year.
    pub fn new(week: u32, year: i32) -> Result<Self> {
        match NaiveDate::from_isoywd_opt(year, week, Weekday::Mon) {
            Some(date) => Ok(date.into()),
            None => bail!("invalid week or year"),
        }
    }

    /// Create a new week from its database representation.
    pub fn from_db(db_week: i64) -> Self {
        Week { db_week }
    }

    /// Retrieve the internal representation for this week.
    /// This representation counts the weeks from the first week of 1970
    pub fn db_week(&self) -> i64 {
        self.db_week
    }

    /// Turn into an IsoWeek.
    pub fn iso_week(&self) -> IsoWeek {
        // TODO: make const
        let epoch_week: NaiveDate = NaiveDate::from_isoywd_opt(1970, 1, Weekday::Mon).unwrap();

        let date = epoch_week + TimeDelta::weeks(self.db_week);
        date.iso_week()
    }
}

impl From<NaiveDate> for Week {
    fn from(week: NaiveDate) -> Self {
        // TODO: make const
        let epoch_week: NaiveDate = NaiveDate::from_isoywd_opt(1970, 1, Weekday::Mon).unwrap();

        Week {
            db_week: (week - epoch_week).num_weeks(),
        }
    }
}

impl fmt::Display for Week {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let week: IsoWeek = self.iso_week();
        write!(f, "{}/{}", week.week(), week.year())
    }
}
