use calamine::Reader;
use serde::Serialize;
use std::fs::File;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub enum Docs {
    Excel(Vec<DSheet>),
    Docx(String),
}

#[allow(unused)]
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
        // let target = docx(file_path)?;
        // todo 解析工作目前由 web 端解析
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
    let mut target: Vec<DSheet> = Vec::new();

    for sheet in sheets {
        let range = workbook.worksheet_range(&sheet)?;
        let mut rows = Vec::new();
        for row in range.rows() {
            let mut cell_list: Vec<String> = Vec::new();
            for cell in row.iter() {
                cell_list.push(cell.to_string());
            }
            rows.push(cell_list);
        }
        let map = DSheet { name: sheet, rows };

        target.push(map);
    }
    Ok(target)
}

fn csv(file_path: &str) -> Result<Vec<DSheet>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);

    let mut rows: Vec<Vec<String>> = vec![];
    for result in rdr.records() {
        let record = result?;
        let mut cols: Vec<String> = Vec::new();
        for i in 0..record.len() {
            cols.push(record[i].to_string());
        }
        rows.push(cols);
    }
    let target: Vec<DSheet> = vec![DSheet {
        name: "sheet1".to_string(),
        rows,
    }];
    Ok(target)
}
