use std::fs;
use std::process::Command;


pub fn diffcopy(year: &i32, targetfolder: &str) -> Result<(), Box<dyn std::error::Error>> {
    let target = format!("{targetfolder}{year}");
    println!("{}",target);
    let local = getfolder(&year.to_string())?;
    println!("{}",local);
    robocopy(&target, &local)?;
    Ok(())
}

fn robocopy(selectdir: &str, select_localdir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("start robocopy{select_localdir}");

    let result = Command::new("robocopy")
        .args([
            selectdir,
            select_localdir,
            "*.xlsx",
            "/S",
            "/PURGE",
            "/XO",
            "/xf",
            "~$*",
        ])
        .output()?;
    println!("{}", result.status);
    Ok(())
}

fn makefolder(dbfolder: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 年毎にデータベースを作成し年で選択する

    match fs::read_dir(dbfolder) {
        Ok(_) => {}
        Err(_) => {
            fs::create_dir(dbfolder)?;
        }
    }
    Ok(())
}

fn getfolder(year: &str) -> Result<String, Box<dyn std::error::Error>> {
   // ""のときはデータベースのフォルダを返し、適切なyearのときはエクセルファイルのフォルダを返す
   let basefolder = "C:\\Database";
   let dbfolder = if year.is_empty() {
       basefolder.to_string()
   } else {
       let inyear = year.parse::<u32>();
       match inyear {
           Ok(dbyear) => {
               if (2019..=2035).contains(&dbyear) {
                   let excelfolder = basefolder.to_string() + "\\excel";
                   makefolder(&excelfolder)?;
                   let folder =
                       basefolder.to_string() + "\\excel\\" + format!("{dbyear}").as_str();
                   folder
               } else {
                   basefolder.to_string()
               }
           }
           Err(_) => basefolder.to_string(),
       }
   };
   makefolder(&dbfolder)?;
   Ok(dbfolder)
}
