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
    result.city = field("city").map(|value| normalize::place_name(prefix, value));
    result.postcode = field("postcode");
    result.region = field("region").map(|value| normalize::place_name(prefix, value));
    result.country = field("country");
    result
}
