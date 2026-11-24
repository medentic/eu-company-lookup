#!/usr/bin/env python3
"""Local-only VIES historical replay and source-bound refresh staging."""
import argparse
import collections
import copy
import fcntl
import json
import os
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET

from registry_fixture_data import canonical, digest, applicable_history
from vies_evidence import expected_hash, review_valid

ROOT=Path(__file__).resolve().parents[1]
FIXTURES=ROOT/'tests/fixtures/registry/VIES'
PREFIXES='AT BE BG CY CZ DE DK EE EL ES FI FR HR HU IE IT LT LU LV MT NL PL PT RO SE SI SK XI'.split()

def country_file(prefix):return {'EL':'gr','XI':'gb'}.get(prefix,prefix.lower())

def empty(prefix):return {'schemaVersion':1,'prefix':prefix,'records':{},'history':{},'current':{}}

def append(corpus,record):
    key=digest(record);identifier=record['vatId'];corpus['records'].setdefault(key,record)
    history=corpus['history'].setdefault(identifier,[])
    if key not in history:history.append(key)
    current=corpus['current'].get(identifier)
    if current is None or record['observedAt']>=corpus['records'][current]['observedAt']:corpus['current'][identifier]=key

def validate(corpus):
    assert corpus['schemaVersion']==1 and corpus['prefix'] in PREFIXES
    seen=set()
    for identifier,keys in corpus['history'].items():
        assert identifier.startswith(corpus['prefix']) and len(keys)==len(set(keys))
        for key in keys:
            record=corpus['records'][key];assert record['vatId']==identifier and digest(record)==key
            assert record['category'] in ('reviewed','ambiguous','unresolved','accessFailure','sourceFailure')
            if record.get('case'):
                assert record['case']['vatId']==identifier
                assert record['review']['expectedSha256']==expected_hash(record['case']['expected'])
            seen.add(key)
        assert corpus['current'][identifier] in keys
    assert seen==set(corpus['records'])

def read(prefix):
    value=json.loads((FIXTURES/prefix/'corpus.json').read_text());validate(value);return value

def write(corpus):
    validate(corpus);path=FIXTURES/corpus['prefix']/'corpus.json';path.parent.mkdir(parents=True,exist_ok=True)
    temp=path.with_suffix('.tmp');temp.write_text(json.dumps(corpus,ensure_ascii=False,indent=2)+'\n');temp.replace(path)

def import_legacy(root):
    policy=json.loads((ROOT/'src/vies/fixtures/review-policy.json').read_text())
    for prefix in PREFIXES:
        if (FIXTURES/prefix/'corpus.json').exists():raise ValueError('existing corpus; import must never replace history')
    for prefix in PREFIXES:
        path=root/(country_file(prefix)+'.json');cases=json.loads(path.read_text())
        reviews={r['vatId']:r for r in json.loads((root/'reviews'/path.name).read_text())}
        corpus=empty(prefix)
        for case in cases:
            review=reviews[case['vatId']]
            if not review_valid(case,review,policy):raise ValueError('unbound legacy review: '+case['vatId'])
            append(corpus,{'vatId':case['vatId'],'observedAt':'','category':review['status'],
                'case':case,'review':review,'provenance':{'source':'VIES archived source fields','originalCaseSha256':digest(case),'originalReviewSha256':digest(review)},
                'rationale':'Imported existing source-bound reviewed address fixture unchanged; original observation timestamp not asserted.'})
        write(corpus)

def source_from_soap(row):
    """Independently read matched official fields, never generate address expectations."""
    attempt=row['attempts'][-1]
    if not attempt.get('response'):return None
    if not 200<=attempt.get('httpStatus',0)<300:raise ValueError('success without successful HTTP')
    root=ET.fromstring(attempt['rawBody']);ns='{urn:ec.europa.eu:taxud:vies:services:checkVat:types}'
    soap='{http://schemas.xmlsoap.org/soap/envelope/}'
    bodies=root.findall(soap+'Body')
    if root.tag!=soap+'Envelope' or len(bodies)!=1 or len(bodies[0])!=1:
        raise ValueError('unexpected SOAP envelope/body')
    matches=bodies[0].findall(ns+'checkVatResponse')
    if len(matches)!=1:raise ValueError('missing or ambiguous SOAP response')
    def field(name,optional=False):
        nodes=matches[0].findall(ns+name)
        if not nodes and optional:return None
        if len(nodes)!=1:raise ValueError('missing or ambiguous SOAP '+name)
        return nodes[0].text or ''
    if field('countryCode')+field('vatNumber')!=row['vatId']:raise ValueError('SOAP identifier mismatch')
    valid=field('valid').strip()
    if valid not in ('true','false','1','0'):raise ValueError('unexpected SOAP validity')
    # Empty present address and absent address differ. Meaningful name text stays exact;
    # only all-whitespace/hyphen owner placeholders are treated as missing.
    clean=lambda value:None if value is None or all(c.isspace() or c=='-' for c in value) else value
    source={'valid':valid in ('true','1'),'company':clean(field('name',True)),'address':field('address',True)}
    response=attempt['response']
    if response['vatId']!=row['vatId'] or response['valid']!=source['valid'] or response['company']!=source['company'] or response['address']['fullAddressByVies']!=source['address']:
        raise ValueError('captured SOAP and decoded source disagree')
    return source

def stage_row(corpus,row):
    original_hash=digest(row)
    for key in reversed(corpus['history'].get(row['vatId'],[])):
        existing=corpus['records'][key]
        if existing.get('provenance',{}).get('originalObservationSha256')==original_hash:
            return copy.deepcopy(existing)
    original=copy.deepcopy(row);source=source_from_soap(row)
    record={'vatId':row['vatId'],'observedAt':row['checkedAt'],'observation':original,
            'provenance':{'source':'https://ec.europa.eu/taxation_customs/vies/services/checkVatService','originalObservationSha256':digest(row)}}
    if source is None:
        error=row['attempts'][-1].get('error') or {}
        code=error.get('code') if isinstance(error,dict) else None
        access=code in ('transportTimeout','transportError','bodyReadError','totalDeadlineExceeded','httpError','globalRateLimit','memberStateRateLimit','serviceUnavailable','memberStateUnavailable','providerTimeout')
        record.update(category='accessFailure' if access else 'sourceFailure',rationale='Captured provider/transport or source-protocol error; no matched VAT validity or company lifecycle assertion.')
        return record
    previous=corpus['records'].get(corpus['current'].get(row['vatId']))
    # A prior access failure does not erase the last reviewed source expectation.
    if previous is None or not previous.get('case'):
        previous=next((corpus['records'][key] for key in reversed(corpus['history'].get(row['vatId'],[])) if corpus['records'][key].get('case')),None)
    if previous and previous['case']['valid']==source['valid'] and previous['case']['viesOutput']=={'company':source['company'],'address':source['address']}:
        record.update(category=previous['category'],case=copy.deepcopy(previous['case']),review=copy.deepcopy(previous['review']),
            rationale='Independently checked matched SOAP country, number, validity, company and original address against the preceding reviewed source. Retained unchanged address expectations and ambiguity classification; original history preserved.')
    else:
        record.update(category='unresolved',source=source,rationale='Matched SOAP source is new or changed. Address interpretation requires source-bound review; no expected fields copied from parser output.')
    return record

def ingest(paths):
    (ROOT/'target').mkdir(exist_ok=True)
    with (ROOT/'target/vies-fixtures.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
        return ingest_locked(paths)

def ingest_locked(paths):
    corpora={p:read(p) for p in PREFIXES};counts=collections.Counter()
    for path in paths:
        for line in path.read_text().splitlines():
            row=json.loads(line);corpus=corpora[row['vatId'][:2]];record=stage_row(corpus,row)
            append(corpus,record);counts[record['category']]+=1
    for corpus in corpora.values():write(corpus)
    print(dict(counts))

def coverage(corpus):
    counts=collections.Counter();matched=collections.Counter();dates=collections.Counter()
    for key in corpus['current'].values():
        record=corpus['records'][key]
        if record.get('observedAt'):dates[record['observedAt'][:10]]+=1
        observation=record.get('observation')
        if observation and observation['attempts'][-1].get('response'):
            matched['valid' if observation['attempts'][-1]['response']['valid'] else 'invalid']+=1
        if record['category']=='reviewed':counts['valid' if record['case']['valid'] else 'invalid']+=1
        else:counts[record['category']]+=1
    return {'prefix':corpus['prefix'],'latest':dict(counts),'validTarget':200,'invalidTarget':1,
            'historicalSnapshots':len(corpus['records']),
            'matchedValidityWithRetainedSoap':dict(matched),'observationsByRecordedDate':dict(sorted(dates.items())),
            'note':'VIES validity is separate from national active lifecycle. Undated legacy observations do not establish fresh verification.'}

def replay(args):
    corpora=[read(p) for p in args.prefix or PREFIXES]
    def identity(record):
        case=record.get('case') or {}
        return digest(record.get('observation') or {'vatId':record['vatId'],'valid':case.get('valid'),'source':case.get('viesOutput')})
    selected={c['prefix']:applicable_history(c,identity) for c in corpora}
    rounds=max(len(keys) for histories in selected.values() for keys in histories.values())
    with tempfile.TemporaryDirectory(prefix='vies-fixture-replay-',dir=ROOT/'target') as folder:
        folder=Path(folder);journals=[]
        for index in range(rounds):
            directory=folder/str(index);(directory/'reviews').mkdir(parents=True);count=0
            for corpus in corpora:
                cases=[];reviews=[]
                for keys in selected[corpus['prefix']].values():
                    if index>=len(keys):continue
                    record=corpus['records'][keys[index]]
                    if record.get('observation'):journals.append(record['observation'])
                    if record.get('case'):cases.append(record['case']);reviews.append(record['review'])
                if cases:
                    name=country_file(corpus['prefix'])+'.json'
                    (directory/name).write_text(canonical(cases));(directory/'reviews'/name).write_text(canonical(reviews));count+=len(cases)
            if count:
                run(args,'vies::address::corpus::saved_corpus',directory,{})
                print('Address replay history round',index,':',count,'reviewed/ambiguous snapshots')
        if journals:
            (folder/'observations.jsonl').write_text(''.join(canonical(r)+'\n' for r in journals))
            run(args,'vies::tests::probe::records::validate_saved_typed_journals',None,{'VIES_REPLAY_DIR':str(folder).replace(str(ROOT),'/work')})
            print('Typed SOAP/transport replay:',len(journals),'snapshots')

def run(args,test,corpus,env):
    command=['docker','run','--rm','--pull=never','--network','none','-v',str(ROOT)+':/work','-w','/work']
    if corpus:command+=['-v',str(corpus)+':/work/src/vies/fixtures/countries:ro']
    for k,v in env.items():command+=['-e',k+'='+v]
    command+=['--entrypoint',str(Path(args.binary).resolve()).replace(str(ROOT),'/work'),args.docker_image,test,'--ignored','--exact']
    result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    if result.returncode or '1 passed' not in result.stdout:
        raise ValueError('replay failed: '+result.stdout[-10000:])

def main():
    parser=argparse.ArgumentParser(description=__doc__);sub=parser.add_subparsers(dest='command',required=True)
    p=sub.add_parser('import-legacy');p.add_argument('--directory',type=Path,required=True)
    p=sub.add_parser('ingest');p.add_argument('journals',type=Path,nargs='+')
    p=sub.add_parser('coverage');p=sub.add_parser('replay');p.add_argument('--prefix',action='append',choices=PREFIXES);p.add_argument('--docker-image',required=True);p.add_argument('--binary',required=True)
    args=parser.parse_args()
    if os.environ.get('CI','').lower() not in ('','0','false'):parser.error('local-only workflow; never run in CI')
    if args.command=='import-legacy':import_legacy(args.directory)
    elif args.command=='ingest':ingest(args.journals)
    elif args.command=='replay':replay(args)
    else:print(json.dumps([coverage(read(p)) for p in PREFIXES],indent=2))

if __name__=='__main__':main()
