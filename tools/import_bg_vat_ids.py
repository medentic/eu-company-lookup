#!/usr/bin/env python3
"""Extract a geographically balanced ID-only sample from the NRA VAT bulletin.

Usage: python3 tools/import_bg_vat_ids.py /path/to/bulletin.7z
Requires bsdtar. Does not query VIES or generate address expectations.
"""
import collections
import csv
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
archive = Path(sys.argv[1])
process = subprocess.Popen(
    ['bsdtar', '-xOf', str(archive), 'VATCompanies.csv'], stdout=subprocess.PIPE
)
groups = collections.defaultdict(set)
seen = set()
skipped = collections.Counter()
source_timestamp = None
for outer in csv.reader(io.TextIOWrapper(process.stdout, encoding='cp1251', newline=''), delimiter=';'):
    if not outer:
        continue
    if source_timestamp is None:
        source_timestamp = outer[0].split(',')[0]
    # Company names may contain commas/newlines; start at the quoted VAT column.
    match = re.search(r',"(BG[0-9]+)"', outer[0])
    if not match:
        continue
    row = next(csv.reader(io.StringIO(outer[0][match.start() + 1:], newline='')))
    if len(row) != 11:
        skipped['malformedRows'] += 1
        continue
    vat, region, municipality = row[:3]
    if not re.fullmatch(r'BG[0-9]{9}', vat):
        skipped['nonNineDigitIds'] += 1
        continue
    if not row[5] or row[7]:
        skipped['noRegistrationOrDeregistered'] += 1
        continue
    if not region:
        skipped['missingRegion'] += 1
        continue
    if vat in seen:
        skipped['duplicates'] += 1
        continue
    seen.add(vat)
    # The source uses СОФИЯ for both; municipality distinguishes the capital.
    if region == 'СОФИЯ':
        region = 'СОФИЯ-ГРАД' if municipality == 'СТОЛИЧНА' else 'СОФИЯ-ОБЛАСТ'
    groups[region].add(vat)
assert process.wait() == 0, 'archive extraction failed'
assert len(groups) == 28, f'Expected 28 regional groups, got {sorted(groups)}'
assert all(len(ids) >= 358 for ids in groups.values()), 'insufficient regional coverage'

# Stable hash ordering avoids favouring early registrations or source file order.
def rank(vat):
    return hashlib.sha256(('bg-vat-corpus-v1:' + vat).encode()).digest()

ranked = {region: sorted(ids, key=rank) for region, ids in groups.items()}
selected = []
counts = collections.Counter()
regions = sorted(groups)
# Interleave regions so even the first 100 IDs cover the whole country.
for index in range(358):
    for region in regions:
        if len(selected) == 10_000:
            break
        selected.append(ranked[region][index])
        counts[region] += 1
assert len(selected) == len(set(selected)) == 10_000
assert all(re.fullmatch(r'BG[0-9]{9}', vat) for vat in selected)
output = ROOT / 'src/vies/fixtures/ids/bg.json'
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(selected, indent=2) + '\n')
metadata = {
    'source': 'Bulgarian National Revenue Agency public VAT-registration bulletin',
    'sourcePage': 'https://nra.bg/wps/portal/nra/registers-i-spisuci/Spisuci/Spisuk-na-licata-registrirani-po-DDS',
    'archive': archive.name,
    'archiveSha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
    'sourceTimestamp': source_timestamp,
    'selection': '9-digit VAT IDs with registration date, no deregistration date, known region; stable SHA-256 ranking within region, round-robin across regions',
    'count': len(selected),
    'viesChecked': False,
    'regionalCounts': dict(sorted(counts.items())),
    'eligibleRegionalCounts': {region: len(groups[region]) for region in regions},
    'skippedRows': dict(skipped),
    'outputSha256': hashlib.sha256(output.read_bytes()).hexdigest(),
}
(ROOT / 'src/vies/fixtures/ids/bg.source.json').write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(metadata, ensure_ascii=False, indent=2))
