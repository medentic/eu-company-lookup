# Austrian Firmenbuch

The native client uses anonymous JustizOnline public search followed by the
source-returned company detail route. It needs no account, key or cookies.
Search results must match the complete Firmenbuch number; a nearby number or
similar company name cannot select a record. Leading zeroes are preserved.

The search explicitly requests `state=ALL`, covering registered, deleted and
historical entries according to the official [filter configuration](https://justizonline.gv.at/jop/service/fba/filterConfig).
The portal defaults to `ACTIVE`; that default can hide a deleted company. Empty or ambiguous search responses
are errors, not proof of absence. Paid documents and authenticated routes are
not requested. Native status, legal form and address text remain source values.

The shared facade dispatches Austrian company lookups to this adapter. Source
legal-form names populate the common legal form. Street, city and postal code
populate the structured address; country and full-address text stay null because
the response supplies neither. The source status is retained natively; `ACTIVE` maps to active and `DELETED`
to closed. Historical/other states remain unknown. VAT status remains null. Synthetic tests
cover identity binding, anonymous transport, source preservation and access
failure semantics. The reviewed sample meets the 200-distinct-ID national gate,
including at least 200 active companies and a deleted company, with complete
source-bound offline replay. Empty all-status searches remain unresolved and
are excluded from accepted coverage; they do not establish company closure.
Prepared reviewed observations and history may be committed under
`tests/fixtures/registry/` for local-only replay and refresh; original raw collections
and queues remain ignored. Fixtures stay outside the published crate.

Transport mechanics use the private [registry transport helpers](../registry_transport/mod.rs).
Each client instance owns its admission state, shared only with its clones;
country identifiers, accepted HTTP statuses, decoding and mapping stay here.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

## Access, pacing and recovery

A successful lookup uses two HTTP requests: all-status search, then matched
detail. Search requests up to 10 entries and rejects truncated result sets;
historical names can occupy several entries for one company. The default two-second interval applies between exchanges and after the
lookup; both requests share the 30-second deadline. This is client pacing, not a
verified JustizOnline quota. An empty all-status search still does not establish company closure.
When several exact matches include explicitly `HISTORIC` names, those historical
entries are excluded and the sole remaining record is selected. Multiple
nonhistorical matches remain errors; revision numbers are never ranked. Access rejection must not trigger a paid-document
or authenticated-route fallback.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
