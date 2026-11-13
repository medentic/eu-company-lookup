//! Normalize a working copy before extraction; never mutate source text.
use regex_lite::Regex;
use std::sync::LazyLock;

static BG_MARKERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"[оО][бБ][лЛ]\s*\.|[бБ][уУ][лЛ]\s*\.|[бБ][лЛ]\s*\.|[вВ][хХ]\s*\.|[еЕ][тТ]\s*\.|[аА][пП]\s*\.|[уУ][лЛ]\s*\.|[гГ][рР]\s*\."
).unwrap()
});
static DETAILS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[ ,]+)(?:[бБ][лЛ]\.|[вВ][хХ]\.|[еЕ][тТ]\.|[аА][пП]\.)").unwrap()
});
static POSTAL_SEPARATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[ \t]{3,}([0-9]{5}|[0-9]{3} [0-9]{2})[ ]+").unwrap());

pub(super) fn address(prefix: &str, raw: &str) -> String {
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
    // BG is a single logical line. Preserve all text while repairing recognized
    // marker boundaries. Do not split a marker embedded in an ordinary word.
    value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    let mut end = 0;
    let mut in_details = false;
    for found in BG_MARKERS.find_iter(&value) {
        let previous = value[..found.start()].chars().next_back();
        let boundary = previous.is_none_or(|c| !c.is_alphabetic())
            || (in_details && previous.is_some_and(char::is_uppercase));
        if !boundary {
            continue;
        }
        out.push_str(&value[end..found.start()]);
        let marker: String = found
            .as_str()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let lower = marker.to_lowercase();
        let is_street = matches!(lower.as_str(), "ул." | "бул.");
        if is_street && out.to_lowercase().starts_with("жк ") {
            out = out.trim_end_matches([' ', ',']).to_string();
            out.push_str(", ");
        } else if !out.is_empty() && !out.ends_with([' ', ',']) {
            out.push(' ');
        }
        out.push_str(&marker);
        if is_street {
            out.push(' ');
        }
        in_details = matches!(lower.as_str(), "бл." | "вх." | "ет." | "ап.");
        end = found.end();
    }
    out.push_str(&value[end..]);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn bulgarian_delivery(line: &str) -> (Option<String>, Option<String>) {
    let Some(found) = DETAILS.find(line) else {
        return (Some(line.into()), None);
    };
    let primary = line[..found.start()].trim_matches([' ', ',']);
    let details = line[found.start()..].trim_start_matches([' ', ',']).trim();
    // A block-only address is still useful delivery information in address2.
    (
        (!primary.is_empty()).then(|| primary.into()),
        (!details.is_empty()).then(|| details.into()),
    )
}

/// Presentation only: BG uppercase place names, never raw source or delivery text.
/// Mixed-case values are already curated. Preserve dotted abbreviations, known
/// address acronyms and Roman numerals; do not apply these rules internationally.
pub(super) fn place_name(prefix: &str, value: String) -> String {
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
