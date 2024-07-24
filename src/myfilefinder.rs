use std::fs;
use std::path::PathBuf;
// use walkdir::DirEntry;
// use walkdir::WalkDir;

/// 指定されたベースパターンを元に、揺らぎのある正規表現を生成する関数
fn generate_fuzzy_pattern(base_pattern: &str) -> Option<(String, u32, u32)> {
    let parts: Vec<&str> = base_pattern.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let to_digit = |st: &str| {
        st.chars()
            .take_while(|x| x.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
    };

    let part1 = parts[0];
    let part2 = to_digit(parts[1]);
    let part3 = to_digit(parts[2]);
    match (part2, part3) {
        (Ok(p2), Ok(p3)) => Some((part1.to_string(), p2, p3)),
        _ => None,
    }
}

fn is_match_item(target: &str, pattern_file: &Option<(String, u32, u32)>) -> bool {
    let targetfile = generate_fuzzy_pattern(target);
    // let pattern_file = generate_fuzzy_pattern(base_pattern);

    match (targetfile, pattern_file) {
        (Some(target), Some(pattern)) => {
            target.0.contains(&pattern.0) && target.1 == pattern.1 && target.2 == pattern.2
        }
        _ => false,
    }
}

pub fn files_search(
    basepath: PathBuf,
    base_pattern: &str,
) -> Result<impl Iterator<Item = PathBuf>, Box<dyn std::error::Error>> {
    let pattern = format!("./**/*.{}", "pdf");

    let base_pattern = generate_fuzzy_pattern(base_pattern);

    let builder = globmatch::Builder::new(&pattern)
        .case_sensitive(false)
        .build(basepath)?;

    let prebuilder = builder
        .into_iter()
        .filter(|x| x.is_ok())
        .flatten()
        .filter(move |f| is_match_item(f.file_name().unwrap().to_str().unwrap(), &base_pattern));

    Ok(prebuilder)
}

// /// 指定されたパターンを含むファイルを検索し、最初に見つかったファイルを開く関数
// pub fn find_files<P: AsRef<Path>>(dir: P, base_pattern: &str) -> impl Iterator<Item = PathBuf> {
//     // ターゲットのファイル名と　base_patternのファイル名を比較検索
//     let base_pattern = generate_fuzzy_pattern(base_pattern);

//     let is_entry_match = move |entry: &DirEntry| {
//         let path = entry.path();

//         if path.is_file() {
//             // ターゲットのファイル名を抽出
//             let filename = path.file_name().unwrap().to_str().unwrap();
//             is_match_item(filename, &base_pattern)
//         } else {
//             false
//         }
//     };

//     // ディレクトリ内のファイルを再帰的に列挙
//     let result = WalkDir::new(dir)
//         .into_iter()
//         .filter_map(|x| x.ok())
//         .filter(is_entry_match)
//         .map(|p| p.path().to_owned())
//         .filter(|f| f.extension().unwrap().to_string_lossy().to_lowercase() == "pdf");
//     result
// }

pub fn find_folder_path(root: PathBuf, target_str: &str) -> Option<Vec<PathBuf>> {
    // rootに1層目にあるtarget_strを含むフォルダを探す
    let result = match fs::read_dir(root) {
        Ok(entries) => Some(
            entries
                .flatten()
                .filter(|x| match x.file_type() {
                    Ok(ftype) => ftype.is_dir(),
                    Err(_) => false,
                })
                .filter(|x| x.path().to_str().unwrap_or_default().contains(target_str))
                .map(|x| x.path())
                .collect::<Vec<_>>(),
        ),
        Err(_) => None,
    };

    result
}

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // 検索するディレクトリ
//     let dir = "//LS220DB3C9/share/発注管理/2024/15_NT-AODB01_（株）アライドマテリアル（自動矯正プレスNo.11_大阪安宅機械）/発注済/"; // カレントディレクトリを指定
//     let dir = PathBuf::from_str(dir)?;

//     // 検索するベースパターン
//     let base_pattern = "AODB01-020-1";
//     println!("{:?}", find_files(dir, base_pattern).collect::<Vec<_>>());

//     // find_and_open_file(dir, base_pattern)?;
//     Ok(())
// }
