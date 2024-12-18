pub struct Flag {
    pub long: &'static str,
    pub short: Option<&'static str>,
    pub desc: &'static str,
    pub takes: bool,
}

#[macro_export]
macro_rules! flag {
    // Case: All fields provided
    (
        long: $long:expr,
        short: $short:expr,
        desc: $desc:expr,
        takes: $takes:expr $(,)?
    ) => {
        $crate::commands::flag::Flag {
            long: $long,
            short: Some($short),
            desc: $desc,
            takes: $takes,
        }
    };
    // Case: All fields except takes
    (
        long: $long:expr,
        short: $short:expr,
        desc: $desc:expr $(,)?
    ) => {
        $crate::commands::flag::Flag {
            long: $long,
            short: Some($short),
            desc: $desc,
            takes: false,
        }
    };
    // Case: Only long, desc, and takes
    (
        long: $long:expr,
        desc: $desc:expr,
        takes: $takes:expr $(,)?
    ) => {
        $crate::commands::flag::Flag {
            long: $long,
            short: None,
            desc: $desc,
            takes: $takes,
        }
    };
    // Case: Only long and desc
    (
        long: $long:expr,
        desc: $desc:expr $(,)?
    ) => {
        $crate::commands::flag::Flag {
            long: $long,
            short: None,
            desc: $desc,
            takes: false,
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
           desc:  "clears all registers",
           takes: true,
        };

        assert_eq!("--all", full.long);
        assert_eq!(Some("-a"), full.short);
        assert_eq!("clears all registers", full.desc);
        assert!(full.takes);

        let partial = flag! {
           long: "--all",
           desc:  "clears all registers"
        };

        assert_eq!("--all", partial.long);
        assert_eq!(None, partial.short);
        assert_eq!("clears all registers", partial.desc);
        assert!(!partial.takes);
    }
}
