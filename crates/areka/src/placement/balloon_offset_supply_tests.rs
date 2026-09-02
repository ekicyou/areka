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

// =========================================================================
// task 4.3: 供給結線（`prepare_stages` の実経路）
//
// ここから下が固定するのは**結線**である——⑴ 換算ステップが `windowposition` の
// 合流より**前**に走ること、⑵ 合流後の値が両供給元を同一空間で加算した結果で
// あること、⑶ 飽和が契約の語で**記録に出る**こと（要件 2.3／2.5／7.1／7.4）。
//
// 上の 4.1 群は純関数と 2 ステップの直列合成を固定したが、本番の呼出順そのものと
// 記録の発生は踏んでいない。本群は `prepare_stages`（＝`apply_author_balloon_offset_scale`
// と `apply_scope_windowpositions` を実際に並べている当の関数）を回して踏む。
//
// # シェル軸とバルーン軸を**異なる値**にする方法（フィクスチャ到達性の但し書き）
//
// 実フィクスチャ（emo2）は `seriko.dpi` も balloon `dpi` も宣言しないため 2 軸は
// つねに一致する。ゆえに本群は既存の合成検体
// [`synth_declared_dpi_ghost`]（`placement_prepare_tests.rs`／
// `placement_windowposition_tests.rs` が既に使っている口）へ `seriko.dpi,120`／
// balloon `dpi,144` を宣言させ、primary=240 を与えて **k_shell=2/1・k_balloon=5/3** を
// 作る。これは**合成による構築**であって現行フィクスチャから到達する状態ではない
// ——要件 7.9 の「決定論で判定できる分岐は決定論で踏む」に従い、実機任せにしない。
// =========================================================================

use std::fs;
use std::path::{Path, PathBuf};

use super::resolver::PointPx;
use super::shared_test_support::{TempDir, WA, synth_declared_dpi_ghost, with_com_initialized};
use super::test_support::{ExpectField, LogEvent, capture_logs, expect_one};

/// `windowposition` 供給層の観測点（`placement_windowposition_tests.rs` が
/// 「フィールド名と値の形そのものが契約」と固定している info 行）の本文。
/// 本群はこの行を**合流ステップが走った時刻の目印**として使う。
const WINDOWPOSITION_SUPPLY_NEEDLE: &str = "windowposition を初期既定位置の調整量へ";

/// 作者オフセット宣言つきの合成ゴーストを組む（[`synth_declared_dpi_ghost`] の薄い延長）。
///
/// 既存の口は `seriko.dpi`／balloon `dpi`／balloon `windowposition` までしか宣言
/// できないので、shell の `descript.txt` へ `sakura.balloon.offsetx`／`offsety` を
/// **追記**する。`sakura.` 接頭辞ゆえ宣言が効くのは scope 0 だけであり、scope 1
/// （`kero.`）は未宣言＝`None` のまま残る（要件 2.6 の受理規約は不変）。
fn synth_ghost_with_author_offset(
    root: &TempDir,
    shell_dpi: &str,
    balloon_dpi: &str,
    windowposition: Option<(i32, i32)>,
    offset: (i32, i32),
) -> (PathBuf, PathBuf) {
    let (ghost_root, balloon_dir) =
        synth_declared_dpi_ghost(root, shell_dpi, balloon_dpi, windowposition);
    let shell_descript = ghost_root.join("shell").join("master").join("descript.txt");
    let mut text = fs::read_to_string(&shell_descript).expect("合成 shell descript を読む");
    text.push_str(&format!(
        "sakura.balloon.offsetx,{}\nsakura.balloon.offsety,{}\n",
        offset.0, offset.1
    ));
    fs::write(&shell_descript, text).expect("合成 shell descript へ作者オフセットを追記");
    (ghost_root, balloon_dir)
}

/// scope 0 の合流欄を `prepare_stages` の出力から取り出す。
fn merged_offset(ghost_root: &Path, balloon_dir: &Path, primary_dpi: u32) -> Option<(i32, i32)> {
    let stages = prepare_stages(ghost_root, balloon_dir, Some(primary_dpi))
        .expect("合成ゴーストの準備は成功する");
    stages.cfg.scopes[&0].balloon_offset
}

// -------------------------------------------------------------------------
// 6. 語彙（task 1.3 → 4.1 申し送り: 綴りの檻）
// -------------------------------------------------------------------------

/// 飽和の記録語の綴りを**逐語で**固定する（`balloon_limit.rs`／`drag_follow.rs` の
/// `*_TAG` と同じ流儀）。
///
/// 実機ログの grep（要件 8.3）と決定論テストが同じ定数を参照する以上、綴りは契約で
/// ある。定数どうしを突き合わせる（＝自分自身と等しいことを主張する）と何も証明
/// しないので、**literal に対して**固定する。
#[test]
fn balloon_offset_saturated_tag_is_spelled_verbatim() {
    assert_eq!(BALLOON_OFFSET_SATURATED_TAG, "[balloon-offset] Saturated");
}

// -------------------------------------------------------------------------
// 7. 合流後の値（要件 2.3・7.1・task 4.3 第 2 項）
// -------------------------------------------------------------------------

/// **本番経路**（`prepare_stages`）で、`descript` 由来と `windowposition` 由来が
/// **それぞれの軸**で物理 px へ換算されたうえで加算されること（要件 2.3／1.2）。
///
/// 検体: `seriko.dpi,120`／balloon `dpi,144`／primary=240 ⇒ k_shell=2/1・k_balloon=5/3。
///
/// 手計算（丸め権威 `ScaleRatio::scale_len`＝round half away from zero）:
/// - descript (100, −40) × シェル 2/1 = (200, −80)
/// - windowposition (266, −129) × バルーン 5/3 = (1330/3=443.33→443, 645/3=215 ちょうど→−215)
/// - 合流 = (200+443, −80−215) = **(643, −295)**
///
/// 是正前（換算ステップが無い＝生値素通し）は (100+443, −40−215) = **(543, −255)** に
/// なり本主張は落ちる（要件 7.4 の「是正前は失敗する側」）。
///
/// scope 1 は `kero.balloon.offsetx` を宣言していないので `windowposition` 由来だけが
/// 載る＝換算対象の有無で結果が分かれることも同時に固定する。
#[test]
fn prepare_merges_each_supply_in_its_own_axis() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "120", "144", Some((266, -129)), (100, -40));

        let stages =
            prepare_stages(&ghost_root, &balloon_dir, Some(240)).expect("合成ゴーストの準備");

        assert_eq!(
            stages.cfg.scopes[&0].balloon_offset,
            Some((643, -295)),
            "descript はシェル軸 2/1・windowposition はバルーン軸 5/3 で換算してから加算する"
        );
        assert_eq!(
            stages.cfg.scopes[&1].balloon_offset,
            Some((443, -215)),
            "scope 1 は作者オフセット未宣言ゆえ windowposition 由来だけが載る"
        );
    });
}

/// **順序**の檻（要件 2.3・task 4.3 第 1 項）: 換算は `windowposition` の合流より
/// **前**に走る。
///
/// 3 つの順序が互いに異なる値を出すので、値ひとつで順序が決まる（同じ検体・同じ手計算）:
/// - 正（換算 → 合流）: (100×2, −40×2) = (200, −80) に (443, −215) を足して **(643, −295)**
/// - 逆（合流 → 換算）: (100+443, −40−215) = (543, −255) をシェル軸 2/1 倍して (1086, −510)
///   ——`windowposition` の調整量にまでシェル軸が二重に掛かる
/// - 換算なし（是正前）: (543, −255)
///
/// ゆえに本檻は**ステップを入れ替えても落ちる**（入れ替えても通る種類の主張ではない）。
#[test]
fn prepare_runs_the_conversion_before_the_windowposition_merge() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "120", "144", Some((266, -129)), (100, -40));

        let merged = merged_offset(&ghost_root, &balloon_dir, 240);

        assert_eq!(merged, Some((643, -295)), "換算 → 合流の順");
        assert_ne!(merged, Some((1086, -510)), "合流 → 換算（逆順）ではない");
        assert_ne!(merged, Some((543, -255)), "換算なし（是正前）ではない");
    });
}

/// 合流後の値が**配置解決の出力まで届く**こと（要件 2.3・7.1）。
///
/// task 4.1 の純関数側は `PlacementConfig` を直に組んで合成したが、本檻は実配置経路
/// （`prepare_ghost_windows_with_work_area`）の出力 `ScopePlacement` で観測する。
///
/// 手計算: バルーン面 0 は native 400×224 で、バルーン軸 5/3 倍して 400×5/3=666.67→**667**。
/// scope 0 は `balloon.alignment,left` ゆえ基本位置の寄与は −balloon_w = −667。
/// これに上の合流値 (643, −295) が載る ⇒ balloon_offset = (−667+643, −295) = **(−24, −295)**。
/// 是正前は (−667+543, −255) = (−124, −255) になり落ちる。
#[test]
fn prepare_merged_offset_reaches_the_resolved_placement() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "120", "144", Some((266, -129)), (100, -40));

        let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(240))
            .expect("合成ゴーストの配置準備は成功する");

        let s0 = &p.placements[0];
        assert_eq!(
            s0.balloon_size.w, 667,
            "バルーン幅は 400×5/3=667（手計算の錨）"
        );
        assert_eq!(s0.balloon_offset, PointPx { x: -24, y: -295 });
        // 恒等式（resolver 出力時点の事後条件）は供給元が増えても保たれる。
        assert_eq!(
            s0.balloon_offset,
            PointPx {
                x: s0.balloon_pos.x - s0.char_pos.x,
                y: s0.balloon_pos.y - s0.char_pos.y
            }
        );
    });
}

/// 拡大率 1 の経路は本仕様の前後で**同一出力**である（要件 2.2／7.5）。
///
/// 是正前と同じ「生値どうしの素の和」(100+266, −40−129) = (366, −169)。
/// **本檻は是正前も通る**——要件 7.5 が求める不動点の錨であって、7.4 の対の片割れでは
/// ない（対は上の 3 本が担う）。
#[test]
fn prepare_at_identity_scale_matches_the_pre_spec_merge() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "96", "96", Some((266, -129)), (100, -40));

        assert_eq!(
            merged_offset(&ghost_root, &balloon_dir, 96),
            Some((366, -169))
        );
    });
}

// -------------------------------------------------------------------------
// 8. 飽和の記録（要件 2.5・9.4・task 4.3 第 3 項）
// -------------------------------------------------------------------------

/// 飽和が**契約の語で記録に出る**こと（要件 2.5）。task 4.1 は「採る値が回り込みでは
/// ない」までを固定したが、記録そのものは踏んでいなかった。
///
/// 検体: k_shell=2/1 に対し `sakura.balloon.offsetx,2147483647`（`i32::MAX`）。
/// 手計算: `i32::MAX × 2` は `i32` 域を超える ⇒ 飽和値は `i32::MAX`。y 軸は
/// −40×2=−80 で飽和しない ⇒ 記録は **x 軸ちょうど 1 件**（軸ごとに 1 行の契約）。
///
/// 合流欄の値も回り込まない: `i32::MAX` に windowposition の +443 が
/// `saturating_add` で足されて `i32::MAX` のまま、y は −80−215=−295。
///
/// 是正前（換算ステップが無い）は生値が素通しするだけで飽和が起きず、記録が 0 件に
/// なって本檻は落ちる（要件 7.4）。
#[test]
fn prepare_records_saturation_with_the_contract_tag() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "120", "144", Some((266, -129)), (i32::MAX, -40));

        let (merged, events) = capture_logs(|| merged_offset(&ghost_root, &balloon_dir, 240));

        assert_eq!(
            merged,
            Some((i32::MAX, -295)),
            "飽和値は回り込まず、合流の加算も saturating である"
        );

        let hit = expect_one(&events, BALLOON_OFFSET_SATURATED_TAG);
        assert_eq!(
            hit.level,
            tracing::Level::WARN,
            "飽和の記録は warn（log-first）"
        );
        assert_eq!(hit.expect_field("scope"), "0");
        assert_eq!(hit.expect_field("axis"), "\"x\"");
        assert_eq!(hit.expect_field("raw"), "2147483647");
        assert_eq!(hit.expect_field("saturated_value"), "2147483647");
    });
}

/// **順序**の檻の第 2 面（task 4.3 第 1 項）: 値の一致ではなく**記録の並び**で順序を
/// 直に観測する。
///
/// 換算ステップは飽和したときだけ記録を出し、`windowposition` の合流ステップは scope
/// ごとに必ず観測点 1 行（[`WINDOWPOSITION_SUPPLY_NEEDLE`]）を出す。両者を同じ捕捉窓で
/// 拾えば、**換算の警告が合流の観測点より前に並ぶ**ことがそのまま呼出順の証拠になる。
///
/// この主張は値の算術に一切依存しないので、丸めや軸の割り当てを将来触っても順序だけを
/// 見張り続ける。是正前は警告そのものが出ないので落ちる。
#[test]
fn prepare_logs_the_conversion_before_the_windowposition_supply_point() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "120", "144", Some((266, -129)), (i32::MAX, -40));

        let (_, events) = capture_logs(|| merged_offset(&ghost_root, &balloon_dir, 240));

        let index_of = |needle: &str| -> usize {
            events
                .iter()
                .position(|e: &LogEvent| e.message().contains(needle))
                .unwrap_or_else(|| panic!("`{needle}` を含むログが無い: {events:?}"))
        };
        let converted = index_of(BALLOON_OFFSET_SATURATED_TAG);
        let merged = index_of(WINDOWPOSITION_SUPPLY_NEEDLE);
        assert!(
            converted < merged,
            "換算（{converted} 番目）は windowposition の合流（{merged} 番目）より前に走る"
        );
    });
}
