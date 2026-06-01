use helix_view::graphics::Style;

/// A grapheme associated to a style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledGrapheme<'a> {
    pub symbol: &'a str,
    pub style: Style,
}
