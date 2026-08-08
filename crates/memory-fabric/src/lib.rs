//! Aevum Memory Fabric — native-first local memory (ADR-0016/0018).
//!
//! - [`SqliteBackend`] / [`NativeBackend`]: authority plane (default, autonomous)
//! - [`assemble`]: trust-weighted context (retrieval ∩ epistemic ∩ capability)
//! - [`promotion`]: remote/untrusted recall → attested authorize (never automatic)

pub mod assembly;
pub mod backend;
pub mod embed;
pub mod extract;
pub mod multitenant;
pub mod native;
pub mod promotion;
pub mod scope;
pub mod slop_ingest;
pub mod sqlite;

pub use assembly::{assemble, AssembledContext, AssemblyRequest, RankedHit};
pub use backend::{MemoryBackend, MemoryError, MemoryHit, MemorySource, RemoteFact};
#[cfg(feature = "remote-embed")]
pub use embed::OpenAiCompatibleEmbedder;
pub use embed::{
    default_embedder, ensure_node_embeddings, semantic_hybrid_search,
    semantic_hybrid_search_scoped, EmbedError, Embedder, HashingEmbedder,
};
pub use extract::{
    ingest_structured_json, ingest_text_triples, ExtractError, IngestReport, StructuredEpisodeDoc,
    StructuredFact,
};
pub use multitenant::MultiTenantStore;
pub use native::NativeBackend;
pub use promotion::{ingest_remote_as_inference, promote_to_authorize};
pub use scope::TenantScope;
pub use slop_ingest::{ingest_slop_report, SlopFinding, SlopReport};
pub use sqlite::SqliteBackend;

/// Open the autonomous local backend (SQLite by default; JSON if `AEVUM_GRAPH_STORE=json`).
/// Applies [`TenantScope`] from mission metadata + `AEVUM_TENANT_ID`.
pub fn open_backend(
    mission_dir: impl AsRef<std::path::Path>,
) -> Result<Box<dyn MemoryBackend>, MemoryError> {
    let store = std::env::var("AEVUM_GRAPH_STORE").unwrap_or_else(|_| "sqlite".into());
    if store == "json" {
        Ok(Box::new(NativeBackend::open(mission_dir)?))
    } else {
        Ok(Box::new(SqliteBackend::open(mission_dir)?))
    }
}

/// Open shared multi-tenant store at `AEVUM_MEMORY_ROOT` (or explicit root).
pub fn open_multi_tenant_store(
    root: impl AsRef<std::path::Path>,
) -> Result<MultiTenantStore, MemoryError> {
    MultiTenantStore::open(root)
}
