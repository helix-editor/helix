impl steel::rvals::Custom for crate::Position {}

struct SRopeSlice<'a>(crate::RopeSlice<'a>);

impl<'a> SRopeSlice<'a> {
    pub fn char_to_byte(&self, pos: usize) -> usize {
        self.0.char_to_byte(pos)
    }

    pub fn byte_slice(&'a self, lower: usize, upper: usize) -> SRopeSlice<'a> {
        SRopeSlice(self.0.byte_slice(lower..upper))
    }

    pub fn line(&'a self, cursor: usize) -> SRopeSlice<'a> {
        SRopeSlice(self.0.line(cursor))
    }

    // Reference types are really sus. Not sure how this is going to work, but it might? Hopefully it cleans
    // itself up as we go...
    pub fn as_str(&'a self) -> Option<&'a str> {
        self.0.as_str()
    }
}
