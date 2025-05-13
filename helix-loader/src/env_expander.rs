use regex::Regex;
use std::env;
use std::str::FromStr;

#[derive(Debug)]
enum AllowedEnv {
    Home,
    User,
    ConfigDir,
}

impl FromStr for AllowedEnv {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HOME" => Ok(AllowedEnv::Home),
            "USER" => Ok(AllowedEnv::User),
            "CONFIG_DIR" => Ok(AllowedEnv::ConfigDir),
            _ => Err(()),
        }
    }
}

/// Expands AllowedEnv variables in the input string.
/// Allowed patterns are in the format `${VAR_NAME}`.
pub fn expand_env_vars(input: &str) -> String {
    let re = Regex::new(r"\$\{(\w+)\}").expect("Invalid regex");

    re.replace_all(input, |caps: &regex::Captures| {
        let var_name = &caps[1];
        if let Ok(_allowed) = var_name.parse::<AllowedEnv>() {
            env::var(var_name).unwrap_or_else(|_| "".to_string())
        } else {
            caps.get(0)
                .map_or("".to_string(), |m| m.as_str().to_string())
        }
    })
    .into_owned()
}

pub fn get_env_expanded_toml<T>(toml_str: &str) -> Result<T, toml::de::Error>
where
    T: serde::de::DeserializeOwned
{
    match std::env::consts::OS {
        "linux" => toml::from_str(&expand_env_vars(toml_str)),
        _ => toml::from_str(toml_str),
    }

}

#[cfg(test)]
mod env_expander_tests {
    use toml::Value;
    use std::env;
    use super::get_env_expanded_toml;

    #[test]
    fn expand_envs_and_parse_toml() {
        let toml_text = r#"
            title = "My App"
            dir = "${HOME}/project/source"
            user = "${USER}"
            config = "${CONFIG_DIR}"
            unknown = "${NOT_ALLOWED}"
        "#;

        env::set_var("HOME", "/home/example");
        env::set_var("USER", "example_user");
        env::set_var("CONFIG_DIR", "/etc/myapp");

        let parsed: Value = get_env_expanded_toml(toml_text).unwrap();
        assert_eq!(
            parsed["dir"],
            "/home/example/project/source".into()
        );
        assert_eq!(
            parsed["user"],
            "example_user".into()
        );
        assert_eq!(
            parsed["config"],
            "/etc/myapp".into()
        );
        assert_eq!(
            parsed["unknown"],
            "${NOT_ALLOWED}".into()
        );
    }
}

