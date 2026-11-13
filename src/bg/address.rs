use crate::CompanyAddress;

fn value(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_owned())
}

fn country(address: &mut CompanyAddress) {
    if let Some(country) = address
        .country
        .as_deref()
        .and_then(crate::locations::Country::from_name)
    {
        address.country = Some(country.native_name().into());
        address.country_iso2 = Some(country.iso2().into());
    }
}

fn split_delivery(address: &mut CompanyAddress) {
    // Only apply Bulgarian marker rules to an explicitly Bulgarian address.
    if address.country_iso2.as_deref() != Some("BG") {
        return;
    }
    let Some(line) = address.address.take() else {
        return;
    };
    let (primary, details) = crate::locations::split_delivery(&line, "bg");
    address.address = primary;
    address.address2 = match (details, address.address2.take()) {
        (Some(details), Some(rest)) => Some(format!("{details}\n{rest}")),
        (details, rest) => details.or(rest),
    };
}

fn settlement(s: &str) -> Option<String> {
    let s = s.trim();
    value(
        s.strip_prefix("гр.")
            .or_else(|| s.strip_prefix("с."))
            .unwrap_or(s),
    )
}

pub(super) fn commercial(raw: Option<String>) -> CompanyAddress {
    let mut result = CompanyAddress {
        full_address_by_registry: raw.clone(),
        ..Default::default()
    };
    let Some(raw) = raw else {
        return result;
    };
    let mut delivery = Vec::new();
    let mut countries = Vec::new();
    let mut regions = Vec::new();
    let mut localities = Vec::new();
    for line in raw.lines() {
        // The source puts contact fields in the same HTML paragraph as the office.
        // Preserve them in full_address_by_registry, not delivery components.
        let line = [
            "Телефон:",
            "Факс:",
            "Адрес на електронна поща:",
            "Интернет страница:",
        ]
        .iter()
        .filter_map(|label| line.find(label))
        .min()
        .map_or(line, |end| &line[..end])
        .trim();
        if line.is_empty() {
            continue;
        }
        if let Some(s) = line.strip_prefix("Държава:") {
            countries.push(s);
        } else if let Some(s) = line.strip_prefix("Област:") {
            regions.push(s);
        } else if let Some(s) = line.strip_prefix("Населено място:") {
            localities.push(s);
        } else if line.starts_with("р-н ") || line.starts_with("район ") {
            // Districts remain only in full_address_by_registry.
            continue;
        } else {
            // Portal street labels can follow a residential complex and can
            // directly touch the street name. Remove only redundant labels.
            let line = line
                .replace("бул./ул. ул.", "ул.")
                .replace("бул./ул. бул.", "бул.")
                .replace("бул./ул. жк ", "жк ")
                .replace("бул./ул. ж.к.", "ж.к.");
            delivery.push(line);
        }
    }
    if countries.len() == 1 {
        result.country = value(countries[0]);
    }
    if regions.len() == 1 {
        if let Some((region, municipality)) = regions[0].split_once(", Община:") {
            result.region = value(region);
            result.municipality = value(municipality);
        } else {
            result.region = value(regions[0]);
        }
    }
    if localities.len() == 1 {
        if let Some((city, postcode)) = localities[0].split_once(", п.к.") {
            result.city = settlement(city);
            let postcode = postcode.trim();
            if postcode.len() == 4 && postcode.bytes().all(|b| b.is_ascii_digit()) {
                result.postcode = value(postcode);
            }
        } else {
            result.city = settlement(localities[0]);
        }
    }
    result.address = delivery.first().and_then(|s| value(s));
    if delivery.len() > 1 {
        result.address2 = value(&delivery[1..].join("\n"));
    }
    country(&mut result);
    split_delivery(&mut result);
    if result.country_iso2.as_deref() == Some("BG") {
        for field in [
            &mut result.fields.city,
            &mut result.fields.region,
            &mut result.fields.municipality,
        ] {
            *field = field
                .take()
                .map(|s| crate::locations::normalize::place_name("BG", s));
        }
    }
    result
}

pub(super) fn bulstat(raw: Option<String>) -> CompanyAddress {
    let mut result = CompanyAddress {
        full_address_by_registry: raw.clone(),
        ..Default::default()
    };
    let Some(raw) = raw else {
        return result;
    };
    let Some(s) = raw.strip_prefix("седалище и адрес на управление:")
    else {
        return result;
    };
    let parts: Vec<_> = s.split(", ").map(str::trim).collect();
    // Only the observed labeled Bulgarian structure is split. Unknown/foreign
    // variants retain source text instead of guessing a street or postcode.
    if parts.len() >= 3
        && parts[1].starts_with("пощенски код ")
        && (parts[2].starts_with("гр.") || parts[2].starts_with("с."))
    {
        result.country = value(parts[0]);
        let postcode = parts[1].trim_start_matches("пощенски код ");
        if postcode.len() == 4 && postcode.bytes().all(|b| b.is_ascii_digit()) {
            result.postcode = value(postcode);
        }
        result.city = settlement(parts[2]);
        let start = if parts.get(3).is_some_and(|s| s.starts_with("р-н ")) {
            4
        } else {
            3
        };
        if parts.len() > start {
            result.address = value(&parts[start..].join(", "));
        }
    }
    country(&mut result);
    split_delivery(&mut result);
    if result.country_iso2.as_deref() == Some("BG") {
        for field in [
            &mut result.fields.city,
            &mut result.fields.region,
            &mut result.fields.municipality,
        ] {
            *field = field
                .take()
                .map(|s| crate::locations::normalize::place_name("BG", s));
        }
    }
    result
}
