use serde::Serialize;

/// Country identity, covering the EU/EEA and GB for the existing XI VAT adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum Country {
    /// Austria.
    AT,
    /// Belgium.
    BE,
    /// Bulgaria.
    BG,
    /// Cyprus.
    CY,
    /// Czechia.
    CZ,
    /// Germany.
    DE,
    /// Denmark.
    DK,
    /// Estonia.
    EE,
    /// Greece.
    GR,
    /// Spain.
    ES,
    /// Finland.
    FI,
    /// France.
    FR,
    /// Croatia.
    HR,
    /// Hungary.
    HU,
    /// Ireland.
    IE,
    /// Italy.
    IT,
    /// Lithuania.
    LT,
    /// Luxembourg.
    LU,
    /// Latvia.
    LV,
    /// Malta.
    MT,
    /// Netherlands.
    NL,
    /// Poland.
    PL,
    /// Portugal.
    PT,
    /// Romania.
    RO,
    /// Sweden.
    SE,
    /// Slovenia.
    SI,
    /// Slovakia.
    SK,
    /// Iceland.
    IS,
    /// Liechtenstein.
    LI,
    /// Norway.
    NO,
    /// United Kingdom.
    GB,
}
impl Country {
    /// All supported country metadata entries; this is not lookup coverage.
    pub const ALL: &'static [Self] = &[
        Self::AT,
        Self::BE,
        Self::BG,
        Self::CY,
        Self::CZ,
        Self::DE,
        Self::DK,
        Self::EE,
        Self::GR,
        Self::ES,
        Self::FI,
        Self::FR,
        Self::HR,
        Self::HU,
        Self::IE,
        Self::IT,
        Self::LT,
        Self::LU,
        Self::LV,
        Self::MT,
        Self::NL,
        Self::PL,
        Self::PT,
        Self::RO,
        Self::SE,
        Self::SI,
        Self::SK,
        Self::IS,
        Self::LI,
        Self::NO,
        Self::GB,
    ];
    /// ISO 3166-1 alpha-2 code.
    pub const fn iso2(self) -> &'static str {
        match self {
            Self::AT => "AT",
            Self::BE => "BE",
            Self::BG => "BG",
            Self::CY => "CY",
            Self::CZ => "CZ",
            Self::DE => "DE",
            Self::DK => "DK",
            Self::EE => "EE",
            Self::GR => "GR",
            Self::ES => "ES",
            Self::FI => "FI",
            Self::FR => "FR",
            Self::HR => "HR",
            Self::HU => "HU",
            Self::IE => "IE",
            Self::IT => "IT",
            Self::LT => "LT",
            Self::LU => "LU",
            Self::LV => "LV",
            Self::MT => "MT",
            Self::NL => "NL",
            Self::PL => "PL",
            Self::PT => "PT",
            Self::RO => "RO",
            Self::SE => "SE",
            Self::SI => "SI",
            Self::SK => "SK",
            Self::IS => "IS",
            Self::LI => "LI",
            Self::NO => "NO",
            Self::GB => "GB",
        }
    }
    /// Native display names, with multiple languages separated by slashes.
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::AT => "Österreich",
            Self::BE => "België / Belgique / Belgien",
            Self::BG => "България",
            Self::CY => "Κύπρος / Kıbrıs",
            Self::CZ => "Česko",
            Self::DE => "Deutschland",
            Self::DK => "Danmark",
            Self::EE => "Eesti",
            Self::GR => "Ελλάδα",
            Self::ES => "España",
            Self::FI => "Suomi / Finland",
            Self::FR => "France",
            Self::HR => "Hrvatska",
            Self::HU => "Magyarország",
            Self::IE => "Éire / Ireland",
            Self::IT => "Italia",
            Self::LT => "Lietuva",
            Self::LU => "Lëtzebuerg / Luxembourg / Luxemburg",
            Self::LV => "Latvija",
            Self::MT => "Malta",
            Self::NL => "Nederland",
            Self::PL => "Polska",
            Self::PT => "Portugal",
            Self::RO => "România",
            Self::SE => "Sverige",
            Self::SI => "Slovenija",
            Self::SK => "Slovensko",
            Self::IS => "Ísland",
            Self::LI => "Liechtenstein",
            Self::NO => "Norge / Noreg",
            Self::GB => "United Kingdom",
        }
    }
    /// English display name used to recognize explicit source countries.
    pub const fn english_name(self) -> &'static str {
        match self {
            Self::AT => "Austria",
            Self::BE => "Belgium",
            Self::BG => "Bulgaria",
            Self::CY => "Cyprus",
            Self::CZ => "Czechia",
            Self::DE => "Germany",
            Self::DK => "Denmark",
            Self::EE => "Estonia",
            Self::GR => "Greece",
            Self::ES => "Spain",
            Self::FI => "Finland",
            Self::FR => "France",
            Self::HR => "Croatia",
            Self::HU => "Hungary",
            Self::IE => "Ireland",
            Self::IT => "Italy",
            Self::LT => "Lithuania",
            Self::LU => "Luxembourg",
            Self::LV => "Latvia",
            Self::MT => "Malta",
            Self::NL => "Netherlands",
            Self::PL => "Poland",
            Self::PT => "Portugal",
            Self::RO => "Romania",
            Self::SE => "Sweden",
            Self::SI => "Slovenia",
            Self::SK => "Slovakia",
            Self::IS => "Iceland",
            Self::LI => "Liechtenstein",
            Self::NO => "Norway",
            Self::GB => "United Kingdom",
        }
    }
    /// Whether this country belongs to the EU metadata set.
    pub const fn is_eu(self) -> bool {
        !matches!(self, Self::IS | Self::LI | Self::NO | Self::GB)
    }
    /// Whether this country belongs to the EEA metadata set.
    pub const fn is_eea(self) -> bool {
        !matches!(self, Self::GB)
    }
    /// Parse an ISO country code, case-insensitively. VAT aliases EL/XI are not ISO codes.
    pub fn from_iso2(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|c| c.iso2().eq_ignore_ascii_case(value.trim()))
    }
    /// Recognize explicit ISO, native or English country names without guessing jurisdiction.
    pub fn from_name(value: &str) -> Option<Self> {
        let value = value.trim().to_lowercase();
        Self::ALL.iter().copied().find(|c| {
            c.iso2().to_lowercase() == value
                || c.english_name().to_lowercase() == value
                || c.native_name().to_lowercase() == value
                || c.native_name()
                    .split(" / ")
                    .any(|n| n.to_lowercase() == value)
        })
    }
}
