//! Resaltado sintáctico del editor: reutiliza el lexer real del compilador,
//! así que lo que se colorea es exactamente lo que el compilador tokeniza.

use eframe::egui::{Color32, FontId, TextFormat, text::LayoutJob};
use hulk_compiler::lexer::{Lexer, Token, TokenKind};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightRole {
    Keyword,
    BuiltinFunction,
    FunctionName,
    Variable,
    Number,
    String,
    Boolean,
    Operator,
    Unknown,
    Plain,
}

/// Color del tema asociado a cada rol (editor, tabla de tokens, etc.).
pub fn role_color(role: HighlightRole, theme: &Theme) -> Color32 {
    match role {
        HighlightRole::Keyword => theme.keyword,
        HighlightRole::BuiltinFunction | HighlightRole::FunctionName => theme.function,
        HighlightRole::Variable => theme.variable,
        HighlightRole::Number => theme.number,
        HighlightRole::String => theme.string,
        HighlightRole::Boolean => theme.boolean,
        HighlightRole::Operator => theme.operator,
        HighlightRole::Unknown => theme.unknown,
        HighlightRole::Plain => theme.text,
    }
}

pub fn text_format(font_size: f32, color: Color32) -> TextFormat {
    TextFormat {
        font_id: FontId::monospace(font_size),
        color,
        ..Default::default()
    }
}

pub fn hulk_highlight_job(source: &str, font_size: f32, theme: &Theme) -> LayoutJob {
    let mut job = LayoutJob::default();

    let normal = text_format(font_size, theme.text);
    let keyword = text_format(font_size, theme.keyword);
    let function = text_format(font_size, theme.function);
    let variable = text_format(font_size, theme.variable);
    let number = text_format(font_size, theme.number);
    let string = text_format(font_size, theme.string);
    let boolean = text_format(font_size, theme.boolean);
    let operator = text_format(font_size, theme.operator);
    let unknown = text_format(font_size, theme.unknown);

    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.lex();

    let mut cursor = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.start > cursor {
            job.append(&source[cursor..token.start], 0.0, normal.clone());
        }

        if token.end > token.start {
            let piece = &source[token.start..token.end];
            let format = match classify_highlight_role(&tokens, idx) {
                HighlightRole::Keyword => keyword.clone(),
                HighlightRole::BuiltinFunction | HighlightRole::FunctionName => function.clone(),
                HighlightRole::Variable => variable.clone(),
                HighlightRole::Number => number.clone(),
                HighlightRole::String => string.clone(),
                HighlightRole::Boolean => boolean.clone(),
                HighlightRole::Operator => operator.clone(),
                HighlightRole::Unknown => unknown.clone(),
                HighlightRole::Plain => normal.clone(),
            };
            job.append(piece, 0.0, format);
        }

        cursor = token.end;
    }

    if cursor < source.len() {
        job.append(&source[cursor..], 0.0, normal);
    }

    job
}

pub fn classify_highlight_role(tokens: &[Token], idx: usize) -> HighlightRole {
    let kind = &tokens[idx].kind;
    let prev_kind = idx.checked_sub(1).map(|i| &tokens[i].kind);
    let next_kind = tokens.get(idx + 1).map(|t| &t.kind);

    match kind {
        TokenKind::Let
        | TokenKind::Function
        | TokenKind::Define
        | TokenKind::Type
        | TokenKind::Interface
        | TokenKind::Extends
        | TokenKind::New
        | TokenKind::While
        | TokenKind::For
        | TokenKind::Range
        | TokenKind::In
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::Elif
        | TokenKind::Inherits
        | TokenKind::Is
        | TokenKind::As => HighlightRole::Keyword,
        TokenKind::Print
        | TokenKind::Sin
        | TokenKind::Cos
        | TokenKind::Sqrt
        | TokenKind::Exp
        | TokenKind::Log
        | TokenKind::Rand => HighlightRole::BuiltinFunction,
        TokenKind::Pi | TokenKind::E => HighlightRole::FunctionName,
        TokenKind::Number(_) => HighlightRole::Number,
        TokenKind::String(_) => HighlightRole::String,
        TokenKind::Boolean(_) | TokenKind::Null => HighlightRole::Boolean,
        TokenKind::Identifier(_) => {
            let is_declaration_name = matches!(
                prev_kind,
                Some(TokenKind::Function) | Some(TokenKind::Define)
            );
            let is_call_name = matches!(next_kind, Some(TokenKind::LeftParen));
            if is_declaration_name || is_call_name {
                HighlightRole::FunctionName
            } else {
                HighlightRole::Variable
            }
        }
        TokenKind::Unknown => HighlightRole::Unknown,
        TokenKind::Assign
        | TokenKind::Arrow
        | TokenKind::ThinArrow
        | TokenKind::LeftBracket
        | TokenKind::RightBracket
        | TokenKind::Add
        | TokenKind::Power
        | TokenKind::Concat
        | TokenKind::ConcatSpace
        | TokenKind::Minus
        | TokenKind::Multiply
        | TokenKind::Divide
        | TokenKind::Mod
        | TokenKind::EqualEqual
        | TokenKind::NotEqual
        | TokenKind::Less
        | TokenKind::Greater
        | TokenKind::LessEqual
        | TokenKind::GreaterEqual
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Not
        | TokenKind::DestructiveAssign
        | TokenKind::Colon
        | TokenKind::Comma
        | TokenKind::Semicolon
        | TokenKind::Dot
        | TokenKind::LeftBrace
        | TokenKind::RightBrace
        | TokenKind::LeftParen
        | TokenKind::RightParen => HighlightRole::Operator,
        TokenKind::EOF => HighlightRole::Plain,
    }
}
