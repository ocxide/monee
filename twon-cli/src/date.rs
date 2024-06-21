use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum PaymentPromise {
    Datetime(twon_persistence::Datetime),
    Delta(DurationDelta),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No time provided")]
    NoData,
    #[error("No time number provided")]
    NoNumber,
    #[error("Invalid part type")]
    InvalidPartType,
    #[error("Already parsed part")]
    AlreadyParsedPart,

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

#[derive(Default)]
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
pub struct DurationDelta(Sign, chrono::Months, chrono::Days, chrono::Duration);

impl DurationDelta {
    pub fn add(self, target: &mut twon_persistence::Datetime) {
        match self.0 {
            Sign::Plus => {
                target.checked_add_months(self.1).expect("To add months");
                target.checked_add_days(self.2).expect("To add days");
                *target += self.3;
            }
            Sign::Minus => {
                target.checked_sub_months(self.1).expect("To sub months");
                target.checked_sub_days(self.2).expect("To sub days");
                *target -= self.3;
            }
        }
    }

    fn from_parts(sign: Sign, parts: DurationParts) -> Self {
        let months = chrono::Months::new(parts.years.unwrap_or(0) * 12 + parts.months.unwrap_or(0));
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

        Self(sign, months, days, duration)
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

    for part in parts {
        let (n, part_type) = parse_duration_part(part)?;
        if !duration_parts.store(part_type, n) {
            return Err(Error::AlreadyParsedPart);
        }
    }

    Ok(DurationDelta::from_parts(sign, duration_parts))
}

impl FromStr for PaymentPromise {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(delta) = parse_duration_delta(s) {
            return Ok(PaymentPromise::Delta(delta))
        };

        let datetime = twon_persistence::Datetime::from_str(s)?;
        Ok(PaymentPromise::Datetime(datetime))
    }
}
