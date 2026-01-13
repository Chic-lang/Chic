use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::frontend::diagnostics::Span;
use crate::mir::data::ConstValue;

#[derive(Clone, Debug)]
pub(crate) enum DefaultArgumentValue {
    Const(ConstValue),
    Thunk {
        symbol: String,
        metadata_count: usize,
        span: Option<Span>,
    },
}

#[derive(Default)]
pub(crate) struct DefaultArgumentMap {
    entries: HashMap<String, Vec<Option<DefaultArgumentValue>>>,
}

pub(crate) type DefaultArgumentStore = Rc<RefCell<DefaultArgumentMap>>;

impl DefaultArgumentMap {
    pub(crate) fn record(
        &mut self,
        internal: impl Into<String>,
        values: Vec<Option<DefaultArgumentValue>>,
    ) {
        self.entries.insert(internal.into(), values);
    }

    pub(crate) fn value(&self, internal: &str, index: usize) -> Option<&DefaultArgumentValue> {
        self.entries
            .get(internal)
            .and_then(|slots| slots.get(index))
            .and_then(|value| value.as_ref())
    }
}
