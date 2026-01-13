use std::fmt::{self, Write};
use std::io::{self, BufWriter, Write as IoWrite};

use crate::chic_kind::ChicKind;
use crate::frontend::ast::Module;
use crate::target::Target;

use super::format;

/// Configuration for streamed textual emission.
pub struct TextStreamConfig {
    /// Buffer size for the internal `BufWriter` in bytes.
    pub buffer_capacity: usize,
    /// Optional benchmarking hook invoked after each explicit flush.
    pub on_flush: Option<Box<dyn FnMut(&TextStreamMetrics)>>,
}

impl Default for TextStreamConfig {
    fn default() -> Self {
        Self {
            buffer_capacity: 16 * 1024,
            on_flush: None,
        }
    }
}

/// Metrics captured during streamed emission.
#[derive(Debug, Default, Clone)]
pub struct TextStreamMetrics {
    pub bytes_written: usize,
    pub flushes: usize,
}

/// Streaming writer that feeds textual IR directly into an `io::Write` sink.
pub struct TextStreamWriter<W: IoWrite> {
    writer: BufWriter<W>,
    metrics: TextStreamMetrics,
    hook: Option<Box<dyn FnMut(&TextStreamMetrics)>>,
}

impl<W: IoWrite> TextStreamWriter<W> {
    pub fn with_config(writer: W, config: TextStreamConfig) -> Self {
        let capacity = config.buffer_capacity.max(4096);
        Self {
            writer: BufWriter::with_capacity(capacity, writer),
            metrics: TextStreamMetrics::default(),
            hook: config.on_flush,
        }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        self.metrics.flushes += 1;
        if let Some(hook) = &mut self.hook {
            hook(&self.metrics);
        }
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<(W, TextStreamMetrics)> {
        self.flush()?;
        let metrics = self.metrics;
        match self.writer.into_inner() {
            Ok(writer) => Ok((writer, metrics)),
            Err(err) => Err(err.into_error()),
        }
    }
}

impl<W: IoWrite> Write for TextStreamWriter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.is_empty() {
            return Ok(());
        }
        self.writer
            .write_all(s.as_bytes())
            .map_err(|_| fmt::Error)?;
        self.metrics.bytes_written += s.len();
        Ok(())
    }
}

fn write_header<W: Write>(out: &mut W, target: &Target, kind: ChicKind) -> fmt::Result {
    writeln!(
        out,
        "; Chic codegen output (temporary Rust bootstrap)\n\
         target: {}\n\
         artifact-kind: {}\n",
        target.triple(),
        kind.as_str()
    )
}

/// Generate a textual representation of the module for the selected target/kind.
#[must_use]
pub fn generate_text(module: &Module, target: &Target, kind: ChicKind) -> String {
    let config = TextStreamConfig::default();
    let (buffer, _) = stream_module(Vec::new(), module, target, kind, config)
        .expect("streaming textual IR into Vec");
    String::from_utf8(buffer).expect("textual IR is UTF-8")
}

/// Stream textual IR directly into an `io::Write` sink using the configured buffering.
pub fn stream_module<W: IoWrite>(
    writer: W,
    module: &Module,
    target: &Target,
    kind: ChicKind,
    config: TextStreamConfig,
) -> io::Result<(W, TextStreamMetrics)> {
    let mut stream = TextStreamWriter::with_config(writer, config);
    write_header(&mut stream, target, kind).map_err(fmt_to_io_error)?;
    format::write_module(&mut stream, module).map_err(fmt_to_io_error)?;
    stream.finish()
}

fn fmt_to_io_error(err: fmt::Error) -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        format!("text streaming failed: {err}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chic_kind::ChicKind;
    use crate::frontend::parser::parse_module;
    use crate::target::Target;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn streams_with_metrics_and_hooks() {
        let source = r"
namespace Sample;

public double Add(double x, double y)
{
    return x + y;
}
";
        let module = parse_module(source).expect("parse").module;
        let flush_counter = Rc::new(RefCell::new(0usize));
        let hook_counter = Rc::clone(&flush_counter);
        let mut config = TextStreamConfig::default();
        config.buffer_capacity = 8; // force multiple flushes for tiny buffer
        config.on_flush = Some(Box::new(move |metrics: &TextStreamMetrics| {
            *hook_counter.borrow_mut() += 1;
            assert!(metrics.bytes_written > 0);
        }));

        let (buffer, metrics) = stream_module(
            Vec::new(),
            &module,
            &Target::host(),
            ChicKind::Executable,
            config,
        )
        .expect("stream module");

        println!(
            "stream-metrics: bytes={} flushes={}",
            metrics.bytes_written, metrics.flushes
        );

        let text = String::from_utf8(buffer).expect("utf8");
        assert!(text.contains("fn Add"));
        assert!(metrics.bytes_written > 0);
        assert_eq!(metrics.bytes_written, text.len());
        assert!(*flush_counter.borrow() >= 1);
    }

    #[test]
    fn streams_large_module_to_sink() {
        let mut source = String::from("namespace Large;\n\n");
        for idx in 0..200 {
            source.push_str(&format!(
                "public double Fn{idx}(double x)\n{{\n    return x + {idx}.0;\n}}\n\n"
            ));
        }
        let module = parse_module(&source).expect("parse").module;
        let config = TextStreamConfig {
            buffer_capacity: 32 * 1024,
            on_flush: None,
        };
        let (_sink, metrics) = stream_module(
            io::sink(),
            &module,
            &Target::host(),
            ChicKind::Executable,
            config,
        )
        .expect("stream large module");

        println!(
            "large-stream-metrics: bytes={} flushes={}",
            metrics.bytes_written, metrics.flushes
        );

        assert!(metrics.bytes_written > 10_000);
        assert!(metrics.flushes >= 1);
    }
}
