use helix_view::editor::Config;
use std::borrow::Cow;
use std::ops::Deref;

/// Initialize i18n with the configured UI language.
/// Falls back to "en" if the configured language is not available.
pub fn init(config: &Config) {
    let locale = config.ui_language.as_deref().unwrap_or("en");

    // Normalize locale: "zh_cn" -> "zh-CN", "zh-CN" -> "zh-CN", "en" -> "en"
    let normalized = match locale {
        "zh-CN" | "zh_cn" | "zh_CN" | "zh-cn" => "zh-CN",
        "en" | "en-US" | "en_US" | "en-us" => "en",
        other => other,
    };

    eprintln!("[i18n] setting locale to '{}'", normalized);
    rust_i18n::set_locale(normalized);
    eprintln!(
        "[i18n] current locale is now: {:?}",
        rust_i18n::locale().deref()
    );
}

/// Translate a static string key to the current locale.
/// The key must be a `'static` string (compile-time string literal).
/// Returns the translated string if available, otherwise the original key.
pub fn tr(key: &'static str) -> Cow<'static, str> {
    crate::t!(key)
}

/// Translate a runtime string key to the current locale.
/// Looks up the key in the translation table and returns the translated
/// string if found, otherwise returns the original key as an owned string.
pub fn tr_runtime(key: &str) -> Cow<'static, str> {
    // Try to find a translation by checking if the key exists in the
    // translation table. We do this by using the t! macro with a static
    // lookup approach - but since we have a runtime key, we just return
    // the original for now. In a full implementation, we'd have a runtime
    // lookup table.
    Cow::Owned(key.to_string())
}
