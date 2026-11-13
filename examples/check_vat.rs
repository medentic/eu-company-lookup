//! Look up VAT validity and company details through the live VIES service.
use eu_company_lookup::{Client, VatLookupResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vat = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example check_vat -- <VAT-ID>")?;
    let result: VatLookupResult = Client::new()?.lookup_vat(&vat).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
