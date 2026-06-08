# REV-004 - Editorial Compactness

Findings reviewed:

- Free stays short and trend-oriented.
- `summary.text` is limited by guard to at most two explicit dates.
- `key_days` has at least one item and at most two.
- `evidence_summary` has one to three entries.
- `watch_summary` is mandatory and evidence is required when status is not `none`.

Status: fixed by Free-specific provider payload and evidence validations.

