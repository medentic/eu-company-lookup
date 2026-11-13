//! New result terminology preserves source compatibility with published 1.x names.
use eu_company_lookup::{
    BgCompanyLookupResult, BgRegistryClient, Client, CompanyRecord, LookupError, VatCheck,
    VatLookupResult,
};
use std::future::Future;

fn vat_lookup(_: impl Future<Output = Result<VatLookupResult, LookupError>> + Send) {}
fn legacy_vat_lookup(_: impl Future<Output = Result<VatCheck, LookupError>> + Send) {}
fn company_lookup(_: impl Future<Output = Result<BgCompanyLookupResult, LookupError>> + Send) {}
fn legacy_company_lookup(_: impl Future<Output = Result<CompanyRecord, LookupError>> + Send) {}

#[test]
fn lookup_methods_accept_both_preferred_and_legacy_result_annotations() {
    let client = Client::new().unwrap();
    let registry = BgRegistryClient::new().unwrap();
    // Futures are deliberately not polled: this is an offline consumer contract.
    vat_lookup(client.lookup_vat("BG123456789"));
    legacy_vat_lookup(client.lookup_vat("BG123456789"));
    company_lookup(client.lookup_bg_company("123456789"));
    legacy_company_lookup(client.lookup_bg_company("123456789"));
    company_lookup(registry.lookup("123456789"));
    legacy_company_lookup(registry.lookup("123456789"));
}

#[test]
fn aliases_are_interchangeable_without_conversion() {
    fn vat_forward(value: VatCheck) -> VatLookupResult {
        value
    }
    fn vat_backward(value: VatLookupResult) -> VatCheck {
        value
    }
    fn company_forward(value: CompanyRecord) -> BgCompanyLookupResult {
        value
    }
    fn company_backward(value: BgCompanyLookupResult) -> CompanyRecord {
        value
    }
    // Function pointers enforce identity in both directions, not just conversion support.
    let _: fn(VatLookupResult) -> VatCheck = vat_forward;
    let _: fn(VatCheck) -> VatLookupResult = vat_backward;
    let _: fn(BgCompanyLookupResult) -> CompanyRecord = company_forward;
    let _: fn(CompanyRecord) -> BgCompanyLookupResult = company_backward;
}
