//! 供給層の換算ステップ（[`super::apply_author_balloon_offset_scale`]）の決定論テスト。
//!
//! ここが固定するのは**換算そのもの**——作者空間の生値が合流欄へ入る前にシェル軸の
//! 拡大率で物理 px へ置き換わること、拡大率 1 では従来と同一であること、未宣言は
//! `None` のまま残ること、飽和が回り込まないこと。
//!
//! `prepare_stages_for_shell` の**呼出順**そのものと、飽和が実際に警告として出ること、および
//! 合流後の値を実配置経路で観測することは、下の「供給結線」の群が持つ。

use areka_emo_compose::ScaleRatio;

use super::config::{PlacementConfig, ScopeConfig};
use super::*;
use crate::shared_test_support::{cfg_of, k};

/// scope 1 つだけを持つ最小の `PlacementConfig`。`offset` をそのまま合流欄へ置く。
fn cfg_with_offset(offset: Option<(i32, i32)>) -> PlacementConfig {
    cfg_of(vec![(
        0,
        ScopeConfig {
            balloon_offset: offset,
            ..ScopeConfig::default()
        },
    )])
}

/// 換算ステップを scope 0 に対して 1 度走らせ、合流欄の値を返す。
fn scaled(offset: Option<(i32, i32)>, k: ScaleRatio) -> Option<(i32, i32)> {
    let mut cfg = cfg_with_offset(offset);
    apply_author_balloon_offset_scale(&mut cfg, &[0], k);
    cfg.scopes[&0].balloon_offset
}

// -------------------------------------------------------------------------
// 1. 換算そのもの（シェル軸）
// -------------------------------------------------------------------------

/// 拡大率 ≠ 1 のとき、宣言された生値は**換算後の値**として合流欄に載る。
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

/// 符号は換算で保存される——大きさだけが k 倍される（`scale_signed` へ委譲）。
#[test]
fn conversion_preserves_sign_on_both_axes() {
    assert_eq!(scaled(Some((-8, 8)), k(2, 1)), Some((-16, 16)));
    assert_eq!(scaled(Some((8, -8)), k(2, 1)), Some((16, -16)));
    assert_eq!(scaled(Some((0, 0)), k(2, 1)), Some((0, 0)));
}

// -------------------------------------------------------------------------
// 2. 拡大率 1
// -------------------------------------------------------------------------

/// 拡大率 1 では合流欄が**換算を入れる前（生値のまま）と bit 同一**である。
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

/// **拡大率 1 では生値が素通しになる、という原則の唯一の例外**。
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
// 3. 受理規約は不変
// -------------------------------------------------------------------------

/// 未宣言（＝`config.rs` の `resolve_scope` が `None` を出す腕）は
/// 換算ステップを素通りし、`None` のまま残る。
///
/// どの軸を採るか（片軸だけなら書かれない軸は 0）は `config.rs` の所有であり、本ステップは
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
// 4. 飽和
// -------------------------------------------------------------------------

/// `i32` 域を超えた軸は**回り込ませず飽和値**（`±i32::MAX`）を採る。
///
/// 飽和が警告として記録されることの固定は下の「供給結線」の群の所有。ここが押さえるのは
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

// =========================================================================
// 供給結線（`prepare_stages_for_shell` の実経路）
//
// ここから下が固定するのは**結線**である——⑴ 換算ステップが `windowposition` の
// 合流より**前**に走ること、⑵ 合流後の値が両供給元を同一空間で加算した結果で
// あること、⑶ 飽和が契約の語で**記録に出る**こと。
//
// 上の純関数の側の檻は換算ステップだけを見ており、合流も本番の呼出順も記録の発生も
// 踏んでいない。本群は `prepare_stages_for_shell`（＝`scale_then_merge` で
// `apply_author_balloon_offset_scale` と `merge_scope_windowpositions` を並べている当の関数）
// を回して踏む。
//
// # シェル軸とバルーン軸を**異なる値**にする方法（フィクスチャ到達性の但し書き）
//
// 実フィクスチャ（emo2）は `seriko.dpi` も balloon `dpi` も宣言しないため 2 軸は
// つねに一致する。ゆえに本群は既存の合成検体
// [`synth_declared_dpi_ghost`]（`placement_prepare_tests.rs`／
// `placement_windowposition_tests.rs` が既に使っている口）へ `seriko.dpi,120`／
// balloon `dpi,144` を宣言させ、primary=240 を与えて **k_shell=2/1・k_balloon=5/3** を
// 作る。これは**合成による構築**であって現行フィクスチャから到達する状態ではない
// ——決定論で判定できる分岐は決定論で踏み、実機任せにしない。
// =========================================================================

use std::fs;
use std::path::{Path, PathBuf};

use super::resolver::PointPx;
use super::shared_test_support::{TempDir, WINDOWPOSITION_SUPPLY_NEEDLE};
use crate::sample_test_support::{balloon_root, emo2_root};
use crate::test_support::{WA, synth_declared_dpi_ghost, with_com_initialized};
use log_capture_kit::{CapturedEvent, ExpectField, capture, expect_one};

/// 作者オフセット宣言つきの合成ゴーストを組む（[`synth_declared_dpi_ghost`] の薄い延長）。
///
/// 既存の口は `seriko.dpi`／balloon `dpi`／balloon `windowposition` までしか宣言
/// できないので、shell の `descript.txt` へ `sakura.balloon.offsetx`／`offsety` を
/// **追記**する。`sakura.` 接頭辞ゆえ宣言が効くのは scope 0 だけであり、scope 1
/// （`kero.`）は未宣言＝`None` のまま残る。
fn synth_ghost_with_author_offset(
    root: &TempDir,
    shell_dpi: &str,
    balloon_dpi: &str,
    windowposition: Option<(i32, i32)>,
    offset: (i32, i32),
) -> (PathBuf, PathBuf) {
    let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(
        root.path(),
        &emo2_root(),
        &balloon_root(),
        shell_dpi,
        balloon_dpi,
        windowposition,
    );
    let shell_descript = ghost_root.join("shell").join("master").join("descript.txt");
    let mut text = fs::read_to_string(&shell_descript).expect("合成 shell descript を読む");
    text.push_str(&format!(
        "sakura.balloon.offsetx,{}\nsakura.balloon.offsety,{}\n",
        offset.0, offset.1
    ));
    fs::write(&shell_descript, text).expect("合成 shell descript へ作者オフセットを追記");
    (ghost_root, balloon_dir)
}

/// scope 0 の合流欄を `prepare_stages_for_shell` の出力から取り出す。
fn merged_offset(ghost_root: &Path, balloon_dir: &Path, primary_dpi: u32) -> Option<(i32, i32)> {
    let stages = prepare_stages_for_shell(ghost_root, balloon_dir, Some(primary_dpi), None)
        .expect("合成ゴーストの準備は成功する");
    stages.cfg.scopes[&0].balloon_offset
}

// -------------------------------------------------------------------------
// 6. 語彙（綴りの檻）
// -------------------------------------------------------------------------

/// 飽和の記録語の綴りを**逐語で**固定する（`balloon_limit.rs`／`balloon_drag.rs` の
/// `*_TAG` と同じ流儀）。
///
/// 実機ログの grep と決定論テストが同じ定数を参照する以上、綴りは契約で
/// ある。定数どうしを突き合わせる（＝自分自身と等しいことを主張する）と何も証明
/// しないので、**literal に対して**固定する。
#[test]
fn balloon_offset_saturated_tag_is_spelled_verbatim() {
    assert_eq!(BALLOON_OFFSET_SATURATED_TAG, "[balloon-offset] Saturated");
}

// -------------------------------------------------------------------------
// 7. 合流後の値
// -------------------------------------------------------------------------

/// **本番経路**（`prepare_stages_for_shell`）で、`descript` 由来と `windowposition` 由来が
/// **それぞれの軸**で物理 px へ換算されたうえで加算されること。
///
/// 検体: `seriko.dpi,120`／balloon `dpi,144`／primary=240 ⇒ k_shell=2/1・k_balloon=5/3。
///
/// 手計算（丸め権威 `ScaleRatio::scale_len`＝round half away from zero）:
/// - descript (100, −40) × シェル 2/1 = (200, −80)
/// - windowposition (266, −129) × バルーン 5/3 = (1330/3=443.33→443, 645/3=215 ちょうど→−215)
/// - 合流 = (200+443, −80−215) = **(643, −295)**
///
/// scope 1 は `kero.balloon.offsetx` を宣言していないので `windowposition` 由来だけが
/// 載る＝換算対象の有無で結果が分かれることも同時に固定する。
///
/// 合流後の値は**配置解決の出力まで届く**。上の純関数の側の檻は
/// `PlacementConfig` を直に組んで合成したが、ここは同じ準備を解いた出力 `ScopePlacement`
/// （`prepare_ghost_windows_with_work_area` と同じ `resolve`）でも観測する。
///
/// 手計算: バルーン面 0 は native 400×224 で、バルーン軸 5/3 倍して 400×5/3=666.67→**667**。
/// scope 0 は `balloon.alignment,left` ゆえ基本位置の寄与は −balloon_w = −667。
/// これに上の合流値 (643, −295) が載る ⇒ balloon_offset = (−667+643, −295) = **(−24, −295)**。
#[test]
fn prepare_merges_each_supply_in_its_own_axis_and_reaches_the_placement() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "120", "144", Some((266, -129)), (100, -40));

        let stages = prepare_stages_for_shell(&ghost_root, &balloon_dir, Some(240), None)
            .expect("合成ゴーストの準備");

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

        let p = stages.resolve(WA);

        let s0 = &p.placements[0];
        assert_eq!(
            s0.balloon_size.w, 667,
            "バルーン幅は 400×5/3=667（手計算の錨）"
        );
        assert_eq!(s0.balloon_offset, PointPx { x: -24, y: -295 });
    });
}

/// 拡大率 1 の経路は換算を入れる前と後で**同一出力**である。
///
/// 換算を入れる前と同じ「生値どうしの素の和」(100+266, −40−129) = (366, −169)。
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
// 8. 飽和の記録
// -------------------------------------------------------------------------

/// 飽和が**契約の語で記録に出る**こと。上の純関数の側の檻は「採る値が回り込みでは
/// ない」までを固定したが、記録そのものは踏んでいなかった。
///
/// 検体: k_shell=2/1 に対し `sakura.balloon.offsetx,2147483647`（`i32::MAX`）。
/// 手計算: `i32::MAX × 2` は `i32` 域を超える ⇒ 飽和値は `i32::MAX`。y 軸は
/// −40×2=−80 で飽和しない ⇒ 記録は **x 軸ちょうど 1 件**（軸ごとに 1 行の契約）。
///
/// 合流欄の値も回り込まない: `i32::MAX` に windowposition の +443 が
/// `saturating_add` で足されて `i32::MAX` のまま、y は −80−215=−295。
///
/// 同じ記録で**順序**も見る: 値の一致ではなく**記録の並び**で順序を直に観測する。
/// 換算ステップは飽和したときだけ記録を出し、`windowposition` の合流ステップは scope
/// ごとに必ず観測点 1 行（`WINDOWPOSITION_SUPPLY_NEEDLE`）を出す。この行を**合流ステップが
/// 走った時刻の目印**として使う。両者を同じ捕捉窓で拾えば、**換算の警告が合流の観測点より
/// 前に並ぶ**ことがそのまま呼出順の証拠になる。この主張は値の算術に一切依存しないので、
/// 丸めや軸の割り当てを将来触っても順序だけを見張り続ける。
#[test]
fn prepare_records_saturation_with_the_contract_tag_before_the_supply_point() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "120", "144", Some((266, -129)), (i32::MAX, -40));

        let (merged, events) = capture(|| merged_offset(&ghost_root, &balloon_dir, 240));

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

        let index_of = |needle: &str| -> usize {
            events
                .iter()
                .position(|e: &CapturedEvent| e.message().contains(needle))
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

// -------------------------------------------------------------------------
// 9. descript の分の欄
// -------------------------------------------------------------------------

/// どの鍵（`sakura.`・`kero.`・別名 `char0.`/`char1.`。`config.rs` は無印を読まない）もどれも同じ欄 `balloon_offset_descript_x` に入り、
/// 換算の後は換算値になる。y だけの宣言・未宣言は 0。
#[test]
fn descript_field_takes_the_scaled_value_from_every_key_form() {
    let build = |pairs: &[(&str, &str)]| {
        let shell = areka_parsers::kv::parse_kv(
            &pairs
                .iter()
                .map(|(k, v)| format!("{k},{v}\n"))
                .collect::<String>(),
        );
        let mut cfg = super::config::build_placement_config(&Default::default(), &shell);
        let ids: Vec<usize> = cfg.scopes.keys().copied().collect();
        let raw: Vec<i32> = ids
            .iter()
            .map(|i| cfg.scopes[i].balloon_offset_descript_x)
            .collect();
        apply_author_balloon_offset_scale(&mut cfg, &ids, k(2, 1));
        let scaled: Vec<i32> = ids
            .iter()
            .map(|i| cfg.scopes[i].balloon_offset_descript_x)
            .collect();
        (raw, scaled)
    };
    // 生値は解く時点で入り、換算後は換算値（k=2/1）。
    assert_eq!(
        build(&[
            ("sakura.balloon.offsetx", "30"),
            ("kero.balloon.offsetx", "-7")
        ]),
        (vec![30, -7], vec![60, -14])
    );
    assert_eq!(
        build(&[
            ("char0.balloon.offsetx", "30"),
            ("kero.name", "k"),
            ("char1.balloon.offsetx", "30")
        ]),
        (vec![30, 30], vec![60, 60])
    );
    assert_eq!(
        build(&[("sakura.balloon.offsety", "5")]),
        (vec![0], vec![0])
    );
}

/// 合流の後も欄は descript の分の換算値のままで、`windowposition.x` の分を含まない。
/// 検体は上の合流の檻と同じ: descript 100×2=200・windowposition 443 ⇒ 合流 643 だが欄は 200。
#[test]
fn descript_field_excludes_the_windowposition_part_after_merge() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_ghost_with_author_offset(&root, "120", "144", Some((266, -129)), (100, -40));
        let stages = prepare_stages_for_shell(&ghost_root, &balloon_dir, Some(240), None)
            .expect("合成ゴーストの準備");
        assert_eq!(stages.cfg.scopes[&0].balloon_offset, Some((643, -295)));
        assert_eq!(stages.cfg.scopes[&0].balloon_offset_descript_x, 200);
        assert_eq!(stages.cfg.scopes[&1].balloon_offset_descript_x, 0);
    });
}
