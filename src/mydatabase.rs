use rusqlite::{params, Connection, Error, ToSql};
use std::path::Path;

// 一時的にデータを保持して扱いやすくするための構造体
#[derive(Debug, Default, PartialEq)]

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
    conn.execute(
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
        price INTEGER
    )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_parts_search 
         ON partstable(itemtype, order_no, unit_no)",
        [],
    )?;
    Ok(())
}

pub fn insertsql(savepath: &Path, partsitem: &[PartsItem]) -> Result<usize, Error> {
    // Vecで受け取ったアイテムを指定されたPathのデータベースへ登録する
    let mut conn = Connection::open(savepath)?;
    let tx = conn.transaction()?;
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

    // conn.execute_batch("BEGIN;")?;

    {
        let mut stmt = tx.prepare(statement)?;

        for item in partsitem.iter() {
            counter += 1;
            let partnum: Vec<&str> = item.parts_no.split('-').collect();
            let partno = if partnum.len() > 1 {
                partnum[1]
            } else {
                &item.parts_no
            };
            stmt.execute(params![
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
            ])?;
        }
    }
    // conn.execute_batch("COMMIT;")?;
    tx.commit()?;

    // conn.cache_flush()?;
    // }
    Ok(counter)
}

// pub fn order_readsql(
//     savepath: &Path,
//     orderno: &str,
//     itemtype: &str,
//     unitno: &str,
//     searchword: &str,
//     ordercheck: &bool, // 納期超過チェックフラグ
// ) -> Result<Vec<PartsItem>, Error> {
//     let conn = Connection::open(savepath)?;

//     conn.execute_batch("BEGIN;")?;

//     let mut state = conn.prepare("SELECT DISTINCT * From partstable WHERE itemtype == ?")?;
//     let partsitem_iter = state.query_map(params![itemtype], |row| {
//         Ok(PartsItem {
//             // db_id: row.get(0)?,
//             order_no: row.get(1)?,
//             unit_no: row.get(2)?,
//             parts_no: row.get(3)?,
//             rev_mark: row.get(4)?,
//             name: row.get(5)?,
//             itemtype: row.get(6)?,
//             model: row.get(7)?,
//             maker: row.get(8)?,
//             itemqty: row.get(9)?,
//             remarks: row.get(10)?,
//             condition: row.get(11)?,
//             vender: row.get(12)?,
//             order_date: row.get(13)?,
//             delivery_date: row.get(14)?,
//             delicondition: row.get(15)?,
//             price: row.get(16)?,
//         })
//     })?;

//     conn.execute_batch("COMMIT;")?;

//     let searchword = searchword.trim().to_lowercase();
//     let orderno = orderno.trim().to_lowercase();
//     let unitno = unitno.trim();

//     let parts_selector = |it: &PartsItem| {
//         select_order(&orderno, it) && select_multi_units(unitno, it) && search_word(&searchword, it)
//     };

//     let result = partsitem_iter.flatten().filter(parts_selector);

//     if *ordercheck {
//         Ok(result
//             .filter(|it| {
//                 !(it.condition.contains('済')
//                     || it.condition.contains("在庫")
//                     || it.condition.contains("キャンセル")
//                     || it.name.contains("欠番"))
//             })
//             .collect())
//     } else {
//         Ok(result.collect())
//     }
// }
pub fn order_readsql(
    savepath: &Path,
    orderno: &str,
    itemtype: &str,
    unitno: &str,
    searchword: &str,
    ordercheck: &bool,
) -> Result<Vec<PartsItem>, Error> {
    let conn = Connection::open(savepath)?;

    // 1. 基本となるSQL
    let mut sql = "SELECT * FROM partstable WHERE itemtype = ?1".to_string();
    let mut params: Vec<Box<dyn ToSql>> = vec![Box::new(itemtype.to_string())];

    // 2. unit_no の絞り込み (IN 句を使用)
    let units: Vec<i32> = unitno
        .split_whitespace()
        .filter_map(|s| s.parse::<i32>().ok())
        .collect();
    if !units.is_empty() {
        // 1. 指定された数だけ "?" を作り、カンマでつなげて一つの String にする
        let placeholders = vec!["?"; units.len()].join(", ");

        // 2. 結合した文字列を SQL に埋め込む
        sql.push_str(&format!(" AND unit_no IN ({})", placeholders));

        for u in units {
            params.push(Box::new(u));
        }
    }

    // 3. ordercheck (除外条件をSQLで処理)
    if *ordercheck {
        sql.push_str(
            " AND condition NOT LIKE '%済%' 
              AND condition NOT LIKE '%在庫%' 
              AND condition NOT LIKE '%キャンセル%' 
              AND name NOT LIKE '%欠番%'",
        );
    }

    // 4. ステートメントの準備と実行
    let mut stmt = conn.prepare(&sql)?;

    // params_from_iter を使うことで動的な数のパラメータを渡せます
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(PartsItem {
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

    // 前もって小文字変換しておくことで、フィルタリングの際に毎回変換する必要がなくなります
    let orderno = orderno.trim().to_lowercase();
    let searchword = searchword.trim().to_lowercase();
    // 5. 残りの複雑なキーワード検索のみ Rust 側でフィルタリング
    let result: Vec<PartsItem> = rows
        .flatten()
        .filter(|it| select_order(&orderno, it) && search_word(&searchword, it))
        .collect();

    Ok(result)
}
fn select_order(orderno: &str, parts: &PartsItem) -> bool {
    // ordernoはあらかじめ小文字変換しておくことを前提とする
    if orderno.is_empty() {
        return true;
    };

    orderno
        .split_whitespace()
        .all(|x| parts.order_no.to_lowercase().contains(x))
}

// fn select_multi_units(unitsno: &str, parts: &PartsItem) -> bool {
//     if unitsno.is_empty() {
//         return true;
//     };

//     unitsno
//         .split_whitespace()
//         .map(|x| x.parse::<i32>().unwrap_or(99999))
//         .any(|x| x == parts.unit_no)
// }

fn search_word(searchwords: &str, parts: &PartsItem) -> bool {
    // searchwordsはあらかじめ小文字変換しておくことを前提とする
    if searchwords.is_empty() {
        return true;
    };

    // 小文字変換してオーダー番号、名前、型式、メーカ、備考、商社の中でヒットする項目を探す
    let is_pattern = |it: &PartsItem, pattern: &str| {
        it.name.to_lowercase().contains(pattern)
            || it.model.to_lowercase().contains(pattern)
            || it.maker.to_lowercase().contains(pattern)
            || it.remarks.contains(pattern)
            || it.vender.contains(pattern)
    };

    searchwords.split_whitespace().all(|x| is_pattern(parts, x))
}
