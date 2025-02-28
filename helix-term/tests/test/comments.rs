use super::*;

/// Comment and uncomment
#[tokio::test(flavor = "multi_thread")]
async fn test_injected_comment_tokens_simple() -> anyhow::Result<()> {
    // Uncomment inner injection
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

    // Comment inner injection
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

    // Block comment inner injection
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

    // Block uncomment inner injection
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

#[tokio::test(flavor = "multi_thread")]
async fn test_injected_comment_tokens_continue_comment() -> anyhow::Result<()> {
    test((
        indoc! {r#"\
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should #[|c]#ontinue comments
              foo();
            </script>
        "#},
        ":lang html<ret>i<ret>",
        indoc! {r#"\
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should
              // #[|c]#ontinue comments
              foo();
            </script>
        "#},
    ))
    .await?;

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

    test((
        indoc! {r#"\
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should #[|c]#ontinue comments
              foo();
            </script>
        "#},
        ":lang html<ret>i<ret>",
        indoc! {r#"\
            <p>Some text 1234</p>
            <script type="text/javascript">
              // This line should
              // #[|c]#ontinue comments
              foo();
            </script>
        "#},
    ))
    .await?;

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

/// Selections in different regions
#[tokio::test(flavor = "multi_thread")]
async fn test_injected_comment_tokens_multiple_selections() -> anyhow::Result<()> {
    // Comments two different injection layers with different comments
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

    // Uncomments two different injection layers with different comments
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

    // Works with multiple selections
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

    // Works with nested injection layers: html, js then css
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

    // Full-line selection commenting
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

    // Works with block comment toggle across different layers
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

    // Many selections on the same line
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

    // Many single-selections
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
