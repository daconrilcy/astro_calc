# REV-003 - Projection And Response Shape

Findings reviewed:

- Public Free response exposes `summary`, `dominant_theme`, `key_days`, `advice`, `watch_summary`, `evidence_summary`, `quality`.
- Public Free response does not expose `daily_timeline`, `best_days`, `watch_days`, windows, `domain_sections` or `strategy`.
- `key_days` remains the JSON field; public front label is "Jours à retenir".

Status: fixed by compact writer and response evidence guard.

