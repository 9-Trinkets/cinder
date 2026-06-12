use std::error::Error;
use std::path::Path;

pub use neuron_app::run::{
    RoleExecutionError, RoleExecutionResponse, RoleMetadata, WorkflowDefinition,
    WorkflowLocalRunner as LocalWorkflowRunner, WorkflowRoleConfig,
    WorkflowRoleService as NeuronRoleService, WorkflowRoutingMode, WorkflowTraceContext,
};

pub fn load_workflow(path: &Path) -> Result<WorkflowDefinition, Box<dyn Error>> {
    neuron_app::run::load_workflow(path)
}

pub fn run_workflow<R>(
    workflow: &WorkflowDefinition,
    instruction: &str,
    trace_events: bool,
    trace_dir: &Path,
    runner: R,
) -> Result<String, Box<dyn Error>>
where
    R: LocalWorkflowRunner + 'static,
{
    neuron_app::run::run_local_workflow(workflow, instruction, trace_events, trace_dir, runner)
}

pub fn evaluate_symbolic_value(
    config: &serde_json::Value,
    input: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    neuron_app::run::evaluate_symbolic_value(config, input)
}

pub fn evaluate_symbolic_role(
    prompt: &str,
    role_cfg: &WorkflowRoleConfig,
    request_id: &str,
) -> Result<String, String> {
    neuron_app::run::evaluate_symbolic_role(prompt, role_cfg, request_id)
}
