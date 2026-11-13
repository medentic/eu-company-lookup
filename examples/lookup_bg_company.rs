//! Look up explicit Bulgarian business codes with one shared client.
use eu_company_lookup::{Client, LookupError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ids: Vec<_> = std::env::args().skip(1).collect();
    if ids.is_empty() {
        return Err("usage: cargo run --example lookup_bg_company -- <EIK-or-BULSTAT> ...".into());
    }
    let client = Client::new()?;
    for id in ids {
        match client.lookup_bg_company(&id).await {
            Ok(result) => println!("{}", serde_json::to_string_pretty(&result)?),
            Err(LookupError::NotFound) => {
                println!("NotFound: neither Bulgarian register returned a matching record")
            }
            Err(error) => {
                if error.is_service_unavailable() {
                    eprintln!("Verification unavailable; the identifier may still be correct.");
                }
                return Err(error.into());
            }
        }
    }
    Ok(())
}
