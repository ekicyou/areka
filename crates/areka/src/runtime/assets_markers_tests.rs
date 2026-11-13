//! 起動の資産が面ごとの定義の束と箱の矢印の絵を運ぶテスト（balloon-scroll-arrows 要件 1.7・7.3）。
//!
//! 一時フォルダ（ワークツリーの `target\` の下）に本物の PNG を書き、本物の WIC で読む
//! （起動と背景のスレッドの資産づくりと同じ道）。

use temp_path_kit::TempPath;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::*;

fn png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]))
        .save(path)
        .expect("PNG を書けるはず");
}

fn text(path: &Path, body: &str) {
    std::fs::write(path, body).expect("設定ファイルを書けるはず");
}

/// 箱 `a` は自分の名前（`myarrow`）の絵、箱 `b` は既定の名前の絵をシェルのフォルダから引く。
/// 既定の名前の順はスコープごと（スコープ 0 は `arrow0`、スコープ 1 は `arrowk0` を先に採る）。バルーンは面 0 と、自分の設定ファイルで `arrow.filename` を替えた面 1 を持つ。
#[test]
fn boot_assets_carry_face_definitions_and_box_arrow_images() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let root = TempPath::under_target("assets-markers");
    let ghost = root.path().join("ghost").join("master");
    let shell = root.path().join("shell").join("master");
    let balloon = root.path().join("balloon");
    for dir in [&ghost, &shell, &balloon] {
        std::fs::create_dir_all(dir).expect("検体のフォルダ");
    }
    text(&ghost.join("descript.txt"), "charset,UTF-8\r\nname,g\r\n");
    text(&shell.join("descript.txt"), "charset,UTF-8\r\n");
    text(
        &shell.join("surfaces.txt"),
        "charset,UTF-8\r\n\
         balloon.a\r\n{\r\nsize,100,50\r\narrow.filename,myarrow\r\n}\r\n\
         balloon.b\r\n{\r\nsize,100,50\r\n}\r\n\
         surface0\r\n{\r\nelement0,overlay,base.png,0,0\r\n\
         element1,balloon,a,0,0\r\nelement2,balloon,b,0,60\r\n}\r\n",
    );
    png(&shell.join("base.png"), 8, 8);
    png(&shell.join("myarrow0.png"), 3, 5);
    png(&shell.join("myarrow1.png"), 4, 6);
    png(&shell.join("arrow0.png"), 2, 2);
    png(&shell.join("arrowk0.png"), 5, 5);

    text(&balloon.join("descript.txt"), "charset,UTF-8\r\n");
    text(
        &balloon.join("balloons1s.txt"),
        "charset,UTF-8\r\narrow.filename,alt\r\n",
    );
    png(&balloon.join("balloons0.png"), 20, 20);
    png(&balloon.join("balloons1.png"), 20, 20);
    png(&balloon.join("arrow0.png"), 7, 7);
    png(&balloon.join("arrow1.png"), 7, 8);
    png(&balloon.join("alt0.png"), 9, 9);
    png(&balloon.join("alt1.png"), 9, 10);

    let boot = match build_boot_assets(root.path(), &balloon, &[0, 1], 96, 96) {
        Ok(boot) => boot,
        Err(e) => panic!("資産づくりは成功する: {e:?}"),
    };

    let size = |img: &Option<std::sync::Arc<areka_emo_present::balloon_marker::MarkerImage>>| {
        img.as_ref().map(|i| (i.width, i.height))
    };
    let boxes: Vec<_> = boot
        .boxes
        .markers
        .iter()
        .flat_map(|(actor, boxes)| {
            boxes.iter().map(move |(name, pair)| {
                (
                    actor.as_str(),
                    name.as_str(),
                    size(&pair.up),
                    size(&pair.down),
                )
            })
        })
        .collect();
    assert_eq!(
        boxes,
        vec![
            ("0", "a", Some((3, 5)), Some((4, 6))),
            ("0", "b", Some((2, 2)), None),
            ("1", "a", Some((3, 5)), Some((4, 6))),
            ("1", "b", Some((5, 5)), None),
        ],
        "箱ごとの矢印の絵はシェルのフォルダから引き、無い番号は空のまま（埋めるのは文字の層）"
    );

    let defs = &boot.balloons[0].definitions;
    assert_eq!(
        (
            size(&defs.face0().arrows.up),
            size(&defs.face0().arrows.down)
        ),
        (Some((7, 7)), Some((7, 8))),
        "面 0 はバルーンのフォルダの既定の名前の絵"
    );
    assert_eq!(
        (
            defs.for_face(1).model.arrow_keys().filename.as_deref(),
            size(&defs.for_face(1).arrows.up),
            size(&defs.for_face(1).arrows.down),
        ),
        (Some("alt"), Some((9, 9)), Some((9, 10))),
        "面 1 は自分の設定ファイルを重ねたモデルと、その名前の絵"
    );
}
