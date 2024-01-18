use calamine::{open_workbook, DataType, Reader, Xlsx};
use chrono::{Duration, NaiveDate};
// use std::{path::PathBuf, str::FromStr};
use std::path::PathBuf;

pub fn readexcel(filename: &PathBuf) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    // let filename = ".\\AOTB01-01_加工品（テーピング).xlsx";
    // let mut excel = open_workbook_auto(filename).unwrap();
    //  一つのファイルのsheet1 を読み取り値を取得する
    let excel: Xlsx<_> = open_workbook(filename)?;
    let sheetname = &excel.sheet_names()[0];
    let mut content_data= Vec::new();
    let mut excel: Xlsx<_> = open_workbook(filename)?;
    let range = excel.worksheet_range(sheetname.as_str());

    match range {
        Ok(rng) => {
            rng.rows().for_each(|row| {
                let mut exlinedata: Vec<String> = Vec::new();
                row.iter().for_each(|col| {
                    match *col {
                        DataType::Empty => exlinedata.push("".to_string()),//空の場合
                        DataType::String(ref s) => exlinedata.push(s.trim().to_string()),//文字列の場合
                        DataType::Float(ref f) => exlinedata.push(f.to_string()),//浮動小数型の場合
                        DataType::Int(ref i) => exlinedata.push(i.to_string()),//整数型の場合
                        DataType::DateTime(ref d) => {
                            // f64のｄの値を日付データに変換
                            let datetime = from_days_since_1900(*d as i64);
                            exlinedata.push(datetime.to_string())
                        }
                        _ => (),
                    }
                });
                // 出力例
                // 90:購入:AOB202-90-102:セットカラー:PSCS20-10:ミスミ:1:手配済:ミスミ:760:760:
                // 読み取ったデータ配列長さが5よりも大きく,材質および型式の欄が空でない場合
                if exlinedata.len() > 5 && !exlinedata[4].is_empty() {
                    match exlinedata[1].as_str() {
                        "加工" | "購入" => {
                            content_data.push(exlinedata);
                        }
                        _ => {}
                    }
                }
            });
        }
        _ => content_data.push(vec!["".to_string()]),
    }
    Ok(content_data)
}

fn from_days_since_1900(days_since_1900: i64) -> NaiveDate {
    let d1900 = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
    d1900 + Duration::days(days_since_1900 - 2)
}

#[test]
fn readtest()->Result<(),Box<dyn std::error::Error>> {
    use std::str::FromStr; 
    let testpath = PathBuf::from_str("SOD301-20ロボット部購入品.xlsx").unwrap();
    match readexcel(&testpath){
        Ok(st)=>{
            println!("{:?}",st);
        },
        Err(e)=>return Err(e)
    };
    Ok(())
}