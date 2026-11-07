//! Authored institution identities and the governing context of accepted commands.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A modeled institution is distinct from an executive holder or governing mandate.
/// ```compile_fail
/// use sim_core::{GoverningContext, ExecutiveBodyId};
/// fn wrong_family(context: &mut GoverningContext) {
///     context.primary = ExecutiveBodyId("cabinet".into());
/// }
/// ```
/// ```compile_fail
/// use sim_core::{GoverningContext, GovernmentId};
/// fn wrong_family(context: &mut GoverningContext) { context.primary = GovernmentId(1); }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct InstitutionId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstitutionDefinition {
    pub id: InstitutionId,
    pub country: CountryId,
    pub name: String,
    /// A decision-route name, or consultative_council.
    pub role: String,
    pub source: String,
    pub source_version: String,
    pub quality: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstitutionBinding {
    pub country: CountryId,
    pub route: DecisionRoute,
    pub primary: InstitutionId,
    pub council: InstitutionId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoverningInstitutionsDefinition {
    pub institutions: Vec<InstitutionDefinition>,
    pub bindings: Vec<InstitutionBinding>,
}
fn role(route: &DecisionRoute) -> &'static str {
    match route {
        DecisionRoute::Legislature => "legislature",
        DecisionRoute::PartyCouncil => "party_council",
        DecisionRoute::RoyalDecree => "royal_decree",
        DecisionRoute::ReligiousCouncil => "religious_council",
        DecisionRoute::MilitaryCouncil => "military_council",
        DecisionRoute::ExecutiveDecree => "executive_decree",
    }
}
impl GoverningInstitutionsDefinition {
    pub fn institution(&self, id: &InstitutionId) -> Option<&InstitutionDefinition> {
        self.institutions.iter().find(|i| &i.id == id)
    }
    fn binding(
        &self,
        country: &CountryId,
        route: &DecisionRoute,
    ) -> Result<&InstitutionBinding, String> {
        self.bindings
            .iter()
            .find(|b| &b.country == country && &b.route == route)
            .ok_or_else(|| "Missing governing institution binding".into())
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        let roles = [
            "legislature",
            "party_council",
            "royal_decree",
            "religious_council",
            "military_council",
            "executive_decree",
            "consultative_council",
        ];
        for i in &self.institutions {
            if i.id.0.is_empty()
                || i.id.0.len() > 192
                || !ids.insert(&i.id)
                || content.country(&i.country).is_none()
                || !roles.contains(&i.role.as_str())
                || i.name.trim().is_empty()
                || i.name.len() > 256
                || i.source_version.trim().is_empty()
                || i.source_version.len() > 192
                || i.quality != "authored"
                || !content.sources.iter().any(|s| s.id == i.source)
            {
                return Err("Invalid modeled institution identity or provenance".into());
            }
        }
        if self.institutions.len() != content.countries.len() * 7
            || self.bindings.len() != content.countries.len() * 6
        {
            return Err("Institution coverage must include all seven roles and six decision routes per country".into());
        }
        for country in &content.countries {
            for r in roles {
                if self
                    .institutions
                    .iter()
                    .filter(|i| i.country == country.id && i.role == r)
                    .count()
                    != 1
                {
                    return Err("Duplicate or missing country institution role".into());
                }
            }
            for route in [
                DecisionRoute::Legislature,
                DecisionRoute::PartyCouncil,
                DecisionRoute::RoyalDecree,
                DecisionRoute::ReligiousCouncil,
                DecisionRoute::MilitaryCouncil,
                DecisionRoute::ExecutiveDecree,
            ] {
                if self
                    .bindings
                    .iter()
                    .filter(|b| b.country == country.id && b.route == route)
                    .count()
                    != 1
                {
                    return Err("Duplicate or missing institution route binding".into());
                }
                let b = self.binding(&country.id, &route)?;
                if self
                    .institution(&b.primary)
                    .is_none_or(|i| i.country != country.id || i.role != role(&route))
                    || self
                        .institution(&b.council)
                        .is_none_or(|i| i.country != country.id || i.role != "consultative_council")
                {
                    return Err("Governing institution binding crosses countries or roles".into());
                }
            }
        }
        Ok(())
    }
}
/// Context at submission, not evidence of a vote, approval or later consent.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoverningContext {
    pub government: GovernmentId,
    pub route: DecisionRoute,
    pub primary: InstitutionId,
    pub council: Option<InstitutionId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoverningInstitutionsView {
    pub context: GoverningContext,
    pub primary: InstitutionDefinition,
    pub council: Option<InstitutionDefinition>,
    pub backing: i32,
    pub threshold: i32,
    pub council_backing: i32,
    pub council_threshold: i32,
}
impl World {
    pub(crate) fn capture_governing_context(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<Option<GoverningContext>, String> {
        let Some(definition) = &content.governing_institutions else {
            return Ok(None);
        };
        let rules = self.governing_rules(content, id)?;
        let binding = definition.binding(id, &rules.route)?;
        Ok(Some(GoverningContext {
            government: self
                .countries
                .get(id)
                .ok_or("Unknown governing country")?
                .government_number,
            route: rules.route.clone(),
            primary: binding.primary.clone(),
            council: (rules.council_threshold > 0 || rules.route == DecisionRoute::RoyalDecree)
                .then(|| binding.council.clone()),
        }))
    }
    pub fn governing_institutions_view(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &CountryId,
    ) -> Result<Option<GoverningInstitutionsView>, String> {
        self.check_institution_content(content)?;
        if viewer != Viewer::Government(id.clone()) {
            return Ok(None);
        }
        let Some(context) = self.capture_governing_context(content, id)? else {
            return Ok(None);
        };
        let definition = content
            .governing_institutions
            .as_ref()
            .ok_or("Missing institutions")?;
        let c = self.countries.get(id).ok_or("Unknown governing country")?;
        let rules = self.governing_rules(content, id)?;
        Ok(Some(GoverningInstitutionsView {
            primary: definition
                .institution(&context.primary)
                .ok_or("Unknown primary institution")?
                .clone(),
            council: context
                .council
                .as_ref()
                .map(|id| {
                    definition
                        .institution(id)
                        .cloned()
                        .ok_or("Unknown council institution")
                })
                .transpose()?,
            backing: if context.route == DecisionRoute::Legislature {
                c.legislative_support
            } else {
                c.elite_support
            },
            threshold: rules.threshold,
            council_backing: c.council_support,
            council_threshold: rules.council_threshold,
            context,
        }))
    }
    pub fn command_governing_context(
        &self,
        content: &Content,
        viewer: Viewer,
        command: CommandId,
    ) -> Result<Option<GoverningContext>, String> {
        self.check_institution_content(content)?;
        let Some(entry) = self.journal_entry(&command) else {
            return Ok(None);
        };
        if viewer != Viewer::Government(entry.command.issuer.clone()) {
            return Ok(None);
        }
        Ok(entry.governing_context.clone())
    }
    fn check_institution_content(&self, content: &Content) -> Result<(), String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Institution inspection requires the campaign's exact content".into());
        }
        Ok(())
    }
    pub(crate) fn validate_governing_context(
        &self,
        content: &Content,
        entry: &JournalEntry,
    ) -> Result<(), String> {
        let Some(definition) = &content.governing_institutions else {
            return if entry.governing_context.is_none() {
                Ok(())
            } else {
                Err("Earlier campaigns cannot contain governing institution context".into())
            };
        };
        if entry.developer {
            return if entry.governing_context.is_none() {
                Ok(())
            } else {
                Err("Developer bypass has no governing institution context".into())
            };
        }
        let context = entry
            .governing_context
            .as_ref()
            .ok_or("Missing accepted-command governing context")?;
        let id = &entry.command.issuer;
        let c = self.countries.get(id).ok_or("Unknown context country")?;
        // Budgets dispatch after constitutional consent but before end-of-day politics.
        // Their validated sponsor is authoritative for this intraday submission.
        let government = if let Some(bill) = entry.budget_bill {
            let sponsor = c
                .fiscal_plans
                .as_ref()
                .and_then(|s| s.bills.iter().find(|b| b.id == bill))
                .map(|b| b.sponsor)
                .ok_or("Missing budget context sponsor")?;
            let proposal = self
                .journal_entry(&CommandId(bill.0))
                .and_then(|entry| entry.governing_context.as_ref())
                .ok_or("Missing budget proposal governing context")?;
            if proposal.government != sponsor {
                return Err("Budget sponsor conflicts with its accepted proposal context".into());
            }
            sponsor
        } else {
            c.executive_identity
                .as_ref()
                .ok_or("Missing executive provenance")?
                .bodies
                .values()
                .filter_map(|b| match b.origin {
                    ExecutiveBodyOrigin::Generated { government, .. }
                        if b.created_day <= entry.command.day =>
                    {
                        Some(government)
                    }
                    _ => None,
                })
                .max()
                .unwrap_or(GovernmentId(1))
        };
        let rules = if let Some(change) = c.constitution.as_ref().and_then(|s| {
            s.history
                .iter()
                .rev()
                .find(|h| h.day <= entry.command.day && h.government_number <= government)
        }) {
            &content
                .constitutions
                .as_ref()
                .and_then(|d| d.template(&change.to))
                .ok_or("Missing context constitution")?
                .government
        } else {
            &content
                .country(id)
                .ok_or("Missing opening context")?
                .government
        };
        let binding = definition.binding(id, &rules.route)?;
        let council = (rules.council_threshold > 0 || rules.route == DecisionRoute::RoyalDecree)
            .then(|| binding.council.clone());
        if context.government != government
            || context.route != rules.route
            || context.primary != binding.primary
            || context.council != council
        {
            return Err("Accepted-command governing context conflicts with its country, mandate or decision route".into());
        }
        Ok(())
    }
}
