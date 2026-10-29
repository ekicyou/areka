//! テーマ間で共有する検体の取得と、要件の数値から手で書いた期待値の表。
//!
//! 出典 spec: `default-balloon-selfmade`（要件 **12.1**・**2.2**・**3.1**・**3.2**・**5.4**〜**5.7**・
//! **5.9**・**6.2**・**7.1**・**10.3**）。
//!
//! 期待値は要件と設計「ファイルの表」から**手で**書き、絵の道具のコードを参照しない
//! （道具と独立に要件と突き合わせるため・設計「Testing Strategy」）。実測に合わせて緩めない。

use std::path::PathBuf;
use std::sync::LazyLock;

use sample_ghost_kit::SampleRoot;

/// 検体の名前（登記表の行・`install.txt` の `directory`・`descript.txt` の `id`）。
pub(crate) const BALLOON_NAME: &str = "areka-balloon";

/// 検体。プロセス寿命で保持する（`SampleRoot` は破棄で複製の木を消すため）。
static BALLOON: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire(BALLOON_NAME).expect("areka-balloon は登記済みの検体（要件 10.3）")
});

/// バルーンのフォルダ `<根>/balloon/areka-balloon/`。
pub(crate) fn balloon_root() -> PathBuf {
    BALLOON.folder().to_path_buf()
}

// ── 書庫の中身（設計「ファイルの表」・23 ファイル）──────────────────────────────

/// 書庫の全 23 ファイル（名前の昇順）。`.pna`・GIF・2 倍の絵・`online1` 以降は無い
/// （要件 2.4・2.6・3.2・3.4）。
pub(crate) const EXPECTED_FILE_NAMES: [&str; 23] = [
    "arrow0.png",
    "arrow1.png",
    "balloonc0.png",
    "balloonc1.png",
    "balloonc2.png",
    "balloonc3.png",
    "balloonk0.png",
    "balloonk0s.txt",
    "balloonk1.png",
    "balloonk1s.txt",
    "balloons0.png",
    "balloons0s.txt",
    "balloons1.png",
    "balloons1s.txt",
    "balloons2.png",
    "balloons2s.txt",
    "balloons3.png",
    "balloons3s.txt",
    "descript.txt",
    "install.txt",
    "online0.webp",
    "readme.txt",
    "sstp.png",
];

/// PNG の絵 13 枚の寸法（要件 2.1・2.2・3.1・4.1）: 本体側の面 4・相方側の面 2・入力の箱 4・
/// 矢印 2・SSTP 1。
pub(crate) const EXPECTED_PNG_SIZES: [(&str, u32, u32); 13] = [
    ("balloons0.png", 326, 169),
    ("balloons1.png", 326, 169),
    ("balloons2.png", 326, 348),
    ("balloons3.png", 326, 348),
    ("balloonk0.png", 326, 96),
    ("balloonk1.png", 326, 96),
    ("balloonc0.png", 400, 63),
    ("balloonc1.png", 400, 63),
    ("balloonc2.png", 400, 63),
    ("balloonc3.png", 400, 63),
    ("arrow0.png", 11, 10),
    ("arrow1.png", 11, 10),
    ("sstp.png", 7, 9),
];

/// オンラインの印 `online0.webp` の寸法（要件 3.2）。
pub(crate) const EXPECTED_ONLINE_SIZE: (&str, u32, u32) = ("online0.webp", 77, 36);

// ── 文のファイルの鍵と値（要件 5.4〜5.7・5.9・6.2・7.1）─────────────────────────
//
// 並びは手本（research.md 10.2）の順で書くが、比べるのは鍵の集合ごとの一致である。
// `windowpos.*`・`onlinemarker.filename`・`onlinemarker.interval`・`validrect.right,0` は
// 無いこと自体が期待値（要件 3.5・5.7・6.3）。

/// `descript.txt`（手本の全体の値・替えるのは `charset`・`name`・`id`・`craftman`・
/// `use_self_alpha`・`validrect.right` だけ）。
#[expect(dead_code, reason = "定義のテーマ（タスク 3.2）が引く")]
pub(crate) const EXPECTED_DESCRIPT: &[(&str, &str)] = &[
    ("charset", "UTF-8"),
    ("type", "balloon"),
    ("name", "areka バルーン"),
    ("id", "areka-balloon"),
    ("craftman", "ekicyou"),
    ("use_self_alpha", "1"),
    ("origin.x", "14"),
    ("origin.y", "11"),
    ("validrect.left", "0"),
    ("validrect.top", "0"),
    ("validrect.right", "326"),
    ("validrect.bottom", "-20"),
    ("wordwrappoint.x", "-20"),
    ("wordwrappoint.y", "0"),
    ("font.name", "ＭＳ ゴシック"),
    ("font.height", "12"),
    ("font.color.r", "0"),
    ("font.color.g", "0"),
    ("font.color.b", "0"),
    ("cursor.blendmethod", "mergepennot"),
    ("cursor.style", "square"),
    ("cursor.brush.color.r", "22"),
    ("cursor.brush.color.g", "143"),
    ("cursor.brush.color.b", "231"),
    ("cursor.pen.color.r", "8"),
    ("cursor.pen.color.g", "8"),
    ("cursor.pen.color.b", "194"),
    ("cursor.font.color.r", "201"),
    ("cursor.font.color.g", "255"),
    ("cursor.font.color.b", "255"),
    ("arrow0.x", "303"),
    ("arrow0.y", "10"),
    ("arrow1.x", "303"),
    ("arrow1.y", "-20"),
    ("onlinemarker.x", "4"),
    ("onlinemarker.y", "-39"),
    ("sstpmarker.x", "10"),
    ("sstpmarker.y", "-15"),
    ("sstpmessage.x", "20"),
    ("sstpmessage.y", "-17"),
    ("sstpmessage.font.name", "ＭＳ Ｐゴシック"),
    ("sstpmessage.font.height", "12"),
    ("sstpmessage.font.color.r", "56"),
    ("sstpmessage.font.color.g", "163"),
    ("sstpmessage.font.color.b", "163"),
    ("number.font.name", "Times New Roman"),
    ("number.font.height", "14"),
    ("number.font.color.r", "32"),
    ("number.font.color.g", "76"),
    ("number.font.color.b", "120"),
    ("number.xr", "-25"),
    ("number.y", "-18"),
    ("communicatebox.x", "8"),
    ("communicatebox.y", "21"),
    ("communicatebox.width", "385"),
    ("communicatebox.height", "20"),
];

/// 面別の設定 6 つ（要件 5.5・`windowpos.*` を写さない＝要件 5.7・`wordwrappoint.y,0` は手本どおり）。
#[expect(dead_code, reason = "定義のテーマ（タスク 3.2）が引く")]
pub(crate) const EXPECTED_FACE_SETTINGS: [(&str, &[(&str, &str)]); 6] = [
    (
        "balloons0s.txt",
        &[
            ("charset", "UTF-8"),
            ("origin.x", "14"),
            ("origin.y", "11"),
            ("wordwrappoint.x", "-30"),
            ("wordwrappoint.y", "0"),
            ("onlinemarker.x", "3"),
            ("sstpmarker.x", "10"),
            ("sstpmessage.x", "20"),
        ],
    ),
    (
        "balloons1s.txt",
        &[
            ("charset", "UTF-8"),
            ("origin.x", "23"),
            ("origin.y", "11"),
            ("wordwrappoint.x", "-20"),
            ("wordwrappoint.y", "0"),
            ("onlinemarker.x", "12"),
            ("sstpmarker.x", "19"),
            ("sstpmessage.x", "29"),
        ],
    ),
    (
        "balloons2s.txt",
        &[
            ("charset", "UTF-8"),
            ("origin.x", "14"),
            ("origin.y", "14"),
            ("wordwrappoint.x", "-30"),
            ("wordwrappoint.y", "0"),
        ],
    ),
    (
        "balloons3s.txt",
        &[
            ("charset", "UTF-8"),
            ("origin.x", "23"),
            ("origin.y", "14"),
            ("wordwrappoint.x", "-20"),
            ("wordwrappoint.y", "0"),
        ],
    ),
    (
        "balloonk0s.txt",
        &[
            ("charset", "UTF-8"),
            ("origin.x", "14"),
            ("origin.y", "14"),
            ("validrect.bottom", "-14"),
            ("wordwrappoint.x", "-30"),
            ("wordwrappoint.y", "0"),
        ],
    ),
    (
        "balloonk1s.txt",
        &[
            ("charset", "UTF-8"),
            ("origin.x", "24"),
            ("origin.y", "14"),
            ("validrect.bottom", "-14"),
            ("wordwrappoint.x", "-20"),
            ("wordwrappoint.y", "0"),
        ],
    ),
];

/// `install.txt`（要件 7.1・設計「ファイルの表」）。
#[expect(dead_code, reason = "定義のテーマ（タスク 3.2）が引く")]
pub(crate) const EXPECTED_INSTALL: &[(&str, &str)] = &[
    ("charset", "UTF-8"),
    ("type", "balloon"),
    ("name", "areka バルーン"),
    ("directory", "areka-balloon"),
];
