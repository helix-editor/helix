use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent() -> anyhow::Result<()> {
    let app = || AppBuilder::new().with_file("foo.go", None);

    let enter_tests = [
        (
            helpers::platform_line(indoc! {r##"
                type Test struct {#[}|]#
            "##}),
            "i<ret>",
            helpers::platform_line(indoc! {"\
                type Test struct {
                \t#[|\n]#
                }
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                func main() {
                \tswitch nil {#[}|]#
                }
             "}),
            "i<ret>",
            helpers::platform_line(indoc! {"\
                func main() {
                \tswitch nil {
                \t\t#[|\n]#
                \t}
                }
            "}),
        ),
    ];

    for test in enter_tests {
        test_with_config(app(), test).await?;
    }

    Ok(())
}
