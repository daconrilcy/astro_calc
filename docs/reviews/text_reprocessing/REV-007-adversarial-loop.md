# REV-007 - Cycle adversarial longueur et titres

## Perimetre

Troisieme passe apres `REV-006`, focalisee sur les effets de bord des processors generiques sur les champs publics non corporels.

## Finding corrige

### P2 - Le controle de longueur pouvait modifier les titres

`SentenceAndLengthProcessor` utilisait le filtre de texte public general. Avec une limite minimale, un champ comme `title` pouvait recevoir une phrase fallback, ce qui degrade la structure editoriale.

Correction:

- Ajout de `is_length_controlled_path`.
- Le controle de longueur ne cible plus que les textes racine et champs de corps (`text`, `body`, `content`, `advice`, `watch_point`, `main`).
- Test ajoute: `text_reprocessing_length_processor_does_not_expand_titles`.

## Findings restants

Aucun P0/P1/P2/P3 ouvert apres ce cycle.

## Verification

- `cargo test -p astral_llm_application text_reprocessing`
