//! Configurable authority, consent stages and political survival.
use crate::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionRoute {
    Legislature,
    PartyCouncil,
    RoyalDecree,
    ReligiousCouncil,
    MilitaryCouncil,
    ExecutiveDecree,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernmentDefinition {
    pub route: DecisionRoute,
    pub threshold: i32,
    pub consent_days: u32,
    pub council_threshold: i32,
    pub leadership_review_days: Option<u32>,
}
impl GovernmentDefinition {
    pub fn label(&self) -> String {
        let name = match self.route {
            DecisionRoute::Legislature => "Legislative vote",
            DecisionRoute::PartyCouncil => "Party leadership and cadre decision",
            DecisionRoute::RoyalDecree => "Royal decree with council consultation",
            DecisionRoute::ReligiousCouncil => "Religious council",
            DecisionRoute::MilitaryCouncil => "Military council",
            DecisionRoute::ExecutiveDecree => "Executive decree",
        };
        format!(
            "{name}: backing ≥{}, council ≥{}; consent {} days",
            self.threshold, self.council_threshold, self.consent_days
        )
    }
    pub fn check(&self, c: &Country, policy: &str, value: i32) -> Result<(), String> {
        if policy == "coalition" {
            return Ok(());
        }
        let support = if self.route == DecisionRoute::Legislature {
            c.legislative_support
        } else {
            c.elite_support
        };
        if support < self.threshold || c.council_support < self.council_threshold {
            return Err(format!(
                "{}; current backing {}, council {}. Negotiate governing support first.",
                self.label(),
                support,
                c.council_support
            ));
        }
        if self.route == DecisionRoute::RoyalDecree && matches!(policy, "services" | "welfare") {
            let increase = value
                - if policy == "services" {
                    c.services
                } else {
                    c.welfare
                };
            if increase > 4 && c.council_support < 65 {
                return Err("A royal spending expansion above 4 GDP points requires council backing 65; negotiate support or phase the increase".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyStage {
    AwaitingConsent,
    Implementing,
}
impl World {
    pub(crate) fn resolve_consents(&mut self, content: &Content, id: &CountryId) {
        let government = self
            .governing_rules(content, id)
            .expect("government")
            .clone();
        let due: Vec<_> = self.countries[id]
            .projects
            .iter()
            .filter(|p| p.stage == PolicyStage::AwaitingConsent && p.consent_day <= self.day)
            .cloned()
            .collect();
        for project in due {
            let result = government.check(&self.countries[id], &project.policy, project.value);
            self.countries
                .get_mut(id)
                .expect("country")
                .committed_cash
                .0 -= project.reserved.0;
            match result {
                Ok(()) => {
                    let paid = self.pay(
                        id,
                        Account::Treasury,
                        Account::Firms,
                        project.reserved.0,
                        "Approved policy capital payment",
                    );
                    self.countries
                        .get_mut(id)
                        .expect("country")
                        .budget
                        .capital
                        .0 += paid;
                    let p = self
                        .countries
                        .get_mut(id)
                        .expect("country")
                        .projects
                        .iter_mut()
                        .find(|p| p.id == project.id)
                        .expect("project");
                    p.stage = PolicyStage::Implementing;
                    p.reserved = Money(0);
                    self.event(id,true,format!("{} received institutional consent. {}m paid to contractors; construction due {}.",project.policy,paid,date(project.due_day)));
                }
                Err(reason) => {
                    self.countries
                        .get_mut(id)
                        .expect("country")
                        .projects
                        .retain(|p| p.id != project.id);
                    self.event(id,true,format!("{} refused at consent: {reason}. Released {}m of reserved cash; proposal administration costs remain paid.",project.policy,project.reserved.0));
                }
            }
        }
    }
}
