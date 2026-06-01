pub const DOT: &str = "•";

/// Marker to use when plotting data points
#[derive(Debug, Clone, Copy)]
pub enum Marker {
    /// One point per cell in shape of dot
    Dot,
    /// One point per cell in shape of a block
    Block,
    /// Up to 8 points per cell
    Braille,
}
