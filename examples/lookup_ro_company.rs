//! Look up one explicitly supplied Romanian CUI through ANAF.
use eu_company_lookup::RoRegistryClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let id = args
        .next()
        .ok_or("usage: cargo run --example lookup_ro_company -- <CUI>")?;
    if args.next().is_some() {
        return Err("supply exactly one CUI".into());
    }
    let result = RoRegistryClient::new()?.lookup(&id).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
