use crate::tree_sitter::query::predicate::{InvalidPredicateError, Predicate};
use crate::tree_sitter::query::QueryStr;

#[derive(Debug)]
pub struct QueryProperty {
    pub key: QueryStr,
    pub val: Option<QueryStr>,
}

impl QueryProperty {
    pub fn parse(predicate: &Predicate) -> Result<Self, InvalidPredicateError> {
        predicate.check_min_arg_count(1)?;
        predicate.check_max_arg_count(2)?;
        let key = predicate.query_str_arg(0)?;
        let val = (predicate.num_args() == 1)
            .then(|| predicate.query_str_arg(1))
            .transpose()?;
        Ok(QueryProperty { key, val })
    }
}
