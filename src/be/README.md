# Belgian NBB Consult

The native client calls the anonymous company lookup used by NBB Consult, with
English legal-form and legal-situation labels. No cookies, keys or accounts are
required. NBB identifies this company information as derived from CBE; the
provider label remains NBB, not a direct CBE verification claim.

The complete ten-digit enterprise number must match the response. Names and
structured addresses remain source observations. Native legal-situation codes
do not establish normalized lifecycle or VAT status. An HTTP failure or missing
record is not proof of absence from CBE. The shared facade dispatches Belgian company lookups to this adapter.
Street name and number form the delivery line; the source box number remains
in the second address field. Native components are also retained separately.
Country normalization uses only source-provided values. Original full-address
text, VAT fields, normalized lifecycle and record URL remain null.
Collection and mapping verification remain in progress.

See NBB's [Consult terms](https://www.nbb.be/en/central-balance-sheet-office/consultation/consult/terms-use)
for the source and scope of the identity data. Collected records stay local-only.

Transport mechanics use the private [registry transport helpers](../registry_transport/mod.rs).
Each client instance owns its admission state, shared only with its clones;
country identifiers, accepted HTTP statuses, decoding and mapping stay here.
