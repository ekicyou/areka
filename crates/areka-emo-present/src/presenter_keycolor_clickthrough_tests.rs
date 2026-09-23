//! 抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト
//! （完了 spec `areka-P0-shell-implicit-surface` の要件 4.10）。
//!
//! # 何を固定するか
//!
//! α を持たない標準テンプレートの絵は、抜き色で抜かれた場所が透明になり、そこをクリックすると
//! 背後の窓へ抜ける。本テストは、実物の絵が「焼く → 合成する → 当たりのマスクを作る」を通った
//! 結果、抜かれた画素ではマスクが「外」、残った画素では「内」になっていることを、面の外形の
//! 全画素について言い切る。途中のどの段で抜いた α が落ちても、ここで赤になる。
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
//! presenter に装着して面の表示の指令を適用する（ここで合成し、マスクを作る）。段を飛ばしたり
//! 途中の段を単独で呼んだりはしない。
//!
//! # 読むマスクは当たり判定が読むのと同じもの
//!
//! 判定に使うマスクは、装着した窓の面 entity に載る `AlphaMaskResource` から読む。wintf の
//! 当たり判定が読むのと同じ `Arc` の中身であり、写しではない。キャッシュが束ねるマスクだけを
//! 読んで済ませることはしない。
//!
//! # 正解は製品の段の出力から導かない
//!
//! 「抜かれた画素」の正解は、テスト自身が同じ PNG を復号した生の画素から決める。正規化・焼き・
//! 合成・マスクのどの段の出力も正解づくりには使わない（使うと、壊れた段の出力どうしを突き
//! 合わせて緑になってしまう）。
//!
//! # GPU とのやりとり
//!
//! GPU からの読み戻しは 0 回である。一方で、表示の記録（`record_display`）のときに CPU から
//! GPU への転送は起きる。GPU 資源の前提は既存の presenter 系のテストと同じで、実窓は作らない。

use super::*;

use std::path::Path;

use areka_emo_atlas::{ElementDecoder, WicDecoderArm};
use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::test_support::{make_world_with_gpu, mount_entities, show_ok, spawn_window_with_dpi};
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

/// 同じ PNG を復号した生の画素から「抜かれた画素」を定める（製品の正規化・焼き・合成・マスクの
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

/// 面の外形の全画素について、抜かれた画素では `is_hit` が false・抜かれなかった画素では true で
/// あることを主張する。食い違いがあれば件数と先頭 5 件の座標を失敗の文言に出す。
fn assert_keyed_out_pixels_leave_the_mask(
    at: &str,
    mask: &AlphaMask,
    keyed: &[bool],
    w: u32,
    h: u32,
) {
    assert_eq!(
        (mask.width(), mask.height()),
        (w, h),
        "{at}: マスクの外形が PNG の外形と違う"
    );
    let mut mismatches = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let want_hit = !keyed[(y * w + x) as usize];
            if mask.is_hit(x, y) != want_hit {
                mismatches.push((x, y, want_hit));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{at}: 抜き色の判定とマスクが {} 画素で食い違う（先頭 5 件 (x, y, 期待は内か): {:?}）",
        mismatches.len(),
        &mismatches[..mismatches.len().min(5)]
    );
}

/// 抜き色で透明になった場所のクリックが背後へ抜けることを固定する（完了 spec
/// `areka-P0-shell-implicit-surface` の要件 4.10）。
///
/// 標準テンプレートの検体 2 体の 3 面を、読む（焼く）→ 装着して面を表示する（合成し、マスクを
/// 作る）まで製品と同じ順で通し、当たり判定が読むマスクで「左上と同じ色の画素は外・それ以外は
/// 内」が全画素で成り立つことを主張する。
#[test]
fn keyed_out_pixels_leave_the_hit_mask_and_the_rest_stay_inside() {
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
    for (target, shell, _, surface_id, _, at) in faces {
        let window = spawn_window_with_dpi(&mut world, 96);
        presenter
            .attach_target(
                &mut world,
                target,
                window,
                shell.build_world(),
                shell.atlas().clone(),
                96,
            )
            .unwrap_or_else(|e| panic!("{at}: attach_target が失敗した: {e:?}"));
        show_ok(&mut presenter, &mut world, target, surface_id);
    }

    for (target, _, dir, _, png, at) in faces {
        let (keyed, w, h) = keyed_pixels(&decoder, &dir.join(png));
        let surface_entity = mount_entities(&presenter, target).0;
        let mask = world
            .get::<AlphaMaskResource>(surface_entity)
            .unwrap_or_else(|| panic!("{at}: 面 entity に AlphaMaskResource が載っていない"))
            .mask()
            .unwrap_or_else(|| panic!("{at}: マスクが供給されていない（表示が成立していない）"));
        assert_keyed_out_pixels_leave_the_mask(at, mask, &keyed, w, h);
    }
}
