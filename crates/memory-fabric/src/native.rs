//! Local-first TemporalGraph persistence — no mocks, real graph.json I/O.

use std::fs;
use std::path::{Path, PathBuf};

use aevum_evidence_graph::{GraphSnapshot, SearchHit, TemporalGraph};

use crate::backend::{MemoryBackend, MemoryError, RemoteFact};

pub struct NativeBackend {
    path: PathBuf,
    graph: TemporalGraph,
}

impl NativeBackend {
    pub fn open(mission_dir: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let path = mission_dir.as_ref().join("graph.json");
        let mut backend = Self {
            path,
            graph: TemporalGraph::new(),
        };
        if backend.path.exists() {
            backend.load()?;
        }
        Ok(backend)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl MemoryBackend for NativeBackend {
    fn name(&self) -> &'static str {
        "native"
    }

    fn load(&mut self) -> Result<(), MemoryError> {
        let raw = fs::read_to_string(&self.path)
            .map_err(|e| MemoryError::Io(format!("{}: {e}", self.path.display())))?;
        let snap: GraphSnapshot = serde_json::from_str(&raw)
            .map_err(|e| MemoryError::Backend(format!("invalid graph.json: {e}")))?;
        self.graph =
            TemporalGraph::from_snapshot(snap).map_err(|e| MemoryError::Backend(e.to_string()))?;
        Ok(())
    }

    fn save(&self) -> Result<(), MemoryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| MemoryError::Io(format!("mkdir: {e}")))?;
        }
        let text = serde_json::to_string_pretty(&self.graph.to_snapshot())
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        fs::write(&self.path, text).map_err(|e| MemoryError::Io(e.to_string()))?;
        Ok(())
    }

    fn graph(&self) -> &TemporalGraph {
        &self.graph
    }

    fn graph_mut(&mut self) -> &mut TemporalGraph {
        &mut self.graph
    }

    fn search(
        &self,
        query: &str,
        as_of: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>, MemoryError> {
        use crate::embed::{default_embedder, semantic_hybrid_search};
        let embedder = default_embedder();
        semantic_hybrid_search(&self.graph, query, as_of, limit, embedder.as_ref())
            .map_err(MemoryError::from)
    }

    fn remote_search(&self, _query: &str, _limit: usize) -> Result<Vec<RemoteFact>, MemoryError> {
        Ok(vec![])
    }
}
