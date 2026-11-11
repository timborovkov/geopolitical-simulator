//! Participant negotiation data and the distinct public signed-agreement inspector.
use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicTreatyRatification {
    pub country: CountryId,
    pub submitted: Option<CampaignDay>,
    pub review: Option<CampaignDay>,
    pub passed: Option<CampaignDay>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicTreatyNotice {
    pub by: CountryId,
    pub day: CampaignDay,
    pub until: CampaignDay,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicTreatyMilestone {
    pub id: TreatyMilestoneId,
    pub day: CampaignDay,
    pub revision: TreatyRevision,
    pub outcome: TreatyOutcome,
    pub actor: Option<CountryId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedTreatyDetail {
    pub administration_paid: Money,
    pub service_charge: Money,
    pub available_national_cash: Money,
    pub authority: String,
    pub own_signature: Option<TreatySignature>,
    pub own_ratification: Option<TreatyRatification>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreatyView {
    pub id: TreatyId,
    pub parties: [CountryId; 2],
    pub revision: TreatyRevision,
    pub offered_by: CountryId,
    pub terms: NonAggressionTerms,
    pub stage: TreatyStage,
    pub offer_deadline: Option<CampaignDay>,
    pub ratification_submission_deadline: Option<CampaignDay>,
    pub signed_on: Option<CampaignDay>,
    pub activation_day: Option<CampaignDay>,
    pub activated_on: Option<CampaignDay>,
    pub expires_at: Option<CampaignDay>,
    pub closed_on: Option<CampaignDay>,
    pub notice: Option<PublicTreatyNotice>,
    pub ratifications: Vec<PublicTreatyRatification>,
    pub milestones: Vec<PublicTreatyMilestone>,
    pub owned: Option<OwnedTreatyDetail>,
}

impl World {
    /// Unaccepted negotiations are visible only on the controlling government's
    /// own desk. Naming another viewer or country never unlocks private terms.
    pub fn treaties_view(
        &self,
        content: &Content,
        viewer: Viewer,
        country: &CountryId,
    ) -> Result<Vec<TreatyView>, String> {
        if self.content_hash != content.hash() {
            return Err("Treaty inspection requires the campaign's exact content".into());
        }
        if !self.countries.contains_key(country) {
            return Err("Unknown treaty country".into());
        }
        self.validate_treaties(content)?;
        let Some(state) = &self.diplomacy else {
            return Ok(Vec::new());
        };
        let rules = content
            .diplomacy
            .as_ref()
            .ok_or("Missing treaty definitions")?;
        let owned_desk =
            viewer == Viewer::Government(self.player.clone()) && country == &self.player;
        let mut result = Vec::new();
        for record in state
            .agreements
            .values()
            .filter(|record| record.participant(country))
        {
            let own = owned_desk && record.participant(&self.player);
            if !own && record.signed_on.is_none() {
                continue;
            }
            let owned = if own {
                Some(OwnedTreatyDetail {
                    administration_paid: record
                        .administration_paid
                        .get(&self.player)
                        .copied()
                        .unwrap_or(Money(0)),
                    service_charge: rules.administration(
                        content
                            .country(&self.player)
                            .ok_or("Unknown controlling government")?,
                    )?,
                    available_national_cash: self.player_country().available_cash(),
                    authority: self.governing_rules(content, &self.player)?.label(),
                    own_signature: record.signatures.get(&self.player).cloned(),
                    own_ratification: record.ratifications.get(&self.player).cloned(),
                })
            } else {
                None
            };
            let public_from = record
                .milestones
                .iter()
                .find(|milestone| milestone.outcome == TreatyOutcome::Signed)
                .map(|milestone| milestone.id);
            let milestones = record
                .milestones
                .iter()
                .filter(|milestone| own || public_from.is_some_and(|first| milestone.id >= first))
                .map(|milestone| PublicTreatyMilestone {
                    id: milestone.id,
                    day: milestone.day,
                    revision: milestone.revision,
                    outcome: milestone.outcome,
                    actor: milestone.actor.clone(),
                })
                .collect();
            result.push(TreatyView {
                id: record.id,
                parties: record.parties.clone(),
                revision: record.revision,
                offered_by: record.offered_by.clone(),
                terms: record.terms.clone(),
                stage: record.stage,
                offer_deadline: (own && record.stage == TreatyStage::Negotiating)
                    .then_some(record.offer_deadline),
                ratification_submission_deadline: record
                    .signed_on
                    .map(|signed| crate::diplomacy::treaty_date(signed, rules.response_days))
                    .transpose()?,
                signed_on: record.signed_on,
                activation_day: record.activation_day,
                activated_on: record.activated_on,
                expires_at: record.expires_at,
                closed_on: record.closed_on,
                notice: record.notice.as_ref().map(|notice| PublicTreatyNotice {
                    by: notice.by.clone(),
                    day: notice.day,
                    until: notice.until,
                }),
                ratifications: record
                    .parties
                    .iter()
                    .map(|party| {
                        let request = record.ratifications.get(party);
                        PublicTreatyRatification {
                            country: party.clone(),
                            submitted: request.map(|request| request.submitted),
                            review: request.map(|request| request.review),
                            passed: request.and_then(|request| request.passed),
                        }
                    })
                    .collect(),
                milestones,
                owned,
            });
        }
        Ok(result)
    }

    /// A retained owned Chronicle reference can point at its treaty without
    /// exposing any other country's private negotiation or events.
    pub fn event_treaty(
        &self,
        content: &Content,
        viewer: Viewer,
        event: &EventId,
    ) -> Result<Option<TreatyId>, String> {
        if viewer != Viewer::Government(self.player.clone()) || event.country != self.player {
            return Ok(None);
        }
        let Some(event) = self.event_by_reference(content, viewer, event)? else {
            return Ok(None);
        };
        if let Some(EventContext::Treaty {
            agreement,
            milestone,
        }) = event.context
        {
            self.validate_treaty_event(event, agreement, milestone)?;
            Ok(Some(agreement))
        } else {
            Ok(None)
        }
    }
}
