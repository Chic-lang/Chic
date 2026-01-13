use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const RUN_LOG_VERSION: &str = "0.1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunLog {
    pub version: String,
    pub rng_streams: Vec<RngStreamLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RngStreamLog {
    pub id: u64,
    pub seed: u128,
    #[serde(default)]
    pub events: Vec<RngEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RngEvent {
    pub index: u64,
    pub op: RngEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RngEventKind {
    Next { bits: u32 },
    Advance { by: u64 },
    Split { child: u64 },
}

#[derive(Debug)]
pub enum RunLogError {
    Io(std::io::Error),
    Decode(serde_json::Error),
    MissingRng,
}

impl fmt::Display for RunLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read run log: {err}"),
            Self::Decode(err) => write!(f, "failed to decode run log: {err}"),
            Self::MissingRng => write!(f, "run log does not contain RNG data"),
        }
    }
}

impl std::error::Error for RunLogError {}

#[derive(Default)]
struct RunLogState {
    enabled: bool,
    output: Option<PathBuf>,
    log: RunLog,
    next_index: HashMap<u64, u64>,
}

impl RunLogState {
    fn ensure_stream(&mut self, stream_id: u64, seed: u128) {
        if self.log.rng_streams.iter().any(|s| s.id == stream_id) {
            return;
        }
        self.log.rng_streams.push(RngStreamLog {
            id: stream_id,
            seed,
            events: Vec::new(),
        });
        self.next_index.insert(stream_id, 0);
    }

    fn append_event(&mut self, stream_id: u64, seed: u128, op: RngEventKind) {
        if !self.enabled {
            return;
        }
        self.ensure_stream(stream_id, seed);
        let index = self.next_index.entry(stream_id).or_insert(0);
        let event = RngEvent { index: *index, op };
        *index = index.saturating_add(1);
        if let Some(stream) = self.log.rng_streams.iter_mut().find(|s| s.id == stream_id) {
            stream.events.push(event);
        }
        if let Some(path) = self.output.clone() {
            let _ = write_log(&self.log, &path);
        }
    }
}

static RUN_LOGGER: Lazy<Mutex<RunLogState>> = Lazy::new(|| {
    let mut state = RunLogState::default();
    state.log.version = RUN_LOG_VERSION.to_string();
    Mutex::new(state)
});
static ENV_LOG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

fn env_log_path() -> Option<PathBuf> {
    ENV_LOG_PATH
        .get_or_init(|| std::env::var_os("CHIC_RUN_LOG").map(PathBuf::from))
        .clone()
}

pub fn enable_logging(output: Option<PathBuf>) {
    let mut guard = RUN_LOGGER.lock().expect("run log mutex poisoned");
    guard.enabled = true;
    guard.output = output;
}

pub fn disable_logging() {
    let mut guard = RUN_LOGGER.lock().expect("run log mutex poisoned");
    guard.enabled = false;
    guard.output = None;
    guard.log = RunLog {
        version: RUN_LOG_VERSION.to_string(),
        rng_streams: Vec::new(),
    };
    guard.next_index.clear();
}

pub fn record_rng_event(stream_id: u64, seed: u128, op: RngEventKind) {
    let mut guard = RUN_LOGGER.lock().expect("run log mutex poisoned");
    guard.append_event(stream_id, seed, op);
}

pub fn record_stream(stream_id: u64, seed: u128) {
    let mut guard = RUN_LOGGER.lock().expect("run log mutex poisoned");
    if !guard.enabled {
        return;
    }
    guard.ensure_stream(stream_id, seed);
    if let Some(path) = guard.output.clone() {
        let _ = write_log(&guard.log, &path);
    }
}

pub fn maybe_enable_from_env() {
    if let Some(path) = env_log_path() {
        enable_logging(Some(path));
    }
}

pub fn snapshot() -> RunLog {
    RUN_LOGGER
        .lock()
        .expect("run log mutex poisoned")
        .log
        .clone()
}

pub fn flush() -> Result<Option<PathBuf>, RunLogError> {
    let guard = RUN_LOGGER.lock().expect("run log mutex poisoned");
    let Some(path) = guard.output.clone() else {
        return Ok(None);
    };
    write_log(&guard.log, &path)?;
    Ok(Some(path))
}

pub fn load(path: &Path) -> Result<RunLog, RunLogError> {
    let body = fs::read_to_string(path).map_err(RunLogError::Io)?;
    serde_json::from_str(&body).map_err(RunLogError::Decode)
}

pub fn seeds(log: &RunLog) -> Vec<(u64, u128)> {
    log.rng_streams
        .iter()
        .map(|stream| (stream.id, stream.seed))
        .collect()
}

fn write_log(log: &RunLog, path: &Path) -> Result<(), RunLogError> {
    let encoded = serde_json::to_string_pretty(log).map_err(RunLogError::Decode)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(RunLogError::Io)?;
    }
    fs::write(path, encoded).map_err(RunLogError::Io)
}
