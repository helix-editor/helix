use helix_core::doc_formatter::FormattedGrapheme;
use helix_core::text_annotations::LineAnnotation;
use helix_core::Position;

/// Reserves vertical space for the multi-line portion of a Copilot ghost text
/// suggestion.
///
/// The first line of the suggestion is rendered inline after the cursor via an
/// [`InlineAnnotation`](helix_core::text_annotations::InlineAnnotation); every
/// subsequent line is drawn on its own virtual line below the cursor's document
/// line. This `LineAnnotation` only reserves the empty space for those lines;
/// the matching decoration in `helix-term` fills it in. The two are kept in sync
/// by anchoring on the same cursor char index.
pub struct CopilotGhostText {
    /// The char index the suggestion is anchored at (the cursor at request time).
    cursor: usize,
    /// The number of virtual lines to reserve (the suggestion lines after the
    /// first).
    lines: usize,
    /// Whether the cursor anchor has been seen on the visual line currently
    /// being laid out.
    anchored: bool,
}

impl CopilotGhostText {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<'a>(cursor: usize, lines: usize) -> Box<dyn LineAnnotation + 'a> {
        Box::new(CopilotGhostText {
            cursor,
            lines,
            anchored: false,
        })
    }
}

impl LineAnnotation for CopilotGhostText {
    fn reset_pos(&mut self, char_idx: usize) -> usize {
        self.anchored = false;
        if self.cursor >= char_idx {
            self.cursor
        } else {
            usize::MAX
        }
    }

    fn skip_concealed_anchors(&mut self, conceal_end_char_idx: usize) -> usize {
        if self.cursor >= conceal_end_char_idx {
            self.cursor
        } else {
            usize::MAX
        }
    }

    fn process_anchor(&mut self, grapheme: &FormattedGrapheme) -> usize {
        if grapheme.char_idx == self.cursor {
            self.anchored = true;
        }
        usize::MAX
    }

    fn insert_virtual_lines(
        &mut self,
        _line_end_char_idx: usize,
        _line_end_visual_pos: Position,
        _doc_line: usize,
    ) -> Position {
        if self.anchored {
            self.anchored = false;
            Position::new(self.lines, 0)
        } else {
            Position::new(0, 0)
        }
    }
}
