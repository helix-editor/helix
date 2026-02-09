use std::{
    io::{self, stdout, Stdout, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
mod test {
    fn hello_world() {
        1 + 1;

        let does_indentation_work = 1;

        let mut really_long_variable_name_using_up_the_line =
            really_long_fn_that_should_definitely_go_on_the_next_line();
        really_long_variable_name_using_up_the_line =
            really_long_fn_that_should_definitely_go_on_the_next_line();
        really_long_variable_name_using_up_the_line |=
            really_long_fn_that_should_definitely_go_on_the_next_line();

        let (
            a_long_variable_name_in_this_tuple,
            b_long_variable_name_in_this_tuple,
            c_long_variable_name_in_this_tuple,
            d_long_variable_name_in_this_tuple,
            e_long_variable_name_in_this_tuple,
        ): (usize, usize, usize, usize, usize) =
            if really_long_fn_that_should_definitely_go_on_the_next_line() {
                (
                    03294239434,
                    1213412342314,
                    21231234134,
                    834534234549898789,
                    9879234234543853457,
                )
            } else {
                (0, 1, 2, 3, 4)
            };

        let test_function = function_with_param(this_param,
            that_param
        );

        let test_function = function_with_param(
            this_param,
            that_param
        );

        let test_function = function_with_proper_indent(param1,
            param2,
        );

        let selection = Selection::new(
            changes
                .clone()
                .map(|(start, end, text): (usize, usize, Option<Tendril>)| {
                    let len = text.map(|text| text.len()).unwrap() - 1; // minus newline
                    let pos = start + len;
                    Range::new(pos, pos)
                })
                .collect(),
            0,
        );

        return;
    }
}

impl<A, D> MyTrait<A, D> for YourType
where
    A: TraitB + TraitC,
    D: TraitE + TraitF,
{

}
#[test]
//
match test {
    Some(a) => 1,
    None => {
        unimplemented!()
    }
}
std::panic::set_hook(Box::new(move |info| {
    hook(info);
}));

{ { {
    1
}}}

pub fn change<I>(document: &Document, changes: I) -> Self
where
    I: IntoIterator<Item = Change> + ExactSizeIterator,
{
    [
        1,
        2,
        3,
    ];
    (
        1,
        2
    );
    true
}
