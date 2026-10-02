//! 同步状态机：增量 FETCH、ops_queue 重放、IDLE / 29min NOOP。

use chck_types::{AccountId, EngineError, FolderId, SyncState};
use std::time::{Duration, Instant};

pub const BACKOFF_MIN: Duration = Duration::from_secs(60);
pub const BACKOFF_MAX: Duration = Duration::from_secs(30 * 60);
pub const IDLE_KEEPALIVE: Duration = Duration::from_secs(29 * 60);

pub struct SyncEngine {
    pub state: SyncState,
    pub account_id: Option<AccountId>,
    folders: Vec<FolderId>,
    next: usize,
    pub supports_idle: bool,
    backoff: Duration,
    backoff_until: Option<Instant>,
}

impl Default for SyncEngine {
    fn default() -> Self {
        Self {
            state: SyncState::Idle,
            account_id: None,
            folders: Vec::new(),
            next: 0,
            supports_idle: false,
            backoff: BACKOFF_MIN,
            backoff_until: None,
        }
    }
}

impl SyncEngine {
    pub fn tick(&mut self, now: Instant) -> Result<(), EngineError> {
        match self.state {
            SyncState::Backoff if self.backoff_until.is_none_or(|t| now >= t) => {
                self.state = SyncState::Connecting;
            }
            SyncState::Idle | SyncState::Done | SyncState::Idling => {
                self.state = SyncState::Connecting;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn bind(&mut self, account_id: AccountId) {
        self.account_id = Some(account_id);
    }

    pub fn connected(&mut self) {
        self.state = SyncState::Discovering;
        self.backoff = BACKOFF_MIN;
        self.backoff_until = None;
    }

    pub fn discovered(&mut self, folders: Vec<FolderId>, supports_idle: bool) {
        self.folders = folders;
        self.next = 0;
        self.supports_idle = supports_idle;
        self.state = SyncState::SyncFolders;
    }

    pub fn next_folder(&mut self) -> Option<FolderId> {
        if self.next >= self.folders.len() {
            self.state = SyncState::Done;
            return None;
        }
        self.state = SyncState::SyncFolder;
        let id = self.folders[self.next].clone();
        self.next += 1;
        Some(id)
    }

    pub fn finish(&mut self, desktop_idle: bool) {
        self.state = if desktop_idle { SyncState::Idling } else { SyncState::Idle };
    }

    pub fn idle_event(&mut self) {
        self.next = 0;
        if self.folders.is_empty() {
            self.state = SyncState::Done;
        } else {
            self.state = SyncState::SyncFolder;
        }
    }

    pub fn fail(&mut self, now: Instant) {
        self.state = SyncState::Backoff;
        self.backoff_until = Some(now + self.backoff);
        self.backoff = (self.backoff * 2).min(BACKOFF_MAX);
    }

    #[must_use]
    pub fn inbox(&self) -> Option<&FolderId> {
        self.folders.first()
    }

    #[must_use]
    pub fn backoff_delay(&self) -> Duration {
        self.backoff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_idle() {
        assert_eq!(SyncEngine::default().state, SyncState::Idle);
    }

    #[test]
    fn walks_folders_then_idles_on_desktop() {
        let mut sync = SyncEngine::default();
        sync.bind(AccountId::from("a1"));
        sync.tick(Instant::now()).unwrap();
        assert_eq!(sync.state, SyncState::Connecting);
        sync.connected();
        assert_eq!(sync.state, SyncState::Discovering);
        sync.discovered(vec![FolderId::from("inbox"), FolderId::from("sent")], true);
        assert_eq!(sync.next_folder().unwrap().as_str(), "inbox");
        assert_eq!(sync.state, SyncState::SyncFolder);
        assert_eq!(sync.next_folder().unwrap().as_str(), "sent");
        assert!(sync.next_folder().is_none());
        assert_eq!(sync.state, SyncState::Done);
        sync.finish(true);
        assert_eq!(sync.state, SyncState::Idling);
        sync.idle_event();
        assert_eq!(sync.state, SyncState::SyncFolder);
    }

    #[test]
    fn mobile_returns_to_idle_without_hang() {
        let mut sync = SyncEngine::default();
        sync.tick(Instant::now()).unwrap();
        sync.connected();
        sync.discovered(vec![FolderId::from("inbox")], false);
        while sync.next_folder().is_some() {}
        sync.finish(false);
        assert_eq!(sync.state, SyncState::Idle);
    }

    #[test]
    fn backoff_doubles_until_cap_then_reconnects() {
        let mut sync = SyncEngine::default();
        let t0 = Instant::now();
        sync.fail(t0);
        assert_eq!(sync.state, SyncState::Backoff);
        assert_eq!(sync.backoff_delay(), BACKOFF_MIN * 2);
        sync.tick(t0).unwrap();
        assert_eq!(sync.state, SyncState::Backoff);
        sync.tick(t0 + BACKOFF_MIN).unwrap();
        assert_eq!(sync.state, SyncState::Connecting);
        for _ in 0..8 {
            sync.fail(t0);
        }
        assert_eq!(sync.backoff_delay(), BACKOFF_MAX);
    }
}
