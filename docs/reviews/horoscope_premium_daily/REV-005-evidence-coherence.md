# REV-005 - Evidence coherence

Status: closed.

Checks:

- Every Premium timeline entry cites evidence provided in the interpretation
  request.
- Invented evidence keys are rejected.
- Domain sections carry evidence keys.

Findings:

- P1: Premium response shape could cite keys outside the request evidence pack.
  Correction: Premium validation reuses recursive evidence collection and
  rejects invented keys with `HOROSCOPE_EVIDENCE_MISMATCH`.

Result: P0/P1 fixed.
