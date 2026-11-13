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
        return Some(format!(
            r"^(?P<street>[\s\S]*?)\n(?P<postcode>{postal})[ \t]+(?P<city>[^\n]+)$"
        ));
    }
    Some(match country {
        "BG" => r"^(?P<street>[^\n]+?)(?:[ ]+[оО][бБ][лЛ]\.[ ]*(?P<region>[^,\n]+?))?(?:,[ ]*|[ ]+)(?:[гГ][рР]\.|[сС]\.)[ ]*(?P<city>[^,\n]+?)(?:[ ]+(?P<postcode>[0-9]{4}))?$",
        "EE" => r"^(?P<street>[^\n]+?)[ ]{3,}(?P<postcode>[0-9]{5})[ ]+(?P<city>[^\n]+)$",
        "EL" => r"^(?P<street>[^\n]+?)[ ]{3,}(?P<postcode>[0-9]{3}[ ]?[0-9]{2})[ ]+-[ ]+(?P<city>[^\n]+)$",
        "IT" => r"^(?P<street>[\s\S]+?)\n(?P<postcode>[0-9]{5})[ ]+(?P<city>[^\n]+?)[ ]+(?P<region>[A-Z]{2})$",
        "LT" => r"^(?P<street>[^,\n]+),[ ]*(?P<city>[^,\n]+),[ ]*(?P<postcode>LT-?[0-9]{5})$",
        // LV often has parish/municipality in this position, not city: leave unsupported.
        "SI" => r"^(?P<street>[^\n]+),[ ]*(?P<postcode>[0-9]{4})[ ]+(?P<city>[^\n]+)$",
        "PT" => r"^(?P<street>[^\n]+)\n(?P<locality>[^\n]+)\n(?P<postcode>[0-9]{4}-[0-9]{3})[ ]+(?P<city>[^\n]+)$",
        "SK" => r"^(?P<street>[^\n]+)\n(?P<postcode>[0-9]{3}[ ]?[0-9]{2})[ ]+(?P<city>[^\n]+)\n(?P<country>Slovensko|SLOVENSKO|Slovakia|SLOVAKIA)$",
        _ => return None,
    }.into())
}
