// Initial rules adapted from justasojourner/VatCheck (BSD-3-Clause).
// See THIRD-PARTY-LICENSE.txt. Tightened postal shapes and lossless captures.
// Unsupported countries deliberately have no catch-all "street" rule.
pub(crate) fn rule(country: &str) -> Option<String> {
    let postal = match country {
        "AT" => r"(?:AT-)?[0-9]{4}",
        "BE" | "CY" | "DK" => r"[0-9]{4}",
        "FI" | "FR" => r"[0-9]{5}",
        "LU" => r"(?:L-)?[0-9]{4}",
        "PL" => r"[0-9]{2}-[0-9]{3}",
        "NL" => r"[0-9]{4}[ ]?[A-Z]{2}",
        "SE" | "CZ" => r"[0-9]{3}[ ]?[0-9]{2}",
        _ => "",
    };
    if !postal.is_empty() {
        // Keep all preceding lines as address lines; don't pretend they are all street names.
        let delivery = if country == "SE" {
            // Swedish VIES can return only a postal code and routing locality.
            r"(?:(?P<street>[\s\S]*?)\n)?"
        } else {
            r"(?P<street>[\s\S]*?)\n"
        };
        return Some(format!(
            r"^{delivery}(?P<postcode>{postal})[ \t]+(?P<city>[^\n]+)$"
        ));
    }
    Some(match country {
        "BG" => r"^(?P<street>[^\n]*?)(?:(?:^|[ ]+)[оО][бБ][лЛ]\.[ ]*(?P<region>[^,\n]+?))?(?:,[ ]*|[ ]+)(?:[гГ][рР]\.|[сС]\.)[ ]*(?P<city>[^,\n]+?)(?:[ ]+(?P<postcode>[0-9]{4}))?$",
        // HR names the settlement before a separate postal routing locality.
        "HR" => r"^(?P<street>[^,\n]+),[ ]*(?P<city>[^,\n]+),[ ]*(?P<postcode>[0-9]{5})[ ]+[^,\n]+$",
        // Require a digit-free trailing locality so four-digit street numbers
        // cannot steal the postcode when another postcode follows them.
        "HU" => r"^(?P<street>[^\n]+)[ ]+(?P<postcode>[0-9]{4})[ ]+(?P<city>[^0-9\n]+)$",
        "EE" => r"^(?P<street>[^\n]+?)\n(?P<postcode>[0-9]{5})[ ]+(?P<city>[^\n]+)$",
        "EL" => r"^(?P<street>[^\n]+?)\n(?P<postcode>[0-9]{3}[ ]?[0-9]{2})[ ]+-[ ]+(?P<city>[^\n]+)$",
        "IT" => r"^(?P<street>[\s\S]+?)\n(?P<postcode>[0-9]{5})[ ]+(?P<city>[^\n]+?)[ ]+(?P<region>[A-Z]{2})$",
        "LT" => r"^(?P<street>[^,\n]+),[ ]*(?P<city>[^,\n]+),[ ]*(?P<postcode>LT-?[0-9]{5})$",
        // LV often has parish/municipality in this position, not city: leave unsupported.
        "SI" => r"^(?P<street>[^\n]+),[ ]*(?P<postcode>[0-9]{4})[ ]+(?P<city>[^\n]+)$",
        "MT" => r"^(?P<street>[\s\S]*?)\n(?P<postcode>[A-Z]{3}[ ]?[0-9]{4}|[A-Z]{3}[ ][0-9]{2,3})\n(?P<city>[^\n]+)$",
        "PT" => r"^(?P<street>[^\n]+)\n(?P<locality>[^\n]+)\n(?P<postcode>[0-9]{4}-[0-9]{3})[ ]+(?P<city>[^\n]+)$",
        "SK" => r"^(?P<street>[^\n]+)\n(?P<postcode>[0-9]{3}[ ]?[0-9]{2})[ ]+(?P<city>[^\n]+)\n(?P<country>Slovensko|SLOVENSKO|Slovakia|SLOVAKIA)$",
        _ => return None,
    }.into())
}
