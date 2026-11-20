//! The test documents have no grammar so foldable blocks are computed from
//! indentation: folding `fn a() {` hides `x` and `y` and keeps the closing `}`
//! visible (`fn a() {⋯}`).

use super::*;

// `j`/`k` remember the column they started from; the key sequences that end
// with them also press `;` (collapse selection) to drop it before comparing.

#[tokio::test(flavor = "multi_thread")]
async fn vertical_movement_skips_folds() -> anyhow::Result<()> {
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zaj;",
        "fn a() {\n  x\n  y\n}\n#[z|]#\n",
    ))
    .await?;
    test((
        "fn a() {\n  x\n  y\n}\n#[z|]#\n",
        "ggzaGk;",
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
    ))
    .await?;
    // `za` twice opens the fold again
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zazaj;",
        "fn a() {\n#[ |]# x\n  y\n}\nz\n",
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn horizontal_movement_skips_folds() -> anyhow::Result<()> {
    // `{` -> fold marker -> `}`
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zagll",
        "fn a() {#[\n|]#  x\n  y\n}\nz\n",
    ))
    .await?;
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zaglll",
        "fn a() {\n  x\n  y\n#[}|]#\nz\n",
    ))
    .await?;
    // and back
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zaglllhh",
        "fn a() #[{|]#\n  x\n  y\n}\nz\n",
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn selection_inside_fold_opens_it() -> anyhow::Result<()> {
    // searching for `y` opens the fold: afterwards `j` from the first line no longer skips it
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "za/y<ret>",
        "fn a() {\n  x\n  #[y|]#\n}\nz\n",
    ))
    .await?;
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "za/y<ret>ggj;",
        "fn a() {\n#[ |]# x\n  y\n}\nz\n",
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_selected_lines() -> anyhow::Result<()> {
    // fold the lines of `x` and `y`: `x` stays visible as the header
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "jxxzfggjj;",
        "fn a() {\n  x\n  y\n#[}|]#\nz\n",
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_all_and_unfold_all() -> anyhow::Result<()> {
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zMj;",
        "fn a() {\n  x\n  y\n}\n#[z|]#\n",
    ))
    .await?;
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zMzRj;",
        "fn a() {\n#[ |]# x\n  y\n}\nz\n",
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn folds_follow_edits() -> anyhow::Result<()> {
    // inserting a line above the fold moves the fold with the text
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zaOtop<esc>ggjj;",
        "top\nfn a() {\n  x\n  y\n}\n#[z|]#\n",
    ))
    .await?;
    // undo keeps the fold intact as well
    test((
        "#[f|]#n a() {\n  x\n  y\n}\nz\n",
        "zaOtop<esc>uj;",
        "fn a() {\n  x\n  y\n}\n#[z|]#\n",
    ))
    .await?;
    Ok(())
}
