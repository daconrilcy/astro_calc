use std::collections::HashSet;

use astral_llm_domain::{
    integration::IntegrationService,
    interpretive_evidence::{ChapterEvidenceSlot, EvidenceRequirement, InterpretiveEvidence},
    ProductGenerationPolicy,
};
use astral_llm_infra::{CanonicalCatalog, EvidenceCanonicalCatalog, SharedCanonicalCatalog};

#[derive(Clone)]
pub struct ReadingCatalog {
    shared: SharedCanonicalCatalog,
}

#[derive(Clone, Copy)]
pub struct AstroBasisRoleCatalogView<'a> {
    roles: &'a HashSet<String>,
}

impl<'a> AstroBasisRoleCatalogView<'a> {
    pub(crate) fn new(shared: &'a SharedCanonicalCatalog) -> Self {
        Self {
            roles: &shared.astro_basis_roles,
        }
    }

    pub fn allowed_roles(&self) -> HashSet<String> {
        self.roles.clone()
    }
}

#[derive(Clone, Copy)]
pub struct EvidenceCatalogView<'a> {
    evidence: &'a EvidenceCanonicalCatalog,
}

impl<'a> EvidenceCatalogView<'a> {
    pub(crate) fn new(evidence: &'a EvidenceCanonicalCatalog) -> Self {
        Self { evidence }
    }

    pub fn slots_for_chapter(&self, chapter_code: &str) -> Vec<&'a ChapterEvidenceSlot> {
        self.evidence.slots_for_chapter(chapter_code)
    }

    pub fn requirements_for_chapter(&self, chapter_code: &str) -> Vec<&'a EvidenceRequirement> {
        self.evidence.requirements_for_chapter(chapter_code)
    }

    pub fn excludes_candidate(&self, chapter_code: &str, ev: &InterpretiveEvidence) -> bool {
        self.evidence.excludes_candidate(chapter_code, ev)
    }
}

impl ReadingCatalog {
    pub fn new(shared: SharedCanonicalCatalog) -> Self {
        Self { shared }
    }

    pub(crate) fn shared_catalog(&self) -> &SharedCanonicalCatalog {
        &self.shared
    }

    pub(crate) fn canonical_catalog(&self) -> &CanonicalCatalog {
        self.shared.as_ref()
    }

    pub fn integration_service(&self, service_code: &str) -> Option<&IntegrationService> {
        self.shared.integration_service(service_code)
    }

    pub fn list_integration_services(&self, include_planned: bool) -> Vec<&IntegrationService> {
        self.shared.list_integration_services(include_planned)
    }

    pub fn product_policy(&self, product_code: &str) -> Option<&ProductGenerationPolicy> {
        self.shared.product_policy(product_code)
    }

    pub fn astro_basis_roles_view(&self) -> AstroBasisRoleCatalogView<'_> {
        AstroBasisRoleCatalogView::new(self.shared_catalog())
    }

    pub fn evidence_catalog_view(&self) -> EvidenceCatalogView<'_> {
        EvidenceCatalogView::new(&self.shared_catalog().evidence)
    }
}

impl From<SharedCanonicalCatalog> for ReadingCatalog {
    fn from(shared: SharedCanonicalCatalog) -> Self {
        Self::new(shared)
    }
}
