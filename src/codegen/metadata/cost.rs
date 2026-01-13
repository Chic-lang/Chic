//! Cost and trace metadata emission helpers.

use std::fmt::Write;

use crate::perf::PerfMetadata;

use super::schema::MetadataWriter;

pub(crate) fn append_cost_metadata(payload: &mut MetadataWriter, perf: &PerfMetadata) {
    for cost in &perf.costs {
        let mut parts = Vec::new();
        if let Some(value) = cost.cpu_budget_us {
            parts.push(format!("cpu_us={value}"));
        }
        if let Some(value) = cost.gpu_budget_us {
            parts.push(format!("gpu_us={value}"));
        }
        if let Some(value) = cost.mem_budget_bytes {
            parts.push(format!("mem_bytes={value}"));
        }
        if let Some(trace) = perf
            .tracepoints
            .iter()
            .find(|trace| trace.function == cost.function)
        {
            parts.push(format!("trace_id={}", trace.trace_id));
            parts.push(format!("trace_label={}", trace.label));
        }
        let joined = parts.join(";");
        let _ = writeln!(payload, "cost:{}={joined}", cost.function);
    }
}

pub(crate) fn append_trace_metadata(payload: &mut MetadataWriter, perf: &PerfMetadata) {
    for trace in &perf.tracepoints {
        let level = match trace.level {
            crate::perf::TraceLevel::Perf => "perf",
            crate::perf::TraceLevel::Debug => "debug",
        };
        let mut parts = vec![
            format!("id={}", trace.trace_id),
            format!("label={}", trace.label),
            format!("level={level}"),
        ];
        if let Some(cost) = trace.budget.as_ref() {
            if let Some(value) = cost.cpu_budget_us {
                parts.push(format!("cpu_us={value}"));
            }
            if let Some(value) = cost.gpu_budget_us {
                parts.push(format!("gpu_us={value}"));
            }
            if let Some(value) = cost.mem_budget_bytes {
                parts.push(format!("mem_bytes={value}"));
            }
        }
        let joined = parts.join(";");
        let _ = writeln!(payload, "trace:{}={joined}", trace.function);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf::{CostModel, TraceLevel, Tracepoint, trace_id};

    #[test]
    fn encodes_cost_and_trace_metadata() {
        let mut perf = PerfMetadata::default();
        let cost = CostModel {
            function: "Demo::trace".into(),
            cpu_budget_us: Some(10),
            gpu_budget_us: None,
            mem_budget_bytes: Some(512),
            span: None,
        };
        let trace = Tracepoint {
            function: "Demo::trace".into(),
            label: "trace".into(),
            label_id: None,
            level: TraceLevel::Perf,
            trace_id: trace_id("Demo::trace", "trace"),
            span: None,
            budget: Some(cost.clone()),
        };
        perf.costs.push(cost);
        perf.tracepoints.push(trace);

        let mut writer = MetadataWriter::default();
        append_cost_metadata(&mut writer, &perf);
        append_trace_metadata(&mut writer, &perf);
        let body = writer.into_body();
        assert!(body.contains("cost:Demo::trace=cpu_us=10;mem_bytes=512;trace_id="));
        assert!(body.contains("trace:Demo::trace=id="));
    }
}
