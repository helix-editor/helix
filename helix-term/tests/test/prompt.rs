use crate::test::helpers::test_harness::{TestCase, TestHarness};

#[tokio::test(flavor = "multi_thread")]
async fn test_history_completion() -> anyhow::Result<()> {
    TestHarness::default()
        .push_test_case(
            TestCase::default()
                .with_keys(":asdf<ret>:theme d<C-n><tab>")
                .with_validation_fn(Box::new(|cx| {
                    cx.assert_app_is_ok();
                })),
        )
        .run()
        .await
}
