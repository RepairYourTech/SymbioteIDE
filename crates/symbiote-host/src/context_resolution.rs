//! Host-side context/credential resolution wiring (#217 slice): the store
//! adapter for `symbiote-context::ContextSource` and the resolution entry
//! the dispatch activation path uses. Every fact comes from the store —
//! the caller cannot substitute its own — and credential values never
//! pass through this module (only the operator's broker, held by
//! `WorkerTransports`, ever issues a lease).
use symbiote_context::{ContextSource, ResolutionError, WorkItemText};
use symbiote_domain::{ProjectId, WorkId};
use symbiote_store::Store;

/// The store-backed [`ContextSource`]. Reads are the same canonical
/// readers the protocol serves; failures collapse to `StoreFailed` so no
/// store text leaks into resolution errors.
pub(crate) struct StoreContextSource<'a> {
    pub store: &'a Store,
}

impl ContextSource for StoreContextSource<'_> {
    fn work_item_text(
        &self,
        project: &ProjectId,
        work: &WorkId,
    ) -> Result<Option<WorkItemText>, ResolutionError> {
        let item = self
            .store
            .work_item(project, work)
            .map_err(|_| ResolutionError::StoreFailed)?;
        let spec = item.spec();
        Ok(Some(WorkItemText {
            description: spec.description.clone(),
            requirements: spec.requirements.clone(),
            constraints: spec.constraints.clone(),
            risks: spec.risks.clone(),
            acceptance: spec.acceptance.clone(),
        }))
    }
}

/// Resolves the origin work reference for a task (the task's classified
/// origin), collapsing the store errors the same way.
pub(crate) fn task_origin_work(
    store: &Store,
    task_id: &symbiote_domain::TaskId,
) -> Result<(ProjectId, WorkId), ResolutionError> {
    let task = store
        .task(task_id)
        .map_err(|_| ResolutionError::StoreFailed)?;
    let origin = store
        .task_origin(&task.project_id().clone(), task_id)
        .map_err(|_| ResolutionError::StoreFailed)?
        .ok_or(ResolutionError::NoPrompt)?;
    let reference = origin.reference();
    Ok((task.project_id().clone(), reference.id.clone()))
}

// The end-to-end resolution test (fixture → resolve_context over the
// real store) lives in runner.rs tests where the full fixture lives;
// this module's adapter is covered there against a real Store.
