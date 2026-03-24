use std::sync::LazyLock;

use bat::assets::HighlightingAssets;
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::{SyntaxReference, SyntaxSet},
};

thread_local!(pub static BAT_ASSETS: HighlightingAssets = HighlightingAssets::from_binary());

/// Takes the content of a paste and an optional extension.
/// If no extension is provided, it uses multiple heuristics to detect the language.
///
/// Returns `None` if the syntax cannot be determined.
pub fn highlight(content: &str, ext: Option<&str>) -> Option<String> {
    static SS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

    BAT_ASSETS
        .with(|f| {
            let ss = f.get_syntax_set().ok().unwrap_or(&SS);

            // 1. Try by explicit extension (e.g., .rs, .toml)
            let mut syntax = ext.and_then(|e| ss.find_syntax_by_extension(e));

            // 2. If no extension or unknown extension, try by first line (Shebangs/Headers)
            if syntax.is_none() {
                syntax = ss.find_syntax_by_first_line(content);
            }

            // 3. If still nothing, try to guess based on content structure (TOML, YAML, MD)
            if syntax.is_none() {
                syntax = guess_syntax_from_content(content, ss);
            }

            // Fix: Use an explicit match or if-let instead of the ? operator on Option
            // inside a closure that returns a Result.
            let syntax: &SyntaxReference = match syntax {
                Some(s) => s,
                None => return Ok(None),
            };

            let mut html_generator =
                ClassedHTMLGenerator::new_with_class_style(syntax, ss, ClassStyle::Spaced);

            // Process lines with endings to ensure generator state is maintained correctly
            for line in LinesWithEndings(content.trim()) {
                html_generator.parse_html_for_line_which_includes_newline(line)?;
            }

            Ok::<_, syntect::Error>(Some(html_generator.finalize()))
        })
        .ok()
        .flatten()
}

/// Heuristic-based detection for formats that don't use shebangs.
fn guess_syntax_from_content<'a>(content: &str, ss: &'a SyntaxSet) -> Option<&'a SyntaxReference> {
    let trimmed = content.trim_start();

    // Check for Markdown headers or lists
    if trimmed.starts_with('#') || trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        if let Some(s) = ss.find_syntax_by_extension("md") {
            return Some(s);
        }
    }

    // Check for TOML-like headers [section] or key = "value"
    if trimmed.starts_with('[') || (trimmed.contains('=') && trimmed.contains('"')) {
        if let Some(s) = ss.find_syntax_by_extension("toml") {
            return Some(s);
        }
    }

    // Check for YAML-like separators or key-value pairs
    if trimmed.starts_with("---") || (trimmed.contains(':') && !trimmed.contains('{')) {
        if let Some(s) = ss.find_syntax_by_extension("yaml") {
            return Some(s);
        }
    }

    // Check for JSON
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Some(s) = ss.find_syntax_by_extension("json") {
            return Some(s);
        }
    }

    None
}

/// Helper struct to iterate over lines while preserving newline characters.
pub struct LinesWithEndings<'a>(&'a str);

impl<'a> Iterator for LinesWithEndings<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            let split = self.0.find('\n').map_or(self.0.len(), |i| i + 1);
            let (line, rest) = self.0.split_at(split);
            self.0 = rest;
            Some(line)
        }
    }
}
