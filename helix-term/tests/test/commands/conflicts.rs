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

// ─── Resolution commands (<space>x{c,i,b,a}) ─────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn accept_current_two_way() -> anyhow::Result<()> {
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            #[o|]#urs line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xc",
        indoc::indoc! {"\
            before
            #[o|]#urs line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_incoming_two_way() -> anyhow::Result<()> {
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            #[o|]#urs line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xi",
        indoc::indoc! {"\
            before
            #[t|]#heirs line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_all_two_way() -> anyhow::Result<()> {
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            #[o|]#urs line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xa",
        indoc::indoc! {"\
            before
            #[o|]#urs line
            theirs line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_base_three_way() -> anyhow::Result<()> {
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            #[o|]#urs line
            ||||||| base
            base line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xb",
        indoc::indoc! {"\
            before
            #[b|]#ase line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_current_cursor_outside_conflict() -> anyhow::Result<()> {
    // Cursor outside any conflict → no change
    test((
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< HEAD
            ours line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xc",
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< HEAD
            ours line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
    ))
    .await?;

    Ok(())
}

// ─── accept_change_at_cursor (<space>xx) ──────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_on_current_marker() -> anyhow::Result<()> {
    // Cursor on <<<<<<< line → accepts current
    test((
        indoc::indoc! {"\
            before
            #[<|]#<<<<<< HEAD
            ours line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[o|]#urs line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_in_current_content() -> anyhow::Result<()> {
    // Cursor in ours content → accepts current
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            #[o|]#urs line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[o|]#urs line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_on_separator() -> anyhow::Result<()> {
    // Cursor on ======= line → no-op
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            ours line
            #[=|]#======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            ours line
            #[=|]#======
            theirs line
            >>>>>>> branch
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_in_incoming_content() -> anyhow::Result<()> {
    // Cursor in theirs content → accepts incoming
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            ours line
            =======
            #[t|]#heirs line
            >>>>>>> branch
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[t|]#heirs line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_on_incoming_marker() -> anyhow::Result<()> {
    // Cursor on >>>>>>> line → accepts incoming
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            ours line
            =======
            theirs line
            #[>|]#>>>>>> branch
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[t|]#heirs line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_on_base_marker() -> anyhow::Result<()> {
    // Cursor on ||||||| line → accepts base
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            ours line
            #[||]#|||||| base
            base line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[b|]#ase line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_in_base_content() -> anyhow::Result<()> {
    // Cursor in base content → accepts base
    test((
        indoc::indoc! {"\
            before
            <<<<<<< HEAD
            ours line
            ||||||| base
            #[b|]#ase line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[b|]#ase line
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_outside_conflict() -> anyhow::Result<()> {
    // Cursor outside any conflict → no-op
    test((
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< HEAD
            ours line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            #[b|]#efore
            <<<<<<< HEAD
            ours line
            =======
            theirs line
            >>>>>>> branch
            after
        "},
    ))
    .await?;

    Ok(())
}

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

// ─── jj snapshot format: resolution commands ──────────────────────────────────
//
// jj conflicts look like:
//
//   <<<<<<< Conflict 1 of 1
//   +++++++ side #1
//   s1
//   ------- base
//   base
//   +++++++ side #2
//   s2
//   >>>>>>> Conflict 1 of 1 ends
//
// Sections (0-indexed): 0=Side(s1), 1=Base(base), 2=Side(s2)
// accept_current  → first Side → s1
// accept_incoming → last  Side → s2
// accept_base     → first Base → base
// accept_all      → all  Sides → s1 + s2

#[tokio::test(flavor = "multi_thread")]
async fn accept_current_jj() -> anyhow::Result<()> {
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            +++++++ side #1
            #[s|]#1
            ------- base
            base
            +++++++ side #2
            s2
            >>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xc",
        indoc::indoc! {"\
            before
            #[s|]#1
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_incoming_jj() -> anyhow::Result<()> {
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            +++++++ side #1
            #[s|]#1
            ------- base
            base
            +++++++ side #2
            s2
            >>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xi",
        indoc::indoc! {"\
            before
            #[s|]#2
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_base_jj() -> anyhow::Result<()> {
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            +++++++ side #1
            #[s|]#1
            ------- base
            base
            +++++++ side #2
            s2
            >>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xb",
        indoc::indoc! {"\
            before
            #[b|]#ase
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_all_jj() -> anyhow::Result<()> {
    // accept_all concatenates all Side sections (skips Base)
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            +++++++ side #1
            #[s|]#1
            ------- base
            base
            +++++++ side #2
            s2
            >>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xa",
        indoc::indoc! {"\
            before
            #[s|]#1
            s2
            after
        "},
    ))
    .await?;

    Ok(())
}

// ─── jj snapshot format: accept_change_at_cursor (<space>xx) ──────────────────

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_jj_first_side_marker() -> anyhow::Result<()> {
    // Cursor on first +++++++ line → accepts that side (section 0)
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            #[+|]#++++++ side #1
            s1
            ------- base
            base
            +++++++ side #2
            s2
            >>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[s|]#1
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_jj_first_side_content() -> anyhow::Result<()> {
    // Cursor in first side content → accepts section 0
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            +++++++ side #1
            #[s|]#1
            ------- base
            base
            +++++++ side #2
            s2
            >>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[s|]#1
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_jj_base_marker() -> anyhow::Result<()> {
    // Cursor on ------- line → accepts base section
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            +++++++ side #1
            s1
            #[-|]#------ base
            base
            +++++++ side #2
            s2
            >>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[b|]#ase
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_jj_second_side_marker() -> anyhow::Result<()> {
    // Cursor on second +++++++ line → accepts that side (section 2)
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            +++++++ side #1
            s1
            ------- base
            base
            #[+|]#++++++ side #2
            s2
            >>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[s|]#2
            after
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn accept_at_cursor_jj_close_marker() -> anyhow::Result<()> {
    // Cursor on >>>>>>> line → accepts last section (section 2, the last Side)
    test((
        indoc::indoc! {"\
            before
            <<<<<<< Conflict 1 of 1
            +++++++ side #1
            s1
            ------- base
            base
            +++++++ side #2
            s2
            #[>|]#>>>>>> Conflict 1 of 1 ends
            after
        "},
        "<space>xx",
        indoc::indoc! {"\
            before
            #[s|]#2
            after
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
