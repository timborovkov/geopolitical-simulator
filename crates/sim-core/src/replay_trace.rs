//! Opt-in developer replay evidence, separate from authoritative snapshots.
use crate::{CampaignDay, CommandId, Content, CountryId, Scenario, World};
use serde::{Deserialize, Serialize, ser::SerializeMap};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, Read, Write},
};

pub const TRACE_VERSION: u32 = 1;
const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayTraceIdentity {
    pub rules: u32,
    pub content_hash: u64,
    pub seed: u64,
    pub scenario: Scenario,
    pub player: CountryId,
    pub start_day: CampaignDay,
    pub end_day: CampaignDay,
    pub manual_input_hash: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayTraceHeader {
    pub version: u32,
    pub identity: ReplayTraceIdentity,
    pub expected_snapshot_hash: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "command", rename_all = "snake_case")]
pub enum ReplayBoundary {
    Opening,
    Day,
    Command(CommandId),
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReplayDomain {
    Funds,
    Households,
    Physical,
    Governance,
    Delivery,
    Reports,
    Policies,
    Other,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum ReplayComponent {
    Control,
    Commands,
    Events,
    Ledger,
    Country {
        country: CountryId,
        domain: ReplayDomain,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayComponentHash {
    pub component: ReplayComponent,
    pub hash: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayTracePoint {
    pub sequence: u64,
    pub day: CampaignDay,
    pub boundary: ReplayBoundary,
    pub world_hash: u64,
    pub components: Vec<ReplayComponentHash>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReplayTraceOutcome {
    Complete {
        final_hash: u64,
        snapshot_matches: bool,
    },
    Failed {
        error: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayTraceFooter {
    pub points: u64,
    pub outcome: ReplayTraceOutcome,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "record", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplayTraceRecord {
    Header { header: ReplayTraceHeader },
    Point { point: ReplayTracePoint },
    Footer { footer: ReplayTraceFooter },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayTraceSummary {
    pub header: ReplayTraceHeader,
    pub footer: ReplayTraceFooter,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayDivergence {
    pub sequence: u64,
    pub expected: Option<(CampaignDay, ReplayBoundary, u64)>,
    pub actual: Option<(CampaignDay, ReplayBoundary, u64)>,
    pub components: Vec<ReplayComponent>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayTraceComparison {
    pub compared_points: u64,
    pub first_difference: Option<ReplayDivergence>,
    pub expected_outcome: ReplayTraceOutcome,
    pub actual_outcome: ReplayTraceOutcome,
}

// Sort object keys explicitly, including when serde_json uses insertion order.
// Array order remains significant, as it does in canonical world snapshots.
struct OrderedValue<'a>(&'a Value);
impl Serialize for OrderedValue<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Object(values) => {
                let mut entries: Vec<_> = values.iter().collect();
                entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, &OrderedValue(value))?;
                }
                map.end()
            }
            Value::Array(values) => serializer.collect_seq(values.iter().map(OrderedValue)),
            other => other.serialize(serializer),
        }
    }
}
fn value_hash(value: &Value) -> Result<u64, String> {
    serde_json::to_vec(&OrderedValue(value))
        .map(|bytes| crate::stable_hash(&bytes))
        .map_err(|e| e.to_string())
}
fn take_group(map: &mut Map<String, Value>, names: &[&str]) -> Value {
    Value::Object(
        names
            .iter()
            .filter_map(|name| map.remove(*name).map(|value| ((*name).into(), value)))
            .collect(),
    )
}
/// Hash live host truth for developer inspection. Normal player projections do
/// not call this function. Every field is covered; future fields enter Other.
pub fn component_hashes(world: &World) -> Result<Vec<ReplayComponentHash>, String> {
    let Value::Object(mut root) = serde_json::to_value(world).map_err(|e| e.to_string())? else {
        return Err("World must serialize as an object".into());
    };
    let mut components = Vec::new();
    for (component, names) in [
        (ReplayComponent::Commands, &["journal"][..]),
        (ReplayComponent::Events, &["events", "event_sequences"][..]),
        (ReplayComponent::Ledger, &["ledger"][..]),
    ] {
        components.push(ReplayComponentHash {
            component,
            hash: value_hash(&take_group(&mut root, names))?,
        });
    }
    let Value::Object(countries) = root.remove("countries").ok_or("Missing world countries")?
    else {
        return Err("World countries must serialize as an object".into());
    };
    for (id, country) in countries {
        let Value::Object(mut fields) = country else {
            return Err("Country must serialize as an object".into());
        };
        let groups: &[(ReplayDomain, &[&str])] = &[
            (
                ReplayDomain::Funds,
                &[
                    "accounts",
                    "opening_money",
                    "debt",
                    "committed_cash",
                    "arrears",
                    "arrears_households",
                    "arrears_firms",
                    "arrears_external",
                    "budget",
                    "budget_reviews",
                    "financing",
                    "cash_management",
                ],
            ),
            (ReplayDomain::Households, &["population", "households"]),
            (
                ReplayDomain::Physical,
                &[
                    "energy_stock",
                    "food_stock",
                    "energy_capacity",
                    "food_capacity",
                    "opening_energy_capacity",
                    "opening_food_capacity",
                    "markets",
                    "reserves",
                    "industrial_trade",
                ],
            ),
            (
                ReplayDomain::Governance,
                &[
                    "id",
                    "ruling_party",
                    "coalition",
                    "seats",
                    "executive",
                    "government_number",
                    "next_election",
                    "next_leadership_review",
                    "council_support",
                    "confidence_crises",
                    "legislative_support",
                    "elite_support",
                    "approval",
                    "constitution",
                    "electoral_authority",
                    "cabinet",
                    "subnational",
                    "territorial",
                ],
            ),
            (
                ReplayDomain::Delivery,
                &["projects", "commitments", "undertakings"],
            ),
            (
                ReplayDomain::Reports,
                &[
                    "report",
                    "history",
                    "statistical_archive",
                    "income_tax_report",
                ],
            ),
            (
                ReplayDomain::Policies,
                &["tax", "services", "welfare", "administration"],
            ),
        ];
        for &(domain, names) in groups {
            components.push(ReplayComponentHash {
                component: ReplayComponent::Country {
                    country: CountryId(id.clone()),
                    domain,
                },
                hash: value_hash(&take_group(&mut fields, names))?,
            });
        }
        components.push(ReplayComponentHash {
            component: ReplayComponent::Country {
                country: CountryId(id),
                domain: ReplayDomain::Other,
            },
            hash: value_hash(&Value::Object(fields))?,
        });
    }
    components.push(ReplayComponentHash {
        component: ReplayComponent::Control,
        hash: value_hash(&Value::Object(root))?,
    });
    components.sort_by(|a, b| a.component.cmp(&b.component));
    Ok(components)
}
fn write_record(writer: &mut impl Write, record: &ReplayTraceRecord) -> Result<(), String> {
    let bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
    if bytes.len() + 1 > MAX_LINE_BYTES {
        return Err("Replay trace record exceeds the 1 MiB limit".into());
    }
    writer
        .write_all(&bytes)
        .and_then(|()| writer.write_all(b"\n"))
        .map_err(|e| e.to_string())
}
/// Stream a detached replay, retaining no full-state history in memory. The
/// original campaign is immutable. A replay failure is recorded in the footer;
/// capture/I/O failures return Err and leave an explicitly incomplete stream.
pub fn write_replay_trace(
    original: &World,
    content: &Content,
    writer: &mut impl Write,
) -> Result<ReplayTraceSummary, String> {
    original.validate(content)?;
    let inputs: Vec<_> = original
        .journal
        .iter()
        .filter(|entry| !entry.ai && entry.budget_bill.is_none())
        .collect();
    let header = ReplayTraceHeader {
        version: TRACE_VERSION,
        identity: ReplayTraceIdentity {
            rules: original.rules,
            content_hash: original.content_hash,
            seed: original.seed,
            scenario: original.scenario,
            player: original.player.clone(),
            start_day: content.start_day()?,
            end_day: original.day,
            manual_input_hash: crate::stable_hash(
                &serde_json::to_vec(&inputs).map_err(|e| e.to_string())?,
            ),
        },
        expected_snapshot_hash: original.hash(),
    };
    write_record(
        writer,
        &ReplayTraceRecord::Header {
            header: header.clone(),
        },
    )?;
    let mut points = 0_u64;
    let mut capture_error = None;
    let result =
        crate::persistence::replay_observed(original, content, false, |world, boundary| {
            let captured: Result<(), String> = (|| {
                let sequence = points
                    .checked_add(1)
                    .ok_or("Replay trace point counter overflow")?;
                let point = ReplayTracePoint {
                    sequence,
                    day: world.day,
                    boundary,
                    world_hash: world.hash(),
                    components: component_hashes(world)?,
                };
                write_record(writer, &ReplayTraceRecord::Point { point })?;
                points = sequence;
                Ok(())
            })();
            if let Err(error) = &captured {
                capture_error = Some(error.clone());
            }
            captured
        });
    if let Some(error) = capture_error {
        return Err(error);
    }
    let outcome = match result {
        Ok(world) => {
            let final_hash = world.hash();
            ReplayTraceOutcome::Complete {
                final_hash,
                snapshot_matches: final_hash == header.expected_snapshot_hash,
            }
        }
        Err(error) => ReplayTraceOutcome::Failed { error },
    };
    let footer = ReplayTraceFooter { points, outcome };
    write_record(
        writer,
        &ReplayTraceRecord::Footer {
            footer: footer.clone(),
        },
    )?;
    writer.flush().map_err(|e| e.to_string())?;
    Ok(ReplayTraceSummary { header, footer })
}

struct TraceReader<R> {
    reader: R,
    header: ReplayTraceHeader,
    points: u64,
    last: Option<(CampaignDay, u64)>,
    last_command: CommandId,
    footer: Option<ReplayTraceFooter>,
}
fn read_record(reader: &mut impl BufRead) -> Result<Option<ReplayTraceRecord>, String> {
    let mut line = String::new();
    let size = reader
        .take(MAX_LINE_BYTES as u64 + 1)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    if size == 0 {
        return Ok(None);
    }
    if size > MAX_LINE_BYTES {
        return Err("Replay trace record exceeds the 1 MiB limit".into());
    }
    serde_json::from_str(&line)
        .map(Some)
        .map_err(|e| format!("Invalid replay trace record: {e}"))
}
impl<R: BufRead> TraceReader<R> {
    fn new(mut reader: R) -> Result<Self, String> {
        let Some(ReplayTraceRecord::Header { header }) = read_record(&mut reader)? else {
            return Err("Missing replay trace header".into());
        };
        if header.version != TRACE_VERSION {
            return Err(format!(
                "Unsupported replay trace version {}",
                header.version
            ));
        }
        if header.identity.start_day > header.identity.end_day
            || header.identity.end_day.units() > crate::MAX_DAY
        {
            return Err("Invalid replay trace calendar bounds".into());
        }
        Ok(Self {
            reader,
            header,
            points: 0,
            last: None,
            last_command: CommandId(0),
            footer: None,
        })
    }
    fn next_point(&mut self) -> Result<Option<ReplayTracePoint>, String> {
        if self.footer.is_some() {
            return Ok(None);
        }
        match read_record(&mut self.reader)? {
            Some(ReplayTraceRecord::Point { point }) => {
                if point.sequence
                    != self
                        .points
                        .checked_add(1)
                        .ok_or("Replay trace point counter overflow")?
                    || point.day < self.header.identity.start_day
                    || point.day > self.header.identity.end_day
                    || point
                        .components
                        .windows(2)
                        .any(|pair| pair[0].component >= pair[1].component)
                    || point.components.is_empty()
                {
                    return Err("Invalid replay trace point or component ordering".into());
                }
                let valid = match (&point.boundary, self.last) {
                    (ReplayBoundary::Opening, None) => point.day == self.header.identity.start_day,
                    (ReplayBoundary::Day, Some((day, _))) => {
                        day.checked_add(crate::DaySpan::new(1)) == Some(point.day)
                    }
                    (ReplayBoundary::Command(id), Some((day, _))) => {
                        point.day == day && *id > self.last_command
                    }
                    _ => false,
                };
                if !valid {
                    return Err("Invalid replay trace boundary sequence".into());
                }
                if let ReplayBoundary::Command(id) = point.boundary {
                    self.last_command = id;
                }
                self.points = point.sequence;
                self.last = Some((point.day, point.world_hash));
                Ok(Some(point))
            }
            Some(ReplayTraceRecord::Footer { footer }) => {
                if footer.points != self.points {
                    return Err("Replay trace footer count disagrees with its points".into());
                }
                match &footer.outcome {
                    ReplayTraceOutcome::Complete {
                        final_hash,
                        snapshot_matches,
                    } => {
                        if self.last != Some((self.header.identity.end_day, *final_hash))
                            || *snapshot_matches
                                != (*final_hash == self.header.expected_snapshot_hash)
                        {
                            return Err("Invalid completed replay trace footer".into());
                        }
                    }
                    ReplayTraceOutcome::Failed { error } if error.is_empty() => {
                        return Err("Failed replay trace needs its reason".into());
                    }
                    ReplayTraceOutcome::Failed { .. } => {}
                }
                if read_record(&mut self.reader)?.is_some() {
                    return Err("Unexpected records after replay trace footer".into());
                }
                self.footer = Some(footer);
                Ok(None)
            }
            Some(ReplayTraceRecord::Header { .. }) => {
                Err("Unexpected second replay trace header".into())
            }
            None => Err("Replay trace ended before its footer".into()),
        }
    }
}
fn differing_components(a: &ReplayTracePoint, b: &ReplayTracePoint) -> Vec<ReplayComponent> {
    let a: BTreeMap<_, _> = a
        .components
        .iter()
        .map(|entry| (&entry.component, entry.hash))
        .collect();
    let b: BTreeMap<_, _> = b
        .components
        .iter()
        .map(|entry| (&entry.component, entry.hash))
        .collect();
    let keys: BTreeSet<_> = a.keys().chain(b.keys()).copied().collect();
    keys.into_iter()
        .filter(|key| a.get(key) != b.get(key))
        .cloned()
        .collect()
}
/// Compare matching input identities through complete streams, returning the
/// earliest daily/command boundary and all changed components at that boundary.
/// Snapshot agreement is reported separately from equality of the executions.
pub fn compare_replay_traces(
    expected: impl BufRead,
    actual: impl BufRead,
) -> Result<ReplayTraceComparison, String> {
    let mut a = TraceReader::new(expected)?;
    let mut b = TraceReader::new(actual)?;
    if a.header.identity != b.header.identity {
        return Err("Replay traces have different rules, content, seed, player, scenario, target date or manual inputs".into());
    }
    let mut compared_points = 0;
    let mut first_difference = None;
    loop {
        let left = a.next_point()?;
        let right = b.next_point()?;
        if left.is_none() && right.is_none() {
            break;
        }
        let components = match (&left, &right) {
            (Some(x), Some(y)) => differing_components(x, y),
            _ => Vec::new(),
        };
        if let (Some(_), Some(_)) = (&left, &right) {
            compared_points += 1;
        }
        if first_difference.is_none()
            && (left.as_ref().map(|p| (p.day, &p.boundary, p.world_hash))
                != right.as_ref().map(|p| (p.day, &p.boundary, p.world_hash))
                || !components.is_empty())
        {
            first_difference = Some(ReplayDivergence {
                sequence: left
                    .as_ref()
                    .or(right.as_ref())
                    .expect("one point")
                    .sequence,
                expected: left.map(|p| (p.day, p.boundary, p.world_hash)),
                actual: right.map(|p| (p.day, p.boundary, p.world_hash)),
                components,
            });
        }
    }
    Ok(ReplayTraceComparison {
        compared_points,
        first_difference,
        expected_outcome: a.footer.expect("validated footer").outcome,
        actual_outcome: b.footer.expect("validated footer").outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn component_hashes_sort_objects_but_preserve_array_order_and_values() {
        let a: Value = serde_json::from_str(r#"{"b":[1,2],"a":{"y":2,"x":1}}"#).unwrap();
        let b: Value = serde_json::from_str(r#"{"a":{"x":1,"y":2},"b":[1,2]}"#).unwrap();
        assert_eq!(value_hash(&a).unwrap(), value_hash(&b).unwrap());
        assert_ne!(
            value_hash(&a).unwrap(),
            value_hash(&serde_json::json!({"a":{"x":1,"y":2},"b":[2,1]})).unwrap()
        );
    }
}
