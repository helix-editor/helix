pub const FULL: &str = "█";
pub const SEVEN_EIGHTHS: &str = "▉";
pub const THREE_QUARTERS: &str = "▊";
pub const FIVE_EIGHTHS: &str = "▋";
pub const HALF: &str = "▌";
pub const THREE_EIGHTHS: &str = "▍";
pub const ONE_QUARTER: &str = "▎";
pub const ONE_EIGHTH: &str = "▏";

#[derive(Debug, Clone)]
pub struct Set {
    pub full: &'static str,
    pub seven_eighths: &'static str,
    pub three_quarters: &'static str,
    pub five_eighths: &'static str,
    pub half: &'static str,
    pub three_eighths: &'static str,
    pub one_quarter: &'static str,
    pub one_eighth: &'static str,
    pub empty: &'static str,
}

pub const THREE_LEVELS: Set = Set {
    full: FULL,
    seven_eighths: FULL,
    three_quarters: HALF,
    five_eighths: HALF,
    half: HALF,
    three_eighths: HALF,
    one_quarter: HALF,
    one_eighth: " ",
    empty: " ",
};

pub const NINE_LEVELS: Set = Set {
    full: FULL,
    seven_eighths: SEVEN_EIGHTHS,
    three_quarters: THREE_QUARTERS,
    five_eighths: FIVE_EIGHTHS,
    half: HALF,
    three_eighths: THREE_EIGHTHS,
    one_quarter: ONE_QUARTER,
    one_eighth: ONE_EIGHTH,
    empty: " ",
};
