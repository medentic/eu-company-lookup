use super::*;
use crate::{ClientBuilder, ClientConfig};
use lopdf::{Document, Permissions};

#[test]
fn observation_requires_a_matched_decision_with_date_and_reference() {
    let vat = VatId::parse("BG123456786").unwrap();
    let observation = || Observation {
        country: "BG".into(),
        number: vat.number().into(),
        date: "01/01/2026 12:34:56".into(),
        reference: "SYNTHETIC-123".into(),
        outcome: "Yes, valid VAT number".into(),
        cells: Vec::new(),
    };
    assert!(observation().fields(&vat).is_ok());
    assert!(
        Observation {
            outcome: "No, invalid VAT number for cross border transactions within the EU".into(),
            ..observation()
        }
        .fields(&vat)
        .is_ok()
    );
    assert!(matches!(
        Observation {
            number: "111".into(),
            ..observation()
        }
        .fields(&vat),
        Err(LookupFailure::MismatchedResponse)
    ));
    assert!(matches!(
        Observation {
            reference: "---".into(),
            ..observation()
        }
        .fields(&vat),
        Err(LookupFailure::ViesPdf("consultation_number_missing"))
    ));
    assert!(
        Observation {
            date: String::new(),
            ..observation()
        }
        .fields(&vat)
        .is_err()
    );
    assert!(
        Observation {
            outcome: "Service unavailable".into(),
            ..observation()
        }
        .fields(&vat)
        .is_err()
    );
}

// A synthetic, local form exercises submission and Chromium's own print layout.
// Padding forces a second sheet at full scale. A second submit invalidates the result.
const FORM: &str = r#"<!doctype html><meta charset="utf-8"><style>
body {font:16px sans-serif} table {border-collapse:collapse;width:100%}
td {padding:12px;border-bottom:1px solid #ddd} @media print {main {min-height:1150px}}
</style><main><h1>VIES VAT number validation</h1><form id="vat-validation-form">
<select formcontrolname="country"><option></option><option>BG</option></select>
<input formcontrolname="vatNumber"><select formcontrolname="requesterCountry"><option></option><option>FI</option><option>BG</option></select>
<input formcontrolname="requesterVatNumber"><button data-testid="verifyBtn">Verify</button></form>
<section class="vat-result" hidden><p data-testid="textResult">Yes, valid VAT number</p>
<button id="print">Print</button><table>
<tr><td>Member State / Northern Ireland</td><td data-testid="msCode">BG</td></tr>
<tr><td>VAT Number</td><td data-testid="vatNumber">123456786</td></tr>
<tr><td>Date when request received</td><td data-testid="requestDate">01/01/2026 12:34:56</td></tr>
<tr><td>Name</td><td>Синтетична компания</td></tr>
<tr><td>Address</td><td>Примерна улица 1</td></tr>
<tr><td>Consultation Number</td><td data-testid="requestIdentifier"></td></tr>
</table></section></main><script>
document.querySelector('form').onsubmit = event => {
 event.preventDefault();
 const values = Array.from(event.target.querySelectorAll('[formcontrolname]'), e => e.value);
 if(values.join(',') !== 'BG,123456786,FI,20774740') return;
 const ref = document.querySelector('[data-testid="requestIdentifier"]');
 ref.textContent = ref.textContent ? '' : 'SYNTHETIC-CHECK-123';
 event.target.hidden = true; document.querySelector('section').hidden = false;
 location.hash='/vat-validation-result';
};
</script>"#;

#[tokio::test]
#[ignore = "offline browser integration; requires Chrome or Chromium and pdftotext"]
async fn prints_locked_a4_from_the_submitted_page() {
    let dir = tempfile::tempdir().unwrap();
    let form = dir.path().join("form.html");
    std::fs::write(&form, FORM).unwrap();
    let mut url = reqwest::Url::from_file_path(form).unwrap();
    url.set_fragment(Some("/vat-validation"));
    for (default_vat, requester) in [("FI20774740", None), ("BG123456786", Some("FI20774740"))] {
        let config = ClientConfig {
            requester_vat: Some(VatId::parse(default_vat).unwrap()),
            ..Default::default()
        };
        let mut client = ClientBuilder::empty().vies(config).build().unwrap();
        client.set_pdf_test_url(url.to_string());
        let bytes = client
            .vies_result_pdf("BG123456786", requester)
            .await
            .unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let state = doc.encryption_state.as_ref().unwrap();
        assert_eq!(state.revision(), 6);
        let permissions = state.permissions();
        assert!(permissions.contains(
            Permissions::PRINTABLE_IN_HIGH_QUALITY | Permissions::PRINTABLE | Permissions::COPYABLE
        ));
        assert!(!permissions.intersects(
            Permissions::MODIFIABLE
                | Permissions::ANNOTABLE
                | Permissions::FILLABLE
                | Permissions::ASSEMBLABLE
        ));
        assert_eq!(doc.get_pages().len(), 1);
        let page = doc.get_dictionary(doc.get_pages()[&1]).unwrap();
        let media = page.get(b"MediaBox").unwrap().as_array().unwrap();
        assert!((media[2].as_float().unwrap() - 595.28).abs() < 1.0);
        assert!((media[3].as_float().unwrap() - 841.89).abs() < 1.0);
        let text = doc.extract_text(&[1]).unwrap();
        for value in ["SYNTHETIC-CHECK-123", "123456786", "12:34:56", "Синтетична"] {
            assert!(text.contains(value), "{value}");
        }
    }
}
