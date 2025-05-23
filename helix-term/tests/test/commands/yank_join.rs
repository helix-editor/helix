use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn no_flags() -> anyhow::Result<()> {
    test((
        indoc! {"\
                #[o|]#ne
                #(t|)#wo
                three"
        },
        ":yank-join<ret>p",
        indoc! {"\
                o#[o
                t|]#ne
                t#(o
                t|)#wo
                three"
        },
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn separator_flag() -> anyhow::Result<()> {
    test((
        indoc! {"\
                #[o|]#ne
                #(t|)#wo
                three"
        },
        ":yank-join --separator x<ret>p",
        indoc! {"\
                o#[oxt|]#ne
                t#(oxt|)#wo
                three"
        },
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn register_flag() -> anyhow::Result<()> {
    test((
        indoc! {"\
                #[o|]#ne
                #(t|)#wo
                three"
        },
        // 1. Yank the two selections into register `x`
        // 2. Move cursor down + right
        // 3. Yank two other selections into the default register
        // 4. Move cursor back up + left, to the original position
        // 5. Pasting from register `x` will paste as if actions 2, 3 and 4 never occured
        ":yank-join --register x<ret>jl:yank-join<ret>kh\"xp",
        indoc! {"\
                o#[o
                t|]#ne
                t#(o
                t|)#wo
                three"
        },
    ))
    .await?;

    Ok(())
}
