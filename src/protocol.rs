//! JSON-ответы, демо-данные и обработка строки запроса.
//!
//! После TCP-подключения сервер шлёт [`BANNER`]: несколько строк UTF-8, затем **пустая строка** (`\n`),
//! затем на каждый запрос клиента — одна строка JSON.

use serde_json::{json, Value};

use crate::engine::{Cell, Engine};
use crate::parser::{parse_query, CreateTable, InsertRow, ParsedQuery, QueryError};

pub const BANNER: &str = concat!(
    "ИволгаQL мини-сервер — шлите один запрос в строке (UTF-8).\n",
    "Команда «помощь» для синтаксиса.\n",
    "Ответ — одна строка JSON (UTF-8); краткое сообщение — в поле bird (🐦).\n",
    "\n",
);

pub fn format_help_ru() -> String {
    r#"ИволгаQL — подмножество команд (регистр ключевых слов не важен):

  СОЗДАТЬ ТАБЛИЦУ имя ( кол1, кол2, … )
  ВСТАВИТЬ В имя ЗНАЧЕНИЯ ( литерал1, литерал2, … )
  ВЫБРАТЬ * ИЗ имя [ ГДЕ столбец оператор значение [ И … ] ]
  ВЫБРАТЬ кол1, кол2 ИЗ имя [ ГДЕ … ]

Операторы: = != <> < > <= >=
Строки в одинарных кавычках, кавычка внутри — удвоить ('').
"#
    .to_string()
}

pub fn seed_demo(engine: &mut Engine) {
    engine
        .create_table(&CreateTable {
            name: "склад".into(),
            columns: vec!["товар".into(), "цена".into(), "остаток".into()],
        })
        .expect("seed create_table");
    engine
        .insert_row(&InsertRow {
            table: "склад".into(),
            values: vec![Cell::Str("Хлеб".into()), Cell::Int(42), Cell::Int(120)],
        })
        .expect("seed insert");
    engine
        .insert_row(&InsertRow {
            table: "склад".into(),
            values: vec![Cell::Str("Молоко".into()), Cell::Int(89), Cell::Int(15)],
        })
        .expect("seed insert");
    engine
        .insert_row(&InsertRow {
            table: "склад".into(),
            values: vec![
                Cell::Str("ИволгаQL-мерч".into()),
                Cell::Int(1337),
                Cell::Int(3),
            ],
        })
        .expect("seed insert");
}

fn map_query_error(e: QueryError) -> Value {
    match e {
        QueryError::Parse(m) => json!({ "ok": false, "error": format!("разбор: {m}") }),
        QueryError::Lex(m) => json!({ "ok": false, "error": format!("лексика: {m}") }),
    }
}

pub fn run_query(engine: &mut Engine, line: &str) -> Value {
    let q = line.trim();
    if q.is_empty() {
        return json!({ "ok": false, "error": "пустая строка" });
    }

    let parsed = match parse_query(q) {
        Ok(p) => p,
        Err(e) => return map_query_error(e),
    };

    match parsed {
        ParsedQuery::Help => json!({
            "ok": true,
            "bird": "Справка в поле help_text.",
            "help_text": format_help_ru(),
        }),
        ParsedQuery::Create(stmt) => match engine.create_table(&stmt) {
            Ok(()) => json!({
                "ok": true,
                "bird": "Таблица создана, можно вставлять строки.",
                "command": "create_table",
                "table": stmt.name,
                "columns": stmt.columns,
            }),
            Err(e) => json!({ "ok": false, "error": e }),
        },
        ParsedQuery::Insert(stmt) => match engine.insert_row(&stmt) {
            Ok(()) => json!({
                "ok": true,
                "bird": "Строка вставлена.",
                "command": "insert",
                "table": stmt.table,
            }),
            Err(e) => json!({ "ok": false, "error": e }),
        },
        ParsedQuery::Select(stmt) => match engine.select(&stmt) {
            Ok((cols, rows)) => {
                let rowcount = rows.len();
                json!({
                    "ok": true,
                    "bird": "Результат в полях columns и rows.",
                    "command": "select",
                    "columns": cols,
                    "rows": rows,
                    "rowcount": rowcount,
                })
            }
            Err(e) => json!({ "ok": false, "error": e }),
        },
    }
}

pub fn reply_line(value: &Value) -> Result<String, serde_json::Error> {
    let mut s = serde_json::to_string(value)?;
    s.push('\n');
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_line_errors() {
        let mut e = Engine::new();
        let v = run_query(&mut e, "   ");
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "пустая строка");
    }

    #[test]
    fn help_json_ok() {
        let mut e = Engine::new();
        let v = run_query(&mut e, "помощь");
        assert_eq!(v["ok"], true);
        assert!(v["help_text"].as_str().unwrap().contains("ВЫБРАТЬ"));
    }

    #[test]
    fn demo_select_rowcount() {
        let mut e = Engine::new();
        seed_demo(&mut e);
        let v = run_query(&mut e, "выбрать * из склад где остаток > 10");
        assert_eq!(v["ok"], true);
        assert_eq!(v["rowcount"], 2);
    }

    #[test]
    fn reply_line_has_newline() {
        let s = reply_line(&json!({"ok": true})).unwrap();
        assert!(s.ends_with('\n'));
        assert_eq!(s.lines().count(), 1);
    }
}
