//! The command line. Without options, the macro plays with its saved options.

use std::fmt;

use relay_core::model::{PlaybackOptions, Repeat};
use relay_core::playback::{MAX_SPEED, MIN_SPEED};

pub const USAGE: &str = "\
Plays the Relay macro inside this program.

Options:
  --repeat N|forever   Play it N times, or until stopped (default: as saved)
  --speed X            Play X times faster, from 0.01 to 100 (default: as saved)
  --no-countdown       Start right away, without the 3-second countdown
  --quiet              Show no window; the exit code says how it ended
  --help               Show this help

Stop it with Esc or Ctrl+Alt+End.

Exit codes: 0 completed, 1 error, 2 bad options, 3 stopped,
4 kill switch, 5 a pixel check or Find image step timed out, 6 screen locked.";

/// How to play, once the options are applied to the saved ones.
#[derive(Debug, Clone, PartialEq)]
pub struct Cli {
    pub repeat: Repeat,
    pub speed: f32,
    pub countdown: bool,
    pub quiet: bool,
}

impl Cli {
    /// The macro's playback options with these applied.
    pub fn apply(&self, saved: &PlaybackOptions) -> PlaybackOptions {
        PlaybackOptions { repeat: self.repeat, speed: self.speed, ..saved.clone() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Run(Cli),
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArgError(pub String);

impl fmt::Display for ArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Parses the arguments (without the program's name). Options take their
/// value as the next argument or after `=`.
pub fn parse(args: impl IntoIterator<Item = String>, saved: &PlaybackOptions) -> Result<Parsed, ArgError> {
    let mut cli = Cli { repeat: saved.repeat, speed: saved.speed, countdown: true, quiet: false };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = |what: &str| {
            inline.clone().or_else(|| args.next()).ok_or_else(|| ArgError(format!("{what} needs a value")))
        };
        match name.as_str() {
            "--repeat" => cli.repeat = repeat(&value("--repeat")?)?,
            "--speed" => cli.speed = speed(&value("--speed")?)?,
            "--no-countdown" | "--quiet" | "--help" | "-h" | "/?" if inline.is_some() => {
                return Err(ArgError(format!("{name} takes no value")));
            }
            "--no-countdown" => cli.countdown = false,
            "--quiet" => cli.quiet = true,
            "--help" | "-h" | "/?" => return Ok(Parsed::Help),
            _ => return Err(ArgError(format!("Unknown option: {arg}"))),
        }
    }
    Ok(Parsed::Run(cli))
}

fn repeat(v: &str) -> Result<Repeat, ArgError> {
    if v.eq_ignore_ascii_case("forever") {
        return Ok(Repeat::Forever);
    }
    match v.parse::<u32>() {
        Ok(n) if n >= 1 => Ok(Repeat::Count(n)),
        _ => Err(ArgError(format!("--repeat must be a number of times (1 or more) or forever, not \"{v}\""))),
    }
}

fn speed(v: &str) -> Result<f32, ArgError> {
    match v.parse::<f64>() {
        Ok(x) if (MIN_SPEED..=MAX_SPEED).contains(&x) => Ok(x as f32),
        _ => Err(ArgError(format!("--speed must be a number from {MIN_SPEED} to {MAX_SPEED}, not \"{v}\""))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved() -> PlaybackOptions {
        PlaybackOptions { repeat: Repeat::Count(3), speed: 1.5, humanize: true, ..Default::default() }
    }

    fn run(args: &[&str]) -> Cli {
        match parse(args.iter().map(|a| a.to_string()), &saved()) {
            Ok(Parsed::Run(cli)) => cli,
            other => panic!("{args:?}: {other:?}"),
        }
    }

    fn error(args: &[&str]) -> String {
        match parse(args.iter().map(|a| a.to_string()), &saved()) {
            Err(e) => e.to_string(),
            other => panic!("{args:?}: {other:?}"),
        }
    }

    #[test]
    fn no_options_play_as_saved() {
        let cli = run(&[]);
        assert_eq!(cli, Cli { repeat: Repeat::Count(3), speed: 1.5, countdown: true, quiet: false });
        let applied = cli.apply(&saved());
        assert_eq!(applied, saved());
    }

    #[test]
    fn options_override_the_saved_ones() {
        let cli = run(&["--repeat", "forever", "--speed=2", "--no-countdown", "--quiet"]);
        assert_eq!(cli, Cli { repeat: Repeat::Forever, speed: 2.0, countdown: false, quiet: true });
        let applied = cli.apply(&saved());
        assert_eq!((applied.repeat, applied.speed, applied.humanize), (Repeat::Forever, 2.0, true));
        assert_eq!(run(&["--repeat=1", "--speed", "0.5"]).repeat, Repeat::Count(1));
        assert_eq!(run(&["--repeat", "FOREVER"]).repeat, Repeat::Forever);
        assert_eq!(run(&["--speed", "0.01"]).speed, 0.01);
        assert_eq!(run(&["--speed", "100"]).speed, 100.0);
    }

    #[test]
    fn help_is_asked_for() {
        for h in ["--help", "-h", "/?"] {
            assert_eq!(parse([h.to_string()], &saved()), Ok(Parsed::Help), "{h}");
        }
        assert_eq!(parse(["--quiet".into(), "--help".into()], &saved()), Ok(Parsed::Help));
    }

    #[test]
    fn bad_options_say_what_is_wrong() {
        assert_eq!(error(&["--bogus"]), "Unknown option: --bogus");
        assert_eq!(error(&["play"]), "Unknown option: play");
        assert_eq!(error(&["--repeat"]), "--repeat needs a value");
        assert_eq!(error(&["--speed"]), "--speed needs a value");
        assert_eq!(error(&["--repeat", "0"]), "--repeat must be a number of times (1 or more) or forever, not \"0\"");
        assert_eq!(error(&["--repeat=-2"]), "--repeat must be a number of times (1 or more) or forever, not \"-2\"");
        assert_eq!(error(&["--speed", "abc"]), "--speed must be a number from 0.01 to 100, not \"abc\"");
        assert_eq!(error(&["--speed", "0"]), "--speed must be a number from 0.01 to 100, not \"0\"");
        assert_eq!(error(&["--speed", "101"]), "--speed must be a number from 0.01 to 100, not \"101\"");
        assert_eq!(error(&["--speed", "NaN"]), "--speed must be a number from 0.01 to 100, not \"NaN\"");
        assert_eq!(error(&["--quiet=yes"]), "--quiet takes no value");
    }
}
