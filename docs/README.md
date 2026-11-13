# Documentation

Start with the [package README](../README.md) for installation, examples,
supported providers and the country-registry roadmap.

## Using the library

These references describe the public contract and are included in the crate:

- [VIES](reference/vies.md): identifiers, VAT results, address fields and failures.
- [Bulgarian registry](reference/bulgarian-registry.md): Commercial Register/BULSTAT
  lookup, fallback, company fields and coverage limitations.
- [Romanian registry](reference/romanian-registry.md): ANAF taxpayer details,
  fiscal status, dates and verification limitations.
- [API stability](reference/api-stability.md): Rust/JSON compatibility and address policy.

## Contributing

Read [CONTRIBUTING.md](../CONTRIBUTING.md) for setup, module layout and required checks.
Repository-only development guides cover:

- [Testing and evidence](development/testing.md): offline coverage, reviewed fixtures,
  provenance and interpretation of reports.
- [Live collectors](development/collectors.md): queue import, bounded collection,
  resume, replay and the test-only VIES retry experiment.

## Maintaining releases

The [release procedure](maintainers/releasing.md) covers candidate review,
archive verification and approval before publication.
[AGENTS.md](../AGENTS.md) contains instructions for automated contributors.

Source metadata belongs beside its dataset under `src/**/fixtures/`, not in this
folder. Local run reports and audit journals stay under ignored evidence paths;
they are not documentation or crate contents.
