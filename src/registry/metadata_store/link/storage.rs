//! Single-link primitives: read a link's [`LinkMetadata`], the cache-aware
//! `read_link`, and the cache helpers behind it (gated on `link_cache_ttl`).

use tracing::{instrument, warn};

use angos_oci::Namespace;

use crate::registry::{
    Error,
    metadata_store::{LinkKind, LinkMetadata, MetadataStore},
};

impl MetadataStore {
    /// Read the stored [`LinkMetadata`] for `link` within `namespace`: a tag
    /// from its ordered entries, a revision or referrer from its record. Every
    /// other kind is stored as a reference key carrying no metadata of its own.
    pub async fn read_link_reference(
        &self,
        namespace: &Namespace,
        link: &LinkKind,
    ) -> Result<LinkMetadata, Error> {
        match link {
            LinkKind::Tag(tag) => self.resolve_tag(namespace, tag).await,
            LinkKind::Digest(digest) => self.resolve_revision(namespace, digest).await,
            LinkKind::Referrer { subject, referrer } => {
                self.resolve_referrer(namespace, subject, referrer).await
            }
            _ => Err(Error::NotFound),
        }
    }

    /// Cache-aware link read with no access-time side effect: serve from the
    /// link cache, else read through and populate it.
    #[instrument(skip(self))]
    pub async fn read_link(
        &self,
        namespace: &Namespace,
        link: &LinkKind,
    ) -> Result<LinkMetadata, Error> {
        if let Some(cached) = self.cache_get(namespace, link).await {
            return Ok(cached);
        }
        let data = self.read_link_reference(namespace, link).await?;
        self.cache_put(namespace, link, &data).await;
        Ok(data)
    }

    fn cache_key(namespace: &Namespace, link: &LinkKind) -> String {
        format!("link:{namespace}:{link}")
    }

    pub async fn cache_get(&self, namespace: &Namespace, link: &LinkKind) -> Option<LinkMetadata> {
        if self.link_cache_ttl == 0 {
            return None;
        }
        let cache = self.cache.as_ref()?;
        cache
            .retrieve::<LinkMetadata>(&Self::cache_key(namespace, link))
            .await
            .ok()
            .flatten()
    }

    pub async fn cache_put(&self, namespace: &Namespace, link: &LinkKind, metadata: &LinkMetadata) {
        if self.link_cache_ttl == 0 {
            return;
        }
        // Every kind expires on the configured TTL. A revision or referrer
        // record never mutates, but it does get deleted, and invalidation
        // reaches only the process that performed the delete: pinning those
        // records would keep every other replica answering for content that is
        // gone. The record read they save is a single GET.
        if let Some(cache) = &self.cache {
            let key = Self::cache_key(namespace, link);
            if let Err(err) = cache.store(&key, metadata, self.link_cache_ttl).await {
                warn!("Failed to store link metadata in cache for {namespace}/{link}: {err}");
            }
        }
    }

    pub async fn cache_invalidate(&self, namespace: &Namespace, link: &LinkKind) {
        if let Some(cache) = &self.cache {
            let _ = cache.delete_value(&Self::cache_key(namespace, link)).await;
        }
    }
}
