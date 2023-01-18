#![windows_subsystem = "windows"]
extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

mod mydatabase;
mod myexcelread;
mod robocopy;
use anyhow::{Context, Result};
use glob::glob;
use mydatabase::{createtable, insertsql, order_readsql, PartsItem};
use myexcelread::readexcel;
use nwd::NwgUi;
use nwg::NativeUi;
use robocopy::diffcopy;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::path::Path;
use std::{env, fs};
use webbrowser::{self, Browser};

use std::{cell::RefCell, thread};

/// The dialog UI
#[derive(Default, NwgUi)]
pub struct ReloadDialog {
    data: RefCell<Option<String>>,
    #[nwg_control(size: (300, 115), position: (650, 300), title: "Reloadする年代を入力してください", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [ReloadDialog::close] )]
    window: nwg::Window,
    // 年度
    #[nwg_control(text: "年代", position: (15, 12),)]
    year_label: nwg::Label,
    #[nwg_control(text: "",position: (50, 10),size: (220, 22),focus:true)]
    year_input: nwg::TextInput,

    #[nwg_control(text: "Reloadしますか？", position: (15, 45),)]
    choice_label: nwg::Label,

    #[nwg_control(text: "YES", position: (10, 70), size: (130, 40))]
    #[nwg_events( OnButtonClick: [ReloadDialog::choose(SELF, CTRL)] )]
    choice_yes: nwg::Button,

    #[nwg_control(text: "NO", position: (160, 70), size: (130, 40))]
    #[nwg_events( OnButtonClick: [ReloadDialog::choose(SELF, CTRL)] )]
    choice_no: nwg::Button,
}

impl ReloadDialog {
    /// Create the dialog UI on a new thread. The dialog result will be returned by the thread handle.
    /// To alert the main GUI that the dialog completed, this function takes a notice sender object.
    fn popup(sender: nwg::NoticeSender, year: String) -> thread::JoinHandle<String> {
        thread::spawn(move || {
            // Create the UI just like in the main function
            let app = ReloadDialog::build_ui(Default::default()).expect("Failed to build UI");
            app.year_input.set_text(&year);
            nwg::dispatch_thread_events();

            // Notice the main thread that the dialog completed
            sender.notice();

            // Return the dialog data
            app.data.take().unwrap_or("Cancelled!".to_owned())
        })
    }

    fn close(&self) {
        nwg::stop_thread_dispatch();
    }

    fn choose(&self, btn: &nwg::Button) {
        let mut data = self.data.borrow_mut();
        if btn == &self.choice_no {
            *data = Some("Reloadはキャンセルされました".to_string());
        } else if btn == &self.choice_yes {
            *data = self.reload_database();
        }

        self.window.close();
    }
    fn get_database_path(&self, selectyear: i32) -> String {
        let dbfolder = "C:\\Database";
        format!("{}\\parts{}.db3", dbfolder, selectyear)
    }

    fn reload_database(&self) -> Option<String> {
        let settingitem = SettingItem::new();
        let selectyear = self.year_input.text().trim().parse::<i32>();
        // 範囲外の年代の入力に対するガードパターン
        match selectyear {
            Ok(n) => {
                if (2019..=2035).contains(&n) {
                } else {
                    return Some("年代に数値を正しく入力してください.".to_string());
                }
            }
            Err(_) => return Some("年代に数値を正しく入力してください.".to_string()),
        }

        match selectyear {
            Ok(num) => {
                self.choice_yes.set_enabled(false);
                self.choice_label.set_text("DataBase更新中");
                let dpath = self.get_database_path(num);
                let databasepath = Path::new(dpath.as_str());
                let temppath = self.get_database_path(9999);
                let dbtemp_path = Path::new(temppath.as_str());
                match settingitem {
                    Ok(st) => {
                        let targetfolder = st.searchfolder;
                        diffcopy(&num, &targetfolder).unwrap();
                        match read_excel_files(num, dbtemp_path) {
                            Ok(getitems) => {
                                let statustext =
                                    format!("{}件をデータベースに登録しました", getitems);
                                if getitems > 0 {
                                    delete_db_file(databasepath)
                                        .expect("データベースが削除できませんでした");
                                    fs::rename(dbtemp_path, databasepath)
                                        .expect("データベースを改名できません");
                                } else {
                                    delete_db_file(dbtemp_path)
                                        .expect("データベースが削除できませんでした");
                                    delete_db_file(databasepath)
                                        .expect("データベースが削除できませんでした");
                                }
                                Some(statustext)
                            }
                            Err(_) => Some("".to_string()),
                        }
                    }
                    Err(_) => Some("C:\\Database\\partsetting.txtが見つかりません".to_string()),
                }
            }
            _ => Some("".to_string()),
        }
    }
}

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
//設定パラメータを保持する構造体
struct SettingItem {
    maxdisplay_linenumber: usize,
    searchfolder: String,
}
impl SettingItem {
    fn new() -> Result<Self> {
        /*
        partsetting.txtに記載にitemname@と記載後、値を書くことで新しい設定値を定義できる。
        設定値をファイルから読み出す
        Setting名称をキーとして設定値をHashMapに格納。タプルの0番目に数値、1番目にVec<文字列>
        Open Setting file
        */
        let setting_file = "C:\\Database\\partsetting.txt";
        let f = fs::File::open(setting_file).with_context(|| "failed to open file".to_string())?;

        let mut buffer = BufReader::new(f);
        let mut settings = String::with_capacity(1028);

        buffer
            .read_to_string(&mut settings)
            .with_context(|| "failed to read settings".to_string())?;
        // 改行文字やタブ文字など不用な文字を削除

        for removestr in [" ", "\r\n", "\n", "\t"].iter() {
            settings = settings.replace(removestr, "");
        }

        settings = settings.trim().to_string();
        let setting_items = settings.split("---");
        let mut partsettings = HashMap::new();

        for setting_info in setting_items {
            let setting_group: Vec<&str> = setting_info.split('@').collect();
            let mut item_value = Vec::new();
            let mut item_str = String::new();
            let values = setting_group[1].split(';');
            for val in values.filter(|x| !x.contains("//")) {
                // 数値と文字列で取り込み方を分岐
                match val.parse::<usize>() {
                    Ok(n) => item_value.push(n),
                    Err(_) => {
                        item_str = val.to_string();
                    }
                }
            }
            let item_name = setting_group[0].to_string();
            partsettings.insert(item_name, (item_value, item_str));
        }
        // パースされた数値を取り出したいときは~.0 文字列のときは~.1
        let maxdisplay_linenumber = partsettings["max_line"].0[0];
        let searchfolder = (partsettings["search_folder"].1).to_string();

        Ok(Self {
            maxdisplay_linenumber,
            searchfolder,
        })
    }
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

fn excelvec_to_partsitem(ordername: &str, data: &[String]) -> PartsItem {
    let getext = |x: usize| {
        if data.len() < x + 1 {
            "".to_string()
        } else {
            data[x].to_string()
        }
    };

    PartsItem {
        // db_id: 0,
        order_no: match ordername.split_once('_') {
            Some(name) => name.1.to_string(),
            None => ordername.to_string(),
        },
        unit_no: match getext(0).parse::<i32>() {
            Ok(num) => num,
            _ => 0,
        },
        parts_no: match getext(2).split_once('-') {
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
    if fs::read(datapath).is_ok() {
        fs::remove_file(datapath)?;
    }

    Ok(())
}

fn get_dbyear() -> Result<Vec<String>> {
    let selectdir = "C:\\Database\\".to_string();
    let currentpath = Path::new(selectdir.as_str());
    let mut getnames = Vec::new();
    if env::set_current_dir(currentpath).is_ok() {
        let pattern = "./*.db3".to_string();
        let dbnames = glob(&pattern)?;
        for name in dbnames {
            let dbname = name?.to_owned();
            let yearname = dbname.to_string_lossy().into_owned();
            let yearname = yearname.replace(".db3", "");
            let yearname = yearname.replace("parts", "");
            // let nyearname = yearname[..];
            getnames.push(yearname);
        }
    }

    getnames.reverse();
    Ok(getnames)
}

fn read_excel_files(selectyear: i32, datapath: &Path) -> Result<usize> {
    // エクセルファイルを検索してデータベースへ登録する

    let selectdir = format!("C:\\Database\\excel\\{}\\", selectyear);
    createtable(datapath)?;
    let mut counter = 0;
    let currentpath = Path::new(selectdir.as_str());
    let mut getitems = Vec::new();

    match env::set_current_dir(currentpath) {
        Ok(_) => {
            for partype in ["購入", "加工"].into_iter() {
                let pattern = format!("./**/*{}*.xlsx", partype);
                let targetfiles = glob(&pattern)?;

                for itemname in targetfiles {
                    let excelname = itemname?;

                    if let Ok(datavec) = readexcel(&excelname) {
                        // Ok(datavec) => {
                        let mut inner_counter = 0;

                        datavec.iter().for_each(|dt| {
                            let filename = excelname.file_name().unwrap().to_str().unwrap();
                            let ordername = excelname.parent().unwrap().to_str().unwrap();
                            let item = excelvec_to_partsitem(ordername, dt);

                            if !filename.contains("~$") {
                                getitems.push(item);
                                inner_counter += 1;
                            }
                        });

                        counter += inner_counter;
                    }
                    //     _ => (),
                    // }
                }
            }
        }
        Err(e) => {
            println!("{}", e);
        }
    };
    insertsql(datapath, &getitems)?;
    Ok(counter)
}

fn guiapp() {
    nwg::init().expect("Failed to init Native Windows GUI");
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
    // PopUpダイアログでやり取りするための変数
    dialog_data: RefCell<Option<thread::JoinHandle<String>>>,

    // マクロを使ってwindow 構成を生成している
    #[nwg_control(size:(1500,500), position: (300, 300), title: "部品管理")]
    #[nwg_events( OnWindowClose:[DataViewApp::exit],OnInit: [DataViewApp::load_data])]
    window: nwg::Window,

    #[nwg_resource(family: "Meiryo", size: 20)]
    appfont: nwg::Font,

    // PopUpダイアログでイベント通知の変数
    #[nwg_control]
    #[nwg_events( OnNotice: [DataViewApp::read_dialog_output] )]
    dialog_notice: nwg::Notice,

    // レイアウト管理
    #[nwg_layout(parent:window,max_row:Some(16),spacing:3)]
    mylayout: nwg::GridLayout,
    // 部品リスト
    #[nwg_control(item_count: 16,list_style:nwg::ListViewStyle::Detailed,
        ex_flags: nwg::ListViewExFlags::AUTO_COLUMN_SIZE | nwg::ListViewExFlags::FULL_ROW_SELECT)]
    #[nwg_layout_item(layout: mylayout,col: 0, col_span: 10, row: 0, row_span: 15)]
    #[nwg_events(OnListViewClick:[DataViewApp::select_list_action])]
    data_view: nwg::ListView,

    // google search
    #[nwg_control(text:"Google Search",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col: 10,row:0)]
    #[nwg_events(OnButtonClick:[DataViewApp::google_search])]
    google_btn: nwg::Button,

    // 年度
    #[nwg_control(text: "年代",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 1)]
    yearlabel: nwg::Label,
    // #[nwg_control(text: "",font: Some(&data.appfont),focus:true)]
    // #[nwg_layout_item(layout: mylayout, col: 10, row: 2)]
    // #[nwg_events()]
    // year_input: nwg::TextInput,
    #[nwg_control(font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 2)]
    #[nwg_events( OnComboxBoxSelection: [DataViewApp::update_view] )]
    year_input: nwg::ComboBox<String>,

    // 注文番号
    #[nwg_control(text: "注文番号",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 3)]
    orderlabel: nwg::Label,
    #[nwg_control(text: "",font: Some(&data.appfont),focus:false)]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 4, row_span: 1)]
    order_input: nwg::TextInput,

    #[nwg_control(text: "枝番号",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 5)]
    unitlabel: nwg::Label,
    #[nwg_control(text: "",font: Some(&data.appfont),focus:false)]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 6, row_span: 1)]
    #[nwg_events(OnTextInput:[DataViewApp::update_view])]
    unit_input: nwg::TextInput,

    // 購入加工選択ボックス
    #[nwg_control(font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 7)]
    #[nwg_events( OnComboxBoxSelection: [DataViewApp::update_view] )]
    partstype: nwg::ComboBox<&'static str>,

    // 検索語
    #[nwg_control(text: "検索語",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 8)]
    searchlabel: nwg::Label,
    #[nwg_control(text: "",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 9)]
    search_edit: nwg::TextInput,

    // 検索ボタン
    #[nwg_control(text:"Search",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col: 10,row:10)]
    #[nwg_events(OnButtonClick:[DataViewApp::update_view])]
    search_btn: nwg::Button,

    // 未手配チェックボックス
    #[nwg_control(text:"未手配チェック")]
    #[nwg_layout_item(layout:mylayout,col: 10,row:11)]
    #[nwg_events(OnButtonClick:[DataViewApp::update_view])]
    ordered_check: nwg::CheckBox,

    // クリアボタン
    #[nwg_control(text:"検索/枝番Clear",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col: 10,row:13)]
    #[nwg_events(OnButtonClick:[DataViewApp::clear_all])]
    clear_btn: nwg::Button,
    //型式
    #[nwg_control(text: "リストをクリック",font: Some(&data.appfont),readonly:true)]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 12)]
    model_edit: nwg::TextInput,

    // 合計金額
    #[nwg_control(text: "合計金額",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 14)]
    grosslabel: nwg::Label,
    #[nwg_control(text: "----",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 10, row: 15)]
    grossprice: nwg::Label,

    // ステータスバー1
    #[nwg_control(text: "Status",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 1,col_span:3, row: 15)]
    statuslabel1: nwg::Label,

    // ステータスバー2
    #[nwg_control(text: "Status",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 6,col_span:3, row: 15)]
    statuslabel2: nwg::Label,

    // Reloadボタン
    #[nwg_control(text:"Reload",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col:0,row:15)]
    #[nwg_events(OnButtonClick:[DataViewApp::open_dialog])]
    reload_btn: nwg::Button,
}

impl DataViewApp {
    fn load_data(&self) {
        let dataview = &self.data_view;

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

        dataview.set_headers_enabled(true);

        let mtemppath = self.get_database_path(9999);
        let mdbtemp_path = Path::new(mtemppath.as_str());

        delete_db_file(mdbtemp_path).expect("ok");

        self.konyu_data();
        self.partstype.set_collection(vec!["購入", "加工"]);
        self.partstype.set_selection(Some(0));
        self.set_year_select();
        self.year_input.set_selection(Some(0));
    }

    fn set_year_select(&self) {
        let years = get_dbyear();

        match years {
            Ok(ys) => self.year_input.set_collection(ys),
            _ => self.year_input.set_collection(vec!["".to_string()]),
        };
    }

    fn get_database_path(&self, selectyear: i32) -> String {
        let dbfolder = "C:\\Database";
        format!("{}\\parts{}.db3", dbfolder, selectyear)
    }

    fn clear_all(&self) {
        self.clear_btn.set_enabled(false);
        self.search_edit.set_text("");
        self.unit_input.set_text("");
        self.ordered_check
            .set_check_state(nwg::CheckBoxState::Unchecked);
    }

    fn konyu_data(&self) {
        self.data_view.update_column(4, "型式");
        self.data_view.update_column(5, "メーカ");
    }

    fn kakou_data(&self) {
        self.data_view.update_column(4, "材質");
        self.data_view.update_column(5, "処理");
    }

    fn update_view(&self) {
        if self.clear_btn.enabled() {
            let value = self.partstype.selection_string();
            match value.as_ref().map(|x| x as &str) {
                Some("購入") => {
                    self.konyu_data();
                    self.set_listdatabase()
                }
                Some("加工") => {
                    self.kakou_data();
                    self.set_listdatabase()
                }
                None | Some(_) => (),
            }
        } else {
            self.clear_btn.set_enabled(true);
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
        let orderedcheck = self.ordered_check.check_state() == nwg::CheckBoxState::Checked;
        let mut grossprice = 0;
        dataview.clear();

        let selectyear = self.year_input.selection_string();
        let yearnum = match selectyear {
            Some(year) => year.parse::<i32>()?,
            None => 9999,
        };
        let selectedtype = self.partstype.selection_string().unwrap();
        // 年代ガード
        if (2019..=2035).contains(&yearnum) || yearnum == 0 {
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
            &orderedcheck,
        )?;

        self.statuslabel1
            .set_text(format!("{}件の該当項目があります", contents.len()).as_str());
        let mut has_zero = false;
        let settingitem = SettingItem::new();

        match settingitem {
            Ok(st) => {
                self.search_btn.set_enabled(false);
                let listlimit = st.maxdisplay_linenumber;
                // guiにアイテムをセット
                for (indexnum, items) in contents.iter().enumerate() {
                    let gpartprice = items.price * items.itemqty;
                    if gpartprice == 0
                        && items.name.trim() != "欠番"
                        && !items.remarks.contains("支給品")
                    {
                        has_zero = true
                    }
                    grossprice += gpartprice;
                    // string_to_time(&items.delivery_date);
                    let toitem = [
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

                    // GUIの表の構成
                    let dataview = &self.data_view;
                    dataview.insert_items_row(Some(indexnum as i32), &toitem);

                    if indexnum > listlimit {
                        self.statuslabel1
                            .set_text(format!("{}件までを表示しています。", listlimit).as_str());
                        break;
                    }
                }
                self.search_btn.set_enabled(true);
            }
            Err(_) => self
                .statuslabel1
                .set_text("C:\\Database\\partsetting.txtが見つかりません"),
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

    fn select_list_action(&self) {
        // リストを選択した際の動作を定義
        let listview = &self.data_view;
        // get row
        let selectrow = listview.selected_item();
        if let Some(row) = selectrow {
            // Some(row) => {
            // 選択行の注番をセット
            // let items = listview.item(row, 0, 20).unwrap().text;
            // self.order_input.set_text(&items);
            let linesize = 80;
            let model = listview.item(row, 4, linesize).unwrap().text;
            self.model_edit.set_text(&model);
            let maker = listview.item(row, 5, 20).unwrap().text;
            self.statuslabel1
                .set_text(format!("{}: {}", maker, model).as_str());
            // listview.select_item(row, true);
        }
        //     None => (),
        // }
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
                match webbrowser::open_browser(Browser::Default, &open_url) {
                    Ok(_) => self
                        .statuslabel1
                        .set_text(format!("{}をWEB検索", searchword).as_str()),
                    Err(e) => self.statuslabel1.set_text(format!("{}", e).as_str()),
                }
            }
            None => self
                .statuslabel1
                .set_text("検索エラー:アイテムを選択してください"),
        }
    }

    fn open_dialog(&self) {
        // Disable the button to stop the user from spawning multiple dialogs
        self.reload_btn.set_enabled(false);
        self.statuslabel1.set_text("Reload中:検索可能");
        let year = match self.year_input.selection_string() {
            Some(s) => s,
            None => "".to_string(),
        };
        *self.dialog_data.borrow_mut() =
            Some(ReloadDialog::popup(self.dialog_notice.sender(), year));
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn read_dialog_output(&self) {
        self.reload_btn.set_enabled(true);
        let data = self.dialog_data.borrow_mut().take();
        if let Some(handle) = data {
            // Some(handle) => {
            let getyear = self.year_input.selection();
            let dialog_result = handle.join().unwrap();
            self.statuslabel1.set_text(&dialog_result);
            self.set_year_select();
            self.year_input.set_selection(getyear);
        }
        //     None => {}
        // }
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
