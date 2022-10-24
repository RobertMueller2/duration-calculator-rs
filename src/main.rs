use std::env;
use std::io::{self, BufRead};
use std::str::FromStr;

use chrono::Duration;
use debug_print::debug_println;
use lazy_static::lazy_static;
use regex::Regex;

fn main() {
    let args: Vec<String> = env::args().collect();
    let arg_str = args[1..].join(" ");

    let mut d = Duration::zero();
    let mut printed :bool = false;

    // read stdin only if there is a redirect
    if atty::isnt(atty::Stream::Stdin) {
        for line in io::stdin().lock().lines() {
            let ls = line.unwrap_or_else(|_| panic!("IO error reading stdin"));
            let d_line =
                Duration::from_str(&ls).unwrap_or_else(|| panic!("cannot parse {:?}", &ls));
            d = d.saturated_add(&d_line);
        }

        printed = true;
        d.print();
        println!();
    }

    let d_args = Duration::from_str(&arg_str)
        .unwrap_or_else(|| panic!("cannot parse {:?} from arguments as duration", &arg_str));

    // don't print 0 if there is already a result from stdin
    if d_args != Duration::zero() || !printed {
        d = d.saturated_add(&d_args);
        d.print();
    }
}

trait DurationPrint {
    fn print(self);
}

impl DurationPrint for Duration {
    fn print(mut self) {
        if self < Duration::zero() {
            print!("-");
            self = self * -1;
        }

        let n = self.num_seconds();

        print!("{}h", n / 3600);
        print!("{}m", (n % 3600) / 60);
        print!("{}s", n % 60);
    }
}

trait DurationCalculate {
    fn saturated_add(&self, rhs: &Self) -> Self;
    fn saturated_sub(&self, rhs: &Self) -> Self;
}

impl DurationCalculate for Duration {
    fn saturated_add(&self, rhs: &Duration) -> Duration {
        self.checked_add(rhs).unwrap_or(Duration::max_value())
    }

    fn saturated_sub(&self, rhs: &Duration) -> Duration {
        self.checked_sub(rhs).unwrap_or(Duration::min_value())
    }
}

trait DurationParse {
    fn from_str(input: &str) -> Option<Duration>;
    fn token_to_duration(count: i64, unit: &str) -> Option<Duration>;
}

impl DurationParse for Duration {
    fn from_str(input: &str) -> Option<Duration> {
        lazy_static! {
            static ref LINE_PATTERN: Regex =
                Regex::new(r#"^(?:\s*[+-]\s*(?:\d+\s*(?:y|d|h|m|s)\s*)+)+$"#).unwrap();
            static ref DURATION_COMPOSITE_PATTERN: Regex =
                Regex::new(r#"(?P<sign>[+-])\s*(?P<duration>(?:(?:\d+)(?:y|d|h|m|s))+)"#)
                    .unwrap();
            static ref DURATION_PATTERN: Regex =
                Regex::new(r#"(?P<count>\d+)(?P<unit>y|d|h|m|min|s)"#).unwrap();
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
