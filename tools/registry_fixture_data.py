"""Versioned, repository-only registry observations; no network or Cargo calls."""
import copy
import hashlib
import json
from pathlib import Path

COUNTRIES = ('AT', 'BE', 'CZ', 'EE', 'FI', 'FR', 'GR', 'IE', 'IS', 'LT', 'LV', 'NO', 'PL', 'RO', 'SK')
SOURCES = {
 'BE':'https://consult.cbso.nbb.be/api/rs-consult/companies',
 'EE':'https://ariregister.rik.ee/eng/api/autocomplete',
 'FI':'https://avoindata.prh.fi/opendata-ytj-api/v3/companies',
 'FR':'https://recherche-entreprises.api.gouv.fr/search',
 'IE':'https://opendata.cro.ie/api/3/action/datastore_search',
 'IS':'https://www.skatturinn.is/fyrirtaekjaskra/leit/kennitala',
 'LT':'https://get.data.gov.lt/datasets/gov/rc/jar/iregistruoti/JuridinisAsmuo',
 'LV':'https://data.gov.lv/dati/api/3/action/datastore_search',
 'PL':'https://api-krs.ms.gov.pl/api/krs/',

 'AT':'https://justizonline.gv.at/jop/service/fba',
 'CZ':'https://ares.gov.cz/ekonomicke-subjekty-v-be/rest',
 'GR':'https://publicity.businessportal.gr/',
 'NO':'https://data.brreg.no/enhetsregisteret/api/enheter',
 'RO':'https://webservicesp.anaf.ro/api/PlatitorTvaRest/v9/tva',
 'SK':'https://www.registeruz.sk/cruz-public/api',
}

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))

def digest(value):
    return hashlib.sha256(canonical(value).encode()).hexdigest()

def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()

def pick(value, keys):
    return {k: value[k] for k in keys.split() if k in value}

def nested(value, key, transform):
    if isinstance(value.get(key), dict):
        value[key] = transform(value[key])
    return value

# Keep only fields consumed by the adapters. Do not commit full register extracts,
# personal-role records, credentials, contact channels, financial reports or logs.
def payload(country, stage, value):
    from registry_fixture_projection import KEYS, project
    if country in KEYS:
        return project(country, value)

    if country == 'AT':
        if 'companies' in value:
            out = pick(value, 'numResults companies state')
            out['companies'] = [pick(x, 'id fnr status') for x in value['companies']]
            return out
        out = pick(value, 'id fnr name status domicile date legalForm address')
        nested(out, 'legalForm', lambda x: pick(x, 'abbr name'))
        return nested(out, 'address', lambda x: pick(x, 'street zipCode city'))
    if country == 'GR':
        info = pick(value['companyInfo'], 'errorCode errorMessage payload')
        if isinstance(info.get('payload'), dict) and 'company' in info['payload']:
            c = pick(info['payload']['company'], 'id name namei18n afm companyStatus legalType dateStart dateGemiRegistered company_address company_address_map isABranch isForeignBranch')
            nested(c, 'companyStatus', lambda x: pick(x, 'id status'))
            nested(c, 'legalType', lambda x: pick(x, 'id desc'))
            info['payload'] = {'company': c}
        return {'companyInfo': info}
    if country == 'NO':
        out = pick(value, 'respons_klasse underRekonstruksjonsforhandlingDato underUtenlandskInsolvensbehandlingDato paategninger organisasjonsnummer navn organisasjonsform forretningsadresse postadresse registrertIMvaregisteret registrertIForetaksregisteret konkurs underAvvikling underTvangsavviklingEllerTvangsopplosning stiftelsesdato slettedato registreringsdatoEnhetsregisteret registreringsdatoForetaksregisteret maalform hjemmeside naeringskode1 naeringskode2 naeringskode3')
        for key in ('organisasjonsform','naeringskode1','naeringskode2','naeringskode3'):
            nested(out, key, lambda x: pick(x, 'kode beskrivelse'))
        for key in ('forretningsadresse','postadresse'):
            nested(out, key, lambda x: pick(x, 'land landkode postnummer poststed kommune kommunenummer adresse'))
        if isinstance(out.get('paategninger'), list):
            out['paategninger'] = [pick(x, 'infotype tekst innfoertDato') for x in out['paategninger']]
        return out
    if country == 'SK':
        if stage == 'search': return pick(value, 'id existujeDalsieId')
        if stage == 'detail': return pick(value, 'id ico nazovUJ dic ulica mesto psc pravnaForma datumZalozenia datumZrusenia datumPoslednejUpravy zdrojDat sidlo kraj okres skNace velkostOrganizacie druhVlastnictva stav rpoLifecycle')
        if stage == 'rpoSearch': return {'results': [payload('SK', 'rpoDetail', x) for x in value['results']]}
        out = pick(value, 'id identifiers establishment termination legalStatuses')
        if isinstance(out.get('identifiers'), list): out['identifiers'] = [pick(x, 'value validFrom validTo') for x in out['identifiers']]
        if isinstance(out.get('legalStatuses'), list):
            out['legalStatuses'] = [nested(pick(x, 'value validFrom validTo'), 'value', lambda v: pick(v, 'value code codelistCode')) for x in out['legalStatuses']]
        return out
    if country == 'CZ':
        if stage == 'ros':
            return {'icoId':value.get('icoId'), 'zaznamy':[pick(x,'ico datumZaniku stavSubjektu') for x in value.get('zaznamy',[])]}
        out = pick(value, 'ico obchodniJmeno sidlo pravniForma pravniFormaRos dic dicSkDph datumVzniku datumZaniku datumAktualizace primarniZdroj seznamRegistraci kod popis')
        address = 'kodStatu nazevStatu kodKraje nazevKraje kodOkresu nazevOkresu kodObce nazevObce kodSpravnihoObvodu nazevSpravnihoObvodu kodMestskehoObvodu nazevMestskehoObvodu kodMestskeCastiObvodu nazevMestskeCastiObvodu kodUlice nazevUlice cisloDomovni doplnekAdresy kodCastiObce cisloOrientacni cisloOrientacniPismeno nazevCastiObce kodAdresnihoMista psc textovaAdresa cisloDoAdresy standardizaceAdresy pscTxt typCisloDomovni'
        nested(out, 'sidlo', lambda x: pick(x,address))
        return nested(out, 'seznamRegistraci', lambda x: {k:v for k,v in x.items() if k.startswith('stavZdroje')})
    if country == 'RO':
        out = pick(value, 'cod message found notFound')
        if isinstance(out.get('found'), list):
            records=[]
            for x in out['found']:
                r = pick(x, 'date_generale adresa_sediu_social adresa_domiciliu_fiscal inregistrare_scop_Tva inregistrare_RTVAI stare_inactiv inregistrare_SplitTVA')
                nested(r,'date_generale',lambda v:pick(v,'cui data denumire nrRegCom stare_inregistrare data_inregistrare forma_juridica cod_CAEN adresa statusRO_e_Factura'))
                for section,prefix in [('adresa_sediu_social','s'),('adresa_domiciliu_fiscal','d')]:
                    nested(r,section,lambda v:pick(v,' '.join(prefix+k for k in ['denumire_Strada','numar_Strada','denumire_Localitate','denumire_Judet','tara','cod_Postal','detalii_Adresa'])))
                for section,key in [('inregistrare_scop_Tva','scpTVA'),('inregistrare_RTVAI','statusTvaIncasare'),('stare_inactiv','statusInactivi'),('inregistrare_SplitTVA','statusSplitTVA')]:nested(r,section,lambda v:pick(v,key))
                records.append(r)
            out['found']=records
        return out
    raise ValueError('unsupported country')

def prepare(country, observation, review):
    """Project provider payloads; retain original digests, never copy live outputs."""
    original = copy.deepcopy(observation)
    row = pick(observation, 'id taxId checkedAt completedAt requestDate schemaVersion')
    failure=observation.get('lookupError') or {}
    if failure.get('isServiceUnavailable') is True:
        row['collectionFailure']=pick(failure.get('details') or {},'provider reason upstreamHttpStatus retryAfterSeconds')
    hashes = {}
    def exchange(raw, stage):
        if raw is None: return None
        out = pick(raw, 'httpStatus contentType retryAfter')
        if isinstance(raw.get('body'), str):
            hashes[stage] = sha(raw['body'])
            if country=='IS' and raw.get('httpStatus')==200:
                from registry_fixture_html import primary, overview, pdf_excerpt
                if 'encoding' in raw:out['encoding']=raw['encoding']
                if stage=='body':out['body']=primary(raw['body'])
                elif stage=='overview4':out['body']=pdf_excerpt(raw['body'])
                else:out['body']=overview(raw['body'],int(stage.removeprefix('overview')))
                return out
            # Non-JSON/error bodies are unnecessary for HTTP failure replay.
            try: body = json.loads(raw['body'])
            except ValueError: body = None
            if isinstance(body,dict) and (raw.get('httpStatus')==200 or country=='CZ' and raw.get('httpStatus')==404):
                out['body'] = canonical(payload(country,stage,body))
        return out
    if country=='AT':
        row['capture']={'responses':[exchange(x,str(n)) for n,x in enumerate(observation['capture'].get('responses',[]))]}
        row['bodySha256']=digest(row['capture'])
    elif country=='SK':
        row['capture']={k:exchange(v,k) for k,v in (observation.get('capture') or {}).items() if k in ('search','detail','rpoSearch','rpoDetail')}
        for key,stage in [('bodySha256','detail'),('searchSha256','search')]:
            b=(row['capture'].get(stage) or {}).get('body');row[key]=sha(b) if b is not None else None
        row['captureSha256']=digest(row['capture'])
    else:
        key='transport' if country=='RO' else 'capture'
        row[key]=exchange(observation.get(key),'body')
        if country=='CZ' and isinstance((observation.get(key) or {}).get('ros'),dict):row[key]['ros']=exchange(observation[key]['ros'],'ros')
        if country=='IS' and isinstance((observation.get(key) or {}).get('overviewResponses'),list):
            row[key]['overviewResponses']=[exchange(v,'overview'+str(n)) for n,v in enumerate(observation[key]['overviewResponses'])]
        if country=='LV' and isinstance((observation.get(key) or {}).get('lifecycle'),dict):
            row[key]['lifecycle']={k:exchange(v,k) for k,v in observation[key]['lifecycle'].items()}
        if country=='PL':
            def branch(raw, stage):
                out=exchange(raw,stage)
                if isinstance(raw,dict) and out is not None:
                    for nested_stage in ('association','full'):
                        if isinstance(raw.get(nested_stage),dict):
                            out[nested_stage]=branch(raw[nested_stage],stage+'/'+nested_stage)
                return out
            row[key]=branch(observation.get(key),'body')
        b=(row.get(key) or {}).get('body')
        if country=='PL' and b is None:
            cap=row.get(key) or {}
            b=(cap.get('full') or {}).get('body') or (cap.get('association') or {}).get('body') or ((cap.get('association') or {}).get('full') or {}).get('body')
        row['bodySha256']=sha(b) if b is not None else None
        if country in ('CZ','LV','PL','IS'):row['captureSha256']=digest(row['capture'])
    fields=copy.deepcopy(review.get('fields',{}))
    for field in ('/taxId','/companyName','/status'):
        if field in fields and fields[field] is None and field!='/status':raise ValueError('invalid identity review')
    reviewed={'fields':fields,'bodySha256':row['bodySha256']}
    if 'captureSha256' in row:reviewed['captureSha256']=row['captureSha256']
    if 'outcome' in review:reviewed['outcome']=review['outcome']
    return row,reviewed,{'originalObservationSha256':digest(original),'originalBodyHashes':hashes,'source':SOURCES[country],'preparation':('is-excerpt-v1: selected company HTML; session/cart tokens consistently redacted; PDF recreated from independently extracted identifier/date/status only; original hashes retained' if country=='IS' else 'adapter-fields-v1: unused fields removed; retained values unchanged; fixture hashes recomputed')}

def empty(country):
    return {'schemaVersion':1,'country':country,'activeTarget':200,'records':{},'history':{},'current':{},'replay':[]}

def add(corpus, record):
    """Append a review revision; old objects remain immutable. Caller decides review."""
    key=digest(record);row=record['observation'];identifier=row.get('id',row.get('taxId'))
    assert identifier and record['reviewer'] and record['rationale']
    corpus['records'].setdefault(key,record)
    history=corpus['history'].setdefault(identifier,[])
    if key not in history:history.append(key)
    # One applicable review per captured observation; superseded reviews stay in history.
    obs=digest(row)
    corpus['replay']=[k for k in corpus['replay'] if digest(corpus['records'][k]['observation'])!=obs]
    corpus['replay'].append(key)
    current=corpus['current'].get(identifier)
    stamp=lambda r:r['observation'].get('checkedAt',r['observation'].get('completedAt',r['observation'].get('requestDate','')))
    if current is None or stamp(record)>=stamp(corpus['records'][current]):corpus['current'][identifier]=key
    return key

def validate(corpus):
    assert corpus['schemaVersion']==1 and corpus['country'] in COUNTRIES and corpus['activeTarget']==200
    records=corpus['records'];assert records and corpus['current'] and corpus['replay']
    history=set()
    for identifier,keys in corpus['history'].items():
        assert len(keys)==len(set(keys))
        for key in keys:
            r=records[key];assert r['observation'].get('id',r['observation'].get('taxId'))==identifier;history.add(key)
    assert history==set(records)
    for key,r in records.items():
        assert key==digest(r),'modified immutable fixture record'
        assert r['reviewer'] and r['reviewer'] != 'PENDING' and r['rationale'] and not r['rationale'].startswith('PENDING')
        expected = {'resolved':'verifiedPositive','unresolved':'unresolved','accessFailure':'failed','sourceFailure':'failed','negative':'negativeObservation'}
        partial_access=(r['category']=='accessFailure' and r['expectedValidation']=='unresolved'
                        and r['observation'].get('collectionFailure',{}).get('reason')
                        and r['review']['fields'].get('/status') is None)
        assert r['expectedValidation'] == expected[r['category']] or partial_access
        if r['category'] == 'resolved':
            assert r['review']['fields'].get('/status') in ('active','closed','inactive','liquidation','bankruptcy','receivership',
                'liquidationAndReceivership','restructuring','pendingClosure','liquidationStayed','administration')
        assert r['category'] in ('resolved','unresolved','accessFailure','sourceFailure','negative')
        assert r['expectedValidation'] in ('verifiedPositive','unresolved','failed','negativeObservation')
        assert r['review']['bodySha256']==r['observation']['bodySha256']
        if 'captureSha256' in r['observation']:assert r['review']['captureSha256']==r['observation']['captureSha256']
    assert len(corpus['replay'])==len(set(corpus['replay']))
    for identifier,key in corpus['current'].items():assert key in corpus['history'][identifier] and key in corpus['replay']
    for key in corpus['replay']:assert key in records

def read(path):
    data=json.loads(Path(path).read_text());validate(data);return data

def write(path,data):
    validate(data);path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    tmp=path.with_suffix('.tmp');tmp.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n');tmp.replace(path)


def applicable_history(corpus, observation_identity):
    """Keep historical observations, using only their latest review revision."""
    selected={}
    for identifier,keys in corpus['history'].items():
        latest={observation_identity(corpus['records'][key]):key for key in keys}
        selected[identifier]=[key for key in keys if latest[observation_identity(corpus['records'][key])]==key]
    return selected
