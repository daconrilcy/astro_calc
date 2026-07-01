Repondez exclusivement en JSON conforme au schema natal_reading_v1.
Respectez les limites de mots par chapitre si indiquees.
Incluez astro_basis pour la tracabilite interpretative.
Pour chaque chapitre, renseignez `summary_sentence` avec une seule phrase autonome qui resume le chapitre.
interpretive_role : uniquement core, supporting, nuance ou domain_score (snake_case).
Dans le corps de chaque chapitre (`body`), evitez toute repetition de formulations :
privilegiez un style fluide, avec des formulations distinctes a chaque paragraphe.
Redigez la prose avec une ponctuation francaise normale : conservez les points,
virgules, points-virgules, deux-points, points d'exclamation et points
d'interrogation quand ils sont utiles a la lisibilite.
Ne supprimez jamais la ponctuation de phrase ; remplacez seulement le tiret
cadratin `—` par le tiret simple `-`.
