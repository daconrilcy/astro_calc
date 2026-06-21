Statut: closed

Objet:
- review adversariale orientee frontieres pour la tranche 2026-06-21 qui
  deplace l'ownership de l'execution transitoire non natale vers
  `application/transient_chart.rs`.

Frontieres revues:
- `src/application/transient_chart.rs` est le seam partage unique pour
  reconstruire un `NatalChartInput` transitoire puis appeler le moteur
  ephemeris;
- `src/features/simplified/service.rs` et
  `src/features/horoscope/application/horoscope_service.rs` restent
  responsables de leur validation produit, de leurs filtres et de l'assemblage
  de sortie, sans dupliquer la mutation de `birth_datetime_utc` et
  `product_code`;
- aucun import `crate::features::*` ni `infra::db` n'entre dans le seam
  applicatif partage;
- la fermeture de la sous-vague exige un garde-fou de gouvernance et une preuve
  de comportement, pas seulement une deduplication textuelle.

Cycle 1 - Finding:
- F1: l'ownership etait bien recentre sur `application`, mais aucune defense
  repository-wide n'interdisait encore le retour des appels directs
  `.calculate_chart(` dans les services non natals. Risque: frontiere de
  responsabilite refermee puis repercee lors d'une evolution locale.

Correction:
- ajout d'un garde-fou de gouvernance qui impose le seam partage dans les
  services `simplified` et `horoscope`;
- ajout d'un test de comportement dedie au seam transitoire;
- mise a jour de la trace documentaire de la vague dans
  `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`.

Verification:
- `cargo test -p astral_calculator --test transient_chart_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `rg -n "calculate_transient_chart_facts|\\.calculate_chart\\(" astral_calculator/src/features/simplified/service.rs astral_calculator/src/features/horoscope/application/horoscope_service.rs`

Findings restants: Aucun

Aucun finding ouvert.
