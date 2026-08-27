//! 供給層の換算ステップ（[`super::apply_author_balloon_offset_scale`]）の決定論テスト
//! （areka-P0-balloon-offset-dpi・要件 1.2／2.1／2.2／2.3／2.5／2.6・task 4.1）。
//!
//! ここが固定するのは**換算そのもの**——作者空間の生値が合流欄へ入る前にシェル軸の
//! 拡大率で物理 px へ置き換わること、拡大率 1 では従来と同一であること、受理規約
//! （両軸が揃ったときのみ採用）が変わらないこと、飽和が回り込まないこと。
//!
//! `prepare_stages` の**呼出順**そのものと、飽和が実際に警告として出ること、および
//! 合流後の値を実配置経路で観測することは task 4.3 の所有である。本ファイルは 4.3 が
//! 追記する土台でもある（design「新規テストファイル」の `balloon_offset_supply_tests.rs`）。

use areka_emo_compose::ScaleRatio;

use super::config::{PlacementConfig, ScopeConfig};
use super::windowposition::{apply_windowposition, to_screen_adjust};
use super::*;

/// テスト用 k（非ゼロ前提）。
fn k(num: u32, den: u32) -> ScaleRatio {
    ScaleRatio::new(num, den).expect("テストの k は非ゼロ")
}

/// scope 1 つだけを持つ最小の `PlacementConfig`。`offset` をそのまま合流欄へ置く。
fn cfg_with_offset(offset: Option<(i32, i32)>) -> PlacementConfig {
    let mut cfg = PlacementConfig {
        scopes: Default::default(),
        zorder_raw: None,
        sticky_window_raw: None,
        shell_dpi_raw: None,
    };
    cfg.scopes.insert(
        0,
        ScopeConfig {
            balloon_offset: offset,
            ..ScopeConfig::default()
        },
    );
    cfg
}

/// 換算ステップを scope 0 に対して 1 度走らせ、合流欄の値を返す。
fn scaled(offset: Option<(i32, i32)>, k: ScaleRatio) -> Option<(i32, i32)> {
    let mut cfg = cfg_with_offset(offset);
    apply_author_balloon_offset_scale(&mut cfg, &[0], k);
    cfg.scopes[&0].balloon_offset
}

// -------------------------------------------------------------------------
// 1. 換算そのもの（要件 2.1・design D2＝シェル軸）
// -------------------------------------------------------------------------

/// 拡大率 ≠ 1 のとき、宣言された生値は**換算後の値**として合流欄に載る（要件 2.1／1.2）。
///
/// 是正前は生値がそのまま残るため、本主張が「是正前は失敗する側」である
/// （design Testing Strategy 是正⑴）。
///
/// 期待値は既存の丸め権威 `ScaleRatio::scale_len`（round half away from zero）から
/// 手計算した literal であって、実装の出力を書き写したものではない:
/// 266→332.5→333・129→161.25→161（5/4）、100→150・40→60（3/2）。
#[test]
fn declared_offset_is_replaced_by_the_converted_value() {
    assert_eq!(scaled(Some((266, -129)), k(120, 96)), Some((333, -161)));
    assert_eq!(scaled(Some((100, -40)), k(144, 96)), Some((150, -60)));
    // 拡大率 < 1（作者が 192dpi を宣言し 96dpi 画面へ出る腕）も同じ規則で縮む。
    assert_eq!(scaled(Some((266, -129)), k(96, 192)), Some((133, -65)));
}

/// 符号は換算で保存される——大きさだけが k 倍される（要件 9.3・`scale_signed` へ委譲）。
#[test]
fn conversion_preserves_sign_on_both_axes() {
    assert_eq!(scaled(Some((-8, 8)), k(2, 1)), Some((-16, 16)));
    assert_eq!(scaled(Some((8, -8)), k(2, 1)), Some((16, -16)));
    assert_eq!(scaled(Some((0, 0)), k(2, 1)), Some((0, 0)));
}

// -------------------------------------------------------------------------
// 2. 拡大率 1（要件 2.2・7.5）
// -------------------------------------------------------------------------

/// 拡大率 1 では合流欄が**本仕様の適用前と bit 同一**である（要件 2.2）。
///
/// `ScaleRatio::new(96, 96)` は正準形 1/1 へ約分されるため、`ONE` を直に渡した場合と
/// 区別されない——DPI の絶対値ではなく比だけが効くことも同時に固定する。
#[test]
fn identity_scale_is_byte_identical_to_the_raw_value() {
    for raw in [
        (0, 0),
        (266, -129),
        (-190, 75),
        (i32::MAX, -i32::MAX),
        (1, -1),
    ] {
        assert_eq!(scaled(Some(raw), ScaleRatio::ONE), Some(raw), "raw={raw:?}");
        assert_eq!(scaled(Some(raw), k(96, 96)), Some(raw), "raw={raw:?}");
        assert_eq!(scaled(Some(raw), k(120, 120)), Some(raw), "raw={raw:?}");
    }
}

/// **要件 2.2 の字面に対する唯一の例外**（task 1.3 → 4.1 申し送り）。
///
/// 恒等比でも `i32::MIN` だけは生値が素通しにならず `-i32::MAX` へ飽和する。出所は
/// 既存の単一丸め権威 `scale_signed`（`v.unsigned_abs()` が `2^31` になり `i32` へ
/// 収まらない）であり、`windowposition` 由来の調整量は**今日すでに同じ挙動**である。
/// 回り込みは起きず、飽和は値としても警告としても報告されるため製品欠陥ではない。
///
/// ゆえに作者が書ける生値の実質的な受理範囲は `i32::MIN + 1 ..= i32::MAX` である
/// ——この 1 点が黙って別の値（回り込み等）へ変わらないよう本檻が固定する。
/// 純関数側の対は `follow_offset_space_tests.rs` の
/// `supply_identity_saturates_only_at_i32_min`。
#[test]
fn identity_scale_saturates_only_at_i32_min() {
    assert_eq!(
        scaled(Some((i32::MIN, 5)), ScaleRatio::ONE),
        Some((-i32::MAX, 5))
    );
    assert_eq!(
        scaled(Some((5, i32::MIN)), ScaleRatio::ONE),
        Some((5, -i32::MAX))
    );
    // 隣の値（`i32::MIN + 1`）は素通しする＝例外はちょうど 1 点だけである。
    assert_eq!(
        scaled(Some((i32::MIN + 1, 0)), ScaleRatio::ONE),
        Some((i32::MIN + 1, 0))
    );
}

// -------------------------------------------------------------------------
// 3. 受理規約は不変（要件 2.6）
// -------------------------------------------------------------------------

/// 未宣言・片軸のみ宣言（＝`config.rs` の `offset_x.zip(offset_y)` が `None` を出す腕）は
/// 換算ステップを素通りし、`None` のまま残る（要件 2.6）。
///
/// 「両軸が揃ったときのみ採用する」判断は `config.rs` の所有であり、本ステップは
/// その結果を受け取るだけで**採否そのものを 1 bit も変えない**。
#[test]
fn unset_offset_stays_unset_across_every_scale() {
    for ratio in [ScaleRatio::ONE, k(120, 96), k(96, 192)] {
        assert_eq!(scaled(None, ratio), None);
    }
}

/// 配置表に無い scope を渡しても表を生やさない（合流先を発明しない）。
#[test]
fn unknown_scope_does_not_create_a_scope_entry() {
    let mut cfg = cfg_with_offset(Some((10, 10)));
    apply_author_balloon_offset_scale(&mut cfg, &[7], k(2, 1));
    assert!(!cfg.scopes.contains_key(&7));
    // 既存 scope も触られない（要求された scope だけが対象）。
    assert_eq!(cfg.scopes[&0].balloon_offset, Some((10, 10)));
}

// -------------------------------------------------------------------------
// 4. 飽和（要件 2.5）
// -------------------------------------------------------------------------

/// `i32` 域を超えた軸は**回り込ませず飽和値**（`±i32::MAX`）を採る（要件 2.5）。
///
/// 飽和が警告として記録されることの固定は task 4.3 の所有。ここが押さえるのは
/// 「採る値が回り込みではない」ことである。
#[test]
fn out_of_range_conversion_saturates_instead_of_wrapping() {
    assert_eq!(
        scaled(Some((i32::MAX, -i32::MAX)), k(2, 1)),
        Some((i32::MAX, -i32::MAX))
    );
    // 片軸だけ飽和する腕でも、もう一方は正しく換算される。
    assert_eq!(scaled(Some((i32::MAX, 3)), k(3, 1)), Some((i32::MAX, 9)));
}

// -------------------------------------------------------------------------
// 5. 供給軸の分離と合流値（要件 2.3・design Testing Strategy「Unit Tests」4 の前半）
// -------------------------------------------------------------------------

/// シェル軸とバルーン軸が**異なる**とき、`descript` 由来はシェル軸で・`windowposition`
/// 由来はバルーン軸で換算され、**加算後の合流値が同一空間（物理 px）の和になる**
/// （要件 2.3・1.2）。
///
/// 現行フィクスチャは `seriko.dpi`／balloon `dpi` を宣言していないため 2 軸は今日つねに
/// 一致する。この分岐はフィクスチャでは踏めないので、供給の 2 ステップを直に並べて踏む
/// （要件 7.9「決定論で判定できない事項のみを実機へ回す」）。
///
/// 手計算: descript (100, −40) × シェル 5/4 = (125, −50)。
/// windowposition (10, 20) × バルーン 2/1 = (20, 40)。合流 = (145, −10)。
/// 混在したまま加算していた従来は (100+20, −40+40) = (120, 0) になり、本主張は落ちる。
#[test]
fn shell_axis_and_balloon_axis_sum_in_the_same_space() {
    let shell = k(120, 96);
    let balloon = k(192, 96);
    let mut cfg = cfg_with_offset(Some((100, -40)));

    // 供給の順序は本番と同じ——換算が先・windowposition の合流が後（design D3）。
    apply_author_balloon_offset_scale(&mut cfg, &[0], shell);
    assert_eq!(
        cfg.scopes[&0].balloon_offset,
        Some((125, -50)),
        "合流の前にシェル軸で換算済みであること"
    );

    let adjust = to_screen_adjust(Some(10), Some(20), balloon);
    assert_eq!(
        adjust,
        Some((20, 40)),
        "windowposition はバルーン軸で換算される"
    );
    apply_windowposition(&mut cfg, 0, adjust);

    assert_eq!(cfg.scopes[&0].balloon_offset, Some((145, -10)));
}

/// 拡大率 1 では、合流後の値も本仕様の適用前と bit 同一である（要件 2.2／2.3）。
#[test]
fn merged_value_at_identity_matches_the_pre_spec_sum() {
    let mut cfg = cfg_with_offset(Some((266, -129)));
    apply_author_balloon_offset_scale(&mut cfg, &[0], ScaleRatio::ONE);
    apply_windowposition(
        &mut cfg,
        0,
        to_screen_adjust(Some(-190), Some(75), ScaleRatio::ONE),
    );
    assert_eq!(cfg.scopes[&0].balloon_offset, Some((266 - 190, -129 + 75)));
}
