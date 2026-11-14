//! Look up one explicitly supplied Romanian CUI through ANAF.
use eu_company_lookup::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let id = args
        .next()
        .ok_or("usage: cargo run --example lookup_ro_company -- <CUI>")?;
    if args.next().is_some() {
        return Err("supply exactly one CUI".into());
    }
    let result = Client::new()?.lookup_company(&id, "RO").await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
