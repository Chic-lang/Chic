use super::*;

enum BindingBase {
    Let,
    Var,
    Typed(String),
}

#[expect(
    clippy::too_many_lines,
    reason = "Foreach binding parsing remains monolithic until lexer tokens expose richer structure"
)]
pub(crate) fn parse_foreach_binding(
    binding_text: &str,
    span: Option<Span>,
) -> Result<ForeachBindingInfo, LoweringDiagnostic> {
    let tokens = lex(binding_text)
        .tokens
        .into_iter()
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Comment | TokenKind::Unknown(_)
            )
        })
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        return Err(LoweringDiagnostic {
            message: "foreach binding requires an identifier".into(),
            span,
        });
    }

    let mut index = 0usize;
    let mut mode = ForeachBindingMode::Value;

    if let Some(Token {
        kind: TokenKind::Keyword(keyword),
        ..
    }) = tokens.get(index)
    {
        match keyword {
            Keyword::In => {
                mode = ForeachBindingMode::In;
                index += 1;
            }
            Keyword::Ref => {
                mode = ForeachBindingMode::Ref;
                index += 1;
                if tokens.get(index).is_some_and(|next| {
                    matches!(next.kind, TokenKind::Identifier)
                        && next.lexeme.eq_ignore_ascii_case("readonly")
                }) {
                    mode = ForeachBindingMode::RefReadonly;
                    index += 1;
                }
            }
            _ => {}
        }
    }

    if index >= tokens.len() {
        return Err(LoweringDiagnostic {
            message: "foreach binding requires a variable name".into(),
            span,
        });
    }

    let name_index;
    let base = match &tokens[index].kind {
        TokenKind::Keyword(Keyword::Let) => {
            index += 1;
            if index >= tokens.len() {
                return Err(LoweringDiagnostic {
                    message: "expected identifier after `let` in foreach binding".into(),
                    span,
                });
            }
            name_index = index;
            BindingBase::Let
        }
        TokenKind::Keyword(Keyword::Var) => {
            index += 1;
            if index >= tokens.len() {
                return Err(LoweringDiagnostic {
                    message: "expected identifier after `var` in foreach binding".into(),
                    span,
                });
            }
            name_index = index;
            BindingBase::Var
        }
        _ => {
            if tokens.len() - index < 2 {
                return Err(LoweringDiagnostic {
                    message: "typed foreach binding requires `Type Identifier`".into(),
                    span,
                });
            }
            name_index = tokens.len() - 1;
            let type_tokens = &tokens[index..name_index];
            let type_text = type_tokens
                .iter()
                .map(|tok| tok.lexeme.as_str())
                .collect::<String>();
            if type_text.is_empty() {
                return Err(LoweringDiagnostic {
                    message: "could not parse type in foreach binding".into(),
                    span,
                });
            }
            BindingBase::Typed(type_text)
        }
    };

    let name_token = tokens.get(name_index).ok_or_else(|| LoweringDiagnostic {
        message: "foreach binding requires an identifier".into(),
        span,
    })?;

    if !matches!(name_token.kind, TokenKind::Identifier) {
        return Err(LoweringDiagnostic {
            message: "foreach binding name must be an identifier".into(),
            span: Some(name_token.span),
        });
    }

    if name_index + 1 < tokens.len() {
        return Err(LoweringDiagnostic {
            message: "foreach binding does not allow trailing tokens".into(),
            span,
        });
    }

    if matches!(base, BindingBase::Let)
        && matches!(
            mode,
            ForeachBindingMode::Ref | ForeachBindingMode::RefReadonly
        )
    {
        return Err(LoweringDiagnostic {
            message: "`let` foreach bindings cannot use `ref` modifiers".into(),
            span,
        });
    }

    let ty = match &base {
        BindingBase::Typed(text) => Ty::named(text.clone()),
        _ => Ty::Unknown,
    };

    let mutable = matches!(mode, ForeachBindingMode::Ref);

    Ok(ForeachBindingInfo {
        mode,
        mutable,
        ty,
        name: name_token.lexeme.clone(),
    })
}
