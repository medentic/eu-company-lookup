"""Prepared Commercial Register sources for the existing scoped company reviews.

This deliberately excludes role/financial/document data outside those reviews and
redacts contact channels embedded in the address field. It is not a full extract.
"""
import copy
import json
import re
from registry_fixture_data import canonical, digest, pick, sha

CONTACT=r'(?:Телефон:|Факс:|Адрес на електронна поща:|Интернет страница:)'
FIELDS={'00010','00020','00030','00040','00050','00270'}

def address_text(text):
    return '\n'.join(line.rstrip() for line in (re.split(CONTACT,line,maxsplit=1)[0] for line in text.splitlines()) if line.strip())

def prepare(raw,review):
    value=json.loads(raw);out=pick(value,'deedStatus uic fullName hasInstructions');out['sections']=[]
    for section in value['sections']:
        if section['subUICType']!=1:continue
        s=pick(section,'subUICType');s['subDeeds']=[]
        for deed in section['subDeeds']:
            d=pick(deed,'subUIC subUICType subDeedIsClosed');d['groups']=[]
            for group in deed['groups']:
                fields=[]
                for field in group['fields']:
                    if field['fieldIdent'] not in FIELDS:continue
                    f=pick(field,'fieldIdent nameCode fieldOperation htmlData fieldEntryDate')
                    if f['fieldIdent']=='00050' and f.get('htmlData'):
                        f['htmlData']=re.sub(CONTACT+r'.*?(?=<br\b|</p\b|</div\b|$)','',f['htmlData'],flags=re.S|re.I)
                    fields.append(f)
                d['groups'].append({'fields':fields})
            s['subDeeds'].append(d)
        out['sections'].append(s)
    prepared=canonical(out);bound=copy.deepcopy(review)
    allowed={'taxId','source','companyName','companyEnglishName','legalForm','status','registryUrl','registryDetails.hasInstructions','registryDetails.identifierEntryDate','registryDetails.identifierEntryYear'}
    excluded={'registryDetails.managers','registryDetails.owners','registryDetails.capital','registryDetails.documents'}
    for path in list(bound['expected']):
        if path in excluded:del bound['expected'][path]
        elif path not in allowed and not path.startswith('address.'):raise ValueError('review exceeds prepared scope: '+path)
    bound['scope']='Prepared identity, company lifecycle and office fields only; contact channels and all role, financial and document assertions excluded. Original review hash retained.'
    key='address.fullAddressByRegistry'
    if isinstance(bound['expected'].get(key),str):bound['expected'][key]=address_text(bound['expected'][key])
    bound['sourceSha256']=sha(prepared)
    return prepared,bound,{'originalBodySha256':sha(raw),'originalReviewSha256':digest(review),
        'preparation':'commercial-company-v1: selected identity, lifecycle and office fields; unused roles/documents/financials removed; contact suffixes removed from source and reviewed full-address text; original hashes retained'}
