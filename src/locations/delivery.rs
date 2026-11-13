//! Shared delivery-line extraction for Bulgarian source adapters.
use regex_lite::Regex;
use std::sync::LazyLock;

static DETAILS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[ ,]+)(?:[бБ][лЛ]\.|[вВ][хХ]\.|[еЕ][тТ]\.|[аА][пП]\.)|(?:^|, *)(?:[сС][гГ][рР][аА][дД][аА]|[сС][тТ][аА][яЯ]) +[^ ,]|(?:, *)(?:[кК][аА][бБ][иИ][нН][еЕ][тТ]) +[^ ,]|(?:^| +)/[пП][аА][рР][тТ][еЕ][рР][еЕ][нН] +[еЕ][тТ][аА][жЖ]/$").unwrap()
});

// Only whole, comma-separated components with an asterisk-only value are
// placeholders. Preserve stars in real names, suffixes and unrecognized text.
static STAR_COMPONENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:ж\.?к\.|бл\.|вх\.|ет\.|ап\.)\s*\*+$").unwrap());

// Abbreviated п.к. is also used for postal codes. Only omit short-number
// delivery components (the reviewed П.К 176 pattern), never four-digit codes.
static POST_BOX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:п\. *к\.? +[0-9]{1,3}|пощенска +кутия +(?:№ *)?[0-9]+)$").unwrap()
});

static EMPTY_COMPLEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^ж\.?к\. *[.*-]+$").unwrap());

fn primary_value(value: &str) -> Option<String> {
    (!value
        .chars()
        .all(|c| matches!(c, ' ' | '.' | ',' | '-' | '*' | '№')))
    .then(|| value.to_owned())
}

pub(crate) fn bulgarian_delivery(line: &str) -> (Option<String>, Option<String>) {
    let cleaned = line
        .split(',')
        .filter(|part| {
            !STAR_COMPONENT.is_match(part.trim())
                && !EMPTY_COMPLEX.is_match(&part.trim().to_lowercase())
                && !POST_BOX.is_match(&part.trim().to_lowercase())
        })
        .collect::<Vec<_>>()
        .join(",");
    let line = cleaned.trim_matches([' ', ',']);
    let Some(found) = DETAILS.find(line) else {
        return (primary_value(line), None);
    };
    let primary = line[..found.start()].trim_matches([' ', ',']);
    let details = line[found.start()..].trim_start_matches([' ', ',']).trim();
    // A block-only address is still useful delivery information in address2.
    (
        primary_value(primary),
        (!details.is_empty()).then(|| details.into()),
    )
}

#[cfg(test)]
mod tests {
    use crate::locations::split_delivery;

    #[test]
    fn asterisk_only_components_are_absent_but_real_values_are_preserved() {
        assert_eq!(
            split_delivery("ж.к. *, ул. Дюните № 6, бл. *, вх. *, ет. *, ап. *", "bg"),
            (Some("ул. Дюните № 6".into()), None)
        );
        assert_eq!(split_delivery("жк *, бл. **, ап. *", "bg"), (None, None));
        assert_eq!(
            split_delivery("ул. Звезда*, бл. 2*, вх. *, ет. 3, ап. *А", "bg"),
            (
                Some("ул. Звезда*".into()),
                Some("бл. 2*, ет. 3, ап. *А".into())
            )
        );
        assert_eq!(
            split_delivery("ул. *, неизвестно *", "bg"),
            (Some("ул. *, неизвестно *".into()), None)
        );
        assert_eq!(split_delivery("бл. *", "en"), (Some("бл. *".into()), None));
    }
}

#[cfg(test)]
mod post_box_tests {
    use crate::locations::split_delivery;
    #[test]
    fn omit_explicit_boxes_without_removing_postcodes_or_phone_like_numbers() {
        assert_eq!(
            split_delivery("ул. ЗОНА, П.К 176", "BG"),
            (Some("ул. ЗОНА".into()), None)
        );
        assert_eq!(
            split_delivery("ул. ЗОНА, пощенска кутия №1234, ет. 2", "BG"),
            (Some("ул. ЗОНА".into()), Some("ет. 2".into()))
        );
        for text in ["п.к. 1766", "049 02 003", "тел. 049 02 003"] {
            assert_eq!(split_delivery(text, "BG"), (Some(text.into()), None));
        }
    }
}

#[cfg(test)]
mod office_tests {
    use crate::locations::split_delivery;

    #[test]
    fn explicit_office_details_require_comma_and_value() {
        for (source, details) in [
            ("ул. Пример № 3, кабинет 2", "кабинет 2"),
            ("ул. Пример № 3, КАБИНЕТ А", "КАБИНЕТ А"),
        ] {
            assert_eq!(
                split_delivery(source, "BG"),
                (Some("ул. Пример № 3".into()), Some(details.into()))
            );
        }
        for source in [
            "ул. Кабинет 2",
            "ул. Пример, кабинет",
            "ул. Пример, кабинет, друго",
            "ул. Кабинетна № 2",
        ] {
            assert_eq!(split_delivery(source, "BG"), (Some(source.into()), None));
        }
    }
}
