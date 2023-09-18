use helix_term::application::Application;

use super::*;

mod movement;
mod write;

#[tokio::test(flavor = "multi_thread")]
async fn test_selection_duplication() -> anyhow::Result<()> {
    // Forward
    test((
        platform_line(indoc! {"\
            #[lo|]#rem
            ipsum
            dolor
            "}),
        "CC",
        platform_line(indoc! {"\
            #(lo|)#rem
            #(ip|)#sum
            #[do|]#lor
            "}),
    ))
    .await?;

    // Backward
    test((
        platform_line(indoc! {"\
            #[|lo]#rem
            ipsum
            dolor
            "}),
        "CC",
        platform_line(indoc! {"\
            #(|lo)#rem
            #(|ip)#sum
            #[|do]#lor
            "}),
    ))
    .await?;

    // Copy the selection to previous line, skipping the first line in the file
    test((
        platform_line(indoc! {"\
            test
            #[testitem|]#
            "}),
        "<A-C>",
        platform_line(indoc! {"\
            test
            #[testitem|]#
            "}),
    ))
    .await?;

    // Copy the selection to previous line, including the first line in the file
    test((
        platform_line(indoc! {"\
            test
            #[test|]#
            "}),
        "<A-C>",
        platform_line(indoc! {"\
            #[test|]#
            #(test|)#
            "}),
    ))
    .await?;

    // Copy the selection to next line, skipping the last line in the file
    test((
        platform_line(indoc! {"\
            #[testitem|]#
            test
            "}),
        "C",
        platform_line(indoc! {"\
            #[testitem|]#
            test
            "}),
    ))
    .await?;

    // Copy the selection to next line, including the last line in the file
    test((
        platform_line(indoc! {"\
            #[test|]#
            test
            "}),
        "C",
        platform_line(indoc! {"\
            #(test|)#
            #[test|]#
            "}),
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_goto_file_impl() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::new()?;

    fn match_paths(app: &Application, matches: Vec<&str>) -> usize {
        app.editor
            .documents()
            .filter_map(|d| d.path()?.file_name())
            .filter(|n| matches.iter().any(|m| *m == n.to_string_lossy()))
            .count()
    }

    // Single selection
    test_key_sequence(
        &mut AppBuilder::new().with_file(file.path(), None).build()?,
        Some("ione.js<esc>%gf"),
        Some(&|app| {
            assert_eq!(1, match_paths(app, vec!["one.js"]));
        }),
        false,
    )
    .await?;

    // Multiple selection
    test_key_sequence(
        &mut AppBuilder::new().with_file(file.path(), None).build()?,
        Some("ione.js<ret>two.js<esc>%<A-s>gf"),
        Some(&|app| {
            assert_eq!(2, match_paths(app, vec!["one.js", "two.js"]));
        }),
        false,
    )
    .await?;

    // Cursor on first quote
    test_key_sequence(
        &mut AppBuilder::new().with_file(file.path(), None).build()?,
        Some("iimport 'one.js'<esc>B;gf"),
        Some(&|app| {
            assert_eq!(1, match_paths(app, vec!["one.js"]));
        }),
        false,
    )
    .await?;

    // Cursor on last quote
    test_key_sequence(
        &mut AppBuilder::new().with_file(file.path(), None).build()?,
        Some("iimport 'one.js'<esc>bgf"),
        Some(&|app| {
            assert_eq!(1, match_paths(app, vec!["one.js"]));
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multi_selection_paste() -> anyhow::Result<()> {
    test((
        platform_line(indoc! {"\
            #[|lorem]#
            #(|ipsum)#
            #(|dolor)#
            "}),
        "yp",
        platform_line(indoc! {"\
            lorem#[|lorem]#
            ipsum#(|ipsum)#
            dolor#(|dolor)#
            "}),
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multi_selection_shell_commands() -> anyhow::Result<()> {
    // pipe
    test((
        platform_line(indoc! {"\
            #[|lorem]#
            #(|ipsum)#
            #(|dolor)#
            "}),
        "|echo foo<ret>",
        platform_line(indoc! {"\
            #[|foo\n]#
            
            #(|foo\n)#
            
            #(|foo\n)#
            
            "}),
    ))
    .await?;

    // insert-output
    test((
        platform_line(indoc! {"\
            #[|lorem]#
            #(|ipsum)#
            #(|dolor)#
            "}),
        "!echo foo<ret>",
        platform_line(indoc! {"\
            #[|foo\n]#
            lorem
            #(|foo\n)#
            ipsum
            #(|foo\n)#
            dolor
            "}),
    ))
    .await?;

    // append-output
    test((
        platform_line(indoc! {"\
            #[|lorem]#
            #(|ipsum)#
            #(|dolor)#
            "}),
        "<A-!>echo foo<ret>",
        platform_line(indoc! {"\
            lorem#[|foo\n]#
            
            ipsum#(|foo\n)#
            
            dolor#(|foo\n)#
            
            "}),
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_undo_redo() -> anyhow::Result<()> {
    // A jumplist selection is created at a point which is undone.
    //
    // * 2[<space>   Add two newlines at line start. We're now on line 3.
    // * <C-s>       Save the selection on line 3 in the jumplist.
    // * u           Undo the two newlines. We're now on line 1.
    // * <C-o><C-i>  Jump forward an back again in the jumplist. This would panic
    //               if the jumplist were not being updated correctly.
    test(("#[|]#", "2[<space><C-s>u<C-o><C-i>", "#[|]#")).await?;

    // A jumplist selection is passed through an edit and then an undo and then a redo.
    //
    // * [<space>    Add a newline at line start. We're now on line 2.
    // * <C-s>       Save the selection on line 2 in the jumplist.
    // * kd          Delete line 1. The jumplist selection should be adjusted to the new line 1.
    // * uU          Undo and redo the `kd` edit.
    // * <C-o>       Jump back in the jumplist. This would panic if the jumplist were not being
    //               updated correctly.
    // * <C-i>       Jump forward to line 1.
    test(("#[|]#", "[<space><C-s>kduU<C-o><C-i>", "#[|]#")).await?;

    // In this case we 'redo' manually to ensure that the transactions are composing correctly.
    test(("#[|]#", "[<space>u[<space>u", "#[|]#")).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_extend_line() -> anyhow::Result<()> {
    // extend with line selected then count
    test((
        platform_line(indoc! {"\
            #[l|]#orem
            ipsum
            dolor
            
            "}),
        "x2x",
        platform_line(indoc! {"\
            #[lorem
            ipsum
            dolor\n|]#
            
            "}),
    ))
    .await?;

    // extend with count on partial selection
    test((
        platform_line(indoc! {"\
            #[l|]#orem
            ipsum
            
            "}),
        "2x",
        platform_line(indoc! {"\
            #[lorem
            ipsum\n|]#
            
            "}),
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_character_info() -> anyhow::Result<()> {
    // UTF-8, single byte
    test_key_sequence(
        &mut helpers::AppBuilder::new().build()?,
        Some("ih<esc>h:char<ret>"),
        Some(&|app| {
            assert_eq!(
                r#""h" (U+0068) Dec 104 Hex 68"#,
                app.editor.get_status().unwrap().0
            );
        }),
        false,
    )
    .await?;

    // UTF-8, multi-byte
    test_key_sequence(
        &mut helpers::AppBuilder::new().build()?,
        Some("ië<esc>h:char<ret>"),
        Some(&|app| {
            assert_eq!(
                r#""ë" (U+0065 U+0308) Hex 65 + cc 88"#,
                app.editor.get_status().unwrap().0
            );
        }),
        false,
    )
    .await?;

    // Multiple characters displayed as one, escaped characters
    test_key_sequence(
        &mut helpers::AppBuilder::new().build()?,
        Some(":line<minus>ending crlf<ret>:char<ret>"),
        Some(&|app| {
            assert_eq!(
                r#""\r\n" (U+000d U+000a) Hex 0d + 0a"#,
                app.editor.get_status().unwrap().0
            );
        }),
        false,
    )
    .await?;

    // Non-UTF-8
    test_key_sequence(
        &mut helpers::AppBuilder::new().build()?,
        Some(":encoding ascii<ret>ih<esc>h:char<ret>"),
        Some(&|app| {
            assert_eq!(r#""h" Dec 104 Hex 68"#, app.editor.get_status().unwrap().0);
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_char_backward() -> anyhow::Result<()> {
    // don't panic when deleting overlapping ranges
    test((
        platform_line("#(x|)# #[x|]#"),
        "c<space><backspace><esc>",
        platform_line("#[\n|]#"),
    ))
    .await?;
    test((
        platform_line("#( |)##( |)#a#( |)#axx#[x|]#a"),
        "li<backspace><esc>",
        platform_line("#(a|)##(|a)#xx#[|a]#"),
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_word_backward() -> anyhow::Result<()> {
    // don't panic when deleting overlapping ranges
    test((
        platform_line("fo#[o|]#ba#(r|)#"),
        "a<C-w><esc>",
        platform_line("#[\n|]#"),
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_word_forward() -> anyhow::Result<()> {
    // don't panic when deleting overlapping ranges
    test((
        platform_line("fo#[o|]#b#(|ar)#"),
        "i<A-d><esc>",
        platform_line("fo#[\n|]#"),
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_char_forward() -> anyhow::Result<()> {
    test((
        platform_line(indoc! {"\
                #[abc|]#def
                #(abc|)#ef
                #(abc|)#f
                #(abc|)#
            "}),
        "a<del><esc>",
        platform_line(indoc! {"\
                #[abc|]#ef
                #(abc|)#f
                #(abc|)#
                #(abc|)#
            "}),
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_insert_with_indent() -> anyhow::Result<()> {
    const INPUT: &str = "\
#[f|]#n foo() {
    if let Some(_) = None {

    }
\x20
}

fn bar() {

}";

    // insert_at_line_start
    test((
        INPUT,
        ":lang rust<ret>%<A-s>I",
        "\
#[f|]#n foo() {
    #(i|)#f let Some(_) = None {
        #(\n|)#\
\x20   #(}|)#
#(\x20|)#
#(}|)#
#(\n|)#\
#(f|)#n bar() {
    #(\n|)#\
#(}|)#",
    ))
    .await?;

    // insert_at_line_end
    test((
        INPUT,
        ":lang rust<ret>%<A-s>A",
        "\
fn foo() {#[\n|]#\
\x20   if let Some(_) = None {#(\n|)#\
\x20       #(\n|)#\
\x20   }#(\n|)#\
\x20#(\n|)#\
}#(\n|)#\
#(\n|)#\
fn bar() {#(\n|)#\
\x20   #(\n|)#\
}#(|)#",
    ))
    .await?;

    Ok(())
}
