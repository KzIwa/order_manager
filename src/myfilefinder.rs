use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

fn generate_fuzzy_pattern(base_pattern: &str) -> Option<(String, u32, u32)> {
    let parts: Vec<_> = base_pattern.split(&['-', '_'][..]).collect();
    if parts.len() < 3 {
        return None;
    }

    let to_digit = |st: &str| {
        st.chars()
            .take_while(|x| x.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
    };

    let head_index = if parts[0].to_lowercase() == *"nt" {
        1
    } else {
        0
    };
    let part1 = parts[head_index];
    let part2 = to_digit(parts[head_index + 1]);
    let part3 = to_digit(parts[head_index + 2]);
    match (part2, part3) {
        (Ok(p2), Ok(p3)) => Some((part1.to_string(), p2, p3)),
        _ => None,
    }
}

fn is_match_item(target: Option<&OsStr>, pattern_file: &Option<(String, u32, u32)>) -> bool {
    match target {
        Some(tg) => {
            let targetfile = generate_fuzzy_pattern(&tg.to_string_lossy());

            match (targetfile, pattern_file) {
                (Some(target), Some(pattern)) => target.1 == pattern.1 && target.2 == pattern.2,
                _ => false,
            }
        }
        None => false,
    }
}

pub fn files_search(
    basepath: PathBuf,
    base_pattern: &str,
) -> Option<impl Iterator<Item = PathBuf>> {
    let base_pattern = generate_fuzzy_pattern(base_pattern);
    let itempattern = match base_pattern.clone() {
        Some(pt) => {
            format!("./**/*{}*{}*{}*.{}", pt.0, pt.1, pt.2, "pdf")
        }
        None => "".to_string(),
    };
    let builder = globmatch::Builder::new(&itempattern)
        .case_sensitive(false)
        .build(basepath);

    match builder {
        Ok(bld) => Some(
            bld.into_iter()
                .filter(|x| x.is_ok())
                .flatten()
                .filter(move |f| is_match_item(f.file_name(), &base_pattern)),
        ),
        Err(_) => None,
    }
}

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
