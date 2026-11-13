# Source maintenance instructions

Read [the source map](README.md) and the README in the affected domain before
editing. Root [agent instructions](../AGENTS.md) still apply.

- Trace identity and error handling before changing a parser. A decoder error must
  never become `false`, an empty successful record or a definitive miss.
- Keep provider modules private. Update explicit root exports and external consumer
  tests deliberately when changing the public API.
- Keep facade behavior, capability contracts and actual enabled adapters aligned.
  Discovery must remain offline and must not construct transport clients.
- Preserve source text and native status alongside normalization. Review the
  model conversion as well as the native decoder when changing a provider field.
- Keep unit/parser/transport tests next to their owner; use `tests/` for public
  contracts. Test endpoint overrides remain private and test-only.
- Update the owning module README when field mappings, routing or ownership change.
  Comments should explain evidence and invariants rather than restate statements.
- Module READMEs and this file are repository-only; do not broaden Cargo's archive
  include patterns to ship agent instructions or local evidence.
