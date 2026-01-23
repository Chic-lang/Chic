use crate::error::Result;
use crate::format::config::{FormatConfig, NewlineStyle};
use crate::format::{MemberSort, TypeSort};
use crate::frontend::lexer::{Keyword, Token, TokenKind, lex};
use crate::frontend::parser::parse_module;

/// Output of formatting a single source string.
#[derive(Debug, Clone)]
pub struct FormatOutcome {
    pub formatted: String,
    pub metadata: FormatMetadata,
}

/// Lightweight metadata describing the formatted source for file-organisation helpers.
#[derive(Debug, Clone, Default)]
pub struct FormatMetadata {
    pub namespace: Option<String>,
    pub top_level_types: Vec<String>,
    pub types: Vec<TypeMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeMetadata {
    pub name: String,
    pub kind: crate::format::TypeSort,
    pub members: Vec<crate::format::MemberSort>,
}

impl Default for TypeMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: TypeSort::Class,
            members: Vec::new(),
        }
    }
}

/// Format Chic source code according to the provided configuration.
///
/// Parsing is performed first to ensure the source is valid before rewriting.
pub fn format_source(source: &str, config: &FormatConfig) -> Result<FormatOutcome> {
    if !config.enabled {
        return Ok(FormatOutcome {
            formatted: source.to_string(),
            metadata: FormatMetadata::default(),
        });
    }
    let parsed = match parse_module(source) {
        Ok(parsed) => Some(parsed),
        Err(err) => {
            // Best-effort formatting for snippets or partial code; only fail when enforcement demands it.
            if config.enforce == crate::format::FormatEnforcement::Error {
                return Err(err.into());
            }
            None
        }
    };
    let tokens = reorder_imports(lex(source).tokens, config);
    let mut formatter = TokenFormatter::new(config, tokens);
    let formatted = formatter.format();
    let metadata = parsed
        .as_ref()
        .map(|parsed| collect_metadata(&parsed.module))
        .unwrap_or_default();
    Ok(FormatOutcome {
        formatted,
        metadata,
    })
}

struct TokenFormatter<'a> {
    config: &'a FormatConfig,
    tokens: Vec<Token>,
    output: String,
    indent: usize,
    indent_unit: String,
    at_line_start: bool,
    line_len: usize,
    newline: &'static str,
    paren_depth: usize,
    bracket_depth: usize,
    brace_depth: usize,
    control_keyword: Option<Keyword>,
    last_token: Option<TokenKind>,
    pending_space: bool,
    paren_indent: Vec<usize>,
    in_switch: usize,
    switch_indents: Vec<usize>,
    last_case_label: bool,
    just_closed_block: bool,
}

impl<'a> TokenFormatter<'a> {
    fn new(config: &'a FormatConfig, tokens: Vec<Token>) -> Self {
        let newline = match config.newline {
            crate::format::NewlineStyle::Crlf => "\r\n",
            crate::format::NewlineStyle::Lf => "\n",
        };
        Self {
            config,
            tokens,
            output: String::new(),
            indent: 0,
            indent_unit: if config.indent.use_tabs {
                "\t".to_string()
            } else {
                " ".repeat(config.indent.size as usize)
            },
            at_line_start: true,
            line_len: 0,
            newline,
            paren_depth: 0,
            bracket_depth: 0,
            brace_depth: 0,
            control_keyword: None,
            last_token: None,
            pending_space: false,
            paren_indent: Vec::new(),
            in_switch: 0,
            switch_indents: Vec::new(),
            last_case_label: false,
            just_closed_block: false,
        }
    }

    fn format(&mut self) -> String {
        for token in self.tokens.clone() {
            let kind = token.kind.clone();
            if self.just_closed_block {
                let join_with_prev = matches!(kind, TokenKind::Keyword(Keyword::Else))
                    && !self.config.r#if.else_on_new_line;
                if join_with_prev {
                    self.flush_pending_space();
                    self.pending_space = false;
                    self.push_raw(" ");
                    self.last_token = None;
                    self.just_closed_block = false;
                } else if matches!(kind, TokenKind::Whitespace) {
                    continue;
                } else {
                    self.write_newline_if_needed();
                    self.just_closed_block = false;
                }
            }
            match kind {
                TokenKind::Whitespace => {
                    // ignore original whitespace; formatting is deterministic.
                }
                TokenKind::Comment => {
                    self.flush_pending_space();
                    self.emit_comment(&token.lexeme);
                    self.last_token = Some(TokenKind::Comment);
                }
                TokenKind::DocComment => {
                    self.flush_pending_space();
                    self.emit_doc_comment(&token.lexeme);
                    self.last_token = Some(TokenKind::DocComment);
                }
                TokenKind::Punctuation('{') => {
                    self.handle_open_brace();
                    self.last_token = Some(TokenKind::Punctuation('{'));
                }
                TokenKind::Punctuation('}') => {
                    self.handle_close_brace();
                    self.last_token = Some(TokenKind::Punctuation('}'));
                }
                TokenKind::Punctuation(';') => {
                    self.handle_semicolon();
                    self.last_token = Some(TokenKind::Punctuation(';'));
                }
                TokenKind::Punctuation(',') => {
                    self.handle_comma();
                    self.last_token = Some(TokenKind::Punctuation(','));
                }
                TokenKind::Punctuation('(') => {
                    self.handle_open_paren();
                    self.last_token = Some(TokenKind::Punctuation('('));
                }
                TokenKind::Punctuation(')') => {
                    self.handle_close_paren();
                    self.last_token = Some(TokenKind::Punctuation(')'));
                }
                TokenKind::Punctuation('[') => {
                    self.flush_pending_space();
                    self.push_text("[");
                    self.bracket_depth += 1;
                    self.last_token = Some(TokenKind::Punctuation('['));
                }
                TokenKind::Punctuation(']') => {
                    if self.bracket_depth > 0 {
                        self.bracket_depth -= 1;
                    }
                    self.flush_pending_space();
                    self.push_text("]");
                    self.last_token = Some(TokenKind::Punctuation(']'));
                }
                TokenKind::Keyword(keyword) => {
                    self.handle_keyword(keyword, &token.lexeme);
                    self.last_token = Some(TokenKind::Keyword(keyword));
                }
                TokenKind::Identifier
                | TokenKind::NumberLiteral(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::CharLiteral(_) => {
                    self.write_space_if_needed();
                    self.push_text(&token.lexeme);
                    self.last_token = Some(kind.clone());
                }
                TokenKind::Operator(op) => {
                    self.handle_operator(op);
                    self.last_token = Some(TokenKind::Operator(op));
                }
                TokenKind::Unknown(ch) => {
                    if ch == '\n' {
                        self.write_newline();
                    } else {
                        self.write_space_if_needed();
                        self.push_text(&ch.to_string());
                    }
                    self.last_token = Some(TokenKind::Unknown(ch));
                }
                TokenKind::Punctuation(other) => {
                    self.handle_other_punctuation(other);
                    self.last_token = Some(TokenKind::Punctuation(other));
                }
            }
        }
        self.trim_trailing_newlines();
        if self.config.trailing_newline {
            if !self.output.ends_with(self.newline) {
                self.output.push_str(self.newline);
            }
        } else {
            while self.output.ends_with(self.newline) {
                let len = self.newline.len();
                let new_len = self.output.len().saturating_sub(len);
                self.output.truncate(new_len);
            }
        }
        if self.config.trim_trailing_whitespace {
            self.output = trim_trailing_whitespace(&self.output, self.newline);
        }
        self.output.clone()
    }

    fn handle_keyword(&mut self, keyword: Keyword, lexeme: &str) {
        match keyword {
            Keyword::Else => {
                if self.config.r#if.else_on_new_line && !self.at_line_start {
                    self.write_newline();
                } else {
                    self.write_space_if_needed();
                }
                self.push_text(lexeme);
                self.control_keyword = Some(keyword);
            }
            Keyword::Case | Keyword::Default => {
                if self.config.switch.blank_line_between_cases && self.last_case_label {
                    self.write_newline_if_needed();
                    self.write_newline();
                }
                let target_indent = self.indent_to_switch();
                if !self.at_line_start {
                    self.write_newline();
                }
                self.indent = target_indent;
                self.at_line_start = true;
                self.push_text(lexeme);
                let body_indent = target_indent.saturating_add(1);
                self.indent = body_indent;
                self.pending_space = true;
                self.control_keyword = Some(keyword);
                self.last_case_label = true;
            }
            Keyword::Switch => {
                self.write_space_if_needed();
                self.push_text(lexeme);
                self.control_keyword = Some(keyword);
                self.in_switch += 1;
                self.last_case_label = false;
            }
            Keyword::If | Keyword::For | Keyword::While | Keyword::Function | Keyword::Fn => {
                self.write_space_if_needed();
                self.push_text(lexeme);
                self.control_keyword = Some(keyword);
            }
            Keyword::Namespace
            | Keyword::Struct
            | Keyword::Union
            | Keyword::Enum
            | Keyword::Class
            | Keyword::Delegate
            | Keyword::Interface
            | Keyword::Extension
            | Keyword::Trait
            | Keyword::Impl
            | Keyword::Testcase => {
                self.ensure_separation_for_word();
                self.push_text(lexeme);
                self.control_keyword = Some(keyword);
            }
            _ => {
                self.ensure_separation_for_word();
                self.push_text(lexeme);
                self.control_keyword = None;
            }
        }
    }

    fn handle_operator(&mut self, op: &'static str) {
        match op {
            "." | "::" => {
                // No spacing for member access/qualification.
                self.flush_pending_space();
                self.push_text(op);
            }
            "#!" => {
                // Crate attributes must stay tight: `#![no_std]` (no space before `[`)
                self.flush_pending_space();
                self.push_text(op);
            }
            "!" => {
                // Unary not stays tight with its operand: `!value`
                self.write_space_if_needed();
                self.push_text(op);
            }
            _ => {
                if op == "=" && matches!(self.last_token, Some(TokenKind::Punctuation('>'))) {
                    self.pending_space = true;
                }
                self.write_space_if_needed();
                self.push_text(op);
                self.pending_space = true;
            }
        }
    }

    fn handle_other_punctuation(&mut self, ch: char) {
        match ch {
            '.' => {
                // Member/namespace access stays tight.
                self.flush_pending_space();
                self.push_text(".");
            }
            ':' => {
                if matches!(
                    self.control_keyword,
                    Some(Keyword::Case) | Some(Keyword::Default)
                ) {
                    self.flush_pending_space();
                    self.push_text(":");
                    self.write_newline();
                } else {
                    self.write_space_if_needed();
                    self.push_text(":");
                    self.pending_space = true;
                }
            }
            _ => {
                self.write_space_if_needed();
                self.push_text(&ch.to_string());
            }
        }
    }

    fn handle_open_paren(&mut self) {
        let needs_space = matches!(
            self.control_keyword,
            Some(Keyword::If | Keyword::While | Keyword::For | Keyword::Switch)
        ) && self.config.r#if.space_before_parentheses;
        if needs_space {
            self.pending_space = true;
        }
        self.flush_pending_space();
        self.push_text("(");
        self.paren_depth += 1;
        self.paren_indent.push(self.indent + 1);
    }

    fn handle_close_paren(&mut self) {
        if self.paren_depth > 0 {
            self.paren_depth -= 1;
        }
        self.paren_indent.pop();
        self.flush_pending_space();
        self.push_text(")");
    }

    fn handle_open_brace(&mut self) {
        let entering_switch = matches!(self.control_keyword, Some(Keyword::Switch));
        if matches!(
            self.control_keyword,
            Some(Keyword::Case) | Some(Keyword::Default)
        ) {
            // Case bodies should align their opening brace with the label.
            self.indent = self.indent_to_switch();
        }
        let style = self
            .config
            .switch
            .braces_style
            .unwrap_or(self.config.braces.style);
        let new_line_before = matches!(style, crate::format::BraceStyle::Allman)
            && !self.at_line_start
            && self.control_keyword.is_some();
        if new_line_before {
            self.write_newline();
        } else {
            self.write_space_if_needed();
        }
        self.flush_pending_space();
        self.push_text("{");
        self.brace_depth += 1;
        self.write_newline();
        self.indent += 1;
        if entering_switch {
            self.switch_indents.push(self.indent);
            self.last_case_label = false;
        }
        self.control_keyword = None;
    }

    fn handle_close_brace(&mut self) {
        let closing_switch = self.in_switch > 0 && self.brace_depth == self.in_switch;
        if closing_switch {
            if let Some(base_indent) = self.switch_indents.last().copied() {
                self.indent = base_indent.saturating_sub(1);
            }
        } else if self.indent > 0 {
            self.indent -= 1;
        }
        if !self.at_line_start {
            self.write_newline();
        }
        self.brace_depth = self.brace_depth.saturating_sub(1);
        self.push_text("}");
        self.control_keyword = None;
        self.just_closed_block = true;
        if self.in_switch > 0 && self.brace_depth < self.in_switch {
            self.in_switch -= 1;
            if let Some(base_indent) = self.switch_indents.pop() {
                self.indent = base_indent.saturating_sub(1);
            }
            self.last_case_label = false;
        }
    }

    fn handle_semicolon(&mut self) {
        self.flush_pending_space();
        self.push_text(";");
        if self.paren_depth == 0 {
            self.write_newline();
        } else {
            self.pending_space = true;
        }
        self.control_keyword = None;
    }

    fn handle_comma(&mut self) {
        self.flush_pending_space();
        self.push_text(",");
        if self.should_wrap_comma() {
            self.write_newline();
        } else {
            self.pending_space = true;
        }
    }

    fn should_wrap_comma(&self) -> bool {
        if self.paren_depth == 0 {
            return false;
        }
        let threshold = self.config.max_line_length;
        self.line_len >= threshold
    }

    fn emit_comment(&mut self, text: &str) {
        if !self.at_line_start {
            self.write_newline();
        }
        for (idx, line) in text.lines().enumerate() {
            if idx > 0 {
                self.write_newline();
            }
            self.write_indent();
            self.push_raw(line.trim_end());
        }
        self.write_newline();
        self.control_keyword = None;
    }

    fn emit_doc_comment(&mut self, text: &str) {
        if !self.at_line_start {
            self.write_newline();
        }
        for (idx, line) in text.lines().enumerate() {
            if idx > 0 {
                self.write_newline();
            }
            self.write_indent();
            let cleaned = line.trim_end();
            let line_text = if cleaned.starts_with("///") {
                cleaned.to_string()
            } else {
                format!("///{cleaned}")
            };
            self.push_raw(&line_text);
        }
        self.write_newline();
    }

    fn write_space_if_needed(&mut self) {
        let needs_space = match self.last_token {
            Some(TokenKind::Operator(op)) => op != "!",
            Some(TokenKind::Keyword(_))
            | Some(TokenKind::Identifier)
            | Some(TokenKind::NumberLiteral(_))
            | Some(TokenKind::StringLiteral(_))
            | Some(TokenKind::CharLiteral(_)) => true,
            Some(TokenKind::Punctuation(')')) => true,
            Some(TokenKind::Punctuation(']')) => true,
            Some(TokenKind::Punctuation('}')) => true,
            _ => false,
        };
        if needs_space {
            self.pending_space = true;
        }
        self.flush_pending_space();
    }

    fn ensure_separation_for_word(&mut self) {
        match self.last_token {
            Some(TokenKind::Keyword(_))
            | Some(TokenKind::Identifier)
            | Some(TokenKind::NumberLiteral(_))
            | Some(TokenKind::StringLiteral(_))
            | Some(TokenKind::CharLiteral(_)) => self.pending_space = true,
            Some(TokenKind::Punctuation(')')) | Some(TokenKind::Punctuation(']')) => {
                self.pending_space = true;
            }
            _ => {}
        }
        self.flush_pending_space();
    }

    fn flush_pending_space(&mut self) {
        if self.pending_space && !self.at_line_start {
            self.push_raw(" ");
        }
        self.pending_space = false;
    }

    fn write_indent(&mut self) {
        if !self.at_line_start {
            return;
        }
        let unit_len = self.indent_unit.len();
        for _ in 0..self.indent {
            self.output.push_str(&self.indent_unit);
            self.line_len += unit_len;
        }
        self.at_line_start = false;
    }

    fn push_text(&mut self, text: &str) {
        self.write_indent();
        self.output.push_str(text);
        self.line_len += text.len();
        self.at_line_start = false;
    }

    fn push_raw(&mut self, text: &str) {
        self.output.push_str(text);
        if self.at_line_start {
            self.line_len += text.len();
        } else {
            self.line_len += text.len();
        }
        self.at_line_start = false;
    }

    fn write_newline(&mut self) {
        self.output.push_str(self.newline);
        self.line_len = 0;
        self.at_line_start = true;
        self.pending_space = false;
    }

    fn write_newline_if_needed(&mut self) {
        if !self.at_line_start {
            self.write_newline();
        }
    }

    fn indent_to_switch(&self) -> usize {
        if self.in_switch > 0 {
            if let Some(base_indent) = self.switch_indents.last().copied() {
                base_indent
                    .saturating_sub(1)
                    .saturating_add(self.config.switch.case_indent as usize)
            } else {
                self.indent
            }
        } else {
            self.indent
        }
    }

    fn trim_trailing_newlines(&mut self) {
        while self.output.ends_with(self.newline) {
            let len = self.newline.len();
            let new_len = self.output.len().saturating_sub(len);
            self.output.truncate(new_len);
        }
    }
}

fn trim_trailing_whitespace(input: &str, newline: &str) -> String {
    let mut remaining = input;
    let mut result = String::new();
    while let Some(idx) = remaining.find(newline) {
        let (line, tail) = remaining.split_at(idx);
        result.push_str(line.trim_end_matches(|ch: char| ch == ' ' || ch == '\t'));
        result.push_str(newline);
        remaining = &tail[newline.len()..];
    }
    result.push_str(remaining.trim_end_matches(|ch: char| ch == ' ' || ch == '\t'));
    result
}

fn reorder_imports(tokens: Vec<Token>, config: &FormatConfig) -> Vec<Token> {
    if !config.usings.sort && !config.usings.blank_line_between_groups {
        return tokens;
    }
    let mut idx = 0;
    let mut prefix = Vec::new();
    let mut pending_leading = Vec::new();
    while idx < tokens.len() {
        match &tokens[idx].kind {
            TokenKind::Whitespace => {
                idx += 1;
            }
            TokenKind::Comment | TokenKind::DocComment => {
                pending_leading.push(tokens[idx].clone());
                idx += 1;
            }
            kind if is_import_keyword(kind) => {
                break;
            }
            _ => {
                return tokens;
            }
        }
    }

    let mut imports: Vec<Vec<Token>> = Vec::new();
    while idx < tokens.len() {
        match &tokens[idx].kind {
            TokenKind::Whitespace => {
                idx += 1;
            }
            TokenKind::Comment | TokenKind::DocComment => {
                pending_leading.push(tokens[idx].clone());
                idx += 1;
            }
            kind if is_import_keyword(kind) => {
                let start = idx;
                idx += 1;
                while idx < tokens.len() {
                    if matches!(tokens[idx].kind, TokenKind::Punctuation(';')) {
                        idx += 1;
                        break;
                    }
                    idx += 1;
                }
                let mut block = Vec::new();
                block.extend(pending_leading.drain(..));
                block.extend_from_slice(&tokens[start..idx]);
                normalize_import_block(&mut block);
                imports.push(block);
            }
            _ => {
                prefix.extend(pending_leading.drain(..));
                break;
            }
        }
    }

    if imports.is_empty() {
        return tokens;
    }

    if config.usings.sort {
        imports.sort_by(|a, b| {
            let group_a = import_group(a, config.usings.group);
            let group_b = import_group(b, config.usings.group);
            group_a
                .cmp(&group_b)
                .then_with(|| import_key(a).cmp(&import_key(b)))
        });
    }

    let mut reordered = Vec::new();
    reordered.extend(prefix);
    let mut previous_group: Option<u8> = None;
    for block in imports {
        let group = import_group(&block, config.usings.group);
        if config.usings.blank_line_between_groups {
            if let Some(prev) = previous_group {
                let should_separate = if config.usings.sort {
                    prev != group
                } else {
                    group < prev
                };
                if should_separate {
                    reordered.push(Token {
                        kind: TokenKind::Unknown('\n'),
                        lexeme: newline_str(config).to_string(),
                        span: crate::frontend::diagnostics::Span::new(0, 0),
                    });
                }
            }
        }
        reordered.extend(block);
        previous_group = Some(group);
    }
    reordered.extend_from_slice(&tokens[idx..]);
    reordered
}

fn is_import_keyword(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Keyword(Keyword::Import) | TokenKind::Keyword(Keyword::Using)
    )
}

fn normalize_import_block(tokens: &mut [Token]) {
    if let Some(keyword) = tokens
        .iter_mut()
        .find(|token| is_import_keyword(&token.kind))
    {
        if matches!(keyword.kind, TokenKind::Keyword(Keyword::Using)) {
            keyword.kind = TokenKind::Keyword(Keyword::Import);
            keyword.lexeme = "import".to_string();
        }
    }
}

fn import_group(tokens: &[Token], group: crate::format::UsingGroup) -> u8 {
    let ident = leading_identifier(tokens).unwrap_or_default();
    match group {
        crate::format::UsingGroup::None => 0,
        crate::format::UsingGroup::SystemFirst => {
            if ident.eq_ignore_ascii_case("system") {
                0
            } else {
                1
            }
        }
        crate::format::UsingGroup::StdFirst => {
            if ident.eq_ignore_ascii_case("std") {
                0
            } else {
                1
            }
        }
        crate::format::UsingGroup::Custom => 0,
    }
}

fn import_key(tokens: &[Token]) -> String {
    let mut key = String::new();
    for token in tokens {
        match token.kind {
            TokenKind::Whitespace | TokenKind::Comment | TokenKind::DocComment => continue,
            _ => key.push_str(token.lexeme.as_str()),
        }
    }
    key
}

fn leading_identifier(tokens: &[Token]) -> Option<String> {
    let mut seen_import = false;
    for token in tokens {
        match token.kind {
            TokenKind::Keyword(Keyword::Using) | TokenKind::Keyword(Keyword::Import) => {
                seen_import = true
            }
            TokenKind::Identifier if seen_import => return Some(token.lexeme.clone()),
            _ => {}
        }
    }
    None
}

fn newline_str(config: &FormatConfig) -> &'static str {
    match config.newline {
        NewlineStyle::Lf => "\n",
        NewlineStyle::Crlf => "\r\n",
    }
}

fn collect_metadata(module: &crate::frontend::ast::items::Module) -> FormatMetadata {
    let mut names = Vec::new();
    let mut types = Vec::new();
    collect_top_level_types(&module.items, &mut names, &mut types);
    FormatMetadata {
        namespace: module
            .namespace
            .clone()
            .or_else(|| first_namespace(&module.items)),
        top_level_types: names,
        types,
    }
}

fn collect_top_level_types(
    items: &[crate::frontend::ast::items::Item],
    names: &mut Vec<String>,
    out: &mut Vec<TypeMetadata>,
) {
    for item in items {
        match item {
            crate::frontend::ast::items::Item::Struct(decl) => {
                names.push(decl.name.clone());
                out.push(TypeMetadata {
                    name: decl.name.clone(),
                    kind: TypeSort::Struct,
                    members: struct_member_kinds(decl),
                });
            }
            crate::frontend::ast::items::Item::Union(decl) => {
                names.push(decl.name.clone());
                out.push(TypeMetadata {
                    name: decl.name.clone(),
                    kind: TypeSort::Struct,
                    members: union_member_kinds(decl),
                });
            }
            crate::frontend::ast::items::Item::Enum(decl) => {
                names.push(decl.name.clone());
                out.push(TypeMetadata {
                    name: decl.name.clone(),
                    kind: TypeSort::Enum,
                    members: enum_member_kinds(),
                });
            }
            crate::frontend::ast::items::Item::Class(decl) => {
                names.push(decl.name.clone());
                out.push(TypeMetadata {
                    name: decl.name.clone(),
                    kind: TypeSort::Class,
                    members: class_member_kinds(decl),
                });
            }
            crate::frontend::ast::items::Item::Interface(decl) => {
                names.push(decl.name.clone());
                out.push(TypeMetadata {
                    name: decl.name.clone(),
                    kind: TypeSort::Interface,
                    members: interface_member_kinds(decl),
                });
            }
            crate::frontend::ast::items::Item::Delegate(decl) => {
                names.push(decl.name.clone());
                out.push(TypeMetadata {
                    name: decl.name.clone(),
                    kind: TypeSort::Delegate,
                    members: Vec::new(),
                });
            }
            crate::frontend::ast::items::Item::Trait(decl) => {
                names.push(decl.name.clone());
                out.push(TypeMetadata {
                    name: decl.name.clone(),
                    kind: TypeSort::Trait,
                    members: trait_member_kinds(decl),
                });
            }
            crate::frontend::ast::items::Item::Extension(decl) => {
                names.push(extension_name(&decl.target));
                out.push(TypeMetadata {
                    name: extension_name(&decl.target),
                    kind: TypeSort::Extension,
                    members: extension_member_kinds(decl),
                });
            }
            crate::frontend::ast::items::Item::Impl(decl) => {
                names.push(impl_name(&decl.target));
                out.push(TypeMetadata {
                    name: impl_name(&decl.target),
                    kind: TypeSort::Impl,
                    members: impl_member_kinds(decl),
                });
            }
            crate::frontend::ast::items::Item::Namespace(ns) => {
                collect_top_level_types(&ns.items, names, out)
            }
            _ => {}
        }
    }
}

fn first_namespace(items: &[crate::frontend::ast::items::Item]) -> Option<String> {
    items.iter().find_map(|item| {
        if let crate::frontend::ast::items::Item::Namespace(ns) = item {
            Some(ns.name.clone())
        } else {
            None
        }
    })
}

fn struct_member_kinds(decl: &crate::frontend::ast::items::StructDecl) -> Vec<MemberSort> {
    let mut kinds = Vec::new();
    if !decl.fields.is_empty() {
        kinds.extend(std::iter::repeat(MemberSort::Fields).take(decl.fields.len()));
    }
    if !decl.properties.is_empty() {
        kinds.extend(std::iter::repeat(MemberSort::Properties).take(decl.properties.len()));
    }
    if !decl.constructors.is_empty() {
        kinds.extend(std::iter::repeat(MemberSort::Constructors).take(decl.constructors.len()));
    }
    if !decl.consts.is_empty() {
        kinds.extend(std::iter::repeat(MemberSort::Consts).take(decl.consts.len()));
    }
    if !decl.methods.is_empty() {
        kinds.extend(std::iter::repeat(MemberSort::Methods).take(decl.methods.len()));
    }
    if !decl.nested_types.is_empty() {
        kinds.extend(std::iter::repeat(MemberSort::NestedTypes).take(decl.nested_types.len()));
    }
    kinds
}

fn union_member_kinds(_decl: &crate::frontend::ast::items::UnionDecl) -> Vec<MemberSort> {
    Vec::new()
}

fn enum_member_kinds() -> Vec<MemberSort> {
    Vec::new()
}

fn class_member_kinds(decl: &crate::frontend::ast::items::ClassDecl) -> Vec<MemberSort> {
    let mut kinds = Vec::new();
    for member in &decl.members {
        match member {
            crate::frontend::ast::items::ClassMember::Field(_) => kinds.push(MemberSort::Fields),
            crate::frontend::ast::items::ClassMember::Property(prop) => {
                if prop.is_static {
                    kinds.push(MemberSort::Statics);
                } else {
                    kinds.push(MemberSort::Properties);
                }
            }
            crate::frontend::ast::items::ClassMember::Method(method) => {
                if method
                    .modifiers
                    .iter()
                    .any(|m| m.eq_ignore_ascii_case("static"))
                {
                    kinds.push(MemberSort::Statics);
                } else {
                    kinds.push(MemberSort::Methods);
                }
            }
            crate::frontend::ast::items::ClassMember::Constructor(_) => {
                kinds.push(MemberSort::Constructors)
            }
            crate::frontend::ast::items::ClassMember::Const(_) => kinds.push(MemberSort::Consts),
        }
    }
    if !decl.nested_types.is_empty() {
        kinds.extend(std::iter::repeat(MemberSort::NestedTypes).take(decl.nested_types.len()));
    }
    kinds
}

fn interface_member_kinds(decl: &crate::frontend::ast::items::InterfaceDecl) -> Vec<MemberSort> {
    let mut kinds = Vec::new();
    for member in &decl.members {
        match member {
            crate::frontend::ast::items::InterfaceMember::Method(_) => {
                kinds.push(MemberSort::Methods)
            }
            crate::frontend::ast::items::InterfaceMember::Property(prop) => {
                if prop.is_static {
                    kinds.push(MemberSort::Statics);
                } else {
                    kinds.push(MemberSort::Properties);
                }
            }
            crate::frontend::ast::items::InterfaceMember::AssociatedType(_) => {
                kinds.push(MemberSort::NestedTypes)
            }
            crate::frontend::ast::items::InterfaceMember::Const(_) => {
                kinds.push(MemberSort::Consts)
            }
        }
    }
    kinds
}

fn trait_member_kinds(decl: &crate::frontend::ast::items::TraitDecl) -> Vec<MemberSort> {
    let mut kinds = Vec::new();
    for member in &decl.members {
        match member {
            crate::frontend::ast::items::TraitMember::Method(_) => kinds.push(MemberSort::Methods),
            crate::frontend::ast::items::TraitMember::AssociatedType(_) => {
                kinds.push(MemberSort::NestedTypes)
            }
            crate::frontend::ast::items::TraitMember::Const(_) => kinds.push(MemberSort::Consts),
        }
    }
    kinds
}

fn extension_member_kinds(decl: &crate::frontend::ast::items::ExtensionDecl) -> Vec<MemberSort> {
    decl.members.iter().map(|_| MemberSort::Methods).collect()
}

fn impl_member_kinds(decl: &crate::frontend::ast::items::ImplDecl) -> Vec<MemberSort> {
    decl.members
        .iter()
        .map(|member| match member {
            crate::frontend::ast::items::ImplMember::Method(func) => {
                if func
                    .modifiers
                    .iter()
                    .any(|m| m.eq_ignore_ascii_case("static"))
                {
                    MemberSort::Statics
                } else {
                    MemberSort::Methods
                }
            }
            crate::frontend::ast::items::ImplMember::AssociatedType(_) => MemberSort::NestedTypes,
            crate::frontend::ast::items::ImplMember::Const(_) => MemberSort::Consts,
        })
        .collect()
}

fn extension_name(target: &crate::frontend::ast::types::TypeExpr) -> String {
    format!("extension<{target:?}>")
}

fn impl_name(target: &crate::frontend::ast::types::TypeExpr) -> String {
    format!("impl<{target:?}>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{BraceStyle, UsingGroup};

    #[test]
    fn formats_class_allman_style() {
        let source = "class Sample { fn Main(){ return; } }";
        let expected = "\
class Sample
{
    fn Main()
    {
        return;
    }
}
";
        let output = format_source(source, &FormatConfig::default())
            .expect("format")
            .formatted;
        assert_eq!(output, expected);
        let second = format_source(&output, &FormatConfig::default())
            .expect("format")
            .formatted;
        assert_eq!(second, output);
    }

    #[test]
    fn formats_if_else_kandr() {
        let mut config = FormatConfig::default();
        config.braces.style = BraceStyle::KAndR;
        config.r#if.else_on_new_line = false;
        let source = "fn Main(){ if(x){return;} else{ return; } }";
        let expected = "\
fn Main() {
    if (x) {
        return;
    } else {
        return;
    }
}
";
        let output = format_source(source, &config).expect("format").formatted;
        assert_eq!(output, expected);
    }

    #[test]
    fn preserves_comments_and_docs() {
        let source = "/// summary\nclass C{ //leading\nfn Go(){//do\nreturn;}}\n";
        let expected = "\
/// summary
class C
{
    //leading
    fn Go()
    {
        //do
        return;
    }
}
";
        let output = format_source(source, &FormatConfig::default())
            .expect("format")
            .formatted;
        assert_eq!(output, expected);
    }

    #[test]
    fn formats_switch_cases_with_body_indent() {
        let source = "switch(x){case 0:return;case 1:{return;}}";
        let expected = "\
switch (x)
{
    case 0:
        return;
    case 1:
    {
        return;
    }
}
";
        let output = format_source(source, &FormatConfig::default())
            .expect("format")
            .formatted;
        assert_eq!(output, expected);
    }

    #[test]
    fn switch_can_separate_cases() {
        let mut config = FormatConfig::default();
        config.switch.blank_line_between_cases = true;
        let source = "switch(x){case 0:return;case 1:return;}";
        let expected = "\
switch (x)
{
    case 0:
        return;

    case 1:
        return;
}
";
        let output = format_source(source, &config).expect("format").formatted;
        assert_eq!(output, expected);
    }

    #[test]
    fn collects_metadata_for_file_org() {
        let source = "namespace Sample{class One{} class Two{}}";
        let outcome = format_source(source, &FormatConfig::default()).expect("format");
        assert_eq!(
            outcome.metadata.top_level_types,
            vec!["One".to_string(), "Two".to_string()]
        );
        assert_eq!(outcome.metadata.namespace, Some("Sample".into()));
    }

    #[test]
    fn sorts_import_directives_with_grouping() {
        let mut config = FormatConfig::default();
        config.usings.sort = true;
        config.usings.group = UsingGroup::SystemFirst;
        config.usings.blank_line_between_groups = true;
        let source = "using Zeta; using System.IO; using Abc.Core;";
        let expected = "\
import System.IO;

import Abc.Core;
import Zeta;
";
        let output = format_source(source, &config).expect("format").formatted;
        assert_eq!(output, expected);
    }

    #[test]
    fn preserves_import_order_when_sort_disabled() {
        let mut config = FormatConfig::default();
        config.usings.sort = false;
        config.usings.group = UsingGroup::SystemFirst;
        config.usings.blank_line_between_groups = true;
        let source = "using Zeta; using System.IO; using Abc.Core;";
        let expected = "\
import Zeta;

import System.IO;
import Abc.Core;
";
        let output = format_source(source, &config).expect("format").formatted;
        assert_eq!(output, expected);
    }
}
