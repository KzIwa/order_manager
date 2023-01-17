// use chrono::prelude::*;
// use chrono::{DateTime, Local, TimeZone};
use rusqlite::{params, Connection, Error};
use std::path::Path;

// 一時的にデータを保持して扱いやすくするための構造体
#[derive(Debug, Default, Clone)]

pub struct PartsItem {
    // pub db_id: i32,
    pub order_no: String,
    pub unit_no: i32,
    pub parts_no: String,
    pub rev_mark: String,
    pub name: String,
    pub itemtype: String,
    pub model: String,
    pub maker: String,
    pub itemqty: i32,
    pub remarks: String,
    pub condition: String,
    pub vender: String,
    pub order_date: String,
    pub delivery_date: String,
    pub delicondition: String,
    pub price: i32,
}

pub fn createtable(savepath: &Path) -> Result<(), Error> {
    let conn = Connection::open(savepath)?;
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS partstable(
        id INTEGER PRIMARY KEY,
        order_no TEXT,
        unit_no INTEGER,
        parts_no TEXT,
        rev_mark TEXT,
        name TEXT,
        itemtype TEXT,
        model TEXT,
        maker TEXT,
        itemqty INTEGER,
        remarks TEXT,
        condition TEXT,
        vender TEXT,
        order_date TEXT,
        delivery_date TEXT,
        delivery_condition TEXT,
        price INTERGER
    )",
        [],
    ) {
        Ok(num) => {
            println!("Database created {}", num)
        }
        Err(error) => {
            println!("create error {}", error)
        }
    }
    Ok(())
}

pub fn insertsql(savepath: &Path, partsitem: &[PartsItem]) -> Result<usize, Error> {
    // Vecで受け取ったアイテムを指定されたPathのデータベースへ登録する
    let conn = Connection::open(savepath)?;
    let mut counter = 0;

    let statement = "INSERT INTO partstable(
        order_no,
        unit_no,
        parts_no,
        rev_mark,
        name,
        itemtype,
        model,
        maker,
        itemqty,
        remarks,
        condition,
        vender,
        order_date,
        delivery_date,
        delivery_condition,
        price
    ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)";

    conn.execute_batch("BEGIN;")?;

    for item in partsitem.iter() {
        counter += 1;
        let partnum: Vec<&str> = item.parts_no.split('-').collect();
        let partno = if partnum.len() > 1 {
            partnum[1]
        } else {
            &item.parts_no
        };
        conn.execute(
            statement,
            params![
                item.order_no,
                item.unit_no,
                partno,
                item.rev_mark,
                item.name,
                item.itemtype,
                item.model,
                item.maker,
                item.itemqty,
                item.remarks,
                item.condition,
                item.vender,
                item.order_date,
                item.delivery_date,
                item.delicondition,
                item.price
            ],
        )?;
    }
    conn.execute_batch("COMMIT;")?;

    conn.cache_flush()?;
    // }
    Ok(counter)
}

pub fn order_readsql(
    savepath: &Path,
    orderno: &str,
    itemtype: &str,
    unitno: &str,
    searchword: &str,
    ordercheck: &bool, // 納期超過チェックフラグ
) -> Result<Vec<PartsItem>, Error> {
    let conn = Connection::open(savepath)?;
    let mut result: Vec<PartsItem> = Vec::new();


    conn.execute_batch("BEGIN;")?;

    let mut state = conn.prepare("SELECT * From partstable WHERE itemtype == ?")?;
    let partsitem_iter = state.query_map(params![itemtype], |row| {
        Ok(PartsItem {
            // db_id: row.get(0)?,
            order_no: row.get(1)?,
            unit_no: row.get(2)?,
            parts_no: row.get(3)?,
            rev_mark: row.get(4)?,
            name: row.get(5)?,
            itemtype: row.get(6)?,
            model: row.get(7)?,
            maker: row.get(8)?,
            itemqty: row.get(9)?,
            remarks: row.get(10)?,
            condition: row.get(11)?,
            vender: row.get(12)?,
            order_date: row.get(13)?,
            delivery_date: row.get(14)?,
            delicondition: row.get(15)?,
            price: row.get(16)?,
        })
    })?;

    for item in partsitem_iter.flatten() {
        // if let Ok(it) = item {
        //     let item = it;
        result.push(item)
        // }
    }
    
    conn.execute_batch("COMMIT;")?;

    let result = select_order(orderno.trim(), &result, ordercheck);
    let result = select_unit(unitno.trim_end(), &result);
    let result = search_word(searchword.trim(), &result);
    Ok(result)
}

fn select_order(pat: &str, parts: &[PartsItem], ordercheck: &bool) -> Vec<PartsItem> {
    let mut result = Vec::new();
    for it in parts.iter() {
        // 納期超過チェック
        if *ordercheck && it.condition.contains("済") {
            continue;
        }
        if pat.is_empty() {
            result.push(it.to_owned());
        } else {
            let searchwords = pat.split_whitespace();
            let mut iscontain = true;
            for searchword in searchwords {
                if !it
                    .order_no
                    .to_lowercase()
                    .contains(&searchword.to_lowercase())
                {
                    iscontain = false;
                    break;
                }
            }
            if iscontain {
                result.push(it.to_owned())
            }
        }
    }
    result
}

fn select_unit(pat: &str, parts: &[PartsItem]) -> Vec<PartsItem> {
    if pat.is_empty() {
        return parts.to_vec();
    }

    match pat.parse::<i32>() {
        Ok(n) => {
            let mut result = Vec::new();
            for it in parts.iter() {
                // if it.unit_no.to_string().contains(n.to_string().as_str()) {
                if &it.unit_no == &n {
                    result.push(it.to_owned());
                }
            }
            result
        }
        Err(_) => parts.to_vec(),
    }
    // output
}

fn search_word(searchword: &str, parts: &[PartsItem]) -> Vec<PartsItem> {
    let pat = searchword.trim();
    if pat.is_empty() {
        return parts.to_vec();
    }

    // 小文字変換してオーダー番号、名前、型式、メーカ、備考、商社の中でヒットする項目を探す
    let is_pattern = |it: &PartsItem, pattern: &str| {
        it.name.to_lowercase().contains(&pattern.to_lowercase())
            || it.model.to_lowercase().contains(&pattern.to_lowercase())
            || it.maker.to_lowercase().contains(&pattern.to_lowercase())
            || it.remarks.contains(pattern)
            || it.vender.contains(pattern)
    };

    let mut result = Vec::new();

    for it in parts.iter() {
        let mut is_ok = true;
        let patterns = pat.split_whitespace();
        for pattern in patterns {
            if is_pattern(it, pattern) {
            } else {
                is_ok = false;
            }
        }
        if is_ok {
            result.push(it.to_owned());
        }
    }
    result
}

// fn string_to_time(st: &str) -> Option<DateTime<Local>> {
//     let st = st.to_string() + " 00:00:00";

//     let result = Local.datetime_from_str(&st, "%Y-%m-%d %H:%M:%S");
//     match result {
//         Ok(time) => Some(time),
//         Err(_) => None,
//     }
// }

// fn isnot_over(item: &PartsItem) -> bool {
//     let today = Local::now();
//     if item.delivery_date.trim() == "" {
//         return true;
//     }
//     let delidate = string_to_time(item.delivery_date.trim());
//     match delidate {
//         Some(date) => {
//             !(today.naive_local().date() >= date.date_naive() && item.delicondition.trim() == "")
//         }
//         None => true,
//     }
// }
