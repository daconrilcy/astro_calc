# REV-MA-001 Foundations

Statut : closed after corrections

Findings initiaux :

- P1 duplication des `HOROSCOPE_*` entre calculator et llm
- P1 absence de taxonomie produit typed pour `natal_simplified` / `natal_full`
- P2 absence de contrats communs factorises pour la nouvelle facade publique
- P2 orchestration publique non isolee dans une gateway dediee

Corrections appliquees :

- ajout de la crate `astral_contracts`
- centralisation des codes de service horoscope dans le registre partage
- introduction des types `ProductTier`, `NatalVariant`, `NatalProductCode`
- publication des premiers schemas `common/*` et `public/natal_*_v2`
- creation de la crate `astral_gateway`

Risques residuels :

- `horoscope` reste encore partiellement oriente legacy dans `astral_llm_application`
- le catalogue d'integration garde des champs historiques en parallele du typage ajoute
