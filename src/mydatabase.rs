use chrono::prelude::*;
use rusqlite::{params, Connection, Error};
use std::path::Path;
// 一時的にデータを保持して扱いやすくするための構造体
#[derive(Debug, Default)]

pub struct PartsItem {
    pub db_id: i32,
    pub order_no: String,
    pub unit_no: i32,
    pub parts_no: String,
    pub rev_mark: String,
    pub name: String,
    pub itemtype: String,
    pub model:String,
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

pub fn createsql(savepath: &Path) -> Result<(), Error> {
    let conn = Connection::open(&savepath)?;
    match conn.execute(
        "CREATE TABLE partstable(
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
    ){
        Ok(num)=>{println!("Database created {}",num)},
        Err(error)=>{println!("create error {}",error)}
    }
    Ok(())
}

fn check_dupe(savepath: &Path, item: &PartsItem) -> Result<Vec<PartsItem>, Error> {
    // 重複確認
    let conn = Connection::open(savepath)?;
    let mut result: Vec<PartsItem> = Vec::new();
    let mut state = conn.prepare(
        "SELECT * From partstable 
    WHERE order_no == ? and unit_no == ? and parts_no == ? ",
    )?;
    let partsitem_iter =
        state.query_map(params![item.order_no, item.unit_no, item.parts_no], |row| {
            Ok(PartsItem {
                db_id: row.get(0)?,
                order_no: row.get(1)?,
                unit_no: row.get(2)?,
                parts_no: row.get(3)?,
                rev_mark: row.get(4)?,
                name: row.get(5)?,
                itemtype: row.get(6)?,
                model:row.get(7)?,
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
    for item in partsitem_iter {
        // println!("{:?}", item?);
        result.push(item?);
    }
    Ok(result)
}
pub fn insertsql(savepath: &Path, partsitem: Vec<PartsItem>) -> Result<(), Error> {
    let conn = Connection::open(savepath)?;
    for item in partsitem.iter() {
        let dupeitem = check_dupe(savepath, item)?;
        if dupeitem.len() > 0 {
            if string_to_time(dupeitem[0].order_date.as_str())
                < string_to_time(item.order_date.as_str())
            {
                updatesql(savepath, item)
            }
            continue;
        } else {
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
            conn.execute(
                statement,
                params![
                    item.order_no,
                    item.unit_no,
                    item.parts_no,
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
    }
    Ok(())
}

pub fn updatesql(savepath: &Path, item: &PartsItem) {
    todo!()
}
pub fn order_readsql(savepath: &Path, orderno: &str,itemtype:&str) -> Result<Vec<PartsItem>, Error> {
    let conn = Connection::open(savepath)?;
    let mut result: Vec<PartsItem> = Vec::new();
    if matches!(orderno, "" | "*") {
        let mut state = conn.prepare("SELECT * From partstable WHERE itemtype == ?")?;
        let partsitem_iter = state.query_map(params![itemtype], |row| {
            Ok(PartsItem {
                db_id: row.get(0)?,
                order_no: row.get(1)?,
                unit_no: row.get(2)?,
                parts_no: row.get(3)?,
                rev_mark: row.get(4)?,
                name: row.get(5)?,
                itemtype: row.get(6)?,
                model:row.get(7)?,
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

        for item in partsitem_iter {
            // println!("{:?}", item?);
            match item {
                Ok(it)=>    result.push(it),
                _=>()
            }
        }
        Ok(result)
    } else {
        let mut state = conn.prepare("SELECT * From partstable WHERE order_no == ?")?;
        let partsitem_iter = state.query_map(params![orderno], |row| {
            Ok(PartsItem {
                db_id: row.get(0)?,
                order_no: row.get(1)?,
                unit_no: row.get(2)?,
                parts_no: row.get(3)?,
                rev_mark: row.get(4)?,
                name: row.get(5)?,
                itemtype: row.get(6)?,
                model:row.get(7)?,
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

        for item in partsitem_iter {
            match item {
                Ok(it)=>    result.push(it),
                _=>()
            }
        }
        Ok(result)
    }
}

fn string_to_time(st: &str) -> Option<DateTime<FixedOffset>> {
    let result = DateTime::parse_from_str(st, "%Y/%m/%d %H:%M:%S");
    match result {
        Ok(time) => Some(time),
        _ => None,
    }
}
