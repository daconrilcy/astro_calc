## 2026-06-15 - Correction modals tokens/couts horoscope daily/period

- Cause racine: les reponses gateway horoscope `daily` et `period` n'exposaient aucun `run_id` exploitable par l'UI de tests.
- Impact: l'UI ne pouvait pas charger l'audit `/api/llm/v1/runs/{run_id}`, donc les modals `Tokens / couts` restaient vides.
- Correction:
  - generation d'un `run_id` cote gateway pour les flux horoscope;
  - propagation de ce `run_id` vers l'API LLM via `debug_run_id`;
  - reutilisation de ce `run_id` par les writers horoscope LLM pour persister l'audit sous le meme identifiant;
  - exposition du `run_id` dans le champ top-level `debug` de la reponse gateway;
  - mise a jour de l'UI de tests pour lire aussi `debug.run_id`.
- Verification:
  - test Rust `gateway_horoscope_v2_tests` mis a jour pour verifier la propagation du `run_id`;
  - test HTML local de l'UI mis a jour pour verifier l'extraction de `debug.run_id`.
