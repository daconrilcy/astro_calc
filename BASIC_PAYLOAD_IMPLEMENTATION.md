# E2E real stabilization - 2026-06-08

## Scope

Stabilized the real end-to-end scenarios present in `scripts/` for horoscope and natal reading flows.

## Horoscope period payloads

- Aligned the OpenAI provider response schema with the public response shape used by the service.
- Removed variant-specific fields from both `properties` and `required` in the provider schema.
- Pruned variant-specific fields during post-processing:
  - free responses keep free-only fields;
  - basic responses remove free-only and premium-only fields;
  - premium responses remove free-only fields and keep premium period fields.
- Re-applied word-count enforcement after text reprocessing and pruning so premium period payloads remain inside their configured bounds.
- Restored canonical request evidence keys when a provider returns empty `evidence_keys` arrays for period markers or domain sections.
- Treated blank evidence keys as absent so canonical request fallbacks are used.
- Re-ran period repetition normalization after provider text reprocessing and word-bound enforcement.

## Natal readings

- Removed repeated symbolic disclaimer boilerplate from generated chapter bodies during natal theme text reprocessing.
- Kept the legal disclaimer field untouched.
- Kept meaningful interpretation sentences when removing boilerplate fragments.
- Ignored repeated structural astrology anchors such as `milieu du ciel`, house labels, and planet-placement trigrams in the repeated-trigram quality counter, while keeping editorial phrase repetition blocking.
- Limited structural astrology trigram exemptions to numbered houses and real zodiac placements so non-placement repetitions such as `Mars en tension` still count.
- Kept useful chapter sentences such as `Cette lecture symbolique met en lumière...` and generic exploratory hypotheses.

## Tests

Added focused regression coverage in:

- `tests/horoscope_v1_tests.rs`
- `tests/text_reprocessing_application_tests.rs`
- `astral_llm/crates/astral_llm_application/src/text_trigrams.rs`

Validated with the real E2E scripts and supporting Rust test suites.
