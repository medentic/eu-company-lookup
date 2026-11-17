# Greek GEMI public company details

The native client uses the anonymous company-details request made by the official
GEMI publicity site. Its JSON request contains the complete GEMI number, Greek
language selection and a null token. No keys, accounts or browser cookies are
required for this observed route. Provider failures remain errors.

GEMI registration numbers and AFM tax numbers are separate identities. This
adapter accepts twelve-digit GEMI numbers, preserving leading zeroes. The
response must match that exact identity. The source AFM stays native; it does
not establish VAT registration. Original registered-office text is kept apart
from the source's shorter map address. Native dates and codes are not interpreted
as cross-country lifecycle status. The shared facade dispatches Greek company lookup to this adapter. It retains
the complete official address and leaves its structured components unknown.
The source Latin-script name populates the foreign-name field. The verified
public company URL uses the exact GEMI number; VAT fields remain unknown.

The [official publicity service](https://publicity.businessportal.gr/) provides
the source. Live observations and source reviews stay local-only.

Transport mechanics use the private [registry transport helpers](../registry_transport/mod.rs).
Each client instance owns its admission state, shared only with its clones;
country identifiers, accepted HTTP statuses, decoding and mapping stay here.
