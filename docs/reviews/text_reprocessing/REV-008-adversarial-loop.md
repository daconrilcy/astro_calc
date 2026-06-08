# REV-008 - Cycle adversarial conformite tests racine

## Perimetre

Passe de conformite avec les regles workspace apres les corrections fonctionnelles.

## Finding corrige

### P2 - Tests ajoutes dans les sources au lieu de `tests/`

Les tests du module avaient ete ajoutes en blocs `#[cfg(test)]` dans les sources des crates, alors que les regles workspace demandent les nouveaux tests dans le repertoire racine `tests/`.

Correction:

- Deplacement des tests application vers `tests/text_reprocessing_application_tests.rs`.
- Deplacement du test domain vers `tests/text_reprocessing_domain_tests.rs`.
- Ajout des targets `[[test]]` dans `astral_llm_application/Cargo.toml` et `astral_llm_domain/Cargo.toml`.
- Suppression des blocs `#[cfg(test)]` des sources `text_reprocessing.rs`.

## Findings restants

Aucun P0/P1/P2/P3 ouvert apres ce cycle.

## Verification

- `cargo test -p astral_llm_application text_reprocessing`
- `cargo test -p astral_llm_domain text_reprocessing`
