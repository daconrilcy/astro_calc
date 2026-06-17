# REV-C - Acces DB Sync Et Orchestration

- Status: `closed`
- Decision: `fix now`

## Findings

- `engine/calculation_refs.rs` et `engine/projection/profiles.rs` avaient connu une regression temporaire vers des valeurs codees en dur.
- `astral_calculator/src/horoscope/builders.rs` conservait un pont DB synchrone legacy.

## Resolution

- Les resolutions de references moteur repassent par repositories injectes et async.
- Les builders horoscope sont passes en async et ne font plus de `run_blocking`.
