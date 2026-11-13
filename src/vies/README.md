# VIES VAT domain

[identifiers.rs](identifiers.rs) owns accepted VAT prefixes and syntax;
[capabilities.rs](capabilities.rs) describes those jurisdictions. EL maps to GR,
XI to GB/GB-NIR; EU/IM and general GB VAT IDs are not supported. National business
identifiers remain separate. See the [consumer guide](../../docs/reference/vies.md).

[client.rs](client.rs) owns admission, spacing, deadlines and cooldowns across
clones. [transport.rs](transport.rs) exchanges bounded SOAP responses at the
configured European Commission `checkVatService` endpoint. There are no automatic
retries. Do not copy collector retry experiments into runtime behavior implicitly.

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
