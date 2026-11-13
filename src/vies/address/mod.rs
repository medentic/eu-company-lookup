// Adapted from Medentic's VIES address parser; rule attribution is retained.
mod normalize;
mod rules;
use crate::VatAddress;

pub(super) fn supports(prefix: &str) -> bool {
    rules::rule(prefix).is_some()
}

pub(super) fn split(prefix: &str, original: Option<String>) -> VatAddress {
    let mut result = VatAddress {
        full_address_by_vies: original,
        ..Default::default()
    };
    let Some(raw) = result.full_address_by_vies.as_deref() else {
        return result;
    };
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
        return result;
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
    result.city = field("city");
    result.postcode = field("postcode");
    result.region = field("region");
    result.country = field("country");
    result
}

#[cfg(test)]
mod tests;
