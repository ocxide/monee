use std::{ops::AddAssign, str::FromStr};

use chrono::{Datelike, Timelike};

#[derive(Debug, Clone)]
pub enum PaymentPromise {
    Datetime(twon::Datetime),
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
    Datetime(<twon::Datetime as FromStr>::Err),
}

impl From<chrono::ParseError> for Error {
    fn from(v: <twon::Datetime as FromStr>::Err) -> Self {
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
    pub fn add(self, target: &mut twon::Datetime) {
        let mode = self.2;

        #[derive(Debug)]
        struct NaiveDuration(chrono::Months, chrono::Days, chrono::Duration);

        const N_PARTS: usize = 6;
        let set_max: [fn(&mut twon::Datetime); N_PARTS] = [
            |_| {},
            |naive| {
                *naive = naive.with_month(12).expect("To set months");
            },
            |naive| {
                const FEBRUARY: u32 = 2;
                let max_day = match naive.month() {
                    FEBRUARY => match naive.year() % 4 == 0 {
                        true => 29,
                        false => 28,
                    },
                    4 | 6 | 9 | 11 => 30,
                    _ => 31,
                };

                *naive = naive.with_day(max_day).expect("To set days");
            },
            |naive| {
                *naive = naive.with_hour(23).expect("To set hours");
            },
            |naive| {
                *naive = naive.with_minute(59).expect("To set minutes");
            },
            |naive| {
                *naive = naive.with_second(59).expect("To set seconds");
            },
        ];

        let (naive_duration, to_set_max) = match mode {
            DurationDeltaMode::EndOfDate => {
                let mut naive_duration = NaiveDuration(
                    chrono::Months::new(0),
                    chrono::Days::new(0),
                    chrono::Duration::default(),
                );

                struct PartDescicion {
                    pub value: Option<u32>,
                    pub add: fn(&mut NaiveDuration, u32),
                }

                let parts: [PartDescicion; N_PARTS] = [
                    PartDescicion {
                        value: self.1.years,
                        add: |naive, v| {
                            naive.0 = chrono::Months::new(12 * v + naive.0.as_u32());
                        },
                    },
                    PartDescicion {
                        value: self.1.months,
                        add: |naive, v| {
                            naive.0 = chrono::Months::new(v + naive.0.as_u32());
                        },
                    },
                    PartDescicion {
                        value: self.1.days,
                        add: |naive, v| {
                            naive.1 = chrono::Days::new(v as u64);
                        },
                    },
                    PartDescicion {
                        value: self.1.hours,
                        add: |naive, v| naive.2.add_assign(chrono::Duration::hours(v as i64)),
                    },
                    PartDescicion {
                        value: self.1.minutes,
                        add: |naive, v| naive.2.add_assign(chrono::Duration::minutes(v as i64)),
                    },
                    PartDescicion {
                        value: self.1.seconds,
                        add: |naive, v| naive.2.add_assign(chrono::Duration::seconds(v as i64)),
                    },
                ];

                let part = parts
                    .iter()
                    .enumerate()
                    .filter(|(_, part)| part.value.is_some())
                    .last()
                    .expect("To contain at least one part with value");

                for part in &parts[..part.0 + 1] {
                    (part.add)(&mut naive_duration, part.value.unwrap_or(0));
                }

                (naive_duration, &set_max[part.0 + 1..])
            }
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

                (
                    NaiveDuration(months, days, duration),
                    #[allow(clippy::out_of_bounds_indexing)]
                    &set_max[N_PARTS + 1..],
                )
            }
        };

        let NaiveDuration(months, days, duration) = naive_duration;
        match self.0 {
            Sign::Plus => {
                *target = target.checked_add_months(months).expect("To add months");
                *target = target.checked_add_days(days).expect("To add days");
                *target += duration;
            }
            Sign::Minus => {
                *target = target.checked_sub_months(months).expect("To sub months");
                *target = target.checked_sub_days(days).expect("To sub days");
                *target -= duration;
            }
        };

        for set_max in to_set_max {
            (set_max)(target);
        }
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

        let datetime = twon::Datetime::from_str(s)?;
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
        let mut date = twon::Datetime::from_str("2020-04-10T13:50:00Z").unwrap();
        let s = "1d eod";

        let payment: PaymentPromise = s.parse().unwrap();
        match payment {
            PaymentPromise::Datetime(_) => unreachable!(),
            PaymentPromise::Delta(delta) => delta.add(&mut date),
        }

        assert_eq!(
            date,
            twon::Datetime::from_str("2020-04-11T23:59:59Z").unwrap()
        );
    }

    #[test]
    fn adds_untils_this() {
        let mut date = twon::Datetime::from_str("2020-04-10T13:50:00Z").unwrap();
        let s = "0y eod";

        let payment: PaymentPromise = s.parse().unwrap();
        match payment {
            PaymentPromise::Datetime(_) => unreachable!(),
            PaymentPromise::Delta(delta) => delta.add(&mut date),
        }

        assert_eq!(
            date,
            twon::Datetime::from_str("2020-12-31T23:59:59Z").unwrap()
        );
    }

    #[test]
    fn subs_until_eod() {
        let mut date = twon::Datetime::from_str("2020-04-10T13:50:00Z").unwrap();
        let s = "-1y eod";

        let payment: PaymentPromise = s.parse().unwrap();
        match payment {
            PaymentPromise::Datetime(_) => unreachable!(),
            PaymentPromise::Delta(delta) => delta.add(&mut date),
        }

        assert_eq!(
            date,
            twon::Datetime::from_str("2019-12-31T23:59:59Z").unwrap()
        );
    }
}
