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

        // Assignment / destructuring RHS continued on the next line. helix
        // deliberately indents these continuations only when reindenting an
        // already-complete expression (an opinionated binary/assignment rule);
        // typing a newline after `=` does not indent the RHS, because the value
        // is a sibling the typing-direction walk never reaches. Left commented so
        // the corpus stays clean in both directions.
        // let mut really_long_variable_name_using_up_the_line =
        //     really_long_fn_that_should_definitely_go_on_the_next_line();
        // really_long_variable_name_using_up_the_line =
        //     really_long_fn_that_should_definitely_go_on_the_next_line();
        // really_long_variable_name_using_up_the_line |=
        //     really_long_fn_that_should_definitely_go_on_the_next_line();
        //
        // let (
        //     a_long_variable_name_in_this_tuple,
        //     b_long_variable_name_in_this_tuple,
        //     c_long_variable_name_in_this_tuple,
        //     d_long_variable_name_in_this_tuple,
        //     e_long_variable_name_in_this_tuple,
        // ): (usize, usize, usize, usize, usize) =
        //     if really_long_fn_that_should_definitely_go_on_the_next_line() {
        //         (03294239434, 1213412342314, 21231234134)
        //     } else {
        //         (0, 1, 2)
        //     };

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

trait Greet {
    fn hello(&self) -> String;

    fn default_greeting() -> String {
        String::from("hi")
    }
}

enum Shape {
    Circle { radius: f64 },
    Square(f64),
    Point,
}

fn process(shape: Shape) -> i32 {
    match shape {
        Shape::Circle { radius } if radius > 0.0 => {
            let area = 3.14 * radius * radius;
            area as i32
        }
        Shape::Square(side) => (side * side) as i32,
        Shape::Point => 0,
    }
}

fn handle(opt: Option<i32>) {
    if let Some(value) = opt {
        println!("{}", value);
    }

    while let Some(item) = iterator.next() {
        process(item);
    }

    let closure = |x: i32| {
        let doubled = x * 2;
        doubled + 1
    };

    let nested: Vec<Vec<i32>> = vec![
        vec![1, 2],
        vec![3, 4],
    ];
}

fn let_else(opt: Option<i32>) -> i32 {
    let Some(value) = opt else {
        return 0;
    };
    value
}

fn branches(x: i32) -> i32 {
    if x > 0 {
        1
    } else if x < 0 {
        -1
    } else {
        0
    }
}

async fn await_chain() -> Result<i32, Error> {
    let value = client
        .request()
        .await?
        .json()
        .await?;
    Ok(value)
}

fn block_rhs() -> i32 {
    let x = {
        let a = 1;
        a + 1
    };
    x
}

fn labeled() -> i32 {
    let result = 'outer: loop {
        loop {
            break 'outer 42;
        }
    };
    result
}

#[derive(
    Debug,
    Clone,
)]
struct Config {
    name: String,
}

fn update_syntax() -> Config {
    Config {
        name: "x".to_string(),
        ..Default::default()
    }
}

fn match_nested(x: i32) -> i32 {
    match x {
        1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15
        | 16 | 17 => 1,
        n if n > 10 => {
            let doubled = n * 2;
            doubled
        }
        _ => match x {
            0 => 0,
            _ => -1,
        },
    }
}

fn binary_cont() -> bool {
    let result = some_long_condition_here
        && another_condition
        && third_condition;
    result
}

fn arm_struct(x: i32) -> Point {
    match x {
        0 => Point {
            x: 0,
            y: 0,
        },
        _ => Point { x: 1, y: 1 },
    }
}

fn chain_off_multiline_call() {
    let x = Foo::new(
        arg1,
        arg2,
    )
    .method()
    .other();
}

fn complex<T, U>(
    first: T,
    second: U,
) -> Result<T, U>
where
    T: Clone,
    U: Debug,
{
    todo!()
}

fn await_chain_off_multiline_call() -> Result<(), Error> {
    let v = build_request(
        url,
        body,
    )
    .send()
    .await?;
    Ok(())
}

// Multi-line string / raw-string bodies are literal content: the @opaque rule
// preserves their existing leading whitespace instead of reformatting it.
fn strings() {
    let s = "first
second
    indented in string";
    let r = r#"
raw line
    indented raw"#;
    println!("{s}{r}");
}
