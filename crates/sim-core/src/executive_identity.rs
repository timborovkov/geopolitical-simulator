//! Live executive references are distinct from presentation and governing mandates.
//! Bookmark terms initialize these records; later reference dates do not force events.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A collective actor key, independent of its name and governmental mandate.
/// ```compile_fail
/// use sim_core::{ExecutiveBodyId, PersonId};
/// let council: ExecutiveBodyId = PersonId("person:example".into());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ExecutiveBodyId(pub String);
/// A country-scoped tenure reference. It is never a row index.
/// ```compile_fail
/// use sim_core::{ExecutiveTenureId, GovernmentId};
/// let tenure: ExecutiveTenureId = GovernmentId(1);
/// ```
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ExecutiveTenureId(pub u32);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutiveIdentityDefinition {
    pub opening_offices: BTreeMap<CountryId, OfficeId>,
    pub initial_collectives: BTreeMap<CountryId, ExecutiveBodyId>,
    pub retained_closed_tenures: u32,
    pub source: String,
    pub source_version: String,
}
impl ExecutiveIdentityDefinition {
    fn reference<'a>(
        &self,
        content: &'a Content,
        country: &CountryId,
    ) -> Result<(&'a OfficeDefinition, Option<&'a OfficeTerm>), String> {
        let identities = content
            .identities
            .as_ref()
            .ok_or("Executive identities need a reference catalogue")?;
        let id = self
            .opening_offices
            .get(country)
            .ok_or("Missing opening executive-office reference")?;
        let start: CivilDate = content.start.parse()?;
        let office = identities
            .offices
            .iter()
            .find(|office| {
                &office.id == id && &office.country == country && office.validity.contains(start)
            })
            .ok_or("Opening executive office is unknown, inactive or belongs to another country")?;
        let term = identities
            .terms
            .iter()
            .find(|term| &term.office == id && term.validity.contains(start));
        if term.is_none() && office.required {
            return Err("A required executive reference cannot have an unknown incumbent".into());
        }
        Ok((office, term))
    }
    fn opening_holder(
        &self,
        content: &Content,
        country: &CountryId,
    ) -> Result<ExecutiveHolder, String> {
        Ok(
            match self.reference(content, country)?.1.map(|term| &term.holder) {
                Some(OfficeHolder::Person { person }) => ExecutiveHolder::Person {
                    person: person.clone(),
                },
                Some(OfficeHolder::Collective { .. }) => ExecutiveHolder::Collective {
                    body: self
                        .initial_collectives
                        .get(country)
                        .ok_or("Collective executive needs a stable configured body ID")?
                        .clone(),
                },
                Some(OfficeHolder::Vacant) => ExecutiveHolder::Vacant,
                None => ExecutiveHolder::Unknown,
            },
        )
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if self.opening_offices.keys().ne(content
            .countries
            .iter()
            .map(|country| &country.id)
            .collect::<BTreeSet<_>>())
            || !(1..=256).contains(&self.retained_closed_tenures)
            || self.source_version.trim().is_empty()
            || self.source_version.len() > 128
            || !content
                .sources
                .iter()
                .any(|source| source.id == self.source)
        {
            return Err("Executive identity definitions need complete country/office coverage, bounded retention and provenance".into());
        }
        let mut collectives = BTreeSet::new();
        let mut body_ids = BTreeSet::new();
        for country in &content.countries {
            let (office, term) = self.reference(content, &country.id)?;
            if country
                .governing_office
                .as_ref()
                .is_some_and(|id| id != &office.id)
            {
                return Err(
                    "Live executive opening reference conflicts with the bookmark governing office"
                        .into(),
                );
            }
            let name = term.map_or_else(
                || "Unknown incumbent".to_string(),
                |term| {
                    content
                        .identities
                        .as_ref()
                        .expect("catalogue")
                        .holder_name(&term.holder)
                },
            );
            if name != country.executive {
                return Err(
                    "Opening executive label does not resolve from its explicit identity reference"
                        .into(),
                );
            }
            if matches!(
                term.map(|term| &term.holder),
                Some(OfficeHolder::Collective { .. })
            ) {
                collectives.insert(&country.id);
                let id = self
                    .initial_collectives
                    .get(&country.id)
                    .ok_or("Missing reference collective ID")?;
                if !id.0.starts_with("reference:")
                    || id.0.len() > 192
                    || !id.0.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'-' | b'.')
                    })
                    || !body_ids.insert(id)
                {
                    return Err("Reference collective IDs must be unique, bounded and separate from generated IDs".into());
                }
            }
        }
        if self.initial_collectives.keys().collect::<BTreeSet<_>>() != collectives {
            return Err(
                "Initial collective IDs must match exactly the declared collective holders".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutiveHolder {
    Person { person: PersonId },
    Collective { body: ExecutiveBodyId },
    Vacant,
    Unknown,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveBodyKind {
    Cabinet,
    LeadershipCouncil,
    TransitionalAuthority,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutiveBodyOrigin {
    Reference {
        term: String,
    },
    Generated {
        serial: u64,
        government: GovernmentId,
        body_kind: ExecutiveBodyKind,
        party: Option<PartyId>,
        target: Option<Regime>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutiveBody {
    pub id: ExecutiveBodyId,
    pub name: String,
    /// Empty means that membership is unspecified, not a generated list of people.
    pub members: Vec<PersonId>,
    pub created_day: CampaignDay,
    pub origin: ExecutiveBodyOrigin,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutiveTenure {
    pub id: ExecutiveTenureId,
    pub government: GovernmentId,
    pub holder: ExecutiveHolder,
    pub holder_since: CampaignDay,
    pub from_day: CampaignDay,
    pub until_day: Option<CampaignDay>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutiveIdentityState {
    /// An opening reference, not a claim that historical office powers persist after reform.
    pub opening_office: OfficeId,
    pub holder: ExecutiveHolder,
    pub holder_since: CampaignDay,
    pub government: GovernmentId,
    pub tenures: Vec<ExecutiveTenure>,
    pub bodies: BTreeMap<ExecutiveBodyId, ExecutiveBody>,
    pub next_generated_body: u64,
}
fn generated_id(country: &CountryId, serial: u64) -> ExecutiveBodyId {
    ExecutiveBodyId(format!("generated:{}:{serial}", country.0))
}
impl ExecutiveIdentityState {
    fn seed(
        content: &Content,
        country: &Country,
        definition: &ExecutiveIdentityDefinition,
        day: CampaignDay,
    ) -> Result<Self, String> {
        let (office, term) = definition.reference(content, &country.id)?;
        let holder = definition.opening_holder(content, &country.id)?;
        let mut bodies = BTreeMap::new();
        if let ExecutiveHolder::Collective { body } = &holder {
            let term = term.ok_or("Missing collective source term")?;
            let OfficeHolder::Collective { name, members } = &term.holder else {
                return Err("Collective reference mismatch".into());
            };
            bodies.insert(
                body.clone(),
                ExecutiveBody {
                    id: body.clone(),
                    name: name.clone(),
                    members: members.clone(),
                    created_day: day,
                    origin: ExecutiveBodyOrigin::Reference {
                        term: term.id.clone(),
                    },
                },
            );
        }
        Ok(Self {
            opening_office: office.id.clone(),
            holder: holder.clone(),
            holder_since: day,
            government: country.government_number,
            tenures: vec![ExecutiveTenure {
                id: ExecutiveTenureId(country.government_number.0),
                government: country.government_number,
                holder,
                holder_since: day,
                from_day: day,
                until_day: None,
            }],
            bodies,
            next_generated_body: 1,
        })
    }
    fn roll(
        &mut self,
        government: GovernmentId,
        holder: ExecutiveHolder,
        day: CampaignDay,
        retain: u32,
    ) -> Result<(), String> {
        if self.government.checked_add(1) != Some(government) {
            return Err("Executive tenures require the next stable governmental mandate".into());
        }
        let previous = self
            .tenures
            .last_mut()
            .ok_or("Missing current executive tenure")?;
        if previous.until_day.is_some() || day < previous.from_day {
            return Err("Invalid executive tenure transition date".into());
        }
        previous.until_day = Some(day);
        if holder != self.holder {
            self.holder_since = day;
        }
        self.holder = holder.clone();
        self.government = government;
        self.tenures.push(ExecutiveTenure {
            id: ExecutiveTenureId(government.0),
            government,
            holder,
            holder_since: self.holder_since,
            from_day: day,
            until_day: None,
        });
        while self.tenures.len() > retain as usize + 1 {
            self.tenures.remove(0);
        }
        Ok(())
    }
    fn holder_name(&self, content: &Content, holder: &ExecutiveHolder) -> Result<String, String> {
        Ok(match holder {
            ExecutiveHolder::Person { person } => content
                .identities
                .as_ref()
                .and_then(|identities| identities.people.iter().find(|entry| &entry.id == person))
                .ok_or("Unknown executive person identity")?
                .name
                .clone(),
            ExecutiveHolder::Collective { body } => self
                .bodies
                .get(body)
                .ok_or("Unknown executive collective identity")?
                .name
                .clone(),
            ExecutiveHolder::Vacant => content
                .identities
                .as_ref()
                .ok_or("Missing identity catalogue")?
                .holder_name(&OfficeHolder::Vacant),
            ExecutiveHolder::Unknown => "Unknown incumbent".into(),
        })
    }
}
impl Country {
    pub(crate) fn replace_generated_executive(
        &mut self,
        content: &Content,
        name: String,
        kind: ExecutiveBodyKind,
        party: Option<PartyId>,
        target: Option<Regime>,
        day: CampaignDay,
    ) -> Result<(), String> {
        match (&content.executive_identities, &mut self.executive_identity) {
            (None, None) => {}
            (Some(definition), Some(state)) => {
                let serial = state.next_generated_body;
                let next = serial
                    .checked_add(1)
                    .ok_or("Executive body identity exhausted")?;
                let id = generated_id(&self.id, serial);
                if serial == 0 || state.bodies.contains_key(&id) {
                    return Err("Executive body identity is already allocated".into());
                }
                state.bodies.insert(
                    id.clone(),
                    ExecutiveBody {
                        id: id.clone(),
                        name: name.clone(),
                        members: Vec::new(),
                        created_day: day,
                        origin: ExecutiveBodyOrigin::Generated {
                            serial,
                            government: self.government_number,
                            body_kind: kind,
                            party,
                            target,
                        },
                    },
                );
                state.next_generated_body = next;
                state.roll(
                    self.government_number,
                    ExecutiveHolder::Collective { body: id },
                    day,
                    definition.retained_closed_tenures,
                )?;
            }
            _ => return Err("Executive identity state/content mismatch".into()),
        }
        self.executive = name;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ExecutiveTenureView {
    pub tenure: ExecutiveTenure,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ExecutiveIdentityView {
    pub holder: ExecutiveHolder,
    pub name: String,
    pub holder_since: CampaignDay,
    pub government: GovernmentId,
    pub mandate_since: CampaignDay,
    pub opening_office: OfficeId,
    pub opening_office_title: String,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
    pub members: Vec<(PersonId, String)>,
    pub body_origin: Option<ExecutiveBodyOrigin>,
    pub opening_reference_term: Option<String>,
    pub tenures: Vec<ExecutiveTenureView>,
}
impl World {
    pub(crate) fn seed_executive_identities(&mut self, content: &Content) -> Result<(), String> {
        if let Some(definition) = &content.executive_identities {
            for country in self.countries.values_mut() {
                country.executive_identity = Some(ExecutiveIdentityState::seed(
                    content, country, definition, self.day,
                )?);
            }
        }
        Ok(())
    }
    pub(crate) fn sync_executive_identity(
        &mut self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let country = self
            .countries
            .get_mut(id)
            .ok_or("Unknown executive country")?;
        match (
            &content.executive_identities,
            &mut country.executive_identity,
        ) {
            (None, None) => Ok(()),
            (Some(definition), Some(state)) => {
                if state.government != country.government_number {
                    state.roll(
                        country.government_number,
                        state.holder.clone(),
                        self.day,
                        definition.retained_closed_tenures,
                    )?;
                }
                Ok(())
            }
            _ => Err("Executive identity state/content mismatch".into()),
        }
    }
    /// Live host truth. Normal clients expose this only within their owned-country view.
    /// Earlier campaigns return None; no live identities are reconstructed for them.
    pub fn executive_identity_view(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<Option<ExecutiveIdentityView>, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Executive inspection requires the campaign's exact content".into());
        }
        let country = self.countries.get(id).ok_or("Unknown executive country")?;
        let Some(state) = &country.executive_identity else {
            return if content.executive_identities.is_none() {
                Ok(None)
            } else {
                Err("Missing live executive identity state".into())
            };
        };
        let definition = content
            .executive_identities
            .as_ref()
            .ok_or("Missing executive identity definitions")?;
        let (office, term) = definition.reference(content, id)?;
        let (source, version, quality, member_ids) = match &state.holder {
            ExecutiveHolder::Person { person } => {
                let person = content
                    .identities
                    .as_ref()
                    .and_then(|identities| {
                        identities.people.iter().find(|entry| &entry.id == person)
                    })
                    .ok_or("Unknown executive person")?;
                (
                    person.source.clone(),
                    person.source_version.clone(),
                    person.quality,
                    Vec::new(),
                )
            }
            ExecutiveHolder::Collective { body } => {
                let body = state.bodies.get(body).ok_or("Unknown executive body")?;
                match &body.origin {
                    ExecutiveBodyOrigin::Reference { .. } => {
                        let term = term.ok_or("Missing collective reference term")?;
                        (
                            term.source.clone(),
                            term.source_version.clone(),
                            term.quality,
                            body.members.clone(),
                        )
                    }
                    ExecutiveBodyOrigin::Generated { .. } => (
                        definition.source.clone(),
                        definition.source_version.clone(),
                        RecordQuality::Authored,
                        Vec::new(),
                    ),
                }
            }
            _ => (
                office.source.clone(),
                office.source_version.clone(),
                RecordQuality::Unknown,
                Vec::new(),
            ),
        };
        let members = member_ids
            .into_iter()
            .map(|person| {
                let name = content
                    .identities
                    .as_ref()
                    .and_then(|identities| {
                        identities.people.iter().find(|entry| entry.id == person)
                    })
                    .ok_or("Unknown collective member")?
                    .name
                    .clone();
                Ok((person, name))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Some(ExecutiveIdentityView {
            holder: state.holder.clone(),
            name: state.holder_name(content, &state.holder)?,
            holder_since: state.holder_since,
            government: state.government,
            mandate_since: state
                .tenures
                .last()
                .ok_or("Missing executive tenure")?
                .from_day,
            opening_office: state.opening_office.clone(),
            opening_office_title: office.title.clone(),
            source,
            source_version: version,
            quality,
            members,
            body_origin: match &state.holder {
                ExecutiveHolder::Collective { body } => Some(
                    state
                        .bodies
                        .get(body)
                        .ok_or("Unknown executive body")?
                        .origin
                        .clone(),
                ),
                _ => None,
            },
            opening_reference_term: term.map(|term| term.id.clone()),
            tenures: state
                .tenures
                .iter()
                .map(|tenure| {
                    Ok(ExecutiveTenureView {
                        name: state.holder_name(content, &tenure.holder)?,
                        tenure: tenure.clone(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        }))
    }
    pub(crate) fn validate_executive_identity(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let country = &self.countries[id];
        let (Some(definition), Some(state)) =
            (&content.executive_identities, &country.executive_identity)
        else {
            return if content.executive_identities.is_none() && country.executive_identity.is_none()
            {
                Ok(())
            } else {
                Err("Executive identity state/content mismatch".into())
            };
        };
        let start = content.start_day()?;
        let (office, term) = definition.reference(content, id)?;
        let initial = definition.opening_holder(content, id)?;
        let mandate_count = country
            .government_number
            .0
            .checked_sub(1)
            .ok_or("Invalid executive mandate")?;
        let expected_terms = mandate_count.min(definition.retained_closed_tenures) as usize + 1;
        if state.opening_office != office.id
            || state.government != country.government_number
            || state.tenures.len() != expected_terms
            || state.next_generated_body == 0
        {
            return Err("Invalid executive office, mandate, retention or body allocator".into());
        }
        let mut generated = BTreeMap::new();
        let mut reference_count = 0;
        for (key, body) in &state.bodies {
            if key != &body.id
                || body.name.trim().is_empty()
                || body.name.len() > 512
                || body.created_day < start
                || body.created_day > self.day
            {
                return Err("Invalid executive body key, label or creation date".into());
            }
            match &body.origin {
                ExecutiveBodyOrigin::Reference { term: source_term } => {
                    reference_count += 1;
                    let term = term.ok_or("Reference executive body lacks a starting term")?;
                    let OfficeHolder::Collective { name, members } = &term.holder else {
                        return Err("Reference executive body was not a collective holder".into());
                    };
                    if source_term != &term.id
                        || definition.initial_collectives.get(id) != Some(key)
                        || &body.name != name
                        || &body.members != members
                        || body.created_day != start
                    {
                        return Err(
                            "Executive collective conflicts with its attributed opening identity"
                                .into(),
                        );
                    }
                }
                ExecutiveBodyOrigin::Generated {
                    serial,
                    government,
                    body_kind,
                    party,
                    target,
                } => {
                    if *serial == 0
                        || *serial >= state.next_generated_body
                        || key != &generated_id(id, *serial)
                        || government.0 < 2
                        || *government > country.government_number
                        || !body.members.is_empty()
                    {
                        return Err("Invalid generated executive identity, mandate or unspecified membership".into());
                    }
                    let name = match (body_kind, party, target) {
                        (ExecutiveBodyKind::Cabinet, Some(party), None) => format!(
                            "{} cabinet {}",
                            content
                                .country(id)
                                .and_then(|country| country.party(party))
                                .ok_or("Generated cabinet has an unknown sponsoring party")?
                                .name,
                            government
                        ),
                        (ExecutiveBodyKind::LeadershipCouncil, None, None) => {
                            format!("Generated leadership council {government}")
                        }
                        (ExecutiveBodyKind::TransitionalAuthority, None, Some(target)) => format!(
                            "Generated transitional {} #{}",
                            content
                                .constitutions
                                .as_ref()
                                .and_then(|rules| rules.template(target))
                                .ok_or("Unknown transitional executive authority")?
                                .title,
                            government
                        ),
                        _ => return Err("Executive body kind and sponsoring party disagree".into()),
                    };
                    if body.name != name || generated.insert(*government, body).is_some() {
                        return Err(
                            "Generated executive label or mandate identity conflicts".into()
                        );
                    }
                }
            }
        }
        if reference_count != usize::from(matches!(initial, ExecutiveHolder::Collective { .. })) {
            return Err("The opening collective identity must remain retained".into());
        }
        let mut previous_body: Option<(u64, CampaignDay)> = None;
        for body in generated.values() {
            let ExecutiveBodyOrigin::Generated { serial, .. } = body.origin else {
                unreachable!("generated index")
            };
            if previous_body.is_some_and(|(previous_serial, day)| {
                serial <= previous_serial || body.created_day < day
            }) {
                return Err("Generated executive allocation order is not monotonic".into());
            }
            previous_body = Some((serial, body.created_day));
        }
        let expected_holder = |government: GovernmentId| {
            generated.range(..=government).next_back().map_or_else(
                || (initial.clone(), start),
                |(_, body)| {
                    (
                        ExecutiveHolder::Collective {
                            body: body.id.clone(),
                        },
                        body.created_day,
                    )
                },
            )
        };
        for (index, tenure) in state.tenures.iter().enumerate() {
            let government = country.government_number.0 - (state.tenures.len() - index - 1) as u32;
            let expected = expected_holder(tenure.government);
            let valid_end = if let Some(next) = state.tenures.get(index + 1) {
                tenure.until_day == Some(next.from_day)
            } else {
                tenure.until_day.is_none()
            };
            if tenure.id != ExecutiveTenureId(government)
                || tenure.government != GovernmentId(government)
                || (tenure.holder.clone(), tenure.holder_since) != expected
                || (tenure.government == GovernmentId(1) && tenure.from_day != start)
                || tenure.from_day < start
                || tenure.from_day > self.day
                || tenure.holder_since > tenure.from_day
                || !valid_end
                || tenure
                    .until_day
                    .is_some_and(|until| until < tenure.from_day || until > self.day)
                || generated
                    .get(&tenure.government)
                    .is_some_and(|body| body.created_day != tenure.from_day)
            {
                return Err(
                    "Invalid executive tenure identity, holder, continuity or dates".into(),
                );
            }
        }
        let expected = expected_holder(country.government_number);
        if (state.holder.clone(), state.holder_since) != expected
            || state.holder_name(content, &state.holder)? != country.executive
        {
            return Err(
                "Current executive presentation does not resolve from its stable holder identity"
                    .into(),
            );
        }
        Ok(())
    }
}
