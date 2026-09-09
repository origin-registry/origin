//! The blob-reclamation marker protocol: the one place a writer and the
//! collector must agree, with no lock.
//!
//! A collector about to delete blob data publishes a [`GcRun`] naming its
//! digest range, re-reads that marker before the irreversible delete, and
//! expires it afterwards; a writer that has just written reference keys lists
//! `v2/gc/` once and backs off on an unexpired run covering one of its
//! digests. Either the writer's reference completed before the collector's
//! liveness listing (the key is younger than the grace period, so the blob
//! reads live), or it completed after, in which case the marker was still
//! visible to the writer's check.
//!
//! That last part is why a finished run expires its marker instead of removing
//! it: a writer whose reference landed after the run's last liveness listing
//! would otherwise find nothing and commit onto reclaimed bytes. The expiry
//! also stops a crashed collector from wedging writers, since a live one
//! fences itself by refreshing before each delete.

use std::time::Duration as StdDuration;

use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use uuid::Uuid;

use angos_backoff::Backoff;

use angos_oci::Digest;
use angos_storage::Error as StorageError;

use crate::registry::{Error, keys::GC_ROOT, metadata_store::MetadataStore};

/// How long a released marker lingers before it expires. Removing it outright
/// would let a writer whose reference key landed after the run's last liveness
/// listing pass its own check and commit onto reclaimed bytes, and a finished
/// run leaves nothing else to find; the linger has to outlast the gap between
/// a writer's reference wave and its collector check, which is one listing
/// plus the backoff budget below. Writers reap what they read expired.
#[cfg(not(test))]
pub const RELEASE_LINGER_MS: i64 = 5_000;
#[cfg(test)]
pub const RELEASE_LINGER_MS: i64 = 100;

/// Attempts and jittered backoff for a writer waiting out a collector run; a
/// run only covers one batch, so the wait is short.
const WRITER_BACKOFF_ATTEMPTS: u32 = 5;
const WRITER_BACKOFF: Backoff =
    Backoff::exponential(StdDuration::from_millis(50), StdDuration::from_millis(800)).with_jitter();

/// One collector run's published claim over an inclusive digest range.
/// `Digest` ordering matches the lexical order of its `algo:hash` string, so
/// the range covers exactly the stored keys between its bounds.
#[derive(Debug, Serialize, Deserialize)]
pub struct GcRun {
    pub start: Digest,
    pub end: Digest,
    pub expires_at: DateTime<Utc>,
    pub instance: String,
}

/// One collector run's range marker: the only key a writer and the collector
/// both consult.
fn gc_run_path(run: &str) -> String {
    format!("{GC_ROOT}/{run}")
}

/// A held claim: the marker key plus the token that proves ownership on
/// refresh.
pub struct GcClaim {
    key: String,
    instance: String,
    start: Digest,
    end: Digest,
}

impl MetadataStore {
    /// Writer-side wait: brief backoff while an unexpired collector run
    /// covers any of `digests`. `false` means still covered after the budget,
    /// so the caller must treat the blobs as being reclaimed.
    pub async fn gc_clear(&self, digests: &[&Digest]) -> Result<bool, Error> {
        for attempt in 0..WRITER_BACKOFF_ATTEMPTS {
            if !self.gc_blocked(digests).await? {
                return Ok(true);
            }
            sleep(WRITER_BACKOFF.delay(attempt)).await;
        }
        Ok(false)
    }

    /// Whether an unexpired collector run covers any of `digests`; one
    /// listing, nothing per blob.
    pub async fn gc_blocked(&self, digests: &[&Digest]) -> Result<bool, Error> {
        let mut token = None;
        loop {
            let page = self.object_store().list(GC_ROOT, 100, token).await?;
            for run in &page.items {
                let key = gc_run_path(run);
                let raw = match self.object_store().get(&key).await {
                    Ok(raw) => raw,
                    // Released between the listing and the read.
                    Err(StorageError::NotFound) => continue,
                    Err(e) => return Err(e.into()),
                };
                // An unreadable marker blocks: failing open here would let a
                // corrupt marker green-light a delete race.
                let Ok(run) = serde_json::from_slice::<GcRun>(&raw) else {
                    return Ok(true);
                };
                if run.expires_at < Utc::now() {
                    // A released marker lingers by design and scrub leaves it
                    // alone, so the writer that reads it expired reaps it.
                    let _ = self.object_store().delete(&key).await;
                    continue;
                }
                if digests
                    .iter()
                    .any(|digest| run.start <= **digest && **digest <= run.end)
                {
                    return Ok(true);
                }
            }
            token = page.next_token;
            if token.is_none() {
                return Ok(false);
            }
        }
    }

    /// Collector side: publish a run marker covering `start..=end`. The
    /// expiry is generous because safety rests on [`Self::gc_refresh`], not
    /// on the timer.
    pub async fn gc_claim(&self, start: &Digest, end: &Digest) -> Result<GcClaim, Error> {
        let claim = GcClaim {
            key: gc_run_path(&Uuid::new_v4().to_string()),
            instance: Uuid::new_v4().to_string(),
            start: start.clone(),
            end: end.clone(),
        };
        // A fresh UUID cannot legitimately exist; adopting one would fence
        // against another collector's live marker.
        let body = self.gc_run_body(&claim)?;
        if !self
            .object_store()
            .create_if_absent(&claim.key, body)
            .await?
        {
            return Err(Error::Internal(format!(
                "gc run marker collision at {}",
                claim.key
            )));
        }
        Ok(claim)
    }

    /// Re-read and re-stamp the claim before an irreversible delete. `false`
    /// means the marker was lost or overwritten, so stop collecting: a writer
    /// may already have read it as expired and skipped its check.
    pub async fn gc_refresh(&self, claim: &GcClaim) -> Result<bool, Error> {
        match self.object_store().get(&claim.key).await {
            Ok(raw) => {
                let Ok(run) = serde_json::from_slice::<GcRun>(&raw) else {
                    return Ok(false);
                };
                if run.instance != claim.instance || run.expires_at < Utc::now() {
                    return Ok(false);
                }
            }
            Err(StorageError::NotFound) => return Ok(false),
            Err(e) => return Err(e.into()),
        }
        let body = self.gc_run_body(claim)?;
        self.object_store().put(&claim.key, body).await?;
        Ok(true)
    }

    /// Expire the claim once the range is done, rather than removing it: a
    /// writer whose reference key landed after this run's last liveness
    /// listing must still find the marker and back off.
    pub async fn gc_release(&self, claim: GcClaim) -> Result<(), Error> {
        let body = encode_run(
            &claim,
            Utc::now() + Duration::milliseconds(RELEASE_LINGER_MS),
        )?;
        self.object_store()
            .put(&claim.key, body)
            .await
            .map_err(Error::from)
    }

    fn gc_run_body(&self, claim: &GcClaim) -> Result<Bytes, Error> {
        // Twice the grace, floored so a marker outlives its own publish under
        // a tiny grace and capped so an absurd one cannot overflow the chrono
        // arithmetic. Capping costs nothing: the expiry only bounds how long a
        // crashed collector wedges writers, since a live one refreshes before
        // every delete.
        let ttl = i64::try_from(self.gc_grace_secs)
            .unwrap_or(i64::MAX)
            .saturating_mul(2)
            .clamp(60, 86_400);
        encode_run(claim, Utc::now() + Duration::seconds(ttl))
    }
}

fn encode_run(claim: &GcClaim, expires_at: DateTime<Utc>) -> Result<Bytes, Error> {
    let run = GcRun {
        start: claim.start.clone(),
        end: claim.end.clone(),
        expires_at,
        instance: claim.instance.clone(),
    };
    Ok(Bytes::from(serde_json::to_vec(&run)?))
}
