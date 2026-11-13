# Changelog

## 0.1.0 — unreleased

- Return the compact company/address response with lossless VIES text and conservative address splitting reused from Medentic; include BSD attribution.

- Add typed VAT syntax validation for the 27 EU VIES prefixes and Northern Ireland's XI prefix.
- Add asynchronous VIES lookup and boolean convenience method, with source dates and optional original owner details.
- Add local capability discovery and explicit EL/GR and XI/GB-NIR mappings.
- Add bounded deadlines/bodies, shared admission limits, cooldowns, explicit source errors and synthetic provider tests.
- Keep national registries, reports and D-U-N-S outside this first release.
