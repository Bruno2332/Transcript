use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub fn name_search_pattern(value: &str) -> String {
    let normalized = value
        .trim()
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase();

    let mut pattern = String::from("(?i)");

    for character in normalized.chars() {
        match character {
            'a' => pattern.push_str("[aáàâãäå]"),
            'e' => pattern.push_str("[eéèêë]"),
            'i' => pattern.push_str("[iíìîï]"),
            'o' => pattern.push_str("[oóòôõö]"),
            'u' => pattern.push_str("[uúùûü]"),
            'c' => pattern.push_str("[cç]"),
            'n' => pattern.push_str("[nñ]"),
            'y' => pattern.push_str("[yýÿ]"),
            _ => pattern.push_str(&regex::escape(&character.to_string())),
        }

        pattern.push_str(r"\p{M}*");
    }

    pattern
}