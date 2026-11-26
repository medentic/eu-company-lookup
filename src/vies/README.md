# VIES VAT domain

[identifiers.rs](identifiers.rs) owns accepted VAT prefixes and syntax;
[capabilities.rs](capabilities.rs) describes those jurisdictions. EL maps to GR,
XI to GB/GB-NIR; EU/IM and general GB VAT IDs are not supported. National business
identifiers remain separate. The single country-format table is shared by local
validation and collector queue checks. Tests cover accepted and rejected syntax
for every prefix and rejection before either lookup method sends HTTP. See the
[consumer guide](../../docs/reference/vies.md).

[checksum.rs](checksum.rs) checks supported national checksums before lookup
admission, using private local arithmetic and conservative variant fallbacks. It does
not change `VatId` parsing, saved evidence or provider validity.
`VatId::validate_checksum()` exposes the same rules offline through the public
`VatChecksumResult` enum; request preflight rejects only `Failed`.

[client.rs](client.rs) owns admission, spacing, deadlines and cooldowns across
clones. [failure.rs](failure.rs) retains private failure facts through SOAP/HTTP
precedence, retry instructions and cooldown selection; the public error is built
only after those decisions. [transport.rs](transport.rs) exchanges bounded SOAP responses at the
configured European Commission `checkVatService` endpoint. There are no automatic
retries. Do not copy collector retry experiments into runtime behavior implicitly.

[pdf/mod.rs](pdf/mod.rs) owns requester-aware form submission and same-page printing.
[pdf/document.rs](pdf/document.rs) verifies the rendered fields and A4 dimensions
and applies the owner permissions before bytes leave the adapter.

## SOAP mapping

[soap.rs](soap.rs) validates namespaces, payload shape, required fields, duplicate
fields, echoed `countryCode`/`vatNumber`, date syntax and boolean representation.
Only a matched `checkVatResponse.valid` becomes public validity, including false.
SOAP faults such as member-state unavailability, input rejection and throttling
remain errors. Unknown faults never become a negative VAT observation.

`requestDate` retains provider date text; `name` supplies optional company text;
`address` retains decoded original text in the result envelope. Placeholder owner
text can be absent without making the VAT identifier invalid. Do not log arbitrary
SOAP bodies or submitted identifiers in errors.

[address/mod.rs](address/mod.rs) wraps shared location parsing and applies VAT
jurisdiction metadata. It preserves `fullAddressByVies` and uses the VAT prefix
for jurisdiction country; this does not independently verify physical location.
Country parsing rules belong in [locations](../locations/README.md).

[tests/soap.rs](tests/soap.rs), [tests/http.rs](tests/http.rs) and
[tests/identifiers.rs](tests/identifiers.rs) cover protocol and transport behavior;
[address/tests.rs](address/tests.rs) covers address behavior. Probe/reporting
modules are test-only. Collected corpora and queues remain local-only; follow
[testing and evidence](../../docs/development/testing.md) for reviewed replay.
