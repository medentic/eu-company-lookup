use crate::{LimitScope, LookupError, VatId};
use chrono::NaiveDate;
use roxmltree::Node;

const NS: &str = "urn:ec.europa.eu:taxud:vies:services:checkVat:types";
const SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

pub(super) struct Response {
    pub valid: bool,
    pub request_date: String,
    pub company: Option<String>,
    pub address: Option<String>,
}

pub(super) fn request(vat: &VatId) -> String {
    // Validated formats allow only ASCII alphanumerics, * and +: no XML syntax.
    format!(
        r#"<soap:Envelope xmlns:soap="{SOAP}"><soap:Body><checkVat xmlns="{NS}"><countryCode>{}</countryCode><vatNumber>{}</vatNumber></checkVat></soap:Body></soap:Envelope>"#,
        vat.prefix(),
        vat.number()
    )
}

fn one_child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Result<Node<'a, 'input>, LookupError> {
    let mut children = node
        .children()
        .filter(|n| n.has_tag_name((namespace, name)));
    let child = children.next().ok_or(LookupError::MalformedResponse)?;
    if children.next().is_some() {
        return Err(LookupError::MalformedResponse);
    }
    Ok(child)
}

fn field(
    node: Node<'_, '_>,
    namespace: Option<&str>,
    name: &str,
    optional: bool,
) -> Result<Option<String>, LookupError> {
    let mut children = node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == name);
    let Some(child) = children.next() else {
        return if optional {
            Ok(None)
        } else {
            Err(LookupError::MalformedResponse)
        };
    };
    if children.next().is_some()
        || child.tag_name().namespace() != namespace
        || child.children().any(|n| n.is_element())
    {
        return Err(LookupError::MalformedResponse);
    }
    let text: String = child
        .children()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect();
    match child.attribute((XSI, "nil")) {
        Some("true" | "1") if optional && text.is_empty() => return Ok(None),
        Some("false" | "0") | None => {}
        _ => return Err(LookupError::MalformedResponse),
    }
    Ok(Some(text))
}

pub(super) fn decode(xml: &str, vat: &VatId) -> Result<Response, LookupError> {
    // roxmltree defaults reject DTDs, preventing external/general entity expansion.
    let doc = roxmltree::Document::parse(xml).map_err(|_| LookupError::MalformedResponse)?;
    let envelope = doc.root_element();
    if !envelope.has_tag_name((SOAP, "Envelope")) {
        return Err(LookupError::MalformedResponse);
    }
    let body = one_child(envelope, SOAP, "Body")?;
    let mut children = body.children().filter(Node::is_element);
    let payload = children.next().ok_or(LookupError::MalformedResponse)?;
    if children.next().is_some() {
        return Err(LookupError::MalformedResponse);
    }
    if payload.has_tag_name((SOAP, "Fault")) {
        let code =
            field(payload, None, "faultstring", false)?.ok_or(LookupError::MalformedResponse)?;
        return Err(match code.trim() {
            "INVALID_INPUT" => LookupError::ProviderRejectedInput,
            "GLOBAL_MAX_CONCURRENT_REQ" => LookupError::RateLimited {
                scope: LimitScope::Global,
                retry_after: None,
            },
            "MS_MAX_CONCURRENT_REQ" => LookupError::RateLimited {
                scope: LimitScope::MemberState,
                retry_after: None,
            },
            "SERVICE_UNAVAILABLE" => LookupError::ServiceUnavailable { retry_after: None },
            "MS_UNAVAILABLE" => LookupError::MemberStateUnavailable { retry_after: None },
            "TIMEOUT" => LookupError::ProviderTimeout { retry_after: None },
            _ => LookupError::UnknownSoapFault,
        });
    }
    if !payload.has_tag_name((NS, "checkVatResponse")) {
        return Err(LookupError::MalformedResponse);
    }
    if payload.children().filter(Node::is_element).any(|n| {
        n.tag_name().namespace() != Some(NS)
            || ![
                "countryCode",
                "vatNumber",
                "requestDate",
                "valid",
                "name",
                "address",
            ]
            .contains(&n.tag_name().name())
    }) {
        return Err(LookupError::MalformedResponse);
    }
    let country = field(payload, Some(NS), "countryCode", false)?;
    let number = field(payload, Some(NS), "vatNumber", false)?;
    if country.as_deref() != Some(vat.prefix()) || number.as_deref() != Some(vat.number()) {
        return Err(LookupError::MismatchedResponse);
    }
    let valid = match field(payload, Some(NS), "valid", false)?
        .as_deref()
        .map(str::trim)
    {
        Some("true" | "1") => true,
        Some("false" | "0") => false,
        _ => return Err(LookupError::MalformedResponse),
    };
    let request_date =
        field(payload, Some(NS), "requestDate", false)?.ok_or(LookupError::MalformedResponse)?;
    if !valid_date(request_date.trim()) {
        return Err(LookupError::MalformedResponse);
    }
    let name = owner_text(field(payload, Some(NS), "name", true)?);
    let address = field(payload, Some(NS), "address", true)?;
    Ok(Response {
        valid,
        request_date,
        company: name,
        address,
    })
}

fn owner_text(value: Option<String>) -> Option<String> {
    // Preserve original content and whitespace when meaningful, for later evidence.
    value.filter(|s| !s.chars().all(|c| c.is_whitespace() || c == '-'))
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !value.is_ascii()
        || value.len() < 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4]
            .iter()
            .chain(&bytes[5..7])
            .chain(&bytes[8..10])
            .all(u8::is_ascii_digit)
        || &value[..4] == "0000"
        || NaiveDate::parse_from_str(&value[..10], "%Y-%m-%d").is_err()
    {
        return false;
    }
    match &value[10..] {
        "" | "Z" => true,
        zone if zone.len() == 6 => {
            let z = zone.as_bytes();
            matches!(z[0], b'+' | b'-')
                && z[3] == b':'
                && z[1..3].iter().chain(&z[4..6]).all(u8::is_ascii_digit)
                && zone[1..3].parse::<u8>().is_ok_and(|h| h <= 14)
                && zone[4..6]
                    .parse::<u8>()
                    .is_ok_and(|m| m < 60 && (&zone[1..3] != "14" || m == 0))
        }
        _ => false,
    }
}
