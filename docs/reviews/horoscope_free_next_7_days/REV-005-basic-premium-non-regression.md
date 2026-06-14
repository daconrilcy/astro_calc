# REV-005 - Basic Premium Non Regression

Findings reviewed:

- Basic period still keeps a 7-entry `daily_timeline`.
- Premium period still keeps windows, strategy and richer domain sections.
- Free period cannot be a Basic/Premium shape with fewer words.

Status: fixed; `cargo test -p astral_llm_api --test horoscope_tests` passed.
