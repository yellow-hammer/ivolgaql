//! Лексер и парсер подмножества ИволгаQL.

use std::fmt;

use crate::engine::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone)]
pub enum Token {
    Eof,
    Ident(String),
    Star,
    Comma,
    LParen,
    RParen,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    StringLit(String),
    Int(i64),
    Float(f64),
}

#[derive(Debug, Clone)]
pub struct CreateTable {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InsertRow {
    pub table: String,
    pub values: Vec<Cell>,
}

#[derive(Debug, Clone)]
pub struct CmpExpr {
    pub left: String,
    pub op: CmpOp,
    pub right: ExprValue,
}

#[derive(Debug, Clone)]
pub enum ExprValue {
    Literal(Cell),
    Ident(String),
}

#[derive(Debug, Clone)]
pub struct SelectQuery {
    /// `None` — звезда
    pub columns: Option<Vec<String>>,
    pub table: String,
    pub r#where: Vec<CmpExpr>,
}

#[derive(Debug)]
pub enum ParsedQuery {
    Help,
    Create(CreateTable),
    Insert(InsertRow),
    Select(SelectQuery),
}

#[derive(Debug)]
pub struct LexError(pub String);

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LexError {}

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub enum QueryError {
    Lex(String),
    Parse(String),
}

impl From<LexError> for QueryError {
    fn from(e: LexError) -> Self {
        QueryError::Lex(e.0)
    }
}

impl From<ParseError> for QueryError {
    fn from(e: ParseError) -> Self {
        QueryError::Parse(e.0)
    }
}

fn is_keyword(low: &str) -> bool {
    matches!(
        low,
        "выбрать"
            | "из"
            | "где"
            | "и"
            | "или"
            | "вставить"
            | "в"
            | "значения"
            | "создать"
            | "таблицу"
            | "помощь"
    )
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == 'Ё' || c == 'ё'
}

fn cmp_from_tok(t: &Token) -> Option<CmpOp> {
    match t {
        Token::Eq => Some(CmpOp::Eq),
        Token::Ne => Some(CmpOp::Ne),
        Token::Lt => Some(CmpOp::Lt),
        Token::Gt => Some(CmpOp::Gt),
        Token::Le => Some(CmpOp::Le),
        Token::Ge => Some(CmpOp::Ge),
        _ => None,
    }
}

pub fn lex(query: &str) -> Result<Vec<Token>, LexError> {
    let s: Vec<char> = query.trim().chars().collect();
    let n = s.len();
    let mut i = 0usize;
    let mut out: Vec<Token> = Vec::new();

    let peek = |i: usize, off: usize| -> Option<char> {
        let j = i + off;
        if j < n {
            Some(s[j])
        } else {
            None
        }
    };

    while i < n {
        let c = s[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '*' {
            out.push(Token::Star);
            i += 1;
            continue;
        }
        if c == ',' {
            out.push(Token::Comma);
            i += 1;
            continue;
        }
        if c == '(' {
            out.push(Token::LParen);
            i += 1;
            continue;
        }
        if c == ')' {
            out.push(Token::RParen);
            i += 1;
            continue;
        }
        if matches!(c, '<' | '>' | '=' | '!') {
            if c == '<' && peek(i, 1) == Some('=') {
                out.push(Token::Le);
                i += 2;
                continue;
            }
            if c == '>' && peek(i, 1) == Some('=') {
                out.push(Token::Ge);
                i += 2;
                continue;
            }
            if c == '<' && peek(i, 1) == Some('>') {
                out.push(Token::Ne);
                i += 2;
                continue;
            }
            if c == '!' {
                if peek(i, 1) != Some('=') {
                    return Err(LexError("ожидалось != после !".into()));
                }
                out.push(Token::Ne);
                i += 2;
                continue;
            }
            if c == '<' {
                out.push(Token::Lt);
                i += 1;
                continue;
            }
            if c == '>' {
                out.push(Token::Gt);
                i += 1;
                continue;
            }
            if c == '=' {
                out.push(Token::Eq);
                i += 1;
                continue;
            }
        }
        if c == '\'' {
            let mut buf = String::new();
            i += 1;
            let mut closed = false;
            while i < n {
                if s[i] == '\'' {
                    if i + 1 < n && s[i + 1] == '\'' {
                        buf.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    closed = true;
                    break;
                }
                buf.push(s[i]);
                i += 1;
            }
            if !closed {
                return Err(LexError("незакрытая строковая константа".into()));
            }
            out.push(Token::StringLit(buf));
            continue;
        }
        if c.is_ascii_digit() || (c == '-' && peek(i, 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = i;
            if s[i] == '-' {
                i += 1;
            }
            while i < n && (s[i].is_ascii_digit() || s[i] == '.') {
                i += 1;
            }
            let num_s: String = s[start..i].iter().collect();
            if num_s.contains('.') {
                let v: f64 = num_s
                    .parse()
                    .map_err(|_| LexError(format!("не число: {num_s}")))?;
                out.push(Token::Float(v));
            } else {
                let v: i64 = num_s
                    .parse()
                    .map_err(|_| LexError(format!("не число: {num_s}")))?;
                out.push(Token::Int(v));
            }
            continue;
        }
        if is_ident_char(c) {
            let start = i;
            while i < n && is_ident_char(s[i]) {
                i += 1;
            }
            let ident: String = s[start..i].iter().collect();
            let low = ident.to_lowercase();
            let val = if is_keyword(&low) { low } else { ident };
            out.push(Token::Ident(val));
            continue;
        }
        return Err(LexError(format!(
            "неожиданный символ {:?} на позиции {i}",
            c
        )));
    }

    out.push(Token::Eof);
    Ok(out)
}

pub struct Parser {
    pub t: Vec<Token>,
    pub i: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { t: tokens, i: 0 }
    }

    pub fn cur(&self) -> &Token {
        &self.t[self.i]
    }

    pub fn bump(&mut self) -> Token {
        let tok = self.t[self.i].clone();
        if self.i + 1 < self.t.len() {
            self.i += 1;
        }
        tok
    }

    fn match_kw(&mut self, word: &str) -> bool {
        if let Token::Ident(v) = self.cur() {
            if v == word {
                self.i += 1;
                return true;
            }
        }
        false
    }

    fn expect_kw(&mut self, word: &str) -> Result<(), ParseError> {
        if self.match_kw(word) {
            Ok(())
        } else {
            Err(ParseError(format!("ожидалось ключевое слово «{word}»")))
        }
    }
}

fn parse_expr_value(p: &mut Parser) -> Result<ExprValue, ParseError> {
    match p.cur().clone() {
        Token::StringLit(s) => {
            p.bump();
            Ok(ExprValue::Literal(Cell::Str(s)))
        }
        Token::Int(n) => {
            p.bump();
            Ok(ExprValue::Literal(Cell::Int(n)))
        }
        Token::Float(x) => {
            p.bump();
            Ok(ExprValue::Literal(Cell::Float(x)))
        }
        Token::Ident(name) => {
            let name = name.clone();
            p.bump();
            Ok(ExprValue::Ident(name))
        }
        _ => Err(ParseError(
            "ожидалось значение (строка, число или имя столбца)".into(),
        )),
    }
}

fn insert_value_cell(ev: ExprValue) -> Cell {
    match ev {
        ExprValue::Literal(c) => c,
        ExprValue::Ident(s) => Cell::Str(s),
    }
}

fn parse_comparison(p: &mut Parser) -> Result<CmpExpr, ParseError> {
    let left = match p.bump() {
        Token::Ident(s) => s,
        _ => {
            return Err(ParseError("в условии слева должно быть имя столбца".into()));
        }
    };
    let op_tok = p.cur().clone();
    let Some(op) = cmp_from_tok(&op_tok) else {
        return Err(ParseError("ожидался оператор сравнения".into()));
    };
    p.bump();
    let right = parse_expr_value(p)?;
    Ok(CmpExpr { left, op, right })
}

fn parse_where(p: &mut Parser) -> Result<Vec<CmpExpr>, ParseError> {
    if !p.match_kw("где") {
        return Ok(vec![]);
    }
    let mut parts = vec![parse_comparison(p)?];
    while p.match_kw("и") {
        parts.push(parse_comparison(p)?);
    }
    Ok(parts)
}

fn parse_select(p: &mut Parser) -> Result<SelectQuery, ParseError> {
    p.expect_kw("выбрать")?;
    let columns = if matches!(p.cur(), Token::Star) {
        p.bump();
        None
    } else {
        let mut cols = vec![match p.bump() {
            Token::Ident(c) => c,
            other => {
                return Err(ParseError(format!(
                    "ожидался идентификатор столбца, получено {other:?}"
                )));
            }
        }];
        while matches!(p.cur(), Token::Comma) {
            p.bump();
            cols.push(match p.bump() {
                Token::Ident(c) => c,
                other => {
                    return Err(ParseError(format!(
                        "ожидался идентификатор столбца, получено {other:?}"
                    )));
                }
            });
        }
        Some(cols)
    };
    p.expect_kw("из")?;
    let table = match p.bump() {
        Token::Ident(t) => t,
        other => {
            return Err(ParseError(format!(
                "ожидалось имя таблицы, получено {other:?}"
            )));
        }
    };
    let r#where = parse_where(p)?;
    if !matches!(p.cur(), Token::Eof) {
        return Err(ParseError("лишний текст после запроса".into()));
    }
    Ok(SelectQuery {
        columns,
        table,
        r#where,
    })
}

fn parse_create_table(p: &mut Parser) -> Result<CreateTable, ParseError> {
    p.expect_kw("создать")?;
    p.expect_kw("таблицу")?;
    let name = match p.bump() {
        Token::Ident(n) => n,
        other => {
            return Err(ParseError(format!(
                "ожидалось имя таблицы, получено {other:?}"
            )));
        }
    };
    match p.bump() {
        Token::LParen => {}
        other => {
            return Err(ParseError(format!("ожидалась «(», получено {other:?}")));
        }
    }
    let mut columns = vec![match p.bump() {
        Token::Ident(c) => c,
        other => {
            return Err(ParseError(format!("ожидался столбец, получено {other:?}")));
        }
    }];
    while matches!(p.cur(), Token::Comma) {
        p.bump();
        columns.push(match p.bump() {
            Token::Ident(c) => c,
            other => {
                return Err(ParseError(format!("ожидался столбец, получено {other:?}")));
            }
        });
    }
    match p.bump() {
        Token::RParen => {}
        other => {
            return Err(ParseError(format!("ожидалась «)», получено {other:?}")));
        }
    }
    if !matches!(p.cur(), Token::Eof) {
        return Err(ParseError("лишний текст после объявления таблицы".into()));
    }
    Ok(CreateTable { name, columns })
}

fn parse_insert(p: &mut Parser) -> Result<InsertRow, ParseError> {
    p.expect_kw("вставить")?;
    p.expect_kw("в")?;
    let table = match p.bump() {
        Token::Ident(t) => t,
        other => {
            return Err(ParseError(format!(
                "ожидалось имя таблицы, получено {other:?}"
            )));
        }
    };
    p.expect_kw("значения")?;
    match p.bump() {
        Token::LParen => {}
        other => {
            return Err(ParseError(format!("ожидалась «(», получено {other:?}")));
        }
    }
    let mut values = vec![insert_value_cell(parse_expr_value(p)?)];
    while matches!(p.cur(), Token::Comma) {
        p.bump();
        values.push(insert_value_cell(parse_expr_value(p)?));
    }
    match p.bump() {
        Token::RParen => {}
        other => {
            return Err(ParseError(format!("ожидалась «)», получено {other:?}")));
        }
    }
    if !matches!(p.cur(), Token::Eof) {
        return Err(ParseError("лишний текст после вставки".into()));
    }
    Ok(InsertRow { table, values })
}

fn parse_help(p: &mut Parser) -> Result<(), ParseError> {
    p.expect_kw("помощь")?;
    if !matches!(p.cur(), Token::Eof) {
        return Err(ParseError("лишний текст после помощи".into()));
    }
    Ok(())
}

pub fn parse_query(query: &str) -> Result<ParsedQuery, QueryError> {
    let tokens = lex(query)?;
    let head = Parser::new(tokens.clone());
    if matches!(head.cur(), Token::Eof) {
        return Err(QueryError::Parse("пустой запрос".into()));
    }
    if let Token::Ident(cmd) = head.cur() {
        match cmd.as_str() {
            "выбрать" => {
                return Ok(ParsedQuery::Select(parse_select(&mut Parser::new(tokens))?));
            }
            "создать" => {
                return Ok(ParsedQuery::Create(parse_create_table(&mut Parser::new(
                    tokens,
                ))?));
            }
            "вставить" => {
                return Ok(ParsedQuery::Insert(parse_insert(&mut Parser::new(tokens))?));
            }
            "помощь" => {
                parse_help(&mut Parser::new(tokens))?;
                return Ok(ParsedQuery::Help);
            }
            _ => {}
        }
    }
    Err(QueryError::Parse(
        "неизвестная команда; начните с ВЫБРАТЬ, СОЗДАТЬ ТАБЛИЦУ, ВСТАВИТЬ или ПОМОЩЬ".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_unclosed_string() {
        assert!(lex("'нет конца").is_err());
    }

    #[test]
    fn parse_help_variants() {
        assert!(matches!(parse_query("помощь").unwrap(), ParsedQuery::Help));
        assert!(matches!(
            parse_query("  ПОМОЩЬ  ").unwrap(),
            ParsedQuery::Help
        ));
    }

    #[test]
    fn parse_unknown_command() {
        assert!(matches!(
            parse_query("удалить всё"),
            Err(QueryError::Parse(_))
        ));
    }

    #[test]
    fn parse_create_insert_select() {
        let q = "создать таблицу тест (a, b)";
        let ParsedQuery::Create(ct) = parse_query(q).unwrap() else {
            panic!("create");
        };
        assert_eq!(ct.name, "тест");
        assert_eq!(ct.columns, vec!["a".to_string(), "b".to_string()]);

        let q = "вставить в тест значения (1, 'x')";
        let ParsedQuery::Insert(ir) = parse_query(q).unwrap() else {
            panic!("insert");
        };
        assert_eq!(ir.table, "тест");
        assert_eq!(ir.values, vec![Cell::Int(1), Cell::Str("x".into()),]);

        let q = "выбрать * из тест где a = 1";
        let ParsedQuery::Select(sq) = parse_query(q).unwrap() else {
            panic!("select");
        };
        assert!(sq.columns.is_none());
        assert_eq!(sq.table, "тест");
        assert_eq!(sq.r#where.len(), 1);
        assert_eq!(sq.r#where[0].left, "a");
        assert!(matches!(sq.r#where[0].op, CmpOp::Eq));
        assert!(matches!(
            &sq.r#where[0].right,
            ExprValue::Literal(Cell::Int(1))
        ));
    }
}
