use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;

#[cfg(test)]
mod test;

struct QueryAtom {
    kind: QueryAtomKind,
    atom: String,
    ignore_case: bool,
    inverse: bool,
}
impl QueryAtom {
    fn new(atom: &str) -> Option<QueryAtom> {
        let mut atom = atom.to_string();
        let inverse = atom.starts_with('!');
        if inverse {
            atom.remove(0);
        }

        let mut kind = match atom.chars().next() {
            Some('^') => QueryAtomKind::Prefix,
            Some('\'') => QueryAtomKind::Substring,
            _ if inverse => QueryAtomKind::Substring,
            _ => QueryAtomKind::Fuzzy,
        };

        if atom.starts_with(['^', '\'']) {
            atom.remove(0);
        }

        if atom.is_empty() {
            return None;
        }

        if atom.ends_with('$') && !atom.ends_with("\\$") {
            atom.pop();
            kind = if kind == QueryAtomKind::Prefix {
                QueryAtomKind::Exact
            } else {
                QueryAtomKind::Postfix
            }
        }

        Some(QueryAtom {
            kind,
            atom: atom.replace('\\', ""),
            // not ideal but fuzzy_matches only knows ascii uppercase so more consistent
            // to behave the same
            ignore_case: kind != QueryAtomKind::Fuzzy
                && atom.chars().all(|c| c.is_ascii_lowercase()),
            inverse,
        })
    }

    fn indices(&self, matcher: &Matcher, item: &str, indices: &mut Vec<usize>) -> bool {
        // for inverse there are no indices to return
        // just return whether we matched
        if self.inverse {
            return self.matches(matcher, item);
        }
        let buf;
        let item = if self.ignore_case {
            buf = item.to_ascii_lowercase();
            &buf
        } else {
            item
        };
        let off = match self.kind {
            QueryAtomKind::Fuzzy => {
                if let Some((_, fuzzy_indices)) = matcher.fuzzy_indices(item, &self.atom) {
                    indices.extend_from_slice(&fuzzy_indices);
                    return true;
                } else {
                    return false;
                }
            }
            QueryAtomKind::Substring => {
                if let Some(off) = item.find(&self.atom) {
                    off
                } else {
                    return false;
                }
            }
            QueryAtomKind::Prefix if item.starts_with(&self.atom) => 0,
            QueryAtomKind::Postfix if item.ends_with(&self.atom) => item.len() - self.atom.len(),
            QueryAtomKind::Exact if item == self.atom => 0,
            _ => return false,
        };

        indices.extend(off..(off + self.atom.len()));
        true
    }

    fn matches(&self, matcher: &Matcher, item: &str) -> bool {
        let buf;
        let item = if self.ignore_case {
            buf = item.to_ascii_lowercase();
            &buf
        } else {
            item
        };
        let mut res = match self.kind {
            QueryAtomKind::Fuzzy => matcher.fuzzy_match(item, &self.atom).is_some(),
            QueryAtomKind::Substring => item.contains(&self.atom),
            QueryAtomKind::Prefix => item.starts_with(&self.atom),
            QueryAtomKind::Postfix => item.ends_with(&self.atom),
            QueryAtomKind::Exact => item == self.atom,
        };
        if self.inverse {
            res = !res;
        }
        res
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum QueryAtomKind {
    /// Item is a fuzzy match of this behaviour
    ///
    /// Usage: `foo`
    Fuzzy,
    /// Item contains query atom as a continuous substring
    ///
    /// Usage `'foo`
    Substring,
    /// Item starts with query atom
    ///
    /// Usage: `^foo`
    Prefix,
    /// Item ends with query atom
    ///
    /// Usage: `foo$`
    Postfix,
    /// Item is equal to query atom
    ///
    /// Usage `^foo$`
    Exact,
}

#[derive(Default)]
pub struct FuzzyQuery {
    first_fuzzy_atom: Option<String>,
    query_atoms: Vec<QueryAtom>,
}

fn query_atoms(query: &str) -> impl Iterator<Item = &str> + '_ {
    let mut saw_backslash = false;
    query.split(move |c| {
        saw_backslash = match c {
            ' ' if !saw_backslash => return true,
            '\\' => true,
            _ => false,
        };
        false
    })
}

impl FuzzyQuery {
    pub fn refine(&self, query: &str, old_query: &str) -> (FuzzyQuery, bool) {
        // TODO: we could be a lot smarter about this
        let new_query = Self::new(query);
        let mut is_refinement = query.starts_with(old_query);

        // if the last atom is an inverse atom adding more text to it
        // will actually increase the number of matches and we can not refine
        // the matches.
        if is_refinement && !self.query_atoms.is_empty() {
            let last_idx = self.query_atoms.len() - 1;
            if self.query_atoms[last_idx].inverse
                && self.query_atoms[last_idx].atom != new_query.query_atoms[last_idx].atom
            {
                is_refinement = false;
            }
        }

        (new_query, is_refinement)
    }

    pub fn new(query: &str) -> FuzzyQuery {
        let mut first_fuzzy_query = None;
        let query_atoms = query_atoms(query)
            .filter_map(|atom| {
                let atom = QueryAtom::new(atom)?;
                if atom.kind == QueryAtomKind::Fuzzy && first_fuzzy_query.is_none() {
                    first_fuzzy_query = Some(atom.atom);
                    None
                } else {
                    Some(atom)
                }
            })
            .collect();
        FuzzyQuery {
            first_fuzzy_atom: first_fuzzy_query,
            query_atoms,
        }
    }

    pub fn fuzzy_match(&self, item: &str, matcher: &Matcher) -> Option<i64> {
        // use the rank of the first fuzzzy query for the rank, because merging ranks is not really possible
        // this behaviour matches fzf and skim
        let score = self
            .first_fuzzy_atom
            .as_ref()
            .map_or(Some(0), |atom| matcher.fuzzy_match(item, atom))?;
        if self
            .query_atoms
            .iter()
            .any(|atom| !atom.matches(matcher, item))
        {
            return None;
        }
        Some(score)
    }

    pub fn fuzzy_indices(&self, item: &str, matcher: &Matcher) -> Option<(i64, Vec<usize>)> {
        let (score, mut indices) = self.first_fuzzy_atom.as_ref().map_or_else(
            || Some((0, Vec::new())),
            |atom| matcher.fuzzy_indices(item, atom),
        )?;

        // fast path for the common case of just a single atom
        if self.query_atoms.is_empty() {
            return Some((score, indices));
        }

        for atom in &self.query_atoms {
            if !atom.indices(matcher, item, &mut indices) {
                return None;
            }
        }

        // deadup and remove duplicate matches
        indices.sort_unstable();
        indices.dedup();

        Some((score, indices))
    }
}
