//! 抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト
//!
//!
//! # 何を固定するか
//!
//! α を持たない標準テンプレートの絵は、抜き色で抜かれた場所が透明になり、そこをクリックすると
//! 背後の窓へ抜ける。本テストは、実物の絵が「焼く → 計画を組む → 画面の入れ物へ描く」を通った
//! 結果、抜かれた画素ではクリックの当たり（入れ物の α）が「外」、残った画素では「内」になっている
//! ことを、面の外形の全画素について言い切る。途中のどの段で抜いた α が落ちても、ここで赤になる。
//!
//! # 抜かれるのは左上の画素と同じ色の画素
//!
//! 抜き色は左上（座標 0,0）の画素の色で、それと 4 バイトが完全に一致する画素が抜かれる。
//! 一致する画素は絵の内側にも在る（`R_POST_and_KOMAINU` の抜き色は白、`konnoyayame` は緑）。
//! ゆえに本テストは「絵の内側は全部『内』」とは主張せず、「左上と同じ色の画素は『外』・
//! それ以外は『内』」という規則そのものを主張する。
//!
//! # 製品と同じ順・同じ入口で通す
//!
//! 検体のシェルを起動側と同じ読み込みの入口（`load_shell_target`）で読み（ここで焼かれる）、
//! presenter に装着して面の表示の指令を適用し（ここで計画を組んで節へ写す）、画面へ描く。段を
//! 飛ばしたり途中の段を単独で呼んだりはしない。
//!
//! # 読むのはクリックの当たりそのもの
//!
//! 判定は wintf の当たり（`hit_test_entity`・`HitTestMode::SurfaceAlpha`）に面 entity の画素の中心を
//! 渡して読む。クリックの透過が読むのと同じ入れ物の 1 画素であり、写しではない。
//!
//! # 正解は製品の段の出力から導かない
//!
//! 「抜かれた画素」の正解は、テスト自身が同じ PNG を復号した生の画素から決める。正規化・焼き・
//! 計画・描画のどの段の出力も正解づくりには使わない（使うと、壊れた段の出力どうしを突き
//! 合わせて緑になってしまう）。
//!
//! # GPU とのやりとり
//!
//! 画面の入れ物から 1 画素ずつ読み戻す。GPU 資源の前提は既存の presenter 系のテストと同じで、
//! 実窓は作らない（HWND の無い窓の部品へ描く）。

use super::*;

use std::path::Path;

use areka_emo_atlas::{ElementDecoder, WicDecoderArm};
use wintf::ecs::hit_test_entity;

use super::test_support::{
    make_world_with_gpu, mount_entities, paint_screen, screen_point, show_ok, spawn_window_with_dpi,
};
use crate::shell_target::test_support::{konnoyayame_shell_dir, r_post_and_komainu_shell_dir};
use crate::shell_target::{ShellTarget, load_shell_target};

/// 検体のシェルを起動側と同じ読み込みの入口で読む（ここで絵が焼かれる）。
fn load(decoder: &WicDecoderArm, name: &str, shell_dir: &Path) -> ShellTarget {
    let target = load_shell_target(shell_dir, decoder)
        .unwrap_or_else(|e| panic!("{name} のシェル（{}）が読めない: {e}", shell_dir.display()));
    assert!(
        target.bake_errors().is_empty(),
        "{name} で焼く段に落ちた絵がある（前提は 0 件）: {:?}",
        target.bake_errors()
    );
    target
}

/// 同じ PNG を復号した生の画素から「抜かれた画素」を定める（製品の正規化・焼き・計画・描画の
/// どの出力からも導かない）。返り値は行優先・`width * height` 個の bool（true＝抜かれた）。
fn keyed_pixels(decoder: &WicDecoderArm, png: &Path) -> (Vec<bool>, u32, u32) {
    let img = decoder
        .decode(png)
        .unwrap_or_else(|e| panic!("{} が復号できない: {e:?}", png.display()));
    assert!(
        !img.has_alpha,
        "{} が α を持つ（抜き色の扱いを通らないので、このテストの前提が崩れた）",
        png.display()
    );
    let (w, h, stride) = (img.width, img.height, img.stride as usize);
    let key = &img.bgra[0..4];
    let mut keyed = Vec::with_capacity((w * h) as usize);
    for y in 0..h as usize {
        for x in 0..w as usize {
            // 行の詰め物（stride > width*4）は読まない。
            let at = y * stride + x * 4;
            keyed.push(&img.bgra[at..at + 4] == key);
        }
    }
    let n = keyed.iter().filter(|k| **k).count();
    assert!(
        0 < n && n < keyed.len(),
        "{}: 抜かれた画素が {n}／{} 画素（0 より大きく全画素より小さいはず。全画素が同じ色か、どこも抜かれない）",
        png.display(),
        keyed.len()
    );
    (keyed, w, h)
}

/// 面の外形の全画素について、抜かれた画素ではクリックの当たりが外れ・抜かれなかった画素では当たる
/// かを調べる。食い違いがあれば件数と先頭 5 件の座標を載せた失敗の文言を返す（3 面すべての
/// 結果を 1 回の走行で見せるため、ここでは止めずにテスト本体の最後でまとめて主張する）。
fn keyed_out_mismatch(
    at: &str,
    world: &World,
    surface: Entity,
    keyed: &[bool],
    w: u32,
    h: u32,
) -> Option<String> {
    let mut mismatches = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let want_hit = !keyed[(y * w + x) as usize];
            if hit_test_entity(world, surface, screen_point(world, surface, x, y)) != want_hit {
                mismatches.push((x, y, want_hit));
            }
        }
    }
    (!mismatches.is_empty()).then(|| {
        format!(
            "{at}: 抜き色の判定とクリックの当たりが {} 画素で食い違う（先頭 5 件 (x, y, 期待は内か): {:?}）",
            mismatches.len(),
            &mismatches[..mismatches.len().min(5)]
        )
    })
}

/// 抜き色で透明になった場所のクリックが背後へ抜けることを固定する。
///
/// 標準テンプレートの検体 2 体の 3 面を、読む（焼く）→ 装着して面を表示する → 画面へ描くまで製品と
/// 同じ順で通し、クリックの当たりで「左上と同じ色の画素は外・それ以外は内」が全画素で成り立つ
/// ことを主張する。
#[test]
fn keyed_out_pixels_let_clicks_through_and_the_rest_stay_inside() {
    // COM の初期化は make_world_with_gpu が行う。復号器はその後に同じスレッドで作る。
    let mut world = make_world_with_gpu();
    let decoder = WicDecoderArm::new().expect("COM 初期化下で WIC の復号器が作れる");

    let r_post_dir = r_post_and_komainu_shell_dir();
    let konno_dir = konnoyayame_shell_dir();
    let r_post = load(&decoder, "R_POST_and_KOMAINU", &r_post_dir);
    let konno = load(&decoder, "konnoyayame", &konno_dir);

    // (target, 検体, シェルのフォルダ, 面, PNG のファイル名, 名札)
    let faces: [(TargetId, &ShellTarget, &Path, u32, &str, &str); 3] = [
        (
            TargetId(0),
            &r_post,
            &r_post_dir,
            0,
            "surface0000.png",
            "R_POST_and_KOMAINU 面 0",
        ),
        // surfaces.txt に宣言の無い面（ファイル名だけで建つ面）。
        (
            TargetId(1),
            &r_post,
            &r_post_dir,
            10,
            "surface0010.png",
            "R_POST_and_KOMAINU 面 10",
        ),
        // パレット形式の PNG。
        (
            TargetId(2),
            &konno,
            &konno_dir,
            0,
            "surface0000.png",
            "konnoyayame 面 0",
        ),
    ];

    let mut presenter = EmoPresenter::new();
    for (target, shell, _, surface_id, _, _) in faces {
        let window = spawn_window_with_dpi(&mut world, 96);
        presenter.attach_target(
            target,
            window,
            shell.build_world(),
            shell.atlas().clone(),
            96,
        );
        show_ok(&mut presenter, &mut world, target, surface_id);
    }

    paint_screen(&mut world);

    let mut failures = Vec::new();
    for (target, _, dir, _, png, at) in faces {
        let (keyed, w, h) = keyed_pixels(&decoder, &dir.join(png));
        assert_eq!(
            presenter.targets[&target].native_size,
            Some((w, h)),
            "{at}: 表示した面の外形が PNG の外形と違う"
        );
        let surface_entity = mount_entities(&presenter, target).0;
        failures.extend(keyed_out_mismatch(at, &world, surface_entity, &keyed, w, h));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
