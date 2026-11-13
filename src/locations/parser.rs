use super::{Address, normalize, rules};

pub(super) fn split_fields(prefix: &str, raw: &str) -> Address {
    let mut result = Address::default();
    if raw.len() > 65_536 {
        return result;
    }
    let normalized = normalize::address(prefix, raw);
    if normalized.trim().is_empty() || matches!(normalized.trim(), "---" | "N/A") {
        return result;
    }
    let Some(pattern) = rules::rule(prefix) else {
        return result;
    };
    let regex = regex_lite::Regex::new(&pattern).expect("static country address rule");
    let Some(captures) = regex.captures(&normalized) else {
        return match prefix {
            "BG" => bulgarian_locality_only(&normalized).unwrap_or(result),
            "PT" => portuguese_delivery_only(&normalized).unwrap_or(result),
            _ => result,
        };
    };
    let field = |name| {
        captures
            .name(name)
            .map(|m| m.as_str().trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let delivery = field("street");
    let mut lines: Vec<String> = delivery
        .as_deref()
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if !lines.is_empty() {
        result.address = Some(lines.remove(0));
    }
    if let Some(locality) = field("locality") {
        lines.push(locality);
    }
    if !lines.is_empty() {
        result.address2 = Some(lines.join("\n"));
    }
    if prefix == "BG"
        && let Some(line) = result.address.take()
    {
        let (primary, details) = normalize::bulgarian_delivery(&line);
        result.address = primary;
        if let Some(details) = details {
            result.address2 = Some(match result.address2.take() {
                Some(rest) => format!("{details}\n{rest}"),
                None => details,
            });
        }
    }
    result.city = field("city").map(|value| normalize::place_name(prefix, value));
    result.postcode = field("postcode");
    result.region = field("region").map(|value| normalize::place_name(prefix, value));
    result.country = field("country");
    // Finnish 000xx postal names can identify corporate recipients, not settlements.
    // Preserve that label as delivery text; do not assert a geographic city.
    if prefix == "FI"
        && result
            .postcode
            .as_deref()
            .is_some_and(|p| p.starts_with("000"))
        && let Some(label) = result.city.take()
    {
        result.address2 = Some(match result.address2.take() {
            Some(rest) => format!("{rest}\n{label}"),
            None => label,
        });
    }
    if prefix == "EE" {
        estonian_locality(&mut result);
    }
    result
}

// Some locality-only records put the settlement first and repeat the region.
// Require both region names to agree; conflicting or extra text stays unknown.
fn bulgarian_locality_only(value: &str) -> Option<Address> {
    static PATTERN: std::sync::LazyLock<regex_lite::Regex> = std::sync::LazyLock::new(|| {
        regex_lite::Regex::new(r"^(?:[сС]\.|[гГ][рР]\.) *(?P<city>[^,0-9]+), *(?P<repeated_region>[^,0-9]+?) +[оО][бБ][лЛ]\. *(?P<region>[^,0-9]+?)(?: +(?P<postcode>[0-9]{4}))?$").unwrap()
    });
    let captures = PATTERN.captures(value)?;
    let city = captures.name("city")?.as_str().trim();
    let region = captures.name("region")?.as_str().trim();
    let repeated = captures.name("repeated_region")?.as_str().trim();
    if city.is_empty() || region.is_empty() || region.to_lowercase() != repeated.to_lowercase() {
        return None;
    }
    Some(Address {
        city: Some(normalize::place_name("BG", city.to_owned())),
        region: Some(normalize::place_name("BG", region.to_owned())),
        postcode: captures.name("postcode").map(|m| m.as_str().to_owned()),
        ..Address::default()
    })
}

// Estonian source labels distinguish city districts from settlements and municipalities.
// Keep the labels in their own fields rather than merging the hierarchy into city.
fn estonian_locality(result: &mut Address) {
    let Some(locality) = result.city.clone() else {
        return;
    };
    if let Some((district, city)) = locality.split_once(" linnaosa ") {
        if !district.is_empty() && !city.is_empty() {
            result.address2 = Some(format!("{district} linnaosa"));
            result.city = Some(city.to_owned());
        }
    } else if let Some((village, municipality)) = locality.split_once(" küla ")
        && !village.is_empty()
        && municipality.ends_with(" vald")
    {
        result.city = Some(format!("{village} küla"));
        result.municipality = Some(municipality.to_owned());
    }
}

// A labelled Portuguese street plus an unclassified delivery line can be
// retained without guessing a city, district boundary or missing postcode.
fn portuguese_delivery_only(value: &str) -> Option<Address> {
    static PATTERN: std::sync::LazyLock<regex_lite::Regex> = std::sync::LazyLock::new(|| {
        regex_lite::Regex::new(r"^(?P<street>(?i:AVENIDA|RUA) +[^\n]+)\n(?P<detail>[^0-9\n]+)$")
            .unwrap()
    });
    let captures = PATTERN.captures(value)?;
    Some(Address {
        address: Some(captures.name("street")?.as_str().trim().to_owned()),
        address2: Some(captures.name("detail")?.as_str().trim().to_owned()),
        ..Address::default()
    })
}
