//! Composite pattern parsing: structs, enums, tuples, lists, and path-based
//! patterns. Invariants:
//! - Builder helpers normalise enum vs. struct paths into the same
//!   `PatternNode` representation.
//! - Sequence parsing (`parse_pattern_list`) preserves element order and stops
//!   at the supplied delimiter without consuming trailing tokens.

use super::*;

impl PatternParser {
    pub(super) fn parse_record_pattern(
        &mut self,
        path: Option<Vec<String>>,
    ) -> Result<PatternNode, PatternParseError> {
        let start = self.index;
        self.expect_punctuation('{')?;
        let fields = if self.peek_punctuation('}') {
            Vec::new()
        } else {
            let parsed = self.parse_struct_fields()?;
            parsed
        };
        self.expect_punctuation('}')?;
        let span = self.span_from_range(start, self.index);
        if let Some(ref segments) = path {
            let field_count = fields.len();
            for meta in self
                .metadata
                .record_fields
                .iter_mut()
                .rev()
                .take(field_count)
            {
                meta.path = Some(segments.clone());
            }
        }
        Ok(PatternNode::Record(RecordPatternNode {
            path,
            fields,
            span,
        }))
    }

    pub(super) fn parse_tuple_pattern(&mut self) -> Result<PatternNode, PatternParseError> {
        self.expect_punctuation('(')?;
        if self.peek_punctuation(')') {
            self.advance();
            return Ok(PatternNode::Tuple(Vec::new()));
        }

        let mut elements = Vec::new();
        loop {
            elements.push(self.parse_or_pattern()?);
            if self.peek_punctuation(',') {
                self.advance();
                if self.peek_punctuation(')') {
                    break;
                }
                continue;
            }
            break;
        }
        self.expect_punctuation(')')?;
        Ok(PatternNode::Tuple(elements))
    }

    pub(super) fn parse_list_pattern(&mut self) -> Result<PatternNode, PatternParseError> {
        let start = self.index;
        self.expect_punctuation('[')?;
        let mut prefix = Vec::new();
        let mut suffix = Vec::new();
        let mut slice = None;
        let mut saw_slice = false;
        let mut slice_span = None;
        while !self.peek_punctuation(']') {
            if let Some(slice_start) = self.consume_slice_operator() {
                if saw_slice {
                    return Err(self.error("list pattern can contain at most one `..` slice"));
                }
                saw_slice = true;
                if self.peek_pattern_terminator() {
                    slice = Some(Box::new(PatternNode::Wildcard));
                    slice_span = self.span_from_range(slice_start, self.index);
                } else {
                    let pattern = self.parse_or_pattern()?;
                    let pattern = self.normalise_slice_pattern(pattern)?;
                    slice_span = self.span_from_range(slice_start, self.index);
                    slice = Some(Box::new(pattern));
                }
            } else {
                let item = self.parse_or_pattern()?;
                if saw_slice {
                    suffix.push(item);
                } else {
                    prefix.push(item);
                }
            }

            if self.peek_punctuation(',') {
                self.advance();
                if self.peek_punctuation(']') {
                    break;
                }
                continue;
            }
            break;
        }
        self.expect_punctuation(']')?;
        let span = self.span_from_range(start, self.index);
        let list = PatternNode::List(ListPatternNode {
            prefix,
            slice,
            suffix,
            span,
            slice_span,
        });
        if let PatternNode::List(ref list_node) = list {
            if let Some(span) = list_node.slice_span {
                let binding = list_node.slice.as_deref().and_then(|node| {
                    if let PatternNode::Binding(binding) = node {
                        Some(binding.name.clone())
                    } else {
                        None
                    }
                });
                self.metadata.list_slices.push(ListSliceMetadata {
                    span: Some(span),
                    binding,
                });
            }
        }
        Ok(list)
    }

    fn normalise_slice_pattern(
        &mut self,
        pattern: PatternNode,
    ) -> Result<PatternNode, PatternParseError> {
        let span = self.node_span(&pattern);
        match pattern {
            PatternNode::Binding(_) | PatternNode::Wildcard => Ok(pattern),
            PatternNode::Type {
                mut path,
                subpattern: None,
            } if path.len() == 1 => {
                let name = path.pop().unwrap_or_default();
                self.metadata.bindings.push(PatternBindingMetadata {
                    name: name.clone(),
                    span,
                });
                Ok(PatternNode::Binding(BindingPatternNode {
                    name,
                    mutability: PatternBindingMutability::Immutable,
                    mode: PatternBindingMode::Value,
                    span,
                }))
            }
            _ => Err(self.error("list slice patterns only support binding or wildcard targets")),
        }
    }

    pub(super) fn parse_path_pattern(&mut self) -> Result<PatternNode, PatternParseError> {
        let segments = self.parse_path_segments()?;
        if self.peek_punctuation('{') {
            return self.parse_struct_path_pattern(segments);
        }
        if self.peek_punctuation('(') {
            return self.parse_positional_path_pattern(segments);
        }
        self.parse_type_or_enum_path_pattern(segments)
    }

    fn parse_path_segments(&mut self) -> Result<Vec<String>, PatternParseError> {
        let mut segments = Vec::new();
        let mut first = self.parse_identifier("pattern")?;
        self.consume_type_suffix(&mut first);
        segments.push(first);
        while self.peek_punctuation('.') {
            self.advance();
            let mut segment = self.parse_identifier("pattern segment")?;
            self.consume_type_suffix(&mut segment);
            segments.push(segment);
        }
        Ok(segments)
    }

    fn parse_struct_path_pattern(
        &mut self,
        segments: Vec<String>,
    ) -> Result<PatternNode, PatternParseError> {
        self.advance();
        let fields = self.parse_struct_fields()?;
        let field_count = fields.len();
        for meta in self
            .metadata
            .record_fields
            .iter_mut()
            .rev()
            .take(field_count)
        {
            meta.path = Some(segments.clone());
        }
        self.expect_punctuation('}')?;
        Ok(Self::build_struct_pattern(segments, fields))
    }

    fn parse_positional_path_pattern(
        &mut self,
        segments: Vec<String>,
    ) -> Result<PatternNode, PatternParseError> {
        self.advance();
        let elements = self.parse_pattern_list(')')?;
        self.expect_punctuation(')')?;
        Ok(Self::build_positional_pattern(segments, elements))
    }

    fn parse_type_or_enum_path_pattern(
        &mut self,
        mut segments: Vec<String>,
    ) -> Result<PatternNode, PatternParseError> {
        if !self.peek_pattern_terminator() {
            let mut subpattern = self.parse_or_pattern()?;
            if let PatternNode::Type {
                path: binding_path,
                subpattern: None,
            } = &subpattern
            {
                if binding_path.len() == 1 {
                    let name = binding_path[0].clone();
                    let span = self.node_span(&subpattern);
                    self.metadata.bindings.push(PatternBindingMetadata {
                        name: name.clone(),
                        span,
                    });
                    subpattern = PatternNode::Binding(BindingPatternNode {
                        name,
                        mutability: PatternBindingMutability::Immutable,
                        mode: PatternBindingMode::Value,
                        span,
                    });
                }
            }
            return Ok(PatternNode::Type {
                path: segments,
                subpattern: Some(Box::new(subpattern)),
            });
        }

        if segments.len() == 1 {
            return Ok(PatternNode::Type {
                path: segments,
                subpattern: None,
            });
        }

        let variant = segments.pop().unwrap_or_default();
        Ok(PatternNode::Enum {
            path: segments,
            variant,
            fields: VariantPatternFieldsNode::Unit,
        })
    }

    fn consume_type_suffix(&mut self, target: &mut String) {
        loop {
            let Some(token) = self.peek().cloned() else {
                break;
            };
            match token.kind {
                TokenKind::Punctuation('<') => {
                    self.append_generic_arguments(target);
                }
                TokenKind::Operator(ref op) if op.chars().all(|ch| ch == '<') => {
                    self.append_generic_arguments(target);
                }
                TokenKind::Punctuation('[') => {
                    self.append_bracket_suffix(target);
                }
                TokenKind::Punctuation('?') => {
                    target.push('?');
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn append_generic_arguments(&mut self, target: &mut String) {
        let mut depth = 0usize;
        loop {
            let Some(token) = self.advance() else {
                break;
            };
            target.push_str(&token.lexeme);
            match token.kind {
                TokenKind::Punctuation('<') => depth = depth.saturating_add(1),
                TokenKind::Operator(ref op) if op.chars().all(|ch| ch == '<') => {
                    depth = depth.saturating_add(op.len());
                }
                TokenKind::Punctuation('>') => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    if depth == 0 {
                        break;
                    }
                }
                TokenKind::Operator(ref op) if op.chars().all(|ch| ch == '>') => {
                    let count = op.len();
                    if depth > count {
                        depth -= count;
                    } else {
                        depth = 0;
                    }
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    fn append_bracket_suffix(&mut self, target: &mut String) {
        let mut depth = 0usize;
        loop {
            let Some(token) = self.advance() else {
                break;
            };
            target.push_str(&token.lexeme);
            match token.kind {
                TokenKind::Punctuation('[') => {
                    depth = depth.saturating_add(1);
                }
                TokenKind::Punctuation(']') => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    fn build_struct_pattern(
        mut segments: Vec<String>,
        fields: Vec<PatternFieldNode>,
    ) -> PatternNode {
        if segments.len() == 1 {
            return PatternNode::Struct {
                path: segments,
                fields,
            };
        }

        let variant = segments.pop().unwrap_or_default();
        PatternNode::Enum {
            path: segments,
            variant,
            fields: VariantPatternFieldsNode::Struct(fields),
        }
    }

    fn build_positional_pattern(
        mut segments: Vec<String>,
        elements: Vec<PatternNode>,
    ) -> PatternNode {
        if segments.len() == 1 {
            return PatternNode::Positional {
                path: segments,
                elements,
            };
        }

        let variant = segments.pop().unwrap_or_default();
        PatternNode::Enum {
            path: segments,
            variant,
            fields: VariantPatternFieldsNode::Tuple(elements),
        }
    }

    fn parse_struct_fields(&mut self) -> Result<Vec<PatternFieldNode>, PatternParseError> {
        let mut fields = Vec::new();
        while !self.peek_punctuation('}') {
            let field_start = self.index;
            let (name, name_span) = self.parse_spanned_identifier("pattern field")?;
            let pattern = if self.peek_punctuation(':') {
                self.advance();
                self.parse_or_pattern()?
            } else {
                PatternNode::Wildcard
            };
            let pattern_span = self.node_span(&pattern);
            let span = self
                .span_from_range(field_start, self.index)
                .or(name_span)
                .or(pattern_span);
            self.metadata.record_fields.push(RecordFieldMetadata {
                name: name.clone(),
                name_span,
                pattern_span,
                path: None,
            });
            fields.push(PatternFieldNode {
                name,
                pattern,
                span,
                name_span,
            });
            if self.peek_punctuation(',') {
                self.advance();
                if self.peek_punctuation('}') {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(fields)
    }

    fn parse_pattern_list(
        &mut self,
        terminator: char,
    ) -> Result<Vec<PatternNode>, PatternParseError> {
        let mut patterns = Vec::new();
        if self.peek_punctuation(terminator) {
            return Ok(patterns);
        }
        loop {
            patterns.push(self.parse_or_pattern()?);
            if self.peek_punctuation(',') {
                self.advance();
                if self.peek_punctuation(terminator) {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(patterns)
    }
}
