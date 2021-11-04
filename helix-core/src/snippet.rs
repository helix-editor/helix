// allow dead_code is only temporary to suppress warning during development
#![allow(dead_code)]
// use std::collections::BTreeMap;
use regex::{Regex, RegexBuilder};
use ropey::Rope;
use smallvec::{smallvec, SmallVec};

use std::error::Error;
use std::fmt::Display;

use crate::{Change, Range, Selection, Tendril, Transaction};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenType {
    Dollar,
    Colon,
    Comma,
    CurlyOpen,
    CurlyClose,
    BackSlash,
    ForwardSlash,
    Pipe,
    Int,
    VariableName,
    Format,
    Plus,
    Dash,
    QuestionMark,
    EOF,
    Undefined,
}

impl Default for TokenType {
    fn default() -> Self {
        TokenType::Undefined
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct Token {
    typ: TokenType,
    pos: usize,
    len: usize,
}

impl Token {
    fn new(typ: TokenType, pos: usize, len: usize) -> Self {
        Self { typ, pos, len }
    }
}

/// map a single char to TokenType
fn char_token_type(ch: char) -> TokenType {
    match ch {
        '$' => TokenType::Dollar,
        ':' => TokenType::Colon,
        ',' => TokenType::Comma,
        '{' => TokenType::CurlyOpen,
        '}' => TokenType::CurlyClose,
        '\\' => TokenType::BackSlash,
        '/' => TokenType::ForwardSlash,
        '|' => TokenType::Pipe,
        '+' => TokenType::Plus,
        '-' => TokenType::Dash,
        '?' => TokenType::QuestionMark,
        _ => TokenType::Undefined,
    }
}

#[derive(Debug, Default)]
struct Scanner {
    pub value: String,
    value_chars: Vec<char>,

    pub pos: usize,
}

fn is_variable_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

/// Scanner produces a stream of tokens
impl Scanner {
    /// accept a `impl Into<String>`, and get ready to serve a stream of tokens
    pub fn text(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.value_chars = self.value.chars().collect();
        self.pos = 0;
    }

    /// retrive the actual text of a token
    pub fn token_text(&self, token: Token) -> String {
        if (token.pos + token.len) <= self.value_chars.len() {
            self.value_chars[token.pos..(token.pos + token.len)]
                .iter()
                .collect::<String>()
        } else {
            "".to_string()
        }
    }

    /// get the next token
    pub fn next(&mut self) -> Token {
        if self.pos >= self.value.len() {
            return Token::new(TokenType::EOF, self.pos, 0);
        }
        let pos = self.pos;
        let mut len = 0;
        let mut value_chars = self.value.chars(); // number
        let ch = match value_chars.nth(pos) {
            Some(ch) => ch,
            None => {
                return Token::new(TokenType::EOF, self.pos, 0);
            }
        };
        let mut typ = char_token_type(ch);
        match typ {
            TokenType::Undefined => {}
            _ => {
                self.pos += 1;
                return Token::new(typ, pos, 1);
            }
        }

        len += 1;

        if ch.is_ascii_digit() {
            typ = TokenType::Int;
            for ch in value_chars {
                if !ch.is_ascii_digit() {
                    break;
                } else {
                    len += 1;
                }
            }
            self.pos += len;
            return Token::new(typ, pos, len);
        }

        if is_variable_char(ch) {
            typ = TokenType::VariableName;
            for ch in value_chars {
                if !is_variable_char(ch) {
                    break;
                } else {
                    len += 1;
                }
            }
            self.pos += len;
            return Token::new(typ, pos, len);
        }

        typ = TokenType::Format;
        for ch in value_chars {
            if char_token_type(ch) != TokenType::Undefined
                || ch.is_ascii_digit()
                || is_variable_char(ch)
            {
                break;
            } else {
                len += 1;
            }
        }
        self.pos += len;
        Token::new(typ, pos, len)
    }
}

// What should we report?
#[derive(Debug, Default)]
struct SnippetParseErr {}

impl Display for SnippetParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("")
    }
}
impl Error for SnippetParseErr {}

#[derive(Debug, PartialEq)]
struct SnippetPlaceholder {
    idx: usize,
    value: Vec<SnippetItem>,
}

#[derive(Debug, PartialEq)]
struct SnippetChoice {
    idx: usize,
    choices: Vec<String>,
}

#[derive(Debug, PartialEq)]
struct SnippetVariable {
    var_name: String,
    value: Vec<SnippetItem>,
}

#[derive(Debug)]
struct SnippetTransform {
    reg: Regex,
    // SnippetFormat or Text
    format_item: Vec<SnippetFormat>,
}

impl PartialEq for SnippetTransform {
    fn eq(&self, other: &Self) -> bool {
        self.reg.as_str() == other.reg.as_str() && self.format_item == other.format_item
    }
}

#[derive(Debug, PartialEq)]
enum FormatOperation {
    Upcase,
    Downcase,
    Capitalize,
}

#[derive(Debug, PartialEq)]
enum SnippetFormat {
    // regex provides a very convenient replace method
    Replacer(String),
    // /upcase /downcase /captilize
    Operation(usize, FormatOperation),
    // (capture_group, if_value, else_value)
    If(usize, String, String),
}

#[derive(Debug, PartialEq)]
enum SnippetItem {
    Text(String),
    Choice(SnippetChoice),
    TabStop(usize),
    PlaceHolder(SnippetPlaceholder),
    Variable(SnippetVariable),
    Transform(SnippetTransform),
}

impl SnippetItem {
    // eventually we'll need to move this into Parser
    pub fn generate(
        &self,
        doc: &Rope,
        sels: &Selection,
        start_pos: usize,
        cur_after_pos: usize,
    ) -> (SmallVec<[Range; 1]>, Vec<Change>, usize) {
        let mut changes: Vec<Change> = vec![];
        let mut ranges = smallvec![];
        let mut after_pos = cur_after_pos;

        match self {
            SnippetItem::Text(t) => {
                let tmp: Tendril = t.as_str().into();
                after_pos += tmp.len();
                changes.push((start_pos, start_pos, Some(tmp)));
            }
            SnippetItem::Choice(_) => {
                // todo!()
            }
            SnippetItem::TabStop(idx) => {
                // TabStop will be supported after marks(#703) is implemented
                // ranges.push();
            }
            SnippetItem::PlaceHolder(placeholder) => placeholder.value.iter().for_each(|item| {
                let (mut rngs, mut chgs, l) = item.generate(doc, sels, start_pos, after_pos);
                after_pos = l;
                ranges.append(&mut rngs);
                changes.append(&mut chgs);
            }),
            SnippetItem::Variable(variable) => {
                let expanded_var: String = match variable.var_name.as_str() {
                    // see https://macromates.com/manual/en/environment_variables#dynamic_variables
                    // At least support these variables:
                    // TM_CURRENT_LINE, TM_CURRENT_WORD, TM_DIRECTORY, TM_FILEPATH
                    // TM_LINE_NUMBER, TM_PROJECT_DIRECTORY, TM_SELECTED_TEXT
                    // some of these are tricky, do we pass a `Context` in here or what?
                    "TM_SELECTED_TEXT" => sels.primary().fragment(doc.slice(..)).into(),
                    _ => variable.var_name.clone(),
                };

                changes.push((start_pos, start_pos, Some(expanded_var.as_str().into())));
            }
            SnippetItem::Transform(_) => {
                // Transform alone does not make any sense and
                // should be handled when generating `SnippetItem::Variable`
                unimplemented!()
            }
        }
        (ranges, changes, after_pos)
    }
}

#[derive(Debug, Default)]
struct SnippetParser {
    token: Token,
    scanner: Scanner,

    // move `generate` in SnippetParser, remove this `pub`
    pub s: Vec<SnippetItem>,
}

impl SnippetParser {
    fn init(&mut self, value: impl Into<String>) {
        self.scanner.text(value);
        self.token = self.scanner.next();
        self.s.clear();
    }

    fn next(&mut self) {
        self.token = self.scanner.next();
    }

    fn accept(&mut self, typ: TokenType) -> Option<String> {
        if self.token.typ == typ || typ == TokenType::Undefined && self.token.typ != TokenType::EOF
        {
            let s = self.token_text();
            self.next();
            return Some(s);
        }
        None
    }

    fn accepts(&mut self, typs: &[TokenType]) -> Option<Vec<String>> {
        let tk = self.token;
        let mut result = vec![];
        for t in typs {
            if t != &self.token.typ {
                self.backto(tk);
                return None;
            }
            result.push(self.token_text());
            self.next();
        }
        Some(result)
    }

    fn token_text(&self) -> String {
        self.scanner.token_text(self.token)
    }

    fn backto(&mut self, token: Token) {
        self.scanner.pos = token.pos + token.len;
        self.token = token;
    }

    fn until(&mut self, typ: TokenType) -> Option<String> {
        let mut result = String::default();
        let tk = self.token;
        loop {
            if self.token.typ == TokenType::BackSlash {
                self.next();
                let escaped = self.token_text();
                match escaped.as_str() {
                    "}" | "$" | "/" => {
                        result += &escaped;
                    }
                    _ => {
                        return None;
                    }
                }
            }
            if self.token.typ == typ {
                break;
            }
            if self.token.typ == TokenType::EOF {
                result.clear();
                self.backto(tk);
                return None;
            }
            result += &self.token_text();
            self.next();
        }
        self.next();
        Some(result)
    }

    fn parse(&mut self) -> Result<(), SnippetParseErr> {
        loop {
            if self.token.typ == TokenType::EOF || self.token.typ == TokenType::Undefined {
                break;
            }
            let si = self.parse_snippet()?;
            self.s.push(si);
        }
        Ok(())
    }

    fn parse_snippet(&mut self) -> Result<SnippetItem, SnippetParseErr> {
        self.parse_escaped()
            .or_else(|_| self.parse_tabstop_vari())
            .or_else(|_| self.parse_placeholder())
            .or_else(|_| self.parse_choice())
            .or_else(|_| self.parse_complex_variable())
            .or_else(|_| self.parse_anything())
    }

    fn parse_escaped(&mut self) -> Result<SnippetItem, SnippetParseErr> {
        if self.accept(TokenType::BackSlash).is_some() {
            match self.token.typ {
                TokenType::BackSlash | TokenType::Dollar | TokenType::CurlyClose => {
                    let t = self.token_text();
                    self.next();
                    Ok(SnippetItem::Text(t))
                }
                _ => Ok(SnippetItem::Text(r#"\"#.to_string())),
            }
        } else {
            Err(SnippetParseErr {})
        }
    }

    fn parse_tabstop_vari(&mut self) -> Result<SnippetItem, SnippetParseErr> {
        let tk = self.token;
        if self.accept(TokenType::Dollar).is_some() {
            if let Some(var_name) = self.accept(TokenType::VariableName) {
                // $foo
                Ok(SnippetItem::Variable(SnippetVariable {
                    var_name,
                    value: vec![],
                }))
            } else if let Some(t) = self.accept(TokenType::Int) {
                // $1
                let ts_idx = t.parse::<usize>().unwrap(); // trust the scanner
                Ok(SnippetItem::TabStop(ts_idx))
            } else {
                self.backto(tk);
                Err(SnippetParseErr {})
            }
        } else {
            self.backto(tk);
            Err(SnippetParseErr {})
        }
    }

    fn parse_choice(&mut self) -> Result<SnippetItem, SnippetParseErr> {
        let tk = self.token;
        let mut choice = if self.accept(TokenType::Dollar).is_some()
            && self.accept(TokenType::CurlyOpen).is_some()
        {
            if let Some(idx) = self.accept(TokenType::Int) {
                let idx = idx.parse::<usize>().unwrap();
                SnippetChoice {
                    idx,
                    choices: vec![],
                }
            } else {
                self.backto(tk);
                return Err(SnippetParseErr {});
            }
        } else {
            self.backto(tk);
            return Err(SnippetParseErr {});
        };

        if self.accept(TokenType::Pipe).is_some() {
            let mut choice_str = String::default();
            loop {
                if self.accept(TokenType::EOF).is_some() {
                    self.backto(tk);
                    return Err(SnippetParseErr {});
                } else if self.accept(TokenType::BackSlash).is_some() {
                    // escape \, \| \}
                    match self.token.typ {
                        TokenType::CurlyClose | TokenType::Pipe | TokenType::Comma => {
                            choice_str += &self.token_text();
                        }
                        _ => {
                            choice_str += r"\";
                            choice_str += &self.token_text();
                        }
                    }
                } else if self.accept(TokenType::Comma).is_some() {
                    if !choice_str.is_empty() {
                        choice.choices.push(choice_str.clone());
                        choice_str.clear();
                    }
                } else if self.accept(TokenType::Pipe).is_some()
                    && self.accept(TokenType::CurlyClose).is_some()
                {
                    if !choice_str.is_empty() {
                        choice.choices.push(choice_str.clone());
                        choice_str.clear();
                    }
                    return Ok(SnippetItem::Choice(choice));
                } else if let Some(tmp) = self.accept(TokenType::Undefined) {
                    choice_str += &tmp;
                } else {
                    self.backto(tk);
                    return Err(SnippetParseErr {});
                }
            }
        } else {
            self.backto(tk);
            Err(SnippetParseErr {})
        }
    }

    fn parse_placeholder(&mut self) -> Result<SnippetItem, SnippetParseErr> {
        // ${1} ${1:any}
        let tk = self.token;
        let mut placeholder = if self.accept(TokenType::Dollar).is_some()
            && self.accept(TokenType::CurlyOpen).is_some()
        {
            if let Some(idx) = self.accept(TokenType::Int) {
                let idx = idx.parse::<usize>().unwrap();
                SnippetPlaceholder { idx, value: vec![] }
            } else {
                self.backto(tk);
                return Err(SnippetParseErr {});
            }
        } else {
            self.backto(tk);
            return Err(SnippetParseErr {});
        };

        if self.accept(TokenType::Colon).is_some() {
            loop {
                if self.accept(TokenType::CurlyClose).is_some() {
                    return Ok(SnippetItem::PlaceHolder(placeholder));
                } else if self.accept(TokenType::EOF).is_some() {
                    self.backto(tk);
                    return Err(SnippetParseErr {});
                }

                if let Ok(item) = self.parse_snippet() {
                    placeholder.value.push(item);
                }
            }
        } else if self.accept(TokenType::CurlyClose).is_some() {
            Ok(SnippetItem::PlaceHolder(placeholder))
        } else {
            self.backto(tk);
            Err(SnippetParseErr {})
        }
    }

    fn parse_complex_variable(&mut self) -> Result<SnippetItem, SnippetParseErr> {
        // ${foo} ${foo:any} ${foo/regex/format/options}
        let tk = self.token;
        let mut variable = if let Some(var_names) = self.accepts(&[
            TokenType::Dollar,
            TokenType::CurlyOpen,
            TokenType::VariableName,
        ]) {
            let var_name = &var_names[2];
            SnippetVariable {
                var_name: var_name.clone(),
                value: vec![],
            }
        } else {
            self.backto(tk);
            return Err(SnippetParseErr {});
        };

        if self.accept(TokenType::Colon).is_some() {
            loop {
                if self.accept(TokenType::CurlyClose).is_some() {
                    return Ok(SnippetItem::Variable(variable));
                } else if self.accept(TokenType::EOF).is_some() {
                    self.backto(tk);
                    return Err(SnippetParseErr {});
                }

                if let Ok(item) = self.parse_snippet() {
                    variable.value.push(item);
                }
            }
        } else if self.accept(TokenType::ForwardSlash).is_some() {
            if let Ok(item) = self.parse_transform() {
                variable.value.push(item);
                Ok(SnippetItem::Variable(variable))
            } else {
                self.backto(tk);
                Err(SnippetParseErr {})
            }
        } else if self.accept(TokenType::CurlyClose).is_some() {
            Ok(SnippetItem::Variable(variable))
        } else {
            self.backto(tk);
            Err(SnippetParseErr {})
        }
    }

    fn parse_transform(&mut self) -> Result<SnippetItem, SnippetParseErr> {
        // regex/format/options
        let tk = self.token;

        // regex
        let mut regex_str = String::default();
        loop {
            if self.accept(TokenType::EOF).is_some() {
                self.backto(tk);
                return Err(SnippetParseErr {});
            } else if let Some(escaped) =
                self.accepts(&[TokenType::BackSlash, TokenType::Undefined])
            {
                match escaped[1].as_str() {
                    "/" => {
                        regex_str += &escaped[1];
                    }
                    _ => {
                        regex_str += &escaped[0];
                        regex_str += &escaped[1];
                    }
                }
            } else if self.accept(TokenType::ForwardSlash).is_some() {
                break;
            } else if let Some(t) = self.accept(TokenType::Undefined) {
                regex_str += &t;
            }
        }
        let mut builder = RegexBuilder::new(&regex_str);

        // format
        let mut format_str = String::default();
        let mut format_item = vec![];

        loop {
            if let Some(escaped) = self.accepts(&[TokenType::BackSlash, TokenType::Undefined]) {
                match escaped[1].as_str() {
                    "\\" | "$" | "}" | "/" => {
                        regex_str += &escaped[1];
                    }
                    _ => {
                        regex_str += &escaped[0];
                        regex_str += &escaped[1];
                    }
                }
            } else if self.accept(TokenType::ForwardSlash).is_some() {
                if !format_str.is_empty() {
                    format_item.push(SnippetFormat::Replacer(format_str.clone()));
                    format_str.clear();
                }
                break;
            } else if self.accept(TokenType::EOF).is_some() {
                self.backto(tk);
                return Err(SnippetParseErr {});
            } else {
                let format_start = self.token;

                let mut append_format_str = || {
                    if !format_str.is_empty() {
                        format_item.push(SnippetFormat::Replacer(format_str.clone()));
                        format_str.clear();
                    }
                };
                if let Some(transfrom_group) = self.accepts(&[
                    TokenType::Dollar,
                    TokenType::CurlyOpen,
                    TokenType::Int,
                    TokenType::Colon,
                ]) {
                    let capture_group = transfrom_group[2].parse::<usize>().unwrap();
                    //${1:
                    if let Some(operation) = self.accepts(&[
                        TokenType::ForwardSlash,
                        TokenType::VariableName,
                        TokenType::CurlyClose,
                    ]) {
                        // ${1:/upcase} /downcase /captilize
                        let format_operation = match operation[1].as_str() {
                            "upcase" => FormatOperation::Upcase,
                            "downcase" => FormatOperation::Downcase,
                            "capitalize" => FormatOperation::Capitalize,
                            _ => {
                                self.backto(tk);
                                return Err(SnippetParseErr {});
                            }
                        };

                        append_format_str();
                        format_item.push(SnippetFormat::Operation(capture_group, format_operation));
                    } else if self.accept(TokenType::Plus).is_some() {
                        // ${1:+if}
                        if let Some(if_value) = self.until(TokenType::CurlyClose) {
                            append_format_str();
                            format_item.push(SnippetFormat::If(
                                capture_group,
                                if_value,
                                String::default(),
                            ));
                        }
                    } else if self.accept(TokenType::QuestionMark).is_some() {
                        // ${1:?if:else}
                        if let (Some(if_value), Some(else_value)) = (
                            self.until(TokenType::Colon),
                            self.until(TokenType::CurlyClose),
                        ) {
                            append_format_str();
                            format_item.push(SnippetFormat::If(
                                capture_group,
                                if_value,
                                else_value,
                            ));
                        }
                    } else if self.accept(TokenType::Dash).is_some() {
                        // ${1:-else}
                        if let Some(else_value) = self.until(TokenType::CurlyClose) {
                            append_format_str();
                            format_item.push(SnippetFormat::If(
                                capture_group,
                                String::default(),
                                else_value,
                            ));
                        }
                    } else {
                        // ${1: Accpet these four tokens as if they were normal text
                        self.backto(format_start);
                        for _ in 0..4 {
                            if let Some(t) = self.accept(TokenType::Undefined) {
                                format_str += &t;
                            }
                        }
                    }
                } else {
                    self.backto(format_start);
                    if let Some(t) = self.accept(TokenType::Undefined) {
                        format_str += &t;
                    }
                }
            }
        }

        if let Some(options_str) = self.until(TokenType::CurlyClose) {
            for opt_ch in options_str.chars() {
                match opt_ch {
                    'i' => {
                        builder.case_insensitive(true);
                    }
                    'm' => {
                        builder.multi_line(true);
                    }
                    's' => {
                        builder.dot_matches_new_line(true);
                    }
                    'U' => {
                        builder.swap_greed(true);
                    }
                    'x' => {
                        builder.ignore_whitespace(true);
                    }
                    _ => {}
                }
            }
        }
        if let Ok(reg) = builder.build() {
            Ok(SnippetItem::Transform(SnippetTransform {
                reg,
                format_item,
            }))
        } else {
            Err(SnippetParseErr {})
        }
    }

    fn parse_anything(&mut self) -> Result<SnippetItem, SnippetParseErr> {
        if self.token.typ != TokenType::EOF {
            if let Some(t) = self.accept(TokenType::Undefined) {
                return Ok(SnippetItem::Text(t));
            }
        }

        Err(SnippetParseErr {})
    }

    pub fn generate_changes(
        &self,
        doc: &Rope,
        sels: &Selection,
    ) -> (SmallVec<[Range; 1]>, Vec<Change>, usize) {
        let mut ranges: SmallVec<[Range; 1]> = smallvec![];
        let mut changes = vec![];
        let cur_pos = sels.primary().head;
        let mut after_pos = cur_pos;
        self.s.iter().for_each(|item| {
            let (mut rngs, mut chgs, l) = item.generate(doc, sels, cur_pos, after_pos);
            after_pos = l;
            ranges.append(&mut rngs);
            changes.append(&mut chgs);
        });
        // println!("generate_changes >> {:?} ==>> {:?}", self.s, changes);
        (ranges, changes, after_pos)
    }
}

// we might need a bit more than just `&Rope` and `&Selection`
// but `&mut Context` seems a bit overkill and make it harder to write unit tests
pub fn transaction_from_snippet(
    doc: &Rope,
    original_selection: &Selection,
    snippet_str: &str,
) -> Transaction {
    let mut parser = SnippetParser::default();
    parser.init(snippet_str);
    if parser.parse().is_ok() {
        let (mut ranges, changes, l) = parser.generate_changes(doc, original_selection);
        ranges.push(Range::new(l, l));
        Transaction::change(doc, changes.into_iter()).with_selection(Selection::new(ranges, 0))
    } else {
        Transaction::change(doc, vec![].into_iter())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_scanner() {
        let mut s = Scanner::default();
        s.text("");
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("abc");
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("{{abc}}");
        assert_eq!(s.next().typ, TokenType::CurlyOpen);
        assert_eq!(s.next().typ, TokenType::CurlyOpen);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::CurlyClose);
        assert_eq!(s.next().typ, TokenType::CurlyClose);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("abc() ");
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::Format);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("abc中文");
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::Format);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("abc 123");
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::Format);
        assert_eq!(s.next().typ, TokenType::Int);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("$foo");
        assert_eq!(s.next().typ, TokenType::Dollar);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("$foo_bar");
        assert_eq!(s.next().typ, TokenType::Dollar);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("$foo-bar");
        assert_eq!(s.next().typ, TokenType::Dollar);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::Dash);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("${foo}");
        assert_eq!(s.next().typ, TokenType::Dollar);
        assert_eq!(s.next().typ, TokenType::CurlyOpen);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::CurlyClose);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("${1223:foo}");
        assert_eq!(s.next().typ, TokenType::Dollar);
        assert_eq!(s.next().typ, TokenType::CurlyOpen);
        assert_eq!(s.next().typ, TokenType::Int);
        assert_eq!(s.next().typ, TokenType::Colon);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::CurlyClose);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("\\${}");
        assert_eq!(s.next().typ, TokenType::BackSlash);
        assert_eq!(s.next().typ, TokenType::Dollar);
        assert_eq!(s.next().typ, TokenType::CurlyOpen);
        assert_eq!(s.next().typ, TokenType::CurlyClose);
        assert_eq!(s.next().typ, TokenType::EOF);

        s.text("${foo/regex/format/option}");
        assert_eq!(s.next().typ, TokenType::Dollar);
        assert_eq!(s.next().typ, TokenType::CurlyOpen);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::ForwardSlash);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::ForwardSlash);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::ForwardSlash);
        assert_eq!(s.next().typ, TokenType::VariableName);
        assert_eq!(s.next().typ, TokenType::CurlyClose);
        assert_eq!(s.next().typ, TokenType::EOF);
    }

    #[test]
    fn test_parser() {
        let mut parser = SnippetParser::default();

        parser.init(r#"$1"#);
        assert!(parser.parse().is_ok());
        assert_eq!(&parser.s, &[SnippetItem::TabStop(1)]);

        parser.init(r#"${1}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::PlaceHolder(SnippetPlaceholder {
                idx: 1,
                value: vec![]
            })]
        );

        parser.init(r#"${1:bar}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::PlaceHolder(SnippetPlaceholder {
                idx: 1,
                value: vec![SnippetItem::Text(String::from("bar"))]
            })]
        );

        parser.init(r#"$foo"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::Variable(SnippetVariable {
                var_name: String::from("foo"),
                value: vec![]
            })]
        );

        parser.init(r#"${foo}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::Variable(SnippetVariable {
                var_name: String::from("foo"),
                value: vec![]
            })]
        );

        parser.init(r#"${foo:bar}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::Variable(SnippetVariable {
                var_name: String::from("foo"),
                value: vec![SnippetItem::Text(String::from("bar"))]
            })]
        );

        parser.init(r#"${1|one,two|}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::Choice(SnippetChoice {
                idx: 1,
                choices: vec![String::from("one"), String::from("two"),]
            })]
        );

        parser.init(r#"${TM_FILENAME/.*/${0:/upcase}/}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::Variable(SnippetVariable {
                var_name: String::from("TM_FILENAME"),
                value: vec![SnippetItem::Transform(SnippetTransform {
                    reg: Regex::new(".*").unwrap(),
                    format_item: vec![SnippetFormat::Operation(0, FormatOperation::Upcase)]
                })]
            })]
        );

        parser.init(r#"${TM_FILENAME/(.*)\.TXT/${1:/downcase}/}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::Variable(SnippetVariable {
                var_name: String::from("TM_FILENAME"),
                value: vec![SnippetItem::Transform(SnippetTransform {
                    reg: Regex::new(r"(.*)\.TXT").unwrap(),
                    format_item: vec![SnippetFormat::Operation(1, FormatOperation::Downcase)]
                })]
            })]
        );

        parser.init(r#"${TM_FILEPATH/.*/${0:/capitalize}/}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::Variable(SnippetVariable {
                var_name: String::from("TM_FILEPATH"),
                value: vec![SnippetItem::Transform(SnippetTransform {
                    reg: Regex::new(".*").unwrap(),
                    format_item: vec![SnippetFormat::Operation(0, FormatOperation::Capitalize)]
                })]
            })]
        );

        parser.init(r#"${TM_FILEPATH/.*/${0:/capitalize}/si}"#);
        assert!(parser.parse().is_ok());
        assert_eq!(
            &parser.s,
            &[SnippetItem::Variable(SnippetVariable {
                var_name: String::from("TM_FILEPATH"),
                value: vec![SnippetItem::Transform(SnippetTransform {
                    reg: Regex::new(".*").unwrap(),
                    format_item: vec![SnippetFormat::Operation(0, FormatOperation::Capitalize)]
                })]
            })]
        );
    }

    #[test]
    fn test_snippet_item() {
        let item = SnippetItem::Text(String::from("bar"));

        let doc = Rope::from("hello");
        let doc_len = doc.len_chars();
        let sels = Selection::new(smallvec![Range::new(doc_len, doc_len)], 0);
        let (_ranges, changes, _l) = item.generate(&doc, &sels, 0, 0);
        assert_eq!(changes.as_slice(), &[(0, 0, Some(Tendril::from("bar")))]);
    }

    #[test]
    fn test_snippet_transaction() {
        let mut doc = Rope::from("hello");
        let doc_len = doc.len_chars();
        let sels = Selection::new(smallvec![Range::new(doc_len, doc_len)], 0);
        let t = transaction_from_snippet(&doc, &sels, "foo");
        assert!(t.apply(&mut doc));
        assert_eq!(doc, "hellofoo");

        let mut doc = Rope::from("hello");
        let sels = Selection::new(smallvec![Range::new(0, 0)], 0);
        let t = transaction_from_snippet(&doc, &sels, "foo");
        assert!(t.apply(&mut doc));
        assert_eq!(doc, "foohello");

        let mut doc = Rope::from("hello");
        let doc_len = doc.len_chars();
        let sels = Selection::new(smallvec![Range::new(doc_len, doc_len)], 0);
        let t = transaction_from_snippet(&doc, &sels, "$1foo");
        assert!(t.apply(&mut doc));
        assert_eq!(doc, "hellofoo");

        let mut doc = Rope::from("hello");
        let doc_len = doc.len_chars();
        let sels = Selection::new(smallvec![Range::new(doc_len, doc_len)], 0);
        let t = transaction_from_snippet(&doc, &sels, "foo$1bar");
        assert!(t.apply(&mut doc));
        assert_eq!(doc, "hellofoobar");

        let mut doc = Rope::from("hello");
        let doc_len = doc.len_chars();
        let sels = Selection::new(smallvec![Range::new(doc_len, doc_len)], 0);
        let t = transaction_from_snippet(&doc, &sels, " foo ${1:bar}");
        assert!(t.apply(&mut doc));
        assert_eq!(doc, "hello foo bar");

        let mut doc = Rope::from("hello");
        let doc_len = doc.len_chars();
        let sels = Selection::new(smallvec![Range::new(0, doc_len)], 0);
        let t = transaction_from_snippet(&doc, &sels, " foo ${TM_SELECTED_TEXT}");
        assert!(t.apply(&mut doc));
        assert_eq!(doc, "hello foo hello");
    }
}
