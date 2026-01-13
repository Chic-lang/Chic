use super::*;

parser_impl! {
    pub(super) fn parse_region_statement(
        &mut self,
        start_pos: Option<usize>,
    ) -> Option<Statement> {
        let name = self.consume_identifier("expected region name after 'region'")?;
        let body = self.parse_block()?;
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
            StatementKind::Region { name, body },
        ))
    }
}
