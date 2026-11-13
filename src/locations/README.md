# Address interpretation and country metadata

This public module has no HTTP client and makes no registration decisions.
[mod.rs](mod.rs) exposes parsing, support discovery and Bulgarian delivery splitting.
[address.rs](address.rs) defines nullable components; [country.rs](country.rs)
defines country names/codes. Country metadata coverage exceeds lookup coverage.

| Implementation | Responsibility |
| --- | --- |
| [rules.rs](rules.rs) | Country-format extraction rules and their attribution. |
| [parser.rs](parser.rs) | Shared extraction using those rules. |
| [romanian.rs](romanian.rs) | Romanian plain-text parsing. |
| [normalize.rs](normalize.rs) | Working-copy presentation normalization. |
| [delivery.rs](delivery.rs) | Bulgarian delivery-line structure. |

The format hint chooses rules (GR aliases EL); it is not evidence of physical
country. Unsupported formats yield no guessed structured components. `Address`
itself is not a raw-text envelope: callers must keep source text. VIES and BG
registry adapters do this in their own result envelopes. ANAF structured fields
are mapped directly in the common model, not reparsed here.

When correcting a rule, preserve punctuation and source text in the envelope,
keep ambiguous values unknown and verify neighboring formats. For BG, review
street numbers versus postcodes/post boxes, locality labels, mixed-case names and
entrance/floor/apartment markers. Do not put provider validity or source-specific
HTML handling into these helpers.

Synthetic cases live beside these modules and in
[the VIES address tests](../vies/address/tests.rs) and [BG tests](../bg/tests.rs).
Use focused offline tests and available reviewed corpus replay; a parser supporting
a country format is not proof of complete national address coverage.

Estonian wide-space postal boundaries are normalized before extraction. Explicit
`linnaosa` district labels go to the second address line; `küla` settlements and
`vald` municipalities remain separate locality and municipality fields. The original
VIES address stays unchanged in its envelope.

Finnish `000xx` postal recipient labels remain secondary delivery text, with the
city unknown: these labels may identify organizations rather than settlements.

Croatian comma-delimited observations distinguish the settlement from the final
postal routing locality. Hungarian observations support a final four-digit
postcode and digit-free locality. Extra ambiguous components remain unparsed.

Maltese observations support a standalone alphanumeric postcode line followed by
locality; all preceding delivery lines retain their order. Short source postal
labels are preserved without padding or asserting current postal validity.

Swedish postal-only observations may omit the delivery line entirely; street
fields remain unknown while the supplied postcode and locality are retained.

For incomplete Portuguese observations, an explicitly labelled `AVENIDA` or
`RUA` line and a second non-numeric line can be retained as delivery text. City
and postcode remain unknown; external addresses are never substituted.
