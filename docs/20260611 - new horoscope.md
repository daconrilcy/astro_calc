flowchart TD
    A["Calcul natal déjà produit<br/>natal_calculation_response"] --> C
    B["Calcul horoscope déjà produit<br/>period_calculation_response"] --> C
    L["Langue de sortie<br/>target_language_code: fr / en / es / de"] --> C
    P["Persona astrologue JSON<br/>ton, champ lexical, priorités, style"] --> C
    S["Profil sécurité<br/>pas de santé, pas de fatalisme,<br/>pas de promesse financière"] --> C

    C["Assembler les inputs LLM<br/>faits de calcul + keywords + sécurité + structure"] --> D

    D["Compression sémantique<br/>extraire signaux dominants, intensités,<br/>jours clés, fenêtres, domaines"] --> E

    E["Construire le brief LLM<br/>sans phrase publique pré-rédigée"] --> F

    F["Prompt système<br/>rôle du LLM, sécurité, langue,<br/>contrat JSON, interdits"] --> G
    E --> G

    G["LLM writer<br/>rédaction complète des textes publics"] --> H

    H["Réponse JSON brute<br/>horoscope_period_response_v1"] --> I{"Validation structurelle<br/>JSON valide ?<br/>champs requis ?<br/>dates cohérentes ?"}

    I -- "Non" --> R1["Retry technique<br/>corriger uniquement le JSON<br/>sans changer l'interprétation"]
    R1 --> I

    I -- "Oui" --> J{"Validation sécurité<br/>pas de santé ?<br/>pas de fatalisme ?<br/>pas de promesse financière ?<br/>pas de jargon interne ?"}

    J -- "Non" --> R2["LLM editor sécurité<br/>réécrire uniquement les champs fautifs"]
    R2 --> J

    J -- "Oui" --> K{"Validation qualité texte<br/>pas d'artefacts ?<br/>pas de phrase tronquée ?<br/>pas de répétition mécanique ?<br/>langue respectée ?"}

    K -- "Non" --> R3["LLM editor stylistique ciblé<br/>naturaliser les champs fautifs<br/>conserver structure et evidence_keys"]
    R3 --> K

    K -- "Oui" --> M["Post-traitement léger<br/>typographie, espaces, ponctuation,<br/>normalisation finale"]

    M --> N{"Contrôle final<br/>texte public propre ?"}

    N -- "Non" --> R4["Échec contrôlé<br/>fallback premium court<br/>ou demande de régénération"]
    N -- "Oui" --> O["Réponse publique finale<br/>UI contract uniquement"]

    O --> UI["Affichage UI<br/>vue d'ensemble, conseils,<br/>jours clés, timeline, domaines,<br/>fenêtres, stratégie"]

    O --> LOG["Logs qualité / debug<br/>non affichés à l'utilisateur"]

    subgraph PublicContract["Contrat public UI"]
        PC1["week_overview"]
        PC2["advice"]
        PC3["key_days"]
        PC4["best_days"]
        PC5["watch_days"]
        PC6["daily_timeline"]
        PC7["domain_sections"]
        PC8["best_windows"]
        PC9["strategy"]
    end

    O --> PublicContract

    subgraph DebugOnly["Debug interne non affiché"]
        D1["raw_signals"]
        D2["evidence_keys"]
        D3["quality_checks"]
        D4["model_info"]
        D5["retry_count"]
        D6["persona_id"]
        D7["target_language_code"]
    end

    LOG --> DebugOnly