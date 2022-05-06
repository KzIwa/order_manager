#![windows_subsystem = "windows"]
extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;
mod mydatabase;
mod myexcelread;

use anyhow::{Error, Result};
use nwd::{NwgPartial, NwgUi};
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{AlignSelf, Dimension, FlexDirection},
};
use nwg::NativeUi;
use std::{env, f32::consts::E};

use chrono::{DateTime, FixedOffset, Local};
use glob::glob;
use mydatabase::{createsql, insertsql, order_readsql, PartsItem};
use myexcelread::readexcel;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    // 年毎にデータベースを作成し年で選択する
    let selectyear = 2019;
    let dbfolder = "C:\\Database";
    match fs::read_dir(dbfolder) {
        Ok(_) => {}
        Err(_) => {
            fs::create_dir(dbfolder).unwrap();
        }
    }
    // fs::create_dir("C:\\Database");
    let selectdir = format!("{}\\parts{}.db3", dbfolder, selectyear);
    let databasepath = Path::new(selectdir.as_str());
    createsql(databasepath)?;

    guiapp();

    // read_excel_files(selectyear,databasepath)?;
    Ok(())
}

fn excelvec_to_partsitem(ordername: &str, data: &Vec<String>) -> Result<PartsItem> {
    Ok(PartsItem {
        db_id: 0,
        order_no: match ordername.split_once("-") {
            Some(name) => name.0.to_string(),
            None => ordername.to_string(),
        },
        unit_no: data[0].parse::<i32>()?,
        parts_no: match data[2].to_string().split_once("-") {
            Some(pno) => pno.1.to_string(),
            None => data[2].to_string(),
        },
        rev_mark: data[3].to_string(),
        name: data[4].to_string(),
        itemtype: data[1].to_string(),
        model: data[5].to_string(),
        maker: data[6].to_string(),
        itemqty: data[7].parse::<i32>()?,
        remarks: data[8].to_string(),
        condition: data[9].to_string(),
        vender: data[10].to_string(),
        order_date: "".to_string(),
        delivery_date: "".to_string(),
        delicondition: "".to_string(),
        price: match data[11].parse::<i32>() {
            Ok(num) => num,
            _ => 0,
        },
    })
}
fn read_excel_files(selectyear: i32, datapath: &Path) -> Result<()> {
    let selectdir = format!("\\\\LS220DB3C9\\share\\発注管理\\{}\\", selectyear);
    let currentpath = Path::new(selectdir.as_str());
    match env::set_current_dir(currentpath) {
        Ok(_) => {
            for partype in ["購入", "加工"].into_iter() {
                let pattern = format!("./**/*{}*.xlsx", partype);
                let targetfiles = glob(&pattern).expect("cannot find excel file");
                for itemname in targetfiles {
                    let excelname = itemname.expect("can not open excel file");
                    match readexcel(&excelname) {
                        Ok(datavec) => {
                            let mut insert_items: Vec<PartsItem> = Vec::new();
                            for dt in datavec.iter() {
                                let ordername = excelname.file_name().unwrap().to_str().unwrap();
                                let data = excelvec_to_partsitem(ordername, dt);
                                match data {
                                    Ok(item) => {
                                        insert_items.push(item);
                                    }
                                    _ => (),
                                }
                            }
                            insertsql(datapath, insert_items).ok();
                        }
                        _ => (),
                    }
                }
            }
        }
        Err(e) => {
            println!("{}", e);
        }
    };
    Ok(())
}

fn guiapp() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let window = Default::default();
    let _app = DataViewApp::build_ui(window).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}

#[derive(Default, NwgUi)]
pub struct DataViewApp {
    // マクロを使ってwindow 構成を生成している
    #[nwg_control(size:(1500,500), position: (300, 300), title: "部品管理")]
    #[nwg_events( OnWindowClose:[DataViewApp::exit],OnInit: [DataViewApp::load_data])]
    window: nwg::Window,

    #[nwg_resource(family: "Meiryo", size: 19)]
    appfont: nwg::Font,

    // レイアウト管理
    #[nwg_layout(parent:window,max_row:Some(14),spacing:3)]
    mylayout: nwg::GridLayout,

    // 部品リスト
    #[nwg_control(item_count: 16,list_style:nwg::ListViewStyle::Detailed,
        ex_flags: nwg::ListViewExFlags::AUTO_COLUMN_SIZE | nwg::ListViewExFlags::FULL_ROW_SELECT)]
    #[nwg_layout_item(layout: mylayout,col: 0, col_span: 5, row: 0, row_span: 13)]
    #[nwg_events(OnListViewClick:[DataViewApp::getlistitem])]
    data_view: nwg::ListView,

    // 購入加工選択ボックス
    #[nwg_control(collection: vec!["購入", "加工"], selected_index: Some(0), font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 0)]
    #[nwg_events( OnComboxBoxSelection: [DataViewApp::update_view] )]
    partstype: nwg::ComboBox<&'static str>,

    // 年度
    #[nwg_control(text: "年代",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 1)]
    yearlabel: nwg::Label,
    #[nwg_control(text: "",font: Some(&data.appfont),focus:true)]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 2)]
    #[nwg_events()]
    year_input: nwg::TextInput,

    // 購入加工の状態保持無限ループを防ぐ 状態フラグ["購入", "加工"]
    #[nwg_control(text: "")]
    typelabel: nwg::Label,

    // 注文番号
    #[nwg_control(text: "注文番号",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 3)]
    orderlabel: nwg::Label,

    #[nwg_control(text: "",font: Some(&data.appfont),focus:true)]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 4, row_span: 1)]
    #[nwg_events()]
    order_input: nwg::TextInput,

    #[nwg_control(text: "枝番号",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 5)]
    unitlabel: nwg::Label,
    #[nwg_control(text: "",font: Some(&data.appfont),focus:true)]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 6, row_span: 1)]
    #[nwg_events()]
    unit_input: nwg::TextInput,
    // 検索語
    #[nwg_control(text: "検索語",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 8)]
    searchlabel: nwg::Label,
    #[nwg_control(text: "",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 9)]
    search_edit: nwg::TextInput,
    // 検索ボタン
    #[nwg_control(text:"Search",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col:5,row:10)]
    #[nwg_events(OnButtonClick:[DataViewApp::set_listdatabase])]
    search_btn: nwg::Button,

    // クリアボタン
    #[nwg_control(text:"Clear",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col:5,row:11)]
    #[nwg_events(OnButtonClick:[DataViewApp::clear_all])]
    clear_btn: nwg::Button,

    // 合計金額
    #[nwg_control(text: "合計金額",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 12)]
    grosslabel: nwg::Label,
    #[nwg_control(text: "----",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 13)]
    grossprice: nwg::Label,

    // ステータスバー
    #[nwg_control(text: "Status",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 1, row: 13)]
    statuslabel: nwg::Label,
    // Reloadボタン
    #[nwg_control(text:"Reload",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col:0,row:13)]
    #[nwg_events(OnButtonClick:[DataViewApp::reload_database])]
    reload_btn: nwg::Button,
}

impl DataViewApp {
    fn load_data(&self) {
        let dataview = &self.data_view;
        // 状態フラグなので見えないようにしている
        self.typelabel.set_visible(false);
        // リストビューの初期セッティング
        dataview.insert_column("注番");
        dataview.insert_column("枝番");
        dataview.insert_column("番号");
        dataview.insert_column("部品名称");
        dataview.insert_column("材質/型式");
        dataview.insert_column("処理/メーカ");
        dataview.insert_column("数量");
        dataview.insert_column("備考");
        dataview.insert_column("発注状況");
        dataview.insert_column("発注先");
        dataview.insert_column("発注日");
        dataview.insert_column("予定納期");
        dataview.insert_column("入荷済み");
        dataview.insert_column("単価");
        dataview.insert_column("金額");
        dataview.insert_column("SQL_ID");
        dataview.set_headers_enabled(true);
        self.konyu_data()
    }

    fn get_database_path(&self, selectyear: i32) -> String {
        let dbfolder = "C:\\Database";
        format!("{}\\parts{}.db3", dbfolder, selectyear)
    }

    fn clear_all(&self) {
        self.search_edit.set_text("");
        self.unit_input.set_text("");
        self.order_input.set_text("");
    }
    fn reload_database(&self) {
        let selectyear = self.year_input.text().parse::<i32>();
        match selectyear {
            Ok(n) => {
                if 2019 <= n && n <= 2030 {
                    ()
                } else {
                    return self
                        .statuslabel
                        .set_text("年代に数値を正しく入力してください");
                }
            }
            Err(_) => self
                .statuslabel
                .set_text("年代に数値を正しく入力してください"),
        }
        match selectyear {
            Ok(num) => {
                self.statuslabel.set_text("データベースを作成中です");
                let dpath = self.get_database_path(num);
                let databasepath = Path::new(dpath.as_str());
                read_excel_files(num, databasepath).ok();
                self.statuslabel.set_text("データベースを作成しました。")
            }
            _ => (),
        }
    }

    fn parse_data(&self) {
        todo!()
    }
    fn konyu_data(&self) {
        self.typelabel.set_text("購入");
        self.data_view.update_column(4, "型式");
        self.data_view.update_column(5, "メーカ");
    }
    fn kakou_data(&self) {
        self.typelabel.set_text("加工");
        self.data_view.update_column(4, "材質");
        self.data_view.update_column(5, "処理");
    }
    fn update_view(&self) {
        let value = self.partstype.selection_string();
        // matchの範囲外に処理を入れると無限ループ
        // 状態フラグで無限ループ防いでいる
        match value.as_ref().map(|x| x as &str) {
            Some("購入") => {
                if self.typelabel.text() != "購入" {
                    self.konyu_data(); //状態ラベルを切換
                    self.set_listdatabase()
                }
            }
            Some("加工") => {
                if self.typelabel.text() != "加工" {
                    self.kakou_data(); //状態ラベルを切換
                    self.set_listdatabase()
                }
            }
            None | Some(_) => (),
        }
    }

    fn set_listdatabase(&self) {
        match self.read_database() {
            Ok(_) => (),
            Err(e) => self.statuslabel.set_text(e.to_string().as_str()),
        }
    }
    fn read_database(&self) -> Result<()> {
        let dataview = &self.data_view;
        let mut grossprice = 0;
        dataview.clear();
        let selectyear = self.year_input.text();
        let yearnum = selectyear.parse::<i32>()?;
        let selectedtype = self.partstype.selection_string().unwrap();
        if 2019 <= yearnum && yearnum < 2030 {
            ()
        } else {
            self.statuslabel
                .set_text("年代は2019～2030の値を入力してください");
            return Ok(());
        }
        let select_order = self.order_input.text();
        let search_word = self.search_edit.text();
        let selectdir = self.get_database_path(yearnum);
        let selectunit = self.unit_input.text();
        let databasepath = Path::new(selectdir.as_str());
        let contents = order_readsql(
            databasepath,
            &select_order,
            &selectedtype,
            &selectunit,
            &search_word,
        )?;
   
        self.statuslabel
            .set_text(format!("{}件の該当項目があります", contents.len()).as_str());
        for (indexnum, items) in contents.iter().enumerate() {
            let gpartprice = items.price * items.itemqty;
            grossprice += gpartprice;
            let toitem = vec![
                items.order_no.to_string(),
                items.unit_no.to_string(),
                items.parts_no.to_string(),
                items.name.to_string(),
                items.model.to_string(),
                items.maker.to_string(),
                items.itemqty.to_string(),
                items.remarks.to_string(),
                items.condition.to_string(),
                items.vender.to_string(),
                items.order_date.to_string(),
                items.delivery_date.to_string(),
                items.delicondition.to_string(),
                items.price.to_string(),
                gpartprice.to_string(),
                items.db_id.to_string(),
            ];
            self.set_item(indexnum as i32, &toitem);
        }
        self.grossprice.set_text(grossprice.to_string().as_str());
        // リストを選択状態にする
        dataview.select_item(0, true);
        Ok(())
    }

    fn set_item<T: ToString>(&self, indexnum: i32, listdata: &Vec<T>) {
        let itemdata: Vec<String> = listdata.iter().map(|x| x.to_string()).collect();
        let dataview = &self.data_view;

        for (colnum, itemtext) in itemdata.iter().enumerate() {
            let listdata = nwg::InsertListViewItem {
                index: Some(indexnum),
                column_index: colnum as i32,
                text: Some(itemtext.into()),
                image: None,
            };
            dataview.insert_item(listdata)
        }
        self.update_view()
    }

    fn getlistitem(&self) {
        let listview = &self.data_view;
        // get row
        let selectrow = listview.selected_item();

        match selectrow {
            Some(row) => {
                let items = listview.item(row, 0, 20).unwrap().text;
                self.order_input.set_text(&items);
                // listview.select_item(row, true);
            }
            None => (),
        }
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}
