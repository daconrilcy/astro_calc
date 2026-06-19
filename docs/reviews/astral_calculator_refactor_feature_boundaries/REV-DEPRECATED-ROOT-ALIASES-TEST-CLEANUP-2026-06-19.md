# Review feature boundaries - nettoyage tests aliases racine deprécies - 2026-06-19

Verification:

- les tests internes repo-wide utilisent les chemins canoniques et ne
  reintroduisent pas les wrappers historiques dans le code de production;
- la compatibilite publique restante est isolee dans un test dedie
  `tests/deprecated_root_alias_compat_tests.rs`;
- le garde-fou exclut uniquement les fichiers de gouvernance et de compatibilite
  explicitement assumes.

Conclusion:

- Aucun finding ouvert.
