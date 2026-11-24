#!/usr/bin/env python3
"""Local-only prepared BULSTAT fallback-chain fixtures and historical replay."""
import argparse
import copy
import fcntl
import html
import json
import os
from pathlib import Path
import subprocess
import tempfile
from urllib.parse import parse_qs,urlsplit

import bg_fixtures as history
from registry_fixture_data import canonical,digest,pick,sha,applicable_history
from registry_fixture_html import Node,Tree

ROOT=history.ROOT
PATH=ROOT/'tests/fixtures/registry/BULSTAT/corpus.json'
HEADINGS=('РЕГИСТЪР БУЛСТАТ - СПРАВКИ','Справка за субект на БУЛСТАТ')

def page(source,detail):
    tree=Tree(source).root
    if not all(title in tree.text() for title in HEADINGS):raise ValueError('unrecognized BULSTAT page')
    selected=[]
    for node in tree.walk():
        if node.tag!='a':node.attrs={}
    for row in tree.walk('tr'):
        cells=[n for n in row.children if isinstance(n,Node) and n.tag in ('td','th')]
        if detail:
            if len(cells)!=4:continue
            label=cells[0].text().strip()
            if label not in ('Код по БУЛСТАТ','Нефизическо лице','Физическо лице','Състояние') and not (label=='' and cells[1].text().strip().startswith('седалище и адрес на управление:')):continue
            # History/action cells are outside the reviewed company facts.
            selected.append('<tr>'+cells[0].render()+cells[1].render()+'<td></td><td></td></tr>')
        else:
            if len(cells)!=2:continue
            identifier=cells[0].text().strip()
            if not identifier.isascii() or not identifier.isdigit() or len(identifier) not in (9,13):continue
            links=list(row.walk('a'));kept=[]
            for link in links:
                url=link.attrs.get('href','');parsed=urlsplit(url);query=parse_qs(parsed.query)
                if set(query)-{'__report','uic','__format','__locale','__designer','report_title','report_description'}:
                    raise ValueError('unreviewed BULSTAT link query fields')
                if parsed.netloc=='reports.bulstat.bg' and parsed.path=='/bulstat-ireports/preview' and query.get('uic')==[identifier] and query.get('__report')==['https://reports.bulstat.bg/bulstat-ireports/v_rpt1.rptdesign']:
                    kept.append('<a href="'+html.escape(url,quote=True)+'">'+html.escape(link.text())+'</a>')
            if not kept:raise ValueError('unreviewed search link')
            selected.append('<tr><td>'+''.join(kept)+'</td>'+cells[1].render()+'</tr>')
    if not selected:raise ValueError('no selected company rows')
    return ''.join('<h1>'+title+'</h1>' for title in HEADINGS)+'<table id="tab1">'+''.join(selected)+'</table>'

def prepare(row):
    if not history.valid_identifier(row['taxId']):raise ValueError('invalid business identifier')
    requests=row['requests']
    if len(requests)!=3:raise ValueError('incomplete fallback chain')
    prepared=pick(row,'schemaVersion taxId checkedAt');prepared['requests']=[]
    for n,request in enumerate(requests):
        out=pick(request,'provider contentType error');out['trace']=pick(request['trace'],'httpStatus retryAfter')
        body=bytes(request['body'])
        if n==0:
            if body:raise ValueError('not a definitive Commercial Register miss')
            out['body']=[]
        else:out['body']=list(page(body.decode('utf-8'),n==2).encode('utf-8'))
        prepared['requests'].append(out)
    return prepared

def read():
    c=json.loads(PATH.read_text());history.validate(c);return c

def save(c):
    history.validate(c);PATH.parent.mkdir(parents=True,exist_ok=True)
    tmp=PATH.with_suffix('.tmp');tmp.write_text(json.dumps(c,ensure_ascii=False,indent=2)+'\n');tmp.replace(PATH)

def import_legacy(directory):
    if PATH.exists():raise ValueError('refusing to replace history')
    reviews=json.loads((directory/'review.json').read_text())
    c={'schemaVersion':1,'country':'BG','provider':'Bulstat','source':'https://reports.bulstat.bg/bulstat-ireports/preview','activeTarget':200,'records':{},'history':{},'current':{}}
    for identifier,review in reviews.items():
        if not history.valid_identifier(identifier):raise ValueError('invalid business identifier')
        path=directory/'observations'/(identifier+'.json');original=path.read_bytes();row=json.loads(original)
        if sha(original.decode())!=review['observationSha256']:raise ValueError('unbound historical review')
        prepared=prepare(row);raw=canonical(prepared);bound=copy.deepcopy(review);bound['sourceSha256']=sha(raw);bound['observationSha256']=sha(raw)
        record={'taxId':identifier,'observedAt':row['checkedAt'],'category':'resolved','rawResponse':raw,'review':bound,
                'provenance':{'originalObservationSha256':sha(original.decode()),'originalReviewSha256':digest(review),'preparation':'bulstat-company-v1: exact company search/detail rows and definitive CR miss; removed unrelated sections and action/history cells'}}
        history.append(c,record)
    save(c)

def ingest(paths):
    with (ROOT/'target/bulstat-fixtures.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        return ingest_locked(paths)

def ingest_locked(paths):
    c=read()
    for path in paths:
        row=json.loads(path.read_text());identifier=row['taxId']
        if any(c['records'][key].get('provenance',{}).get('originalObservationSha256')==sha(path.read_text()) for key in c['history'].get(identifier,[])):continue
        previous=next((c['records'][key] for key in reversed(c['history'].get(identifier,[])) if c['records'][key].get('review')),None)
        record={'taxId':identifier,'observedAt':row['checkedAt'],'category':'unresolved','provenance':{'originalObservationSha256':sha(path.read_text())}}
        if row['outcome']=='bulstatFound':
            prepared=prepare(row);raw=canonical(prepared);record['rawResponse']=raw
            old=json.loads(previous['rawResponse']) if previous else None
            if old and old['requests']==prepared['requests']:
                review=copy.deepcopy(previous['review']);review['observationSha256']=sha(raw);review['sourceSha256']=sha(raw)
                record.update(category='resolved',review=review,rationale='Prepared official search/detail company facts and fallback chain unchanged; retained source-bound expectations.')
            else:record['rationale']='Prepared source changed or new; explicit source review pending.'
        elif (row.get('lookupError') or {}).get('isServiceUnavailable') is True or row['outcome'] in ('serviceUnavailable','rateLimited','timeout','providerError','coolingDown','accessDenied','httpError','busy'):record.update(category='accessFailure',error=row.get('error'),rationale='Access failure, not company absence or lifecycle change.')
        else:
            if row['outcome'] in ('unexpectedResponse','mismatchedResponse','responseTooLarge','invalidInput','lookupError'):record['category']='sourceFailure'
            record.update(outcome=row['outcome'],rationale='Unresolved collector outcome; review retained local observation before classifying.')
        history.append(c,record)
    save(c)

def replay(args):
    c=read();selected=applicable_history(c,lambda r:digest([r['taxId'],r['observedAt'],r.get('rawResponse')]))
    with tempfile.TemporaryDirectory(prefix='bulstat-fixture-replay-',dir=ROOT/'target') as temp:
        root=Path(temp)
        for index in range(max(map(len,selected.values()))):
            folder=root/str(index);(folder/'observations').mkdir(parents=True);reviews={}
            for identifier,keys in selected.items():
                if index>=len(keys):continue
                record=c['records'][keys[index]]
                if not record.get('review'):continue
                (folder/'observations'/(identifier+'.json')).write_text(record['rawResponse'])
                reviews[identifier]=record['review']
            if not reviews:continue
            (folder/'manifest.json').write_text(canonical({'ids':list(reviews)}));(folder/'review.json').write_text(canonical(reviews))
            command=['docker','run','--rm','--pull=never','--network','none','-v',str(ROOT)+':/work','-w','/work','-e','BULSTAT_RUN_DIR='+str(folder).replace(str(ROOT),'/work'),'--entrypoint',str(Path(args.binary).resolve()).replace(str(ROOT),'/work'),args.docker_image,'bg::bulstat_probe::replay_bulstat_batch','--ignored','--exact']
            result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            if result.returncode or '1 passed' not in result.stdout:raise ValueError(result.stdout[-10000:])
            print('BULSTAT history round',index,len(reviews),'snapshots passed')

def main():
    p=argparse.ArgumentParser(description=__doc__);s=p.add_subparsers(dest='command',required=True)
    q=s.add_parser('import-legacy');q.add_argument('--directory',type=Path,required=True)
    q=s.add_parser('ingest');q.add_argument('observations',nargs='+',type=Path)
    s.add_parser('coverage');q=s.add_parser('replay');q.add_argument('--docker-image',required=True);q.add_argument('--binary',required=True)
    args=p.parse_args()
    if os.environ.get('CI','').lower() not in ('','0','false'):p.error('local-only workflow; never run in CI')
    if args.command=='import-legacy':import_legacy(args.directory)
    elif args.command=='ingest':ingest(args.observations)
    elif args.command=='replay':replay(args)
    else:print(json.dumps(history.coverage(read()),indent=2))

if __name__=='__main__':main()
