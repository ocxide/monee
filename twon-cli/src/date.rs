use std::{ops::AddAssign, str::FromStr};

use chrono::{Datelike, NaiveDateTime, TimeZone, Timelike};

#[derive(Debug, Clone)]
pub enum PaymentPromise {
    Datetime(twon_persistence::Datetime),
    Delta(DurationDelta),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not recognize datetime or duration")]
    Unrecognizable,
    #[error("No time provided")]
    NoData,
    #[error("No time number provided")]
    NoNumber,
    #[error("Invalid part type")]
    InvalidPartType,
    #[error("Already parsed part")]
    AlreadyParsedPart,

    #[error("Data was found after mode expression")]
    DataPostMode,

    #[error(transparent)]
    Datetime(<twon_persistence::Datetime as FromStr>::Err),
}

impl From<chrono::ParseError> for Error {
    fn from(v: <twon_persistence::Datetime as FromStr>::Err) -> Self {
        Self::Datetime(v)
    }
}

#[derive(Debug, Clone)]
pub enum Sign {
    Plus,
    Minus,
}

enum DurationPartType {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

#[derive(Default, Debug, Clone, Copy)]
enum DurationDeltaMode {
    EndOfDate,
    #[default]
    Exact,
}

fn parse_duration_part(part: &str) -> Result<(u32, DurationPartType), Error> {
    let until = part
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit())
        .map(|(i, _)| i)
        .last()
        .ok_or(Error::NoNumber)?;

    let (n_str, part_type_key) = part.split_at(until + 1);

    let part_type = match part_type_key {
        "y" | "Y" => DurationPartType::Year,
        "M" => DurationPartType::Month,
        "d" | "D" => DurationPartType::Day,
        "h" | "H" => DurationPartType::Hour,
        "m" => DurationPartType::Minute,
        "s" | "S" => DurationPartType::Second,
        _ => return Err(Error::InvalidPartType),
    };

    let n = n_str.parse().map_err(|_| Error::NoNumber)?;

    Ok((n, part_type))
}

#[derive(Debug, Default, Clone)]
struct DurationParts {
    pub years: Option<u32>,
    pub months: Option<u32>,
    pub days: Option<u32>,
    pub hours: Option<u32>,
    pub minutes: Option<u32>,
    pub seconds: Option<u32>,
}

impl DurationParts {
    fn store(&mut self, part_type: DurationPartType, n: u32) -> bool {
        let field = match part_type {
            DurationPartType::Year => &mut self.years,
            DurationPartType::Month => &mut self.months,
            DurationPartType::Day => &mut self.days,
            DurationPartType::Hour => &mut self.hours,
            DurationPartType::Minute => &mut self.minutes,
            DurationPartType::Second => &mut self.seconds,
        };

        if field.is_none() {
            *field = Some(n);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct DurationDelta(Sign, DurationParts, DurationDeltaMode);

impl DurationDelta {
    pub fn add(self, target: &mut twon_persistence::Datetime) {
        let mode = self.2;

        match mode {
            DurationDeltaMode::EndOfDate => {}
            DurationDeltaMode::Exact => {
                let parts = self.1;
                let months =
                    chrono::Months::new(parts.years.unwrap_or(0) * 12 + parts.months.unwrap_or(0));
                let days = chrono::Days::new(parts.days.unwrap_or(0) as u64);

                let mut duration = chrono::Duration::default();
                if let Some(hours) = parts.hours {
                    duration += chrono::Duration::hours(hours as i64);
                }
                if let Some(minutes) = parts.minutes {
                    duration += chrono::Duration::minutes(minutes as i64);
                }
                if let Some(seconds) = parts.seconds {
                    duration += chrono::Duration::seconds(seconds as i64);
                }

                match self.0 {
                    Sign::Plus => {
                        target.checked_add_months(months).expect("To add months");
                        target.checked_add_days(days).expect("To add days");
                        *target += duration;
                    }
                    Sign::Minus => {
                        target.checked_sub_months(months).expect("To sub months");
                        target.checked_sub_days(days).expect("To sub days");
                        *target -= duration;
                    }
                }

                return;
            }
        }

        struct PartDescicion {
            pub value: Option<u32>,
            pub set_max: fn(&mut NaiveDateTime, u32),
            pub add: fn(&mut NaiveDateTime, u32),
            pub max: u32,
        }

        let mut naive = target.naive_utc();

        let parts: [PartDescicion; 6] = [
            PartDescicion {
                value: self.1.seconds,
                set_max: |naive, v| {
                    *naive = naive.with_second(v).expect("To set seconds");
                },
                add: |naive, v| naive.add_assign(chrono::Duration::seconds(v as i64)),
                max: 59,
            },
            PartDescicion {
                value: self.1.minutes,
                set_max: |naive, v| {
                    *naive = naive.with_minute(v).expect("To set minutes");
                },
                add: |naive, v| naive.add_assign(chrono::Duration::minutes(v as i64)),
                max: 59,
            },
            PartDescicion {
                value: self.1.hours,
                set_max: |naive, v| {
                    *naive = naive.with_hour(v).expect("To set hours");
                },
                add: |naive, v| naive.add_assign(chrono::Duration::hours(v as i64)),
                max: 23,
            },
            PartDescicion {
                value: self.1.days,
                set_max: |naive, v| {
                    *naive = naive.with_day(v).expect("To set days");
                },
                add: |naive, v| naive.add_assign(chrono::Duration::days(v as i64)),
                max: 31,
            },
            PartDescicion {
                value: self.1.months,
                set_max: |naive, v| {
                    *naive = naive.with_month(v).expect("To set months");
                },
                add: |naive, v| {
                    *naive = naive
                        .checked_add_months(chrono::Months::new(v))
                        .expect("To add months");
                },
                max: 12,
            },
            PartDescicion {
                value: self.1.years,
                set_max: |naive, v| {
                    *naive = naive.with_year(v as i32).expect("To set years");
                },
                add: |naive, v| {
                    *naive = naive
                        .checked_add_months(chrono::Months::new(12 * v))
                        .expect("To add years");
                },
                max: i32::MAX as u32,
            },
        ];

        let mut parts = parts.into_iter();
        for part in parts.by_ref() {
            match part.value {
                None => (part.set_max)(&mut naive, part.max),
                Some(v) => {
                    (part.add)(&mut naive, v);
                    break;
                }
            }
        }

        for part in parts {
            (part.add)(&mut naive, part.value.unwrap_or(0));
        }

        *target = target.timezone().from_utc_datetime(&naive);
    }

    fn from_parts(sign: Sign, parts: DurationParts, mode: DurationDeltaMode) -> Self {
        Self(sign, parts, mode)
    }
}

fn parse_duration_delta(s: &str) -> Result<DurationDelta, Error> {
    let (sign, s) = match s.chars().next() {
        None => return Err(Error::NoData),
        Some('+') => (Sign::Plus, &s[1..]),
        Some('-') => (Sign::Minus, &s[1..]),
        _ => (Sign::Plus, s),
    };

    let mut duration_parts = DurationParts::default();

    let mut parts = s.split_whitespace();
    let first = parts.next().ok_or(Error::NoData)?;

    let (n, part_type) = parse_duration_part(first)?;
    if !duration_parts.store(part_type, n) {
        return Err(Error::AlreadyParsedPart);
    }

    let mut mode = None;

    for part in parts.by_ref() {
        match part {
            "eod" => {
                mode = Some(DurationDeltaMode::EndOfDate);
                break;
            }
            "exact" => {
                mode = Some(DurationDeltaMode::Exact);
                break;
            }
            _ => {
                let (n, part_type) = parse_duration_part(part)?;
                if !duration_parts.store(part_type, n) {
                    return Err(Error::AlreadyParsedPart);
                }
            }
        }
    }

    if mode.is_some_and(|_| parts.next().is_some()) {
        return Err(Error::DataPostMode);
    }

    Ok(DurationDelta::from_parts(
        sign,
        duration_parts,
        mode.unwrap_or_default(),
    ))
}

impl FromStr for PaymentPromise {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let is_duration = match s.as_bytes() {
            [b'+' | b'-', ..] => true,
            [b'0'..=b'9', b'0'..=b'9', b'0'..=b'9', b'0'..=b'9', b'-', ..] => false,
            [b'0'..=b'9', ..] => true,
            _ => return Err(Error::Unrecognizable),
        };

        if is_duration {
            let delta = parse_duration_delta(s)?;
            return Ok(PaymentPromise::Delta(delta));
        };

        let datetime = twon_persistence::Datetime::from_str(s)?;
        Ok(PaymentPromise::Datetime(datetime))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_duration() {
        let s = "2d 4h 5m 6s";
        let result = s.parse();
        dbg!(&result);
        assert!(matches!(result, Ok(PaymentPromise::Delta(_))));
    }

    #[test]
    fn detect_duration_with_sign() {
        let s = "-2d 4h 5m 6s";
        let result = s.parse();
        dbg!(&result);
        assert!(matches!(result, Ok(PaymentPromise::Delta(_))));
    }

    #[test]
    fn detects_datetime() {
        let s = "2020-01-01T00:00:00Z";
        let result = s.parse();
        dbg!(&result);
        assert!(matches!(result, Ok(PaymentPromise::Datetime(_))));
    }

    #[test]
    fn detects_eod() {
        let s = "1d eod";
        let result = s.parse();
        dbg!(&result);
        assert!(matches!(
            result,
            Ok(PaymentPromise::Delta(DurationDelta(
                _,
                _,
                DurationDeltaMode::EndOfDate
            )))
        ));
    }

    #[test]
    fn adds_until_eod() {
        let mut date = twon_persistence::Datetime::from_str("2020-04-10T13:50:00Z").unwrap();
        let s = "1d eod";

        let payment: PaymentPromise = s.parse().unwrap();
        match payment {
            PaymentPromise::Datetime(_) => unreachable!(),
            PaymentPromise::Delta(delta) => delta.add(&mut date),
        }

        assert_eq!(
            date,
            twon_persistence::Datetime::from_str("2020-04-11T23:59:59Z").unwrap()
        );
    }

    #[test]
    fn adds_untils_this() {
        let mut date = twon_persistence::Datetime::from_str("2020-04-10T13:50:00Z").unwrap();
        let s = "0y eod";

        let payment: PaymentPromise = s.parse().unwrap();
        match payment {
            PaymentPromise::Datetime(_) => unreachable!(),
            PaymentPromise::Delta(delta) => delta.add(&mut date),
        }

        assert_eq!(
            date,
            twon_persistence::Datetime::from_str("2021-12-31T23:59:59Z").unwrap()
        );
    }
}
