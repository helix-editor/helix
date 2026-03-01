#[cfg(feature = "steel")]
pub mod steel_implementations {

    use std::{borrow::Cow, collections::BTreeMap, ffi::c_void, ptr::NonNull, sync::Arc};

    use regex_cursor::regex_automata::util::syntax::Config;

    use ropey::RopeSlice;
    use steel::{
        gc::ShareableMut,
        rvals::{as_underlying_type, AsRefSteelVal, Custom, SteelString},
        steel_vm::{
            builtin::{BuiltInModule, MarkdownDoc},
            register_fn::RegisterFn,
        },
        SteelErr, SteelVal,
    };

    use helix_stdx::rope::{Regex, RegexBuilder, RopeSliceExt};
    use tree_house::{
        query_iter::QueryIterEvent,
        tree_sitter::{Grammar, Node, Query, Tree},
        Language, Layer,
    };

    use crate::{
        syntax::{
            self,
            config::{AutoPairConfig, SoftWrap},
            pretty_print_tree,
        },
        Range, Syntax,
    };

    impl steel::rvals::Custom for crate::Position {}
    impl steel::rvals::Custom for crate::Selection {}
    impl steel::rvals::Custom for AutoPairConfig {}
    impl steel::rvals::Custom for SoftWrap {}

    struct SteelRopeRegex(Regex);

    #[allow(unused)]
    #[derive(Debug)]
    struct RegexError(String);

    impl steel::rvals::Custom for SteelRopeRegex {}

    pub struct TreeSitterMatch {
        captures: BTreeMap<String, Vec<TreeSitterNode>>,
    }

    impl TreeSitterMatch {
        pub fn new(captures: BTreeMap<String, Vec<TreeSitterNode>>) -> Self {
            TreeSitterMatch { captures }
        }
        pub fn get_capture(&self, capture: String) -> Option<Vec<TreeSitterNode>> {
            self.captures.get(&capture).cloned()
        }

        pub fn get_captures(&self) -> Vec<String> {
            let mut ret = vec![];
            for (k, _) in self.captures.iter() {
                ret.push(k.to_string());
            }
            ret
        }
    }
    impl steel::rvals::Custom for TreeSitterMatch {}

    #[derive(Clone, Debug)]
    pub struct TreeSitterQuery {
        inner: Arc<Query>,
    }

    impl TreeSitterQuery {
        pub fn get_inner(&self) -> &Arc<Query> {
            &self.inner
        }

        pub fn new(grammar: Grammar, source: &str) -> Result<Self, SteelErr> {
            Query::new(grammar, source, |_, _| Ok(()))
                .map(|q| TreeSitterQuery { inner: Arc::new(q) })
                .map_err(|e| SteelErr::new(steel::rerrs::ErrorKind::Generic, e.to_string()))
        }
    }
    impl steel::rvals::Custom for TreeSitterQuery {}

    #[derive(Clone, Debug)]
    pub struct TreeSitterQueryLoader {
        fun: SteelVal,
    }

    impl TreeSitterQueryLoader {
        pub fn new(fun: SteelVal) -> Result<Self, String> {
            if !fun.is_function() {
                return Err("Bad value!".into());
            }
            let SteelVal::BoxedFunction(func) = &fun else {
                return Err("Not a boxed function".into());
            };
            // arity *must* be one
            if func.get_arity().unwrap_or(1) == 1 {
                return Ok(TreeSitterQueryLoader { fun });
            } else {
                return Err(format!("Bad Arity: {}", func.arity.unwrap()));
            }
        }

        pub fn load(&self, lang: &SteelVal) -> Result<Option<TreeSitterQuery>, SteelErr> {
            let fun = self.fun.clone();
            let SteelVal::BoxedFunction(func) = &fun else {
                return Err(SteelErr::new(
                    steel::rerrs::ErrorKind::TypeMismatch,
                    "unable to get boxed function".into(),
                ));
            };

            let val = match (func.function)(&vec![lang.clone()]) {
                Ok(f) => f,
                Err(e) => return Err(e),
            };

            let SteelVal::Custom(custom) = val else {
                return Ok(None);
            };

            let tsquery =
                steel::rvals::as_underlying_type::<TreeSitterQuery>(custom.write().as_ref())
                    .map(|t| t.clone());

            return Ok(tsquery);
        }
    }

    impl steel::rvals::Custom for TreeSitterQueryLoader {}

    #[derive(Clone)]
    pub struct TreeSitterSyntax {
        inner: Arc<Syntax>,
    }

    impl steel::rvals::Custom for TreeSitterSyntax {}

    impl TreeSitterSyntax {
        pub fn new(
            source: SteelRopeSlice,
            language: Language,
            loader: &crate::syntax::Loader,
        ) -> Result<Self, SteelErr> {
            Syntax::new(source.to_slice(), language, loader)
                .map(|syn| Self {
                    inner: Arc::new(syn),
                })
                .map_err(|e| SteelErr::new(steel::rerrs::ErrorKind::Generic, e.to_string()))
        }

        pub fn get_inner(&self) -> &Arc<Syntax> {
            &self.inner
        }

        pub fn get_tree_from_range(&self, lower: u32, upper: u32) -> Option<TreeSitterTree> {
            let layer = self.get_inner().layer_for_byte_range(lower, upper);
            let lang = self.get_inner().layer(layer).language;
            self.get_inner()
                .layer(layer)
                .tree()
                .map(|l| TreeSitterTree::new(l, lang))
        }

        pub fn get_tree(&self) -> TreeSitterTree {
            let lang = self.get_inner().root_language();
            TreeSitterTree::new(self.get_inner().tree(), lang)
        }

        pub fn get_trees_byte_range(syn: &Syntax, lower: u32, upper: u32) -> Vec<TreeSitterTree> {
            let layers = syn.layers_for_byte_range(lower, upper);

            layers
                .map(|layer| {
                    let l = syn.layer(layer);
                    let tree = l.tree().unwrap();
                    TreeSitterTree::new(tree, l.language)
                })
                .collect()
        }

        pub fn run_query(
            syn: &Syntax,
            loader: &syntax::Loader,
            query_loader: TreeSitterQueryLoader,
            source: RopeSlice,
            lower: u32,
            upper: u32,
        ) -> Result<TreeSitterMatch, SteelErr> {
            let mut query_map = BTreeMap::new();
            for layer in syn.layers_for_byte_range(lower, upper) {
                let lang = syn.layer(layer).language;
                let val = SteelVal::StringV(SteelString::from(
                    loader.language(lang).config().language_id.clone(),
                ));
                let loaded = match query_loader.load(&val) {
                    Ok(l) => l,
                    Err(e) => return Err(e),
                };

                if loaded.is_some() {
                    query_map.insert(lang, loaded.unwrap());
                }
            }
            let mut captures: BTreeMap<String, Vec<TreeSitterNode>> = BTreeMap::new();
            let mut layers: Vec<(Layer, TreeSitterTree)> = vec![];
            let load = |lang| {
                return query_map.get(&lang).map(|q| q.get_inner().as_ref());
            };

            for event in syn.query_iter::<_, (), _>(source, load, lower..upper) {
                let QueryIterEvent::Match(m) = event else {
                    continue;
                };
                let layer = syn.layer_for_byte_range(m.node.start_byte(), m.node.end_byte());
                let lang = syn.layer(layer).language;

                let tree = match layers.iter().position(|(l, _)| l == &layer) {
                    Some(idx) => &layers[idx].1,
                    None => {
                        let new_tree = syn.layer(layer).tree().unwrap();
                        let t = TreeSitterTree::new(new_tree, lang);

                        layers.push((layer, t));
                        &layers.last().unwrap().1
                    }
                };

                let capture_name = query_map
                    .get(&lang)
                    .unwrap()
                    .get_inner()
                    .capture_name(m.capture)
                    .to_string();

                if captures.contains_key(&capture_name) {
                    captures
                        .entry(capture_name)
                        .and_modify(|e| e.push(TreeSitterNode::new(m.node, tree)));
                    continue;
                }
                captures.insert(capture_name, vec![TreeSitterNode::new(m.node, tree)]);
            }
            Ok(TreeSitterMatch { captures })
        }
    }

    #[derive(Clone)]
    pub struct TreeSitterTree {
        inner: Arc<Tree>,
        language: Language,
    }
    impl Custom for TreeSitterTree {
        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<TreeSitterTree>(other) {
                self.get_inner().root_node().id() == other.get_inner().root_node().id()
            } else {
                false
            }
        }
    }

    impl TreeSitterTree {
        pub fn new(inner: &Tree, language: Language) -> Self {
            Self {
                inner: Arc::new(inner.clone()),
                language,
            }
        }
        pub fn get_root(&self) -> TreeSitterNode {
            let node = self.inner.root_node();
            let extended = unsafe { std::mem::transmute::<_, Node<'static>>(node) };
            TreeSitterNode::new(extended, self)
        }

        pub fn get_inner(&self) -> &Arc<Tree> {
            &self.inner
        }

        pub fn get_language(&self) -> Language {
            self.language
        }
    }

    #[derive(Clone)]
    pub struct TreeSitterNode {
        inner: Node<'static>,
        // reference to keep tree alive
        _tree: Arc<Tree>,
        // language
        _lang: Language,
    }

    impl Custom for TreeSitterNode {
        fn fmt(&self) -> Option<std::result::Result<String, std::fmt::Error>> {
            if self.is_visible() {
                return Some(Ok(format!("#TSNode<({:?})>", self.kind())));
            }

            Some(Ok(format!(
                "#TSNode<\"{}\">",
                self.kind().replace('"', "\\\"")
            )))
        }

        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<TreeSitterNode>(other) {
                self.inner.id() == other.inner.id()
            } else {
                false
            }
        }

        fn equality_hint_general(&self, other: &steel::SteelVal) -> bool {
            match other {
                SteelVal::Custom(c) => Self::equality_hint(self, c.read().as_ref()),

                _ => false,
            }
        }
    }

    impl TreeSitterNode {
        pub fn print_tree(&self) -> String {
            let mut output = String::new();
            let _ = pretty_print_tree(&mut output, self.inner.clone());
            return output;
        }
        pub fn new(node: Node<'_>, t: &TreeSitterTree) -> TreeSitterNode {
            // rely on the fact that when copying the treesitter tree, there is refcounting (we keep it alive :D): https://github.com/tree-sitter/tree-sitter/blob/630fa52717f2c575a53e21b1d324ade8e528b0bd/lib/src/tree.c#L24

            // HACK: move ownership from the original tree to the new one
            // this is unsafe but we can assume that if this function is being called that the node in fact belongs to the given tree
            let extended: Node<'static> = unsafe {
                #[repr(C)]
                struct PubNode {
                    pub context: [u32; 4],
                    pub id: NonNull<c_void>,
                    pub tree: NonNull<c_void>,
                }
                let mut r: PubNode = std::mem::transmute(node);
                let root: PubNode = std::mem::transmute(t.get_inner().root_node());
                r.tree = root.tree;
                std::mem::transmute(r)
            };
            return TreeSitterNode {
                inner: extended,
                _tree: Arc::clone(t.get_inner()),
                _lang: t.get_language(),
            };
        }

        fn new_internal(&self, node: Node<'static>) -> TreeSitterNode {
            Self {
                inner: node,
                _tree: self._tree.clone(),
                _lang: self._lang,
            }
        }

        pub fn get_tree(&self) -> TreeSitterTree {
            TreeSitterTree::new(&self._tree, self._lang)
        }

        pub fn parent(&self) -> Option<TreeSitterNode> {
            self.inner.parent().map(|n| self.new_internal(n))
        }

        pub fn children(&self) -> Vec<TreeSitterNode> {
            let mut ret: Vec<TreeSitterNode> = vec![];
            for child in self.inner.children() {
                ret.push(self.new_internal(child));
            }
            ret
        }

        pub fn named_children(&self) -> Vec<TreeSitterNode> {
            let mut ret: Vec<TreeSitterNode> = vec![];
            for i in 0..self.inner.named_child_count() {
                if let Some(c) = self.inner.named_child(i) {
                    ret.push(self.new_internal(c))
                }
            }
            ret
        }

        pub fn is_contained_within_byte_range(&self, lower: u32, upper: u32) -> bool {
            self.inner.is_contained_within(lower..upper)
        }

        pub fn named_descendant_byte_range(
            &self,
            lower: u32,
            upper: u32,
        ) -> Option<TreeSitterNode> {
            self.inner
                .named_descendant_for_byte_range(lower, upper)
                .map(|n| self.new_internal(n))
        }

        pub fn descendant_byte_range(&self, lower: u32, upper: u32) -> Option<TreeSitterNode> {
            self.inner
                .descendant_for_byte_range(lower, upper)
                .map(|n| self.new_internal(n))
        }

        pub fn kind(&self) -> SteelString {
            self.inner.kind().into()
        }

        pub fn is_named(&self) -> bool {
            self.inner.is_named()
        }

        pub fn is_extra(&self) -> bool {
            self.inner.is_extra()
        }
        pub fn is_missing(&self) -> bool {
            self.inner.is_missing()
        }

        pub fn start_byte(&self) -> u32 {
            self.inner.start_byte()
        }

        fn is_visible(&self) -> bool {
            self.is_missing()
                || (self.is_named()
                    && self
                        .inner
                        .grammar()
                        .node_kind_is_visible(self.inner.kind_id()))
        }

        pub fn end_byte(&self) -> u32 {
            self.inner.end_byte()
        }
    }

    impl steel::rvals::Custom for RegexError {
        fn fmt(&self) -> Option<std::result::Result<String, std::fmt::Error>> {
            Some(Ok(format!("{:?}", self.0)))
        }
    }

    impl From<String> for RegexError {
        fn from(value: String) -> Self {
            Self(value)
        }
    }

    impl SteelRopeRegex {
        fn new(re: SteelString) -> Result<Self, RegexError> {
            match RegexBuilder::new().syntax(Config::new()).build(re.as_str()) {
                Ok(regex) => Ok(SteelRopeRegex(regex)),
                Err(err) => Err(RegexError(err.to_string())),
            }
        }

        fn is_match(&self, haystack: SteelRopeSlice) -> bool {
            match self.0.find(haystack.to_slice().regex_input()) {
                Some(m) => m.start() != m.end(),
                None => false,
            }
        }

        fn find(&self, haystack: SteelRopeSlice) -> Option<SteelRopeSlice> {
            match self.0.find(haystack.to_slice().regex_input()) {
                Some(m) => {
                    if m.start() == m.end() {
                        None
                    } else {
                        haystack.slice(m.start(), m.end()).ok()
                    }
                }
                None => None,
            }
        }

        pub fn find_all(&self, haystack: SteelRopeSlice) -> Option<Vec<SteelRopeSlice>> {
            let matches = self.0.find_iter(haystack.to_slice().regex_input());
            let mut ret: Vec<SteelRopeSlice> = vec![];
            for m in matches {
                if m.start() == m.end() {
                    continue;
                }
                let s = haystack.clone().slice(m.start(), m.end());
                if let Ok(slice) = s {
                    ret.push(slice);
                }
            }
            Some(ret)
        }

        pub fn split(&self, haystack: SteelRopeSlice) -> Option<Vec<SteelRopeSlice>> {
            let matches = self.0.split(haystack.to_slice().regex_input());
            let mut ret: Vec<SteelRopeSlice> = vec![];
            for m in matches {
                if m.start == m.end {
                    continue;
                }
                let s = haystack.clone().slice(m.start, m.end);
                if let Ok(slice) = s {
                    ret.push(slice);
                }
            }
            Some(ret)
        }

        pub fn splitn(&self, haystack: SteelRopeSlice, n: usize) -> Option<Vec<SteelRopeSlice>> {
            let matches = self.0.splitn(haystack.to_slice().regex_input(), n);
            let mut ret: Vec<SteelRopeSlice> = vec![];
            for m in matches {
                if m.start == m.end {
                    continue;
                }
                let s = haystack.clone().slice(m.start, m.end);
                if let Ok(slice) = s {
                    ret.push(slice);
                }
            }
            Some(ret)
        }
    }

    impl steel::rvals::Custom for Range {}

    #[allow(unused)]
    pub struct RopeyError(ropey::Error);

    impl steel::rvals::Custom for RopeyError {}

    impl From<ropey::Error> for RopeyError {
        fn from(value: ropey::Error) -> Self {
            Self(value)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RangeKind {
        Char,
        Byte,
    }

    #[derive(Clone, PartialEq, Eq)]
    pub struct SteelRopeSlice {
        text: crate::Rope,
        start: usize,
        end: usize,
        kind: RangeKind,
    }

    impl Custom for SteelRopeSlice {
        // `equal?` on two ropes should return true if they are the same
        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<SteelRopeSlice>(other) {
                self == other
            } else {
                false
            }
        }

        fn equality_hint_general(&self, other: &steel::SteelVal) -> bool {
            match other {
                SteelVal::StringV(s) => self.to_slice() == s.as_str(),
                SteelVal::Custom(c) => Self::equality_hint(self, c.read().as_ref()),

                _ => false,
            }
        }

        fn fmt(&self) -> Option<std::result::Result<String, std::fmt::Error>> {
            Some(Ok(format!("#<Rope:\"{}\">", self.to_slice())))
        }
    }

    impl std::fmt::Display for SteelRopeSlice {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.to_slice().fmt(f)
        }
    }

    impl SteelRopeSlice {
        pub fn from_string(string: SteelString) -> Self {
            Self {
                text: crate::Rope::from_str(string.as_str()),
                start: 0,
                end: string.len(),
                kind: RangeKind::Char,
            }
        }

        pub fn new(rope: crate::Rope) -> Self {
            let end = rope.len_chars();
            Self {
                text: rope,
                start: 0,
                end,
                kind: RangeKind::Char,
            }
        }

        pub fn to_slice(&self) -> crate::RopeSlice<'_> {
            match self.kind {
                RangeKind::Char => self.text.slice(self.start..self.end),
                RangeKind::Byte => self.text.byte_slice(self.start..self.end),
            }
        }

        pub fn insert_str(&self, char_idx: usize, text: SteelString) -> Result<Self, RopeyError> {
            let slice = self.to_slice();
            let mut rope = ropey::Rope::from(slice);
            rope.try_insert(char_idx, &text)?;
            Ok(Self::new(rope))
        }

        pub fn insert_char(&self, char_idx: usize, c: char) -> Result<Self, RopeyError> {
            let slice = self.to_slice();
            let mut rope = ropey::Rope::from(slice);
            rope.try_insert_char(char_idx, c)?;
            Ok(Self::new(rope))
        }

        pub fn try_line_to_char(&self, line: usize) -> Result<usize, RopeyError> {
            self.to_slice().try_line_to_char(line).map_err(RopeyError)
        }

        pub fn try_line_to_byte(&self, line: usize) -> Result<usize, RopeyError> {
            self.to_slice().try_line_to_byte(line).map_err(RopeyError)
        }

        pub fn try_char_to_line(&self, line: usize) -> Result<usize, RopeyError> {
            self.to_slice().try_char_to_line(line).map_err(RopeyError)
        }

        pub fn try_byte_to_line(&self, line: usize) -> Result<usize, RopeyError> {
            self.to_slice().try_byte_to_line(line).map_err(RopeyError)
        }

        pub fn line(mut self, cursor: usize) -> Result<Self, RopeyError> {
            match self.kind {
                RangeKind::Char => {
                    let slice = self.text.get_slice(self.start..self.end).ok_or(RopeyError(
                        ropey::Error::CharIndexOutOfBounds(self.start, self.end),
                    ))?;

                    // Move the start range, to wherever this lines up
                    let index = slice.try_line_to_char(cursor)?;

                    let line = slice.get_line(cursor).ok_or(RopeyError(
                        ropey::Error::LineIndexOutOfBounds(cursor, slice.len_lines()),
                    ))?;

                    self.start += index;
                    self.end = self.start + line.len_chars();

                    Ok(self)
                }
                RangeKind::Byte => {
                    let slice =
                        self.text
                            .get_byte_slice(self.start..self.end)
                            .ok_or(RopeyError(ropey::Error::ByteIndexOutOfBounds(
                                self.start, self.end,
                            )))?;

                    // Move the start range, to wherever this lines up
                    let index = slice.try_line_to_byte(cursor)?;

                    let line = slice.get_line(cursor).ok_or(RopeyError(
                        ropey::Error::LineIndexOutOfBounds(cursor, slice.len_lines()),
                    ))?;

                    self.start += index;
                    self.end = self.start + line.len_bytes();

                    Ok(self)
                }
            }
        }

        pub fn slice(mut self, lower: usize, upper: usize) -> Result<Self, RopeyError> {
            match self.kind {
                RangeKind::Char => {
                    self.end = self.start + upper;
                    self.start += lower;

                    // Just check that this is legal
                    self.text.get_slice(self.start..self.end).ok_or(RopeyError(
                        ropey::Error::CharIndexOutOfBounds(self.start, self.end),
                    ))?;

                    Ok(self)
                }
                RangeKind::Byte => {
                    self.start = self.text.try_byte_to_char(self.start)? + lower;
                    self.end = self.start + (upper - lower);

                    self.text
                        .get_byte_slice(self.start..self.end)
                        .ok_or(RopeyError(ropey::Error::ByteIndexOutOfBounds(
                            self.start, self.end,
                        )))?;

                    self.kind = RangeKind::Char;
                    Ok(self)
                }
            }
        }

        pub fn byte_slice(mut self, lower: usize, upper: usize) -> Result<Self, RopeyError> {
            match self.kind {
                RangeKind::Char => {
                    self.start = self.text.try_char_to_byte(self.start)? + lower;
                    self.end = self.start + (upper - lower);
                    self.kind = RangeKind::Byte;

                    // Just check that this is legal
                    self.text.get_slice(self.start..self.end).ok_or(RopeyError(
                        ropey::Error::CharIndexOutOfBounds(self.start, self.end),
                    ))?;

                    Ok(self)
                }
                RangeKind::Byte => {
                    self.start += lower;
                    self.end = self.start + (upper - lower);

                    self.text
                        .get_byte_slice(self.start..self.end)
                        .ok_or(RopeyError(ropey::Error::ByteIndexOutOfBounds(
                            self.start, self.end,
                        )))?;

                    Ok(self)
                }
            }
        }

        pub fn char_to_byte(&self, pos: usize) -> Result<usize, RopeyError> {
            Ok(self.to_slice().try_char_to_byte(pos)?)
        }

        pub fn byte_to_char(&self, pos: usize) -> Result<usize, RopeyError> {
            Ok(self.to_slice().try_byte_to_char(pos)?)
        }

        pub fn len_chars(&self) -> usize {
            self.to_slice().len_chars()
        }

        pub fn len_bytes(&self) -> usize {
            self.to_slice().len_bytes()
        }

        pub fn get_char(&self, index: usize) -> Option<char> {
            self.to_slice().get_char(index)
        }

        pub fn len_lines(&self) -> usize {
            self.to_slice().len_lines()
        }

        pub fn trim_start(mut self) -> Self {
            let slice = self.to_slice();

            for (idx, c) in slice.chars().enumerate() {
                if !c.is_whitespace() {
                    match self.kind {
                        RangeKind::Char => {
                            self.start += idx;
                        }
                        RangeKind::Byte => {
                            self.start += slice.char_to_byte(idx);
                        }
                    }

                    break;
                }
            }

            self
        }

        pub fn starts_with(&self, pat: SteelString) -> bool {
            self.to_slice().starts_with(pat.as_str())
        }

        pub fn ends_with(&self, pat: SteelString) -> bool {
            self.to_slice().ends_with(pat.as_str())
        }
    }

    pub fn rope_module() -> BuiltInModule {
        let mut module = BuiltInModule::new("helix/core/text");

        macro_rules! register_value {
            ($name:expr, $func:expr, $doc:expr) => {
                module.register_fn($name, $func);
                module.register_doc($name, MarkdownDoc(Cow::Borrowed($doc)));
            };
        }

        register_value!(
            "Rope?",
            |value: SteelVal| SteelRopeSlice::as_ref(&value).is_ok(),
            "Check if the given value is a rope"
        );

        register_value!(
            "string->rope",
            SteelRopeSlice::from_string,
            r#"Converts a string into a rope.

```scheme
(string->rope value) -> Rope?
```

* value : string?
            "#
        );

        register_value!(
            "RopeRegex?",
            |value: SteelVal| SteelRopeRegex::as_ref(&value).is_ok(),
            "Check if the given value is a rope regex"
        );
        register_value!(
            "rope-regex",
            SteelRopeRegex::new,
            r#"Build a new RopeRegex? with a string

```scheme
(rope-regex string) -> RopeRegex?
```

* string: string?
            "#
        );
        register_value!(
            "rope-regex-find",
            SteelRopeRegex::find,
            r#"Find the first match in a given rope

```scheme
(rope-regex-find regex rope) -> Rope?
```

* regex: RopeRegex?
* rope: Rope?
            "#
        );
        register_value!(
            "rope-regex-match?",
            SteelRopeRegex::is_match,
            r#"Returns if a regex is matching on a given rope

```scheme
(rope-regex->match? regex rope) -> bool?
```

* regex: RopeRegex?
* rope: Rope?
            "#
        );
        register_value!(
            "rope-regex-find*",
            SteelRopeRegex::find_all,
            r#"Find and return all matches in a given rope

```scheme
(rope-regex-find* regex rope) -> '(Rope?)
```
* regex: RopeRegex?
* rope: Rope?
            "#
        );
        register_value!(
            "rope-regex-split",
            SteelRopeRegex::split,
            r#"Split on the match in a given rope

```scheme
(rope-regex-split regex rope) -> '(Rope?)
```

* regex: RopeRegex?
* rope: Rope?
"#
        );
        register_value!(
            "rope-regex-splitn",
            SteelRopeRegex::splitn,
            r#"Split n times on the match in a given rope, return the rest

```scheme
(rope-regex-splitn regex rope n) -> '(Rope?)
```

* regex: RopeRegex?
* rope: Rope?
* n: (and positive? int?)
"#
        );

        register_value!(
            "rope->slice",
            SteelRopeSlice::slice,
            r#"Take a slice from using character indices from the rope.
Returns a new rope value.

```scheme
(rope->slice rope start end) -> Rope?
```

* rope : Rope?
* start: (and positive? int?)
* end: (and positive? int?)
"#
        );

        register_value!(
            "rope-char->byte",
            SteelRopeSlice::char_to_byte,
            r#"Convert the character offset into a byte offset for a given rope"#
        );

        register_value!(
            "rope-byte->char",
            SteelRopeSlice::byte_to_char,
            r#"Convert the byte offset into a character offset for a given rope"#
        );

        register_value!(
            "rope-line->char",
            SteelRopeSlice::try_line_to_char,
            r#"Convert the given line index to a character offset for a given rope

```scheme
(rope-line->char rope line-offset) -> int?
```

* rope : Rope?
* line-offset: int?
            "#
        );

        register_value!(
            "rope-line->byte",
            SteelRopeSlice::try_line_to_byte,
            r#"Convert the given line index to a byte offset for a given rope

```scheme
(rope-line->byte rope line-offset) -> int?
```

* rope : Rope?
* line-offset: int?
            "#
        );

        register_value!(
            "rope-char->line",
            SteelRopeSlice::try_char_to_line,
            r#"Convert the given character offset to a line offset for a given rope

```scheme
(rope-char->line rope char-index) -> int?
```

* rope : Rope?
* char-index : int?

            "#
        );

        register_value!(
            "rope-byte->line",
            SteelRopeSlice::try_byte_to_line,
            r#"Convert the given byte offset to a line offset for a given rope

```scheme
(rope-byte->line rope byte-index) -> int?
```

* rope : Rope?
* byte-index : int?

            "#
        );

        register_value!(
            "rope->byte-slice",
            SteelRopeSlice::byte_slice,
            r#"Take a slice of this rope using byte offsets

```scheme
(rope->byte-slice rope start end) -> Rope?
```

* rope: Rope?
* start: (and positive? int?)
* end: (and positive? int?)
"#
        );

        register_value!(
            "rope->line",
            SteelRopeSlice::line,
            r#"Get the line at the given line index. Returns a rope.

```scheme
(rope->line rope index) -> Rope?

```

* rope : Rope?
* index : (and positive? int?)
"#
        );

        register_value!(
            "rope->string",
            SteelRopeSlice::to_string,
            "Convert the given rope to a string"
        );

        register_value!(
            "rope-len-chars",
            SteelRopeSlice::len_chars,
            "Get the length of the rope in characters"
        );
        register_value!(
            "rope-len-bytes",
            SteelRopeSlice::len_chars,
            "Get the length of the rope in bytes"
        );

        register_value!(
            "rope-char-ref",
            SteelRopeSlice::get_char,
            "Get the character at the given index"
        );

        register_value!(
            "rope-len-lines",
            SteelRopeSlice::len_lines,
            "Get the number of lines in the rope"
        );

        register_value!(
            "rope-starts-with?",
            SteelRopeSlice::starts_with,
            "Check if the rope starts with a given pattern"
        );

        register_value!(
            "rope-ends-with?",
            SteelRopeSlice::ends_with,
            "Check if the rope ends with a given pattern"
        );

        register_value!(
            "rope-trim-start",
            SteelRopeSlice::trim_start,
            "Remove the leading whitespace from the given rope"
        );

        register_value!(
            "rope-insert-string",
            SteelRopeSlice::insert_str,
            "Insert a string at the given index into the rope"
        );

        register_value!(
            "rope-insert-char",
            SteelRopeSlice::insert_char,
            "Insert a character at the given index"
        );

        module
    }

    pub fn treesitter_module() -> BuiltInModule {
        let mut module = BuiltInModule::new("helix/core/treesitter");
        module
            .register_fn("TSTree?", |value: SteelVal| {
                TreeSitterTree::as_ref(&value).is_ok()
            })
            .register_fn("TSNode?", |value: SteelVal| {
                TreeSitterNode::as_ref(&value).is_ok()
            })
            .register_fn("TSQueryLoader?", |value: SteelVal| {
                TreeSitterQueryLoader::as_ref(&value).is_ok()
            })
            .register_fn("TSSyntax?", |value: SteelVal| {
                TreeSitterSyntax::as_ref(&value).is_ok()
            })
            .register_fn("TSQuery?", |value: SteelVal| {
                TreeSitterQuery::as_ref(&value).is_ok()
            })
            .register_fn("TSMatch?", |value: SteelVal| {
                TreeSitterMatch::as_ref(&value).is_ok()
            });

        module.register_fn("tsquery-loader", TreeSitterQueryLoader::new);
        module.register_fn("tstree->root", TreeSitterTree::get_root);

        module
            .register_fn("tsnode->tstree", TreeSitterNode::get_tree)
            .register_fn("tsnode-parent", TreeSitterNode::parent)
            .register_fn("tsnode-children", TreeSitterNode::children)
            .register_fn("tsnode-named-children", TreeSitterNode::named_children)
            .register_fn(
                "tsnode-within-byte-range?",
                TreeSitterNode::is_contained_within_byte_range,
            )
            .register_fn(
                "tsnode-descendant-byte-range",
                TreeSitterNode::descendant_byte_range,
            )
            .register_fn(
                "tsnode-named-descendant-byte-range",
                TreeSitterNode::named_descendant_byte_range,
            )
            .register_fn("tsnode-kind", TreeSitterNode::kind)
            .register_fn("tsnode-named?", TreeSitterNode::is_named)
            .register_fn("tsnode-extra?", TreeSitterNode::is_extra)
            .register_fn("tsnode-missing?", TreeSitterNode::is_missing)
            .register_fn("tsnode-visible?", TreeSitterNode::is_visible)
            .register_fn("tsnode-print-tree", TreeSitterNode::print_tree)
            .register_fn("tsnode-end-byte", TreeSitterNode::end_byte)
            .register_fn("tsnode-start-byte", TreeSitterNode::start_byte);

        module
            .register_fn("tsmatch-captures", TreeSitterMatch::get_captures)
            .register_fn("tsmatch-capture", TreeSitterMatch::get_capture);

        module
            .register_fn(
                "tssyntax->tree-byte-range",
                TreeSitterSyntax::get_tree_from_range,
            )
            .register_fn(
                "tssyntax->layers-byte-range",
                |syn: TreeSitterSyntax, lower: u32, upper: u32| -> Vec<TreeSitterTree> {
                    TreeSitterSyntax::get_trees_byte_range(&syn.get_inner(), lower, upper)
                },
            )
            .register_fn("tssyntax->tree", TreeSitterSyntax::get_tree);

        module
    }
}
