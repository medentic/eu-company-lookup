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
the response supplies neither. The search status is retained natively; no
normalized lifecycle or VAT status is inferred. Synthetic tests
cover identity binding, anonymous transport, source preservation and access
failure semantics. Collected observations and reviewed expectations stay local.

Transport mechanics use the private [registry transport helpers](../registry_transport/mod.rs).
Each client instance owns its admission state, shared only with its clones;
country identifiers, accepted HTTP statuses, decoding and mapping stay here.
