# Bulgarian registry adapter

Start with the [consumer reference](../../docs/reference/bulgarian-registry.md).
[client.rs](client.rs) owns transport, admission, cooldown and fallback;
[identifiers.rs](identifiers.rs) owns nine/thirteen-digit business-code syntax.
[commercial.rs](commercial.rs) and [bulstat.rs](bulstat.rs) decode source records;
[html.rs](html.rs), [address.rs](address.rs) and [legal_form.rs](legal_form.rs)
interpret their text. All modules remain private.

## Provider routing and absence

The configured Commercial Register endpoint is on `portal.registryagency.bg`
under `/CR/api/Deeds/`. The request uses the complete identifier and a Sofia
end-of-day `entryDate`. A successful, empty HTTP 200 body with permitted content
type is the recognized miss. Whitespace, JSON null, HTTP 404, malformed records
and service failures do not trigger fallback.

Only that miss permits BULSTAT search on `reports.bulstat.bg/bulstat-ireports/`.
Search requires the expected report headings and exact code. Detail is fetched
using a constructed official URL and must match the same code and search name.
A recognized empty search report containing `Няма намерен резултат!` is a miss;
nonempty unmatched or ambiguous reports are inconclusive. Public `NotFound`
requires both misses. The whole flow shares one deadline.

## Commercial Register field map

The decoder requires a matching top-level `uic`, one main type-1 subdeed and
consistent field identifiers/name codes. Operations 1/3 are read; operation 2 is
erased. Duplicate fields, unexpected operations or mismatched codes are errors.

| Field identifier / name code | Output or purpose |
| --- | --- |
| `00010` / `CR_F_1_L` | Confirm UIC; `fieldEntryDate` becomes identifier entry date, not incorporation date. |
| `00020` / `CR_F_2_L` | `registryDetails.registeredName`; top-level `fullName` supplies company name. |
| `00030` / `CR_F_3_L` | Legal form via the explicit abbreviation map. |
| `00040` / `CR_F_4_L` | Registered foreign-script name; not a generated translation. |
| `00050` / `CR_F_5_L` | Registered office; retain decoded text before extraction. |
| `00060` / `CR_F_6_L` | Source business activity. |
| `00070` / `CR_F_7_L` | Managers. |
| `00230` / `CR_F_23_L` | Sole capital owners; not comprehensive beneficial ownership. |
| `00270` / `CR_F_27_L` | Explicit deletion marker used in lifecycle mapping. |
| `00310`, `00320` / `CR_F_31_L`, `CR_F_32_L` | Registered/paid-in capital; exact decimal strings and retained text. |
| `1001AJ` / `CR_F_1001_L` | Constitutive-document references from matching type-13 subdeeds; no downloads. |

A closed main subdeed, `deedStatus == 4`, or the recognized deletion text takes
precedence and maps to `Closed`. Otherwise status 1/2 maps to `Active`; other
codes map to `Unknown`. Retain the original numeric code and decimal-string native
status. Never infer a new code's meaning from its number.

## BULSTAT field map

| Source | Output |
| --- | --- |
| Exact search row name | `companyName`; names may contain commas. |
| `Код по БУЛСТАТ` | Identity check against the full requested code. |
| Exactly one `Нефизическо лице` or `Физическо лице` | Original `subjectText`; explicit legal-form suffix may be normalized. |
| `Състояние` | Original status; only `развиващ дейност` currently maps to `Active`. Other present values map to `Unknown`. |
| `седалище и адрес на управление:` | Address envelope; multiple address blocks retain raw text without guessing a structured address. |

Use [tests.rs](tests.rs) for synthetic decoder/transport regressions. The probe
modules are test-only evidence tools, not public APIs. Read the
[collector guide](../../docs/development/collectors.md) before invoking them.
