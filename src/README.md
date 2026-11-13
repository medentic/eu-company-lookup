# Source architecture

This is one crate with private provider adapters and an explicit public surface in
[lib.rs](lib.rs). Read the [API contract](../docs/reference/api-stability.md) before
changing exported types or serialized fields. These module guides describe the
working source; they do not establish a published version or live coverage.

## Domain ownership

| Domain | Owner | Boundary |
| --- | --- | --- |
| Client construction and dispatch | [client.rs](client.rs) | Enabled adapters, named configuration and local disabled-operation errors. |
| Support discovery | [capabilities.rs](capabilities.rs) | Compiled support; the client filters it to enabled providers without network I/O. |
| Identity and result contracts | [model](model/README.md) | National identity, common company view, source details, errors and JSON. |
| VAT verification | [vies](vies/README.md) | VAT syntax, SOAP exchange, matched validity and VAT jurisdiction. |
| Bulgarian business registration | [bg](bg/README.md) | Commercial Register first, then BULSTAT only after a definitive miss. |
| Romanian fiscal registration | [ro](ro/README.md) | ANAF CUI/date matching and domestic fiscal observations. |
| Address interpretation | [locations](locations/README.md) | Offline extraction and country metadata; no registration decisions. |

## Request flow

A facade method checks whether its adapter is enabled, then the adapter validates
input, applies admission/deadline policy, exchanges bounded responses and checks
source identity before producing a result. `lookup_company` takes an already
validated `BusinessId` and converts the native BG/RO result into a common view.
Conversion performs no extra lookup and does not combine evidence from providers.

Country metadata, VAT jurisdiction, national identifier scheme, provider and
physical address country are different concepts. For example, EL is the VIES
prefix for GR; XI represents GB-NIR, not general GB VAT support. A country in the
capability catalog with no providers is metadata, not an implemented lookup.

## Where a change belongs

Change a source decoder for changed provider fields; change `locations` for a
shared address rule; change `model` for a consumer contract; change the facade and
catalog together for adapter configuration or operation availability. Provider
parsers and test endpoints remain private. External consumer contracts are tested
in [tests](../tests); provider unit tests remain beside the implementation.

For the maintenance workflow and verification matrix, see the
[agent handoff guide](../docs/maintainers/agent-handoff.md).
