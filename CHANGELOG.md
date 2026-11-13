# Changelog

## 0.1.0 — unreleased

- Normalize spaces after Bulgarian block, entrance, floor and apartment markers in structured delivery fields.

- Present uppercase Bulgarian city and region names in title case, preserving mixed-case names, dotted abbreviations, known address acronyms and Roman numerals.

- Normalize a working copy before address extraction; repair Bulgarian marker boundaries and move block/entrance/floor/apartment details to address2 while preserving source text.

- Return the compact company/address response with lossless VIES text and conservative address splitting reused from Medentic; include BSD attribution.

- Add typed VAT syntax validation for the 27 EU VIES prefixes and Northern Ireland's XI prefix.
- Add asynchronous VIES lookup and boolean convenience method, with source dates and optional original owner details.
- Add local capability discovery and explicit EL/GR and XI/GB-NIR mappings.
- Add bounded deadlines/bodies, shared admission limits, cooldowns, explicit source errors and synthetic provider tests.
- Keep national registries, reports and D-U-N-S outside this first release.
