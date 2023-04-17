use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent() -> anyhow::Result<()> {
    let app = || AppBuilder::new().with_file("foo.yaml", None);

    let below_tests = [
        (
            helpers::platform_line(indoc! {r##"
                #[t|]#op:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  #[\n|]#
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {r##"
                top:
                  b#[a|]#z: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  #[\n|]#
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {r##"
                top:
                  baz: foo
                  bazi#[:|]#
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    #[\n|]#
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {r##"
                top:
                  baz: foo
                  bazi:
                    more: #[yes|]#
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    #[\n|]#
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {r##"
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: becaus#[e|]#
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                    #[\n|]#
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:#[\n|]#
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    #[\n|]#
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1#[\n|]#
                    - 2
                  bax: foox
                fook:
            "}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    #[\n|]#
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:#[\n|]#
            "}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
                  #[\n|]#
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: |
                    some
                    multi
                    line
                    string#[\n|]#
                fook:
            "}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: |
                    some
                    multi
                    line
                    string
                    #[\n|]#
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: >
                    some
                    multi
                    line#[\n|]#
                    string
                fook:
            "}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: >
                    some
                    multi
                    line
                    #[\n|]#
                    string
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: >#[\n|]#
                fook:
            "}),
            "o",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: >
                    #[\n|]#
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              - top:#[\n|]#
                  baz: foo
                  bax: foox
                fook:
            "}),
            "o",
            helpers::platform_line(indoc! {"\
              - top:
                  #[\n|]#
                  baz: foo
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              - top:
                  baz: foo#[\n|]#
                  bax: foox
                fook:
            "}),
            "o",
            helpers::platform_line(indoc! {"\
              - top:
                  baz: foo
                  #[\n|]#
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              - top:
                  baz: foo
                  bax: foox#[\n|]#
                fook:
            "}),
            "o",
            helpers::platform_line(indoc! {"\
              - top:
                  baz: foo
                  bax: foox
                  #[\n|]#
                fook:
            "}),
        ),
    ];

    for test in below_tests {
        test_with_config(app(), test).await?;
    }

    let above_tests = [
        (
            helpers::platform_line(indoc! {r##"
                #[t|]#op:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "O",
            helpers::platform_line(indoc! {"\
                #[\n|]#
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {r##"
                top:
                  b#[a|]#z: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  #[\n|]#
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {r##"
                top:
                  baz: foo
                  bazi#[:|]#
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  #[\n|]#
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {r##"
                top:
                  baz: foo
                  bazi:
                    more: #[yes|]#
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    #[\n|]#
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {r##"
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: becaus#[e|]#
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "##}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    #[\n|]#
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:#[\n|]#
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                    #[\n|]#
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1#[\n|]#
                    - 2
                  bax: foox
                fook:
            "}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    #[\n|]#
                    - 1
                    - 2
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                fook:#[\n|]#
            "}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bazi:
                    more: yes
                    why: because
                  quux:
                    - 1
                    - 2
                  bax: foox
                  #[\n|]#
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: |
                    some
                    multi
                    line
                    string#[\n|]#
                fook:
            "}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: |
                    some
                    multi
                    line
                    #[\n|]#
                    string
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: >
                    some#[\n|]#
                    multi
                    line
                    string
                fook:
            "}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: >
                    #[\n|]#
                    some
                    multi
                    line
                    string
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: >
                fook:#[\n|]#
            "}),
            "O",
            helpers::platform_line(indoc! {"\
                top:
                  baz: foo
                  bax: >
                    #[\n|]#
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              - top:
                  baz: foo#[\n|]#
                  bax: foox
                fook:
            "}),
            "O",
            helpers::platform_line(indoc! {"\
              - top:
                  #[\n|]#
                  baz: foo
                  bax: foox
                fook:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              - top:
                  baz: foo
                  bax: foox
                fook:#[\n|]#
            "}),
            "O",
            helpers::platform_line(indoc! {"\
              - top:
                  baz: foo
                  bax: foox
                  #[\n|]#
                fook:
            "}),
        ),
    ];

    for test in above_tests {
        test_with_config(app(), test).await?;
    }

    Ok(())
}
