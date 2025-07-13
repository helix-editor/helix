use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent() -> anyhow::Result<()> {
    let app = || AppBuilder::new().with_file("foo.go", None);

    let enter_tests = [
        (
            indoc! {r##"
                type Test struct {#[}|]#
            "##},
            "i<ret>",
            indoc! {"\
                type Test struct {
                \t#[|\n]#
                }
            "},
        ),
        (
            indoc! {"\
                func main() {
                \tswitch nil {#[}|]#
                }
             "},
            "i<ret>",
            indoc! {"\
                func main() {
                \tswitch nil {
                \t\t#[|\n]#
                \t}
                }
            "},
        ),
    ];

    for test in enter_tests {
        test_with_config(app(), test).await?;
    }

    Ok(())
}
