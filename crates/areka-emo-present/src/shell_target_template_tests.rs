//! 里々・YAYA の標準テンプレート 2 体を、**実物の絵を焼いて**確かめる檻（要件 7.4・7.5）。
//!
//! `shell_target_base_image_tests.rs` がメモリ上の復号器で見るのは土台の絵の決まり方（表ア〜エ）
//! だけである。本ファイルはそこを通したうえで、実在するシェルのフォルダを [`load_shell_target`]
//! に渡し、fs の一覧・文字コードの判定・実物の PNG の復号・抜き色までを 1 本に通した結果を見る。
//!
//! # 2 体が何を代表しているか
//!
//! どちらのテンプレートも `surfaces.txt` に `element` 行を **0 本**しか持たない
//! （要件 Introduction の実測表）。
//! 絵はすべてファイル名の慣習（`surface0000.png` 形式）だけで置かれており、本仕様の適用前は
//! 面の画像が 1 枚も土台にならない＝**1 体も絵が出ない**。ゆえに「外形が実寸になる」こと自体が、
//! 名前の判定（要件 1）から焼き付けまでが通ったことの証拠である。
//!
//! - `R_POST_and_KOMAINU`（里々）: 面 10 は `surfaces.txt` に波括弧を**持たない**。画像だけで
//!   存在する面が起動の相方側として引けること（要件 3.1・3.2）をここで判定する。絵は α
//!   チャンネルを持たない truecolor の PNG なので、左上の色が抜かれる（要件 4.1）。
//! - `konnoyayame`（YAYA）: 面 0 が `animation0`（`sometimes`）でまばたきのコマを持ち、その相手
//!   （面 1031〜1033）は画像だけで存在する面である。コマの相手の解決（要件 3.4）を、コマを
//!   入れた合成と入れない合成の**画素の差**で判定する。
//!
//! # なぜ差の「在処」まで見るのか
//!
//! コマの相手が解決できなければ、合成は失敗せず**そのコマを描かずに**続行する（要件 3.5）ので、
//! 素通りした形は「差が 0」として現れる。逆に差だけを数えると、コマが位置を無視して全面に
//! 描かれた形も緑になってしまう。ゆえに差が 1 画素以上在ることと、差が矩形
//! (93,103)〜(165,133)（コマの位置＋コマの絵の実寸 72×30）の中だけに在ることを両方判定する。
//!
//! # 受け口
//!
//! 検体と COM の初期化は `shell_target_test_support.rs`（[`super::test_support`]）から受ける。
//! `vendors/sample_ghost/<検体名>/` の直パスは 1 か所も綴らない（要件 7.11）。

use super::*;

use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::{
    BindSet, ComposeMethod, ComposedSurface, Composer, PatternFrame, PatternState,
};

use super::test_support::{
    konnoyayame_shell_dir, r_post_and_komainu_shell_dir, with_com_initialized,
};

/// `R_POST_and_KOMAINU` の面 0 の外形（要件 3.1・7.4・`surface0000.png` の実寸）。
const R_POST_SURFACE0: (u32, u32) = (236, 462);
/// 同 面 10 の外形（`surface0010.png` の実寸）。宣言の無い面である。
const R_POST_SURFACE10: (u32, u32) = (140, 160);

/// `konnoyayame` の面 0 の外形（要件 3.1・7.5・`surface0000.png` の実寸）。
const KONNOYAYAME_SURFACE0: (u32, u32) = (260, 390);
/// 同 面 10 の外形（`surface0010.png` の実寸）。
const KONNOYAYAME_SURFACE10: (u32, u32) = (200, 200);

/// まばたきの `animation0`（`konnoyayame` の面 0）。
const BLINK_ANIMATION: u32 = 0;
/// `animation0.pattern0` の相手の面（画像だけで存在する・要件 3.4）。
const BLINK_SURFACE: u32 = 1031;
/// `animation0.pattern*` の位置（`surfaces.txt` の実測・全コマ共通）。
const BLINK_AT: (i64, i64) = (93, 103);
/// コマの絵の実寸（`surface1031.png`・要件 4.5 が挙げる 72×30）。
const BLINK_SIZE: (u32, u32) = (72, 30);

/// 面 1 枚を合成する（有効 bind は無し・コマは引数で与える）。
fn compose(
    target: &ShellTarget,
    world: &EmoWorld,
    id: u32,
    patterns: &PatternState,
) -> ComposedSurface {
    Composer::new()
        .compose(world, target.atlas(), id, &BindSet::default(), patterns)
        .unwrap_or_else(|e| panic!("面 {id} は合成できる: {e}"))
}

/// 合成結果の 1 画素（premultiplied BGRA）を引く。
fn pixel(surface: &ComposedSurface, x: u32, y: u32) -> [u8; 4] {
    let at = (y * surface.stride() + x * 4) as usize;
    surface.bytes()[at..at + 4]
        .try_into()
        .expect("4 バイトの画素")
}

/// `surfaces.txt` が面 `id` の波括弧を持つか（宣言の有無）。
fn is_declared(target: &ShellTarget, id: u32) -> bool {
    target.shell.surfaces.iter().any(|s| s.id == id)
}

/// 検体のシェルを実物の絵ごと読む（COM 初期化済みの中でだけ呼べる）。
fn load(shell_dir: std::path::PathBuf) -> ShellTarget {
    let decoder = WicDecoderArm::new().expect("COM 初期化下で WIC ファクトリが作れる");
    let target = load_shell_target(&shell_dir, &decoder)
        .unwrap_or_else(|e| panic!("{} のシェルは読める: {e}", shell_dir.display()));
    assert!(
        target.bake_errors().is_empty(),
        "{} で焼く段に落ちた絵がある（要件 7.4・7.5 は 0 件）: {:?}",
        shell_dir.display(),
        target.bake_errors()
    );
    target
}

/// 要件 3.1・3.2・4.1・7.4: `R_POST_and_KOMAINU` は名前の慣習だけで両側の絵が出る。
///
/// 面 10 は `surfaces.txt` に波括弧を持たない——それでも面の表に居て 140×160 で合成できることが、
/// 「画像だけで存在する面」（要件 2.2）が起動の相方側として届いたことを示す。`\s[10]` が同じ面へ
/// 届くこと（要件 3.2）も、表示の段が引くのは同じ面の表（[`EmoWorld::surface`]）なのでここに載る。
///
/// 左上の画素の α が 0 であることは抜き色（要件 4.1）の結果である。全画素が透明でも同じ表明が
/// 立つので、不透明な画素が 1 つ以上在ることを較正として併せて判定する。
#[test]
fn r_post_and_komainu_shows_both_scopes_from_file_names_alone() {
    with_com_initialized(|| {
        let target = load(r_post_and_komainu_shell_dir());
        let world = target.build_world();

        // 前提の較正: 面 0 は波括弧を持ち（当たり判定などがある）、面 10 は持たない。
        // ここが逆なら、以下の「宣言の無い面 10」は別の面を見ていることになる。
        assert!(
            is_declared(&target, 0),
            "前提: 面 0 は surfaces.txt に波括弧を持つ"
        );
        assert!(
            !is_declared(&target, 10),
            "前提: 面 10 は surfaces.txt に波括弧を持たない（画像だけで存在する面）"
        );
        assert!(
            world.surface(10).is_some(),
            "宣言の無い面 10 が面の表に居ない（要件 2.2 の「存在する面」になっていない）"
        );

        let surface0 = compose(&target, &world, 0, &PatternState::default());
        assert_eq!(
            (surface0.width(), surface0.height()),
            R_POST_SURFACE0,
            "面 0 の外形が本体側の実寸と違う（要件 3.1）"
        );

        let surface10 = compose(&target, &world, 10, &PatternState::default());
        assert_eq!(
            (surface10.width(), surface10.height()),
            R_POST_SURFACE10,
            "面 10 の外形が相方側の実寸と違う（要件 3.1）"
        );

        assert_eq!(
            pixel(&surface0, 0, 0)[3],
            0,
            "面 0 の左上の画素が透明でない（α を持たない絵の抜き色が効いていない・要件 4.1）"
        );
        // 較正: 全画素が透明な絵なら上の表明は恒真になる。
        assert!(
            surface0.bytes().chunks_exact(4).any(|px| px[3] == 255),
            "面 0 に不透明な画素が 1 つも無い（絵が抜かれすぎている）"
        );
    });
}

/// 要件 3.1・7.5: `konnoyayame` も名前の慣習だけで両側の絵が出る。
#[test]
fn konnoyayame_shows_both_scopes_from_file_names_alone() {
    with_com_initialized(|| {
        let target = load(konnoyayame_shell_dir());
        let world = target.build_world();

        let surface0 = compose(&target, &world, 0, &PatternState::default());
        assert_eq!(
            (surface0.width(), surface0.height()),
            KONNOYAYAME_SURFACE0,
            "面 0 の外形が本体側の実寸と違う（要件 3.1）"
        );

        let surface10 = compose(&target, &world, 10, &PatternState::default());
        assert_eq!(
            (surface10.width(), surface10.height()),
            KONNOYAYAME_SURFACE10,
            "面 10 の外形が相方側の実寸と違う（要件 3.1）"
        );
    });
}

/// 要件 3.4・11.5・7.5: 面 0 のまばたきのコマが、画像だけで存在する面 1031 を位置 93,103 に描く。
///
/// コマの相手が解決できなければ合成は失敗せずそのコマを飛ばす（要件 3.5）ので、素通りした形は
/// 「差が 0」として現れる。差が 1 画素以上在ることと、差がコマの矩形の中だけに在ることを
/// 両方判定するのは、位置を無視して描かれた形を緑にしないためである。
#[test]
fn konnoyayame_blink_frame_changes_pixels_only_inside_the_frame_rect() {
    with_com_initialized(|| {
        let target = load(konnoyayame_shell_dir());
        let world = target.build_world();

        // 前提の較正: コマの相手は `surfaces.txt` に波括弧を持たず、画像だけで面の表に居る。
        assert!(
            !is_declared(&target, BLINK_SURFACE),
            "前提: 面 {BLINK_SURFACE} は surfaces.txt に波括弧を持たない"
        );
        assert!(
            world.surface(BLINK_SURFACE).is_some(),
            "コマの相手の面 {BLINK_SURFACE} が面の表に居ない（要件 3.4 の解決が効いていない）"
        );

        let base = compose(&target, &world, 0, &PatternState::default());

        let mut patterns = PatternState::default();
        patterns.set(
            BLINK_ANIMATION,
            PatternFrame {
                surface_id: BLINK_SURFACE,
                method: ComposeMethod::Overlay,
                x: BLINK_AT.0,
                y: BLINK_AT.1,
            },
        );
        let blinked = compose(&target, &world, 0, &patterns);

        assert_eq!(
            (blinked.width(), blinked.height()),
            (base.width(), base.height()),
            "コマは外形に寄与しない（外形が動いたら以下の画素の突き合わせは同じ座標を見ていない）"
        );

        // 差が収まるべき矩形（コマの位置＋コマの絵の実寸）。
        let (left, top) = (BLINK_AT.0 as u32, BLINK_AT.1 as u32);
        let (right, bottom) = (left + BLINK_SIZE.0, top + BLINK_SIZE.1);
        assert_eq!(
            (left, top, right, bottom),
            (93, 103, 165, 133),
            "前提: 矩形は tasks 6.1 と design が挙げる (93,103)〜(165,133) でなければならない"
        );

        let mut differing = 0usize;
        let mut outside: Vec<(u32, u32)> = Vec::new();
        for y in 0..base.height() {
            for x in 0..base.width() {
                if pixel(&base, x, y) == pixel(&blinked, x, y) {
                    continue;
                }
                differing += 1;
                if !(left..right).contains(&x) || !(top..bottom).contains(&y) {
                    outside.push((x, y));
                }
            }
        }

        assert!(
            differing > 0,
            "コマを入れても画素が 1 つも変わらない（相手の面が解決できず素通りしている）"
        );
        assert!(
            outside.is_empty(),
            "コマの矩形 ({left},{top})〜({right},{bottom}) の外で {} 画素が変わった（先頭 5 件: {:?}）",
            outside.len(),
            &outside[..outside.len().min(5)]
        );
    });
}
