use crate::frontend::diagnostics::Span;
use crate::mir::{
    AcceleratorCopyKind, BasicBlock, LocalId, MirBody, Operand, Place, Statement, StatementKind,
    StreamMetadata, Ty,
};

/// Helper for emitting accelerator/stream statements and registering stream metadata.
pub struct AcceleratorBuilder<'body> {
    body: &'body mut MirBody,
}

impl<'body> AcceleratorBuilder<'body> {
    #[must_use]
    pub fn new(body: &'body mut MirBody) -> Self {
        Self { body }
    }

    /// Register a stream local with optional memory space metadata, returning the stable stream id.
    pub fn register_stream(&mut self, local: LocalId, mem_space: Option<Ty>) -> u32 {
        if let Some(existing) = self
            .body
            .stream_metadata
            .iter()
            .find(|meta| meta.local == local)
        {
            return existing.stream_id;
        }
        let stream_id = self.body.stream_metadata.len() as u32;
        self.body.stream_metadata.push(StreamMetadata {
            local,
            mem_space,
            stream_id,
        });
        stream_id
    }

    /// Emit an `EnqueueKernel` statement into the provided block, tagging the stream metadata table.
    pub fn enqueue_kernel(
        &mut self,
        block: &mut BasicBlock,
        stream: Place,
        kernel: Operand,
        args: Vec<Operand>,
        completion: Option<Place>,
        span: Option<Span>,
    ) {
        self.register_stream(stream.local, None);
        block.statements.push(Statement {
            span,
            kind: StatementKind::EnqueueKernel {
                stream,
                kernel,
                args,
                completion,
            },
        });
    }

    /// Emit an `EnqueueCopy` statement and ensure the stream appears in body metadata.
    pub fn enqueue_copy(
        &mut self,
        block: &mut BasicBlock,
        stream: Place,
        dst: Place,
        src: Place,
        bytes: Operand,
        kind: AcceleratorCopyKind,
        completion: Option<Place>,
        span: Option<Span>,
    ) {
        self.register_stream(stream.local, None);
        block.statements.push(Statement {
            span,
            kind: StatementKind::EnqueueCopy {
                stream,
                dst,
                src,
                bytes,
                kind,
                completion,
            },
        });
    }

    /// Emit a `RecordEvent` for the given stream.
    pub fn record_event(
        &mut self,
        block: &mut BasicBlock,
        stream: Place,
        event: Place,
        span: Option<Span>,
    ) {
        self.register_stream(stream.local, None);
        block.statements.push(Statement {
            span,
            kind: StatementKind::RecordEvent { stream, event },
        });
    }

    /// Emit a `WaitEvent` statement to join on the provided event (and optional stream).
    pub fn wait_event(
        &mut self,
        block: &mut BasicBlock,
        event: Place,
        stream: Option<Place>,
        span: Option<Span>,
    ) {
        block.statements.push(Statement {
            span,
            kind: StatementKind::WaitEvent { event, stream },
        });
    }
}
