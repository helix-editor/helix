use std::path::PathBuf;

use tui::text::{Span, Spans};

use crate::ui::menu::Item;

pub struct FileInfoData {
    pub root_path: PathBuf,
    pub show_icons: bool,
}

pub struct FileInfo {
    path: PathBuf,
    icon_character: char,
}

impl FileInfo {
    pub fn new(path: PathBuf, icon_character: char) -> Self {
        Self {
            path,
            icon_character,
        }
    }

    pub fn icon_character(&self) -> char {
        self.icon_character
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Item for FileInfo {
    /// Root prefix to strip.
    type Data = FileInfoData;

    fn label(&self, data: &Self::Data) -> Spans {
        let mut result = vec![Span::raw(format!(
            "  {}",
            self.path
                .strip_prefix(&data.root_path)
                .unwrap_or(&self.path)
                .to_string_lossy()
        ))];
        if data.show_icons {
            result.push(Span::raw(self.icon_character.to_string()));
        }
        result.into()
    }
}

impl From<PathBuf> for FileInfo {
    fn from(path: PathBuf) -> Self {
        let file_name = path
            .file_name()
            .map(|x| x.to_str().map(get_icon_character_from_filename))
            .flatten()
            .flatten();
        if let Some(icon_character) = file_name {
            FileInfo {
                path,
                icon_character,
            }
        } else {
            let icon_character = path
                .extension()
                .map(|x| x.to_str().map(get_icon_character_from_extension))
                .flatten()
                .flatten()
                .unwrap_or('\u{f15b}');
            FileInfo {
                path,
                icon_character,
            }
        }
    }
}

// Adapted from https://github.com/ogham/exa/blob/45b6413fd0a82e93376a1fc2778c9188806edc7f/src/output/icons.rs

fn get_icon_character_from_filename(file: &str) -> Option<char> {
    match file {
        ".Trash" => Some('\u{f1f8}'),             // 
        ".atom" => Some('\u{e764}'),              // 
        ".bashprofile" => Some('\u{e615}'),       // 
        ".bashrc" => Some('\u{f489}'),            // 
        ".git" => Some('\u{f1d3}'),               // 
        ".gitattributes" => Some('\u{f1d3}'),     // 
        ".gitconfig" => Some('\u{f1d3}'),         // 
        ".github" => Some('\u{f408}'),            // 
        ".gitignore" => Some('\u{f1d3}'),         // 
        ".gitmodules" => Some('\u{f1d3}'),        // 
        ".rvm" => Some('\u{e21e}'),               // 
        ".vimrc" => Some('\u{e62b}'),             // 
        ".vscode" => Some('\u{e70c}'),            // 
        ".zshrc" => Some('\u{f489}'),             // 
        "Cargo.lock" => Some('\u{e7a8}'),         // 
        "bin" => Some('\u{e5fc}'),                // 
        "config" => Some('\u{e5fc}'),             // 
        "docker-compose.yml" => Some('\u{f308}'), // 
        "Dockerfile" => Some('\u{f308}'),         // 
        "ds_store" => Some('\u{f179}'),           // 
        "gitignore_global" => Some('\u{f1d3}'),   // 
        "go.mod" => Some('\u{e626}'),             // 
        "go.sum" => Some('\u{e626}'),             // 
        "gradle" => Some('\u{e256}'),             // 
        "gruntfile.coffee" => Some('\u{e611}'),   // 
        "gruntfile.js" => Some('\u{e611}'),       // 
        "gruntfile.ls" => Some('\u{e611}'),       // 
        "gulpfile.coffee" => Some('\u{e610}'),    // 
        "gulpfile.js" => Some('\u{e610}'),        // 
        "gulpfile.ls" => Some('\u{e610}'),        // 
        "hidden" => Some('\u{f023}'),             // 
        "include" => Some('\u{e5fc}'),            // 
        "lib" => Some('\u{f121}'),                // 
        "localized" => Some('\u{f179}'),          // 
        "Makefile" => Some('\u{f489}'),           // 
        "node_modules" => Some('\u{e718}'),       // 
        "npmignore" => Some('\u{e71e}'),          // 
        "PKGBUILD" => Some('\u{f303}'),           // 
        "rubydoc" => Some('\u{e73b}'),            // 
        "yarn.lock" => Some('\u{e718}'),          // 
        _ => None,
    }
}

fn get_icon_character_from_extension(ext: &str) -> Option<char> {
    match ext {
        "ai" => Some('\u{e7b4}'),             // 
        "android" => Some('\u{e70e}'),        // 
        "apk" => Some('\u{e70e}'),            // 
        "apple" => Some('\u{f179}'),          // 
        "avi" => Some('\u{f03d}'),            // 
        "avif" => Some('\u{f1c5}'),           // 
        "avro" => Some('\u{e60b}'),           // 
        "awk" => Some('\u{f489}'),            // 
        "bash" => Some('\u{f489}'),           // 
        "bash_history" => Some('\u{f489}'),   // 
        "bash_profile" => Some('\u{f489}'),   // 
        "bashrc" => Some('\u{f489}'),         // 
        "bat" => Some('\u{f17a}'),            // 
        "bats" => Some('\u{f489}'),           // 
        "bmp" => Some('\u{f1c5}'),            // 
        "bz" => Some('\u{f410}'),             // 
        "bz2" => Some('\u{f410}'),            // 
        "c" => Some('\u{e61e}'),              // 
        "c++" => Some('\u{e61d}'),            // 
        "cab" => Some('\u{e70f}'),            // 
        "cc" => Some('\u{e61d}'),             // 
        "cfg" => Some('\u{e615}'),            // 
        "class" => Some('\u{e256}'),          // 
        "clj" => Some('\u{e768}'),            // 
        "cljs" => Some('\u{e76a}'),           // 
        "cls" => Some('\u{f034}'),            // 
        "cmd" => Some('\u{e70f}'),            // 
        "coffee" => Some('\u{f0f4}'),         // 
        "conf" => Some('\u{e615}'),           // 
        "cp" => Some('\u{e61d}'),             // 
        "cpio" => Some('\u{f410}'),           // 
        "cpp" => Some('\u{e61d}'),            // 
        "cs" => Some('\u{f81a}'),             // 
        "csh" => Some('\u{f489}'),            // 
        "cshtml" => Some('\u{f1fa}'),         // 
        "csproj" => Some('\u{f81a}'),         // 
        "css" => Some('\u{e749}'),            // 
        "csv" => Some('\u{f1c3}'),            // 
        "csx" => Some('\u{f81a}'),            // 
        "cxx" => Some('\u{e61d}'),            // 
        "d" => Some('\u{e7af}'),              // 
        "dart" => Some('\u{e798}'),           // 
        "db" => Some('\u{f1c0}'),             // 
        "deb" => Some('\u{e77d}'),            // 
        "diff" => Some('\u{f440}'),           // 
        "djvu" => Some('\u{f02d}'),           // 
        "dll" => Some('\u{e70f}'),            // 
        "doc" => Some('\u{f1c2}'),            // 
        "docx" => Some('\u{f1c2}'),           // 
        "ds_store" => Some('\u{f179}'),       // 
        "DS_store" => Some('\u{f179}'),       // 
        "dump" => Some('\u{f1c0}'),           // 
        "ebook" => Some('\u{e28b}'),          // 
        "ebuild" => Some('\u{f30d}'),         // 
        "editorconfig" => Some('\u{e615}'),   // 
        "ejs" => Some('\u{e618}'),            // 
        "elm" => Some('\u{e62c}'),            // 
        "env" => Some('\u{f462}'),            // 
        "eot" => Some('\u{f031}'),            // 
        "epub" => Some('\u{e28a}'),           // 
        "erb" => Some('\u{e73b}'),            // 
        "erl" => Some('\u{e7b1}'),            // 
        "ex" => Some('\u{e62d}'),             // 
        "exe" => Some('\u{f17a}'),            // 
        "exs" => Some('\u{e62d}'),            // 
        "fish" => Some('\u{f489}'),           // 
        "flac" => Some('\u{f001}'),           // 
        "flv" => Some('\u{f03d}'),            // 
        "font" => Some('\u{f031}'),           // 
        "fs" => Some('\u{e7a7}'),             // 
        "fsi" => Some('\u{e7a7}'),            // 
        "fsx" => Some('\u{e7a7}'),            // 
        "gdoc" => Some('\u{f1c2}'),           // 
        "gem" => Some('\u{e21e}'),            // 
        "gemfile" => Some('\u{e21e}'),        // 
        "gemspec" => Some('\u{e21e}'),        // 
        "gform" => Some('\u{f298}'),          // 
        "gif" => Some('\u{f1c5}'),            // 
        "git" => Some('\u{f1d3}'),            // 
        "gitattributes" => Some('\u{f1d3}'),  // 
        "gitignore" => Some('\u{f1d3}'),      // 
        "gitmodules" => Some('\u{f1d3}'),     // 
        "go" => Some('\u{e626}'),             // 
        "gradle" => Some('\u{e256}'),         // 
        "groovy" => Some('\u{e775}'),         // 
        "gsheet" => Some('\u{f1c3}'),         // 
        "gslides" => Some('\u{f1c4}'),        // 
        "guardfile" => Some('\u{e21e}'),      // 
        "gz" => Some('\u{f410}'),             // 
        "h" => Some('\u{f0fd}'),              // 
        "hbs" => Some('\u{e60f}'),            // 
        "hpp" => Some('\u{f0fd}'),            // 
        "hs" => Some('\u{e777}'),             // 
        "htm" => Some('\u{f13b}'),            // 
        "html" => Some('\u{f13b}'),           // 
        "hxx" => Some('\u{f0fd}'),            // 
        "ico" => Some('\u{f1c5}'),            // 
        "image" => Some('\u{f1c5}'),          // 
        "img" => Some('\u{e271}'),            // 
        "iml" => Some('\u{e7b5}'),            // 
        "ini" => Some('\u{f17a}'),            // 
        "ipynb" => Some('\u{e606}'),          // 
        "iso" => Some('\u{e271}'),            // 
        "j2c" => Some('\u{f1c5}'),            // 
        "j2k" => Some('\u{f1c5}'),            // 
        "jad" => Some('\u{e256}'),            // 
        "jar" => Some('\u{e256}'),            // 
        "java" => Some('\u{e256}'),           // 
        "jfi" => Some('\u{f1c5}'),            // 
        "jfif" => Some('\u{f1c5}'),           // 
        "jif" => Some('\u{f1c5}'),            // 
        "jl" => Some('\u{e624}'),             // 
        "jmd" => Some('\u{f48a}'),            // 
        "jp2" => Some('\u{f1c5}'),            // 
        "jpe" => Some('\u{f1c5}'),            // 
        "jpeg" => Some('\u{f1c5}'),           // 
        "jpg" => Some('\u{f1c5}'),            // 
        "jpx" => Some('\u{f1c5}'),            // 
        "js" => Some('\u{e74e}'),             // 
        "json" => Some('\u{e60b}'),           // 
        "jsx" => Some('\u{e7ba}'),            // 
        "jxl" => Some('\u{f1c5}'),            // 
        "ksh" => Some('\u{f489}'),            // 
        "latex" => Some('\u{f034}'),          // 
        "less" => Some('\u{e758}'),           // 
        "lhs" => Some('\u{e777}'),            // 
        "license" => Some('\u{f718}'),        // 
        "localized" => Some('\u{f179}'),      // 
        "lock" => Some('\u{f023}'),           // 
        "log" => Some('\u{f18d}'),            // 
        "lua" => Some('\u{e620}'),            // 
        "lz" => Some('\u{f410}'),             // 
        "lz4" => Some('\u{f410}'),            // 
        "lzh" => Some('\u{f410}'),            // 
        "lzma" => Some('\u{f410}'),           // 
        "lzo" => Some('\u{f410}'),            // 
        "m" => Some('\u{e61e}'),              // 
        "mm" => Some('\u{e61d}'),             // 
        "m4a" => Some('\u{f001}'),            // 
        "markdown" => Some('\u{f48a}'),       // 
        "md" => Some('\u{f48a}'),             // 
        "mjs" => Some('\u{e74e}'),            // 
        "mk" => Some('\u{f489}'),             // 
        "mkd" => Some('\u{f48a}'),            // 
        "mkv" => Some('\u{f03d}'),            // 
        "mobi" => Some('\u{e28b}'),           // 
        "mov" => Some('\u{f03d}'),            // 
        "mp3" => Some('\u{f001}'),            // 
        "mp4" => Some('\u{f03d}'),            // 
        "msi" => Some('\u{e70f}'),            // 
        "mustache" => Some('\u{e60f}'),       // 
        "nix" => Some('\u{f313}'),            // 
        "node" => Some('\u{f898}'),           // 
        "npmignore" => Some('\u{e71e}'),      // 
        "odp" => Some('\u{f1c4}'),            // 
        "ods" => Some('\u{f1c3}'),            // 
        "odt" => Some('\u{f1c2}'),            // 
        "ogg" => Some('\u{f001}'),            // 
        "ogv" => Some('\u{f03d}'),            // 
        "otf" => Some('\u{f031}'),            // 
        "part" => Some('\u{f43a}'),           // 
        "patch" => Some('\u{f440}'),          // 
        "pdf" => Some('\u{f1c1}'),            // 
        "php" => Some('\u{e73d}'),            // 
        "pl" => Some('\u{e769}'),             // 
        "plx" => Some('\u{e769}'),            // 
        "pm" => Some('\u{e769}'),             // 
        "png" => Some('\u{f1c5}'),            // 
        "pod" => Some('\u{e769}'),            // 
        "ppt" => Some('\u{f1c4}'),            // 
        "pptx" => Some('\u{f1c4}'),           // 
        "procfile" => Some('\u{e21e}'),       // 
        "properties" => Some('\u{e60b}'),     // 
        "ps1" => Some('\u{f489}'),            // 
        "psd" => Some('\u{e7b8}'),            // 
        "pxm" => Some('\u{f1c5}'),            // 
        "py" => Some('\u{e606}'),             // 
        "pyc" => Some('\u{e606}'),            // 
        "r" => Some('\u{f25d}'),              // 
        "rakefile" => Some('\u{e21e}'),       // 
        "rar" => Some('\u{f410}'),            // 
        "razor" => Some('\u{f1fa}'),          // 
        "rb" => Some('\u{e21e}'),             // 
        "rdata" => Some('\u{f25d}'),          // 
        "rdb" => Some('\u{e76d}'),            // 
        "rdoc" => Some('\u{f48a}'),           // 
        "rds" => Some('\u{f25d}'),            // 
        "readme" => Some('\u{f48a}'),         // 
        "rlib" => Some('\u{e7a8}'),           // 
        "rmd" => Some('\u{f48a}'),            // 
        "rpm" => Some('\u{e7bb}'),            // 
        "rs" => Some('\u{e7a8}'),             // 
        "rspec" => Some('\u{e21e}'),          // 
        "rspec_parallel" => Some('\u{e21e}'), // 
        "rspec_status" => Some('\u{e21e}'),   // 
        "rss" => Some('\u{f09e}'),            // 
        "rtf" => Some('\u{f718}'),            // 
        "ru" => Some('\u{e21e}'),             // 
        "rubydoc" => Some('\u{e73b}'),        // 
        "sass" => Some('\u{e603}'),           // 
        "scala" => Some('\u{e737}'),          // 
        "scm" => Some('\u{f671}'),            // 
        "scss" => Some('\u{e749}'),           // 
        "sh" => Some('\u{f489}'),             // 
        "shell" => Some('\u{f489}'),          // 
        "slim" => Some('\u{e73b}'),           // 
        "sln" => Some('\u{e70c}'),            // 
        "so" => Some('\u{f17c}'),             // 
        "sql" => Some('\u{f1c0}'),            // 
        "sqlite3" => Some('\u{e7c4}'),        // 
        "sty" => Some('\u{f034}'),            // 
        "styl" => Some('\u{e600}'),           // 
        "stylus" => Some('\u{e600}'),         // 
        "svg" => Some('\u{f1c5}'),            // 
        "swift" => Some('\u{e755}'),          // 
        "t" => Some('\u{e769}'),              // 
        "tar" => Some('\u{f410}'),            // 
        "taz" => Some('\u{f410}'),            // 
        "tbz" => Some('\u{f410}'),            // 
        "tbz2" => Some('\u{f410}'),           // 
        "tex" => Some('\u{f034}'),            // 
        "tgz" => Some('\u{f410}'),            // 
        "tiff" => Some('\u{f1c5}'),           // 
        "tlz" => Some('\u{f410}'),            // 
        "toml" => Some('\u{e615}'),           // 
        "torrent" => Some('\u{e275}'),        // 
        "ts" => Some('\u{e628}'),             // 
        "tsv" => Some('\u{f1c3}'),            // 
        "tsx" => Some('\u{e7ba}'),            // 
        "ttf" => Some('\u{f031}'),            // 
        "twig" => Some('\u{e61c}'),           // 
        "txt" => Some('\u{f15c}'),            // 
        "txz" => Some('\u{f410}'),            // 
        "tz" => Some('\u{f410}'),             // 
        "tzo" => Some('\u{f410}'),            // 
        "video" => Some('\u{f03d}'),          // 
        "vim" => Some('\u{e62b}'),            // 
        "vue" => Some('\u{fd42}'),            // ﵂
        "war" => Some('\u{e256}'),            // 
        "wav" => Some('\u{f001}'),            // 
        "webm" => Some('\u{f03d}'),           // 
        "webp" => Some('\u{f1c5}'),           // 
        "windows" => Some('\u{f17a}'),        // 
        "woff" => Some('\u{f031}'),           // 
        "woff2" => Some('\u{f031}'),          // 
        "xhtml" => Some('\u{f13b}'),          // 
        "xls" => Some('\u{f1c3}'),            // 
        "xlsx" => Some('\u{f1c3}'),           // 
        "xml" => Some('\u{f121}'),            // 
        "xul" => Some('\u{f121}'),            // 
        "xz" => Some('\u{f410}'),             // 
        "yaml" => Some('\u{f481}'),           // 
        "yml" => Some('\u{f481}'),            // 
        "zip" => Some('\u{f410}'),            // 
        "zsh" => Some('\u{f489}'),            // 
        "zsh-theme" => Some('\u{f489}'),      // 
        "zshrc" => Some('\u{f489}'),          // 
        "zst" => Some('\u{f410}'),            // 
        _ => None,
    }
}
