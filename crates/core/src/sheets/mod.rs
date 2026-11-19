//! Editable spreadsheets as data: tabs, columns, cells and formulas.
//!
//! Pure, so what a download holds and how an upload is read can be tested
//! without a file. The native app turns a [`Sheet`] into `.xlsx` bytes
//! (coloured frozen header, column formats, formulas filled down blank rows).

pub mod product;
pub mod rates;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

/// A formula's value as last worked out, cached in the file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Cached {
    Text(String),
    Num(Decimal),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Cell {
    Empty,
    Text(String),
    Num(Decimal),
    /// A formula (no leading "=") and the value it currently works out to, if known.
    Formula {
        f: String,
        v: Option<Cached>,
    },
}

impl Cell {
    /// Text, or [`Cell::Empty`] for "" (an empty string writes no cell).
    pub fn text(s: impl Into<String>) -> Self {
        let s = s.into();
        if s.is_empty() { Self::Empty } else { Self::Text(s) }
    }

    pub fn formula(f: String, v: Cached) -> Self {
        Self::Formula { f, v: Some(v) }
    }
}

impl From<Decimal> for Cell {
    fn from(n: Decimal) -> Self {
        Self::Num(n)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    Percent,
    Money,
}

/// A column's formula for sheet row `r` (1-based, the header is row 1).
pub type FormulaFn = Arc<dyn Fn(usize) -> String + Send + Sync>;

#[derive(Clone)]
pub struct Column {
    pub header: String,
    pub width: u16,
    pub format: Option<Format>,
    /// Marks a calculated column: grey heading, and filled down the blank rows.
    pub formula: Option<FormulaFn>,
}

impl fmt::Debug for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Column").field("header", &self.header).field("formula", &self.formula.is_some()).finish()
    }
}

impl Column {
    pub fn new(header: impl Into<String>, width: u16) -> Self {
        Self { header: header.into(), width, format: None, formula: None }
    }

    pub fn money(mut self) -> Self {
        self.format = Some(Format::Money);
        self
    }

    pub fn percent(mut self) -> Self {
        self.format = Some(Format::Percent);
        self
    }

    pub fn calc(mut self, f: impl Fn(usize) -> String + Send + Sync + 'static) -> Self {
        self.formula = Some(Arc::new(f));
        self
    }

    /// This column's formula at row `r`, if it is calculated.
    pub fn formula_at(&self, r: usize) -> Option<String> {
        self.formula.as_ref().map(|f| f(r))
    }
}

/// One tab: `rows` from row 2, then `blank_rows` empty rows holding the formulas.
#[derive(Clone, Debug)]
pub struct Sheet {
    pub name: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Cell>>,
    pub blank_rows: usize,
}

/// A tab with its formulas worked out row by row: what the native side
/// writes to `.xlsx` (a [`Sheet`]'s formula closures can't cross IPC).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetData {
    pub name: String,
    pub columns: Vec<ColumnData>,
    /// Row 2 onwards, the blank formula rows included.
    pub rows: Vec<Vec<Cell>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColumnData {
    pub header: String,
    pub width: u16,
    pub format: Option<Format>,
    /// Calculated: a grey heading instead of blue.
    pub calculated: bool,
}

impl Sheet {
    pub fn data(&self) -> SheetData {
        let first_blank = self.rows.len() + 2;
        let mut rows = self.rows.clone();
        rows.extend((0..self.blank_rows).map(|b| formula_row(&self.columns, first_blank + b)));
        SheetData {
            name: self.name.clone(),
            columns: self
                .columns
                .iter()
                .map(|c| ColumnData { header: c.header.clone(), width: c.width, format: c.format, calculated: c.formula.is_some() })
                .collect(),
            rows,
        }
    }
}

/// An empty row `r` that still carries the column formulas, so a row typed
/// into works out like the rest.
pub fn formula_row(columns: &[Column], r: usize) -> Vec<Cell> {
    columns.iter().map(|c| c.formula_at(r).map_or(Cell::Empty, |f| Cell::Formula { f, v: None })).collect()
}

/// Column letter for a 0-based index: 0 → A, 25 → Z, 26 → AA.
pub fn letter(mut i: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(char::from(b'A' + u8::try_from(i % 26).unwrap_or(0)));
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    out.iter().rev().collect()
}

/// A number as JS `String(n)` prints it in messages: 120, 12.5, 217.8.
pub(crate) fn js_num(n: Decimal) -> String {
    n.normalize().to_string()
}

/// First eight, comma separated, with "…" when there are more.
pub(crate) fn list<S: AsRef<str>>(xs: &[S]) -> String {
    let shown: Vec<&str> = xs.iter().take(8).map(AsRef::as_ref).collect();
    format!("{}{}", shown.join(", "), if xs.len() > 8 { "…" } else { "" })
}

#[cfg(test)]
pub(crate) mod testing {
    use super::{Cached, Cell, Format, Sheet};

    /// A tab as the browser read it back (`sheet_to_json`, `raw: false`): money
    /// to 2 decimals, formulas as their cached value, blank rows kept.
    pub fn grid(sheet: &Sheet) -> Vec<Vec<String>> {
        let mut out = vec![sheet.columns.iter().map(|c| c.header.clone()).collect::<Vec<_>>()];
        for row in &sheet.rows {
            out.push(
                row.iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let money = sheet.columns.get(i).is_some_and(|c| c.format == Some(Format::Money));
                        let num = |n: &rust_decimal::Decimal| {
                            if money { format!("{:.2}", n.round_dp(2)) } else { n.normalize().to_string() }
                        };
                        match c {
                            Cell::Empty | Cell::Formula { v: None, .. } => String::new(),
                            Cell::Text(s) | Cell::Formula { v: Some(Cached::Text(s)), .. } => s.clone(),
                            Cell::Num(n) | Cell::Formula { v: Some(Cached::Num(n)), .. } => num(n),
                        }
                    })
                    .collect(),
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_fills_formulas_down_the_blank_rows() {
        let sheet = Sheet {
            name: "T".into(),
            columns: vec![Column::new("A", 10), Column::new("B", 10).calc(|r| format!("A{r}*2"))],
            rows: vec![vec![Cell::text("x"), Cell::Empty]],
            blank_rows: 2,
        };
        let d = sheet.data();
        assert_eq!(d.rows.len(), 3);
        assert_eq!(d.rows[2][1], Cell::Formula { f: "A4*2".into(), v: None });
        assert!(d.columns[1].calculated && !d.columns[0].calculated);
    }

    #[test]
    fn letters() {
        assert_eq!(letter(0), "A");
        assert_eq!(letter(15), "P");
        assert_eq!(letter(25), "Z");
        assert_eq!(letter(26), "AA");
        assert_eq!(letter(27), "AB");
    }
}
