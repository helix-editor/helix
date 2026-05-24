use super::*;

// ─── goto_next_conflict (]X) ──────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn goto_next_conflict_from_before() -> anyhow::Result<()> {
    // Cursor before the conflict → jumps to the <<<<<<< line
    // Note: #[<|]# selects the first '<', so the remaining text on that line
    // must be "<<<<<< HEAD" (6 '<') to produce "<<<<<<< HEAD" (7 '<') total.
    test((
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            after
        "},
        "]=",
        indoc::indoc! {"\
            before
            #[<|]#<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn goto_next_conflict_from_inside() -> anyhow::Result<()> {
    // Cursor inside the first conflict → jumps to the second conflict
    test((
        indoc::indoc! {"\
            <<<<<<< HEAD
            #[o|]#urs
            =======
            theirs
            >>>>>>> branch
            <<<<<<< HEAD
            ours2
            =======
            theirs2
            >>>>>>> branch
        "},
        "]=",
        indoc::indoc! {"\
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            #[<|]#<<<<<< HEAD
            ours2
            =======
            theirs2
            >>>>>>> branch
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn goto_next_conflict_no_conflict() -> anyhow::Result<()> {
    // No conflict markers → cursor stays put
    test(("#[h|]#ello\nworld\n", "]=", "#[h|]#ello\nworld\n")).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn goto_next_conflict_after_last() -> anyhow::Result<()> {
    // Cursor after the last conflict → no movement
    test((
        indoc::indoc! {"\
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            #[a|]#fter
        "},
        "]=",
        indoc::indoc! {"\
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            #[a|]#fter
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn goto_next_conflict_count() -> anyhow::Result<()> {
    // 2]X skips two conflicts at once
    test((
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< HEAD
            a
            =======
            b
            >>>>>>> branch
            between
            <<<<<<< HEAD
            c
            =======
            d
            >>>>>>> branch
            after
        "},
        "2]=",
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            a
            =======
            b
            >>>>>>> branch
            between
            #[<|]#<<<<<< HEAD
            c
            =======
            d
            >>>>>>> branch
            after
        "},
    ))
    .await?;

    Ok(())
}

// ─── goto_prev_conflict ([X) ──────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn goto_prev_conflict_from_after() -> anyhow::Result<()> {
    // Cursor after the conflict → jumps to the <<<<<<< line
    test((
        indoc::indoc! {"\
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            #[a|]#fter
        "},
        "[=",
        indoc::indoc! {"\
            #[<|]#<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn goto_prev_conflict_from_inside() -> anyhow::Result<()> {
    // Cursor inside the second conflict (past its <<<<<<< line) → jumps to that
    // conflict's own start (so navigation lands on its <<<<<<< line)
    test((
        indoc::indoc! {"\
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            <<<<<<< HEAD
            #[o|]#urs2
            =======
            theirs2
            >>>>>>> branch
        "},
        "[=",
        indoc::indoc! {"\
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            #[<|]#<<<<<< HEAD
            ours2
            =======
            theirs2
            >>>>>>> branch
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn goto_prev_conflict_on_marker_line() -> anyhow::Result<()> {
    // Cursor exactly on the <<<<<<< line of the second conflict → jumps to the
    // first conflict (since prev_conflict uses strict-less-than on start)
    test((
        indoc::indoc! {"\
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            #[<|]#<<<<<< HEAD
            ours2
            =======
            theirs2
            >>>>>>> branch
        "},
        "[=",
        indoc::indoc! {"\
            #[<|]#<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
            <<<<<<< HEAD
            ours2
            =======
            theirs2
            >>>>>>> branch
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn goto_prev_conflict_before_first() -> anyhow::Result<()> {
    // Cursor before any conflict → no movement
    test((
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
        "},
        "[=",
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< HEAD
            ours
            =======
            theirs
            >>>>>>> branch
        "},
    ))
    .await?;

    Ok(())
}

// ─── jj-style markers ─────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn goto_conflict_jj_style() -> anyhow::Result<()> {
    // jj-style markers have change IDs and commit messages as labels on every
    // marker line; navigation should work identically
    test((
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< ouyysnvk c9a24f82 \"first version\"
            1st version
            ||||||| zxwrknxy 62f152a0 \"base\"
            original
            =======
            2nd version
            >>>>>>> kyqztmxm cf165681 \"second version\"
            after
        "},
        "]=",
        indoc::indoc! {"\
            before
            #[<|]#<<<<<< ouyysnvk c9a24f82 \"first version\"
            1st version
            ||||||| zxwrknxy 62f152a0 \"base\"
            original
            =======
            2nd version
            >>>>>>> kyqztmxm cf165681 \"second version\"
            after
        "},
    ))
    .await?;

    Ok(())
}
