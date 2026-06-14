# Natal Gateway Taxonomy

Cette refonte introduit une taxonomie produit alignee entre `natal_simplified` et `natal_full`.

Produits publics v2 :

- `natal_simplified_free`
- `natal_simplified_basic`
- `natal_simplified_premium`
- `natal_full_free`
- `natal_full_basic`
- `natal_full_premium`

Principes :

- la gateway publique orchestre calculator puis llm ;
- `astral_calculator` reste proprietaire des contrats et sorties de calcul ;
- `astral_llm` reste proprietaire des contrats et sorties de generation ;
- la difference `free/basic/premium` est portee par une policy typed ;
- la difference `simplified/full` est portee par la variante de calcul et le profil LLM cible.
