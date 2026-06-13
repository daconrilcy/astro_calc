# REV-005 - Response shape et evidence

## Contexte

La reponse Premium etend `horoscope_period_response`.

## Checks

- `strategy` est present.
- `domain_sections` contient 3 a 5 sections.
- Aucune evidence publique n'est inventee.
- Aucune fuite de code technique ou de hint interne.

## Findings

- P1 : la shape Basic pouvait passer sans elements Premium si les guards ne regardaient que la timeline.

## Corrections

- Guard `HOROSCOPE_PERIOD_PREMIUM_INSUFFICIENT_DETAIL`.
- Validation dediee des fenetres, strategy et profondeur domaines.

## Statut

Closed - aucun P0/P1 ouvert.
