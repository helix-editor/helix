use super::*;

mod simple {
    use super::*;
    #[tokio::test(flavor = "multi_thread")]
    async fn uncomment_inner_injection() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|on this line s]#hould use the javascript comment token(s).
              foo();
            </script>
        "#},
            ":lang html<ret> c",
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              Comment toggle #[|on this line s]#hould use the javascript comment token(s).
              foo();
            </script>
        "#},
        ))
        .await?;

        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn comment_inner_injection() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              Comment toggle #[|on this line s]#hould use the javascript comment token(s).
              foo();
            </script>
        "#},
            ":lang html<ret> c",
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|on this line s]#hould use the javascript comment token(s).
              foo();
            </script>
        "#},
        ))
        .await?;

        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn block_comment_inner_injection() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|on this line s]#hould use the javascript comment token(s).
              foo();
            </script>
        "#},
            ":lang html<ret> C",
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|/* on this line s */]#hould use the javascript comment token(s).
              foo();
            </script>
        "#},
        ))
        .await?;

        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn block_uncomment_inner_injection() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|/* on this line s */]#hould use the javascript comment token(s).
              foo();
            </script>
        "#},
            ":lang html<ret> C",
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|on this line s]#hould use the javascript comment token(s).
              foo();
            </script>
        "#},
        ))
        .await?;

        Ok(())
    }
}

mod injected_comment_tokens_continue_comment {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn adds_new_comment_on_newline() -> anyhow::Result<()> {
        test((
            indoc! {r#"
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should #[|c]#ontinue comments
              foo();
            </script>
        "#},
            ":lang html<ret>i<ret>",
            indoc! {r#"
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should
              // #[|c]#ontinue comments
              foo();
            </script>
        "#},
        ))
        .await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn continues_comment() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should
              // #[|c]#ontinue comments
              foo();
            </script>
        "#},
            ":lang html<ret>i<ret>",
            indoc! {r#"\
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should
              //
              // #[|c]#ontinue comments
              foo();
            </script>
        "#},
        ))
        .await?;
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_injected_comment_tokens_continue_comment_d() -> anyhow::Result<()> {
    test((
        indoc! {r#"\
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should #[|c]#ontinue comments
              foo();
            </script>
        "#},
        ":lang html<ret>O",
        indoc! {r#"\
            <p>Some text 1234</p>
            <script type="text/javascript">
              // #[
            |]#  // This line should continue comments
              foo();
            </script>
        "#},
    ))
    .await?;

    Ok(())
}

mod multiple_selections_different_injection_layers {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn comments_two_different_injection_layers_with_different_comments() -> anyhow::Result<()>
    {
        test((
            indoc! {r#"\
            <p>Comment toggle #[|on this line ]#should use the HTML comment token(s).</p>
            <script type="text/javascript">
              Comment toggle #(|on this line )#should use the javascript comment token(s).
              foo();
            </script>
        "#},
            ":lang html<ret> c",
            indoc! {r#"\
            <!-- <p>Comment toggle #[|on this line ]#should use the HTML comment token(s).</p> -->
            <script type="text/javascript">
              // Comment toggle #(|on this line )#should use the javascript comment token(s).
              foo();
            </script>
        "#},
        ))
        .await?;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn uncomments_two_different_injection_layers_with_different_comments(
    ) -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <!-- <p>Comment toggle #[|on this line ]#should use the HTML comment token(s).</p> -->
            <script type="text/javascript">
              // Comment toggle #(|on this line )#should use the javascript comment token(s).
              foo();
            </script>
        "#},
            ":lang html<ret> c",
            indoc! {r#"\
            <p>Comment toggle #[|on this line ]#should use the HTML comment token(s).</p>
            <script type="text/javascript">
              Comment toggle #(|on this line )#should use the javascript comment token(s).
              foo();
            </script>
        "#},
        ))
        .await?;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn works_with_multiple_selections() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <p>Comment toggle #(|on this line )#should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|on this line ]#should use the javascript comment token(s).
              foo();
            </script>
        "#},
            ":lang html<ret> c",
            indoc! {r#"\
            <!-- <p>Comment toggle #(|on this line )#should use the HTML comment token(s).</p> -->
            <script type="text/javascript">
              Comment toggle #[|on this line ]#should use the javascript comment token(s).
              foo();
            </script>
        "#},
        ))
        .await?;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn works_with_nested_injection_layers_html_js_then_css() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <!-- <p>Comment toggle #(|on this line)# should use the HTML comment token(s).</p> -->
            <script type="text/javascript">
              // Comment toggle #(|on this line)# should use the javascript comment token(s).
              foo();
              css`
                h#[tml {
                  background-color: |]#red;
                }
              `
            </script>
        "#},
            ":lang html<ret> c",
            indoc! {r#"\
            <p>Comment toggle #(|on this line)# should use the HTML comment token(s).</p>
            <script type="text/javascript">
              Comment toggle #(|on this line)# should use the javascript comment token(s).
              foo();
              css`
                /* h#[tml { */
                  /* background-color: |]#red; */
                }
              `
            </script>
        "#},
        ))
        .await?;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn full_line_selection_commenting() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle on this line should use the javascript comment token(s).
            #[  foo();
              css`
            |]#    html {
                  background-color: red;
                }
              `;
            </script>
        "#},
            ":lang html<ret> c",
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle on this line should use the javascript comment token(s).
            #[  // foo();
              // css`
            |]#    html {
                  background-color: red;
                }
              `;
            </script>
        "#},
        ))
        .await?;
        test((
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle on this line should use the javascript comment token(s).
            #[  // foo();
              // css`
            |]#    html {
                  background-color: red;
                }
              `;
            </script>
        "#},
            ":lang html<ret> c",
            indoc! {r#"\
            <p>Comment toggle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle on this line should use the javascript comment token(s).
            #[  foo();
              css`
            |]#    html {
                  background-color: red;
                }
              `;
            </script>
        "#},
        ))
        .await?;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn block_comment_toggle_across_different_layers() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
            <p>Comment toggle #(|on this line)# should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|on this line]# should use the javascript comment token(s).
              foo();
            </script>
        "#},
            ":lang html<ret> C",
            indoc! {r#"\
            <p>Comment toggle #(|<!-- on this line -->)# should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment toggle #[|/* on this line */]# should use the javascript comment token(s).
              foo();
            </script>
        "#},
        ))
        .await?;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn multiple_selections_same_line() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
                <p>C#[|o]#mment t#(|o)#ggle #(|o)#n this line sh#(|o)#uld use the HTML c#(|o)#mment t#(|o)#ken(s).</p>
                <script type="text/javascript">
                  // Comment toggle on this line should use the javascript comment token(s).
                  foo();
                  css`
                    html {
                      background-color: red;
                    }
                  `;
                </script>
            "#},
            ":lang html<ret> C",
            indoc! {r#"\
                <p>C#[|<!-- o -->]#mment t#(|<!-- o -->)#ggle #(|<!-- o -->)#n this line sh#(|<!-- o -->)#uld use the HTML c#(|<!-- o -->)#mment t#(|<!-- o -->)#ken(s).</p>
                <script type="text/javascript">
                  // Comment toggle on this line should use the javascript comment token(s).
                  foo();
                  css`
                    html {
                      background-color: red;
                    }
                  `;
                </script>
            "#},
        ))
        .await?;
        test((
            indoc! {r#"\
                <p>C#[|<!-- o -->]#mment t#(|<!-- o -->)#ggle #(|<!-- o -->)#n this line sh#(|<!-- o -->)#uld use the HTML c#(|<!-- o -->)#mment t#(|<!-- o -->)#ken(s).</p>
                <script type="text/javascript">
                  // Comment toggle on this line should use the javascript comment token(s).
                  foo();
                  css`
                    html {
                      background-color: red;
                    }
                  `;
                </script>
            "#},
            ":lang html<ret> C",
            indoc! {r#"\
                <p>C#[|o]#mment t#(|o)#ggle #(|o)#n this line sh#(|o)#uld use the HTML c#(|o)#mment t#(|o)#ken(s).</p>
                <script type="text/javascript">
                  // Comment toggle on this line should use the javascript comment token(s).
                  foo();
                  css`
                    html {
                      background-color: red;
                    }
                  `;
                </script>
            "#},
        ))
        .await?;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn many_single_line_selections() -> anyhow::Result<()> {
        test((
            indoc! {r#"\
                <p>C#[|o]#mment t#(|o)#ggle #(|o)#n this line sh#(|o)#uld use the HTML c#(|o)#mment t#(|o)#ken(s).</p>
                <script type="text/javascript">
                  // C#(|o)#mment t#(|o)#ggle #(|o)#n this line sh#(|o)#uld use the javascript c#(|o)#mment t#(|o)#ken(s).
                  f#(|o)##(|o)#();
                  css`
                    html {
                      backgr#(|o)#und-c#(|o)#l#(|o)#r: red;
                    }
                  `;
                </script>
            "#},
            ":lang html<ret> C",
            indoc! {r#"\
                <p>C#[|<!-- o -->]#mment t#(|<!-- o -->)#ggle #(|<!-- o -->)#n this line sh#(|<!-- o -->)#uld use the HTML c#(|<!-- o -->)#mment t#(|<!-- o -->)#ken(s).</p>
                <script type="text/javascript">
                  // C#(|/* o */)#mment t#(|/* o */)#ggle #(|/* o */)#n this line sh#(|/* o */)#uld use the javascript c#(|/* o */)#mment t#(|/* o */)#ken(s).
                  f#(|/* o */)##(|/* o */)#();
                  css`
                    html {
                      backgr#(|/* o */)#und-c#(|/* o */)#l#(|/* o */)#r: red;
                    }
                  `;
                </script>
            "#},
        ))
        .await?;
        test((
            indoc! {r#"\
                <p>C#[|<!-- o -->]#mment t#(|<!-- o -->)#ggle #(|<!-- o -->)#n this line sh#(|<!-- o -->)#uld use the HTML c#(|<!-- o -->)#mment t#(|<!-- o -->)#ken(s).</p>
                <script type="text/javascript">
                  // C#(|/* o */)#mment t#(|/* o */)#ggle #(|/* o */)#n this line sh#(|/* o */)#uld use the javascript c#(|/* o */)#mment t#(|/* o */)#ken(s).
                  f#(|/* o */)##(|/* o */)#();
                  css`
                    html {
                      backgr#(|/* o */)#und-c#(|/* o */)#l#(|/* o */)#r: red;
                    }
                  `;
                </script>
            "#},
            ":lang html<ret> C",
            indoc! {r#"\
                <p>C#[|o]#mment t#(|o)#ggle #(|o)#n this line sh#(|o)#uld use the HTML c#(|o)#mment t#(|o)#ken(s).</p>
                <script type="text/javascript">
                  // C#(|o)#mment t#(|o)#ggle #(|o)#n this line sh#(|o)#uld use the javascript c#(|o)#mment t#(|o)#ken(s).
                  f#(|o)##(|o)#();
                  css`
                    html {
                      backgr#(|o)#und-c#(|o)#l#(|o)#r: red;
                    }
                  `;
                </script>
            "#},
        ))
        .await?;
        Ok(())
    }
}

/// A selection that spans across several injections takes comment tokens
/// from the injection with the bigger scope
#[tokio::test(flavor = "multi_thread")]
async fn test_injected_comment_tokens_selection_across_different_layers() -> anyhow::Result<()> {
    test((
        indoc! {r#"\
            <p>Comment tog#[|gle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment togg]#le on this line should use the javascript comment token(s).
              foo();
            </script>
        "#},
        ":lang html<ret> c",
        indoc! {r#"\
            <!-- <p>Comment tog#[|gle on this line should use the HTML comment token(s).</p> -->
            <!-- <script type="text/javascript"> -->
              <!-- // Comment togg]#le on this line should use the javascript comment token(s). -->
              foo();
            </script>
        "#},
    ))
    .await?;
    test((
        indoc! {r#"\
            <p>Comment tog#[|gle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment togg]#le on this line should use the javascript comment token(s).
              foo();
            </script>
        "#},
        ":lang html<ret> C",
        indoc! {r#"\
            <p>Comment tog#[|<!-- gle on this line should use the HTML comment token(s).</p>
            <script type="text/javascript">
              // Comment togg -->]#le on this line should use the javascript comment token(s).
              foo();
            </script>
        "#},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_join_selections_comment() -> anyhow::Result<()> {
    test((
        indoc! {"\
            /// #[a|]#bc
            /// def
        "},
        ":lang rust<ret>J",
        indoc! {"\
            /// #[a|]#bc def
        "},
    ))
    .await?;

    // Only join if the comment token matches the previous line.
    test((
        indoc! {"\
            #[| // a
            // b
            /// c
            /// d
            e
            /// f
            // g]#
        "},
        ":lang rust<ret>J",
        indoc! {"\
            #[| // a b /// c d e f // g]#
        "},
    ))
    .await?;

    test((
        "#[|\t// Join comments
\t// with indent]#",
        ":lang go<ret>J",
        "#[|\t// Join comments with indent]#",
    ))
    .await?;

    Ok(())
}
