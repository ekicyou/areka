//! `areka.exe` の書き出しの試験の共有の部品（検体の組み立てと exe の回し方）。
//!
//! areka は bin だけのクレートなので `#[cfg(test)]` の `offline_dump_test_support` は使えず、
//! 検体はここで `temp-path-kit`・`image`・`std` だけで組む（形は同じ: 3 つのテキストは
//! `charset,UTF-8` を先頭に CRLF、シェルの descript.txt には `seriko.use_self_alpha,1`）。

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use temp_path_kit::TempPath;

/// 1 色で塗った絵 1 枚（乗算していない RGBA）。
pub struct Solid {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub rgba: [u8; 4],
}

/// 灰の土台 16×8。
pub const BASE: Solid = Solid {
    name: "base.png",
    width: 16,
    height: 8,
    rgba: [128, 128, 128, 255],
};

/// 赤の 8×8。
pub const RED: Solid = Solid {
    name: "red.png",
    width: 8,
    height: 8,
    rgba: [255, 0, 0, 255],
};

/// `ghost/master/descript.txt` と `shell/master/`（descript.txt・surfaces.txt・絵）を組み、
/// 根を返す（Drop で消える）。
pub fn build_ghost(
    label: &str,
    ghost_descript: &[&str],
    surfaces: &[&str],
    images: &[Solid],
) -> TempPath {
    let root = TempPath::under_target(label);
    write_ghost(root.path(), ghost_descript, surfaces, images);
    root
}

/// [`build_ghost`] と同じ形のゴーストを `at` のフォルダに組む。
pub fn write_ghost(at: &Path, ghost_descript: &[&str], surfaces: &[&str], images: &[Solid]) {
    let ghost_master = at.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("ゴーストのフォルダ作成");
    write_lines(&ghost_master.join("descript.txt"), &[], ghost_descript);
    let shell = at.join("shell").join("master");
    std::fs::create_dir_all(&shell).expect("シェルのフォルダ作成");
    write_lines(
        &shell.join("descript.txt"),
        &["seriko.use_self_alpha,1"],
        &["name,master"],
    );
    write_lines(&shell.join("surfaces.txt"), &[], surfaces);
    for image in images {
        image::RgbaImage::from_pixel(image.width, image.height, image::Rgba(image.rgba))
            .save(shell.join(image.name))
            .unwrap_or_else(|e| panic!("{}: {e}", image.name));
    }
}

/// CSV の 1 行（囲みを外した 4 列）。
pub struct Row {
    pub time: String,
    pub ghost: String,
    pub level: String,
    pub body: String,
}

/// `"時刻","ゴースト名","レベル","内容"` の 1 行を読む。列の中の `""` は `"` に戻す。
pub fn parse_row(line: &str) -> Row {
    let inner = line
        .strip_prefix('"')
        .and_then(|l| l.strip_suffix('"'))
        .unwrap_or_else(|| panic!("各列を \" で囲む: {line}"));
    let cols: Vec<String> = inner
        .splitn(4, "\",\"")
        .map(|c| c.replace("\"\"", "\""))
        .collect();
    assert_eq!(cols.len(), 4, "4 列: {line}");
    Row {
        time: cols[0].clone(),
        ghost: cols[1].clone(),
        level: cols[2].clone(),
        body: cols[3].clone(),
    }
}

/// BOM 付きの CSV を読んで行に分ける（無ければ panic）。
pub fn csv_rows(path: &Path) -> Vec<Row> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.strip_prefix('\u{FEFF}')
        .expect("BOM 付き")
        .split_terminator("\r\n")
        .map(parse_row)
        .collect()
}

/// `charset,UTF-8`・`head`・`lines` の順に CRLF で書く。
fn write_lines(path: &Path, head: &[&str], lines: &[&str]) {
    let mut text = String::from("charset,UTF-8\r\n");
    for line in head.iter().chain(lines) {
        text.push_str(line);
        text.push_str("\r\n");
    }
    std::fs::write(path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

/// 建った `areka.exe` を `cwd` で `args` を付けて回し、終わるまで待って終了コードと標準エラーを返す。
/// 告知のモーダルで止まらないよう `AREKA_NO_ALERT=1` を渡す。
pub fn run_areka(cwd: &Path, args: &[&str]) -> (i32, String) {
    run_areka_with_env(cwd, args, &[])
}

/// [`run_areka`] に環境変数 `envs` を足して回す。
pub fn run_areka_with_env(cwd: &Path, args: &[&str], envs: &[(&str, &OsStr)]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_areka"))
        .args(args)
        .envs(envs.iter().copied())
        .current_dir(cwd)
        .env("AREKA_NO_ALERT", "1")
        .env("NO_COLOR", "1")
        .output()
        .expect("areka.exe を起動できる");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out
        .status
        .code()
        .unwrap_or_else(|| panic!("終了コードが無い: {stderr}"));
    (code, stderr)
}

/// PNG の幅と高さ。
pub fn png_size(path: &Path) -> (u32, u32) {
    image::image_dimensions(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// フォルダの直下のファイル名（昇順）。
pub fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| {
            e.expect("一覧の 1 件")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}
