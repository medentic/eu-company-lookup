use super::IeCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult};
impl From<IeCompanyLookupResult> for CompanyLookupResult {
    fn from(v: IeCompanyLookupResult) -> Self {
        let legal_form = v
            .details
            .company_type
            .clone()
            .filter(|s| !s.trim().is_empty());
        let native_registry_status = v
            .details
            .company_status
            .clone()
            .or_else(|| v.details.company_status_code.map(|code| code.to_string()));
        let status = match v.details.company_status_code {
            Some(code) => {
                let mapped = code_lifecycle(code);
                let text = v.details.company_status.as_deref().map(lifecycle);
                Some(if text.is_some_and(|s| s != mapped) {
                    crate::CompanyStatus::Unknown
                } else {
                    mapped
                })
            }
            None => native_registry_status.as_deref().map(lifecycle),
        };
        // CRO records the company's statutory registered-office address.
        // Keep source lines in order without guessing a city/county from their position.
        let lines: Vec<_> = [
            &v.details.company_address_1,
            &v.details.company_address_2,
            &v.details.company_address_3,
            &v.details.company_address_4,
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .collect();
        let postcode = v.details.eircode.clone().filter(|s| !s.trim().is_empty());
        let registered_office =
            (!lines.is_empty() || postcode.is_some()).then(|| crate::CompanyAddress {
                full_address_by_registry: None,
                fields: crate::locations::Address {
                    address: lines.first().cloned(),
                    address2: (lines.len() > 1).then(|| lines[1..].join("\n")),
                    postcode,
                    ..Default::default()
                },
            });
        let registry_details = CompanyDetails::IrishCro {
            native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "IE".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            source: registry_details.provider(),
            registry_details,
            registered_office,
            legal_form,
            status,
            fiscal_address: None,
            vat_registered: None,
            vat_id: None,
            registry_url: None,
            foreign_name: None,
        }
    }
}

// Exact provider states; insolvency/proceedings and future values remain distinguishable.
fn lifecycle(value: &str) -> crate::CompanyStatus {
    use crate::CompanyStatus::*;
    match value.trim().to_uppercase().as_str() {
        "NORMAL (FORM E8 (NOTICE OF APPOINTMENT OF RECEIVER) PER S436 CA 2014 HAS BEEN FILED WITH THE CRO IN RESPECT OF PART OR ALL OF THE PROPERTY OF THE COMPANY, WHICH PROPERTY MAY OR MAY NOT BE BENEFICIALLY OWNED BY THE COMPANY. PLEASE REFER TO THE RELEVANT E8 FORM(S) FOR FURTHER INFORMATION)" => {
            Receivership
        }
        "LIQUIDATION" | "LIQUIDATION (UK)" => Liquidation,
        "LIQUIDATION(RECEIVERSHIP)" | "LIQUIDATION (RECEIVERSHIP)" => LiquidationAndReceivership,
        "LIQUIDATION STAYED" => LiquidationStayed,
        "EXAMINERSHIP" => Restructuring,
        "STRIKE OFF LISTED" => PendingClosure,
        "ADMINISTRATION" | "ADMINISTRATION (UK)" => Administration,
        "INACTIVE" => Inactive,
        "NORMAL" | "REINSTATED" | "REINSTATED HCO" => Active,
        "STRUCK OFF"
        | "DISSOLVED"
        | "DISSOLVED-20 YEARS"
        | "CEASED"
        | "CEASED IRL"
        | "DELETED CB MERGER"
        | "DELETED PER SE MERGE"
        | "DELETED AS PER SE TR"
        | "DISSOLVED POSTMERGER"
        | "DISSOLVED BY DIVIS"
        | "DE-REGISTERED ICAV"
        | "DELETED CB CONVERSION"
        | "DE-REGISTERED INVESTMENT COMPANY" => Closed,
        _ => Unknown,
    }
}

fn code_lifecycle(code: u64) -> crate::CompanyStatus {
    use crate::CompanyStatus::*;
    match code {
        1052 | 1149 | 1152 | 1177 => Liquidation,
        1053 | 1153 => Receivership,
        1055 | 1155 => LiquidationAndReceivership,
        1056 | 1154 => Restructuring,
        1156 => PendingClosure,
        1165 => LiquidationStayed,
        1067 | 1167 | 1170 => Administration,
        1181 => Inactive,
        1051 | 1141 | 1151 | 1160 | 1161 => Active,
        1058 | 1063 | 1064 | 1148 | 1157 | 1158 | 1159 | 1163 | 1168 | 1169 | 1172 | 1173
        | 1174 | 1176 | 1178 | 1310 => Closed,
        _ => Unknown,
    }
}

#[test]
fn documented_cro_proceedings_have_distinct_states() {
    use crate::CompanyStatus::*;
    for (code, label, expected) in [
        (1152, "Liquidation", Liquidation),
        (
            1155,
            "Liquidation(Receivership)",
            LiquidationAndReceivership,
        ),
        (1154, "Examinership", Restructuring),
        (1156, "Strike Off Listed", PendingClosure),
        (1165, "Liquidation stayed", LiquidationStayed),
        (1167, "Administration (UK)", Administration),
        (1181, "InActive", Inactive),
    ] {
        assert_eq!(code_lifecycle(code), expected);
        assert_eq!(lifecycle(label), expected);
    }
    assert_eq!(code_lifecycle(1153), Receivership);
    for code in [1050, 1150, 1062, 1162, 1166, 9999] {
        assert_eq!(code_lifecycle(code), Unknown);
    }
}
