//! スコープ 0・1・2 で既定の着せ替えと名前が分かれる検体（spec: areka-P0-offline-surface-dump
//! 要件 14.4・research.md §10.3 の `g_scope` と同じ形）。画面の試験と書き出しの試験が共有する
//! ただ 1 つの組み立て（写さない）。`offline_dump` は `emo2_boot` の部品を使わないので、
//! クレートの根に置く。
//!
//! 土台は 24×8 の灰。面 0 に bind のアニメーションが 3 本（10＝左の赤・11＝中の緑・12＝右の青）。
//! 既定はスコープ 0（`sakura.*`）が 10・1（`kero.*`）が 11・2（`char2.*`）が 12 で、スコープ 3
//! （`char3.*` なし）は何も着ない。名前のパーツは 3 つのスコープとも 10＝赤・11＝緑・12＝青で、
//! カテゴリだけがスコープごとに違う（ほかのスコープの名前で当たらないことを見分けるため）。

use std::path::Path;

use temp_path_kit::TempPath;

/// 左の赤のアニメーション（スコープ 0 の既定）。
pub(crate) const RED: u32 = 10;
/// 中の緑のアニメーション（スコープ 1 の既定）。
pub(crate) const GREEN: u32 = 11;
/// 右の青のアニメーション（スコープ 2 の既定）。
pub(crate) const BLUE: u32 = 12;
/// `sakura.*` の名前のカテゴリ。
pub(crate) const SAKURA_CATEGORY: &str = "服";
/// `kero.*` の名前のカテゴリ。
pub(crate) const KERO_CATEGORY: &str = "帽子";
/// `char2.*` の名前のカテゴリ。
pub(crate) const CHAR2_CATEGORY: &str = "髪";

/// 検体のゴーストを `target\` の下（`temp-path-kit`）に組み、その根を返す（Drop で消える）。
///
/// 中身は `ghost/master/descript.txt` と `shell/master/`（descript.txt・surfaces.txt・PNG 4 枚）。
/// バルーンは置かない。
pub(crate) fn scope_bind_ghost() -> TempPath {
    let root = TempPath::new("areka-scope-bind-ghost");
    let ghost_master = root.path().join("ghost").join("master");
    let shell_master = root.path().join("shell").join("master");
    for dir in [&ghost_master, &shell_master] {
        std::fs::create_dir_all(dir).expect("検体ディレクトリ作成");
    }
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\r\nname,g_scope\r\n",
    )
    .expect("ghost descript");

    let mut descript = String::from("charset,UTF-8\r\nseriko.use_self_alpha,1\r\n");
    for (prefix, category, default_on) in [
        ("sakura", SAKURA_CATEGORY, RED),
        ("kero", KERO_CATEGORY, GREEN),
        ("char2", CHAR2_CATEGORY, BLUE),
    ] {
        for (id, part) in [(RED, "赤"), (GREEN, "緑"), (BLUE, "青")] {
            let on = u8::from(id == default_on);
            descript.push_str(&format!(
                "{prefix}.bindgroup{id}.name,{category},{part}\r\n{prefix}.bindgroup{id}.default,{on}\r\n"
            ));
        }
    }
    std::fs::write(shell_master.join("descript.txt"), descript).expect("shell descript");

    // 着せ替えの絵は面 100〜102 に置き、bind のアニメーションが土台の左・中・右へ重ねる。
    let surfaces = "charset,UTF-8\r\n\
        surface0\r\n{\r\nelement0,overlay,base.png,0,0\r\n\
        animation10.interval,bind\r\nanimation10.pattern0,overlay,100,0,0,0\r\n\
        animation11.interval,bind\r\nanimation11.pattern0,overlay,101,0,8,0\r\n\
        animation12.interval,bind\r\nanimation12.pattern0,overlay,102,0,16,0\r\n}\r\n\
        surface100\r\n{\r\nelement0,overlay,red.png,0,0\r\n}\r\n\
        surface101\r\n{\r\nelement0,overlay,green.png,0,0\r\n}\r\n\
        surface102\r\n{\r\nelement0,overlay,blue.png,0,0\r\n}\r\n";
    std::fs::write(shell_master.join("surfaces.txt"), surfaces).expect("surfaces.txt");
    for (name, w, rgba) in [
        ("base.png", 24, [128, 128, 128, 255]),
        ("red.png", 8, [255, 0, 0, 255]),
        ("green.png", 8, [0, 255, 0, 255]),
        ("blue.png", 8, [0, 0, 255, 255]),
    ] {
        write_solid_png(&shell_master.join(name), w, 8, rgba);
    }
    root
}

/// 1 色で塗った 8 ビット RGBA の PNG を書く（署名 → IHDR → IDAT → IEND）。
// shortcut: MCP の PNG の組み立ては共有の置き場へ移るまで呼べない（6.1）、移ったらそちらを呼ぶ。
fn write_solid_png(path: &Path, width: u32, height: u32, rgba: [u8; 4]) {
    // 各行はフィルタ 0 の 1 バイトの後に同じ色を幅の数だけ。
    let row = [&[0][..], &rgba.repeat(width as usize)].concat();
    let raw = row.repeat(height as usize);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let idat = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in [
        (b"IHDR", &ihdr[..]),
        (b"IDAT", &idat[..]),
        (b"IEND", &[][..]),
    ] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = areka_nar::crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    std::fs::write(path, out).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}
