# REV-003 - Slot scoring and timeline

Status: closed.

Checks:

- Premium builds exactly 12 slot plans from `horoscope_time_slot_profiles`.
- Public timeline validation checks count, order and labels.
- Timeline entries must carry text, advice and evidence.

Findings:

- P1: `timeline[12]` could be interpreted as only "present".
  Correction: `validate_premium_response_evidence` checks exactly 12 entries
  in the same order and with the same public labels as the slot profile.

Result: P0/P1 fixed.
