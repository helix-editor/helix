use crate::syntax::{config::Configuration, Loader, LoaderError};
use serde::de::Error;

/// Language configuration based on built-in languages.toml.
pub fn default_lang_config() -> Configuration {
    helix_loader::config::default_lang_config()
        .try_into()
        .expect("Could not deserialize built-in languages.toml")
}

/// Language configuration loader based on built-in languages.toml.
pub fn default_lang_loader() -> Loader {
    Loader::new(default_lang_config()).expect("Could not compile loader for default config")
}

#[derive(Debug)]
pub enum LanguageLoaderError {
    DeserializeError(toml::de::Error),
    LoaderError(LoaderError),
}

impl std::fmt::Display for LanguageLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeserializeError(err) => write!(f, "Failed to parse language config: {err}"),
            Self::LoaderError(err) => write!(f, "Failed to compile language config: {err}"),
        }
    }
}

impl std::error::Error for LanguageLoaderError {}

/// Language configuration based on user configured languages.toml.
pub fn user_lang_config() -> Result<Configuration, toml::de::Error> {
    helix_loader::config::user_lang_config()?.try_into()
}

/// Language configuration loader based on user configured languages.toml.
pub fn user_lang_loader() -> Result<Loader, LanguageLoaderError> {
    let toml_value = helix_loader::config::user_lang_config()
        .map_err(LanguageLoaderError::DeserializeError)?;

    // Convert to string so we can get span information on parse errors
    let toml_string = toml::to_string(&toml_value)
        .map_err(|e| {
            eprintln!("Failed to serialize TOML value: {}", e);
            LanguageLoaderError::DeserializeError(toml::de::Error::custom("Failed to serialize TOML"))
        })?;

    let config: Configuration = toml::from_str(&toml_string)
        .map_err(|e| {
            // Now we have span information
            if let Some(span) = e.span() {
                let (line, col) = byte_pos_to_line_col(&toml_string, span.start);
                eprintln!("Error at line {}, column {}: {}", line, col, e);
                show_error_context(&toml_string, span);
            }
            LanguageLoaderError::DeserializeError(e)
        })?;

    Loader::new(config).map_err(LanguageLoaderError::LoaderError)
}

fn byte_pos_to_line_col(source: &str, pos: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in source.char_indices() {
        if i >= pos {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

fn show_error_context(content: &str, span: std::ops::Range<usize>) {
   let (line_num, _) = byte_pos_to_line_col(content, span.start);
   let lines: Vec<&str> = content.lines().collect();

   eprintln!("Context around error:");
   // If the error line starts with [[, it's a section header - start from that line
   // Otherwise, show the 3 preceding lines for context
   let start_line = if line_num <= lines.len() && lines[line_num-1].trim().starts_with("[[") {
       line_num
   } else {
       line_num.saturating_sub(3).max(1)
   };

   // Find the end of this section (next [[...]] or reasonable limit)
   let mut end_line = line_num + 50; // reasonable limit
   for i in (line_num + 1)..=lines.len().min(line_num + 50) {
       if i <= lines.len() && lines[i-1].trim().starts_with("[[") {
           end_line = i - 1;
           break;
       }
   }
   end_line = end_line.min(lines.len());

   for i in start_line..=end_line {
       let marker = if i == line_num { ">>> " } else { "    " };
       if i > 0 && i <= lines.len() {
           eprintln!("  {}{:4}: {}", marker, i, lines[i-1]);
       }
   }
}
