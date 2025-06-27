/// This module provides a command-line tool for parsing and manipulating
/// duration strings. It supports input via command-line arguments as well
/// as standard input, and can perform arithmetic on durations.
///
/// # Examples
///
/// ```fish
/// $ ./duration-calculator-rs "3d 20h 10m 15s"
/// 92h 10m 15s
///
/// $ echo "2d 5h" | ./duration-calculator-rs
/// 53h 00m 00s
///
/// $ ./duration-calculator-rs "-1y 3h 40m"
/// -8763h 40m 00s
///
/// Process both stdin and arguments:
///  $  echo -e "24h\n24m" | ./duration-calculator-rs 25m
/// 24h 24m 00s
/// 24h 49m 00s
///
/// With options:
///  $ echo 1m | target/release/duration-calculator-rs --compact --total-prefix total --stdin-sum-prefix today - 2m
/// today 0h01m00s
/// total -0h01m00s
/// ```
use std::cmp::Ordering;
use std::env;
use std::fmt;
use std::io::{self, BufRead, IsTerminal};
use std::str::FromStr;

use chrono::Duration;
use debug_print::debug_println;
use lazy_static::lazy_static;
use regex::Regex;

fn main() {
    let exe = env::args().next().unwrap_or_default();
    let args: Vec<String> = env::args().skip(1).collect();
    let mut compact: bool = false;
    let mut compactset: bool = false;

    let mut stdin_total_prefix_open: bool = false;
    let mut stdin_total_prefix = String::new();
    let mut total_prefix_open: bool = false;
    let mut total_prefix = String::new();

    let mut args_duration = Vec::new();

    for a in args {
        match a.as_str() {
            "-c" | "--compact" if !compactset => {
                compact = true;
                compactset = true;
            }
            "-t" | "--total-prefix" if total_prefix.is_empty() => {
                total_prefix_open = true;
            }
            "-s" | "--stdin-sum-prefix" if stdin_total_prefix.is_empty() => {
                stdin_total_prefix_open = true;
            }
            "-c" | "--compact" | "-t" | "--total-prefix" | "-s" | "--stdin-sum-prefix" => {
                eprintln!("{a} provided more than once");
                eprintln!();
                print_usage_and_exit(&exe, 1);
            }
            _ if (total_prefix_open || stdin_total_prefix_open) && a.starts_with('-') => {
                eprintln!("ambiguous prefix {a}");
                eprintln!();
                print_usage_and_exit(&exe, 2);
            }
            _ if total_prefix_open => {
                total_prefix_open = false;
                total_prefix = a + " ";
            }
            _ if stdin_total_prefix_open => {
                stdin_total_prefix_open = false;
                stdin_total_prefix = a + " ";
            }
            _ => {
                args_duration.push(a);
            }
        };
    }

    let arg_str = args_duration.join(" ");

    if total_prefix_open {
        eprintln!("error parsing total summary prefix");
        eprintln!();
        print_usage_and_exit(&exe, 3);
    }

    if stdin_total_prefix_open {
        eprintln!("error parsing stdin_total summary prefix");
        eprintln!();
        print_usage_and_exit(&exe, 4);
    }

    let mut d = Duration::zero();
    let mut printed: bool = false;

    // read stdin only if there is a redirect
    if !io::stdin().is_terminal() {
        for line in io::stdin().lock().lines() {
            let ls = line.unwrap_or_else(|_| panic!("IO error reading stdin"));
            let d_line =
                Duration::from_str(&ls).unwrap_or_else(|| panic!("cannot parse {:?}", &ls));
            d = d.saturated_add(&d_line);
        }

        printed = true;
        println!("{}{}", stdin_total_prefix, DisplayableDuration(d, compact));
    }

    let d_from_args = Duration::from_str(&arg_str)
        .unwrap_or_else(|| panic!("cannot parse {:?} from arguments as duration", &arg_str));

    // don't print 0 if there is already a result from stdin
    if d_from_args != Duration::zero() || !printed {
        d = d.saturated_add(&d_from_args);
        println!("{}{}", total_prefix, DisplayableDuration(d, compact));
    }
}

fn print_usage_and_exit(exe: &str, errorlevel: i32) {
    print_usage(exe);
    std::process::exit(errorlevel);
}

fn print_usage(exe: &str) {
    println!("Usage:");
    println!();
    println!("{exe} [Options] [Duration String]");
    println!();
    println!("where Options:");
    println!("-c|--compact\tCompact output");
    println!("-t|--total-prefix <prefix>\tPrefix the end sum with <prefix>");
    println!("-s|--stdin-sum-prefix <prefix>\tPrefix the stdin sum with <prefix>");
}

pub struct DisplayableDuration(pub Duration, pub bool);

impl fmt::Display for DisplayableDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sgn = match Duration::zero().cmp(&self.0) {
            /* so, er, if 0 is greater than the duration, the sign is negative. I'm
            deleting this from working memory and hopefully never have to look again.
            */
            Ordering::Greater => -1,
            _ => 1,
        };

        /*  if the duration is negative, display sign prefixing the whole duration,
           but keep the portions positive. -2h-05m-20s looks odd, doesn't it?
        */
        let n = sgn * self.0.num_seconds();
        let hours = n / 3600;
        let minutes = (n % 3600) / 60;
        let seconds = n % 60;

        if self.1 {
            write!(
                f,
                "{}{}h{:02}m{:02}s",
                if sgn < 0 { "-" } else { "" },
                hours,
                minutes,
                seconds
            )
        } else {
            write!(
                f,
                "{}{}h {:02}m {:02}s",
                if sgn < 0 { "-" } else { "" },
                hours,
                minutes,
                seconds
            )
        }
    }
}

/// A trait for performing arithmetic operations on durations not already covered in the standard
trait DurationCalculate {
    /// Adds two durations and returns the result or maximum value for overflow
    fn saturated_add(&self, rhs: &Self) -> Self;

    /// Adds two durations and returns the result or minimum value for overflow
    fn saturated_sub(&self, rhs: &Self) -> Self;
}

impl DurationCalculate for Duration {
    fn saturated_add(&self, rhs: &Duration) -> Duration {
        self.checked_add(rhs).unwrap_or(Duration::MAX)
    }

    fn saturated_sub(&self, rhs: &Duration) -> Duration {
        self.checked_sub(rhs).unwrap_or(Duration::MIN)
    }
}

/// A trait for parsing duration strings.
trait DurationParse {
    /// Parses a "line" of a duration string and returns a `Duration` or `None` if the input is invalid.
    fn from_str(input: &str) -> Option<Duration>;

    /// Converts the smallest token (e.g. "5m", "4s") to a `Duration` object or `None` for invalid input.
    fn token_to_duration(count: i64, unit: &str) -> Option<Duration>;
}

impl DurationParse for Duration {
    fn from_str(input: &str) -> Option<Duration> {
        lazy_static! {
            static ref LINE_PATTERN: Regex =
                Regex::new(r"^(?:\s*[+-]\s*(?:\d+\s*(?:y|d|h|m|s)\s*)+)+$").unwrap();
            static ref DURATION_COMPOSITE_PATTERN: Regex =
                Regex::new(r"(?P<sign>[+-])\s*(?P<duration>\s*(?:\d+\s*(?:y|d|h|m|s)\s*)+)")
                    .unwrap();
            static ref DURATION_PATTERN: Regex =
                Regex::new(r"(?P<count>\d+)\s*(?P<unit>y|d|h|m|min|s)").unwrap();
        }

        let mut duration = Duration::zero();

        if input.is_empty() {
            return Some(duration);
        }

        // ugh...
        let line = match input.chars().next() {
            Some('+') | Some('-') => input.to_owned(),
            _ => "+".to_owned() + input,
        };

        let line = line.split('#').next().unwrap();

        if !LINE_PATTERN.is_match(line) {
            return None;
        }

        for caps in DURATION_COMPOSITE_PATTERN.captures_iter(line) {
            let operator_function = match &caps["sign"] {
                "+" => Duration::checked_add,
                "-" => Duration::checked_sub,
                _ => unreachable!(),
            };
            debug_println!("outer: {:?}", &caps);

            for inner_caps in DURATION_PATTERN.captures_iter(&caps["duration"]) {
                debug_println!("inner: {:?}", &inner_caps);
                let count = i64::from_str(&inner_caps["count"]).unwrap();
                duration = match Self::token_to_duration(count, &inner_caps["unit"]) {
                    Some(d) => match operator_function(&duration, &d) {
                        Some(dd) => dd,
                        None => d,
                    },
                    None => duration,
                };

                debug_println!(" {:#?} duration", duration);
            }
        }

        Some(duration)
    }

    fn token_to_duration(count: i64, unit: &str) -> Option<Duration> {
        match unit {
            "y" => Some(Duration::days(365 * count)),
            "d" => Some(Duration::days(count)),
            "h" => Some(Duration::hours(count)),
            "m" => Some(Duration::minutes(count)),
            "s" => Some(Duration::seconds(count)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_to_duration() {
        let cases = vec![
            (5, "y", Duration::days(365 * 5)),
            (2, "d", Duration::days(2)),
            (3, "h", Duration::hours(3)),
            (30, "m", Duration::minutes(30)),
            (10, "s", Duration::seconds(10)),
            (0, "y", Duration::zero()),
        ];

        for (count, unit, expected) in cases {
            let result = Duration::token_to_duration(count, unit);
            assert_eq!(result, Some(expected));
        }
    }

    #[test]
    fn test_from_str() {
        let cases = vec![
            ("", Duration::zero()),
            (
                "3d 20h 10m 15s",
                Duration::days(3)
                    + Duration::hours(20)
                    + Duration::minutes(10)
                    + Duration::seconds(15),
            ),
            ("+2d 5h", Duration::days(2) + Duration::hours(5)),
            (
                "-1y 3h + 40m",
                Duration::days(-365) - Duration::hours(3) + Duration::minutes(40),
            ),
            ("+3h-2m", Duration::hours(3) - Duration::minutes(2)),
            ("2d 5h # Comment", Duration::days(2) + Duration::hours(5)),
            ("-2d 5h # Comment", -Duration::days(2) - Duration::hours(5)),
        ];

        for (input, expected) in cases {
            let result = Duration::from_str(input).unwrap();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_saturated_add_and_sub() {
        let cases = vec![
            (
                Duration::days(5),
                Duration::days(3),
                Duration::days(8),
                Duration::days(2),
            ),
            (
                Duration::hours(5),
                Duration::hours(3),
                Duration::hours(8),
                Duration::hours(2),
            ),
            (
                Duration::minutes(30),
                Duration::minutes(20),
                Duration::minutes(50),
                Duration::minutes(10),
            ),
        ];

        for (a, b, expected_add, expected_sub) in cases {
            let result_add = a.saturated_add(&b);
            let result_sub = a.saturated_sub(&b);
            assert_eq!(result_add, expected_add);
            assert_eq!(result_sub, expected_sub);
        }
    }
}
