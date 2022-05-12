#![windows_subsystem = "windows"]
extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

mod mydatabase;
mod myexcelread;

use anyhow::Result;
use nwd::NwgUi;

use nwg::NativeUi;
use std::env;

// use chrono::{DateTime, FixedOffset, Local};
use glob::glob;
use mydatabase::{createtable, insertsql, order_readsql, PartsItem};
use myexcelread::readexcel;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    // 年毎にデータベースを作成し年で選択する
    let dbfolder = "C:\\Database";
    match fs::read_dir(dbfolder) {
        Ok(_) => {}
        Err(_) => {
            fs::create_dir(dbfolder).unwrap();
        }
    }
    guiapp();
    Ok(())
}
fn pretty_print_int(i: i32) -> String {
    // 千の桁カンマ区切りで文字列を返す
    let mut s = String::new();
    let i_str = i.to_string();

    let a = i_str.chars().rev().enumerate();

    for (idx, val) in a {
        if idx != 0 && idx % 3 == 0 {
            s.insert(0, ',');
        }
        s.insert(0, val);
    }
    s
}

fn excelvec_to_partsitem(ordername: &str, data: &Vec<String>) -> PartsItem {
    let getext = |x: usize| {
        if data.len() < x + 1 {
            "".to_string()
        } else {
            data[x].to_string()
        }
    };
    PartsItem {
        db_id: 0,
        order_no: match ordername.split_once("-") {
            Some(name) => name.0.to_string(),
            None => ordername.to_string(),
        },
        unit_no: match getext(0).parse::<i32>() {
            Ok(num) => num,
            _ => 0,
        },
        parts_no: match getext(2).split_once("-") {
            Some(pno) => pno.1.to_string(),
            None => getext(2),
        },
        rev_mark: getext(3),
        name: getext(4),
        itemtype: getext(1),
        model: getext(5),
        maker: getext(6),
        itemqty: match getext(7).parse::<i32>() {
            Ok(num) => num,
            _ => 0,
        },
        remarks: getext(8),
        condition: getext(9),
        vender: getext(10),
        order_date: getext(13),
        delivery_date: getext(14),
        delicondition: getext(15),
        price: match getext(11).parse::<i32>() {
            Ok(num) => num,
            _ => 0,
        },
    }
}
fn delete_db_file(datapath: &Path) -> Result<()> {
    fs::remove_file(datapath)?;
    Ok(())
}
fn read_excel_files(selectyear: i32, datapath: &Path) -> Result<usize> {
    // エクセルファイルを検索してデータベースへ登録する
    let selectdir: String;
    delete_db_file(datapath)?;
    if selectyear == 0 {
        selectdir = format!("D:\\Data\\Excelbackup\\");
    } else {
        selectdir = format!("\\\\LS220DB3C9\\share\\発注管理\\{}\\", selectyear);
    }
    createtable(datapath)?;
    let mut counter = 0;
    let currentpath = Path::new(selectdir.as_str());
    let mut getitems: Vec<PartsItem> = Vec::new();
    match env::set_current_dir(currentpath) {
        Ok(_) => {
            for partype in ["購入", "加工"].into_iter() {
                let pattern = format!("./**/*{}*.xlsx", partype);
                let targetfiles = glob(&pattern).expect("cannot find excel file");
                for itemname in targetfiles {
                    let excelname = itemname.expect("can not open excel file");
                    // println!("{:?}", excelname);

                    match readexcel(&excelname) {
                        Ok(datavec) => {
                            let mut insert_items: Vec<PartsItem> = Vec::new();
                            for dt in datavec.iter() {
                                let ordername = excelname.file_name().unwrap().to_str().unwrap();

                                let item = excelvec_to_partsitem(ordername, dt);

                                insert_items.push(item);
                            }
                            counter += &insert_items.len();
                            getitems.extend(insert_items);
                            // let instnum = insertsql(datapath, insert_items)?;
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
    insertsql(datapath, getitems)?;
    Ok(counter)
}

fn guiapp() {
    nwg::init().expect("Failed to init Native Windows GUI");
    // nwg::Font::set_global_family("MS UI Gothic").expect("Failed to set default font");
    let mut defaultfont = nwg::Font::default();
    nwg::Font::builder()
        .size(14)
        .family("MS UI Gotic")
        .build(&mut defaultfont)
        .expect("Failed to set default font");
    nwg::Font::set_global_default(Some(defaultfont));
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

    // google search
    #[nwg_control(text:"Google Search",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col:5,row:0)]
    #[nwg_events(OnButtonClick:[DataViewApp::google_search])]
    google_btn: nwg::Button,

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

    // 購入加工選択ボックス
    #[nwg_control(collection: vec!["購入", "加工"], selected_index: Some(0), font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 5, row: 7)]
    #[nwg_events( OnComboxBoxSelection: [DataViewApp::update_view] )]
    partstype: nwg::ComboBox<&'static str>,

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

    // ステータスバー1
    #[nwg_control(text: "Status",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 1, row: 13)]
    statuslabel1: nwg::Label,
    // ステータスバー2
    #[nwg_control(text: "Status",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 4, row: 13)]
    statuslabel2: nwg::Label,
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
        // dataview.insert_column("SQL_ID");
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
        let selectyear = self.year_input.text().trim().parse::<i32>();
        // 範囲外の年代の入力に対するガードパターン
        match selectyear {
            Ok(n) => {
                if 2019 <= n && n <= 2035 {
                    ()
                } else {
                    return self
                        .statuslabel1
                        .set_text("年代に数値を正しく入力してください");
                }
            }
            Err(_) => self
                .statuslabel1
                .set_text("年代に数値を正しく入力してください"),
        }

        match selectyear {
            Ok(num) => {
                self.statuslabel1.set_text("データベースを作成中です");
                let dpath = self.get_database_path(num);
                let databasepath = Path::new(dpath.as_str());
                match read_excel_files(num, databasepath) {
                    Ok(getitems) => {
                        let statustext = format!("{}件をデータベースに登録しました", getitems);
                        self.statuslabel1.set_text(&statustext);
                    }
                    Err(_) => (),
                };
            }
            _ => (),
        }
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
                // if self.typelabel.text() != "加工" {
                self.kakou_data(); //状態ラベルを切換
                self.set_listdatabase()
                // }
            }
            None | Some(_) => (),
        }
    }

    fn set_listdatabase(&self) {
        match self.read_database() {
            Ok(_) => (),
            Err(e) => self.statuslabel1.set_text(e.to_string().as_str()),
        }
    }
    fn read_database(&self) -> Result<()> {
        self.statuslabel1.set_text("");
        self.statuslabel2.set_text("");
        let dataview = &self.data_view;
        let mut grossprice = 0;
        dataview.clear();
        let selectyear = self.year_input.text();
        let yearnum = selectyear.parse::<i32>()?;
        let selectedtype = self.partstype.selection_string().unwrap();
        if (2019 <= yearnum && yearnum <= 2035) || yearnum == 0 {
            ()
        } else {
            self.statuslabel1
                .set_text("年代は2019～2035の値を入力してください");
            return Ok(());
        }
        // gui から 検索キーワードを取得
        let select_order = self.order_input.text();
        let search_word = self.search_edit.text();
        let selectdir = self.get_database_path(yearnum);
        let selectunit = self.unit_input.text();
        let databasepath = Path::new(selectdir.as_str());
        // アイテムを絞り込み検索
        let contents = order_readsql(
            databasepath,
            &select_order,
            &selectedtype,
            &selectunit,
            &search_word,
        )?;

        self.statuslabel1
            .set_text(format!("{}件の該当項目があります", contents.len()).as_str());
        let mut has_zero = false;

        // guiにアイテムをセット
        for (indexnum, items) in contents.iter().enumerate() {
            let gpartprice = items.price * items.itemqty;
            if gpartprice == 0 && items.name.trim() != "欠番" && !items.remarks.contains("支給品")
            {
                has_zero = true
            }
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
                pretty_print_int(items.price),
                pretty_print_int(gpartprice),
                // items.db_id.to_string(),
            ];

            self.setlist_item(indexnum as i32, toitem);
            let listlimit = 5000;
            if indexnum > listlimit {
                self.statuslabel1
                    .set_text(format!("{}件までを表示しています。", listlimit).as_str());
                break;
            }
        }

        if has_zero {
            self.statuslabel2.set_text("合計金額は不正確の可能性あり")
        }
        let gokeikingaku = format!("￥ {}", pretty_print_int(grossprice));
        self.grossprice.set_text(&gokeikingaku);
        // リストを選択状態にする
        dataview.select_item(0, true);
        Ok(())
    }

    fn setlist_item<T: ToString>(&self, indexnum: i32, listdata: Vec<T>) {
        // GUIの表の構成
        let dataview = &self.data_view;
        for (colnum, itemtext) in listdata.iter().enumerate() {
            let listdata = nwg::InsertListViewItem {
                index: Some(indexnum),
                column_index: colnum as i32,
                text: Some(itemtext.to_string()),
                image: None,
            };
            dataview.insert_item(listdata)
        }
    }

    fn getlistitem(&self) {
        let listview = &self.data_view;
        // get row
        let selectrow = listview.selected_item();

        match selectrow {
            Some(row) => {
                let items = listview.item(row, 0, 20).unwrap().text;
                self.order_input.set_text(&items);
                let model = listview.item(row, 4, 20).unwrap().text;
                let maker = listview.item(row, 5, 20).unwrap().text;
                self.statuslabel1
                    .set_text(format!("{}: {}", maker, model).as_str());
                // listview.select_item(row, true);
            }
            None => (),
        }
    }

    fn google_search(&self) {
        let listview = &self.data_view;
        let selectrow = listview.selected_item();
        match selectrow {
            Some(row) => {
                let model = listview.item(row, 4, 30).unwrap().text;
                let maker = listview.item(row, 5, 20).unwrap().text;
                let searchword = format!("{} {}", model, maker);
                let open_url = format!("https://www.google.com/search?q={}", searchword);
                webbrowser::open(&open_url).unwrap();
                self.statuslabel1
                    .set_text(format!("{}をWEB検索", searchword).as_str());
            }
            None => self
                .statuslabel1
                .set_text("検索エラー:アイテムを選択してください"),
        }
    }
    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

#[test]
fn pretty_print_test() {
    assert_eq!(&pretty_print_int(0), "0");
    assert_eq!(&pretty_print_int(1), "1");
    assert_eq!(&pretty_print_int(200), "200");
    assert_eq!(&pretty_print_int(1000), "1,000");
    assert_eq!(&pretty_print_int(50000), "50,000");
    assert_eq!(&pretty_print_int(900000), "900,000");
    assert_eq!(&pretty_print_int(1900000), "1,900,000");
}
