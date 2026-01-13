use crate::diagnostics::{FileCache, Span};
use crate::mir::MirModule;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ChicCoveragePointId(pub u64);

#[derive(Debug, Clone, Copy)]
pub struct ChicCoveragePoint {
    pub id: ChicCoveragePointId,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChicCoverageFileReport {
    pub path: String,
    pub covered: usize,
    pub total: usize,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChicCoverageReport {
    pub total: usize,
    pub covered: usize,
    pub percent: f64,
    pub files: Vec<ChicCoverageFileReport>,
    pub functions: Vec<ChicCoverageFunctionReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChicWorkspacePackageCoverage {
    pub name: String,
    pub covered: usize,
    pub total: usize,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChicWorkspaceCoverageReport {
    pub total: usize,
    pub covered: usize,
    pub percent: f64,
    pub packages: Vec<ChicWorkspacePackageCoverage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChicUncoveredFunction {
    pub name: String,
    pub path: String,
    pub statements: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChicCoverageFunctionReport {
    pub name: String,
    pub path: String,
    pub covered: bool,
    pub statements: usize,
}

#[must_use]
pub fn uncovered_functions(
    mir: &MirModule,
    points: &[ChicCoveragePoint],
    hits: &BTreeSet<ChicCoveragePointId>,
    files: &FileCache,
    root: &Path,
) -> Vec<ChicUncoveredFunction> {
    let covered_functions: BTreeSet<u64> = hits.iter().map(|hit| hit.0 >> 32).collect();
    let mut totals_by_function: BTreeMap<u64, (usize, Option<String>)> = BTreeMap::new();
    for point in points {
        let Some(path) = files.path(point.span.file_id) else {
            continue;
        };
        if !path.starts_with(root) {
            continue;
        }
        let func_id = point.id.0 >> 32;
        let entry = totals_by_function
            .entry(func_id)
            .or_insert((0, Some(path.to_string_lossy().to_string())));
        entry.0 += 1;
        if entry.1.is_none() {
            entry.1 = Some(path.to_string_lossy().to_string());
        }
    }

    let mut uncovered = Vec::new();
    for (func_id, (statements, path)) in totals_by_function {
        if statements == 0 || covered_functions.contains(&func_id) {
            continue;
        }
        let name = mir
            .functions
            .get(func_id as usize)
            .map(|func| func.name.clone())
            .unwrap_or_else(|| format!("<unknown:{func_id}>"));
        let path = path.unwrap_or_else(|| "<unknown>".to_string());
        uncovered.push(ChicUncoveredFunction {
            name,
            path,
            statements,
        });
    }
    uncovered
}

#[must_use]
pub fn collect_statement_points(mir: &MirModule) -> Vec<ChicCoveragePoint> {
    let mut points = Vec::new();
    for (function_index, function) in mir.functions.iter().enumerate() {
        let function_index = function_index as u64;
        let mut statement_index: u32 = 0;
        for block in &function.body.blocks {
            for statement in &block.statements {
                if let Some(span) = statement.span {
                    let id = ChicCoveragePointId((function_index << 32) | (statement_index as u64));
                    points.push(ChicCoveragePoint { id, span });
                }
                statement_index = statement_index.wrapping_add(1);
            }
        }
    }
    points
}

#[must_use]
pub fn report_for_root(
    mir: &MirModule,
    points: &[ChicCoveragePoint],
    hits: &BTreeSet<ChicCoveragePointId>,
    files: &FileCache,
    root: &Path,
) -> ChicCoverageReport {
    let mut totals_by_path: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut total = 0usize;
    let mut covered = 0usize;
    let covered_functions: BTreeSet<u64> = hits.iter().map(|hit| hit.0 >> 32).collect();
    let mut totals_by_function: BTreeMap<u64, (usize, Option<String>)> = BTreeMap::new();

    for point in points {
        let Some(path) = files.path(point.span.file_id) else {
            continue;
        };
        if !path.starts_with(root) {
            continue;
        }
        total += 1;
        let is_hit = covered_functions.contains(&(point.id.0 >> 32));
        if is_hit {
            covered += 1;
        }
        let key = path.to_string_lossy().to_string();
        let entry = totals_by_path.entry(key).or_insert((0, 0));
        entry.0 += 1;
        if is_hit {
            entry.1 += 1;
        }
        let func_id = point.id.0 >> 32;
        let entry = totals_by_function
            .entry(func_id)
            .or_insert((0, Some(path.to_string_lossy().to_string())));
        entry.0 += 1;
        if entry.1.is_none() {
            entry.1 = Some(path.to_string_lossy().to_string());
        }
    }

    let percent = if total == 0 {
        100.0
    } else {
        (covered as f64) * 100.0 / (total as f64)
    };
    let files = totals_by_path
        .into_iter()
        .map(
            |(path, (file_total, file_covered))| ChicCoverageFileReport {
                path,
                covered: file_covered,
                total: file_total,
                percent: if file_total == 0 {
                    100.0
                } else {
                    (file_covered as f64) * 100.0 / (file_total as f64)
                },
            },
        )
        .collect::<Vec<_>>();
    let functions = totals_by_function
        .into_iter()
        .map(|(func_id, (statements, path))| {
            let name = mir
                .functions
                .get(func_id as usize)
                .map(|func| func.name.clone())
                .unwrap_or_else(|| format!("<unknown:{func_id}>"));
            ChicCoverageFunctionReport {
                name,
                path: path.unwrap_or_else(|| "<unknown>".to_string()),
                covered: covered_functions.contains(&func_id),
                statements,
            }
        })
        .collect::<Vec<_>>();

    ChicCoverageReport {
        total,
        covered,
        percent,
        files,
        functions,
    }
}

#[must_use]
pub fn package_src_root(manifest_path: Option<&Path>) -> Option<PathBuf> {
    let manifest_dir = manifest_path.and_then(Path::parent)?;
    Some(manifest_dir.join("src"))
}

#[must_use]
pub fn coverage_output_dir(root: &Path) -> PathBuf {
    root.join("coverage").join("chic")
}

#[must_use]
pub fn package_report_path(root: &Path, package_name: &str) -> PathBuf {
    coverage_output_dir(root).join(format!("{package_name}.json"))
}

#[must_use]
pub fn workspace_report_path(root: &Path) -> PathBuf {
    coverage_output_dir(root).join("workspace.json")
}

pub fn write_report_json(report: &ChicCoverageReport, path: &Path) -> std::io::Result<()> {
    write_json(report, path)
}

pub fn write_workspace_report_json(
    report: &ChicWorkspaceCoverageReport,
    path: &Path,
) -> std::io::Result<()> {
    write_json(report, path)
}

fn write_json<T: Serialize>(report: &T, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, report)?;
    writer.write_all(b"\n")?;
    Ok(())
}
