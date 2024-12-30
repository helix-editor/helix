use std::cmp::Ordering;

use helix_core::diagnostic::Severity;
use helix_core::doc_formatter::{DocumentFormatter, FormattedGrapheme};
use helix_core::graphemes::Grapheme;
use helix_core::text_annotations::TextAnnotations;
use helix_core::{Diagnostic, Position};
use helix_view::annotations::diagnostics::{
    DiagnosticFilter, InlineDiagnosticAccumulator, InlineDiagnosticsConfig,
};

use helix_view::icons::ICONS;
use helix_view::theme::Style;
use helix_view::{Document, Theme};

use crate::ui::document::{LinePos, TextRenderer};
use crate::ui::text_decorations::Decoration;

#[derive(Debug)]
struct Styles {
    hint: Style,
    info: Style,
    warning: Style,
    error: Style,
}

impl Styles {
    fn new(theme: &Theme) -> Styles {
        Styles {
            hint: theme.get("hint"),
            info: theme.get("info"),
            warning: theme.get("warning"),
            error: theme.get("error"),
        }
    }

    fn severity_style(&self, severity: Severity) -> Style {
        match severity {
            Severity::Hint => self.hint,
            Severity::Info => self.info,
            Severity::Warning => self.warning,
            Severity::Error => self.error,
        }
    }
}

pub struct InlineDiagnostics<'a> {
    state: InlineDiagnosticAccumulator<'a>,
    eol_diagnostics: DiagnosticFilter,
    styles: Styles,
}

impl<'a> InlineDiagnostics<'a> {
    pub fn new(
        doc: &'a Document,
        theme: &Theme,
        cursor: usize,
        config: InlineDiagnosticsConfig,
        eol_diagnostics: DiagnosticFilter,
    ) -> Self {
        InlineDiagnostics {
            state: InlineDiagnosticAccumulator::new(cursor, doc, config),
            styles: Styles::new(theme),
            eol_diagnostics,
        }
    }
}

const BL_CORNER: &str = "┘";
const TR_CORNER: &str = "┌";
const BR_CORNER: &str = "└";
const STACK: &str = "├";
const MULTI: &str = "┴";
const HOR_BAR: &str = "─";
const VER_BAR: &str = "│";

struct Renderer<'a, 'b> {
    renderer: &'a mut TextRenderer<'b>,
    first_row: u16,
    row: u16,
    config: &'a InlineDiagnosticsConfig,
    styles: &'a Styles,
}

impl Renderer<'_, '_> {
    fn draw_decoration(&mut self, g: &'static str, severity: Severity, col: u16) {
        self.draw_decoration_at(g, severity, col, self.row)
    }

    fn draw_decoration_at(&mut self, g: &'static str, severity: Severity, col: u16, row: u16) {
        self.renderer.draw_decoration_grapheme(
            Grapheme::new_decoration(g),
            self.styles.severity_style(severity),
            row,
            col,
        );
    }

    fn draw_eol_diagnostic(&mut self, diag: &Diagnostic, row: u16, col: usize) -> u16 {
        let style = self.styles.severity_style(diag.severity());
        let width = self.renderer.viewport.width;
        let start_col = (col - self.renderer.offset.col) as u16;
        let mut end_col = start_col;
        let mut draw_col = (col + 1) as u16;

        // Draw the diagnostic indicator:
        if !self.renderer.column_in_bounds(draw_col as usize, 2) {
            return 0;
        }

        let icons = ICONS.load();

        let symbol = match diag.severity() {
            Severity::Hint => icons.diagnostic().hint(),
            Severity::Info => icons.diagnostic().info(),
            Severity::Warning => icons.diagnostic().warning(),
            Severity::Error => icons.diagnostic().error(),
        };

        self.renderer
            .set_string(self.renderer.viewport.x + draw_col, row, symbol, style);
        draw_col += 2;

        for line in diag.message.lines() {
            if !self.renderer.column_in_bounds(draw_col as usize, 1) {
                break;
            }

            (end_col, _) = self.renderer.set_string_truncated(
                self.renderer.viewport.x + draw_col,
                row,
                line,
                width.saturating_sub(draw_col) as usize,
                |_| style,
                true,
                false,
            );

            draw_col = end_col - self.renderer.viewport.x + 2; // double space between lines
        }

        end_col - start_col
    }

    fn draw_diagnostic(&mut self, diag: &Diagnostic, col: u16, next_severity: Option<Severity>) {
        let severity = diag.severity();
        let (sym, sym_severity) = if let Some(next_severity) = next_severity {
            (STACK, next_severity.max(severity))
        } else {
            (BR_CORNER, severity)
        };
        self.draw_decoration(sym, sym_severity, col);
        for i in 0..self.config.prefix_len {
            self.draw_decoration(HOR_BAR, severity, col + i + 1);
        }

        let text_col = col + self.config.prefix_len + 1;
        let text_fmt = self.config.text_fmt(text_col, self.renderer.viewport.width);
        let annotations = TextAnnotations::default();
        let formatter = DocumentFormatter::new_at_prev_checkpoint(
            diag.message.as_str().trim().into(),
            &text_fmt,
            &annotations,
            0,
        );
        let mut last_row = 0;
        let style = self.styles.severity_style(severity);
        for grapheme in formatter {
            last_row = grapheme.visual_pos.row;
            self.renderer.draw_decoration_grapheme(
                grapheme.raw,
                style,
                self.row + grapheme.visual_pos.row as u16,
                text_col + grapheme.visual_pos.col as u16,
            );
        }
        self.row += 1;
        // height is last_row + 1 and extra_rows is height - 1
        let extra_lines = last_row;
        if let Some(next_severity) = next_severity {
            for _ in 0..extra_lines {
                self.draw_decoration(VER_BAR, next_severity, col);
                self.row += 1;
            }
        } else {
            self.row += extra_lines as u16;
        }
    }

    fn draw_multi_diagnostics(&mut self, stack: &mut Vec<(&Diagnostic, u16)>) {
        let Some(&(last_diag, last_anchor)) = stack.last() else {
            return;
        };
        let start = self
            .config
            .max_diagnostic_start(self.renderer.viewport.width);

        if last_anchor <= start {
            return;
        }
        let mut severity = last_diag.severity();
        let mut last_anchor = last_anchor;
        self.draw_decoration(BL_CORNER, severity, last_anchor);
        let mut stacked_diagnostics = 1;
        for &(diag, anchor) in stack.iter().rev().skip(1) {
            let sym = match anchor.cmp(&start) {
                Ordering::Less => break,
                Ordering::Equal => STACK,
                Ordering::Greater => MULTI,
            };
            stacked_diagnostics += 1;
            severity = severity.max(diag.severity());
            let old_severity = severity;
            if anchor == last_anchor && severity == old_severity {
                continue;
            }
            for col in (anchor + 1)..last_anchor {
                self.draw_decoration(HOR_BAR, old_severity, col)
            }
            self.draw_decoration(sym, severity, anchor);
            last_anchor = anchor;
        }

        // if no diagnostic anchor was found exactly at the start of the
        // diagnostic text  draw an upwards corner and ensure the last piece
        // of the line is not missing
        if last_anchor != start {
            for col in (start + 1)..last_anchor {
                self.draw_decoration(HOR_BAR, severity, col)
            }
            self.draw_decoration(TR_CORNER, severity, start)
        }
        self.row += 1;
        let stacked_diagnostics = &stack[stack.len() - stacked_diagnostics..];

        for (i, (diag, _)) in stacked_diagnostics.iter().rev().enumerate() {
            let next_severity = stacked_diagnostics[..stacked_diagnostics.len() - i - 1]
                .iter()
                .map(|(diag, _)| diag.severity())
                .max();
            self.draw_diagnostic(diag, start, next_severity);
        }

        stack.truncate(stack.len() - stacked_diagnostics.len());
    }

    fn draw_diagnostics(&mut self, stack: &mut Vec<(&Diagnostic, u16)>) {
        let mut stack = stack.drain(..).rev().peekable();
        let mut last_anchor = self.renderer.viewport.width;
        while let Some((diag, anchor)) = stack.next() {
            if anchor != last_anchor {
                for row in self.first_row..self.row {
                    self.draw_decoration_at(VER_BAR, diag.severity(), anchor, row);
                }
            }
            let next_severity = stack.peek().and_then(|&(diag, next_anchor)| {
                (next_anchor == anchor).then_some(diag.severity())
            });
            self.draw_diagnostic(diag, anchor, next_severity);
            last_anchor = anchor;
        }
    }
}

impl Decoration for InlineDiagnostics<'_> {
    fn render_virt_lines(
        &mut self,
        renderer: &mut TextRenderer,
        pos: LinePos,
        virt_off: Position,
    ) -> Position {
        let mut col_off = 0;
        let filter = self.state.filter();
        let eol_diagnostic = match self.eol_diagnostics {
            DiagnosticFilter::Enable(eol_filter) => {
                let eol_diganogistcs = self
                    .state
                    .stack
                    .iter()
                    .filter(|(diag, _)| eol_filter <= diag.severity());
                match filter {
                    DiagnosticFilter::Enable(filter) => eol_diganogistcs
                        .filter(|(diag, _)| filter > diag.severity())
                        .max_by_key(|(diagnostic, _)| diagnostic.severity),
                    DiagnosticFilter::Disable => {
                        eol_diganogistcs.max_by_key(|(diagnostic, _)| diagnostic.severity)
                    }
                }
            }
            DiagnosticFilter::Disable => None,
        };
        if let Some((eol_diagnostic, _)) = eol_diagnostic {
            let mut renderer = Renderer {
                renderer,
                first_row: pos.visual_line,
                row: pos.visual_line,
                config: &self.state.config,
                styles: &self.styles,
            };
            col_off = renderer.draw_eol_diagnostic(eol_diagnostic, pos.visual_line, virt_off.col);
        }

        self.state.compute_line_diagnostics();
        let mut renderer = Renderer {
            renderer,
            first_row: pos.visual_line + virt_off.row as u16,
            row: pos.visual_line + virt_off.row as u16,
            config: &self.state.config,
            styles: &self.styles,
        };
        renderer.draw_multi_diagnostics(&mut self.state.stack);
        renderer.draw_diagnostics(&mut self.state.stack);
        let horizontal_off = renderer.row - renderer.first_row;
        Position::new(horizontal_off as usize, col_off as usize)
    }

    fn reset_pos(&mut self, pos: usize) -> usize {
        self.state.reset_pos(pos)
    }

    fn skip_concealed_anchor(&mut self, conceal_end_char_idx: usize) -> usize {
        self.state.skip_concealed(conceal_end_char_idx)
    }

    fn decorate_grapheme(
        &mut self,
        renderer: &mut TextRenderer,
        grapheme: &FormattedGrapheme,
    ) -> usize {
        self.state
            .proccess_anchor(grapheme, renderer.viewport.width, renderer.offset.col)
    }
}
