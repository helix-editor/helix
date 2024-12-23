use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent() -> anyhow::Result<()> {
    let app = || AppBuilder::new().with_file("foo.yaml", None);

    let below_tests = [
        (
            indoc! {r##"
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
            "##},
            "o",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {r##"
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
            "##},
            "o",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {r##"
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
            "##},
            "o",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {r##"
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
            "##},
            "o",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {r##"
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
            "##},
            "o",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {"\
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
            "},
            "o",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {"\
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
            "},
            "o",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {"\
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
            "},
            "o",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {"\
                top:
                  baz: foo
                  bax: |
                    some
                    multi
                    line
                    string#[\n|]#
                fook:
            "},
            "o",
            indoc! {"\
                top:
                  baz: foo
                  bax: |
                    some
                    multi
                    line
                    string
                    #[\n|]#
                fook:
            "},
        ),
        (
            indoc! {"\
                top:
                  baz: foo
                  bax: >
                    some
                    multi
                    line#[\n|]#
                    string
                fook:
            "},
            "o",
            indoc! {"\
                top:
                  baz: foo
                  bax: >
                    some
                    multi
                    line
                    #[\n|]#
                    string
                fook:
            "},
        ),
        (
            indoc! {"\
                top:
                  baz: foo
                  bax: >#[\n|]#
                fook:
            "},
            "o",
            indoc! {"\
                top:
                  baz: foo
                  bax: >
                    #[\n|]#
                fook:
            "},
        ),
        (
            indoc! {"\
              - top:#[\n|]#
                  baz: foo
                  bax: foox
                fook:
            "},
            "o",
            indoc! {"\
              - top:
                  #[\n|]#
                  baz: foo
                  bax: foox
                fook:
            "},
        ),
        (
            indoc! {"\
              - top:
                  baz: foo#[\n|]#
                  bax: foox
                fook:
            "},
            "o",
            indoc! {"\
              - top:
                  baz: foo
                  #[\n|]#
                  bax: foox
                fook:
            "},
        ),
        (
            indoc! {"\
              - top:
                  baz: foo
                  bax: foox#[\n|]#
                fook:
            "},
            "o",
            indoc! {"\
              - top:
                  baz: foo
                  bax: foox
                  #[\n|]#
                fook:
            "},
        ),
        (
            indoc! {"\
              top:
                baz:
                  - one: two#[\n|]#
                    three: four
                  - top:
                      baz: foo
                      bax: foox
            "},
            "o",
            indoc! {"\
              top:
                baz:
                  - one: two
                    #[\n|]#
                    three: four
                  - top:
                      baz: foo
                      bax: foox
            "},
        ),
        // yaml map without a key
        (
            indoc! {"\
              top:#[\n|]#
            "},
            "o",
            indoc! {"\
              top:
                #[\n|]#
            "},
        ),
        (
            indoc! {"\
              top#[:|]#
              bottom: withvalue
            "},
            "o",
            indoc! {"\
              top:
                #[\n|]#
              bottom: withvalue
            "},
        ),
        (
            indoc! {"\
              bottom: withvalue
              top#[:|]#
            "},
            "o",
            indoc! {"\
              bottom: withvalue
              top:
                #[\n|]#
            "},
        ),
    ];

    for test in below_tests {
        test_with_config(app(), test).await?;
    }

    let above_tests = [
        (
            indoc! {r##"
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
            "##},
            "O",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {r##"
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
            "##},
            "O",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {r##"
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
            "##},
            "O",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {r##"
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
            "##},
            "O",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {r##"
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
            "##},
            "O",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {"\
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
            "},
            "O",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {"\
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
            "},
            "O",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {"\
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
            "},
            "O",
            indoc! {"\
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
            "},
        ),
        (
            indoc! {"\
                top:
                  baz: foo
                  bax: |
                    some
                    multi
                    line
                    string#[\n|]#
                fook:
            "},
            "O",
            indoc! {"\
                top:
                  baz: foo
                  bax: |
                    some
                    multi
                    line
                    #[\n|]#
                    string
                fook:
            "},
        ),
        (
            indoc! {"\
                top:
                  baz: foo
                  bax: >
                    some#[\n|]#
                    multi
                    line
                    string
                fook:
            "},
            "O",
            indoc! {"\
                top:
                  baz: foo
                  bax: >
                    #[\n|]#
                    some
                    multi
                    line
                    string
                fook:
            "},
        ),
        (
            indoc! {"\
                top:
                  baz: foo
                  bax: >
                fook:#[\n|]#
            "},
            "O",
            indoc! {"\
                top:
                  baz: foo
                  bax: >
                    #[\n|]#
                fook:
            "},
        ),
        (
            indoc! {"\
              - top:
                  baz: foo#[\n|]#
                  bax: foox
                fook:
            "},
            "O",
            indoc! {"\
              - top:
                  #[\n|]#
                  baz: foo
                  bax: foox
                fook:
            "},
        ),
        (
            indoc! {"\
              - top:
                  baz: foo
                  bax: foox
                fook:#[\n|]#
            "},
            "O",
            indoc! {"\
              - top:
                  baz: foo
                  bax: foox
                  #[\n|]#
                fook:
            "},
        ),
        (
            indoc! {"\
              top:
                baz:
                  - one: two#[\n|]#
                    three: four
                  - top:
                      baz: foo
                      bax: foox
            "},
            "O",
            indoc! {"\
              top:
                baz:
                  #[\n|]#
                  - one: two
                    three: four
                  - top:
                      baz: foo
                      bax: foox
            "},
        ),
        // yaml map without a key
        (
            indoc! {"\
              top:#[\n|]#
            "},
            "O",
            indoc! {"\
              #[\n|]#
              top:
            "},
        ),
        (
            indoc! {"\
              bottom: withvalue
              top#[:|]#
            "},
            "O",
            indoc! {"\
              bottom: withvalue
              #[\n|]#
              top:
            "},
        ),
        (
            indoc! {"\
              top:
              bottom:#[ |]#withvalue
            "},
            "O",
            indoc! {"\
              top:
                #[\n|]#
              bottom: withvalue
            "},
        ),
    ];

    for test in above_tests {
        test_with_config(app(), test).await?;
    }

    let enter_tests = [
        (
            indoc! {r##"
                foo: #[b|]#ar
            "##},
            "i<ret>",
            indoc! {"\
                foo:
                  #[|b]#ar
            "},
        ),
        (
            indoc! {"\
                foo:#[\n|]#
            "},
            "i<ret>",
            indoc! {"\
                foo:
                  #[|\n]#
            "},
        ),
    ];

    for test in enter_tests {
        test_with_config(app(), test).await?;
    }

    Ok(())
}
