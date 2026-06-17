# REV-W01 plan

- Statut: closed
- Perimetre: extraction de la detection d'aspects vers `astral_calculator/src/astrology/aspects.rs`.
- Invariants:
  - la logique commune d'aspects vit sous `astrology/aspects.rs`;
  - `natal/aspects.rs` reste seulement un wrapper de compatibilite;
  - les appels internes nouveaux utilisent `crate::astrology::aspects`;
  - aucun changement de contrat JSON public.
- Verification:
  - `cargo test -p astral_calculator --test refactor_governance_tests`
  - `cargo test -p astral_calculator`
- Findings restants: Aucun.
