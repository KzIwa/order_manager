use anyhow::Result;
use calamine::{open_workbook, DataType, Reader, Xlsx};
use glob::glob;
use std::path::{Path, PathBuf};
use std::{env, io};

pub fn readexcel(filename: &PathBuf) -> Result<Vec<Vec<String>>> {
    // let filename = ".\\AOTB01-01_加工品（テーピング).xlsx";
    // let mut excel = open_workbook_auto(filename).unwrap();
    //  一つのファイルのsheet1 を読み取り値を取得する
    let excel: Xlsx<_> = open_workbook(&filename)?;
    let sheetnames = excel.sheet_names();
    // println!("{:?}", filename);
    let mut content_data: Vec<Vec<String>> = Vec::new();
    for sheetname in sheetnames.iter() {
        let mut excel: Xlsx<_> = open_workbook(&filename)?;
        let range = excel.worksheet_range(sheetname.as_str());
        match range {
            Some(rng) => {
                for row in rng.unwrap().rows() {
                    let mut exlinedata: Vec<String> = Vec::new();
                    for col in row.iter() {
                        // println!("row={:?},row[0]={:?}",row,row[0]);
                        match *col {
                            DataType::Empty => (exlinedata.push("".to_string())),
                            DataType::String(ref s) => exlinedata.push(s.trim().to_string()),
                            DataType::Float(ref f) => exlinedata.push(f.to_string()),
                            DataType::Int(ref i) => exlinedata.push(i.to_string()),
                            _ => (),
                        }
                    }
                    // 出力例
                    // 90:購入:AOB202-90-102:セットカラー:PSCS20-10:ミスミ:1:手配済:ミスミ:760:760:
                    if exlinedata.len() > 1 && exlinedata[4] != "" {
                        match exlinedata[1].as_str() {
                            "加工" | "購入" => {
                                // println!("{:?}", exlinedata);
                                content_data.push(exlinedata);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {
                // println!("can not open{:?}", &filename);
            }
        }
    }
    if content_data.is_empty() {
        // println!("{:?}はシート構成が異なるため読み取れません", &filename);
    }
    Ok(content_data)
}
