use super::*;

const IN: &str = indoc! {"
            #[pub fn docgen() -> Result<(), DynError> {
                use crate::docgen::*;
                write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
                write(STATIC_COMMANDS_MD_OUTPUT, &static_commands()?);
                write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
                Ok(())
            }\n|]#"};

#[tokio::test(flavor = "multi_thread")]
async fn left() -> anyhow::Result<()> {
    test((
        IN,
        ":align-text-left<ret>",
        indoc! {"\
            #[pub fn docgen() -> Result<(), DynError> {
            use crate::docgen::*;
            write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
            write(STATIC_COMMANDS_MD_OUTPUT, &static_commands()?);
            write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
            Ok(())
            }\n|]#"},
    ))
    .await?;

    Ok(())
}
#[tokio::test(flavor = "multi_thread")]
async fn center() -> anyhow::Result<()> {
    test((
        IN,
        ":align-text-center<ret>",
        indoc! {"\
            #[                   pub fn docgen() -> Result<(), DynError> {
                                         use crate::docgen::*;
                        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
                         write(STATIC_COMMANDS_MD_OUTPUT, &static_commands()?);
                           write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
                                                 Ok(())
                                                   }\n|]#"},
    ))
    .await?;

    Ok(())
}
#[tokio::test(flavor = "multi_thread")]
async fn right() -> anyhow::Result<()> {
    test((
        IN,
        ":align-text-right<ret>",
        indoc! {"\
            #[                                       pub fn docgen() -> Result<(), DynError> {
                                                                       use crate::docgen::*;
                                    write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
                                      write(STATIC_COMMANDS_MD_OUTPUT, &static_commands()?);
                                           write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
                                                                                      Ok(())
                                                                                           }\n|]#"},
    ))
    .await?;

    Ok(())
}
