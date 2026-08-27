//! P5 キーワード幾何の決定論檻（areka-P0-windowposition-limit・task 2.3）。
//!
//! 対象は `windowposition.x` のキーワード語彙が定めるバルーン**基本位置**
//! （`BalloonXMode::CenterTop`／`CenterBottom`・要件 4.2/4.3）と、その周辺の
//! 回帰境界（`Side`＝数値指定・未指定の bit 同一維持・要件 4.5/5.2）である。
//!
//! `resolver_resolve_tests.rs` は既に 1,000 行規模ゆえ、本仕様の追加分は別ファイルへ
//! 置く（ファイル分割規約）。既存の T-R8 群（`Side` の全表）は**無改変のまま**
//! 緑であることが要件 5.2 の主檻であり、本ファイルはその補強として `Side` の
//! 閉じた式を逐語で再検定する。
//!
//! DPI パラメタ化（`DPIS`＝100%/125%/150%/200%）は既存 resolver 檻の流儀を踏襲する
//! ——resolver は物理 px しか見ないので、同じ論理形状を各水準の物理値で与えたとき
//! 期待値も同一の物理式から出る（隠れた `/96` があれば 96 以外で崩れる・要件 4.8）。

use super::test_support::{DPIS, px, work_area};
use super::*;
use crate::placement::config::{
    Alignment, BalloonSide, BalloonXMode, PlacementConfig, ScopeConfig,
};
use crate::placement::shared_test_support::MEASURE_DPI;

/// 左上が原点でない work area（基本位置式の left/top 依存を暴く）。
fn offset_work_area(dpi: i32) -> RectPx {
    RectPx {
        left: px(64, dpi),
        top: px(40, dpi),
        right: px(64 + 1920, dpi),
        bottom: px(40 + 1080, dpi),
    }
}

/// 本ファイル共通のスコープ構成（`ScopeConfig` の全欄を明示列挙する——今後欄が
/// 増えたときここが必ずコンパイルエラーになり、既定が黙って入るのを防ぐ）。
fn keyword_cfg(
    balloon_x_mode: BalloonXMode,
    balloon_alignment: BalloonSide,
    balloon_offset: Option<(i32, i32)>,
    balloon_limit: bool,
) -> ScopeConfig {
    ScopeConfig {
        alignment: Alignment::Bottom,
        default_x: Some(0),
        default_y: None,
        balloon_alignment,
        balloon_offset,
        balloon_limit,
        balloon_x_mode,
    }
}

fn cfg_of(scopes: Vec<(usize, ScopeConfig)>) -> PlacementConfig {
    PlacementConfig {
        scopes: scopes.into_iter().collect(),
        zorder_raw: None,
        sticky_window_raw: None,
        shell_dpi_raw: None,
    }
}

/// キャラ寸とバルーン寸を独立に与える採寸入力（中央揃えの検定には
/// `balloon_w ≠ char_w / 2` が要る＝`resolver_resolve_tests` の `input` は使えない）。
fn input(scope: usize, char_size: SizePx, balloon_size: SizePx) -> ScopeInput {
    ScopeInput {
        scope,
        char_size,
        balloon_size,
    }
}

/// 本ファイル既定の寸法組（論理値・`px()` の厳密整除条件を満たす 4 の倍数）。
/// `char_w − balloon_w = 176` は偶数ゆえ中点が厳密に割り切れる（切り捨ての検定は
/// [`t_k5_center_midpoint_is_integer_division_toward_zero`] が別途担う）。
const CHAR_W: i32 = 400;
const CHAR_H: i32 = 600;
const BALLOON_W: i32 = 224;
const BALLOON_H: i32 = 160;

fn sizes(dpi: i32) -> (SizePx, SizePx) {
    (
        SizePx {
            w: px(CHAR_W, dpi),
            h: px(CHAR_H, dpi),
        },
        SizePx {
            w: px(BALLOON_W, dpi),
            h: px(BALLOON_H, dpi),
        },
    )
}

/// 1 スコープを解決して `(placement, char_size, balloon_size)` を返す共通路。
fn resolve_one(
    dpi: i32,
    wa: RectPx,
    mode: BalloonXMode,
    side: BalloonSide,
    balloon_offset: Option<(i32, i32)>,
) -> (ScopePlacement, SizePx, SizePx) {
    let (cs, bs) = sizes(dpi);
    let cfg = cfg_of(vec![(0, keyword_cfg(mode, side, balloon_offset, true))]);
    let out = resolve_placement(&cfg, wa, &[input(0, cs, bs)], MEASURE_DPI);
    assert_eq!(out.len(), 1, "dpi={dpi}: 出力長＝入力長（空虚一致封じ）");
    (out[0], cs, bs)
}

// ------------------------------------------------------------------
// T-K1: CenterTop の基本位置（要件 4.2）
// ------------------------------------------------------------------

/// T-K1: `center`/`top`＝シェル画像の中央上。
/// 水平はシェル画像（＝キャラ窓 rect）の中央へバルーンの中央を揃え
/// （`char_x + (char_w − balloon_w) / 2`）、垂直はバルーン下端がシェル画像上端に
/// 接する（`char_y − balloon_h`）。k=1（dpi=96）と k≠1（120/144/192）の全水準、
/// および原点非 (0,0) の work area で成立する。
#[test]
fn t_k1_center_top_places_above_shell_center() {
    for dpi in DPIS {
        for (label, wa) in [
            ("原点 (0,0)", work_area(dpi)),
            ("原点非 (0,0)", offset_work_area(dpi)),
        ] {
            let (p, cs, bs) =
                resolve_one(dpi, wa, BalloonXMode::CenterTop, BalloonSide::Left, None);

            let cp = p.char_pos;
            assert_eq!(
                cp,
                PointPx {
                    x: wa.right - cs.w,
                    y: wa.bottom - cs.h
                },
                "dpi={dpi} [{label}]: 前提＝キャラは右下密着（P1/P2 は不変）"
            );
            assert_eq!(
                p.balloon_pos,
                PointPx {
                    x: cp.x + (cs.w - bs.w) / 2,
                    y: cp.y - bs.h
                },
                "dpi={dpi} [{label}]: CenterTop＝中央 x・下端がシェル上端に接する y"
            );
            // 中央揃えの実体: バルーンの中心 x がシェル画像の中心 x と一致する
            // （偶数差ゆえ厳密一致・奇数差の切り捨ては T-K5 が担う）。
            assert_eq!(
                p.balloon_pos.x + bs.w / 2,
                cp.x + cs.w / 2,
                "dpi={dpi} [{label}]: バルーン中心＝シェル画像中心"
            );
            // 接する: バルーン下端 ＝ シェル画像上端
            assert_eq!(
                p.balloon_pos.y + bs.h,
                cp.y,
                "dpi={dpi} [{label}]: バルーン下端＝シェル画像上端（接する）"
            );
            assert_eq!(
                p.balloon_offset,
                PointPx {
                    x: (cs.w - bs.w) / 2,
                    y: -bs.h
                },
                "dpi={dpi} [{label}]: balloon_offset ≡ balloon_pos − char_pos"
            );
        }
    }
}

// ------------------------------------------------------------------
// T-K2: CenterBottom の基本位置（要件 4.3）
// ------------------------------------------------------------------

/// T-K2: `bottom`＝シェル画像の中央下。水平は CenterTop と同一式、垂直は
/// バルーン上端がシェル画像下端に接する（`char_y + char_h`）。
#[test]
fn t_k2_center_bottom_places_below_shell_center() {
    for dpi in DPIS {
        for (label, wa) in [
            ("原点 (0,0)", work_area(dpi)),
            ("原点非 (0,0)", offset_work_area(dpi)),
        ] {
            let (p, cs, bs) =
                resolve_one(dpi, wa, BalloonXMode::CenterBottom, BalloonSide::Left, None);

            let cp = p.char_pos;
            assert_eq!(
                p.balloon_pos,
                PointPx {
                    x: cp.x + (cs.w - bs.w) / 2,
                    y: cp.y + cs.h
                },
                "dpi={dpi} [{label}]: CenterBottom＝中央 x・上端がシェル下端に接する y"
            );
            assert_eq!(
                p.balloon_pos.x + bs.w / 2,
                cp.x + cs.w / 2,
                "dpi={dpi} [{label}]: バルーン中心＝シェル画像中心"
            );
            assert_eq!(
                p.balloon_pos.y,
                cp.y + cs.h,
                "dpi={dpi} [{label}]: バルーン上端＝シェル画像下端（接する）"
            );
            assert_eq!(
                p.balloon_offset,
                PointPx {
                    x: (cs.w - bs.w) / 2,
                    y: cs.h
                },
                "dpi={dpi} [{label}]: balloon_offset ≡ balloon_pos − char_pos"
            );
        }
    }
}

/// T-K1/T-K2 補: 中央上と中央下は水平が同一で垂直だけが異なる
/// （`bottom` が `center` の単なる別名へ退行していないことの判別）。
#[test]
fn t_k12_center_top_and_bottom_differ_only_in_y() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (top, cs, bs) = resolve_one(dpi, wa, BalloonXMode::CenterTop, BalloonSide::Left, None);
        let (bot, _, _) = resolve_one(dpi, wa, BalloonXMode::CenterBottom, BalloonSide::Left, None);

        assert_eq!(
            top.balloon_pos.x, bot.balloon_pos.x,
            "dpi={dpi}: 水平は同一式"
        );
        assert_eq!(
            bot.balloon_pos.y - top.balloon_pos.y,
            cs.h + bs.h,
            "dpi={dpi}: 垂直差＝char_h + balloon_h（上に接する↔下に接する）"
        );
        assert_ne!(
            top.balloon_pos.y, bot.balloon_pos.y,
            "dpi={dpi}: bottom が center の別名へ退行してはならない"
        );
    }
}

// ------------------------------------------------------------------
// T-K3: `y` の数値調整量（要件 4.4）
// ------------------------------------------------------------------

/// T-K3: キーワード指定時も `windowposition.y` の数値は**基本位置への調整量**として
/// 加算される（正負とも）。供給経路は `ScopeConfig.balloon_offset` の `(0, dy)`
/// （キーワード時は数値 x が存在しないため dx=0・design C3(d)）。
#[test]
fn t_k3_numeric_y_is_added_to_keyword_base_position() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
            let (base, _, _) = resolve_one(dpi, wa, mode, BalloonSide::Left, None);
            for dy_logical in [48, -48] {
                let dy = if dy_logical < 0 {
                    -px(-dy_logical, dpi)
                } else {
                    px(dy_logical, dpi)
                };
                let (adj, _, _) = resolve_one(dpi, wa, mode, BalloonSide::Left, Some((0, dy)));

                assert_eq!(
                    adj.balloon_pos,
                    PointPx {
                        x: base.balloon_pos.x,
                        y: base.balloon_pos.y + dy
                    },
                    "dpi={dpi} mode={mode:?} dy={dy}: y 調整量は基本位置へ加算（x は不動）"
                );
                assert_eq!(
                    adj.char_pos, base.char_pos,
                    "dpi={dpi} mode={mode:?}: 調整量はキャラ窓を動かさない"
                );
                assert_eq!(
                    adj.balloon_offset,
                    PointPx {
                        x: base.balloon_offset.x,
                        y: base.balloon_offset.y + dy
                    },
                    "dpi={dpi} mode={mode:?}: offset も同じだけ動く（恒等式維持）"
                );
            }
        }
    }
}

/// T-K3 補: descript の `balloon.offsetx`/`offsety` もキーワード時に従来どおり
/// 加算される（キーワード指定でも offset 加算を継続するのは正典沈黙箇所の
/// areka 裁定・DD8）。x 成分も効く（`(0, dy)` 専用の経路ではない）。
#[test]
fn t_k3_descript_balloon_offset_also_applies_under_keyword() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (ox, oy) = (px(24, dpi), -px(32, dpi));
        for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
            let (base, _, _) = resolve_one(dpi, wa, mode, BalloonSide::Left, None);
            let (with, _, _) = resolve_one(dpi, wa, mode, BalloonSide::Left, Some((ox, oy)));

            assert_eq!(
                with.balloon_pos,
                PointPx {
                    x: base.balloon_pos.x + ox,
                    y: base.balloon_pos.y + oy
                },
                "dpi={dpi} mode={mode:?}: offsetx/offsety は基本位置へ加算"
            );
        }
    }
}

// ------------------------------------------------------------------
// T-K4: キーワードは `balloon.alignment` に依存しない（要件 4.2/4.3）
// ------------------------------------------------------------------

/// T-K4: キーワード指定の基本位置は `balloon.alignment`（left/right）に依存しない
/// ——中央揃えは左右の置き側という概念を持たない。`Side` の左右分岐が
/// キーワード経路へ漏れていれば落ちる。
#[test]
fn t_k4_keyword_geometry_ignores_balloon_side() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
            let (left, _, _) = resolve_one(dpi, wa, mode, BalloonSide::Left, None);
            let (right, _, _) = resolve_one(dpi, wa, mode, BalloonSide::Right, None);

            assert_eq!(
                left.balloon_pos, right.balloon_pos,
                "dpi={dpi} mode={mode:?}: キーワード基本位置は置き側に依存しない"
            );
        }
    }
}

// ------------------------------------------------------------------
// T-K5: 中点は整数除算・0 方向切り捨て（DD8）
// ------------------------------------------------------------------

/// T-K5: `(char_w − balloon_w) / 2` は Rust の整数除算（0 方向切り捨て）であり、
/// 新しい丸め規約ではない（要件 4.8・DD8）。差が奇数の正負両方で固定する。
///
/// 物理 px の直値で組む（奇数寸は `px()` の厳密整除条件を満たさないため——
/// 検定対象は幾何の中点規則であって DPI 変換ではない）。
#[test]
fn t_k5_center_midpoint_is_integer_division_toward_zero() {
    let wa = RectPx {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };
    // (差, キャラ幅, バルーン幅, 期待する中点項)
    let cases = [
        // 正の奇数差 201 → 100（100.5 を 0 方向へ切り捨て）
        (401, 200, 100),
        // 負の奇数差 −201（バルーンがシェルより広い）→ −100（−100.5 を 0 方向へ）
        (200, 401, -100),
        // 正の偶数差 200 → 100（切り捨てが働かない対照）
        (400, 200, 100),
    ];
    for (char_w, balloon_w, expected_half) in cases {
        for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
            let cs = SizePx { w: char_w, h: 600 };
            let bs = SizePx {
                w: balloon_w,
                h: 160,
            };
            let cfg = cfg_of(vec![(0, keyword_cfg(mode, BalloonSide::Left, None, true))]);
            let out = resolve_placement(&cfg, wa, &[input(0, cs, bs)], MEASURE_DPI);
            assert_eq!(out.len(), 1, "空虚一致封じ");

            assert_eq!(
                out[0].balloon_pos.x - out[0].char_pos.x,
                expected_half,
                "char_w={char_w} balloon_w={balloon_w} mode={mode:?}: \
                 中点は整数除算（0 方向切り捨て・DD8）"
            );
        }
    }
}

// ------------------------------------------------------------------
// T-K6: `Side`（数値指定・未指定）の回帰境界（要件 4.5・5.2）
// ------------------------------------------------------------------

/// T-K6: `BalloonXMode::Side` の出力は本仕様適用前の閉じた式と bit 同一である。
/// 適用前の式（P5 原形）を逐語で書き下し、左右 × offset 有無 × 全 DPI 水準で照合する。
///
/// 既存の T-R8 群（`resolver_resolve_tests.rs`・無改変）が同じ表を別角度から
/// 固定しており、本檻はその式そのものを明示する補強である。
#[test]
fn t_k6_side_mode_output_is_bit_identical_to_pre_spec_formula() {
    for dpi in DPIS {
        for wa in [work_area(dpi), offset_work_area(dpi)] {
            for side in [BalloonSide::Left, BalloonSide::Right] {
                for offset in [None, Some((px(24, dpi), -px(32, dpi)))] {
                    let (p, cs, bs) = resolve_one(dpi, wa, BalloonXMode::Side, side, offset);

                    // ---- 本仕様適用前の P5 原形（逐語） ----
                    let cx = p.char_pos.x;
                    let cy = p.char_pos.y;
                    let pre_base_x = match side {
                        BalloonSide::Left => cx - bs.w,
                        BalloonSide::Right => cx + cs.w,
                    };
                    let (ox, oy) = offset.unwrap_or((0, 0));
                    let pre = PointPx {
                        x: pre_base_x + ox,
                        y: cy + oy,
                    };
                    // ---------------------------------------

                    assert_eq!(
                        p.balloon_pos, pre,
                        "dpi={dpi} side={side:?} offset={offset:?}: \
                         Side は本仕様適用前の式と bit 同一（要件 4.5/5.2）"
                    );
                }
            }
        }
    }
}

/// T-K6 補: `ScopeConfig` の既定（＝バルーン定義が読めない scope・未収載 scope）は
/// `Side` であり、キーワード幾何は既定では一切発火しない（要件 4.5 の入口側）。
#[test]
fn t_k6_default_mode_is_side_and_missing_scope_uses_it() {
    assert_eq!(
        ScopeConfig::default().balloon_x_mode,
        BalloonXMode::Side,
        "既定モードは Side（現行の左右分岐と同一挙動）"
    );

    for dpi in DPIS {
        let wa = work_area(dpi);
        let (cs, bs) = sizes(dpi);
        // scopes マップが空 → scope0 は既定 ScopeConfig で解決される
        let out = resolve_placement(&cfg_of(vec![]), wa, &[input(0, cs, bs)], MEASURE_DPI);
        assert_eq!(out.len(), 1, "dpi={dpi}: 空虚一致封じ");

        let cp = out[0].char_pos;
        assert_eq!(
            out[0].balloon_pos,
            PointPx {
                x: cp.x - bs.w,
                y: cp.y
            },
            "dpi={dpi}: 未収載 scope は既定 Side＝left 側・上端揃え（現行挙動）"
        );
    }
}

// ------------------------------------------------------------------
// T-K7: limit の転記（要件 5.1 の運搬部・design C4）
// ------------------------------------------------------------------

/// T-K7: `ScopeConfig.balloon_limit` は `ScopePlacement.balloon_limit` へ**そのまま**
/// 運ばれる（resolver は判定も補正もしない）。未収載 scope は `ScopeConfig::default()`
/// 経由で正典既定 `true`。
#[test]
fn t_k7_balloon_limit_is_transcribed_from_scope_config() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (cs, bs) = sizes(dpi);
        let cfg = cfg_of(vec![
            (
                0,
                keyword_cfg(BalloonXMode::Side, BalloonSide::Left, None, true),
            ),
            (
                1,
                keyword_cfg(BalloonXMode::CenterTop, BalloonSide::Left, None, false),
            ),
        ]);

        let out = resolve_placement(
            &cfg,
            wa,
            &[
                input(0, cs, bs),
                input(1, cs, bs),
                // scope2 は未収載 → ScopeConfig::default()＝正典既定 true
                input(2, cs, bs),
            ],
            MEASURE_DPI,
        );

        assert_eq!(out.len(), 3, "dpi={dpi}: 空虚一致封じ");
        assert!(out[0].balloon_limit, "dpi={dpi}: scope0 は limit 有効");
        assert!(
            !out[1].balloon_limit,
            "dpi={dpi}: scope1 は limit 無効（転記）"
        );
        assert!(
            out[2].balloon_limit,
            "dpi={dpi}: 未収載 scope は正典既定（有効）"
        );
    }
}

/// T-K7 補: limit の値は幾何へ 1 ビットも影響しない（resolver 出力にクランプは
/// 掛からない——補正は下流の関門が所有する）。
#[test]
fn t_k7_balloon_limit_value_does_not_affect_geometry() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (cs, bs) = sizes(dpi);
        for mode in [
            BalloonXMode::Side,
            BalloonXMode::CenterTop,
            BalloonXMode::CenterBottom,
        ] {
            let enabled = resolve_placement(
                &cfg_of(vec![(0, keyword_cfg(mode, BalloonSide::Left, None, true))]),
                wa,
                &[input(0, cs, bs)],
                MEASURE_DPI,
            );
            let disabled = resolve_placement(
                &cfg_of(vec![(0, keyword_cfg(mode, BalloonSide::Left, None, false))]),
                wa,
                &[input(0, cs, bs)],
                MEASURE_DPI,
            );
            assert_eq!(enabled.len(), 1, "dpi={dpi}: 空虚一致封じ");

            assert_eq!(
                enabled[0].char_pos, disabled[0].char_pos,
                "dpi={dpi} mode={mode:?}: limit は char_pos へ影響しない"
            );
            assert_eq!(
                enabled[0].balloon_pos, disabled[0].balloon_pos,
                "dpi={dpi} mode={mode:?}: limit は balloon_pos へ影響しない"
            );
            assert_eq!(
                enabled[0].balloon_offset, disabled[0].balloon_offset,
                "dpi={dpi} mode={mode:?}: limit は balloon_offset へ影響しない"
            );
        }
    }
}

// ------------------------------------------------------------------
// T-K9: 実表示寸確定時の再導出の素材（要件 4.7・2026-08-14 実機是正）
// ------------------------------------------------------------------

/// T-K9: `balloon_keyword_base` は「キーワード指定か否か」を**そのまま**運ぶ。
///
/// `Side` は `None`（数値指定・未指定の scope では再導出が構造的に起きない＝4.5/5.2）、
/// キーワードは `Some((mode, adjust))`（`adjust` は `windowposition.y` の数値と
/// `balloon.offsetx/offsety` が合流した作者指定の調整量・4.4）。
#[test]
fn t_k9_keyword_base_carries_mode_and_adjust_only_for_keyword_modes() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        for offset in [None, Some((px(24, dpi), -px(32, dpi)))] {
            let (side, _, _) = resolve_one(dpi, wa, BalloonXMode::Side, BalloonSide::Left, offset);
            assert_eq!(
                side.balloon_keyword_base, None,
                "dpi={dpi} offset={offset:?}: Side が再導出の素材を持っている（4.5/5.2）"
            );

            for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
                let (p, _, _) = resolve_one(dpi, wa, mode, BalloonSide::Left, offset);
                let (ox, oy) = offset.unwrap_or((0, 0));
                assert_eq!(
                    p.balloon_keyword_base,
                    Some((mode, PointPx { x: ox, y: oy })),
                    "dpi={dpi} mode={mode:?} offset={offset:?}: 素材が mode と調整量を運んでいない"
                );
            }
        }
    }
}

/// T-K9 補: 素材から導いた offset は、同じ寸法では P5 の出力と**完全に一致**する。
///
/// 再導出は「別の寸法で同じ式を引き直す」ものであって、別の幾何を持ち込むものではない
/// ——同一入力で結果が割れれば、切り出した純関数が P5 から乖離した証拠になる。
#[test]
fn t_k9_keyword_base_reproduces_the_p5_offset_for_the_same_sizes() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        for offset in [None, Some((px(24, dpi), -px(32, dpi)))] {
            for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
                let (p, cs, bs) = resolve_one(dpi, wa, mode, BalloonSide::Left, offset);
                let (m, adjust) = p.balloon_keyword_base.expect("キーワードは素材を持つ");

                // 原点 (0,0) 起点で引くと戻り値がそのまま char 左上相対 offset になる。
                let derived = keyword_balloon_pos(m, PointPx { x: 0, y: 0 }, cs, bs, adjust)
                    .expect("キーワードモードは基本位置を持つ");
                assert_eq!(
                    derived, p.balloon_offset,
                    "dpi={dpi} mode={mode:?} offset={offset:?}: \
                     素材から引き直した offset が P5 の出力と一致しない（式が乖離した）"
                );
            }
        }
    }
}

/// T-K9 補: `Side` は素材を持たないので、共有した純関数も `None` を返す
/// （数値指定の分岐がキーワード式へ流れ込む経路が構造的に無い）。
#[test]
fn t_k9_shared_keyword_formula_returns_none_for_side() {
    let cs = SizePx { w: 400, h: 600 };
    let bs = SizePx { w: 224, h: 160 };
    assert_eq!(
        keyword_balloon_pos(
            BalloonXMode::Side,
            PointPx { x: 100, y: 200 },
            cs,
            bs,
            PointPx { x: 7, y: -7 }
        ),
        None,
        "Side がキーワード式で位置を返している（4.5/5.2 の構造保証が壊れている）"
    );
}

// ------------------------------------------------------------------
// T-K8: 事後条件はモードによらず成立（design C4）
// ------------------------------------------------------------------

/// T-K8: `balloon_offset ≡ balloon_pos − char_pos` は全モード × 調整量有無で成立する
/// （キーワード分岐が恒等式を壊さない・design C4「計算式は不変」）。
#[test]
fn t_k8_offset_identity_holds_for_every_mode() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        for mode in [
            BalloonXMode::Side,
            BalloonXMode::CenterTop,
            BalloonXMode::CenterBottom,
        ] {
            for side in [BalloonSide::Left, BalloonSide::Right] {
                for offset in [
                    None,
                    Some((0, px(48, dpi))),
                    Some((px(24, dpi), -px(32, dpi))),
                ] {
                    let (p, _, _) = resolve_one(dpi, wa, mode, side, offset);
                    assert_eq!(
                        p.balloon_offset,
                        PointPx {
                            x: p.balloon_pos.x - p.char_pos.x,
                            y: p.balloon_pos.y - p.char_pos.y
                        },
                        "dpi={dpi} mode={mode:?} side={side:?} offset={offset:?}: \
                         balloon_offset ≡ balloon_pos − char_pos"
                    );
                }
            }
        }
    }
}
