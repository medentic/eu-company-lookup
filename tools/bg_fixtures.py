#!/usr/bin/env python3
"""Local-only prepared Commercial Register history and offline scoped replay."""
import argparse
import collections
import copy
import fcntl
import json
import os
from pathlib import Path
import subprocess
import tempfile

from registry_fixture_data import canonical,digest,sha,pick,applicable_history
from bg_fixture_preparation import prepare

ROOT=Path(__file__).resolve().parents[1]
PATH=ROOT/'tests/fixtures/registry/BG/corpus.json'

def valid_identifier(value):
    return isinstance(value,str) and value.isascii() and value.isdigit() and len(value) in (9,13)

def validate(corpus):
    assert corpus['schemaVersion']==1 and corpus['country']=='BG'
    seen=set()
    for identifier,keys in corpus['history'].items():
        assert valid_identifier(identifier),'invalid business identifier'
        assert len(keys)==len(set(keys))
        for key in keys:
            r=corpus['records'][key];assert digest(r)==key and r['taxId']==identifier
            assert r['category'] in ('resolved','unresolved','accessFailure','sourceFailure','negative')
            if r.get('review'):assert r['review']['sourceSha256']==sha(r['rawResponse'])
            seen.add(key)
        assert corpus['current'][identifier] in keys
    assert seen==set(corpus['records'])

def read():
    c=json.loads(PATH.read_text());validate(c);return c

def append(c,r):
    key=digest(r);identifier=r['taxId'];c['records'].setdefault(key,r)
    history=c['history'].setdefault(identifier,[])
    if key not in history:history.append(key)
    previous=c['current'].get(identifier)
    if previous is None or r['observedAt']>=c['records'][previous]['observedAt']:c['current'][identifier]=key

def stage(c,row):
    identifier=row['taxId']
    if not valid_identifier(identifier):raise ValueError('invalid business identifier')
    for key in reversed(c['history'].get(identifier,[])):
        existing=c['records'][key]
        if existing.get('collection') and existing.get('provenance',{}).get('originalObservationSha256')==digest(row):return copy.deepcopy(existing)
    previous=next((c['records'][key] for key in reversed(c['history'].get(identifier,[])) if c['records'][key].get('review')),None)
    record={'taxId':identifier,'observedAt':row.get('completedAt',row.get('checkedAt','')),
            'category':'unresolved','provenance':{'originalObservationSha256':digest(row)}}
    record['collection']=pick(row,'outcome errorKind serviceUnavailable unavailabilityReason')
    record['collection']['transport']=pick(row.get('transport') or {},'httpStatus retryAfter')
    if row['outcome']=='providerError':
        access=row.get('serviceUnavailable') is True or row.get('errorKind') in ('serviceUnavailable','rateLimited','coolingDown','accessDenied','httpError','busy')
        record.update(category='accessFailure' if access else 'sourceFailure',error=row.get('error'),rationale='Provider/access or source-protocol failure; not a company lifecycle or absence assertion.')
    elif row['outcome']=='notFoundInCommercialRegister':
        if row.get('rawResponse')!='' or row.get('transport',{}).get('httpStatus')!=200 or row.get('error') is not None:
            raise ValueError('negative observation lacks a definitive empty successful response')
        record['collection']['rawResponse']=''
        record.update(category='negative',rationale='Definitive miss is scoped only to the Commercial Register; it does not prove absence from BULSTAT.')
    elif row['outcome']=='parserError':
        record.update(category='sourceFailure',error=row.get('error'),rationale='Captured response could not be interpreted. Full source stays local for review; this is not an access blocker or company absence.')
        record['provenance']['originalBodySha256']=sha(row.get('rawResponse') or '')
    elif row.get('rawResponse'):
        old_review=previous['review'] if previous else {'expected':{'taxId':identifier}}
        prepared,review,provenance=prepare(row['rawResponse'],old_review)
        record['rawResponse']=prepared;record['provenance'].update(provenance)
        if previous and prepared==previous['rawResponse']:
            record.update(category=previous['category'],review=review,rationale='All retained official source fields match the preceding prepared source. Reused source-bound scoped expectations; preserved original hashes and history.')
        else:record['rationale']='Prepared source is new or changed; pending explicit source-bound review. No parser output used as expected data.'
    else:raise ValueError('unsupported observation outcome: '+row['outcome'])
    return record

def ingest(paths):
    with (ROOT/'target/bg-fixtures.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        c=read();before=copy.deepcopy(c['records']);counts=collections.Counter()
        for path in paths:
            row=json.loads(path.read_text());r=stage(c,row);append(c,r);counts[r['category']]+=1
        assert all(c['records'][k]==v for k,v in before.items());validate(c)
        tmp=PATH.with_suffix('.tmp');tmp.write_text(json.dumps(c,ensure_ascii=False,indent=2)+'\n');tmp.replace(PATH)
        print(dict(counts))

def coverage(c):
    counts=collections.Counter()
    for key in c['current'].values():
        r=c['records'][key]
        counts[r['review']['expected'].get('status','unknown') if r['category']=='resolved' else r['category']]+=1
    return {'country':'BG','provider':c['provider'],'latest':dict(counts),'activeTarget':200,'activeShortfall':max(0,200-counts['active']),'snapshots':len(c['records'])}

def validate_negative(record):
    capture=record.get('collection',{})
    assert capture.get('outcome')=='notFoundInCommercialRegister'
    assert capture.get('rawResponse')==''
    assert capture.get('transport',{}).get('httpStatus')==200
    assert not record.get('error')

def replay(args):
    c=read();selected=applicable_history(c,lambda r:digest([r['taxId'],r['observedAt'],r.get('rawResponse')]))
    negatives=0
    for keys in selected.values():
        for key in keys:
            record=c['records'][key]
            if record['category']=='negative':
                validate_negative(record);negatives+=1
    print('BG retained negative capture contracts verified:',negatives,'(register-specific empty HTTP 200; no company body to decode)')
    rounds=max(map(len,selected.values()))
    with tempfile.TemporaryDirectory(prefix='bg-fixture-replay-',dir=ROOT/'target') as temp:
        folder=Path(temp)
        for index in range(rounds):
            cases=[]
            for keys in selected.values():
                if index<len(keys):
                    r=c['records'][keys[index]]
                    if r.get('review'):cases.append({k:r[k] for k in ('taxId','rawResponse','review')})
            if not cases:continue
            (folder/'bg.json').write_text(canonical(cases))
            command=['docker','run','--rm','--pull=never','--network','none','-v',str(ROOT)+':/work','-v',str(folder)+':/work/src/bg/fixtures/commercial:ro','-w','/work','--entrypoint',str(Path(args.binary).resolve()).replace(str(ROOT),'/work'),args.docker_image,'bg::tests::reviewed_commercial_corpus','--ignored','--exact']
            result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            if result.returncode or '1 passed' not in result.stdout:raise ValueError(result.stdout[-10000:])
            print('BG scoped history round',index,':',len(cases),'snapshots passed')
    print(json.dumps(coverage(c),indent=2))

def main():
    parser=argparse.ArgumentParser(description=__doc__);sub=parser.add_subparsers(dest='command',required=True)
    p=sub.add_parser('ingest');p.add_argument('observations',nargs='+',type=Path)
    sub.add_parser('coverage');p=sub.add_parser('replay');p.add_argument('--docker-image',required=True);p.add_argument('--binary',required=True)
    args=parser.parse_args()
    if os.environ.get('CI','').lower() not in ('','0','false'):parser.error('local-only workflow; never run in CI')
    if args.command=='ingest':ingest(args.observations)
    elif args.command=='replay':replay(args)
    else:print(json.dumps(coverage(read()),indent=2))

if __name__=='__main__':main()
