Statut: closed

Objet:
- review adversariale orientee frontieres de couche pour la vague Phase 1 qui
  retire `test_catalog()` du code de production et transfere l'ownership du
  fixture catalog natal vers `tests/common/natal_catalog.rs`.

Frontieres revues:
- `src/features/natal/catalog.rs` ne garde qu'un re-export canonique du
  catalogue runtime et n'heberge plus de fixture builder de test;
- `src/features/natal/payload/build/mod.rs` exige un `BasicPayloadCatalog`
  explicite sur tous les builders publics et ne reintroduit pas de fallback;
- les tests racine dependants du fixture passent par `tests/common/` au lieu
  d'un chemin `src/features/natal/...`;
- la compatibilite publique residuelle reste isolee dans
  `tests/deprecated_root_alias_compat_tests.rs` et non dans le runtime.

Cycle 1 - Finding:
- F1: absence d'artefacts de review fermes specifiques a cette sous-vague de
  frontieres, alors meme qu'elle change une borne nette entre code production et
  tests/support.

Correction:
- ajout des artefacts de review dedies dans les deux repertoires de
  gouvernance;
- mise a jour des railguards pour figer l'invariant "fixtures natales sous
  `tests/common/`, jamais sous `src/`" et rappeler la double trace de review
  requise pour cette vague.

Verification:
- `rg -n "test_catalog\\(|features::natal::catalog::test_catalog|pub fn test_catalog" astral_calculator/src tests`
- `cargo test -p astral_calculator --test payload_tests`
- `cargo test -p astral_calculator --test runtime_tests`
- `cargo test -p astral_calculator --test signals_tests`
- `cargo test -p astral_calculator --test engine_contract_tests -- --test-threads=1`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
