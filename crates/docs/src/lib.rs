pub mod pdf;

use calamine::Reader;
use serde::Serialize;
use std::fs::File;

#[derive(Debug, Clone, Serialize)]
pub enum Docs {
    Excel(Vec<DSheet>),
    Docx(String),
}

impl Docs {
    pub fn excel(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let target = excel(file_path)?;
        Ok(Docs::Excel(target))
    }

    pub fn csv(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let target = csv(file_path)?;
        Ok(Docs::Excel(target))
    }

    pub fn docx(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: 解析工作目前由 web 端解析
        Ok(Docs::Docx(file_path.to_string()))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DSheet {
    name: String,
    rows: Vec<Vec<String>>,
}

fn excel(file_path: &str) -> Result<Vec<DSheet>, Box<dyn std::error::Error>> {
    let mut workbook = calamine::open_workbook_auto(file_path)?;
    let sheets = workbook.sheet_names().to_owned();

    sheets
        .into_iter()
        .map(|sheet| {
            let range = workbook.worksheet_range(&sheet)?;
            let rows = range
                .rows()
                .map(|row| row.iter().map(|cell| cell.to_string()).collect())
                .collect();
            Ok(DSheet { name: sheet, rows })
        })
        .collect()
}

fn csv(file_path: &str) -> Result<Vec<DSheet>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);

    let rows: Result<Vec<Vec<String>>, _> = rdr
        .records()
        .map(|result| {
            result
                .map(|record| record.iter().map(|s| s.to_string()).collect())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        })
        .collect();

    Ok(vec![DSheet { name: "sheet1".to_string(), rows: rows? }])
}
