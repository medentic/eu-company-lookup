use super::*;
fn continued_source() -> Value {
    let mut raw = active_source();
    raw["odpis"]["dane"]["dzial1"]["danePodmiotu"]["formaPrawna"] =
        json!("SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ");
    raw["odpis"]["dane"]["dzial6"] = json!({
        "likwidacja":[
            {"otwarcieLikwidacji":"UCHWAŁA NR 1 Z DNIA 01.01.2010 O LIKWIDACJI SPÓŁKI", "zakonczenieLikwidacji":"UCHWAŁA NR 2 Z DNIA 01.01.2011 O DALSZYM ISTNIENIU SPÓŁKI. , 01.01.2011"},
            {"otwarcieLikwidacji":"UCHWAŁA NR 3 Z DNIA 01.01.2020 O LIKWIDACJI SPÓŁKI", "zakonczenieLikwidacji":"UCHWAŁA NR 4 Z DNIA 01.01.2021 O DALSZYM ISTNIENIU SPÓŁKI. , 01.01.2021", "likwidatorzy":[{}]}
        ],
        "rozwiazanieUniewaznienie":{"okreslenieOkolicznosci":"ROZWIĄZANIE", "informacjaORozwiazaniuUniewaznieniu":"UCHWAŁA NR 3 Z DNIA 01.01.2020 O LIKWIDACJI SPÓŁKI, 01.01.2020"}
    });
    raw
}
fn duplicated_source() -> Value {
    let mut raw = continued_source();
    raw["odpis"]["dane"]["dzial1"]["danePodmiotu"]["nazwa"] =
        json!("Synthetic sp. z o.o. W LIKWIDACJI");
    let order = "SĄD REJESTROWY, POSTANOWIENIEM ORZEKŁ O JEJ ROZWIĄZANIU I USTANOWIŁ LIKWIDATORA, 01.01.2020";
    raw["odpis"]["dane"]["dzial6"] = json!({"likwidacja":[{"otwarcieLikwidacji":order,"zakonczenieLikwidacji":order,"likwidatorzy":[{}]}]});
    raw
}
fn common(raw: &Value) -> CompanyLookupResult {
    decode::decode(
        &serde_json::to_vec(raw).unwrap(),
        &PlBusinessId::parse("0001234567").unwrap(),
    )
    .unwrap()
    .into()
}
#[test]
fn continued_existence_reconciles_historical_dissolution_and_retained_liquidator() {
    let raw = continued_source();
    assert_eq!(common(&raw).status, Some(crate::CompanyStatus::Active));
    assert_eq!(
        serde_json::to_value(common(&raw)).unwrap(),
        crate::test_support::documentation::example("pl", 4)
    );
    for (pointer, value) in [
        (
            "/odpis/dane/dzial6/likwidacja/1/zakonczenieLikwidacji",
            json!("UCHWAŁA NR 4 NIE PODJĘTO UCHWAŁY O DALSZYM ISTNIENIU SPÓŁKI. , 01.01.2021"),
        ),
        (
            "/odpis/dane/dzial6/likwidacja/1/zakonczenieLikwidacji",
            json!("UCHWAŁA NR 4 O DALSZYM ISTNIENIU SPÓŁKI. , 01.01.2030"),
        ),
        (
            "/odpis/dane/dzial6/likwidacja/1/zakonczenieLikwidacji",
            json!("UCHWAŁA NR 4 O DALSZYM ISTNIENIU SPÓŁKI. , 01.01.2019"),
        ),
        (
            "/odpis/dane/dzial6/likwidacja/1/zakonczenieLikwidacji",
            json!("UCHWAŁA NR 4 O DALSZYM ISTNIENIU SPÓŁKI."),
        ),
        (
            "/odpis/dane/dzial6/rozwiazanieUniewaznienie/informacjaORozwiazaniuUniewaznieniu",
            json!("A different dissolution order"),
        ),
        (
            "/odpis/dane/dzial6/rozwiazanieUniewaznienie/okreslenieOkolicznosci",
            json!("UNIEWAŻNIENIE"),
        ),
        (
            "/odpis/dane/dzial1/danePodmiotu/nazwa",
            json!("Synthetic W LIKWIDACJI"),
        ),
    ] {
        let mut mutated = raw.clone();
        *mutated.pointer_mut(pointer).unwrap() = value;
        assert_ne!(
            common(&mutated).status,
            Some(crate::CompanyStatus::Active),
            "{pointer}"
        );
    }
}
#[test]
fn duplicated_opening_requires_appointment_name_and_current_liquidator() {
    let raw = duplicated_source();
    assert_eq!(common(&raw).status, Some(crate::CompanyStatus::Liquidation));
    assert_eq!(
        serde_json::to_value(common(&raw)).unwrap(),
        crate::test_support::documentation::example("pl", 5)
    );
    for (pointer, value) in [
        ("/odpis/dane/dzial6/likwidacja/0/likwidatorzy", json!([])),
        (
            "/odpis/dane/dzial6/likwidacja/0/zakonczenieLikwidacji",
            json!("Liquidation ended"),
        ),
        (
            "/odpis/dane/dzial1/danePodmiotu/nazwa",
            json!("Synthetic sp. z o.o."),
        ),
        ("/odpis/naglowekA/stanZDnia", json!("01.01.2019")),
    ] {
        let mut mutated = raw.clone();
        *mutated.pointer_mut(pointer).unwrap() = value;
        assert_eq!(common(&mutated).status, Some(crate::CompanyStatus::Unknown));
    }
    let mut injected = raw.clone();
    injected["odpis"]["dane"]["dzial6"]["likwidacja"][0]
        .as_object_mut()
        .unwrap()
        .remove("likwidatorzy");
    injected["odpis"]["dane"]["dzial6"]["likwidacja"][0]["liquidatorCount"] = json!(1);
    assert_eq!(
        common(&injected).status,
        Some(crate::CompanyStatus::Unknown)
    );
}

#[test]
fn cancelled_liquidation_without_retained_opening_requires_explicit_resolution() {
    let mut raw = active_source();
    raw["odpis"]["dane"]["dzial1"]["danePodmiotu"]["formaPrawna"] =
        json!("SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ");
    let ending = "01.01.2021 R., NOTARIUSZ SYNTHETIC, UCHWAŁA NR 2 W SPRAWIE DALSZEGO ISTNIENIA SPÓŁKI, UCHYLENIA LIKWIDACJI I ODWOŁANIA LIKWIDATORA , 01.01.2021";
    raw["odpis"]["dane"]["dzial6"] = json!({"likwidacja":[{"zakonczenieLikwidacji":ending}]});
    assert_eq!(common(&raw).status, Some(crate::CompanyStatus::Active));
    assert_eq!(
        serde_json::to_value(common(&raw)).unwrap(),
        crate::test_support::documentation::example("pl", 8)
    );
    for bad in [
        ending.replace("2021", "2030"),
        ending.replace("UCHWAŁA NR", "PROJEKT UCHWAŁA NR"),
        ending.replace("UCHWAŁA NR", "NIE PODJĘTO: UCHWAŁA NR"),
        ending.replace(" I ODWOŁANIA LIKWIDATORA", ""),
        ending.replace("01.01.2021", "31.02.2021"),
    ] {
        let mut changed = raw.clone();
        changed["odpis"]["dane"]["dzial6"]["likwidacja"][0]["zakonczenieLikwidacji"] = json!(bad);
        assert_ne!(common(&changed).status, Some(crate::CompanyStatus::Active));
    }
    for (pointer, value) in [
        (
            "/odpis/dane/dzial1/danePodmiotu/nazwa",
            json!("Synthetic W LIKWIDACJI"),
        ),
        ("/odpis/naglowekA/dataOstatniegoWpisu", Value::Null),
    ] {
        let mut changed = raw.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert_ne!(common(&changed).status, Some(crate::CompanyStatus::Active));
    }
    raw["odpis"]["dane"]["dzial6"]["likwidacja"][0]["likwidatorzy"] = json!([{}]);
    assert_ne!(common(&raw).status, Some(crate::CompanyStatus::Active));
}

#[test]
fn dated_bankruptcy_ending_order_requires_chronology_and_current_registration() {
    let mut raw = active_source();
    raw["odpis"]["dane"]["dzial6"] = json!({"postepowanieUpadlosciowe":[{"informacjaOOgloszeniuUpadlosci":{"data":"01.01.2015"},"opisZakonczeniaProcesuUpadlosci":{"opisSposobuZakonczeniaPostepowania":"POSTANOWIENIE SĄDU REJONOWEGO W SYNTHETIC X GUP 1/15","dataZakonczeniaPostepowania":"01.06.2015"}}]});
    assert_eq!(common(&raw).status, Some(crate::CompanyStatus::Active));
    assert_eq!(
        serde_json::to_value(common(&raw)).unwrap(),
        crate::test_support::documentation::example("pl", 9)
    );
    for bad in [
        Value::Null,
        json!("01.01.2014"),
        json!("01.01.2030"),
        json!("31.02.2015"),
    ] {
        let mut changed = raw.clone();
        changed["odpis"]["dane"]["dzial6"]["postepowanieUpadlosciowe"][0]["opisZakonczeniaProcesuUpadlosci"]
            ["dataZakonczeniaPostepowania"] = bad;
        assert_ne!(common(&changed).status, Some(crate::CompanyStatus::Active));
    }
    raw["odpis"]["dane"]["dzial1"]["danePodmiotu"]["nazwa"] = json!("Synthetic W UPADŁOŚCI");
    assert_ne!(common(&raw).status, Some(crate::CompanyStatus::Active));
}
