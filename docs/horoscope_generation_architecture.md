# Horoscope Generation Architecture

## Objective

Keep the horoscope pipeline simple and predictable:

1. Calculator produces canonical natal/transit facts and evidence keys.
2. Public request may inject an `astrologer_persona`.
3. Writer request packages facts, persona and target output contract.
4. LLM writes the reading.
5. Post-LLM validation blocks only on contract and integration concerns.
6. Editorial heuristics remain audit-only, never blocking.

## Blocking Responsibilities

Blocking validation is limited to:

- JSON schema shape
- service identity and contract version
- required sections for the product tier
- allowed dates and period range
- canonical `evidence_keys`
- canonical `source_snapshot_keys`
- required top-level UX fields

## Non-Blocking Responsibilities

These concerns should not fail generation:

- repeated wording
- mechanical phrasing
- taxonomy wording in prose
- explicit astrological references in every paragraph
- personalization markers in fixed lexical forms
- editorial meta language that does not leak internal field names

Those checks can still feed `debug` or audit warnings when useful.

## File Responsibilities

- `astral_calculator/*`: compute canonical astro facts only
- `src/horoscope/*/public_request.rs`: validate public input and persona safety bounds
- `src/horoscope/*/calculation_request.rs`: build calculator request
- `src/horoscope/*/writer*.rs`: build LLM-facing payload and prompts
- `src/horoscope/*/validators.rs`: enforce output contract only
- `src/horoscope/*/quality.rs`: collect non-blocking editorial warnings
- `src/horoscope/*/response_repair.rs`: technical normalization, never prose rewriting for style

## Product Model

The product split should stay explicit and shallow:

- `free`: compact structured output
- `basic`: guided structured output
- `premium`: richer structured output with more sections

The difference between tiers should come from request shape and required sections, not from dense post-LLM lexical policing.
