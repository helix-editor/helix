use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn test_move_parent_node_end() -> anyhow::Result<()> {
    let tests = vec![
        // single cursor stays single cursor, first goes to end of current
        // node, then parent
        (
            indoc! {r##"
                fn foo() {
                    let result = if true {
                        "yes"
                    } else {
                        "no#["|]#
                    }
                }
            "##},
            "<A-e>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"#[\n|]#
                    }
                }
            "},
        ),
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"#[\n|]#
                    }
                }
            "},
            "<A-e>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"
                    }#[\n|]#
                }
            "},
        ),
        // select mode extends
        (
            indoc! {r##"
                fn foo() {
                    let result = if true {
                        "yes"
                    } else {
                        #["no"|]#
                    }
                }
            "##},
            "v<A-e><A-e>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        #[\"no\"
                    }\n|]#
                }
            "},
        ),
    ];

    for test in tests {
        test_with_config(AppBuilder::new().with_file("foo.rs", None), test).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_move_parent_node_start() -> anyhow::Result<()> {
    let tests = vec![
        // single cursor stays single cursor, first goes to end of current
        // node, then parent
        (
            indoc! {r##"
                fn foo() {
                    let result = if true {
                        "yes"
                    } else {
                        "no#["|]#
                    }
                }
            "##},
            "<A-b>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        #[\"|]#no\"
                    }
                }
            "},
        ),
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"#[\n|]#
                    }
                }
            "},
            "<A-b>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else #[{|]#
                        \"no\"
                    }
                }
            "},
        ),
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else #[{|]#
                        \"no\"
                    }
                }
            "},
            "<A-b>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } #[e|]#lse {
                        \"no\"
                    }
                }
            "},
        ),
        // select mode extends
        (
            indoc! {r##"
                fn foo() {
                    let result = if true {
                        "yes"
                    } else {
                        #["no"|]#
                    }
                }
            "##},
            "v<A-b><A-b>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else #[|{
                        ]#\"no\"
                    }
                }
            "},
        ),
        (
            indoc! {r##"
                fn foo() {
                    let result = if true {
                        "yes"
                    } else {
                        #["no"|]#
                    }
                }
            "##},
            "v<A-b><A-b><A-b>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } #[|else {
                        ]#\"no\"
                    }
                }
            "},
        ),
    ];

    for test in tests {
        test_with_config(AppBuilder::new().with_file("foo.rs", None), test).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_smart_tab_move_parent_node_end() -> anyhow::Result<()> {
    let tests = vec![
        // single cursor stays single cursor, first goes to end of current
        // node, then parent
        (
            indoc! {r##"
                fn foo() {
                    let result = if true {
                        "yes"
                    } else {
                        "no#["|]#
                    }
                }
            "##},
            "i<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"#[|\n]#
                    }
                }
            "},
        ),
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"#[\n|]#
                    }
                }
            "},
            "i<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"
                    }#[|\n]#
                }
            "},
        ),
        // appending to the end of a line should still look at the current
        // line, not the next one
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no#[\"|]#
                    }
                }
            "},
            "a<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"
                    }#[\n|]#
                }
            "},
        ),
        // before cursor is all whitespace, so insert tab
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        #[\"no\"|]#
                    }
                }
            "},
            "i<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                            #[|\"no\"]#
                    }
                }
            "},
        ),
        // if selection spans multiple lines, it should still only look at the
        // line on which the head is
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        #[\"yes\"
                    } else {
                        \"no\"|]#
                    }
                }
            "},
            "a<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    } else {
                        \"no\"
                    }#[\n|]#
                }
            "},
        ),
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        #[\"yes\"
                    } else {
                        \"no\"|]#
                    }
                }
            "},
            "i<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                            #[|\"yes\"
                    } else {
                        \"no\"]#
                    }
                }
            "},
        ),
        (
            indoc! {"\
                fn foo() {
                    #[l|]#et result = if true {
                        #(\"yes\"
                    } else {
                        \"no\"|)#
                    }
                }
            "},
            "i<tab>",
            indoc! {"\
                fn foo() {
                        #[|l]#et result = if true {
                            #(|\"yes\"
                    } else {
                        \"no\")#
                    }
                }
            "},
        ),
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"#[\n|]#
                    } else {
                        \"no\"#(\n|)#
                    }
                }
            "},
            "i<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    }#[| ]#else {
                        \"no\"
                    }#(|\n)#
                }
            "},
        ),
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        #[\"yes\"|]#
                    } else {
                        #(\"no\"|)#
                    }
                }
            "},
            "i<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                            #[|\"yes\"]#
                    } else {
                            #(|\"no\")#
                    }
                }
            "},
        ),
        // if any cursors are not preceded by all whitespace, then do the
        // smart_tab action
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        #[\"yes\"\n|]#
                    } else {
                        \"no#(\"\n|)#
                    }
                }
            "},
            "i<tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                        \"yes\"
                    }#[| ]#else {
                        \"no\"
                    }#(|\n)#
                }
            "},
        ),
        // Ctrl-tab always inserts a tab
        (
            indoc! {"\
                fn foo() {
                    let result = if true {
                        #[\"yes\"\n|]#
                    } else {
                        \"no#(\"\n|)#
                    }
                }
            "},
            "i<S-tab>",
            indoc! {"\
                fn foo() {
                    let result = if true {
                            #[|\"yes\"\n]#
                    } else {
                        \"no    #(|\"\n)#
                    }
                }
            "},
        ),
    ];

    for test in tests {
        test_with_config(AppBuilder::new().with_file("foo.rs", None), test).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn select_all_siblings() -> anyhow::Result<()> {
    let tests = vec![
        // basic tests
        (
            indoc! {r##"
                let foo = bar(#[a|]#, b, c);
            "##},
            "<A-a>",
            indoc! {r##"
                let foo = bar(#[a|]#, #(b|)#, #(c|)#);
            "##},
        ),
        (
            indoc! {r##"
                let a = [
                    #[1|]#,
                    2,
                    3,
                    4,
                    5,
                ];
            "##},
            "<A-a>",
            indoc! {r##"
                let a = [
                    #[1|]#,
                    #(2|)#,
                    #(3|)#,
                    #(4|)#,
                    #(5|)#,
                ];
            "##},
        ),
        // direction is preserved
        (
            indoc! {r##"
                let a = [
                    #[|1]#,
                    2,
                    3,
                    4,
                    5,
                ];
            "##},
            "<A-a>",
            indoc! {r##"
                let a = [
                    #[|1]#,
                    #(|2)#,
                    #(|3)#,
                    #(|4)#,
                    #(|5)#,
                ];
            "##},
        ),
        // can't pick any more siblings - selection stays the same
        (
            indoc! {r##"
                let a = [
                    #[1|]#,
                    #(2|)#,
                    #(3|)#,
                    #(4|)#,
                    #(5|)#,
                ];
            "##},
            "<A-a>",
            indoc! {r##"
                let a = [
                    #[1|]#,
                    #(2|)#,
                    #(3|)#,
                    #(4|)#,
                    #(5|)#,
                ];
            "##},
        ),
        // each cursor does the sibling select independently
        (
            indoc! {r##"
                let a = [
                    #[1|]#,
                    2,
                    3,
                    4,
                    5,
                ];

                let b = [
                    #("one"|)#,
                    "two",
                    "three",
                    "four",
                    "five",
                ];
            "##},
            "<A-a>",
            indoc! {r##"
                let a = [
                    #[1|]#,
                    #(2|)#,
                    #(3|)#,
                    #(4|)#,
                    #(5|)#,
                ];

                let b = [
                    #("one"|)#,
                    #("two"|)#,
                    #("three"|)#,
                    #("four"|)#,
                    #("five"|)#,
                ];
            "##},
        ),
        // conflicting sibling selections get normalized. Here, the primary
        // selection would choose every list item, but because the secondary
        // range covers more than one item, the descendent is the entire list,
        // which means the sibling is the assignment. The list item ranges just
        // get normalized out since the list itself becomes selected.
        (
            indoc! {r##"
                let a = [
                    #[1|]#,
                    2,
                    #(3,
                    4|)#,
                    5,
                ];
            "##},
            "<A-a>",
            indoc! {r##"
                let #(a|)# = #[[
                    1,
                    2,
                    3,
                    4,
                    5,
                ]|]#;
            "##},
        ),
    ];

    for test in tests {
        test_with_config(AppBuilder::new().with_file("foo.rs", None), test).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn select_all_children() -> anyhow::Result<()> {
    let tests = vec![
        // basic tests
        (
            indoc! {r##"
                let foo = bar#[(a, b, c)|]#;
            "##},
            "<A-I>",
            indoc! {r##"
                let foo = bar(#[a|]#, #(b|)#, #(c|)#);
            "##},
        ),
        (
            indoc! {r##"
                let a = #[[
                    1,
                    2,
                    3,
                    4,
                    5,
                ]|]#;
            "##},
            "<A-I>",
            indoc! {r##"
                let a = [
                    #[1|]#,
                    #(2|)#,
                    #(3|)#,
                    #(4|)#,
                    #(5|)#,
                ];
            "##},
        ),
        // direction is preserved
        (
            indoc! {r##"
                let a = #[|[
                    1,
                    2,
                    3,
                    4,
                    5,
                ]]#;
            "##},
            "<A-I>",
            indoc! {r##"
                let a = [
                    #[|1]#,
                    #(|2)#,
                    #(|3)#,
                    #(|4)#,
                    #(|5)#,
                ];
            "##},
        ),
        // can't pick any more children - selection stays the same
        (
            indoc! {r##"
                let a = [
                    #[1|]#,
                    #(2|)#,
                    #(3|)#,
                    #(4|)#,
                    #(5|)#,
                ];
            "##},
            "<A-I>",
            indoc! {r##"
                let a = [
                    #[1|]#,
                    #(2|)#,
                    #(3|)#,
                    #(4|)#,
                    #(5|)#,
                ];
            "##},
        ),
        // each cursor does the sibling select independently
        (
            indoc! {r##"
                let a = #[|[
                    1,
                    2,
                    3,
                    4,
                    5,
                ]]#;

                let b = #([
                    "one",
                    "two",
                    "three",
                    "four",
                    "five",
                ]|)#;
            "##},
            "<A-I>",
            indoc! {r##"
                let a = [
                    #[|1]#,
                    #(|2)#,
                    #(|3)#,
                    #(|4)#,
                    #(|5)#,
                ];

                let b = [
                    #("one"|)#,
                    #("two"|)#,
                    #("three"|)#,
                    #("four"|)#,
                    #("five"|)#,
                ];
            "##},
        ),
    ];

    for test in tests {
        test_with_config(AppBuilder::new().with_file("foo.rs", None), test).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_select_next_sibling() -> anyhow::Result<()> {
    let tests = vec![
        // basic test
        (
            indoc! {r##"
                fn inc(x: usize) -> usize { x + 1 #[}|]#
                fn dec(x: usize) -> usize { x - 1 }
                fn ident(x: usize) -> usize { x }
            "##},
            "<A-n>",
            indoc! {r##"
                fn inc(x: usize) -> usize { x + 1 }
                #[fn dec(x: usize) -> usize { x - 1 }|]#
                fn ident(x: usize) -> usize { x }
            "##},
        ),
        // direction is not preserved and is always forward.
        (
            indoc! {r##"
                fn inc(x: usize) -> usize { x + 1 #[}|]#
                fn dec(x: usize) -> usize { x - 1 }
                fn ident(x: usize) -> usize { x }
            "##},
            "<A-n><A-;><A-n>",
            indoc! {r##"
                fn inc(x: usize) -> usize { x + 1 }
                fn dec(x: usize) -> usize { x - 1 }
                #[fn ident(x: usize) -> usize { x }|]#
            "##},
        ),
    ];

    for test in tests {
        test_with_config(AppBuilder::new().with_file("foo.rs", None), test).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_select_prev_sibling() -> anyhow::Result<()> {
    let tests = vec![
        // basic test
        (
            indoc! {r##"
                fn inc(x: usize) -> usize { x + 1 }
                fn dec(x: usize) -> usize { x - 1 }
                #[|f]#n ident(x: usize) -> usize { x }
            "##},
            "<A-p>",
            indoc! {r##"
                fn inc(x: usize) -> usize { x + 1 }
                #[|fn dec(x: usize) -> usize { x - 1 }]#
                fn ident(x: usize) -> usize { x }
            "##},
        ),
        // direction is not preserved and is always backward.
        (
            indoc! {r##"
                fn inc(x: usize) -> usize { x + 1 }
                fn dec(x: usize) -> usize { x - 1 }
                #[|f]#n ident(x: usize) -> usize { x }
            "##},
            "<A-p><A-;><A-p>",
            indoc! {r##"
                #[|fn inc(x: usize) -> usize { x + 1 }]#
                fn dec(x: usize) -> usize { x - 1 }
                fn ident(x: usize) -> usize { x }
            "##},
        ),
    ];

    for test in tests {
        test_with_config(AppBuilder::new().with_file("foo.rs", None), test).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn match_bracket() -> anyhow::Result<()> {
    let rust_tests = vec![
        // fwd
        (
            indoc! {r##"
                fn foo(x: usize) -> usize { #[x|]# + 1 }
            "##},
            "mm",
            indoc! {r##"
                fn foo(x: usize) -> usize { x + 1 #[}|]#
            "##},
        ),
        // backward
        (
            indoc! {r##"
                fn foo(x: usize) -> usize { #[x|]# + 1 }
            "##},
            "mmmm",
            indoc! {r##"
                fn foo(x: usize) -> usize #[{|]# x + 1 }
            "##},
        ),
        // avoid false positive inside string literal
        (
            indoc! {r##"
                fn foo() -> &'static str { "(hello#[ |]#world)" }
            "##},
            "mm",
            indoc! {r##"
                fn foo() -> &'static str { "(hello world)#["|]# }
            "##},
        ),
        // make sure matching on quotes works
        (
            indoc! {r##"
                fn foo() -> &'static str { "(hello#[ |]#world)" }
            "##},
            "mm",
            indoc! {r##"
                fn foo() -> &'static str { "(hello world)#["|]# }
            "##},
        ),
        // .. on both ends
        (
            indoc! {r##"
                fn foo() -> &'static str { "(hello#[ |]#world)" }
            "##},
            "mmmm",
            indoc! {r##"
                fn foo() -> &'static str { #["|]#(hello world)" }
            "##},
        ),
        // match on siblings nodes
        (
            indoc! {r##"
                fn foo(bar: Option<usize>) -> usize {
                    match bar {
                        Some(b#[a|]#r) => bar,
                        None => 42,
                    } 
                }
            "##},
            "mmmm",
            indoc! {r##"
                fn foo(bar: Option<usize>) -> usize {
                    match bar {
                        Some#[(|]#bar) => bar,
                        None => 42,
                    } 
                }
            "##},
        ),
        // gracefully handle multiple sibling brackets (usally for errors/incomplete syntax trees)
        // in the past we selected the first > instead of the second > here
        (
            indoc! {r##"
                fn foo() {
                    foo::<b#[a|]#r<>> 
                }
            "##},
            "mm",
            indoc! {r##"
                fn foo() {
                    foo::<bar<>#[>|]# 
                }
            "##},
        ),
        // named node with 2 or more children
        (
            indoc! {r##"
                use a::#[{|]#
                    b::{c, d, e, f, g},
                    h, i, j, k, l, m, n,
                };
            "##},
            "mm",
            indoc! {r##"
                use a::{
                    b::{c, d, e, f, g},
                    h, i, j, k, l, m, n,
                #[}|]#;
            "##},
        ),
    ];

    let python_tests = vec![
        // python quotes have a slightly more complex syntax tree
        // that triggerd a bug in an old implementation so we test
        // them here
        (
            indoc! {r##"
                foo_python = "mm does not#[ |]#work on this string"
            "##},
            "mm",
            indoc! {r##"
                foo_python = "mm does not work on this string#["|]#
            "##},
        ),
        (
            indoc! {r##"
                foo_python = "mm does not#[ |]#work on this string"
            "##},
            "mmmm",
            indoc! {r##"
                foo_python = #["|]#mm does not work on this string"
            "##},
        ),
    ];

    for test in rust_tests {
        println!("{test:?}");
        test_with_config(AppBuilder::new().with_file("foo.rs", None), test).await?;
    }
    for test in python_tests {
        println!("{test:?}");
        test_with_config(AppBuilder::new().with_file("foo.py", None), test).await?;
    }

    Ok(())
}
