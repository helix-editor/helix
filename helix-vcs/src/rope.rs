use ropey::Rope;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct RopeLine<'a>(pub ropey::RopeSlice<'a>);

impl<'a> RopeLine<'a> {
    pub fn from_rope(rope: &'a Rope) -> Vec<Self> {
        rope.lines().into_iter().map(RopeLine).collect()
    }
}
