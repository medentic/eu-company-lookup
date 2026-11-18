//! Identity-bound free registry overview; no paid certificate or document gateway.
use super::{IsBusinessId, IsRegistrationOverview};
use crate::RegistryFailure;
use scraper::{Html, Selector};
use std::collections::BTreeMap;

pub(super) type Fields = BTreeMap<String, String>;
fn unexpected<T>() -> Result<T, RegistryFailure> {
    Err(RegistryFailure::UnexpectedResponse)
}
fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static selector")
}
fn html(bytes: &[u8]) -> Result<Html, RegistryFailure> {
    Ok(Html::parse_document(
        std::str::from_utf8(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?,
    ))
}

pub(super) fn cart(bytes: &[u8]) -> Result<String, RegistryFailure> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Cart {
        add_cart_item_result: bool,
        shopping_cart_url: String,
        cart_item_count: usize,
    }
    let cart: Cart =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if !cart.add_cart_item_result || cart.cart_item_count != 1 {
        return unexpected();
    }
    let url = reqwest::Url::parse(&cart.shopping_cart_url)
        .map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str() != Some("vefur.rsk.is")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.path() != "/Vefverslun/Default.aspx"
    {
        return unexpected();
    }
    let pairs: Vec<_> = url.query_pairs().collect();
    if pairs.len() != 1
        || pairs[0].0 != "kid"
        || pairs[0].1.is_empty()
        || pairs[0].1.len() > 64
        || !pairs[0].1.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return unexpected();
    }
    Ok(pairs[0].1.to_string())
}

fn hidden(form: scraper::ElementRef<'_>) -> Result<Fields, RegistryFailure> {
    let mut fields = Fields::new();
    for input in form.select(&selector("input[type=hidden]")) {
        let name = input
            .value()
            .attr("name")
            .ok_or(RegistryFailure::UnexpectedResponse)?;
        let value = input.value().attr("value").unwrap_or("");
        if name.len() > 128
            || value.len() > 131_072
            || fields.len() >= 32
            || fields.insert(name.to_owned(), value.to_owned()).is_some()
        {
            return unexpected();
        }
    }
    Ok(fields)
}
fn single_free_item(doc: &Html, id: &IsBusinessId) -> Result<(), RegistryFailure> {
    let tables: Vec<_> = doc
        .select(&selector("table#MainContent_ucVoruGrid_GridView1"))
        .collect();
    if tables.len() != 1 {
        return unexpected();
    }
    let rows: Vec<_> = tables[0]
        .select(&selector("tr"))
        .filter(|r| {
            r.select(&selector("td"))
                .any(|cell| !cell.text().collect::<String>().trim().is_empty())
        })
        .collect();
    if rows.len() != 1 {
        return unexpected();
    }
    let cells: Vec<_> = rows[0]
        .select(&selector("td"))
        .map(|e| e.text().collect::<String>().trim().to_owned())
        .collect();
    if cells.len() != 4 || cells[0] != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if cells[2] != "0" {
        return unexpected();
    }
    Ok(())
}

pub(super) fn page_fields(
    bytes: &[u8],
    id: &IsBusinessId,
    download: bool,
) -> Result<Fields, RegistryFailure> {
    let doc = html(bytes)?;
    single_free_item(&doc, id)?;
    let forms: Vec<_> = doc.select(&selector("form#ctl01")).collect();
    if forms.len() != 1 {
        return unexpected();
    }
    let mut fields = hidden(forms[0])?;
    if !fields.contains_key("__VIEWSTATE")
        || !fields.contains_key("__EVENTVALIDATION")
        || !fields.contains_key("ctl00$MainContent$ucVoruGrid$hfDownloadToken")
    {
        return unexpected();
    }
    fields.insert("__EVENTTARGET".into(), String::new());
    fields.insert("__EVENTARGUMENT".into(), String::new());
    if download {
        if forms[0]
            .select(&selector(
                "input[name='ctl00$MainContent$ucVoruGrid$GridView1$ctl02$Btn_Saekja']",
            ))
            .count()
            != 1
        {
            return unexpected();
        }
        fields.insert("hfMouseClicked".into(), "true".into());
        fields.insert(
            "ctl00$MainContent$ucVoruGrid$GridView1$ctl02$Btn_Saekja".into(),
            String::new(),
        );
    } else {
        if forms[0]
            .select(&selector("input[name='ctl00$MainContent$btnKaupa']"))
            .count()
            != 1
        {
            return unexpected();
        }
        fields.insert("hfKaupaMouseClicked".into(), "true".into());
        fields.insert("ctl00$MainContent$btnKaupa".into(), "Áfram".into());
        for key in ["buyername", "buyeremail", "merchantemail"] {
            fields.insert(key.into(), String::new());
        }
        fields.insert("language".into(), "IS".into());
        fields.insert("currency".into(), "ISK".into());
    }
    Ok(fields)
}

pub(super) fn confirmation(bytes: &[u8], kid: &str) -> Result<Fields, RegistryFailure> {
    let doc = html(bytes)?;
    let mut found = None;
    for form in doc.select(&selector("form")) {
        let fields = hidden(form)?;
        if fields.contains_key("amount") {
            if found.is_some()
                || fields.get("amount").map(String::as_str) != Some("0")
                || fields.get("ReferenceNumber").map(String::as_str) != Some(kid)
                || form.value().attr("action")
                    != Some(
                        format!("https://vefur.rsk.is/Vefverslun/ReturnPage.aspx?kid={kid}")
                            .as_str(),
                    )
            {
                return unexpected();
            }
            found = Some(fields);
        }
    }
    found.ok_or(RegistryFailure::UnexpectedResponse)
}

pub(super) fn form_body(fields: &Fields) -> String {
    let mut url = reqwest::Url::parse("https://vefur.rsk.is/").expect("static URL");
    url.query_pairs_mut().extend_pairs(fields);
    url.query().unwrap_or("").to_owned()
}

pub(super) fn pdf(
    bytes: &[u8],
    id: &IsBusinessId,
) -> Result<IsRegistrationOverview, RegistryFailure> {
    if !bytes.starts_with(b"%PDF-") {
        return unexpected();
    }
    let document = lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions {
            strict: true,
            ..lopdf::LoadOptions::with_max_decompressed_size(1_048_576)
        },
    )
    .map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if document.is_encrypted() {
        return unexpected();
    }
    let text = document
        .extract_text_with_limit(&[1], 1_048_576)
        .map_err(|_| RegistryFailure::UnexpectedResponse)?;
    text_overview(&text, id)
}
fn text_overview(text: &str, id: &IsBusinessId) -> Result<IsRegistrationOverview, RegistryFailure> {
    let capture = |pattern: &str| -> Result<String, RegistryFailure> {
        let regex = regex_lite::Regex::new(pattern).expect("static expression");
        let matches: Vec<_> = regex.captures_iter(text).collect();
        if matches.len() != 1 {
            return unexpected();
        }
        Ok(matches[0][1].trim().to_owned())
    };
    let identifier = capture(r"Kt:\s*(\d{6}-\d{4})")?;
    if identifier.replace('-', "") != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    let date = capture(r"Yfirlit\s*lögaðila\s*(\d{2}\.\d{2}\.\d{4})")?;
    let issued_on = jiff::civil::Date::strptime("%d.%m.%Y", &date)
        .map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if issued_on.strftime("%d.%m.%Y").to_string() != date {
        return unexpected();
    }
    let native_status = capture(r"(?s)Staða:\s*(.*?)\s*-\s*Stofnað/skráð:")?;
    if native_status.is_empty() || native_status.len() > 200 {
        return unexpected();
    }
    Ok(IsRegistrationOverview {
        kennitala: id.to_string(),
        issued_on,
        native_status,
    })
}

pub(super) fn available(bytes: &[u8], id: &IsBusinessId) -> Result<bool, RegistryFailure> {
    let doc = html(bytes)?;
    let buttons: Vec<_> = doc
        .select(&selector("div.company button[data-typeid='9']"))
        .collect();
    match buttons.as_slice() {
        [] => Ok(false),
        [button] if button.value().attr("data-itemid") == Some(id.as_str()) => Ok(true),
        [_] => Err(RegistryFailure::MismatchedResponse),
        _ => unexpected(),
    }
}

#[cfg(test)]
pub(super) fn synthetic_pdf(text: &[&str]) -> Vec<u8> {
    use lopdf::{
        Object, Stream,
        content::{Content, Operation},
        dictionary,
    };
    let mut doc = lopdf::Document::with_version("1.4");
    let pages = doc.new_object_id();
    let font=doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let resources = doc.add_object(dictionary! {"Font"=>dictionary!{"F1"=>font}});
    let mut operations = vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
    ];
    for (i, line) in text.iter().enumerate() {
        operations.push(Operation::new(
            "Tm",
            vec![
                1.into(),
                0.into(),
                0.into(),
                1.into(),
                20.into(),
                (780 - i as i64 * 20).into(),
            ],
        ));
        let bytes: Vec<u8> = line
            .chars()
            .map(|c| u8::try_from(c as u32).unwrap())
            .collect();
        operations.push(Operation::new("Tj", vec![Object::string_literal(bytes)]));
        operations.push(Operation::new("T*", vec![]));
    }
    operations.push(Operation::new("ET", vec![]));
    let contents = doc.add_object(Stream::new(
        dictionary! {},
        Content { operations }.encode().unwrap(),
    ));
    let page = doc.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"Contents"=>contents});
    doc.objects.insert(pages,Object::Dictionary(dictionary!{"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1,"Resources"=>resources,"MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()]}));
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    doc.trailer.set("Root", catalog);
    let mut output = Vec::new();
    doc.save_to(&mut output).unwrap();
    output
}

#[cfg(test)]
mod contracts {
    use super::*;
    #[test]
    fn overview_binds_identity_date_and_explicit_status() {
        let lines = [
            "Yfirlit lögaðila",
            "18.11.2026",
            "SYNTHETIC EHF. Kt: 410100-0000",
            "Staða: Virk skráning",
            "- Stofnað/skráð: 01.01.2000",
        ];
        let id = IsBusinessId::parse("4101000000").unwrap();
        let value = pdf(&synthetic_pdf(&lines), &id).unwrap();
        assert_eq!(value.kennitala, id.as_str());
        assert_eq!(value.native_status, "Virk skráning");
        assert_eq!(value.issued_on.to_string(), "2026-11-18");
        for (index, bad) in [
            (1, "31.02.2026"),
            (2, "Kt: 420100-0000"),
            (3, "No company status"),
            (0, "Some unrelated overview"),
        ] {
            let mut bad_lines = lines;
            bad_lines[index] = bad;
            assert!(pdf(&synthetic_pdf(&bad_lines), &id).is_err());
        }
        let duplicate = lines.join("\n") + "\nStaða: Other - Stofnað/skráð: 01.01.2000";
        assert!(text_overview(&duplicate, &id).is_err());
        assert!(pdf(b"<html>challenge</html>", &id).is_err());
    }
    #[test]
    fn cart_cannot_select_another_host_or_multiple_items() {
        let raw = serde_json::json!({"addCartItemResult":true,"shoppingCartUrl":"http://vefur.rsk.is/Vefverslun/Default.aspx?kid=ABC123","cartItemCount":1});
        assert_eq!(cart(&serde_json::to_vec(&raw).unwrap()).unwrap(), "ABC123");
        for url in [
            "https://evil.test/Vefverslun/Default.aspx?kid=ABC123",
            "https://vefur.rsk.is/Vefverslun/Default.aspx?kid=ABC&other=1",
            "https://vefur.rsk.is/Vefverslun/Default.aspx?kid=../wrong",
            "https://user@vefur.rsk.is/Vefverslun/Default.aspx?kid=ABC",
        ] {
            let mut bad = raw.clone();
            bad["shoppingCartUrl"] = serde_json::json!(url);
            assert!(cart(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = raw;
        bad["cartItemCount"] = serde_json::json!(2);
        assert!(cart(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    #[test]
    fn confirmation_requires_zero_cost_same_cart_and_same_origin() {
        let source = r#"<form action="https://vefur.rsk.is/Vefverslun/ReturnPage.aspx?kid=ABC123"><input type="hidden" name="amount" value="0"><input type="hidden" name="ReferenceNumber" value="ABC123"></form>"#;
        assert!(confirmation(source.as_bytes(), "ABC123").is_ok());
        for bad in [
            source.replace("value=\"0\"", "value=\"1500\""),
            source.replace("vefur.rsk.is", "evil.test"),
            source.replace("value=\"ABC123\"", "value=\"OTHER\""),
        ] {
            assert!(confirmation(bad.as_bytes(), "ABC123").is_err());
        }
    }
}
