# Review adversariale - natal neutral explanations pre-generation

Date: 2026-06-24

Perimetre:
- gateway natal V2;
- route interne LLM de preparation des explications;
- cache `llm_natal_fact_explanations`;
- injection prompt `neutral_explanations`.

Findings verifies:
- Aucun changement du contrat `generate_reading_response_v1`; le nouveau bloc est sibling gateway.
- La selection des elements est faite apres normalisation LLM, pas dans le calculateur.
- Le cache est accede via le port applicatif de persistence, avec adaptation infra dediee.
- Les echecs de preparation sont non bloquants et ne changent pas une lecture reussie en echec.

Risques residuels:
- Le cache miss ajoute un appel provider avant la lecture principale; surveiller la latence sur les premiers runs.
- La selection deterministe est volontairement conservative et devra etre ajustee si les profils produit demandent un quota different.

Statut:
- Aucun finding ouvert.
