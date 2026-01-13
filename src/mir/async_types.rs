use crate::mir::Ty;

const FUTURE_TYPE_NAMES: [&str; 3] = ["Std::Async::Future", "Std.Async.Future", "Future"];
const TASK_TYPE_NAMES: [&str; 3] = ["Std::Async::Task", "Std.Async.Task", "Task"];

pub fn future_result_ty(ty: &Ty) -> Option<Ty> {
    match ty {
        Ty::Named(named) if matches_any(named.as_str(), &FUTURE_TYPE_NAMES) => {
            named.args().iter().find_map(|arg| arg.as_type()).cloned()
        }
        _ => None,
    }
}

pub fn task_result_ty(ty: &Ty) -> Option<Ty> {
    match ty {
        Ty::Named(named) if matches_any(named.as_str(), &TASK_TYPE_NAMES) => {
            named.args().iter().find_map(|arg| arg.as_type()).cloned()
        }
        _ => None,
    }
}

pub fn is_future_ty(ty: &Ty) -> bool {
    matches!(ty, Ty::Named(named) if matches_any(named.as_str(), &FUTURE_TYPE_NAMES))
}

pub fn is_task_ty(ty: &Ty) -> bool {
    matches!(ty, Ty::Named(named) if matches_any(named.as_str(), &TASK_TYPE_NAMES))
}

fn matches_any(name: &str, options: &[&str]) -> bool {
    let canonical = name.replace('.', "::");
    let tail = canonical.rsplit("::").next().unwrap_or(&canonical);
    let tail_base = tail.split('<').next().unwrap_or(tail);
    options.iter().any(|candidate| {
        let candidate_canonical = candidate.replace('.', "::");
        let candidate_tail = candidate_canonical
            .rsplit("::")
            .next()
            .unwrap_or(&candidate_canonical);
        let candidate_base = candidate_tail.split('<').next().unwrap_or(candidate_tail);
        canonical == candidate_canonical || tail_base == candidate_base
    })
}
