use super::Address;

const SETTLEMENT_LABELS: [&str; 4] = ["küla", "alevik", "alev", "linn"];

// Single-space source layouts can still establish a postal boundary when the
// trailing locality explicitly names a district or settlement. Do not treat an
// unlabelled number/name pair, a missing postcode or a municipality alone as city.
pub(super) fn labelled_postal(value: &str) -> Option<Address> {
    static PATTERN: std::sync::LazyLock<regex_lite::Regex> = std::sync::LazyLock::new(|| {
        regex_lite::Regex::new(
            r"^(?P<street>[^\n]+?) +(?P<postcode>[0-9]{5}) +(?P<city>[^0-9\n]+)$",
        )
        .unwrap()
    });
    let captures = PATTERN.captures(value)?;
    let locality = captures.name("city")?.as_str().trim();
    let district = district_parts(locality).is_some();
    let settlement = settlement_parts(locality).is_some()
        || SETTLEMENT_LABELS.iter().any(|label| {
            locality
                .strip_suffix(&format!(" {label}"))
                .is_some_and(|name| !name.is_empty())
        });
    if !district && !settlement {
        return None;
    }
    let mut result = Address {
        address: Some(captures.name("street")?.as_str().trim().to_owned()),
        postcode: Some(captures.name("postcode")?.as_str().to_owned()),
        city: Some(locality.to_owned()),
        ..Address::default()
    };
    split_locality(&mut result);
    Some(result)
}

// Estonian source labels distinguish city districts from settlements and municipalities.
// Keep the labels in their own fields rather than merging the hierarchy into city.
pub(super) fn split_locality(result: &mut Address) {
    let Some(locality) = result.city.clone() else {
        return;
    };
    if let Some((district, city)) = district_parts(&locality) {
        result.address2 = Some(format!("{district} linnaosa"));
        result.city = Some(city.to_owned());
    } else if !locality.contains(" linnaosa ")
        && let Some((settlement, municipality)) = settlement_parts(&locality)
    {
        result.city = Some(settlement);
        result.municipality = Some(municipality.to_owned());
    }
}

fn district_parts(locality: &str) -> Option<(&str, &str)> {
    locality
        .split_once(" linnaosa ")
        .filter(|(district, city)| !district.is_empty() && !city.is_empty())
}

fn settlement_parts(locality: &str) -> Option<(String, &str)> {
    SETTLEMENT_LABELS.iter().find_map(|label| {
        let (settlement, municipality) = locality.split_once(&format!(" {label} "))?;
        if settlement.is_empty()
            || !(municipality.ends_with(" vald") || municipality.ends_with(" linn"))
        {
            return None;
        }
        Some((format!("{settlement} {label}"), municipality))
    })
}
