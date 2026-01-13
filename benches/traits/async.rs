use criterion::Criterion;
use std::hint::black_box;

pub fn bench(c: &mut Criterion) {
    c.bench_function("traits::async::manual_state_machine", |b| {
        b.iter(|| black_box(run_manual(black_box(10_000))));
    });

    c.bench_function("traits::async::generic_poll", |b| {
        b.iter(|| {
            let mut job = CounterJob::new(10_000);
            black_box(run_job(&mut job));
        });
    });

    c.bench_function("traits::async::dyn_poll", |b| {
        b.iter(|| {
            let mut job = CounterJob::new(10_000);
            let dyn_job: &mut dyn PollJob = &mut job;
            black_box(run_dyn_job(dyn_job));
        });
    });
}

trait PollJob {
    fn poll(&mut self) -> Option<u32>;
}

struct CounterJob {
    current: u32,
    end: u32,
}

impl CounterJob {
    fn new(end: u32) -> Self {
        Self { current: 0, end }
    }
}

impl PollJob for CounterJob {
    fn poll(&mut self) -> Option<u32> {
        if self.current >= self.end {
            return None;
        }
        self.current += 1;
        Some(self.current)
    }
}

fn run_manual(end: u32) -> u32 {
    let mut current = 0;
    while current < end {
        current += 1;
    }
    current
}

fn run_job<J: PollJob>(job: &mut J) -> u32 {
    let mut value = 0;
    while let Some(v) = job.poll() {
        value = v;
    }
    value
}

fn run_dyn_job(job: &mut dyn PollJob) -> u32 {
    let mut value = 0;
    while let Some(v) = job.poll() {
        value = v;
    }
    value
}
