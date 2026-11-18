# Greek GEMI public company details

The native client uses the anonymous company-details request made by the official
GEMI publicity site. Its JSON request contains the complete GEMI number, Greek
language selection and a null token. No keys, accounts or browser cookies are
required for this observed route. Provider failures remain errors.

GEMI registration numbers and AFM tax numbers are separate identities. This
adapter accepts twelve-digit GEMI numbers, preserving leading zeroes. The
response must match that exact identity. The source AFM stays native; it does
not establish VAT registration. Original registered-office text is kept apart
from the source's shorter map address. Exact Greek labels ΕΝΕΡΓΗ and ΔΙΑΓΡΑΦΗ map to active and closed, including
accented spellings. Reviewed liquidation and bankruptcy labels map to their
distinct proceedings; other unsupported states remain unknown. A code without
a label is retained as native status and maps to unknown; absence of both is null. The shared facade dispatches Greek company lookup to this adapter. It retains
the complete official address and leaves its structured components unknown.
The source Latin-script name populates the foreign-name field. The verified
public company URL uses the exact GEMI number; VAT fields remain unknown.

The [official publicity service](https://publicity.businessportal.gr/) provides
the source. Live observations and source reviews stay local-only.

Transport mechanics use the private [registry transport helpers](../registry_transport/mod.rs).
Each client instance owns its admission state, shared only with its clones;
country identifiers, accepted HTTP statuses, decoding and mapping stay here.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

Exact Greek dissolution/liquidation and bankruptcy labels map to their distinct
proceedings, following the official GEMI guidance linked in the mapping audit.
No company-name substring or unverified numeric-code inference is used.

## Access, pacing and recovery

One company-details request uses the default two-second start interval. No
numerical quota has been established for this observed publicity route. Anonymous
access with a null token does not imply a contract for other GEMI APIs. Treat
access rejection, a changed envelope and an unmapped lifecycle as distinct errors.
An AFM tax number or a company-name fragment cannot substitute for GEMI identity
or explicit lifecycle text.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
