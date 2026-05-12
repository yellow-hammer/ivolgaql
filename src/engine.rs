//! Таблицы в памяти и выполнение запросов.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::Serialize;

use crate::parser::{CmpOp, CreateTable, ExprValue, InsertRow, SelectQuery};

/// Значение ячейки (числа, строка).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Cell {
    Str(String),
    Int(i64),
    Float(f64),
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::Str(s) => write!(f, "{s:?}"),
            Cell::Int(n) => write!(f, "{n}"),
            Cell::Float(x) => write!(f, "{x}"),
        }
    }
}

fn cmp_cell(left: &Cell, op: CmpOp, right: &Cell) -> Result<bool, String> {
    use std::cmp::Ordering;
    use CmpOp::*;

    let ord = match (left, right) {
        (Cell::Int(a), Cell::Int(b)) => a.cmp(b),
        (Cell::Float(a), Cell::Float(b)) => a.total_cmp(b),
        (Cell::Int(a), Cell::Float(b)) => {
            let af = *a as f64;
            af.total_cmp(b)
        }
        (Cell::Float(a), Cell::Int(b)) => {
            let bf = *b as f64;
            a.total_cmp(&bf)
        }
        (Cell::Str(a), Cell::Str(b)) => a.cmp(b),
        _ => {
            return Err(format!(
                "несовместимые типы для сравнения: {} и {}",
                left, right
            ));
        }
    };

    Ok(match op {
        Eq => ord == Ordering::Equal,
        Ne => ord != Ordering::Equal,
        Lt => ord == Ordering::Less,
        Gt => ord == Ordering::Greater,
        Le => matches!(ord, Ordering::Less | Ordering::Equal),
        Ge => matches!(ord, Ordering::Greater | Ordering::Equal),
    })
}

#[derive(Debug, Default)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, Cell>>,
}

#[derive(Debug, Default)]
pub struct Engine {
    pub tables: HashMap<String, Table>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_table(&mut self, stmt: &CreateTable) -> Result<(), String> {
        if self.tables.contains_key(&stmt.name) {
            return Err(format!("таблица «{}» уже существует", stmt.name));
        }
        let mut seen = HashSet::new();
        for c in &stmt.columns {
            if !seen.insert(c.as_str()) {
                return Err(format!("повтор столбца «{c}»"));
            }
        }
        self.tables.insert(
            stmt.name.clone(),
            Table {
                columns: stmt.columns.clone(),
                rows: Vec::new(),
            },
        );
        Ok(())
    }

    pub fn insert_row(&mut self, stmt: &InsertRow) -> Result<(), String> {
        let t = self
            .tables
            .get_mut(&stmt.table)
            .ok_or_else(|| format!("нет таблицы «{}»", stmt.table))?;
        if stmt.values.len() != t.columns.len() {
            return Err(format!(
                "ожидалось {} значений, передано {}",
                t.columns.len(),
                stmt.values.len()
            ));
        }
        let mut row = HashMap::new();
        for (col, val) in t.columns.iter().zip(&stmt.values) {
            row.insert(col.clone(), val.clone());
        }
        t.rows.push(row);
        Ok(())
    }

    fn resolve_right(row: &HashMap<String, Cell>, ev: &ExprValue) -> Cell {
        match ev {
            ExprValue::Literal(c) => c.clone(),
            ExprValue::Ident(name) => row
                .get(name)
                .cloned()
                .unwrap_or_else(|| Cell::Str(name.clone())),
        }
    }

    pub fn select(&self, stmt: &SelectQuery) -> Result<(Vec<String>, Vec<Vec<Cell>>), String> {
        let t = self
            .tables
            .get(&stmt.table)
            .ok_or_else(|| format!("нет таблицы «{}»", stmt.table))?;

        let colnames: Vec<String> = if let Some(cols) = &stmt.columns {
            for c in cols {
                if !t.columns.contains(c) {
                    return Err(format!("нет столбца «{c}» в таблице «{}»", stmt.table));
                }
            }
            cols.clone()
        } else {
            t.columns.clone()
        };

        let mut filtered: Vec<&HashMap<String, Cell>> = Vec::new();
        for r in &t.rows {
            let mut ok = true;
            for cond in &stmt.r#where {
                let left_val = r
                    .get(&cond.left)
                    .ok_or_else(|| format!("нет столбца «{}» в строке", cond.left))?;
                let right_val = Self::resolve_right(r, &cond.right);
                match cmp_cell(left_val, cond.op, &right_val) {
                    Ok(b) if b => {}
                    Ok(_) => {
                        ok = false;
                        break;
                    }
                    Err(e) => return Err(e),
                }
            }
            if ok {
                filtered.push(r);
            }
        }

        let out_rows: Vec<Vec<Cell>> = filtered
            .into_iter()
            .map(|r| {
                colnames
                    .iter()
                    .map(|c| r.get(c.as_str()).unwrap().clone())
                    .collect()
            })
            .collect();

        Ok((colnames, out_rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{CmpExpr, CmpOp, CreateTable, ExprValue, InsertRow, SelectQuery};

    #[test]
    fn create_insert_select_roundtrip() {
        let mut e = Engine::new();
        e.create_table(&CreateTable {
            name: "t".into(),
            columns: vec!["n".into(), "s".into()],
        })
        .unwrap();
        e.insert_row(&InsertRow {
            table: "t".into(),
            values: vec![Cell::Int(10), Cell::Str("z".into())],
        })
        .unwrap();

        let (cols, rows) = e
            .select(&SelectQuery {
                columns: Some(vec!["s".into()]),
                table: "t".into(),
                r#where: vec![],
            })
            .unwrap();
        assert_eq!(cols, vec!["s".to_string()]);
        assert_eq!(rows, vec![vec![Cell::Str("z".into())]]);

        let (cols, rows) = e
            .select(&SelectQuery {
                columns: None,
                table: "t".into(),
                r#where: vec![CmpExpr {
                    left: "n".into(),
                    op: CmpOp::Ge,
                    right: ExprValue::Literal(Cell::Int(5)),
                }],
            })
            .unwrap();
        assert_eq!(cols, vec!["n".to_string(), "s".to_string()]);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn duplicate_table_rejected() {
        let mut e = Engine::new();
        let ct = CreateTable {
            name: "t".into(),
            columns: vec!["a".into()],
        };
        e.create_table(&ct).unwrap();
        assert!(e.create_table(&ct).unwrap_err().contains("уже существует"));
    }
}
