#![windows_subsystem = "windows"]
extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

mod mydatabase;
mod myexcelread;
mod myfilefinder;
mod robocopy;

use glob::glob;
use mydatabase::{createtable, insertsql, order_readsql, PartsItem};
use myexcelread::readexcel;
use nwd::NwgUi;
use nwg::{EventData, ListViewColumnSortArrow, NativeUi};
use robocopy::diffcopy;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::{env, fs};
use webbrowser::Browser;

use std::cell::RefCell;
use url::{self, Url};

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

    #[nwg_control(text: "Reloadしますか?", position: (15, 45),)]
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
        format!("{dbfolder}\\parts{selectyear}.db3")
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
                        match diffcopy(&num, &targetfolder) {
                            Ok(_) => (),
                            Err(e) => return Some(e.to_string()),
                        };
                        match read_excel_files(num, dbtemp_path) {
                            Ok(getitems) => {
                                let statustext =
                                    format!("{getitems}件をデータベースに登録しました");
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
                            Err(e) => Some(e.to_string()),
                        }
                    }
                    Err(_) => Some("C:\\Database\\partsetting.txtが見つかりません".to_string()),
                }
            }
            Err(e) => Some(e.to_string()),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 年毎にデータベースを作成し年で選択する
    let dbfolder = "C:\\Database";
    if fs::read_dir(dbfolder).is_err() {
        fs::create_dir(dbfolder)?;
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
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        /*
        partsetting.txtに記載にitemname@と記載後、値を書くことで新しい設定値を定義できる。
        設定値をファイルから読み出す
        Setting名称をキーとして設定値をHashMapに格納。タプルの0番目に数値、1番目にVec<文字列>
        Open Setting file
        */
        let setting_file = "C:\\Database\\partsetting.txt";
        let f = fs::File::open(setting_file)?;

        let mut buffer = BufReader::new(f);
        let mut settings = String::with_capacity(1028);

        buffer.read_to_string(&mut settings)?;
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
    let i_str = i.to_string();
    let strlen = i_str.len();

    let do_insert_comma = |idx, val: char| {
        let pos = strlen - idx - 1;

        if pos != 0 && pos % 3 == 0 {
            val.to_string() + ","
        } else {
            val.to_string()
        }
    };

    i_str
        .chars()
        .enumerate()
        .map(|(idx, val)| do_insert_comma(idx, val))
        .collect()
}

fn excelvec_to_partsitem(ordername: &str, cellsdata: &[String], namesub: &str) -> PartsItem {
    // エクセルから読み取ったデータをPartsItem構造体に変換

    // dataから値を取り出すクロージャ
    let get_data = |x: usize| {
        // dataをキャプチャーしている
        if cellsdata.len() < x + 1 {
            "".to_string()
        } else {
            cellsdata[x].to_string()
        }
    };

    let ordernamesub = match namesub.split_once('.') {
        Some(sn) => sn.0,
        None => namesub,
    };

    let parts_item = PartsItem {
        order_no: match ordername.split_once('_') {
            Some(name) => match name.1.split_once('_') {
                Some(n) => ordernamesub.to_string() + n.1,
                None => ordernamesub.to_string() + name.1,
            },
            None => ordernamesub.to_string() + ordername,
        },

        unit_no: get_data(0).parse::<i32>().unwrap_or(0),

        parts_no: match get_data(2).split_once('-') {
            Some(pno) => pno.1.to_string(),
            None => get_data(2),
        },

        rev_mark: get_data(3),

        name: get_data(4),
        itemtype: get_data(1),
        model: get_data(5),
        maker: get_data(6),

        itemqty: get_data(7)
            .replace('計', "")
            .parse::<i32>()
            .unwrap_or_default(),
        remarks: get_data(8),
        condition: get_data(9),
        vender: get_data(10),
        order_date: get_data(13),
        delivery_date: get_data(14),
        delicondition: get_data(15),
        price: get_data(11).parse::<i32>().unwrap_or_default(),
    };
    parts_item
}

fn delete_db_file(datapath: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if fs::read(datapath).is_ok() {
        fs::remove_file(datapath)?;
    }

    Ok(())
}

fn get_dbyear() -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
            getnames.push(yearname);
        }
    }

    getnames.reverse();
    Ok(getnames)
}

fn read_excel_files(selectyear: i32, datapath: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    // エクセルファイルを検索してデータベースへ登録する

    let selectdir = format!("C:\\Database\\excel\\{selectyear}\\");
    createtable(datapath)?;
    let mut counter = 0;
    let currentpath = Path::new(selectdir.as_str());
    let mut getitems = Vec::new();

    match env::set_current_dir(currentpath) {
        Ok(_) => {
            for partype in ["購入", "加工"].into_iter() {
                let pattern = format!("./**/*{partype}*.xlsx");
                let targetfiles = glob(&pattern)?.filter_map(Result::ok);

                for excelname in targetfiles {
                    // エクセルファイルのファイルパス
                    if let Ok(datavec) = readexcel(&excelname) {
                        let mut inner_counter = 0;

                        datavec.iter().for_each(|dt| {
                            let filename = excelname.file_name().unwrap().to_str().unwrap();
                            let ordername = excelname.parent().unwrap().to_str().unwrap();
                            let item = excelvec_to_partsitem(ordername, dt, filename);

                            if !filename.contains("~$") && !getitems.contains(&item) {
                                getitems.push(item);
                                inner_counter += 1;
                            }
                        });

                        counter += inner_counter;
                    }
                }
            }
        }
        Err(e) => {
            println!("{e}");
        }
    };

    getitems.sort_by_key(|x| x.to_owned().unit_no);
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
    #[nwg_events(OnListViewClick:[DataViewApp::select_list_action],
        OnListViewColumnClick:[DataViewApp::column_click_sort(SELF,EVT_DATA)])]
    data_view: nwg::ListView,

    // google search
    #[nwg_control(text:"Google Search",size:(270,40))]
    #[nwg_layout_item(layout:mylayout,col: 10,row:0)]
    #[nwg_events(OnButtonClick:[DataViewApp::item_search])]
    google_btn: nwg::Button,

    // // 年度
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
    #[nwg_events( OnComboxBoxSelection: [DataViewApp::change_parts_type] )]
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
    #[nwg_control(text:"Clear",size:(270,40))]
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

    fn column_click_sort(&self, event: &EventData) {
        let (_, colnum) = event.on_list_view_item_index();
        let colnums = (0..15).filter(|x| *x != colnum);
        colnums.for_each(|col| self.data_view.set_column_sort_arrow(col, None));
        match self.data_view.column_sort_arrow(colnum) {
            Some(ListViewColumnSortArrow::Down) => {
                self.data_view
                    .set_column_sort_arrow(colnum, Some(ListViewColumnSortArrow::Up));
                let _ = self.read_database(colnum, true);
            }
            _ => {
                self.data_view
                    .set_column_sort_arrow(colnum, Some(ListViewColumnSortArrow::Down));
                let _ = self.read_database(colnum, false);
            }
        }
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
        format!("{dbfolder}\\parts{selectyear}.db3")
    }

    fn clear_all(&self) {
        self.clear_btn.set_enabled(false);
        self.order_input.set_text("");
        self.search_edit.set_text("");
        self.unit_input.set_text("");
        self.ordered_check
            .set_check_state(nwg::CheckBoxState::Unchecked);
    }

    fn konyu_data(&self) {
        self.data_view.update_column(4, "型式");
        self.data_view.update_column(5, "メーカ");
        self.google_btn.set_text("Gooogle Search");
    }

    fn kakou_data(&self) {
        self.data_view.update_column(4, "材質");
        self.data_view.update_column(5, "処理");
        self.google_btn.set_text("図面を開く")
    }

    fn change_parts_type(&self) {
        self.search_edit.set_text("");
        self.update_view();
    }

    fn update_view(&self) {
        if self.clear_btn.enabled() {
            (0..15).for_each(|col| self.data_view.set_column_sort_arrow(col, None));
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
        match self.read_database(0, false) {
            Ok(_) => (),
            Err(e) => self.statuslabel1.set_text(e.to_string().as_str()),
        }
    }

    fn read_database(
        &self,
        sort_col: usize,
        sort_rev: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.statuslabel1.set_text("");
        self.statuslabel2.set_text("");
        let dataview = &self.data_view;
        // 未手配チェック
        let orderedcheck = self.ordered_check.check_state() == nwg::CheckBoxState::Checked;
        // 合計金額
        let mut grossprice = 0;
        dataview.clear();

        let selectyear = self.year_input.selection_string();
        let yearnum = match selectyear {
            Some(year) => year.parse::<i32>()?,
            None => 0,
        };

        let selectedtype = if self.partstype.selection_string().is_some() {
            self.partstype.selection_string().unwrap()
        } else {
            panic!("selected type Err");
        };

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
        let mut contents = order_readsql(
            databasepath,
            &select_order,
            &selectedtype,
            &selectunit,
            &search_word,
            &orderedcheck,
        )?;

        match sort_col {
            0 => contents.sort_by_key(|k| k.order_no.clone()),
            1 => contents.sort_by_key(|k| k.unit_no),
            2 => contents.sort_by_key(|k| k.parts_no.clone()),
            3 => contents.sort_by_key(|k| k.name.clone()),
            4 => contents.sort_by_key(|k| k.model.clone()),
            5 => contents.sort_by_key(|k| k.maker.clone()),
            6 => contents.sort_by_key(|k| k.itemqty),
            7 => contents.sort_by_key(|k| k.remarks.clone()),
            8 => contents.sort_by_key(|k| k.condition.clone()),
            9 => contents.sort_by_key(|k| k.vender.clone()),
            10 => contents.sort_by_key(|k| k.order_date.clone()),
            11 => contents.sort_by_key(|k| k.delivery_date.clone()),
            12 => contents.sort_by_key(|k| k.delicondition.clone()),
            13 => contents.sort_by_key(|k| k.price),
            14 => contents.sort_by_key(|k| k.price * k.itemqty),
            _ => (),
        }
        // if self.sort_type.selection_string().unwrap() == "発注順" {
        //     contents.sort_by_key(|k| k.order_date.to_owned());
        //     contents.reverse();
        // }
        if sort_rev {
            contents.reverse()
        }

        self.statuslabel1
            .set_text(format!("{}→{}件の該当項目があります", search_word, contents.len()).as_str());
        let mut has_zero = false;

        let settingitem = SettingItem::new();

        let to_list_item = |items: &PartsItem, gpartprice: i32| {
            [
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
            ]
        };

        match settingitem {
            Ok(st) => {
                self.search_btn.set_enabled(false);
                let listlimit = st.maxdisplay_linenumber;
                let dataview = &self.data_view;
                // guiにアイテムをセット
                contents
                    .iter()
                    .take(listlimit)
                    .enumerate()
                    .for_each(|(indexnum, items)| {
                        let gpartprice = items.price * items.itemqty;
                        if gpartprice == 0
                            && items.name.trim() != "欠番"
                            && !items.remarks.contains("支給品")
                        {
                            has_zero = true
                        };
                        grossprice += gpartprice;

                        // GUIの表の構成
                        let toitem = to_list_item(items, gpartprice);
                        dataview.insert_items_row(Some(indexnum as i32), &toitem);
                    });

                if contents.len() > listlimit {
                    self.statuslabel1
                        .set_text(format!("{listlimit}件までを表示しています。").as_str());
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
                .set_text(format!("{maker}: {model}").as_str());
            self.statuslabel2.set_text("");
        }
    }
    fn item_search(&self) {
        let mode = self.partstype.selection_string();
        if mode == Some("購入".to_string()) {
            self.google_search();
        } else if mode == Some("加工".to_string()) {
            self.drawing_open();
        }
    }

    fn drawing_open(&self) {
        let listview = &self.data_view;
        let selected_year = &self.year_input.selection_string();
        let selected_row = listview.selected_item();
        let split_pattern = &['-', '_'][..];
        let target_order = selected_row
            .and_then(|row| listview.item(row, 0, 30))
            .map(|x| x.text.split(split_pattern).collect::<Vec<_>>()[0].to_string());

        let target_unit = selected_row
            .and_then(|row| listview.item(row, 1, 5))
            .map(|x| x.text.split(split_pattern).collect::<Vec<_>>()[0].to_string());
        let target_no = selected_row
            .and_then(|row| listview.item(row, 2, 5))
            .map(|x| x.text.split(split_pattern).collect::<Vec<_>>()[0].to_string());

        let basepattern = target_order.clone().and_then(|order| {
            target_unit.and_then(|unit| target_no.map(|no| order + "-" + &unit + "-" + &no))
        });
        let target_name = basepattern.clone().unwrap();

        if let Ok(st) = SettingItem::new() {
            let mut basefolder = PathBuf::from(st.searchfolder);
            if let Some(x) = selected_year.clone() {
                basefolder.push(&x)
            }

            // let items = find_drawings(basefolder, target_order, basepattern);
            match find_drawings(basefolder, target_order, basepattern) {
                Some(initems) => {
                    let result = initems.map(|it| opendir(&it, true)).collect::<Vec<_>>();
                    result.iter().for_each(|x| {
                        if let Err(e) = x {
                            self.statuslabel2.set_text(&format!("{:?}", e))
                        } else {
                            self.statuslabel2
                                .set_text(&format!("{:?} 図面を開きました", target_name))
                        }
                    });
                }
                None => self.statuslabel1.set_text("検索エラー"),
            }

            // if let Some(initems) = items {
            //     let _ = initems
            //         // .iter()
            //         .map(|it| opendir(&it, true))
            //         .collect::<Vec<_>>();
            //     self.statuslabel2
            //         .set_text(&format!("{:?}を開きました", target_name))
        }
    }

    fn google_search(&self) {
        let listview = &self.data_view;
        let selectrow = listview.selected_item();
        match selectrow {
            Some(row) => {
                let model = listview.item(row, 4, 30).unwrap().text;
                let maker = listview.item(row, 5, 20).unwrap().text;
                let mut searchword: String = String::new();

                let maker_url = match maker.to_lowercase().as_str() {
                    "ミスミ" => {
                        searchword = model.to_string();
                        let itemurl = format!(
                        "https://jp.misumi-ec.com/vona2/result/?Keyword={searchword}+&isReSearch=0"
                    );
                        Some(itemurl)
                    }
                    "sus" => {
                        searchword = model.to_string();
                        let itemurl =
                            format!("https://fa.sus.co.jp/service/list?word_box={searchword}");
                        Some(itemurl)
                    }
                    "キーエンス" => {
                        searchword = model.to_string();
                        let itemurl =
                            format!("https://www.keyence.co.jp/search/all/?q={searchword}");
                        Some(itemurl)
                    }
                    "iai" => {
                        let google_searchword = format!("{model} site:www.iai-robot.co.jp");
                        let itemurl =
                            format!("https://www.google.com/search?q={google_searchword}");
                        Some(itemurl)
                    }
                    "smc" => {
                        searchword = model.to_string();
                        let itemurl =format!("https://www.smcworld.com/gsearch/ja-jp/search?query={searchword}&lang=ja_JP");
                        Some(itemurl)
                    }
                    _ => None,
                };

                let google_searchword = format!("{model} {maker}");
                let google_url = format!("https://www.google.com/search?q={google_searchword}");

                if let Some(ourl) = maker_url {
                    // メーカーの検索URLが存在する場合
                    if is_valid_url(&ourl) {
                        self.open_mybrowser(&ourl, &searchword);
                    } else {
                        self.open_mybrowser(&google_url, &google_searchword);
                    }
                } else {
                    // メーカーの検索URLが存在しない場合はgoogle検索
                    self.open_mybrowser(&google_url, &google_searchword)
                };
            }
            None => self
                .statuslabel1
                .set_text("検索エラー:アイテムを選択してください"),
        }
    }

    fn open_mybrowser(&self, url: &str, searchword: &str) {
        match webbrowser::open_browser(Browser::Default, url) {
            Ok(_) => self
                .statuslabel1
                .set_text(format!("{searchword}をweb検索").as_str()),
            Err(e) => self.statuslabel1.set_text(format!("{e}").as_str()),
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
        let data: Option<thread::JoinHandle<String>> = self.dialog_data.borrow_mut().take();
        if let Some(handle) = data {
            let getyear = self.year_input.selection();
            let dialog_result = handle.join().unwrap();
            self.statuslabel1.set_text(&dialog_result);
            self.set_year_select();
            self.year_input.set_selection(getyear);
        }
    }
}
fn is_valid_url(url_str: &str) -> bool {
    match Url::parse(url_str) {
        Ok(url) => {
            // スキームとホストが存在することを確認
            url.scheme() != "" && url.host().is_some()
        }
        Err(_) => false,
    }
}

fn find_drawings(
    basefolder: PathBuf,
    target_order: Option<String>,
    basepattern: Option<String>,
    // ) -> Option<Vec<PathBuf>> {
) -> Option<impl Iterator<Item = PathBuf>> {
    let target_folder: Option<Vec<_>> =
        target_order.and_then(|order| myfilefinder::find_folder_path(basefolder, &order));

    target_folder
        .and_then(|dir| {
            basepattern.map(move |base_p| myfilefinder::files_search(dir[0].clone(), &base_p))
        })
        .flatten()
}

fn opendir(fpath: &PathBuf, is_open_file: bool) -> Result<(), Box<dyn std::error::Error>> {
    if is_open_file {
        // 対象ファイルのフォルダを既定のアプリで開く
        #[cfg(target_os = "macos")]
        Command::new("open").arg(fpath).spawn()?;

        #[cfg(target_os = "windows")]
        Command::new("explorer").arg(fpath).spawn()?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open").arg(fpath).spawn()?;
        Ok(())
    } else {
        // 対象ファイルのフォルダをファインダーで開く
        let filepath = match fpath.parent() {
            Some(f) => f,
            None => return Err("ファイルパスが不正です".into()),
        };
        #[cfg(target_os = "macos")]
        Command::new("open").arg(filepath).spawn()?;

        #[cfg(target_os = "windows")]
        Command::new("explorer").arg(filepath).spawn()?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open").arg(filepath).spawn()?;
        Ok(())
    }
}
#[test]
fn pretty_print_test() {
    assert_eq!(&pretty_print_int(0), "0");
    assert_eq!(&pretty_print_int(1), "1");
    assert_eq!(&pretty_print_int(200), "200");
    assert_eq!(&pretty_print_int(01200), "1,200");
    assert_eq!(&pretty_print_int(1000), "1,000");
    assert_eq!(&pretty_print_int(50000), "50,000");
    assert_eq!(&pretty_print_int(900000), "900,000");
    assert_eq!(&pretty_print_int(1900000), "1,900,000");
}
