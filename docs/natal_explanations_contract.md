# Contrat des explications neutres natal

Ce document precise le format public du sibling `explanations` ajoute a la reponse
natal V2, ainsi que la structure interne utilisee pour alimenter la lecture LLM.

## Objectif

Les explications ne sont pas une interpretation finale. Ce sont des phrases
courtes, neutres et factuelles qui decrivent les elements astrologiques majeurs
retenus avant la generation de la lecture.

Elles servent a deux usages:
- enrichir le prompt principal comme glossaire factuel;
- offrir une vue technique lisible dans l'UI de test et les outils internes.

## Cycle de production

1. la gateway appelle `/v1/internal/natal/explanations/prepare`;
2. le runtime choisit un top deterministe d'elements majeurs;
3. chaque combinaison est relue depuis PostgreSQL si elle existe deja;
4. en cas de cache miss, `gpt-5-mini` produit la phrase neutre;
5. le resultat est upserted en base;
6. la gateway publie `explanations` en sibling de `reading`.

## Structure publique

```json
{
  "status": "complete|partial|unavailable",
  "items": [
    {
      "fact_id": "placement:sun_taurus_house_x",
      "kind_code": "placement|angle|house|axis|aspect",
      "title": "Soleil en Taureau",
      "explanation": "Une identite stable, concrete et patiente.",
      "expression_primary": "Maison X - Carriere",
      "source": "cache|generated"
    }
  ],
  "missing_fact_ids": [],
  "errors": []
}
```

## Signification des champs

- `status`: `complete` quand tous les items retenus sont disponibles, `partial`
  quand une partie seulement a pu etre produite, `unavailable` quand la
  preparation a echoue sans bloquer la lecture.
- `items[]`: ordre de restitution stable, dans le meme ordre que la selection
  applicative.
- `fact_id`: cle stable de la combinaison expliquee.
- `kind_code`: famille logique de l'element.
- `title`: libelle lisible, court, public.
- `explanation`: phrase unique, neutre, descriptive.
- `expression_primary`: zone ou expression dominante associee a l'item.
- `source`: `cache` ou `generated`.
- `missing_fact_ids[]`: elements selectionnes mais non resolus.
- `errors[]`: erreurs de preparation non bloquantes.

## Regles de style

- une seule phrase par item;
- pas de prediction;
- pas de conseil;
- pas de diagnostic;
- pas de psychologie interpretative;
- vocabulaire simple et concret;
- si l'item est un axe, une maison dominante ou un aspect, decrire la relation
  ou la zone activee sans conclure sur la personne.

## Exemple

```json
{
  "status": "complete",
  "items": [
    {
      "fact_id": "placement:sun_taurus_house_x",
      "kind_code": "placement",
      "title": "Soleil en Taureau",
      "explanation": "Une identite stable, concrete et patiente, qui cherche a se construire dans la carriere et la place sociale.",
      "expression_primary": "Maison X - Carriere",
      "source": "cache"
    }
  ],
  "missing_fact_ids": [],
  "errors": []
}
```

## Lien avec la lecture

Le champ `explanations` est un sibling public de `reading` dans
`NatalReadingResponseV2`. Il ne modifie pas le contrat de la lecture finale et
ne doit pas etre confondu avec `reading.reading.chapters[]`.
