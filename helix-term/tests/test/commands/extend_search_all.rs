use super::*;

use helix_core::hashmap;
use helix_term::keymap;
use helix_view::document::Mode;

#[tokio::test(flavor = "multi_thread")]
async fn extend_search_all() -> anyhow::Result<()> {
    let mut config = Config::default();
    config.keys.insert(
        Mode::Normal,
        keymap!({"Normal Mode"
            // Typically you would want to bind these to the same key.
            // e.g. "key" = ["search_selection", "extend_search_all"]
            "a" => search_selection,
            "b" => extend_search_all,
        }),
    );

    // Test extend_search_all
    test_with_config(
        AppBuilder::new().with_config(config.clone()),
        (
            indoc! {"\
            one_cat one cat three
            two_dog two dog xthreex
            #[three|]#_cow three cow
            "},
            "ab",
            indoc! {"\
            one_cat one cat #(three|)#
            two_dog two dog x#(three|)#x
            #[three|]#_cow #(three|)# cow
            "},
        ),
    )
    .await?;

    // Test extend_search_all preserves selection direction.
    test_with_config(
        AppBuilder::new().with_config(config.clone()),
        (
            indoc! {"\
            one_cat one cat three
            two_dog two dog xthreex
            #[|three]#_cow three cow
            "},
            "ab",
            indoc! {"\
            one_cat one cat #(|three)#
            two_dog two dog x#(|three)#x
            #[|three]#_cow #(|three)# cow
            "},
        ),
    )
    .await?;

    // Test extend_search_all with multiple queries.
    test_with_config(
        AppBuilder::new().with_config(config.clone()),
        (
            indoc! {"\
            one_#(|cat)# one cat three
            two_dog two dog xthreex
            #[|three]#_cow three cow cattle
            "},
            "ab",
            indoc! {"\
            one_#(|cat)# one #(|cat)# #(|three)#
            two_dog two dog x#(|three)#x
            #[|three]#_cow #(|three)# cow #(|cat)#tle
            "},
        ),
    )
    .await?;

    Ok(())
}
