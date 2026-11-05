//! Client scheduling only; simulation steps remain ordered and wall-clock free.
use sim_core::{Content, DaySpan, MAX_DAY, World};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};
pub struct Outcome {
    pub world: World,
    pub error: Option<String>,
    pub stopped_for_event: bool,
}
pub struct AdvanceJob {
    receiver: Receiver<Outcome>,
    stop: Arc<AtomicBool>,
    pub completed: Arc<AtomicU32>,
    pub requested: DaySpan,
    pub seeking_event: bool,
    pub start_day: u32,
}
impl AdvanceJob {
    pub fn start(
        mut world: World,
        content: Content,
        elapsed: DaySpan,
        important: bool,
    ) -> Result<Self, String> {
        let days = elapsed.units();
        if world.day > MAX_DAY || days > MAX_DAY.saturating_sub(world.day) {
            return Err("Requested date exceeds the 100-year campaign limit".into());
        }
        if world.content_hash != content.hash() {
            return Err("Campaign content mismatch".into());
        }
        world.validate(&content)?;
        let start_day = world.day;
        let (sender, receiver) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicU32::new(0));
        let cancel = stop.clone();
        let progress = completed.clone();
        std::thread::spawn(move || {
            let mut error = None;
            let mut stopped_for_event = false;
            for _ in 0..days {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Err(reason) = world.advance(&content, sim_core::DaySpan::new(1)) {
                    error = Some(reason);
                    break;
                }
                progress.store(world.day - start_day, Ordering::Relaxed);
                if important
                    && world
                        .events
                        .iter()
                        .any(|e| e.day == world.day && e.country == world.player && e.important)
                {
                    stopped_for_event = true;
                    break;
                }
            }
            let _ = sender.send(Outcome {
                world,
                error,
                stopped_for_event,
            });
        });
        Ok(Self {
            receiver,
            stop,
            completed,
            requested: elapsed,
            seeking_event: false,
            start_day,
        })
    }
    pub fn poll(&self) -> Result<Option<Outcome>, String> {
        match self.receiver.try_recv() {
            Ok(outcome) => Ok(Some(outcome)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err("Simulation worker ended unexpectedly; active campaign preserved".into())
            }
        }
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
    pub fn is_stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}
impl Drop for AdvanceJob {
    fn drop(&mut self) {
        self.stop();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{Action, CountryId};
    #[test]
    fn important_boundaries_stop_a_long_advance_and_replay_exactly() {
        let c = Content::bundled().unwrap();
        let mut w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
        w.issue(
            &c,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
        let job = AdvanceJob::start(w, c.clone(), DaySpan::new(365), true).unwrap();
        let outcome = job
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        assert!(outcome.error.is_none());
        assert!(outcome.world.day <= 14);
        assert!(
            outcome
                .world
                .events
                .iter()
                .any(|e| e.day == outcome.world.day
                    && e.country == outcome.world.player
                    && e.important)
        );
        sim_core::persistence::replay(&outcome.world, &c).unwrap();
    }
    #[test]
    fn worker_matches_the_same_number_of_synchronous_steps() {
        let c = Content::bundled().unwrap();
        let w = World::new(&c, CountryId("GBR".into()), 6).unwrap();
        let job = AdvanceJob::start(w.clone(), c.clone(), DaySpan::new(70), false).unwrap();
        let outcome = job
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let mut sync = w;
        sync.advance(&c, sim_core::DaySpan::new(70)).unwrap();
        assert_eq!(outcome.world.hash(), sync.hash());
    }
    #[test]
    fn an_important_event_on_the_final_requested_day_still_requests_pause() {
        let mut c = Content::bundled().unwrap();
        c.countries
            .iter_mut()
            .find(|d| d.id.0 == "GBR")
            .unwrap()
            .government
            .consent_days = 30;
        let seed = (0_u64..100)
            .find(|seed| {
                !sim_core::stable_hash(format!("{seed}:GBR:0:supply-v2").as_bytes())
                    .is_multiple_of(3)
            })
            .unwrap();
        let mut w = World::new(&c, CountryId("GBR".into()), seed).unwrap();
        w.issue(
            &c,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
        let job = AdvanceJob::start(w, c, DaySpan::new(30), true).unwrap();
        let result = job
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        assert_eq!(result.world.day, 30);
        assert!(result.stopped_for_event);
    }
    #[test]
    fn later_bookmarks_preserve_elapsed_requests_and_worker_progress() {
        let c =
            Content::from_json(include_str!("../../../data/synthetic-dated-test.json")).unwrap();
        let mut expected = World::new(&c, CountryId("AUR".into()), 17).unwrap();
        let start = expected.day;
        assert!(start > 0);
        let elapsed = DaySpan::new(3);
        let job = AdvanceJob::start(expected.clone(), c.clone(), elapsed, false).unwrap();
        assert_eq!(job.start_day, start);
        assert_eq!(job.requested, elapsed);
        let outcome = job
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        assert!(outcome.error.is_none());
        assert_eq!(outcome.world.day, start + elapsed.units());
        assert_eq!(job.completed.load(Ordering::Relaxed), elapsed.units());
        expected.advance(&c, elapsed).unwrap();
        assert_eq!(
            sim_core::persistence::encode(&outcome.world, &c).unwrap(),
            sim_core::persistence::encode(&expected, &c).unwrap()
        );
        assert_eq!(
            outcome.world,
            sim_core::persistence::replay(&outcome.world, &c).unwrap()
        );
    }
    #[test]
    fn cancellation_commits_only_completed_day_boundaries() {
        let c = Content::bundled().unwrap();
        let w = World::new(&c, CountryId("GBR".into()), 6).unwrap();
        let job = AdvanceJob::start(w.clone(), c.clone(), DaySpan::new(1000), false).unwrap();
        job.stop();
        let result = job
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let mut expected = w;
        expected
            .advance(&c, DaySpan::new(result.world.day - expected.day))
            .unwrap();
        assert!(result.world.day < 1000);
        assert_eq!(result.world.hash(), expected.hash());
    }
}
