# Belgian NBB Consult

The native client calls the anonymous company lookup used by NBB Consult, with
English legal-form and legal-situation labels. No cookies, keys or accounts are
required. NBB identifies this company information as derived from CBE; the
provider label remains NBB, not a direct CBE verification claim.

The complete ten-digit enterprise number must match the response. Names and
structured addresses remain source observations. Reviewed legal-situation codes supply lifecycle; they do not establish VAT status. An HTTP failure or missing
record is not proof of absence from CBE. The shared facade dispatches Belgian company lookups to this adapter.
Street name and number form the delivery line; the source box number remains
in the second address field. Native components are also retained separately.
Country normalization uses only source-provided values. Original full-address
text, VAT fields and record URL remain null. Normal situation maps to active; reviewed terminal codes map to closed.
Documented liquidation, bankruptcy and restructuring codes map to distinct proceeding states. A code without a label
is retained as native status; absence of both remains null.
Collection and mapping verification remain in progress.

See NBB's [Consult terms](https://www.nbb.be/en/central-balance-sheet-office/consultation/consult/terms-use)
for the source and scope of the identity data. Collected records stay local-only.

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

One anonymous NBB request supplies CBE-derived company information. The default
two-second start interval is client policy; no numerical provider quota has been
established here. A missing NBB record or access failure is not a direct CBE miss.
Keep legal-situation codes ahead of translated display labels when reviewing new
states; unsupported situations remain unresolved rather than negative.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
