//! Bounded inspection history. No world state, commands or geometry live here.
use crate::{Tab, map};
use sim_core::{CountryId, PolicyId};

const LIMIT: usize = 64;

#[derive(Clone)]
pub struct View {
    pub tab: Tab,
    pub country: CountryId,
    pub policy: PolicyId,
    pub value: i32,
    pub reference: bool,
    pub map: map::View,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> View {
        View {
            tab: Tab::Overview,
            country: CountryId("GBR".into()),
            policy: "energy".into(),
            value: 90,
            reference: false,
            map: map::WorldMap::new().view(),
        }
    }
    #[test]
    fn visits_preserve_edits_and_forward_branch_but_new_routes_replace_it() {
        let mut history = History::default();
        let mut first = view();
        history.observe(first.clone());
        first.value = 95;
        history.observe(first.clone());
        assert!(!history.can_back());
        let mut second = first.clone();
        second.country = CountryId("DEU".into());
        history.observe(second.clone());
        assert_eq!(history.travel(second.clone(), false).unwrap().value, 95);
        assert!(history.can_forward());
        assert_eq!(
            history.travel(first.clone(), true).unwrap().country,
            second.country
        );
        history.travel(second, false).unwrap();
        let mut third = first.clone();
        third.tab = Tab::Treasury;
        history.observe(third.clone());
        assert!(!history.can_forward());
        assert_eq!(history.views.len(), 2);
        assert!(history.travel(third, false).unwrap().tab == Tab::Overview);
        assert!(history.travel(first, false).is_none());
    }
    #[test]
    fn history_is_bounded_and_guide_modes_are_separate_visits() {
        let mut history = History::default();
        let mut current = view();
        for index in 0..100 {
            current.country = CountryId(format!("reference-{index}"));
            history.observe(current.clone());
        }
        assert_eq!(history.views.len(), LIMIT);
        for _ in 1..LIMIT {
            current = history.travel(current, false).unwrap();
        }
        assert_eq!(current.country.0, "reference-36");
        assert!(!history.can_back());
        current.tab = Tab::Guide;
        current.reference = false;
        history.observe(current.clone());
        current.reference = true;
        history.observe(current.clone());
        assert!(!history.travel(current, false).unwrap().reference);
    }
}
impl View {
    fn same_route(&self, other: &Self) -> bool {
        self.tab == other.tab
            && self.country == other.country
            && self.policy == other.policy
            && self.map.same_reference(&other.map)
            && (self.tab != Tab::Guide || self.reference == other.reference)
    }
}

#[derive(Default)]
pub struct History {
    views: Vec<View>,
    cursor: usize,
}
impl History {
    pub fn can_back(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_forward(&self) -> bool {
        self.cursor + 1 < self.views.len()
    }
    pub fn observe(&mut self, view: View) {
        if self
            .views
            .get(self.cursor)
            .is_some_and(|current| current.same_route(&view))
        {
            // Pan, zoom, layer changes and draft edits update this visit, without creating entries.
            self.views[self.cursor] = view;
        } else {
            self.views.truncate(self.cursor + 1);
            self.views.push(view);
            if self.views.len() > LIMIT {
                self.views.remove(0);
            }
            self.cursor = self.views.len() - 1;
        }
    }
    pub fn travel(&mut self, current: View, forward: bool) -> Option<View> {
        self.observe(current);
        if forward {
            if !self.can_forward() {
                return None;
            }
            self.cursor += 1;
        } else {
            if !self.can_back() {
                return None;
            }
            self.cursor -= 1;
        }
        Some(self.views[self.cursor].clone())
    }
}
