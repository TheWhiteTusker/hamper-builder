//! Files the user downloads or uploads: the Excel round trip (written with
//! rust_xlsxwriter, read with calamine) and CSV exports. Saving asks where,
//! through the system dialog.

use lattice_core::api::ApiError;
use lattice_core::js::f;
use lattice_core::sheets::{Cached, Cell, Format, SheetData};
use rust_xlsxwriter::{Color, Format as XFormat, Formula, Workbook};
use std::io::Cursor;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

pub(crate) fn err(e: impl std::fmt::Display) -> ApiError {
    ApiError::Server(format!("Couldn't write the file: {e}"))
}

/// Where to save, chosen in the system dialog; None if cancelled.
pub(crate) async fn ask_where(
    app: &AppHandle,
    file_name: &str,
    label: &str,
    ext: &'static str,
) -> Result<Option<PathBuf>, ApiError> {
    let dialog = app.dialog().file().set_file_name(file_name).add_filter(label, &[ext]);
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file()).await.map_err(err)?;
    picked.map(|p| p.into_path().map_err(err)).transpose()
}

/// Workbook bytes with the old app's look: blue input headings, grey
/// calculated ones, a frozen header row, money and percent columns.
pub fn workbook(sheets: &[SheetData]) -> Result<Vec<u8>, ApiError> {
    let mut wb = Workbook::new();
    let input = XFormat::new().set_bold().set_font_color(Color::White).set_background_color(Color::RGB(0x1F4E78));
    let calc = XFormat::new().set_bold().set_font_color(Color::White).set_background_color(Color::RGB(0x6B7280));
    let percent = XFormat::new().set_num_format_index(10);
    let money = XFormat::new().set_num_format_index(4);
    let plain = XFormat::new();
    for sheet in sheets {
        let ws = wb.add_worksheet();
        ws.set_name(&sheet.name).map_err(err)?;
        ws.set_freeze_panes(1, 0).map_err(err)?;
        let formats: Vec<&XFormat> = sheet
            .columns
            .iter()
            .map(|c| match c.format {
                Some(Format::Percent) => &percent,
                Some(Format::Money) => &money,
                None => &plain,
            })
            .collect();
        for (i, c) in sheet.columns.iter().enumerate() {
            let col = u16::try_from(i).map_err(err)?;
            ws.set_column_width(col, f64::from(c.width)).map_err(err)?;
            // Column formats make cells typed into a blank template pick them up too.
            if c.format.is_some()
                && let Some(fmt) = formats.get(i)
            {
                ws.set_column_format(col, fmt).map_err(err)?;
            }
            ws.write_string_with_format(0, col, &c.header, if c.calculated { &calc } else { &input }).map_err(err)?;
        }
        for (r, row) in sheet.rows.iter().enumerate() {
            let rr = u32::try_from(r + 1).map_err(err)?;
            for (i, cell) in row.iter().enumerate() {
                let col = u16::try_from(i).map_err(err)?;
                let fmt = formats.get(i).copied().unwrap_or(&plain);
                match cell {
                    Cell::Empty => {}
                    Cell::Text(s) => {
                        ws.write_string_with_format(rr, col, s, fmt).map_err(err)?;
                    }
                    Cell::Num(n) => {
                        ws.write_number_with_format(rr, col, f(*n), fmt).map_err(err)?;
                    }
                    Cell::Formula { f: formula, v } => {
                        let mut x = Formula::new(formula);
                        match v {
                            Some(Cached::Num(n)) => x = x.set_result(f(*n).to_string()),
                            Some(Cached::Text(s)) => x = x.set_result(s),
                            None => {}
                        }
                        ws.write_formula_with_format(rr, col, x, fmt).map_err(err)?;
                    }
                }
            }
        }
    }
    wb.save_to_buffer().map_err(err)
}

/// Asks where, then writes the workbook. Returns the path, or None if cancelled.
#[tauri::command(rename_all = "snake_case")]
pub async fn save_xlsx(app: AppHandle, file_name: String, sheets: Vec<SheetData>) -> Result<Option<String>, ApiError> {
    let bytes = workbook(&sheets)?;
    let Some(path) = ask_where(&app, &file_name, "Excel workbook", "xlsx").await? else { return Ok(None) };
    std::fs::write(&path, bytes).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

/// Asks where, then writes `content` (a CSV, say) as UTF-8 with a BOM, so
/// Excel reads the ₹ and accented names correctly.
#[tauri::command(rename_all = "snake_case")]
pub async fn save_text(app: AppHandle, file_name: String, content: String) -> Result<Option<String>, ApiError> {
    let ext = if file_name.ends_with(".csv") { "csv" } else { "txt" };
    let Some(path) = ask_where(&app, &file_name, "CSV file", ext).await? else { return Ok(None) };
    std::fs::write(&path, format!("\u{feff}{content}")).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

/// Asks where, then writes the raw body (a PNG the editor rendered, say).
#[tauri::command]
pub async fn save_bytes(app: AppHandle, request: tauri::ipc::Request<'_>) -> Result<Option<String>, ApiError> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(ApiError::Invalid("The file must be sent as raw bytes.".into()));
    };
    let name = request.headers().get("x-file-name").and_then(|v| v.to_str().ok()).unwrap_or("download").to_string();
    let ext: &'static str = match name.rsplit('.').next() {
        Some("png") => "png",
        Some("pptx") => "pptx",
        Some("pdf") => "pdf",
        _ => "bin",
    };
    let bytes = bytes.clone();
    let Some(path) = ask_where(&app, &name, ext, ext).await? else { return Ok(None) };
    std::fs::write(&path, bytes).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

/// A spreadsheet's cell as SheetJS showed it with `raw: false`, near enough
/// for the importers: numbers in shortest form, dates as YYYY-MM-DD.
fn text(cell: &calamine::Data) -> String {
    use calamine::Data;
    match cell {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(x) => x.to_string(),
        Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.into(),
        Data::DateTime(d) => excel_date(d.as_f64()),
    }
}

/// An Excel serial day as YYYY-MM-DD (right for every date after 1900-02-28,
/// past Excel's phantom 1900-02-29).
fn excel_date(serial: f64) -> String {
    let epoch = time::Date::from_calendar_date(1899, time::Month::December, 30).unwrap_or(time::Date::MIN);
    let day = epoch.checked_add(time::Duration::days(serial.floor() as i64)).unwrap_or(epoch);
    format!("{:04}-{:02}-{:02}", day.year(), u8::from(day.month()), day.day())
}

/// A tab's name and its rows of cell text.
pub type Tab = (String, Vec<Vec<String>>);

/// Every tab of an uploaded workbook (the raw body) as rows of cell text,
/// positioned from A1 so columns read by position line up.
#[tauri::command]
pub fn read_xlsx(request: tauri::ipc::Request<'_>) -> Result<Vec<Tab>, ApiError> {
    use calamine::Reader;
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(ApiError::Invalid("The file must be sent as raw bytes.".into()));
    };
    let unreadable = |_| ApiError::Invalid("Couldn't read that file. Use an .xlsx workbook.".into());
    let mut wb = calamine::open_workbook_auto_from_rs(Cursor::new(bytes.clone())).map_err(unreadable)?;
    let mut out = Vec::new();
    for name in wb.sheet_names() {
        let range = wb.worksheet_range(&name).map_err(unreadable)?;
        let (top, left) = range.start().map_or((0, 0), |(r, c)| (r as usize, c as usize));
        let mut grid: Vec<Vec<String>> = vec![Vec::new(); top];
        for row in range.rows() {
            let mut cells = vec![String::new(); left];
            cells.extend(row.iter().map(text));
            grid.push(cells);
        }
        out.push((name, grid));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::Decimal;
    use lattice_core::sheets::ColumnData;

    #[test]
    fn dates_from_serials() {
        assert_eq!(excel_date(46345.0), "2026-11-19");
        assert_eq!(excel_date(1.0), "1899-12-31");
    }

    #[test]
    fn a_workbook_round_trips_through_the_reader() -> Result<(), ApiError> {
        use calamine::Reader;
        let sheet = SheetData {
            name: "Product Master".into(),
            columns: vec![
                ColumnData { header: "Code".into(), width: 14, format: None, calculated: false },
                ColumnData { header: "Markup %".into(), width: 10, format: Some(Format::Percent), calculated: false },
                ColumnData { header: "SP".into(), width: 10, format: Some(Format::Money), calculated: true },
            ],
            rows: vec![vec![
                Cell::Text("LC/0001/WL".into()),
                Cell::Num(Decimal::new(35, 2)),
                Cell::Formula { f: "B2*100".into(), v: Some(Cached::Num(Decimal::from(35))) },
            ]],
        };
        let bytes = workbook(&[sheet])?;
        let mut wb = calamine::open_workbook_auto_from_rs(Cursor::new(bytes)).map_err(err)?;
        let range = wb.worksheet_range("Product Master").map_err(err)?;
        let rows: Vec<Vec<String>> = range.rows().map(|r| r.iter().map(text).collect()).collect();
        assert_eq!(rows, [vec!["Code", "Markup %", "SP"], vec!["LC/0001/WL", "0.35", "35"]]);
        Ok(())
    }
}
