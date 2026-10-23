//! オフラインの書き出しの試験の検体を `target\` の下（`temp-path-kit`）に組む部品（試験専用・
//! spec: areka-P0-offline-surface-dump 要件 12.6）。
//!
//! 組むのは `ghost/master/descript.txt` と `shell/<フォルダ>/`（descript.txt・surfaces.txt・
//! 1 色塗りの α 付き PNG）。3 つのテキストは `charset,UTF-8` を先頭に CRLF で書き、シェルの
//! descript.txt には `seriko.use_self_alpha,1` も書く（research.md §10.0 の測りと同じ）。
//! スコープごとに既定の着せ替えが分かれる検体は `crate::scope_bind_fixture_test_support` を使う。

use std::path::Path;

use temp_path_kit::TempPath;

/// 1 色で塗った絵 1 枚（乗算していない RGBA）。
pub(crate) struct Solid {
    pub(crate) name: &'static str,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: [u8; 4],
}

/// シェル 1 つ。`descript`・`surfaces` は `charset,UTF-8`（と透過の宣言）の後に書く行。
pub(crate) struct ShellSpec<'a> {
    pub(crate) folder: &'a str,
    pub(crate) descript: &'a [&'a str],
    pub(crate) surfaces: &'a [&'a str],
    pub(crate) images: &'a [Solid],
}

/// 検体のゴーストを組み、その根を返す（Drop で消える）。`ghost_descript` が `None` なら
/// ゴーストの descript.txt を置かない（フォルダだけ作る）。
pub(crate) fn build_ghost(
    label: &str,
    ghost_descript: Option<&[&str]>,
    shells: &[ShellSpec<'_>],
) -> TempPath {
    let root = TempPath::under_target(label);
    let ghost_master = root.path().join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("ゴーストのフォルダ作成");
    if let Some(lines) = ghost_descript {
        write_lines(&ghost_master.join("descript.txt"), &[], lines);
    }
    for shell in shells {
        let dir = root.path().join("shell").join(shell.folder);
        std::fs::create_dir_all(&dir).expect("シェルのフォルダ作成");
        write_lines(
            &dir.join("descript.txt"),
            &["seriko.use_self_alpha,1"],
            shell.descript,
        );
        write_lines(&dir.join("surfaces.txt"), &[], shell.surfaces);
        for image in shell.images {
            write_solid_png(&dir.join(image.name), image);
        }
    }
    root
}

/// runonce の面（土台の上に 100・101・102 を 100ms ずつ）。
pub(crate) const RUNONCE: u32 = 0;
/// always の面（右に 100・101 を 50ms ずつ）。
pub(crate) const ALWAYS: u32 = 1;
/// random,1 の面。
pub(crate) const RANDOM: u32 = 2;
/// periodic,1 の面。
pub(crate) const PERIODIC: u32 = 3;
/// 負の座標の element を持つ面（`CORNER_IMAGE` を -2,-2 に）。
pub(crate) const NEGATIVE: u32 = 4;
/// element の絵が欠けた面（土台は描ける）。
pub(crate) const MISSING_ELEMENT: u32 = 5;
/// 中身の無い `surface6 { }`（`surface6.png` も置かない）。
pub(crate) const EMPTY: u32 = 6;
/// bind のアニメーションの面（mustselect の head と multiple の acc）。
pub(crate) const BIND: u32 = 7;
/// 着せ替えとアニメーションの絵の面（赤・緑・青）。
pub(crate) const PARTS: [u32; 3] = [100, 101, 102];
/// 検体のシェルの面の番号（昇順）。
pub(crate) const VARIETY_SURFACES: [u32; 11] = [
    RUNONCE,
    ALWAYS,
    RANDOM,
    PERIODIC,
    NEGATIVE,
    MISSING_ELEMENT,
    EMPTY,
    BIND,
    PARTS[0],
    PARTS[1],
    PARTS[2],
];
/// 負の座標に置く 4×4 の半透明の絵。
pub(crate) const CORNER_IMAGE: &str = "corner.png";
/// `CORNER_IMAGE` の色（α 128）。
pub(crate) const CORNER_RGBA: [u8; 4] = [255, 255, 0, 128];

/// research.md §10 の形（runonce・always・random・periodic・負の座標・mustselect／multiple・
/// element の絵の欠け・中身の無い面）を 1 つに持つシェル。土台は 16×8 の灰。
pub(crate) fn variety_shell(folder: &str) -> ShellSpec<'_> {
    ShellSpec {
        folder,
        descript: VARIETY_DESCRIPT,
        surfaces: VARIETY_SURFACES_TXT,
        images: VARIETY_IMAGES,
    }
}

const VARIETY_DESCRIPT: &[&str] = &[
    "sakura.bindgroup20.name,head,hat",
    "sakura.bindgroup20.default,1",
    "sakura.bindgroup21.name,head,ribbon",
    "sakura.bindgroup21.default,0",
    "sakura.bindgroup30.name,acc,a",
    "sakura.bindgroup30.default,1",
    "sakura.bindgroup31.name,acc,b",
    "sakura.bindgroup31.default,1",
    "sakura.bindoption0.group,head,mustselect",
    "sakura.bindoption1.group,acc,multiple",
];

const VARIETY_SURFACES_TXT: &[&str] = &[
    "surface0",
    "{",
    "element0,overlay,base.png,0,0",
    "animation0.interval,runonce",
    "animation0.pattern0,overlay,100,100,0,0",
    "animation0.pattern1,overlay,101,100,0,0",
    "animation0.pattern2,overlay,102,100,0,0",
    "}",
    "surface1",
    "{",
    "element0,overlay,base.png,0,0",
    "animation0.interval,always",
    "animation0.pattern0,overlay,100,50,8,0",
    "animation0.pattern1,overlay,101,50,8,0",
    "}",
    "surface2",
    "{",
    "element0,overlay,base.png,0,0",
    "animation0.interval,random,1",
    "animation0.pattern0,overlay,100,100,8,0",
    "}",
    "surface3",
    "{",
    "element0,overlay,base.png,0,0",
    "animation0.interval,periodic,1",
    "animation0.pattern0,overlay,100,100,8,0",
    "}",
    "surface4",
    "{",
    "element0,overlay,base.png,0,0",
    "element1,overlay,corner.png,-2,-2",
    "}",
    // 欠けた絵のほかに土台が描けるので、記録は欠けの 1 件だけで合成は失敗しない。
    "surface5",
    "{",
    "element0,overlay,base.png,0,0",
    "element1,overlay,nofile.png,8,0",
    "}",
    // 中身の無いブレスだけ（読み込みは面を残し、合成は外形 0×0 で失敗する）。
    "surface6",
    "{",
    "}",
    "surface7",
    "{",
    "element0,overlay,base.png,0,0",
    "animation20.interval,bind",
    "animation20.pattern0,overlay,100,0,0,0",
    "animation21.interval,bind",
    "animation21.pattern0,overlay,101,0,0,0",
    "animation30.interval,bind",
    "animation30.pattern0,overlay,102,0,8,0",
    "animation31.interval,bind",
    "animation31.pattern0,overlay,100,0,8,0",
    "}",
    "surface100",
    "{",
    "element0,overlay,red.png,0,0",
    "}",
    "surface101",
    "{",
    "element0,overlay,green.png,0,0",
    "}",
    "surface102",
    "{",
    "element0,overlay,blue.png,0,0",
    "}",
];

const VARIETY_IMAGES: &[Solid] = &[
    Solid {
        name: "base.png",
        width: 16,
        height: 8,
        rgba: [128, 128, 128, 255],
    },
    Solid {
        name: "red.png",
        width: 8,
        height: 8,
        rgba: [255, 0, 0, 255],
    },
    Solid {
        name: "green.png",
        width: 8,
        height: 8,
        rgba: [0, 255, 0, 255],
    },
    Solid {
        name: "blue.png",
        width: 8,
        height: 8,
        rgba: [0, 0, 255, 255],
    },
    Solid {
        name: CORNER_IMAGE,
        width: 4,
        height: 4,
        rgba: CORNER_RGBA,
    },
];

/// `charset,UTF-8`・`head`・`lines` の順に CRLF で書く。
fn write_lines(path: &Path, head: &[&str], lines: &[&str]) {
    let mut text = String::from("charset,UTF-8\r\n");
    for line in head.iter().chain(lines) {
        text.push_str(line);
        text.push_str("\r\n");
    }
    std::fs::write(path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

/// 1 色で塗った 8 ビット RGBA の PNG を書く（α は乗算せずそのまま）。
fn write_solid_png(path: &Path, solid: &Solid) {
    image::RgbaImage::from_pixel(solid.width, solid.height, image::Rgba(solid.rgba))
        .save(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}
