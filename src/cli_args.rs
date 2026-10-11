/// Command-line switches. Only `--hub` exists: the my-utilities hub launches
/// Fast English with it so the hub owns the tray and settings.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CliArgs {
    pub hub: bool,
}

/// `args` includes the program name as its first item, like `std::env::args()`.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> CliArgs {
    CliArgs {
        hub: args.into_iter().skip(1).any(|a| a == "--hub"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn hub_defaults_off() {
        assert_eq!(parse(args(&["FastEnglish.exe"])), CliArgs { hub: false });
        assert!(!parse(args(&["FastEnglish.exe", "--other"])).hub);
    }

    #[test]
    fn hub_flag_parsed() {
        assert!(parse(args(&["FastEnglish.exe", "--hub"])).hub);
        // The program name is never a flag.
        assert!(!parse(args(&["--hub"])).hub);
    }
}
