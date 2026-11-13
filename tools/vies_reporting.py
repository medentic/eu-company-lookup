"""Report statuses shared with Rust via fixtures/report-contract.json."""
from dataclasses import dataclass
from enum import Enum

class Status(str, Enum):
    PENDING = 'pending'
    PROVIDER_ERROR = 'providerError'
    NEEDS_REVIEW = 'needsReview'
    VALIDITY_CHANGED = 'validityChanged'
    SOURCE_CHANGED = 'sourceChanged'
    PARSER_MISMATCH = 'parserMismatch'
    AMBIGUOUS = 'ambiguous'
    PASSED = 'passed'

@dataclass(frozen=True)
class Facts:
    observed: bool = False
    response: bool = False
    expectation: bool = False
    validityChanged: bool = False
    sourceChanged: bool = False
    parserMismatch: bool = False
    ambiguous: bool = False


def classify(f: Facts) -> Status:
    if not f.observed: return Status.PENDING
    if not f.response: return Status.PROVIDER_ERROR
    if not f.expectation: return Status.NEEDS_REVIEW
    if f.validityChanged: return Status.VALIDITY_CHANGED
    if f.sourceChanged: return Status.SOURCE_CHANGED
    if f.parserMismatch: return Status.PARSER_MISMATCH
    if f.ambiguous: return Status.AMBIGUOUS
    return Status.PASSED
