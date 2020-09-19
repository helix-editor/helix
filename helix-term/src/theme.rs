use std::collections::HashMap;
use tui::style::{Color, Style};

/// Color theme for syntax highlighting.
pub struct Theme {
    scopes: Vec<String>,
    mapping: HashMap<&'static str, Style>,
}

// let highlight_names: Vec<String> = [
//     "attribute",
//     "constant.builtin",
//     "constant",
//     "function.builtin",
//     "function.macro",
//     "function",
//     "keyword",
//     "operator",
//     "property",
//     "punctuation",
//     "comment",
//     "escape",
//     "label",
//     // "punctuation.bracket",
//     "punctuation.delimiter",
//     "string",
//     "string.special",
//     "tag",
//     "type",
//     "type.builtin",
//     "constructor",
//     "variable",
//     "variable.builtin",
//     "variable.parameter",
//     "path",
// ];

impl Default for Theme {
    fn default() -> Self {
        let mapping = hashmap! {
            "attribute" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "keyword" => Style::default().fg(Color::Rgb(236, 205, 186)), // almond
            "punctuation" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "punctuation.delimiter" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "operator" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "property" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "variable.parameter" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            // TODO distinguish type from type.builtin?
            "type" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "type.builtin" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "constructor" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "function" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "function.macro" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "comment" => Style::default().fg(Color::Rgb(105, 124, 129)), // sirocco
            "variable.builtin" => Style::default().fg(Color::Rgb(159, 242, 143)), // mint
            "constant" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "constant.builtin" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "string" => Style::default().fg(Color::Rgb(204, 204, 204)), // silver
            "escape" => Style::default().fg(Color::Rgb(239, 186, 93)), // honey
            // used for lifetimes
            "label" => Style::default().fg(Color::Rgb(239, 186, 93)), // honey

            // TODO: diferentiate number builtin
            // TODO: diferentiate doc comment
            // TODO: variable as lilac
            // TODO: mod/use statements as white
            // TODO: mod stuff as chamoise
            // TODO: add "(scoped_identifier) @path" for std::mem::
            //
            // concat (ERROR) @syntax-error and "MISSING ;" selectors for errors

            "module" => Style::default().fg(Color::Rgb(255, 0, 0)), // white
            "variable" => Style::default().fg(Color::Rgb(255, 0, 0)), // white
            "function.builtin" => Style::default().fg(Color::Rgb(255, 0, 0)), // white

            "ui.background" => Style::default().bg(Color::Rgb(59, 34, 76)), // midnight
            "ui.linenr" => Style::default().fg(Color::Rgb(90, 89, 119)), // comet
            "ui.statusline" => Style::default().bg(Color::Rgb(40, 23, 51)), // revolver
        };

        let scopes = mapping.keys().map(ToString::to_string).collect();

        Self { mapping, scopes }
    }
}

impl Theme {
    pub fn get(&self, scope: &str) -> Style {
        self.mapping
            .get(scope)
            .copied()
            .unwrap_or_else(|| Style::default().fg(Color::Rgb(0, 0, 255)))
    }

    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
}
