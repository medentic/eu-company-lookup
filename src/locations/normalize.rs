//! Normalize a working copy before extraction; never mutate source text.
use regex_lite::Regex;
use std::sync::LazyLock;

static BG_MARKERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"[кК][вВ]\s*\.+|[оО][бБ][лЛ]\s*\.+|[бБ][уУ][лЛ]\s*\.+|[бБ][лЛ]\s*\.+|[вВ][хХ]\s*\.+|[еЕ][тТ]\s*\.+|[аА][пП]\s*\.+|[уУ][лЛ]\s*\.+|[гГ][рР]\s*\.+"
).unwrap()
});
static BG_COMMA_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" +,").unwrap());
static BG_COMPLEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[ ,])[жЖ]\.?[кК](?: *\.+)?( +|$)").unwrap());
static BG_COMPOUND_STREET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([бБ][уУ][лЛ]\.) */ *([уУ][лЛ]\.)").unwrap());
// Uppercase only a single entrance letter, not arbitrary entrance descriptions.
static BG_ENTRANCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([вВ][хХ]\. +)([а-яА-Яa-zA-Z])([ ,]|$)").unwrap());
static POSTAL_SEPARATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[ \t]{3,}([0-9]{5}|[0-9]{3} [0-9]{2})[ ]+").unwrap());

pub(crate) fn address(prefix: &str, raw: &str) -> String {
    let raw = raw.replace("\r\n", "\n").replace('\r', "\n");
    // EE/EL use wide spacing as a structural separator. Preserve that boundary
    // as a newline before collapsing whitespace; ordinary line breaks also stay.
    let raw = if matches!(prefix, "EE" | "EL") {
        POSTAL_SEPARATOR.replace_all(&raw, "\n$1 ").into_owned()
    } else {
        raw
    };
    let mut value = raw
        .trim_end()
        .split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    if prefix != "BG" {
        return value;
    }
    // BG uses a single logical working line for presentation and extraction.
    // The caller retains raw text. Do not split markers embedded in ordinary words.
    value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    value.retain(|c| !matches!(c, '"' | '„' | '“' | '”' | '«' | '»' | '‟'));
    value = BG_COMMA_SPACE.replace_all(&value, ",").into_owned();
    value = BG_COMPLEX.replace_all(&value, "${1}жк. ").into_owned();
    let mut out = String::new();
    let mut end = 0;
    let mut in_details = false;
    for found in BG_MARKERS.find_iter(&value) {
        let previous = value[..found.start()].chars().next_back();
        // A lower-case street marker glued to an upper-case complex name is
        // a known VIES boundary loss (ЛЕВСКИул.). Do not generalize to words.
        let complex_street_boundary = value.to_lowercase().starts_with("жк. ")
            && previous.is_some_and(char::is_uppercase)
            && matches!(found.as_str(), "ул." | "бул.");
        let boundary = complex_street_boundary
            || previous.is_none_or(|c| !c.is_alphabetic())
            || (in_details && previous.is_some_and(char::is_uppercase));
        if !boundary {
            continue;
        }
        out.push_str(&value[end..found.start()]);
        let mut marker: String = found
            .as_str()
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '.')
            .collect();
        marker.push('.');
        let lower = marker.to_lowercase();
        let is_street = matches!(lower.as_str(), "ул." | "бул.");
        if is_street && out.to_lowercase().starts_with("жк. ") && !out.trim_end().ends_with('/') {
            out = out.trim_end_matches([' ', ',']).to_string();
            out.push_str(", ");
        } else if !out.is_empty() && !out.ends_with([' ', ',']) {
            out.push(' ');
        }
        out.push_str(&marker);
        in_details = matches!(lower.as_str(), "бл." | "вх." | "ет." | "ап.");
        if is_street || in_details || lower == "кв." {
            out.push(' ');
        }
        end = found.end();
    }
    out.push_str(&value[end..]);
    let out = BG_COMPOUND_STREET.replace_all(&out, "$1/$2");
    let out = BG_ENTRANCE.replace_all(&out, |captures: &regex_lite::Captures<'_>| {
        format!(
            "{}{}{}",
            &captures[1],
            captures[2].to_uppercase(),
            &captures[3]
        )
    });
    remove_empty_details(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// Drop only recognized delivery markers whose entire value is empty punctuation.
// Keep real hyphenated values such as А-Б and numeric ranges such as 1-3.
fn remove_empty_details(value: &str) -> String {
    let markers: Vec<_> = BG_MARKERS
        .find_iter(value)
        .filter(|m| {
            value[..m.start()]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphabetic())
        })
        .collect();
    let mut out = String::new();
    let mut end = 0;
    for (index, marker) in markers.iter().enumerate() {
        let next = markers.get(index + 1).map_or(value.len(), |m| m.start());
        if matches!(
            marker.as_str().to_lowercase().as_str(),
            "бл." | "вх." | "ет." | "ап."
        ) && value[marker.end()..next]
            .trim_matches([' ', ',', '.', '-', '*'])
            .is_empty()
        {
            let prefix = &value[end..marker.start()];
            let preserve_comma = next < value.len()
                && value[marker.end()..next].contains('*')
                && prefix.trim_end().ends_with(',');
            out.push_str(prefix.trim_end_matches([' ', ',']));
            if preserve_comma {
                out.push(',');
            }
            out.push(' ');
            end = next;
        }
    }
    out.push_str(&value[end..]);
    out.trim_end_matches([' ', ',']).to_owned()
}

pub(crate) use crate::locations::delivery::bulgarian_delivery;

/// Presentation only: BG uppercase place names, never raw source or delivery text.
/// Mixed-case values are already curated. Preserve dotted abbreviations, known
/// address acronyms and Roman numerals; do not apply these rules internationally.
pub(crate) fn place_name(prefix: &str, mut value: String) -> String {
    if prefix == "BG" {
        while let Some((name, tail)) = value.rsplit_once(' ') {
            if tail.is_empty() || !tail.chars().all(|c| matches!(c, '.' | '-')) {
                break;
            }
            value = name.trim_end().to_owned();
        }
    }
    if prefix != "BG"
        || !value.chars().any(char::is_uppercase)
        || value.chars().any(char::is_lowercase)
    {
        return value;
    }
    value
        .split(' ')
        .map(|word| {
            word.split('-')
                .map(|part| {
                    if part.contains('.')
                        || matches!(part, "ЖК" | "КК" | "НП" | "ПП")
                        || (!part.is_empty() && part.chars().all(|c| "IVXLCDM".contains(c)))
                    {
                        return part.to_owned();
                    }
                    let mut first = true;
                    part.chars()
                        .flat_map(|c| {
                            if first && c.is_alphabetic() {
                                first = false;
                                c.to_uppercase().collect::<String>()
                            } else {
                                c.to_lowercase().collect::<String>()
                            }
                            .chars()
                            .collect::<Vec<_>>()
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("-")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
