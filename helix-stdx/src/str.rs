use smartstring::{LazyCompact, SmartString};
use std::borrow::Cow;

/// Unescapes a string, converting escape sequences into their literal characters.
///
/// This function handles the following escape sequences:
/// - `\\n` is converted to `\n` (newline)
/// - `\\t` is converted to `\t` (tab)
/// - `\\u{...}` is converted to the corresponding Unicode character
///
/// Other escape sequences, such as `\\` followed by any character not listed above, will remain unchanged.
///
/// If input is invalid, for example if there is invalid unicode, \u{999999999}, it will return the input as is.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use helix_stdx::str::unescape;
/// let unescaped = unescape("hello\\nworld");
/// assert_eq!("hello\nworld", unescaped);
/// ```
///
/// Unescaping tabs:
///
/// ```
/// # use helix_stdx::str::unescape;
/// let unescaped = unescape("hello\\tworld");
/// assert_eq!("hello\tworld", unescaped);
/// ```
///
/// Unescaping Unicode characters:
///
/// ```
/// # use helix_stdx::str::unescape;
/// let unescaped = unescape("hello\\u{1f929}world");
/// assert_eq!("hello\u{1f929}world", unescaped);
/// assert_eq!("hello🤩world", unescaped);
/// ```
///
/// Handling backslashes:
///
/// ```
/// # use helix_stdx::str::unescape;
/// let unescaped = unescape(r"hello\\world");
/// assert_eq!(r"hello\\world", unescaped);
///
/// let unescaped = unescape(r"hello\\\\world");
/// assert_eq!(r"hello\\\\world", unescaped);
/// ```
///
/// # Note
///
/// This function is opinionated, with a clear purpose of handling user input, not a general or generic unescaping utility, and does not unescape sequences like `\\'` or `\\\"`, leaving them as is.
#[inline]
pub fn unescape(s: &str) -> Cow<'_, str> {
    enum State {
        Normal,
        Escaped,
        Unicode,
    }

    let mut unescaped = String::new();
    let mut state = State::Normal;
    let mut is_escaped = false;
    // NOTE: Max unicode code point is U+10FFFF for a maximum of 6 chars
    let mut unicode = SmartString::<LazyCompact>::new_const();

    for (idx, ch) in s.char_indices() {
        match state {
            State::Normal => match ch {
                '\\' => {
                    if !is_escaped {
                        // PERF: As not every separator will be escaped, we use `String::new` as that has no initial
                        // allocation. If an escape is found, then we reserve capacity thats the len of the separator,
                        // as the new unescaped string will be at least that long.
                        unescaped.reserve(s.len());
                        if idx > 0 {
                            // First time finding an escape, so all prior chars can be added to the new unescaped
                            // version if its not the very first char found.
                            unescaped.push_str(&s[0..idx]);
                        }
                    }
                    state = State::Escaped;
                    is_escaped = true;
                }
                _ => {
                    if is_escaped {
                        unescaped.push(ch);
                    }
                }
            },
            State::Escaped => {
                match ch {
                    'n' => unescaped.push('\n'),
                    't' => unescaped.push('\t'),
                    'u' => {
                        state = State::Unicode;
                        continue;
                    }
                    // Uncomment if you want to handle '\\' to '\'
                    // '\\' => unescaped.push('\\'),
                    _ => {
                        unescaped.push('\\');
                        unescaped.push(ch);
                    }
                }
                state = State::Normal;
            }
            State::Unicode => match ch {
                '{' => continue,
                '}' => {
                    let Ok(digit) = u32::from_str_radix(&unicode, 16) else {
                        return s.into();
                    };
                    let Some(point) = char::from_u32(digit) else {
                        return s.into();
                    };
                    unescaped.push(point);
                    // Might be more unicode to unescape so clear for reuse.
                    unicode.clear();
                    state = State::Normal;
                }
                _ => unicode.push(ch),
            },
        }
    }

    if is_escaped {
        unescaped.into()
    } else {
        s.into()
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_unescape_newline() {
        let unescaped = unescape("hello\\nworld");
        assert_eq!("hello\nworld", unescaped);
    }

    #[test]
    fn should_unescape_tab() {
        let unescaped = unescape("hello\\tworld");
        assert_eq!("hello\tworld", unescaped);
    }

    #[test]
    fn should_unescape_unicode() {
        let unescaped = unescape("hello\\u{1f929}world");
        assert_eq!("hello\u{1f929}world", unescaped, "char: 🤩 ");
        assert_eq!("hello🤩world", unescaped);
    }

    #[test]
    fn should_return_original_input_due_to_bad_unicode() {
        let unescaped = unescape("hello\\u{999999999}world");
        assert_eq!("hello\\u{999999999}world", unescaped);
    }

    #[test]
    fn should_not_unescape_slash() {
        let unescaped = unescape(r"hello\\world");
        assert_eq!(r"hello\\world", unescaped);

        let unescaped = unescape(r"hello\\\\world");
        assert_eq!(r"hello\\\\world", unescaped);
    }

    #[test]
    fn should_not_unescape_slash_single_quote() {
        let unescaped = unescape("\\'");
        assert_eq!(r"\'", unescaped);
    }

    #[test]
    fn should_not_unescape_slash_double_quote() {
        let unescaped = unescape("\\\"");
        assert_eq!(r#"\""#, unescaped);
    }

    #[test]
    fn should_not_change_anything() {
        let unescaped = unescape("'");
        assert_eq!("'", unescaped);
        let unescaped = unescape(r#"""#);
        assert_eq!(r#"""#, unescaped);
    }

    #[test]
    fn should_only_unescape_newline_not_slash_single_quote() {
        let unescaped = unescape("\\n\'");
        assert_eq!("\n'", unescaped);
        let unescaped = unescape("\\n\\'");
        assert_eq!("\n\\'", unescaped);
    }
}
