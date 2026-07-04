use anyhow::{bail, Result};

#[cfg(target_os = "linux")]
use landlock::{
    path_beneath_rules, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus,
    ABI,
};

#[cfg(target_os = "linux")]
pub fn landlock(config: &helix_view::editor::SandboxConfig) -> Result<()> {
    let abi = ABI::V1;

    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))?
        .create()?
        // TODO: generally required paths need some tuning
        // .add_rules(path_beneath_rules(&["/dev/null"], AccessFs::from_read(abi)))?
        .add_rules(path_beneath_rules(
            &[
                "/dev",
                "/etc/ssl/certs",
                "/lib",
                "/lib64",
                "/proc",
                "/tmp",
                "/usr",
            ],
            AccessFs::from_all(abi),
        ))?
        .add_rules(path_beneath_rules(
            helix_loader::runtime_dirs(),
            AccessFs::from_read(abi),
        ))?
        .add_rules(path_beneath_rules(
            &config.extra_readonly_paths,
            AccessFs::from_read(abi),
        ))?
        .add_rules(path_beneath_rules(
            &config.extra_writable_paths,
            AccessFs::from_all(abi),
        ))?
        .add_rules(path_beneath_rules(
            &[
                helix_loader::cache_dir(),
                helix_loader::data_dir(),
                helix_loader::config_dir(), // SAFETY: read-only?
                helix_stdx::env::current_working_dir(),
            ],
            AccessFs::from_all(abi),
        ))?
        .restrict_self()?;

    match status.ruleset {
        RulesetStatus::FullyEnforced => Ok(()),
        _ => bail!("Landlock ruleset could not be fully enforced."),
    }
}
