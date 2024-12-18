pub struct Flag {
    pub long: &'static str,
    pub short: Option<&'static str>,
    pub desc: &'static str,
}

#[macro_export]
macro_rules! flag {
    (
        long: $long:expr,
        short: $short:expr,
        desc: $desc:expr
    ) => {
        $crate::commands::flag::Flag {
            long: $long,
            short: Some($short),
            desc: $desc,
        }
    };
    (
        long: $long:expr,
        desc: $desc:expr
    ) => {
        $crate::commands::flag::Flag {
            long: $long,
            short: None,
            desc: $desc,
        }
    };
}

#[cfg(test)]
mod tests {

    #[test]
    fn should_turn_macro_to_struct() {
        let full = flag! {
           long: "--all",
           short: "-a",
           desc:  "clears all registers"
        };

        assert_eq!("--all", full.long);
        assert_eq!(Some("-a"), full.short);
        assert_eq!("clears all registers", full.desc);

        let partial = flag! {
           long: "--all",
           desc:  "clears all registers"
        };

        assert_eq!("--all", partial.long);
        assert_eq!(None, partial.short);
        assert_eq!("clears all registers", partial.desc);
    }
}
