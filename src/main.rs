// #![windows_subsystem = "windows"]
extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;
// mod mydatabase;
use nwd::NwgUi;
use nwg::NativeUi;
use std::path::Path;
use anyhow::{Result, Error};


fn main()->Result<()>{
    // let savepath = Path::new ("\\\\LS220DB3C9\\share\\共有\\test.db4");
    // // データベースがないときは新規作成
    // match mydatabase::createsql(savepath){
    //     Ok(()) => {
    //     },
    //     _ => println!("既にデータベースがあります")
    // };
    // let partsitem = mydatabase::PartsItem::new(05, 20, "test", "加工", "ミスミ", 1, "test", "no", "ミミ", "22/4/13", "22/5/30", "no", 8500);
    // let insertdata=mydatabase::insertsql(savepath, partsitem)?;
    guiapp();
    Ok(())

}


fn guiapp(){
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let mut window = Default::default();
    let _app = DataViewApp::build_ui(window).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}

#[derive(Default, NwgUi)]
pub struct DataViewApp {
    #[nwg_control(size:(1500,500), position: (300, 300), title: "部品管理",flags:"WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [DataViewApp::exit], OnInit: [DataViewApp::load_data])]
    window: nwg::Window,
    
    #[nwg_resource(family: "Meiryo", size: 19)]
    appfont: nwg::Font,

    // レイアウト管理
    #[nwg_layout(parent: window,spacing:1,min_size:[100,60])]
    mylayout: nwg::GridLayout,

    #[nwg_control(item_count: 10, size: (500, 350), list_style: nwg::ListViewStyle::Detailed, focus: true,
        ex_flags: nwg::ListViewExFlags::GRID | nwg::ListViewExFlags::FULL_ROW_SELECT, 
    )]
    
    #[nwg_layout_item(layout: mylayout, col: 0, col_span: 4, row: 0, row_span: 2)]
    #[nwg_events(OnListViewClick:[DataViewApp::getlistitem],OnKeyRelease:[DataViewApp::getlistitem])]
    data_view: nwg::ListView,

    #[nwg_control(collection: vec!["購入", "加工"], selected_index: Some(0), font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 4, row: 0)]
    #[nwg_events( OnComboxBoxSelection: [DataViewApp::update_view] )]
    view_style: nwg::ComboBox<&'static str>,

    // 購入加工の状態保持無限ループを防ぐ 状態フラグ["購入", "加工"]
    #[nwg_control(text: "")]
    typelabel: nwg::Label,

    #[nwg_control(text: "",font: Some(&data.appfont))]
    #[nwg_layout_item(layout: mylayout, col: 4, row: 1)]
    name_edit: nwg::TextInput,


} 

impl DataViewApp {
    fn load_data(&self){
        let dataview = &self.data_view;
        self.typelabel.set_visible(false);
             // リストビューの初期セッティング
        dataview.insert_column("Unit no");
        dataview.insert_column("Parts no");
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
    }

    fn parse_data(&self){

    }
    fn konyu_data(&self){
        self.typelabel.set_text("購入");
        self.data_view.update_column(3, "型式");
        self.data_view.update_column(4, "メーカ");

    }
    fn kakou_data(&self){
        self.typelabel.set_text("加工");
        self.data_view.update_column(3, "材質");
        self.data_view.update_column(4, "処理");
    }
    fn update_view(&self){
        let value = self.view_style.selection_string();
        match value.as_ref().map(|x| x as &str){
            Some("購入") => {
                if self.typelabel.text()!="購入"{
                self.konyu_data();//状態ラベルを切換
                self.read_database();
                }
            }
            Some("加工") => {
                if self.typelabel.text()!="加工"{
                self.kakou_data();//状態ラベルを切換
            }}
            None | Some(_) => ()
        }
    }

    fn read_database(&self){
        let dataview = &self.data_view;
        dataview.clear();

        let testdata=vec![("1","2temp"),("2","test"),("3","mdo")];
        for (indexnum,items) in testdata.iter().enumerate(){
            let toitem=vec![items.0.to_string(),items.1.to_string()];
            self.set_item(indexnum as i32, &toitem);
        }
        dataview.select_item(0, true);
    }



    fn addread_database(&self){
        let testdata=vec![("1","2temp"),("2","test"),("3","mdo")];
        for (indexnum,items) in testdata.iter().enumerate(){
            let toitem=vec![items.0.to_string(),items.1.to_string()];
            self.set_item(indexnum as i32, &toitem);
        }
    }

    fn set_item<T:ToString>(&self,indexnum:i32,listdata:&Vec<T>){
        let itemdata:Vec<String>=listdata.iter().map(|x|x.to_string()).collect();
        let dataview = &self.data_view;

        for (colnum,itemtext) in itemdata.iter().enumerate(){
            let listdata =nwg::InsertListViewItem{index:Some(indexnum),
                column_index:colnum as i32,text:Some(itemtext.into()),image:None
            };
          dataview.insert_item(listdata)
        }
        self.update_view()
    }

    fn getlistitem(&self){
        let listview = &self.data_view;
        // get row
        let selectnum = listview.selected_item();
        match selectnum {
            Some(num)=>{
                let items=listview.item(num, 1, 20).unwrap().text;
                self.name_edit.set_text(&items);
                listview.select_item(num, true);},
            None=>()
        }
 
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}