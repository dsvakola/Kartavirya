//! Natural-language quick capture: finds a time in text such as
//! "Call client at 4 pm", "Pay fees tomorrow", "Medicine every day at 8",
//! "Check oven in 20 min" or "Meeting on friday 10:30".
//! The text itself is never changed; only a schedule is suggested.

use crate::schedule::DayTimes;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Weekday};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Parsed {
    /// "custom", "hourly", "daily" or "weekly".
    pub remind_type: String,
    pub remind_at: String,
}

/// A time of day found in the text. `exact` is false for "at 4" (no am/pm).
#[derive(Debug, Clone, Copy)]
struct Tod {
    h: u32,
    m: u32,
    exact: bool,
}

#[derive(Debug, Clone, Copy)]
enum Day {
    Today,
    Tomorrow,
    AfterTomorrow,
    Weekday(Weekday),
    Date(u32, u32), // month, day
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Repeat {
    Hourly,
    Daily,
    Weekly,
}

fn clean(tok: &str) -> String {
    tok.trim_matches(|c: char| matches!(c, ',' | '!' | '?' | ';' | '(' | ')' | '"' | '\''))
        .trim_end_matches('.')
        .to_string()
}

fn weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" | "tues" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" | "thur" | "thurs" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    })
}

fn month(s: &str) -> Option<u32> {
    const M: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    if s.len() < 3 {
        return None;
    }
    let p = &s[..3];
    let idx = M.iter().position(|m| *m == p)?;
    // "may" must be the whole word; the others may be full names.
    let full = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let ok = s == p || full[idx] == s || (idx == 8 && s == "sept");
    ok.then_some(idx as u32 + 1)
}

fn day_number(s: &str) -> Option<u32> {
    let digits = s
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th");
    let d: u32 = digits.parse().ok()?;
    (1..=31).contains(&d).then_some(d)
}

fn number_word(s: &str) -> Option<i64> {
    Some(match s {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "ten" => 10,
        "fifteen" => 15,
        "twenty" => 20,
        "thirty" => 30,
        _ => s.parse().ok()?,
    })
}

/// Splits "20min" into (20, "min").
fn split_num_unit(s: &str) -> Option<(i64, &str)> {
    let pos = s.find(|c: char| !c.is_ascii_digit())?;
    if pos == 0 {
        return None;
    }
    Some((s[..pos].parse().ok()?, &s[pos..]))
}

fn unit_minutes(u: &str) -> Option<i64> {
    Some(match u {
        "m" | "min" | "mins" | "minute" | "minutes" => 1,
        "h" | "hr" | "hrs" | "hour" | "hours" => 60,
        "d" | "day" | "days" => 24 * 60,
        "week" | "weeks" => 7 * 24 * 60,
        _ => return None,
    })
}

/// Parses a clock time token, with an optional following am/pm token.
/// Returns the time and how many tokens were used.
fn clock(tok: &str, next: Option<&str>, after_at: bool) -> Option<(Tod, usize)> {
    match tok {
        "noon" | "midday" => {
            return Some((
                Tod {
                    h: 12,
                    m: 0,
                    exact: true,
                },
                1,
            ));
        }
        "midnight" => {
            return Some((
                Tod {
                    h: 23,
                    m: 59,
                    exact: true,
                },
                1,
            ));
        }
        _ => {}
    }
    let (body, mut mer) = if let Some(b) = tok.strip_suffix("am") {
        (b, Some(false))
    } else if let Some(b) = tok.strip_suffix("pm") {
        (b, Some(true))
    } else {
        (tok, None)
    };
    let mut used = 1;
    if mer.is_none() {
        match next {
            Some("am" | "a.m" | "a.m.") => {
                mer = Some(false);
                used = 2;
            }
            Some("pm" | "p.m" | "p.m.") => {
                mer = Some(true);
                used = 2;
            }
            _ => {}
        }
    }
    let (hs, ms) = match body.split_once([':', '.']) {
        Some((h, m)) => (h, Some(m)),
        None => (body, None),
    };
    if hs.is_empty() || hs.len() > 2 || !hs.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut h: u32 = hs.parse().ok()?;
    let m: u32 = match ms {
        Some(m) if m.len() == 2 && m.chars().all(|c| c.is_ascii_digit()) => m.parse().ok()?,
        Some(_) => return None,
        None => 0,
    };
    if m > 59 {
        return None;
    }
    // A bare number counts as a time only after "at"/"by" ("buy 2 apples" is not a time).
    if mer.is_none() && ms.is_none() && !after_at {
        return None;
    }
    let exact = match mer {
        Some(pm) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            if pm && h != 12 {
                h += 12;
            }
            if !pm && h == 12 {
                h = 0;
            }
            true
        }
        None => {
            if h > 23 {
                return None;
            }
            // "16:30" is exact; "at 4" / "4:30" could be morning or afternoon.
            h == 0 || h > 12
        }
    };
    Some((Tod { h, m, exact }, used))
}

fn local(date: NaiveDate, h: u32, m: u32) -> Option<DateTime<Local>> {
    let naive = date.and_hms_opt(h, m, 0)?;
    Local.from_local_datetime(&naive).earliest()
}

/// Picks the hour for an inexact time on `date`: the first of h and h+12
/// that is still ahead of `now`; on other days, small hours mean afternoon.
fn resolve(date: NaiveDate, t: Tod, now: DateTime<Local>) -> Option<DateTime<Local>> {
    if t.exact || t.h == 12 {
        return local(date, t.h, t.m);
    }
    if date == now.date_naive() {
        let am = local(date, t.h, t.m)?;
        if am > now {
            return Some(am);
        }
        return local(date, t.h + 12, t.m);
    }
    let h = if t.h < 7 { t.h + 12 } else { t.h };
    local(date, h, t.m)
}

fn next_weekday(from: NaiveDate, wd: Weekday, include_today: bool) -> NaiveDate {
    let cur = from.weekday().num_days_from_monday() as i64;
    let want = wd.num_days_from_monday() as i64;
    let mut diff = (want - cur).rem_euclid(7);
    if diff == 0 && !include_today {
        diff = 7;
    }
    from + Duration::days(diff)
}

pub fn parse_quick(text: &str, now: DateTime<Local>, times: DayTimes) -> Option<Parsed> {
    let lower = text.to_lowercase();
    let toks: Vec<String> = lower.split_whitespace().map(clean).collect();
    let n = toks.len();

    let mut tod: Option<Tod> = None;
    let mut day: Option<Day> = None;
    let mut repeat: Option<Repeat> = None;
    let mut repeat_wd: Option<Weekday> = None;
    let mut offset_min: Option<i64> = None;
    let mut part: Option<(u32, u32)> = None; // morning / evening, etc.

    let mut i = 0;
    while i < n {
        let t = toks[i].as_str();
        let next = toks.get(i + 1).map(String::as_str);
        let prev = if i > 0 { toks[i - 1].as_str() } else { "" };

        // "in 20 min", "in 2h", "in half an hour"
        if t == "in"
            && let Some(nx) = next
        {
            if nx == "half" && toks.get(i + 2).is_some_and(|s| s == "an" || s == "a") {
                offset_min = Some(30);
                i += 4;
                continue;
            }
            if let Some((num, unit)) = split_num_unit(nx)
                && let Some(u) = unit_minutes(unit)
            {
                offset_min = Some(num * u);
                i += 2;
                continue;
            }
            if let Some(num) = number_word(nx)
                && let Some(u) = toks.get(i + 2).and_then(|s| unit_minutes(s))
            {
                offset_min = Some(num * u);
                i += 3;
                continue;
            }
        }

        match t {
            "hourly" => repeat = Some(Repeat::Hourly),
            "daily" | "everyday" => repeat = Some(Repeat::Daily),
            "weekly" => repeat = Some(Repeat::Weekly),
            "every" | "each" => {
                if let Some(nx) = next {
                    match nx {
                        "hour" => repeat = Some(Repeat::Hourly),
                        "day" | "morning" | "evening" | "night" => {
                            repeat = Some(Repeat::Daily);
                            if nx != "day" {
                                part = Some(if nx == "morning" {
                                    times.daily
                                } else {
                                    times.evening
                                });
                            }
                        }
                        "week" => repeat = Some(Repeat::Weekly),
                        _ => {
                            if let Some(wd) = weekday(nx) {
                                repeat = Some(Repeat::Weekly);
                                repeat_wd = Some(wd);
                            }
                        }
                    }
                    i += 2;
                    continue;
                }
            }
            "today" => day = Some(Day::Today),
            "tonight" => {
                day = Some(Day::Today);
                part = Some(times.evening);
            }
            "tomorrow" | "tmrw" | "tomorow" | "tommorow" => {
                if prev == "after" && toks.get(i.wrapping_sub(2)).is_some_and(|s| s == "day") {
                    day = Some(Day::AfterTomorrow);
                } else {
                    day = Some(Day::Tomorrow);
                }
            }
            "morning" if part.is_none() => part = Some(times.daily),
            "evening" | "night" if part.is_none() => part = Some(times.evening),
            "afternoon" if part.is_none() => part = Some((15, 0)),
            _ => {}
        }

        if let Some(wd) = weekday(t)
            && repeat_wd.is_none()
        {
            day = Some(Day::Weekday(wd));
        }

        // "25 dec", "25th december", "dec 25"
        if let Some(d) = day_number(t)
            && let Some(m) = next.and_then(month)
        {
            day = Some(Day::Date(m, d));
            i += 2;
            continue;
        }
        if let Some(m) = month(t)
            && t != "may"
            && let Some(d) = next.and_then(day_number)
        {
            day = Some(Day::Date(m, d));
            i += 2;
            continue;
        }

        let after_at = matches!(prev, "at" | "by" | "@" | "around");
        if let Some((found, used)) = clock(t, next, after_at) {
            tod = Some(found);
            i += used;
            continue;
        }
        i += 1;
    }

    let fmt = |kind: &str, at: DateTime<Local>| Parsed {
        remind_type: kind.into(),
        remind_at: at.to_rfc3339(),
    };

    if let Some(mins) = offset_min {
        if mins <= 0 || mins > 366 * 24 * 60 {
            return None;
        }
        return Some(fmt("custom", now + Duration::minutes(mins)));
    }

    let today = now.date_naive();
    let tod_or = |dflt: (u32, u32)| {
        tod.or(part.map(|(h, m)| Tod { h, m, exact: true }))
            .unwrap_or(Tod {
                h: dflt.0,
                m: dflt.1,
                exact: true,
            })
    };

    match repeat {
        Some(Repeat::Hourly) => return Some(fmt("hourly", now + Duration::hours(1))),
        Some(Repeat::Daily) => {
            let t = tod_or(times.daily);
            let mut at = resolve(today, t, now)?;
            if at <= now {
                at = resolve(today + Duration::days(1), t, now)?;
            }
            return Some(fmt("daily", at));
        }
        Some(Repeat::Weekly) => {
            let t = tod_or(times.daily);
            let wd = repeat_wd.unwrap_or_else(|| today.weekday());
            let mut date = next_weekday(today, wd, true);
            let mut at = resolve(date, t, now)?;
            if at <= now {
                date += Duration::days(7);
                at = resolve(date, t, now)?;
            }
            return Some(fmt("weekly", at));
        }
        None => {}
    }

    let at = match day {
        None => {
            let t = tod.or(part.map(|(h, m)| Tod { h, m, exact: true }))?;
            let at = resolve(today, t, now)?;
            if at > now {
                at
            } else {
                resolve(today + Duration::days(1), t, now)?
            }
        }
        Some(Day::Today) => {
            let t = tod.or(part.map(|(h, m)| Tod { h, m, exact: true }))?;
            let at = resolve(today, t, now)?;
            if at <= now {
                return None;
            }
            at
        }
        Some(Day::Tomorrow) => resolve(today + Duration::days(1), tod_or(times.daily), now)?,
        Some(Day::AfterTomorrow) => resolve(today + Duration::days(2), tod_or(times.daily), now)?,
        Some(Day::Weekday(wd)) => {
            let t = tod_or(times.daily);
            let mut date = next_weekday(today, wd, true);
            let mut at = resolve(date, t, now)?;
            if at <= now {
                date = next_weekday(today, wd, false);
                at = resolve(date, t, now)?;
            }
            at
        }
        Some(Day::Date(m, d)) => {
            let t = tod_or(times.daily);
            let mut year = now.year();
            let mut date = NaiveDate::from_ymd_opt(year, m, d)?;
            let mut at = resolve(date, t, now)?;
            if at <= now {
                year += 1;
                date = NaiveDate::from_ymd_opt(year, m, d)?;
                at = resolve(date, t, now)?;
            }
            at
        }
    };
    Some(fmt("custom", at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn at(s: &str) -> DateTime<Local> {
        let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        Local.from_local_datetime(&naive).single().unwrap()
    }

    // 2026-11-23 is a Monday.
    fn p(text: &str) -> Option<(String, DateTime<Local>)> {
        parse_quick(text, at("2026-11-23 10:00"), DayTimes::default()).map(|r| {
            (
                r.remind_type,
                DateTime::parse_from_rfc3339(&r.remind_at)
                    .unwrap()
                    .with_timezone(&Local),
            )
        })
    }

    fn custom(s: &str) -> Option<(String, DateTime<Local>)> {
        Some(("custom".into(), at(s)))
    }

    #[test]
    fn plain_text_has_no_schedule() {
        assert_eq!(p("Car keys in the top drawer"), None);
        assert_eq!(p("Buy 2 apples and 3 bananas"), None);
        assert_eq!(p("Spectacles on the bookshelf"), None);
    }

    #[test]
    fn clock_times() {
        assert_eq!(p("Call client at 4 PM"), custom("2026-11-23 16:00"));
        assert_eq!(p("Call client at 4pm"), custom("2026-11-23 16:00"));
        assert_eq!(p("Lunch at 1:30 pm"), custom("2026-11-23 13:30"));
        assert_eq!(p("Report by 16:45"), custom("2026-11-23 16:45"));
        // 9 AM already passed, so tomorrow.
        assert_eq!(p("Stand-up at 9am"), custom("2026-11-24 09:00"));
        // "at 4" without am/pm: next 4 o'clock still ahead today.
        assert_eq!(p("Tea at 4"), custom("2026-11-23 16:00"));
        assert_eq!(p("Seminar at 11"), custom("2026-11-23 11:00"));
        assert_eq!(p("Lunch at noon"), custom("2026-11-23 12:00"));
    }

    #[test]
    fn relative_offsets() {
        assert_eq!(p("Check oven in 20 min"), custom("2026-11-23 10:20"));
        assert_eq!(p("Check oven in 20mins"), custom("2026-11-23 10:20"));
        assert_eq!(p("Call back in 2 hours"), custom("2026-11-23 12:00"));
        assert_eq!(p("Call back in an hour"), custom("2026-11-23 11:00"));
        assert_eq!(p("Break in half an hour"), custom("2026-11-23 10:30"));
        assert_eq!(p("Follow up in 3 days"), custom("2026-11-26 10:00"));
    }

    #[test]
    fn days() {
        assert_eq!(p("Pay fees tomorrow"), custom("2026-11-24 09:00"));
        assert_eq!(p("Pay fees tomorrow at 5 pm"), custom("2026-11-24 17:00"));
        assert_eq!(p("Call tomorrow at 4"), custom("2026-11-24 16:00"));
        assert_eq!(p("Bank tomorrow evening"), custom("2026-11-24 20:00"));
        assert_eq!(p("Movie tonight"), custom("2026-11-23 20:00"));
        assert_eq!(p("Meeting on friday 10:30"), custom("2026-11-27 10:30"));
        assert_eq!(p("Gym on mon at 7am"), custom("2026-11-30 07:00"));
        assert_eq!(p("Pay day after tomorrow"), custom("2026-11-25 09:00"));
        assert_eq!(p("Birthday on 25 dec"), custom("2026-12-25 09:00"));
        assert_eq!(
            p("Birthday on december 25th at 8pm"),
            custom("2026-12-25 20:00")
        );
        assert_eq!(p("Renew on 2 jan"), custom("2027-01-02 09:00"));
        assert_eq!(p("Report today"), None);
    }

    #[test]
    fn repeats() {
        assert_eq!(
            p("Medicine every day at 8 pm"),
            Some(("daily".into(), at("2026-11-23 20:00")))
        );
        assert_eq!(
            p("Water plants daily"),
            Some(("daily".into(), at("2026-11-24 09:00")))
        );
        assert_eq!(
            p("Walk every morning"),
            Some(("daily".into(), at("2026-11-24 09:00")))
        );
        assert_eq!(
            p("Stretch every hour"),
            Some(("hourly".into(), at("2026-11-23 11:00")))
        );
        assert_eq!(
            p("Review every friday at 3pm"),
            Some(("weekly".into(), at("2026-11-27 15:00")))
        );
    }

    #[test]
    fn uses_settings_times() {
        let times = DayTimes {
            evening: (19, 0),
            daily: (7, 30),
        };
        let r = parse_quick("Pay fees tomorrow", at("2026-11-23 10:00"), times).unwrap();
        let t = DateTime::parse_from_rfc3339(&r.remind_at)
            .unwrap()
            .with_timezone(&Local);
        assert_eq!(t, at("2026-11-24 07:30"));
    }

    #[test]
    fn may_is_not_a_month_alone() {
        assert_eq!(p("I may forget the charger"), None);
    }
}
