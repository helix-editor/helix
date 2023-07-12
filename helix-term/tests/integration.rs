#[cfg(feature = "integration")]
mod test {
    mod helpers;

    use self::helpers::test_harness::test;

    #[tokio::test(flavor = "multi_thread")]
    async fn hello_world() -> anyhow::Result<()> {
        test(("#[\n|]#", "ihello world<esc>", "hello world#[|\n]#")).await
    }

    mod auto_indent;
    mod auto_pairs;
    mod commands;
    mod movement;
    mod prompt;
    mod splits;
}
