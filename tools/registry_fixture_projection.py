"""Prepared JSON projections for additional national adapters.

These are input-field allowlists, not expected-result generators. Preserve native
values and null/missing distinctions; source-bound Rust replay verifies preparation.
"""
import copy

KEYS = {
    'BE': 'cbe name legalForm legalSituation streetName streetNumber boxNumber postalCode town country countryCode language languageRegime nisCode legalFormCode legalSituationCode legalSituationDate',
    'EE': 'status data companyId company_id regCode reg_code name historicalNames historical_names legalAddress legal_address zipCode zip_code legalForm legal_form url',
    'FI': 'totalResults companies value source registrationDate languageCode description name type version endDate descriptions register authority city municipalityCode street postCode postOffices postOfficeBox buildingNumber entrance apartmentNumber apartmentIdSuffix co country freeAddressLine typeCodeSet url businessId euId names mainBusinessLine website companyForms companySituations registeredEntries addresses tradeRegisterStatus status lastModified',
    'FR': 'results total_results siret adresse numero_voie indice_repetition type_voie libelle_voie complement_adresse distribution_speciale code_postal libelle_commune commune cedex libelle_cedex code_pays_etranger libelle_pays_etranger libelle_commune_etranger departement region etat_administratif date_creation date_fermeture statut_diffusion_etablissement est_siege siren nom_complet siren_conserve nom_raison_sociale sigle activite_principale categorie_entreprise date_mise_a_jour date_mise_a_jour_insee date_mise_a_jour_rne nature_juridique statut_diffusion siege',
    'IE': 'success result resource_id total records company_num company_name company_status company_type company_reg_date last_ar_date company_address_1 company_address_2 company_address_3 company_address_4 comp_dissolved_date nard last_accounts_date company_status_date eircode company_name_eff_date company_type_eff_date princ_object_code company_status_code company_type_code nace_v2_code',
    'LT': '_data _type _id _revision ja_kodas ja_pavadinimas pilnas_adresas adresas forma statusas reg_data isreg_data stat_data',
    'LV': 'success result resource_id total records filters lifecycle regcode name sepa regtype regtype_text type_text registered terminated closed address reregistration_term index addressid region city atvk id legal_entity_registration_number liquidation_type liquidation_type_text date_from suspension_code suspension_code_text proceeding_id debtor_registration_number proceeding_started_on proceeding_ended_on proceeding_form proceeding_type proceeding_subtype',
    'PL': 'odpis rodzaj naglowekA naglowekP dane dzial1 dzial6 danePodmiotuZagranicznego siedzibaIAdresPodmiotuZagranicznego rejestr numerKRS dataCzasOdpisu stanZDnia dataRejestracjiWKRS dataOstatniegoWpisu stanPozycji nip regon nazwaFormaPrawnaPrzedsiebiorcyZagranicznego innyRejestr nazwa formaPrawna identyfikatory kraj wojewodztwo powiat gmina miejscowosc ulica nrDomu nrLokalu kodPocztowy poczta siedzibaIAdresZakladuGlownego siedziba adres danePodmiotu siedzibaIAdres rozwiazanieUniewaznienie polaczeniePodzialPrzeksztalcenie zawieszenieWznowienie likwidacja postepowanieUpadlosciowe postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja otwarcieLikwidacji zakonczenieLikwidacji informacjaOOgloszeniuUpadlosci opisZakonczeniaProcesuUpadlosci otwarciePostepowaniaRestrukturyzacyjnegoNaprawczegoPrzymusowejRestrukturyzacjiUporzadkowanejLikwidacji zakonczeniePostepowaniaRestrukturyzacyjnegoNaprawczegoPrzymusowejRestrukturyzacjiUporzadkowanejLikwidacji data dataNadaniaKlauzuliWykonalnosci rodzajPostepowania opisSposobuZakonczeniaPostepowania dataZakonczeniaPostepowania dataZawieszenia dataWznowienia opisPodzialuPolaczeniaPrzeksztalcenia okreslenieOkolicznosci opisPolaczeniaPodzialuPrzeksztalcenia podmiotyPowstalePrzejmujace podmiotyPrzejmowane identyfikator krajNazwaRejestru numerWRejestrzeAlboEwidencji krs informacjaORozwiazaniuUniewaznieniu wpis numerWpisu opis dataWpisu dataUprawomocnienia nieprawomocny nrWpisuWprow nrWpisuWykr likwidatorzy liquidatorCount sposobProwadzeniaPostepowania sposobReprezentacjiPodmiotuPrzezLikwidatorow informacjaOUchyleniuUkladu uchylenieUkladu daneSyndyka nadzorcaZarzadcaReprezentujacyAdministratorZastepcaPelnomocnikZarzadcaNadzwyczajny',
}

def project(country, value):
    allowed = set(KEYS[country].split())
    def visit(item, path=()):
        if isinstance(item, list):
            if country == 'PL' and path and path[-1] in ('likwidatorzy', 'daneSyndyka', 'nadzorcaZarzadcaReprezentujacyAdministratorZastepcaPelnomocnikZarzadcaNadzwyczajny'):
                # The adapter consumes only length, never officers' personal data.
                return [{} for _ in item]
            return [visit(v, path) for v in item]
        if isinstance(item, dict):
            output = {}
            for key, v in item.items():
                if key in allowed:
                    output[key] = visit(v, path+(key,))
                elif country == 'PL' and key in ('nazwaSadu', 'nazwaSaduProwadzacegoRejestr', 'organWydajacy', 'sygnatura', 'sygnaturaSprawy', 'Identyfikator'):
                    continue  # Court attribution is not consumed by these typed facts.
                elif country == 'PL' and 'dzial6' in path:
                    # Unknown lifecycle evidence affects classification. Refuse to
                    # discard it or invent a safe replacement without source review.
                    raise ValueError('unreviewed Polish lifecycle field: '+'.'.join(path+(key,)))
            return output
        return copy.deepcopy(item)
    return visit(value)
