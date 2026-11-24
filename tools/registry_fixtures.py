#!/usr/bin/env python3
"""Opt-in local registry replay, bounded refresh, review acceptance and coverage."""
import argparse
import collections
import copy
import datetime
import fcntl
import json
import os
from pathlib import Path
import subprocess
import tempfile
from zoneinfo import ZoneInfo

import registry_fixture_data as data

ROOT=Path(__file__).resolve().parents[1]
FIXTURES=ROOT/'tests/fixtures/registry'
LIVE={'IS':'live_registry_batch','BE':'live_nbb_batch','EE':'live_registry_batch','FI':'live_prh_batch','FR':'live_recherche_batch','IE':'live_registry_batch','LT':'live_registry_batch','LV':'live_registry_batch','PL':'live_registry_batch','AT':'live_firmenbuch_batch','CZ':'live_ares_batch','GR':'live_gemi_batch',
      'NO':'live_registry_batch','RO':'live_anaf_batch','SK':'live_registry_batch'}

def path_in_repo(path):
    path=Path(path).resolve()
    path.relative_to(ROOT)
    return path

def corpus_path(country):return FIXTURES/country/'corpus.json'

def cargo(args, test, env, online=False):
    merged=os.environ.copy()
    if args.docker_image:
        command=['docker','run','--rm','--pull=never']
        if not online:command+=['--network','none']
        command+=['--entrypoint','cargo','-v',str(ROOT)+':/work','-w','/work',
                  '-e','CARGO_HOME=/work/target/cargo-home','-e','CARGO_TARGET_DIR=/work/target/build']
        for k,v in env.items():command+=['-e',k+'='+v.replace(str(ROOT),'/work')]
        command+=[args.docker_image]
    else:
        command=['cargo'];merged.update(env)
    command+=['test','--locked','--offline','--lib',test,'--','--ignored','--exact']
    result=subprocess.run(command,cwd=ROOT,env=merged,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    if len(result.stdout)>20*1024*1024:raise RuntimeError('test output exceeded 20 MiB')
    if result.returncode:print(result.stdout[-10000:])
    return result.returncode

def audit(args,country,records):
    """Run the current private decoder/projector against every selected snapshot."""
    if not records:raise ValueError('no fixtures selected')
    (ROOT/'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='registry-replay-',dir=ROOT/'target') as folder:
        folder=Path(folder);manifest=[];order=[]
        for n,(key,record) in enumerate(records.items()):
            directory=folder/str(n);directory.mkdir();row=record['observation'];identifier=row.get('id',row.get('taxId'))
            (directory/'observations.jsonl').write_text(data.canonical(row)+'\n')
            (directory/'expectations.json').write_text(data.canonical({identifier:record['review']}))
            source_path=str(directory/'observations.jsonl')
            if args.docker_image:source_path=source_path.replace(str(ROOT),'/work')
            manifest.append({'country':country,'path':source_path});order.append(key)
        input_path=folder/'input.json';input_path.write_text(data.canonical(manifest));output=folder/'audit'
        test=country.lower()+'::probe::audit_retained_registry_responses'
        if cargo(args,test,{'REGISTRY_AUDIT_INPUT':str(input_path),'REGISTRY_AUDIT_OUTPUT':str(output)}):raise RuntimeError('Rust audit failed')
        result=json.loads((output/(country+'.json')).read_text())
        if len(result)!=len(order):raise ValueError('incomplete replay output')
        return {key:row for key,row in zip(order,result)}

def mismatches(record,actual):
    errors=[]
    if actual['validationStatus']!=record['expectedValidation']:errors.append('classification')
    if actual.get('differences'):errors.append('reviewed fields')
    if record['expectedValidation'] in ('verifiedPositive','unresolved') and not actual.get('reviewBound'):errors.append('unbound review')
    if record.get('expectedError') is not None and actual.get('error')!=record['expectedError']:errors.append('failure reason')
    return errors

def coverage(corpus):
    counts=collections.Counter();dates=[]
    for key in corpus['current'].values():
        r=corpus['records'][key];status=r['review']['fields'].get('/status')
        if r['category']=='resolved' and r['expectedValidation']=='verifiedPositive':counts[status]+=1
        else:counts[r['category']]+=1
        row=r['observation'];dates.append(row.get('checkedAt',row.get('completedAt',row.get('requestDate','unknown'))))
    known_dates=[stamp for stamp in dates if isinstance(stamp,str) and stamp and stamp!='unknown']
    return {'country':corpus['country'],'latestReviewed':dict(sorted(counts.items())),
            'activeTarget':corpus['activeTarget'],'activeShortfall':max(0,corpus['activeTarget']-counts['active']),
            'reviewedDateRange':[min(known_dates),max(known_dates)] if known_dates else None,
            'undatedObservations':len(dates)-len(known_dates),
            'observationsByRecordedDate':dict(sorted(collections.Counter(stamp[:10] for stamp in known_dates).items())),'replaySnapshots':len(corpus['replay']),
            'preservedReviewRevisions':len(corpus['records']),
            'meaning':'Latest accepted observations per identifier, not a live-status assertion; pending refreshes are not accepted coverage.'}

def replay(args):
    failed=False
    for country in args.country or data.COUNTRIES:
        corpus=data.read(corpus_path(country));records={k:corpus['records'][k] for k in corpus['replay']}
        results=audit(args,country,records)
        for key,record in records.items():
            errors=mismatches(record,results[key])
            if errors:print(country,key,','.join(errors));failed=True
        print(country,':',len(records),'historical/current snapshots replayed;',coverage(corpus)['latestReviewed'])
    return 1 if failed else 0

def refresh(args):
    country=args.country;corpus=data.read(corpus_path(country))
    ids=args.ids or list(corpus['current'])[:args.limit]
    if not ids or len(ids)>100 or len(ids)!=len(set(ids)):raise ValueError('select 1–100 distinct identifiers')
    if args.ids and args.limit is not None:raise ValueError('use --ids or --limit, not both')
    if not args.ids and args.limit is None:raise ValueError('explicit --ids or --limit required')
    directory=path_in_repo(args.run_dir);directory.relative_to(ROOT/'target');directory.mkdir(parents=True,exist_ok=False)
    manifest={'ids':ids}
    if country=='RO':manifest['requestDate']=datetime.datetime.now(ZoneInfo('Europe/Bucharest')).date().isoformat()
    (directory/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    exit_code=cargo(args,country.lower()+'::probe::'+LIVE[country],{country+'_RUN_DIR':str(directory)},online=True)
    return stage_captured(args, exit_code)

def stage_captured(args, exit_code=None):
    """Prepare retained collector output without making any live request."""
    country=args.country;corpus=data.read(corpus_path(country))
    directory=path_in_repo(args.run_dir);directory.relative_to(ROOT/'target')
    ids=json.loads((directory/'manifest.json').read_text())['ids']
    if not isinstance(ids,list) or not 1<=len(ids)<=100 or any(not isinstance(i,str) or not i for i in ids) or len(ids)!=len(set(ids)):
        raise ValueError('retained manifest must select 1–100 distinct identifiers')
    journal=directory/'observations.jsonl'
    if not journal.exists():raise RuntimeError('collector failed before producing a journal')
    rows=[json.loads(line) for line in journal.read_text().splitlines()];staged={};template={}
    if len({r.get('id',r.get('taxId')) for r in rows})!=len(rows):raise ValueError('duplicate observed identifiers')
    if not {r.get('id',r.get('taxId')) for r in rows}<=set(ids):raise ValueError('unexpected observed identifier')
    for original in rows:
        identifier=original.get('id',original.get('taxId'));old_key=corpus['current'].get(identifier);old=corpus['records'].get(old_key,{})
        row,review,provenance=data.prepare(country,original,old.get('review',{'fields':{'/taxId':identifier,'/status':None}}))
        record={'observation':row,'review':review,'provenance':provenance,
                'category':old.get('category','unresolved'),'expectedValidation':old.get('expectedValidation','unresolved'),
                'reviewer':'PENDING','rationale':'PENDING: inspect source and classify before accepting'}
        staged[identifier]=record
        template[identifier]={'fields':review['fields'],'category':record['category'],
                              'expectedValidation':record['expectedValidation'],'rationale':'PENDING'}
    results=audit(args,country,staged) if staged else {}
    report={identifier:{'previousReview':corpus['current'].get(identifier),
                       'changed':bool(mismatches(record,results[identifier])),
                       'actual':{k:v for k,v in results[identifier].items() if k!='sourcePath'}}
            for identifier,record in staged.items()}
    (directory/'staged.json').write_text(json.dumps({'country':country,'records':staged},ensure_ascii=False,indent=2)+'\n')
    (directory/'review-template.json').write_text(json.dumps(template,ensure_ascii=False,indent=2)+'\n')
    (directory/'changes.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    print('Saved pending refresh:',directory.relative_to(ROOT),'; observed',len(rows),'of',len(ids),'; collector exit',exit_code if exit_code is not None else 'unrecorded')
    print('Accepted fixtures and offline expectations have NOT changed. Review source bodies and edit a copy of review-template.json, then accept.')
    return 0 if exit_code in (None,0) and len(rows)==len(ids) else 2

def accept(args):
    # Serialize accepted writes so concurrent local reviews cannot discard history.
    (ROOT/'target').mkdir(exist_ok=True)
    with (ROOT/'target/registry-fixtures.lock').open('a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError('another fixture acceptance is running') from None
        return accept_locked(args)

def accept_locked(args):
    directory=path_in_repo(args.run_dir);staged=json.loads((directory/'staged.json').read_text());country=staged['country']
    corpus=data.read(corpus_path(country));before=copy.deepcopy(corpus)
    supplied=json.loads(Path(args.review_file).read_text());records={}
    if not supplied or not set(supplied)<=set(staged['records']):raise ValueError('review must select nonempty observed identifiers')
    if not args.reviewer.strip() or args.reviewer=='PENDING':raise ValueError('named reviewer required')
    for identifier,review in supplied.items():
        if not review.get('rationale','').strip() or review['rationale'].startswith('PENDING'):raise ValueError('source-bound rationale required')
        record=copy.deepcopy(staged['records'][identifier])
        if record['observation'].get('id',record['observation'].get('taxId')) != identifier:
            raise ValueError('staged identifier mismatch')
        record['review']['fields']=review['fields']
        record.update({k:review[k] for k in ('category','expectedValidation','rationale')})
        record['reviewer']=args.reviewer
        if review.get('expectedError') is not None:record['expectedError']=review['expectedError']
        if record['category']=='resolved' and (record['expectedValidation']!='verifiedPositive' or record['review']['fields'].get('/status') in (None,'unknown')):raise ValueError('resolved review requires explicit lifecycle')
        records[identifier]=record
    results=audit(args,country,records)
    for identifier,record in records.items():
        errors=mismatches(record,results[identifier])
        if errors:raise ValueError(identifier+': '+', '.join(errors))
        data.add(corpus,record)
    assert all(corpus['records'].get(k)==v for k,v in before['records'].items()),'history must remain immutable'
    data.write(corpus_path(country),corpus)
    print('Accepted',len(records),'reviewed revisions; previous responses/reviews retained. Offline replay now uses the accepted revisions.')
    print(json.dumps(coverage(corpus),ensure_ascii=False,indent=2))
    return 0

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--docker-image',help='Optional existing local Rust image; never pulled. Offline replay uses --network none.')
    sub=parser.add_subparsers(dest='command',required=True)
    p=sub.add_parser('replay');p.add_argument('--country',action='append',choices=data.COUNTRIES)
    p=sub.add_parser('coverage');p.add_argument('--country',action='append',choices=data.COUNTRIES);p.add_argument('--require-active',action='store_true',help='Fail if latest reviewed active count is below 200')
    p=sub.add_parser('refresh');p.add_argument('--country',required=True,choices=data.COUNTRIES);p.add_argument('--ids',nargs='+');p.add_argument('--limit',type=int,choices=range(1,101));p.add_argument('--run-dir',required=True)
    p=sub.add_parser('stage');p.add_argument('--country',required=True,choices=data.COUNTRIES);p.add_argument('--run-dir',required=True)
    p=sub.add_parser('accept');p.add_argument('--run-dir',required=True);p.add_argument('--review-file',required=True);p.add_argument('--reviewer',required=True)
    args=parser.parse_args()
    if os.environ.get('CI','').lower() not in ('','0','false'):parser.error('This workflow is local-only; do not execute it in CI.')
    if args.command=='replay':return replay(args)
    if args.command=='refresh':return refresh(args)
    if args.command=='stage':return stage_captured(args)
    if args.command=='accept':return accept(args)
    reports=[coverage(data.read(corpus_path(c))) for c in args.country or data.COUNTRIES];print(json.dumps(reports,ensure_ascii=False,indent=2))
    return int(args.require_active and any(r['activeShortfall'] for r in reports))

if __name__=='__main__':
    try:raise SystemExit(main())
    except (ValueError,AssertionError,KeyError,RuntimeError) as error:raise SystemExit(str(error))
