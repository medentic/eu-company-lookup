# Austrian Firmenbuch

The native client uses anonymous JustizOnline public search followed by the
source-returned company detail route. It needs no account, key or cookies.
Search results must match the complete Firmenbuch number; a nearby number or
similar company name cannot select a record. Leading zeroes are preserved.

The current search covers active records. Empty or ambiguous search responses
are errors, not proof of absence. Paid documents and authenticated routes are
not requested. Native status, legal form and address text remain source values.

The shared facade dispatches Austrian company lookups to this adapter. Source
legal-form names populate the common legal form. Street, city and postal code
populate the structured address; country and full-address text stay null because
the response supplies neither. The source status is retained natively; `ACTIVE` maps to active and `DELETED`
to closed. Historical/other states remain unknown. VAT status remains null. Synthetic tests
cover identity binding, anonymous transport, source preservation and access
failure semantics. Collected observations and reviewed expectations stay local.

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

A successful lookup uses two HTTP requests: active-record search, then matched
detail. The default two-second interval applies between exchanges and after the
lookup; both requests share the 30-second deadline. This is client pacing, not a
verified JustizOnline quota. The active-only search cannot establish absence of a
historical or deleted entity. Access rejection must not trigger a paid-document
or authenticated-route fallback.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
