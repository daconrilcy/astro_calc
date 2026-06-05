# Cas de test natal simplifie - matrice input_precision x computed_scope (plan v2.4).

function Get-SimplifiedNatalParisLocation {
    return @{
        latitude  = 48.8566
        longitude = 2.3522
        label     = "Paris"
    }
}

function Get-SimplifiedNatalPositiveCases {
    $paris = Get-SimplifiedNatalParisLocation
    $contract = "astro_simplified_natal_request_v1"
    $excludedStable = @("ascendant", "houses", "sect", "house_placements")

    return @(
        [ordered]@{
            Label              = "date_only"
            Description        = "Date seule - fenetre ~50h, scope stable"
            Request            = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{ date = "1990-06-15" }
            }
            ExpectedInputPrecision = "date_only"
            ExpectedScope          = "stable_birth_date_profile"
            ExpectedLimitations    = @("birth_time_missing")
            ExpectedExcluded       = $excludedStable
            ExpectCounts           = $false
        },
        [ordered]@{
            Label              = "date_with_location_without_timezone"
            Description        = "Date + lieu sans timezone - scope stable, limitation lieu sans TZ"
            Request            = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{
                    date     = "1990-06-15"
                    location = $paris
                }
                input_metadata           = [ordered]@{ location_label = "Paris, France" }
            }
            ExpectedInputPrecision = "date_with_location_without_timezone"
            ExpectedScope          = "stable_birth_date_profile"
            ExpectedLimitations    = @("birth_time_missing", "location_provided_without_usable_timezone")
            ExpectedExcluded       = $excludedStable
            ExpectCounts           = $false
        },
        [ordered]@{
            Label              = "date_with_timezone_without_time"
            Description        = "Date + timezone sans heure - fenetre 24h locale"
            Request            = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{
                    date     = "1990-06-15"
                    timezone = "Europe/Paris"
                }
            }
            ExpectedInputPrecision = "date_with_timezone_without_time"
            ExpectedScope          = "stable_birth_date_profile"
            ExpectedLimitations    = @("birth_time_missing")
            ExpectedExcluded       = $excludedStable
            ExpectCounts           = $false
        },
        [ordered]@{
            Label              = "date_with_location_and_timezone_without_time"
            Description        = "Date + lieu + timezone sans heure - fenetre 24h locale"
            Request            = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{
                    date     = "1990-06-15"
                    timezone = "Europe/Paris"
                    location = $paris
                }
            }
            ExpectedInputPrecision = "date_with_location_and_timezone_without_time"
            ExpectedScope          = "stable_birth_date_profile"
            ExpectedLimitations    = @("birth_time_missing")
            ExpectedExcluded       = $excludedStable
            ExpectCounts           = $false
        },
        [ordered]@{
            Label              = "datetime_without_location"
            Description        = "Date + heure + timezone sans lieu - positions planetaires + aspects"
            Request            = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{
                    date     = "1990-06-15"
                    time     = "14:30:00"
                    timezone = "Europe/Paris"
                }
            }
            ExpectedInputPrecision = "datetime_without_location"
            ExpectedScope          = "planetary_positions"
            ExpectedLimitations    = @("location_missing_for_ascendant_and_houses")
            ExpectedExcluded       = $excludedStable
            ExpectCounts           = $true
            MinAspectCount         = 0
        },
        [ordered]@{
            Label              = "complete_birth_data"
            Description        = "Donnees completes - theme angular (ASC, maisons, aspects)"
            Request            = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{
                    date     = "1990-06-15"
                    time     = "14:30:00"
                    timezone = "Europe/Paris"
                    location = $paris
                }
            }
            ExpectedInputPrecision = "complete_birth_data"
            ExpectedScope          = "angular_chart"
            ExpectedLimitations    = @()
            ExpectedExcluded       = @()
            ExpectCounts           = $true
            MinPositionCount       = 1
            MinHouseCuspCount      = 1
            MinAspectCount         = 0
        },
        [ordered]@{
            Label              = "date_only_equinox_window"
            Description        = "Date equinoxe - verifie gestion faits ambigus / llm_controls moon.sign"
            Request            = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{ date = "1990-03-21" }
            }
            ExpectedInputPrecision = "date_only"
            ExpectedScope          = "stable_birth_date_profile"
            ExpectedLimitations    = @("birth_time_missing")
            ExpectedExcluded       = $excludedStable
            ExpectCounts           = $false
            AssertMoonAmbiguity    = $true
            AssertSunAmbiguity     = $true
            ExpectAmbiguousChapter = $true
        }
    )
}

function Get-SimplifiedNatalNegativeCases {
    $contract = "astro_simplified_natal_request_v1"
    $paris = Get-SimplifiedNatalParisLocation

    return @(
        [ordered]@{
            Label           = "invalid_date"
            Description     = "Date invalide - 422"
            Request         = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{ date = "not-a-date" }
            }
            ExpectedStatus  = 422
        },
        [ordered]@{
            Label           = "invalid_calendar_date"
            Description     = "Date calendaire impossible - 422"
            Request         = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{ date = "2024-02-30" }
            }
            ExpectedStatus  = 422
        },
        [ordered]@{
            Label           = "time_without_timezone"
            Description     = "Heure sans timezone - 422 metier"
            Request         = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{
                    date = "1990-06-15"
                    time = "14:30:00"
                }
            }
            ExpectedStatus  = 422
        },
        [ordered]@{
            Label           = "invalid_latitude"
            Description     = "Latitude hors bornes - 422"
            Request         = [ordered]@{
                request_contract_version = $contract
                birth                    = [ordered]@{
                    date     = "1990-06-15"
                    location = @{ latitude = 91.0; longitude = 2.3522 }
                }
            }
            ExpectedStatus  = 422
        },
        [ordered]@{
            Label           = "wrong_contract_version"
            Description     = "Contrat request obsolete - 422"
            Request         = [ordered]@{
                request_contract_version = "astro_simplified_natal_request_v0"
                birth                    = [ordered]@{ date = "1990-06-15" }
            }
            ExpectedStatus  = 422
        }
    )
}

function Get-SimplifiedNatalCaseByLabel {
    param([string]$Label)

    $all = @(Get-SimplifiedNatalPositiveCases) + @(Get-SimplifiedNatalNegativeCases)
    $match = $all | Where-Object { $_.Label -eq $Label }
    if (-not $match) {
        throw "Cas inconnu : $Label"
    }
    return $match
}

function Select-SimplifiedNatalCases {
    param(
        [string[]]$Labels = @(),
        [ValidateSet("positive", "negative", "all")]
        [string]$Kind = "all"
    )

    $cases = @()
    if ($Kind -in @("positive", "all")) {
        $cases += Get-SimplifiedNatalPositiveCases
    }
    if ($Kind -in @("negative", "all")) {
        $cases += Get-SimplifiedNatalNegativeCases
    }
    if ($Labels.Count -gt 0) {
        $cases = $cases | Where-Object { $_.Label -in $Labels }
    }
    if ($cases.Count -eq 0) {
        throw "Aucun cas selectionne (Labels=$($Labels -join ','), Kind=$Kind)"
    }
    return $cases
}
