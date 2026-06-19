Statut: closed

Objet:
- re-review adversariale apres deplacement des tests `NatalReusePolicy` vers le repertoire racine `tests/`.

Finding:
- la boucle precedente laissait une couverture ciblee dans `src/`, hors du perimetre de tests autorise par les regles workspace.

Correction:
- suppression du module `#[cfg(test)]` dans `natal_calculation_service.rs`;
- ajout de `tests/natal_reuse_policy_tests.rs` avec doubles de ports et verification des 4 branches attendues.

Verification:
- aucun test ne reste sous `astral_calculator/src`;
- la couverture de reuse policy reste verte via le service public.

Findings restants: Aucun
