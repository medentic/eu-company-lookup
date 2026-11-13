/// Map recognized Bulgarian legal forms without guessing from company names.
pub(super) fn abbreviate(source: &str) -> crate::LegalForm {
    let normalized = source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    match normalized.as_str() {
        "едноличен търговец" | "ет" => crate::LegalForm::Et,
        "събирателно дружество" | "сд" => crate::LegalForm::Sd,
        "еднолично дружество с ограничена отговорност" | "еоод" => {
            crate::LegalForm::Eood
        }
        "дружество с ограничена отговорност" | "оод" => {
            crate::LegalForm::Ood
        }
        "акционерно дружество" | "ад" => crate::LegalForm::Ad,
        "еднолично акционерно дружество" | "еад" => {
            crate::LegalForm::Ead
        }
        _ => crate::LegalForm::Other(source.to_owned()),
    }
}
