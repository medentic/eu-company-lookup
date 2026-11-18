"""Read-only VIES evidence loading and integrity checks; no collector dependency."""
from dataclasses import dataclass
from datetime import date
import hashlib
import json
from pathlib import Path
import re


class EvidenceFiles:
    """Read each file once per evaluation, using the same bytes for hashes and text."""
    def __init__(self):
        self._bytes = {}
        self._json = {}
        self._lines = {}

    def read(self, path):
        path = Path(path)
        if path not in self._bytes:
            self._bytes[path] = path.read_bytes()
        return self._bytes[path]

    def text(self, path):
        # Match Path.read_text's universal-newline handling.
        return self.read(path).decode('utf-8').replace('\r\n', '\n').replace('\r', '\n')

    def sha(self, path):
        return hashlib.sha256(self.read(path)).hexdigest()

    def json(self, path):
        if path not in self._json:
            self._json[path] = json.loads(self.text(path))
        return self._json[path]

    def lines(self, path):
        if path not in self._lines:
            self._lines[path] = self.text(path).splitlines()
        return self._lines[path]

    def journal(self, path):
        if path not in self._json:
            self._json[path] = [json.loads(line) for line in self.lines(path)]
        return self._json[path]


@dataclass(frozen=True)
class RunEvidence:
    ids: list
    rows: list
    fixtures: dict
    reviews: dict
    policy: dict
    replay: dict
    proof_ok: bool
    bindings: dict


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def expected_hash(expected):
    return hashlib.sha256(json.dumps(expected, sort_keys=True, ensure_ascii=False,
                                     separators=(',', ':')).encode()).hexdigest()


def review_valid(case, review, policy):
    try:
        date.fromisoformat(review['reviewedAt'])
        return bool(case and review['vatId'] == case['vatId']
                    and review['status'] in ('reviewed', 'ambiguous')
                    and type(review['reviewVersion']) is int and review['reviewVersion'] >= 1
                    and isinstance(review['note'], str) and review['note'].strip()
                    and review['normalizationVersion'] in policy['knownNormalizationVersions']
                    and review['expectedSha256'] == expected_hash(case['expected']))
    except (KeyError, TypeError, ValueError):
        return False


def fingerprint(root):
    files = sorted((root/'src').rglob('*.rs')) + [root/'Cargo.toml', root/'Cargo.lock']
    hashes = {str(p.relative_to(root)): sha(p) for p in files}
    return hashlib.sha256(json.dumps(hashes, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def snapshot(root, binary_manifest):
    info = json.loads(binary_manifest.read_text())
    if sha(root/info['binary']) != info['sha256']:
        raise ValueError('binary changed')
    # Older build manifests enumerate Rust sources only; replay fingerprints also
    # bind Cargo.toml/lock. Accept that retained format without rewriting evidence.
    sources = {str(p.relative_to(root)) for p in (root/'src').rglob('*.rs')}
    recorded = info['sourceHashes']
    if (not sources <= set(recorded)
            or set(recorded) - sources - {'Cargo.toml', 'Cargo.lock'}
            or any(sha(root/p) != digest for p, digest in recorded.items())):
        raise ValueError('source changed since build')
    return {'binarySha256': info['sha256'], 'sourceFingerprint': fingerprint(root)}


def proof_valid(folder, root, current, files=None):
    """Validate existing proof format; missing or stale evidence never verifies a case."""
    files = files or EvidenceFiles()
    try:
        proof = files.json(folder/'verification.json')
        return bool(current
                    and all(proof[k] == v for k, v in current.items())
                    and proof['observationSha256'] == files.sha(folder/'observations.jsonl')
                    and not list(folder.glob('batch-*.jsonl'))
                    and proof['replaySha256'] == files.sha(folder/'offline-replay.json')
                    and proof['logSha256'] == files.sha(root/proof['log'])
                    and {'vies::address::corpus::replay_probe_journals',
                         'vies::tests::probe::records::validate_saved_typed_journals'} <= set(proof['tests']))
    except (OSError, KeyError, TypeError, ValueError):
        return False


def source_bound(folder, root, row, case, files=None):
    """Retain the original observation binding or the appended source-review event."""
    files = files or EvidenceFiles()
    try:
        raw_hash = hashlib.sha256(row['attempts'][-1]['rawBody'].encode()).hexdigest()
        manifest = files.json(folder/'manifest.json')
        if manifest.get('kind') == 'derivedReplayView':
            binding = manifest['bindings'][row['vatId']]
            if binding['line'] < 1:
                return False
            line = files.lines(root/binding['sourceObservationFile'])[binding['line'] - 1]
            return (sha_bytes(line) == binding['lineSha256'] and json.loads(line) == row
                    and raw_hash == binding['rawBodySha256'])
        events = files.journal(folder/'review-journal.jsonl')
        event = next(e for e in reversed(events) if e['case']['vatId'] == row['vatId'])
        return event['case'] == case and event['sourceSha256'] == raw_hash
    except (OSError, KeyError, TypeError, ValueError, IndexError, StopIteration):
        return False


def sha_bytes(value):
    return hashlib.sha256(value.encode()).hexdigest()


# Read the private Rust format table so collector syntax cannot drift from the client.
def vat_formats(root):
    source = (root / 'src/vies/identifiers.rs').read_text()
    table = source.split('const FORMATS:', 1)[1].split('];', 1)[0]
    formats = dict(re.findall(r'\("([A-Z]{2})", "([^"\n]+)"\)', table))
    if len(formats) != 28:
        raise ValueError('cannot read the supported VIES format table')
    return formats


def load_rows(folder, files=None):
    files = files or EvidenceFiles()
    paths = sorted(folder.glob('batch-*.jsonl'))
    if (folder / 'observations.jsonl').exists():
        paths.append(folder / 'observations.jsonl')
    rows = []
    for path in paths:
        text = files.text(path)
        if text and not text.endswith('\n'):
            raise ValueError(f'partial journal tail: preserve and inspect {path}')
        rows.extend(files.journal(path))
    if len({r['vatId'] for r in rows}) != len(rows):
        raise ValueError('duplicate observed IDs')
    return rows


def load_evidence(root, folder, formats, *, current=None):
    files = EvidenceFiles()
    manifest_path = folder / 'manifest.json'
    ids = files.json(manifest_path)['ids'] if manifest_path.exists() else files.json(folder / 'ids.json')
    if not ids or len(set(ids)) != len(ids):
        raise ValueError('empty or duplicate report selection')
    rows = load_rows(folder, files)
    if any(r['vatId'] not in ids for r in rows):
        raise ValueError('journal contains IDs outside selection')
    country = ids[0][:2].lower()
    if country.upper() not in formats or any(i[:2].lower() != country for i in ids):
        raise ValueError('mixed or unsupported report country')
    country = {'el': 'gr', 'xi': 'gb'}.get(country, country)
    fixtures_path = root / f'src/vies/fixtures/countries/{country}.json'
    fixtures = {c['vatId']: c for c in files.json(fixtures_path)} if fixtures_path.exists() else {}
    reviews_path = root / f'src/vies/fixtures/countries/reviews/{country}.json'
    reviews = {r['vatId']: r for r in files.json(reviews_path)} if reviews_path.exists() else {}
    policy_path = root / 'src/vies/fixtures/review-policy.json'
    policy = files.json(policy_path) if policy_path.exists() else {}
    proof_ok = proof_valid(folder, root, current, files)
    replay_path = folder / 'offline-replay.json'
    replay = {r['vatId']: r['actual'] for r in files.json(replay_path)} if replay_path.exists() else {}
    if replay_path.exists() and len(replay) != len(files.json(replay_path)):
        raise ValueError('duplicate replay IDs')
    bindings = {row["vatId"]: source_bound(folder, root, row, fixtures.get(row["vatId"]), files)
                for row in rows} if proof_ok else {}
    return RunEvidence(ids, rows, fixtures, reviews, policy, replay, proof_ok, bindings)
