use anyhow::{Error, Result};
use rusqlite::{params,Connection};
use std::{path::Path, str::FromStr};

#[derive(Debug)]
pub struct PartsItem {
    unit_no: i32,
    parts_no: i32,
    name: String,
    itemtype: String,
    maker: String,
    itemqty: i32,
    remarks: String,
    condition: String,
    vender: String,
    order_date: String,
    delivery_date: String,
    delicondition: String,
    price: i32,
}

impl PartsItem {
    pub fn new(
        uni: i32,
        par: i32,
        name: &str,
        itemtype: &str,
        maker: &str,
        qty: i32,
        remarks: &str,
        cond: &str,
        vender: &str,
        orderdate: &str,
        delivery: &str,
        delicondition: &str,
        price: i32,
    ) -> Self {
        Self {
            unit_no: uni,
            parts_no: par,
            name: name.to_string(),
            itemtype: itemtype.to_string(),
            maker: maker.to_string(),
            itemqty: qty,
            remarks: remarks.to_string(),
            condition: cond.to_string(),
            vender: vender.to_string(),
            order_date: orderdate.to_string(),
            delivery_date: delivery.to_string(),
            delicondition: delicondition.to_string(),
            price: price,
        }
    }
}

pub fn createsql(savepath: &Path) -> Result<usize> {
    let conn = Connection::open(savepath)?;
    let statement = "
    CREATE TABLE partstable (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        unit_no INTEGER,
        parts_no INTEGER,
        name TEXT,
        itemtype TEXT,
        maker TEXT,
        itemqty INTEGER,
        remarks TEXT,
        condition TEXT,
        vender TEXT,
        order_date TEXT,
        delivery_date TEXT,
        delivery_condition TEXT,
        price INTERGER)
    ";
    Ok(conn.execute(statement, params![])?)
}

pub fn insertsql(savepath: &Path, partsitem: PartsItem) -> Result<()> {
    let conn = Connection::open(savepath)?;
    let statement = "INSERT INTO partstable VALUES()";
        // 1, // :unit_no, // :parts_no, // :name, // :itemtype, // :maker, // :itemqty, // :remarks, 
        // :condition, // :vender, // :order_date, // :delivery_date, // :delivery_condition, // :price
    let mut sql = conn.prepare(statement)?;
    let rows = sql.query_map(params![], |x|x.get(0))?;
    Ok(())
}
pub fn readsql(savepath: &Path) -> Result<(), Error> {
    let mut readvec: Vec<Vec<String>> = Vec::new();
    let connection = sqlite::open(savepath)?;

    let query = "SELECT * FROM users";
    let sqlexe = connection.iterate(query, |items| {
        let mut itemvec: Vec<String> = Vec::new();
        for &item in items.iter() {
            match item.1 {
                Some(sitm) => itemvec.push(sitm.to_string()),
                None => itemvec.push("".to_string()),
            }
        }
        readvec.push(itemvec);
        println!();
        true
    });
    Ok(sqlexe?)
}

// pub fn insertsql(savepath: &Path, items: PartsItem) {}
