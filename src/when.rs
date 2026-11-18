//! Dates and times, all in UTC: turning a count of nanoseconds since 1970
//! into a calendar date and back, the date forms `touch` accepts, and the
//! formats `ls -l` and `date` print.

/// A moment as a calendar date and a time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Civil {
    pub year: i64,
    /// 1 to 12.
    pub month: u32,
    /// 1 to 31.
    pub day: u32,
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub nanos: u32,
    /// 0 is Sunday.
    pub weekday: u32,
    /// Day of the year, from 1.
    pub yday: u32,
}

pub const SECOND: i64 = 1_000_000_000;

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

pub fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 31,
    }
}

/// Seconds since 1970 of a calendar date and time.
pub fn seconds(y: i64, m: u32, d: u32, h: u32, mi: u32, s: u32) -> i64 {
    days_from_civil(y, m, d) * 86400 + h as i64 * 3600 + mi as i64 * 60 + s as i64
}

pub fn civil(ns: i64) -> Civil {
    let secs = ns.div_euclid(SECOND);
    let nanos = ns.rem_euclid(SECOND) as u32;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400) as u32;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    let yday = (days - days_from_civil(year, 1, 1) + 1) as u32;
    Civil {
        year,
        month,
        day,
        hour: rem / 3600,
        min: rem % 3600 / 60,
        sec: rem % 60,
        nanos,
        weekday: (days + 4).rem_euclid(7) as u32,
        yday,
    }
}

pub const MONTHS: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
pub const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

fn digits(s: &[u8]) -> Option<i64> {
    if s.is_empty() || !s.iter().all(|c| c.is_ascii_digit()) || s.len() > 18 {
        return None;
    }
    Some(s.iter().fold(0i64, |n, c| n * 10 + (c - b'0') as i64))
}

/// A fraction of a second written as digits, in nanoseconds.
fn fraction(s: &[u8]) -> Option<i64> {
    if s.is_empty() || !s.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut n = 0i64;
    for i in 0..9 {
        n = n * 10 + s.get(i).map_or(0, |c| (c - b'0') as i64);
    }
    Some(n)
}

fn valid(y: i64, m: i64, d: i64, h: i64, mi: i64, s: i64) -> Option<i64> {
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m as u32) as i64 || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    Some(seconds(y, m as u32, d as u32, h as u32, mi as u32, 0) + s)
}

/// The dates `touch -d` and `date -d` take: `@SECONDS`, `YYYY-MM-DD`, and
/// `YYYY-MM-DD HH:MM[:SS[.frac]]` with a space or a `T` between. Returns
/// nanoseconds since 1970.
pub fn parse_date(text: &[u8]) -> Option<i64> {
    let text = std::str::from_utf8(text).ok()?.trim().as_bytes();
    if let Some(rest) = text.strip_prefix(b"@") {
        let (neg, rest) = match rest.strip_prefix(b"-") {
            Some(r) => (true, r),
            None => (false, rest),
        };
        let (whole, frac) = match rest.iter().position(|c| *c == b'.') {
            Some(i) => (&rest[..i], fraction(&rest[i + 1..])?),
            None => (rest, 0),
        };
        let ns = digits(whole)?.checked_mul(SECOND)? + frac;
        return Some(if neg { -ns } else { ns });
    }
    let (date, time) = match text.iter().position(|c| *c == b' ' || *c == b'T') {
        Some(i) => (&text[..i], Some(std::str::from_utf8(&text[i + 1..]).ok()?.trim_start().as_bytes())),
        None => (text, None),
    };
    let d: Vec<&[u8]> = date.split(|c| *c == b'-').collect();
    if d.len() != 3 {
        return None;
    }
    let (y, m, day) = (digits(d[0])?, digits(d[1])?, digits(d[2])?);
    let (mut h, mut mi, mut s, mut ns) = (0, 0, 0, 0);
    if let Some(time) = time {
        let (hms, frac) = match time.iter().position(|c| *c == b'.' || *c == b',') {
            Some(i) => (&time[..i], Some(&time[i + 1..])),
            None => (time, None),
        };
        let t: Vec<&[u8]> = hms.split(|c| *c == b':').collect();
        if t.len() < 2 || t.len() > 3 || frac.is_some() && t.len() != 3 {
            return None;
        }
        h = digits(t[0])?;
        mi = digits(t[1])?;
        if t.len() == 3 {
            s = digits(t[2])?;
        }
        if let Some(f) = frac {
            ns = fraction(f)?;
        }
    }
    Some(valid(y, m, day, h, mi, s)?.checked_mul(SECOND)? + ns)
}

/// The stamp `touch -t` takes: `[[CC]YY]MMDDhhmm[.ss]`. `this_year` is
/// used when the stamp names none.
pub fn parse_stamp(text: &[u8], this_year: i64) -> Option<i64> {
    let (main, secs) = match text.iter().position(|c| *c == b'.') {
        Some(i) => (&text[..i], Some(&text[i + 1..])),
        None => (text, None),
    };
    if !main.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let two = |i: usize| digits(&main[i..i + 2]);
    let (year, at) = match main.len() {
        8 => (this_year, 0),
        10 => {
            let yy = two(0)?;
            (if yy >= 69 { 1900 + yy } else { 2000 + yy }, 2)
        }
        12 => (digits(&main[..4])?, 4),
        _ => return None,
    };
    let s = match secs {
        Some(s) if s.len() == 2 => digits(s)?,
        Some(_) => return None,
        None => 0,
    };
    Some(valid(year, two(at)?, two(at + 2)?, two(at + 4)?, two(at + 6)?, s)? * SECOND)
}

/// The date column of `ls -l` in the C locale: `Jan  5 12:34` for a time
/// in the last six months, `Jan  5  2020` otherwise.
pub fn ls_date(ns: i64, now: i64) -> String {
    let c = civil(ns);
    let month = &MONTHS[c.month as usize - 1][..3];
    // Half a year of 365.2425 days.
    let recent = ns <= now && ns > now - 15_778_476 * SECOND;
    if recent {
        format!("{} {:>2} {:02}:{:02}", month, c.day, c.hour, c.min)
    } else {
        format!("{} {:>2} {:>5}", month, c.day, c.year)
    }
}

/// ISO week number and the year it belongs to.
fn iso_week(c: &Civil) -> (i64, u32) {
    let wd = (c.weekday + 6) % 7; // Monday is 0
    let week = (c.yday as i64 - wd as i64 + 9) / 7;
    if week < 1 {
        let prev = c.year - 1;
        let days = if is_leap(prev) { 366 } else { 365 };
        (prev, ((c.yday as i64 + days - wd as i64 + 9) / 7) as u32)
    } else if week == 53 {
        let days = if is_leap(c.year) { 366 } else { 365 };
        if c.yday as i64 - wd as i64 + 3 > days {
            (c.year + 1, 1)
        } else {
            (c.year, 53)
        }
    } else {
        (c.year, week as u32)
    }
}

/// Format a time the way `date +FORMAT` does, in the C locale and UTC.
/// Returns `Err` with the conversion that is not supported.
pub fn format(ns: i64, fmt: &[u8]) -> Result<Vec<u8>, String> {
    let c = civil(ns);
    let mut out = Vec::new();
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i] != b'%' || i + 1 == fmt.len() {
            out.push(fmt[i]);
            i += 1;
            continue;
        }
        i += 1;
        // Flags: `-` no padding, `_` pad with spaces, `0` pad with zeros,
        // `^` upper case.
        let (mut pad, mut upper) = (None, false);
        while i < fmt.len() && matches!(fmt[i], b'-' | b'_' | b'0' | b'^' | b'#' | b'+') {
            match fmt[i] {
                b'^' | b'#' => upper = true,
                b'+' => {}
                f => pad = Some(f),
            }
            i += 1;
        }
        let mut width = None;
        while i < fmt.len() && fmt[i].is_ascii_digit() {
            width = Some(width.unwrap_or(0) * 10 + (fmt[i] - b'0') as usize);
            i += 1;
        }
        if i == fmt.len() {
            return Err("an unfinished % conversion".into());
        }
        let mut conv = fmt[i];
        i += 1;
        // %:z and %::z
        let mut colons = 0;
        while conv == b':' && i < fmt.len() {
            colons += 1;
            conv = fmt[i];
            i += 1;
        }
        if colons > 0 && (conv != b'z' || colons > 2) {
            return Err("%:".into());
        }
        let num = |n: i64, w: usize, fill: u8| -> String {
            let w = width.unwrap_or(w);
            match pad.unwrap_or(fill) {
                b'-' => n.to_string(),
                b'_' => format!("{:>w$}", n, w = w),
                _ if fill == b'_' && pad.is_none() => format!("{:>w$}", n, w = w),
                _ => format!("{:0w$}", n, w = w),
            }
        };
        let hour12 = if c.hour % 12 == 0 { 12 } else { c.hour % 12 } as i64;
        let (iso_year, iso_wk) = iso_week(&c);
        let text: String = match conv {
            b'%' => "%".into(),
            b'n' => "\n".into(),
            b't' => "\t".into(),
            b'Y' => num(c.year, 1, b'0'),
            b'C' => num(c.year / 100, 2, b'0'),
            b'y' => num(c.year % 100, 2, b'0'),
            b'm' => num(c.month as i64, 2, b'0'),
            b'd' => num(c.day as i64, 2, b'0'),
            b'e' => num(c.day as i64, 2, b'_'),
            b'H' => num(c.hour as i64, 2, b'0'),
            b'I' => num(hour12, 2, b'0'),
            b'k' => num(c.hour as i64, 2, b'_'),
            b'l' => num(hour12, 2, b'_'),
            b'M' => num(c.min as i64, 2, b'0'),
            b'S' => num(c.sec as i64, 2, b'0'),
            b'N' => {
                let all = format!("{:09}", c.nanos);
                match width {
                    Some(w) if w < 9 => all[..w].to_string(),
                    _ => all,
                }
            }
            b's' => ns.div_euclid(SECOND).to_string(),
            b'j' => num(c.yday as i64, 3, b'0'),
            b'u' => (if c.weekday == 0 { 7 } else { c.weekday }).to_string(),
            b'w' => c.weekday.to_string(),
            b'a' => DAYS[c.weekday as usize][..3].into(),
            b'A' => DAYS[c.weekday as usize].into(),
            b'b' | b'h' => MONTHS[c.month as usize - 1][..3].into(),
            b'B' => MONTHS[c.month as usize - 1].into(),
            b'p' => if c.hour < 12 { "AM" } else { "PM" }.into(),
            b'P' => if c.hour < 12 { "am" } else { "pm" }.into(),
            b'Z' => "UTC".into(),
            b'z' => ["+0000", "+00:00", "+00:00:00"][colons].into(),
            b'q' => ((c.month - 1) / 3 + 1).to_string(),
            b'G' => num(iso_year, 1, b'0'),
            b'g' => num(iso_year % 100, 2, b'0'),
            b'V' => num(iso_wk as i64, 2, b'0'),
            b'U' => num((c.yday as i64 + 6 - c.weekday as i64) / 7, 2, b'0'),
            b'W' => num((c.yday as i64 + 6 - ((c.weekday as i64 + 6) % 7)) / 7, 2, b'0'),
            b'D' => format!("{:02}/{:02}/{:02}", c.month, c.day, c.year % 100),
            b'F' => format!("{}-{:02}-{:02}", c.year, c.month, c.day),
            b'T' => format!("{:02}:{:02}:{:02}", c.hour, c.min, c.sec),
            b'R' => format!("{:02}:{:02}", c.hour, c.min),
            b'r' => format!("{:02}:{:02}:{:02} {}", hour12, c.min, c.sec, if c.hour < 12 { "AM" } else { "PM" }),
            b'c' => format!(
                "{} {} {:>2} {:02}:{:02}:{:02} {}",
                &DAYS[c.weekday as usize][..3],
                &MONTHS[c.month as usize - 1][..3],
                c.day,
                c.hour,
                c.min,
                c.sec,
                c.year
            ),
            b'x' => format!("{:02}/{:02}/{:02}", c.month, c.day, c.year % 100),
            b'X' => format!("{:02}:{:02}:{:02}", c.hour, c.min, c.sec),
            other => return Err(format!("%{}", other as char)),
        };
        if upper {
            out.extend_from_slice(text.to_ascii_uppercase().as_bytes());
        } else {
            out.extend_from_slice(text.as_bytes());
        }
    }
    Ok(out)
}

/// A size as `ls -h` and `du -h` print it: powers of 1,024, rounded up,
/// with one decimal below ten.
pub fn human(n: u64) -> String {
    if n < 1024 {
        return n.to_string();
    }
    let mut div: u128 = 1024;
    for unit in ["K", "M", "G", "T", "P", "E"] {
        let tenths = (n as u128 * 10 + div - 1) / div;
        if tenths < 100 {
            return format!("{}.{}{}", tenths / 10, tenths % 10, unit);
        }
        let whole = (n as u128 + div - 1) / div;
        if whole < 1024 {
            return format!("{}{}", whole, unit);
        }
        div *= 1024;
    }
    n.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar() {
        let c = civil(1_700_000_000 * SECOND + 5);
        assert_eq!((c.year, c.month, c.day, c.hour, c.min, c.sec, c.nanos), (2023, 11, 14, 22, 13, 20, 5));
        assert_eq!((c.weekday, c.yday), (2, 318));
        assert_eq!(seconds(2023, 11, 14, 22, 13, 20), 1_700_000_000);
        assert_eq!(civil(0).weekday, 4);
        assert_eq!(civil(-SECOND).year, 1969);
        assert_eq!(civil(seconds(2024, 2, 29, 0, 0, 0) * SECOND).yday, 60);
    }

    #[test]
    fn dates() {
        assert_eq!(parse_date(b"@1700000000"), Some(1_700_000_000 * SECOND));
        assert_eq!(parse_date(b"@1.5"), Some(1_500_000_000));
        assert_eq!(parse_date(b"2023-11-14"), Some(seconds(2023, 11, 14, 0, 0, 0) * SECOND));
        assert_eq!(parse_date(b"2023-11-14 22:13"), Some(seconds(2023, 11, 14, 22, 13, 0) * SECOND));
        assert_eq!(parse_date(b"2023-11-14T22:13:20.25"), Some(1_700_000_000 * SECOND + 250_000_000));
        assert_eq!(parse_date(b"2023-02-30"), None);
        assert_eq!(parse_date(b"tomorrow"), None);
        assert_eq!(parse_stamp(b"202311142213.20", 1999), Some(1_700_000_000 * SECOND));
        assert_eq!(parse_stamp(b"11142213", 2023), Some(seconds(2023, 11, 14, 22, 13, 0) * SECOND));
        assert_eq!(parse_stamp(b"6901010000", 0), Some(seconds(1969, 1, 1, 0, 0, 0) * SECOND));
        assert_eq!(parse_stamp(b"1311", 2023), None);
    }

    #[test]
    fn formats() {
        let t = 1_700_000_000 * SECOND;
        assert_eq!(ls_date(t, t + SECOND), "Nov 14 22:13");
        assert_eq!(ls_date(t, t + 200 * 86400 * SECOND), "Nov 14  2023");
        assert_eq!(format(t, b"%Y-%m-%d %H:%M:%S %a %b %e %j %s").unwrap(), b"2023-11-14 22:13:20 Tue Nov 14 318 1700000000");
        assert_eq!(format(t, b"%F %T %G-W%V-%u").unwrap(), b"2023-11-14 22:13:20 2023-W46-2");
        assert_eq!(human(1023), "1023");
        assert_eq!(human(1024), "1.0K");
        assert_eq!(human(1025), "1.1K");
        assert_eq!(human(10240), "10K");
        assert_eq!(human(1048575), "1.0M");
    }
}
