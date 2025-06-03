use super::*;

use helix_core::hashmap;
use helix_term::keymap;
use helix_view::document::Mode;

#[tokio::test(flavor = "multi_thread")]
async fn paste_all_selections_before() -> anyhow::Result<()> {
    let mut config = Config::default();
    config.keys.insert(
        Mode::Normal,
        keymap!({"Normal Mode"
            "A-P" => paste_all_selections_before,
            "A-p" => paste_all_selections_after,
        }),
    );

    // Test paste_all_selections_before
    test_with_config(
        AppBuilder::new().with_config(config.clone()),
        (
            indoc! {"\
            #(|one)#_cat
            #(|two)#_dog
            #[|three]#_cow
            "},
            "y<A-P>",
            indoc! {"\
            #(one|)##(two|)##(three|)#one_cat
            #(one|)##(two|)##(three|)#two_dog
            #(one|)##(two|)##[three|]#three_cow
            "},
        ),
    )
    .await?;

    // Test paste_all_selections_after
    // Primary selection is last selection pasted for previous primary.
    test_with_config(
        AppBuilder::new().with_config(config.clone()),
        (
            indoc! {"\
            #(|one)#_cat
            #[|two]#_dog
            #(|three)#_cow
            "},
            "y<A-p>",
            indoc! {"\
            one#(one|)##(two|)##(three|)#_cat
            two#(one|)##(two|)##[three|]#_dog
            three#(one|)##(two|)##(three|)#_cow
            "},
        ),
    )
    .await?;

    Ok(())
}
