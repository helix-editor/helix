use helix_core::shellwords::IntoFlags;

#[derive(Debug, Clone, Copy)]
pub struct Flags(&'static [Flag]);

impl Flags {
    #[inline]
    pub const fn new(flags: &'static [Flag]) -> Self {
        Self(flags)
    }

    #[inline]
    pub const fn empty() -> Self {
        Self(&[])
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn names(&self) -> impl Iterator<Item = &'static str> {
        self.0
            .iter()
            .map(|flag| flag.long)
            .chain(self.0.iter().filter_map(|flag| flag.short))
    }
}

impl IntoIterator for &Flags {
    type Item = &'static Flag;

    type IntoIter = std::slice::Iter<'static, Flag>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoFlags for Flags {
    #[inline]
    fn into_flags(self) -> impl Iterator<Item = &'static str> {
        self.names()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Flag {
    pub long: &'static str,
    pub short: Option<&'static str>,
    pub desc: &'static str,
    pub accepts: Option<&'static str>,
}

#[macro_export]
macro_rules! flags {
    // Empty case
    [] => {
        $crate::commands::flag::Flags::empty()
    };
    // Multiple flags case
    [$({ long: $long:expr, $(short: $short:expr,)? desc: $desc:expr $(, accepts: $accepts:expr)? $(,)?}),* $(,)?] => {
        {
            const FLAGS: &[$crate::commands::flag::Flag] = &[
                $(
                    $crate::commands::flag::Flag {
                        long: $long,
                        short: {
                            #[allow(unused_mut, unused_assignments)]
                            let mut short: Option<&'static str> = None;
                            $(short = Some($short);)?
                            short
                        },
                        desc: $desc,
                        accepts: {
                            #[allow(unused_mut, unused_assignments)]
                            let mut accepts: Option<&'static str> = None;
                            $(accepts = Some($accepts);)?
                            accepts
                        },
                    }
                ),*
            ];
            $crate::commands::flag::Flags::new(FLAGS)
        }
    };
}

#[cfg(test)]
mod tests {

    #[test]
    fn should_turn_macro_to_struct() {
        let flags = flags! [
           {
              long: "--all",
              short: "-a",
              desc:  "clears all registers",
           },
           {
              long: "--all",
              desc:  "clears all registers",
              accepts: "<register>"
           }
        ];

        let mut iter = flags.into_iter();

        let full = iter.next().unwrap();
        assert_eq!("--all", full.long);
        assert_eq!(Some("-a"), full.short);
        assert_eq!("clears all registers", full.desc);
        assert!(full.accepts.is_none());

        let partial = iter.next().unwrap();

        assert_eq!("--all", partial.long);
        assert_eq!(None, partial.short);
        assert_eq!("clears all registers", partial.desc);
        assert_eq!(Some("<register>"), partial.accepts);
    }
}
