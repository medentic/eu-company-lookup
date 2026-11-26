use super::Result;
use crate::model::LookupFailure;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use lopdf::encryption::crypt_filters::{Aes256CryptFilter, CryptFilter};
use lopdf::{Document, EncryptionState, EncryptionVersion, Object, Permissions};
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn finish(bytes: &[u8]) -> Result<Option<Vec<u8>>> {
    let invalid = || LookupFailure::ViesPdf("invalid_printed_pdf");
    let doc = Document::load_mem(bytes).map_err(|_| invalid())?;
    let pages = doc.get_pages();
    if pages.len() > 1 {
        return Ok(None);
    }
    let page = pages.get(&1).ok_or_else(invalid)?;
    let media = doc
        .get_dictionary(*page)
        .and_then(|dict| dict.get(b"MediaBox"))
        .and_then(Object::as_array)
        .map_err(|_| invalid())?;
    let coords: Vec<f32> = media
        .iter()
        .map(Object::as_float)
        .collect::<lopdf::Result<_>>()
        .map_err(|_| invalid())?;
    if !matches!(coords.as_slice(), [x,y,r,t] if (r-x-595.28).abs() < 1. && (t-y-841.89).abs() < 1.)
    {
        return Err(invalid());
    }
    lock(doc)
        .map(Some)
        .map_err(|_| LookupFailure::ViesPdf("pdf_lock_failed"))
}

/// Read the final, encrypted document as a reader opening it without a password.
pub(super) async fn verify_text(pdf: &[u8], fields: &[String]) -> Result<()> {
    use std::process::Stdio;
    use tokio::{io::AsyncWriteExt, process::Command};

    let mut reader = Command::new("pdftotext")
        .args(["-raw", "-enc", "UTF-8", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| LookupFailure::InvalidConfiguration("pdftotext_required"))?;
    let mut stdin = reader
        .stdin
        .take()
        .ok_or(LookupFailure::ViesPdf("pdf_reader_failed"))?;
    let write_pdf = async move { stdin.write_all(pdf).await };
    let (_, output) = futures_util::future::try_join(write_pdf, reader.wait_with_output())
        .await
        .map_err(|_| LookupFailure::ViesPdf("pdf_reader_failed"))?;
    if !output.status.success() {
        return Err(LookupFailure::ViesPdf("pdf_reader_failed"));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| LookupFailure::ViesPdf("pdf_reader_failed"))?;
    let printed = compact(text);
    if fields
        .iter()
        .any(|field| !printed.contains(&compact(field)))
    {
        return Err(LookupFailure::ViesPdf("printed_result_incomplete"));
    }
    Ok(())
}

fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

fn lock(mut doc: Document) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut random = [0; 80];
    getrandom::fill(&mut random).map_err(|_| "secure random source unavailable")?;
    let owner = URL_SAFE_NO_PAD.encode(&random[32..64]);
    let filter: Arc<dyn CryptFilter> = Arc::new(Aes256CryptFilter);
    let state = EncryptionState::try_from(EncryptionVersion::V5 {
        encrypt_metadata: true,
        crypt_filters: BTreeMap::from([(b"StdCF".to_vec(), filter)]),
        file_encryption_key: &random[..32],
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: &owner,
        user_password: "",
        permissions: Permissions::PRINTABLE
            | Permissions::PRINTABLE_IN_HIGH_QUALITY
            | Permissions::COPYABLE
            | Permissions::COPYABLE_FOR_ACCESSIBILITY,
    })?;
    if !doc.trailer.has(b"ID") {
        let id = Object::String(random[64..].to_vec(), lopdf::StringFormat::Hexadecimal);
        doc.trailer.set("ID", vec![id.clone(), id]);
    }
    doc.version = "2.0".into();
    doc.encrypt(&state)?;
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes)?;
    Ok(bytes)
}
