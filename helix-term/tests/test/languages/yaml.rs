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
        (
            helpers::platform_line(indoc! {"\
              top:
                baz:
                  - one: two#[\n|]#
                    three: four
                  - top:
                      baz: foo
                      bax: foox
            "}),
            "o",
            helpers::platform_line(indoc! {"\
              top:
                baz:
                  - one: two
                    #[\n|]#
                    three: four
                  - top:
                      baz: foo
                      bax: foox
            "}),
        ),
        // yaml map without a key
        (
            helpers::platform_line(indoc! {"\
              top:#[\n|]#
            "}),
            "o",
            helpers::platform_line(indoc! {"\
              top:
                #[\n|]#
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              top#[:|]#
              bottom: withvalue
            "}),
            "o",
            helpers::platform_line(indoc! {"\
              top:
                #[\n|]#
              bottom: withvalue
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              bottom: withvalue
              top#[:|]#
            "}),
            "o",
            helpers::platform_line(indoc! {"\
              bottom: withvalue
              top:
                #[\n|]#
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
        (
            helpers::platform_line(indoc! {"\
              top:
                baz:
                  - one: two#[\n|]#
                    three: four
                  - top:
                      baz: foo
                      bax: foox
            "}),
            "O",
            helpers::platform_line(indoc! {"\
              top:
                baz:
                  #[\n|]#
                  - one: two
                    three: four
                  - top:
                      baz: foo
                      bax: foox
            "}),
        ),
        // yaml map without a key
        (
            helpers::platform_line(indoc! {"\
              top:#[\n|]#
            "}),
            "O",
            helpers::platform_line(indoc! {"\
              #[\n|]#
              top:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              bottom: withvalue
              top#[:|]#
            "}),
            "O",
            helpers::platform_line(indoc! {"\
              bottom: withvalue
              #[\n|]#
              top:
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
              top:
              bottom:#[ |]#withvalue
            "}),
            "O",
            helpers::platform_line(indoc! {"\
              top:
                #[\n|]#
              bottom: withvalue
            "}),
        ),
    ];

    for test in above_tests {
        test_with_config(app(), test).await?;
    }

    let enter_tests = [
        (
            helpers::platform_line(indoc! {r##"
                foo: #[b|]#ar
            "##}),
            "i<ret>",
            helpers::platform_line(indoc! {"\
                foo: 
                  #[|b]#ar
            "}),
        ),
        (
            helpers::platform_line(indoc! {"\
                foo:#[\n|]#
            "}),
            "i<ret>",
            helpers::platform_line(indoc! {"\
                foo:
                  #[|\n]#
            "}),
        ),
    ];

    for test in enter_tests {
        test_with_config(app(), test).await?;
    }

    Ok(())
}
