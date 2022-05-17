
use anyhow::Result;
use std::fs;
use std::process::Command;
pub fn diffcopy(year:&i32) -> Result<()> {
    let target = format!("\\\\LS220DB3C9\\share\\発注管理\\{}",year);
    let local = getfolder(&year.to_string())?;
    robocopy(&target, &local)?;
    Ok(())
}

fn robocopy(selectdir: &str, select_localdir: &str) -> Result<()> {
    println!("start robocopy{}", select_localdir);

    let result = Command::new("robocopy").args(&[selectdir,select_localdir,"*.xlsx","/S","/XO"]).output()?;
    println!("{}",result.status);
    Ok(())
}

fn makefolder(dbfolder: &str) -> Result<()> {
    // 年毎にデータベースを作成し年で選択する
    println!("{}",dbfolder);
    match fs::read_dir(dbfolder) {
        Ok(_) => {}
        Err(_) => {
            fs::create_dir(dbfolder)?;
        }
    }
    Ok(())
}
fn getfolder(year: &str) -> Result<String> {
    // ""のときはデータベースのフォルダを返し、適切なyearのときはエクセルファイルのフォルダを返す
    let basefolder = "C:\\Database";
    let dbfolder = if year == "" {
        basefolder.to_string()
    } else {
        let inyear = year.parse::<u32>();
        match inyear {
            Ok(dbyear) => {
                if dbyear >= 2019 && dbyear <= 2035 {
                    let excelfolder=basefolder.to_string()+"\\excel";
                    makefolder(&excelfolder)?;
                    let folder =
                        basefolder.to_string() + "\\excel\\" + format!("{}", dbyear).as_str();
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
