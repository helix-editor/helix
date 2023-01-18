use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;

#[cfg(test)]
mod test;

pub struct FuzzyQuery {
    queries: Vec<String>,
}

impl FuzzyQuery {
    pub fn new(query: &str) -> FuzzyQuery {
        let mut saw_backslash = false;
        let queries = query
            .split(|c| {
                saw_backslash = match c {
                    ' ' if !saw_backslash => return true,
                    '\\' => true,
                    _ => false,
                };
                false
            })
            .filter_map(|query| {
                if query.is_empty() {
                    None
                } else {
                    Some(query.replace("\\ ", " "))
                }
            })
            .collect();
        FuzzyQuery { queries }
    }

    pub fn fuzzy_match(&self, item: &str, matcher: &Matcher) -> Option<i64> {
        // use the rank of the first query for the rank, because merging ranks is not really possible
        // this behaviour matches fzf and skim
        let score = matcher.fuzzy_match(item, self.queries.get(0)?)?;
        if self
            .queries
            .iter()
            .any(|query| matcher.fuzzy_match(item, query).is_none())
        {
            return None;
        }
        Some(score)
    }

    pub fn fuzzy_indicies(&self, item: &str, matcher: &Matcher) -> Option<(i64, Vec<usize>)> {
        if self.queries.len() == 1 {
            return matcher.fuzzy_indices(item, &self.queries[0]);
        }

        // use the rank of the first query for the rank, because merging ranks is not really possible
        // this behaviour matches fzf and skim
        let (score, mut indicies) = matcher.fuzzy_indices(item, self.queries.get(0)?)?;

        // fast path for the common case of not using a space
        // during matching this branch should be free thanks to branch prediction
        if self.queries.len() == 1 {
            return Some((score, indicies));
        }

        for query in &self.queries[1..] {
            let (_, matched_indicies) = matcher.fuzzy_indices(item, query)?;
            indicies.extend_from_slice(&matched_indicies);
        }

        // deadup and remove duplicate matches
        indicies.sort_unstable();
        indicies.dedup();

        Some((score, indicies))
    }
}
