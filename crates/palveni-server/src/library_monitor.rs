//! Coalesce filesystem hints into the same bounded job queue as manual scans.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use palveni_core::{EventBus, JobId, Library, LibraryId, ServerEvent};
use palveni_db::Db;
use palveni_jobs::JobService;
use palveni_library::watch::LocalWatcher;
use tokio::time::{Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

const TICK: Duration = Duration::from_millis(500);
const QUIET: Duration = Duration::from_millis(1500);
const MAX_BATCH: Duration = Duration::from_secs(10);
const RETRY_JOB: Duration = Duration::from_secs(3);
const RETRY_WATCH: Duration = Duration::from_secs(30);
const RECONCILE: Duration = Duration::from_secs(15 * 60);

struct Pending {
    first: Instant,
    due: Instant,
    waiting: Option<JobId>,
}

impl Pending {
    fn new(now: Instant) -> Self {
        Self {
            first: now,
            due: now + QUIET,
            waiting: None,
        }
    }

    fn touch(&mut self, now: Instant) {
        if self.waiting.is_none() {
            self.due = (now + QUIET).min(self.first + MAX_BATCH);
        }
    }
}

#[derive(Default)]
struct Monitor {
    libraries: BTreeMap<LibraryId, Library>,
    pending: BTreeMap<LibraryId, Pending>,
    watcher: LocalWatcher,
    rebuild: bool,
}

impl Monitor {
    fn changed(&mut self, id: LibraryId, now: Instant) {
        if self.libraries.contains_key(&id) {
            self.pending
                .entry(id)
                .and_modify(|p| p.touch(now))
                .or_insert_with(|| Pending::new(now));
        }
    }

    fn library(&mut self, lib: Library, now: Instant) {
        if lib.last_scan_at.is_none() {
            return;
        }
        let id = lib.id;
        let new_paths = self.libraries.get(&id).is_none_or(|l| l.paths != lib.paths);
        self.libraries.insert(id, lib);
        if new_paths {
            self.rebuild = true;
            // Reconcile the gap before watch installation, including time spent offline.
            self.changed(id, now);
        }
    }

    fn remove(&mut self, id: LibraryId) {
        if self.libraries.remove(&id).is_some() {
            self.rebuild = true;
        }
        self.pending.remove(&id);
    }

    async fn refresh(&mut self, db: &Db, now: Instant) -> Result<(), palveni_db::DbError> {
        let current = db.libraries().list().await?;
        let live: std::collections::BTreeSet<_> = current
            .iter()
            .filter(|l| l.last_scan_at.is_some())
            .map(|l| l.id)
            .collect();
        let removed: Vec<_> = self
            .libraries
            .keys()
            .filter(|id| !live.contains(*id))
            .copied()
            .collect();
        for id in removed {
            self.remove(id);
        }
        for lib in current {
            self.library(lib, now);
        }
        Ok(())
    }

    fn drain(&mut self, now: Instant) -> bool {
        let changes = self.watcher.drain();
        for id in changes.libraries {
            self.changed(id, now);
        }
        changes.rearm
    }

    async fn install(&mut self, data: PathBuf) {
        let roots = self
            .libraries
            .values()
            .flat_map(|l| l.paths.iter().map(|root| (l.id, PathBuf::from(root))))
            .collect();
        match tokio::task::spawn_blocking(move || LocalWatcher::open(roots, data)).await {
            Ok(new) => {
                let now = Instant::now();
                // The old watcher continues to collect hints during recursive registration.
                self.drain(now);
                let recovered: Vec<_> = self
                    .watcher
                    .unavailable
                    .difference(&new.unavailable)
                    .copied()
                    .collect();
                for id in new.unavailable.difference(&self.watcher.unavailable) {
                    tracing::warn!(library=%id, "native directory watch unavailable; periodic scans remain enabled");
                }
                self.watcher = new;
                for id in recovered {
                    self.changed(id, now);
                }
                self.rebuild = false;
            }
            Err(e) => tracing::warn!(error=%e,"could not install library notifications"),
        }
    }

    async fn submit_due(&mut self, db: &Db, jobs: &JobService, now: Instant) {
        let due: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, p)| p.due <= now)
            .map(|(id, _)| *id)
            .collect();
        for id in due {
            let p = self
                .pending
                .get_mut(&id)
                .expect("pending entry was just selected");
            p.due = now + RETRY_JOB;
            if let Some(job_id) = p.waiting {
                match db.jobs().get(job_id).await {
                    Ok(Some(job)) if !job.job.state.is_terminal() => continue,
                    Err(e) => {
                        tracing::warn!(error=%e,"could not read pending library scan");
                        continue;
                    }
                    _ => p.waiting = None,
                }
            }
            let Some(lib) = self.libraries.get(&id) else {
                continue;
            };
            match jobs.enqueue_library_scan(id, lib.name.clone()).await {
                Ok((_, true)) => {
                    self.pending.remove(&id);
                }
                // A scan already in progress may have visited the changed directory.
                // Keep one trailing scan until a new job has actually been created.
                Ok((job, false)) => p.waiting = Some(job.id),
                Err(e) => {
                    tracing::warn!(library=%id,error=%e,"could not queue library reconciliation")
                }
            }
        }
    }
}

pub async fn run(
    db: Db,
    jobs: JobService,
    bus: EventBus,
    data: PathBuf,
    shutdown: CancellationToken,
) {
    let mut events = bus.subscribe();
    let mut cursor = events.after(None).latest;
    let mut monitor = Monitor::default();
    let mut reload = true;
    let mut refresh_after = Instant::now();
    let mut rearm_after = Instant::now();
    let mut rearm = false;
    let mut fallback = Instant::now() + RECONCILE;
    let mut ticks = tokio::time::interval(TICK);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! { biased; ()=shutdown.cancelled()=>break, _=ticks.tick()=>() }
        let now = Instant::now();
        let replay = events.after(Some(cursor));
        cursor = replay.latest;
        reload |= replay.reset;
        for event in replay.entries {
            match &event.event {
                ServerEvent::LibraryUpdated(l) => monitor.library(l.clone(), now),
                ServerEvent::LibraryDeleted { id } => monitor.remove(*id),
                _ => (),
            }
        }
        if reload && now >= refresh_after {
            reload = monitor
                .refresh(&db, now)
                .await
                .inspect_err(|e| tracing::warn!(error=%e,"could not refresh monitored libraries"))
                .is_err();
            refresh_after = now + RETRY_JOB;
        }
        rearm |= monitor.drain(now);
        if monitor.rebuild
            || (now >= rearm_after && (rearm || !monitor.watcher.unavailable.is_empty()))
        {
            monitor.install(data.clone()).await;
            rearm_after = now + RETRY_WATCH;
            rearm = false;
        }
        if now >= fallback {
            let ids: Vec<_> = monitor.libraries.keys().copied().collect();
            for id in ids {
                monitor.changed(id, now);
            }
            fallback = now + RECONCILE;
        }
        monitor.submit_due(&db, &jobs, now).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debounce_has_a_deadline_and_keeps_one_trailing_job() {
        let start = Instant::now();
        let mut pending = Pending::new(start);
        for n in 1..18 {
            pending.touch(start + Duration::from_secs(n));
        }
        assert_eq!(pending.due, start + MAX_BATCH);
        pending.waiting = Some(JobId::generate());
        pending.due = start + RETRY_JOB;
        pending.touch(start + Duration::from_secs(10));
        assert_eq!(pending.due, start + RETRY_JOB);
    }
}
