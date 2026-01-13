use super::*;

parser_impl! {
    pub(in crate::frontend::parser) fn collect_union_members(&mut self) -> Vec<UnionMember> {
        let mut members = Vec::new();
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            self.skip_attributes();
            self.stash_leading_doc();
            if self.check_punctuation('}') {
                break;
            }

            let doc = self.take_pending_doc();
            let visibility = self.parse_visibility();
            let is_readonly = self.consume_union_storage_modifiers();
            let info = UnionMemberInfo {
                visibility,
                is_readonly,
                doc,
            };

            // LL1_ALLOW: Union views use the same `record struct` sugar as items and require matching lookahead (docs/compiler/parser.md#ll1-allowances).
            if self.peek_identifier("record") && self.peek_keyword_n(1, Keyword::Struct) {
                self.advance();
                self.advance();
                self.push_union_view_member(&mut members, info);
                continue;
            }

            if self.match_keyword(Keyword::Struct) {
                self.push_union_view_member(&mut members, info);
                continue;
            }

            self.parse_union_field_member(&mut members, info);
        }
        members
    }

    fn push_union_view_member(&mut self, members: &mut Vec<UnionMember>, info: UnionMemberInfo) {
        match self.parse_union_view(info.visibility, info.is_readonly, info.doc) {
            Some(view) => members.push(UnionMember::View(view)),
            None => self.recover_union_member(),
        }
    }

    fn parse_union_field_member(&mut self, members: &mut Vec<UnionMember>, info: UnionMemberInfo) {
        let Some(field_type) = self.parse_type_expr() else {
            self.synchronize_field();
            return;
        };

        let Some(field_name) = self.consume_identifier("expected union field name") else {
            self.synchronize_field();
            return;
        };

        if !self.expect_punctuation(';') {
            self.synchronize_field();
            return;
        }

        members.push(UnionMember::Field(UnionField {
            visibility: info.visibility,
            name: field_name,
            ty: field_type,
            is_readonly: info.is_readonly,
            doc: info.doc,
            attributes: Vec::new(),
        }));
    }

    fn parse_union_view(
        &mut self,
        visibility: Visibility,
        is_readonly: bool,
        doc: Option<DocComment>,
    ) -> Option<UnionViewDecl> {
        let name = self.consume_identifier("expected union view name")?;
        if !self.expect_punctuation('{') {
            return None;
        }
        let fields = self.parse_union_view_fields();
        if !self.expect_punctuation('}') {
            return None;
        }
        Some(UnionViewDecl {
            visibility,
            name,
            fields,
            is_readonly,
            doc,
            attributes: Vec::new(),
        })
    }

    fn parse_union_view_fields(&mut self) -> Vec<FieldDecl> {
        let mut fields = Vec::new();
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            self.skip_attributes();
            self.stash_leading_doc();
            if self.check_punctuation('}') {
                break;
            }
            let field_doc = self.take_pending_doc();
            let field_visibility = self.parse_visibility();
            self.consume_all_borrow_qualifier_misuse(false);
            let Some(field_type) = self.parse_type_expr() else {
                self.synchronize_field();
                continue;
            };

            let Some(field_name) = self.consume_identifier("expected field name") else {
                self.synchronize_field();
                continue;
            };

            if !self.expect_punctuation(';') {
                self.synchronize_field();
                continue;
            }

            fields.push(FieldDecl {
                visibility: field_visibility,
                name: field_name,
                ty: field_type,
                initializer: None,
                doc: field_doc,
                attributes: Vec::new(),
                mmio: None,
                is_required: false,
                display_name: None,
                is_readonly: false,
                is_static: false,
                view_of: None,
            });
        }
        fields
    }

    fn recover_union_member(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation(';') => {
                    self.advance();
                    break;
                }
                TokenKind::Punctuation('}') => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn consume_union_storage_modifiers(&mut self) -> bool {
        let mut is_readonly = false;
        loop {
            if self.consume_all_borrow_qualifier_misuse(true) {
                continue;
            }
            if self.match_keyword(Keyword::Readonly) {
                is_readonly = true;
                continue;
            }
            break;
        }
        is_readonly
    }
}
