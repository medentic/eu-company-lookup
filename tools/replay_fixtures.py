#!/usr/bin/env python3
"""Run every committed real-data replay locally, with container networking disabled."""
import argparse
import os
from pathlib import Path
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[1]

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--docker-image',required=True,help='Existing compatible local Rust image; never pulled')
    p.add_argument('--binary',required=True,help='Current library test executable from cargo test --locked --lib --no-run')
    args=p.parse_args()
    if os.environ.get('CI','').lower() not in ('','0','false'):p.error('real-data replay is local-only')
    binary=Path(args.binary).resolve();binary.relative_to(ROOT)
    if not binary.is_file():p.error('build the current library test executable first')
    commands=[[sys.executable,'tools/registry_fixtures.py','--docker-image',args.docker_image,'replay']]
    commands.extend([sys.executable,'tools/'+name,'replay','--docker-image',args.docker_image,'--binary',str(binary)] for name in ('vies_fixtures.py','bg_fixtures.py','bulstat_fixtures.py'))
    for command in commands:
        print('Running',Path(command[1]).name,flush=True)
        subprocess.run(command,cwd=ROOT,check=True)
    print('All committed reviewed fixture replays passed. This does not assert current live coverage or resolve deferred access failures.')

if __name__=='__main__':main()
