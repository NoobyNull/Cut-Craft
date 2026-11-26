//! What Relief from Photo keeps while the Depth add-on works: the estimate running, the
//! High detail tick, and the High detail model's download or why it failed.

use crate::depth::{Estimating, FetchingDetail};

#[derive(Default)]
pub(super) struct PhotoDepth {
    /// The estimate running, while the Depth add-on works on a picture.
    pub(super) estimate: Option<Estimating>,
    /// The High detail tick: unset means on once the model is fetched.
    high_detail: Option<bool>,
    /// The High detail model downloading.
    fetch: Option<FetchingDetail>,
    /// Why its last download failed.
    failed: Option<String>,
}

impl PhotoDepth {
    /// An estimate is running.
    pub(super) fn estimating(&self) -> bool {
        self.estimate.is_some()
    }

    /// The estimate and its answer, once the add-on has answered; nothing runs after.
    pub(super) fn take_answer(&mut self) -> Option<(Estimating, Result<Vec<f32>, String>)> {
        let answer = self.estimate.as_ref()?.answer()?;
        Some((self.estimate.take()?, answer))
    }

    /// Whether High detail is used: as ticked, else whenever the model is here (`have`).
    pub(super) fn high_detail(&self, have: bool) -> bool {
        self.high_detail.unwrap_or(have)
    }

    /// High detail ticked or unticked.
    pub(super) fn set_high_detail(&mut self, on: bool) {
        self.high_detail = Some(on);
    }

    /// Takes the High detail download's answer, once there is one.
    pub(super) fn check_fetch(&mut self) {
        if let Some(answer) = self.fetch.as_ref().and_then(FetchingDetail::answer) {
            self.fetch = None;
            self.fetched(answer);
        }
    }

    /// How far the High detail download has got, 0 to 1, while it runs.
    pub(super) fn fetching(&self) -> Option<f32> {
        self.fetch.as_ref().map(FetchingDetail::fraction)
    }

    /// Why the last High detail download failed, if it did.
    pub(super) fn failed(&self) -> Option<&str> {
        self.failed.as_deref()
    }

    /// The High detail download started, or why it couldn't.
    pub(super) fn fetch_started(&mut self, started: Result<FetchingDetail, String>) {
        match started {
            Ok(fetch) => (self.fetch, self.failed) = (Some(fetch), None),
            Err(e) => self.failed = Some(e),
        }
    }

    /// The High detail download finished: once here, the model is used.
    fn fetched(&mut self, answer: Result<(), String>) {
        match answer {
            Ok(()) => (self.failed, self.high_detail) = (None, Some(true)),
            Err(e) => self.failed = Some(e),
        }
    }
}

#[cfg(test)]
mod tests;
