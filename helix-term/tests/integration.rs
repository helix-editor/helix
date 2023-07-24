mod test {

    #[tokio::test(flavor = "multi_thread")]
    async fn hello_world() -> anyhow::Result<()> {
        crate::test!(("#[\n|]#"), ("ihello world<esc>"), ("hello world#[|\n]#")).await
    }

    mod auto_indent;
    mod auto_pairs;
    mod backend;
    mod commands;
    mod helpers;
    mod movement;
    mod prompt;
    mod splits;
}
