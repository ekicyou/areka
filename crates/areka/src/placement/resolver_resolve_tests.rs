//! `resolve_placement` の檻: bottom アンカー・スコープ連鎖・クランプ（T-R1/R2/R3/R5/R6）。
//!
//! 元は 1 ファイルに 4 主題が同居して 1,000 行規約を超えていたため主題別に分割した。
//! 他の 3 主題は `resolver_resolve_{free,balloon,contract}_tests.rs`。

use super::resolve_test_support::{cfg_of, input, offset_work_area, scope_cfg};
use super::test_support::{DPIS, px, work_area};
use super::*;
use crate::placement::config::Alignment;

// ------------------------------------------------------------------
// T-R1: bottom 右下基準（P1＋P2 の scope0 項）
// ------------------------------------------------------------------

/// T-R1: `char_y = bottom − h`・`char_x(0) = right − w0 − defaultx`（2.1/2.2/2.5/2.10）。
/// dpi 全 4 水準で成立する（純物理式・隠れたスケールなし）。
#[test]
fn t_r1_bottom_anchors_bottom_right() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h, dx) = (px(400, dpi), px(600, dpi), px(40, dpi));
        let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Bottom, Some(dx), None))]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

        assert_eq!(out.len(), 1, "dpi={dpi}: 出力長＝入力長");
        assert_eq!(out[0].scope, 0);
        assert_eq!(
            out[0].char_pos,
            PointPx {
                x: wa.right - w - dx,
                y: wa.bottom - h
            },
            "dpi={dpi}: bottom 右下基準"
        );
        assert_eq!(out[0].char_size, SizePx { w, h }, "dpi={dpi}: 寸法は転記");
    }
}

/// T-R1 補: 原点が (0,0) でない work area でも右下基準式が成立する
/// （`right`/`bottom` 由来であり `width`/`height` 由来ではない）。
#[test]
fn t_r1_bottom_holds_on_offset_work_area() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Bottom, Some(0), None))]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

        assert_eq!(
            out[0].char_pos,
            PointPx {
                x: wa.right - w,
                y: wa.bottom - h
            },
            "dpi={dpi}"
        );
    }
}

/// T-R1 補: `defaultx` 未指定（None）は 0 扱い＝右端密着（2.10）。
/// `cfg.scopes` 未収載スコープも `ScopeConfig::default()`（Bottom・オフセット
/// なし）で同一に配置される（doc 記載の決定の檻）。
#[test]
fn t_r1_missing_scope_config_defaults_to_bottom_flush() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));

        // scopes マップが完全に空 → scope0 は既定 ScopeConfig で解決
        let out = resolve_placement(&cfg_of(vec![]), wa, &[input(0, w, h)]);

        assert_eq!(out.len(), 1, "dpi={dpi}");
        assert_eq!(
            out[0].char_pos,
            PointPx {
                x: wa.right - w,
                y: wa.bottom - h
            },
            "dpi={dpi}: 既定＝bottom・defaultx なし＝右端密着"
        );
    }
}

// ------------------------------------------------------------------
// T-R2: スコープ連鎖（P2・scg 2.1/2.2・2.10・DD3）
// ------------------------------------------------------------------

/// T-R2: `base_x(n≥1) = char_x(n−1) − w(n)`（隣接・gap 0・scg 2.1/2.2）。
/// `kero.defaultx=0` は基準密着であって右端に戻らない（DD3 の檻）。
/// 3 スコープで一般連鎖と、隣接ペアの隙間が 0 であることを固定する。
#[test]
fn t_r2_scope_chain_defaultx_zero_stays_adjacent() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w0, h0) = (px(400, dpi), px(600, dpi));
        let (w1, h1) = (px(320, dpi), px(480, dpi));
        let (w2, h2) = (px(200, dpi), px(400, dpi));
        let cfg = cfg_of(vec![
            (0, scope_cfg(Alignment::Bottom, Some(0), None)),
            (1, scope_cfg(Alignment::Bottom, Some(0), None)),
            (2, scope_cfg(Alignment::Bottom, Some(0), None)),
        ]);

        let out = resolve_placement(
            &cfg,
            wa,
            &[input(0, w0, h0), input(1, w1, h1), input(2, w2, h2)],
        );

        assert_eq!(out.len(), 3, "dpi={dpi}");
        let x0 = wa.right - w0;
        assert_eq!(out[0].char_pos.x, x0, "dpi={dpi}: scope0 右端密着");
        assert_eq!(
            out[1].char_pos.x,
            x0 - w1,
            "dpi={dpi}: base_x(1)=char_x(0)−w1・defaultx=0＝隣接・gap 0（scg 2.1/2.2）"
        );
        assert_ne!(
            out[1].char_pos.x,
            wa.right - w1,
            "dpi={dpi}: kero.defaultx=0 が右端に戻ってはならない（DD3）"
        );
        assert_ne!(
            out[1].char_pos.x,
            x0 - w0,
            "dpi={dpi}: 前スコープ幅を引く旧式へ戻ってはならない（不等幅でのみ判別可・scg 2.1/2.2）"
        );
        assert_eq!(
            out[2].char_pos.x,
            (x0 - w1) - w2,
            "dpi={dpi}: base_x(2)=char_x(1)−w2（一般連鎖・scg 2.1/2.2）"
        );
        // 隣接ペアの隙間はいずれも 0（scope n の右端＝scope n−1 の左端・scg 2.2）
        assert_eq!(
            out[0].char_pos.x - (out[1].char_pos.x + w1),
            0,
            "dpi={dpi}: scope0/scope1 の隙間 0（scg 2.1/2.2）"
        );
        assert_eq!(
            out[1].char_pos.x - (out[2].char_pos.x + w2),
            0,
            "dpi={dpi}: scope1/scope2 の隙間 0（scg 2.1/2.2）"
        );
        // Y は各スコープの h で独立に bottom 基準
        assert_eq!(out[1].char_pos.y, wa.bottom - h1, "dpi={dpi}");
        assert_eq!(out[2].char_pos.y, wa.bottom - h2, "dpi={dpi}");
    }
}

/// T-R2 本丸: 幅が等しいか否かで**結果の形が分岐しない**（隣接・gap 0・scg 2.1/2.2）。
///
/// 確定規則（SSP 実測 H1）は `char_x(0) = right − w(0)`・
/// `char_x(n≥1) = char_x(n−1) − w(n)`。引くのは**自スコープの幅**ゆえ、隣接ペアの
/// 隙間は幅の組み合わせによらず常に 0 になる（幅差が隙間へ漏れない・scg 2.2）。
///
/// 本檻の要点は、不等幅（400/320/200）と等幅（320/320）を**同一のヘルパ**
/// （同一式・同一 assert 形）で検定することにある（scg 2.5＝等幅を特殊扱いしない）。
/// 期待値は幅列からの畳み込みで生成し、等幅側に手書き定数を置かない
/// （偶然一致する定数では「同一式で配置されている」ことを主張できない）。
/// `DPIS`＝100%／125%／150%／200% の全 4 水準で決定論的に実行する（scg 2.4/3.5）。
#[test]
fn t_r2_unequal_widths_leave_no_gap() {
    /// 幅列を確定規則で検定する**唯一の検定路**（不等幅・等幅で分岐しない）。
    /// 検定するのは P2（キャラ窓 X の連鎖）のみで、P5（バルーン）は混ぜない。
    fn assert_chain_is_adjacent(dpi: i32, label: &str, widths: &[i32]) -> Vec<ScopePlacement> {
        let wa = work_area(dpi);
        let cfg = cfg_of(
            (0..widths.len())
                .map(|s| (s, scope_cfg(Alignment::Bottom, Some(0), None)))
                .collect(),
        );
        let inputs: Vec<ScopeInput> = widths
            .iter()
            .enumerate()
            .map(|(s, &w)| input(s, w, px(600, dpi) - s as i32 * px(40, dpi)))
            .collect();

        let out = resolve_placement(&cfg, wa, &inputs);
        assert_eq!(
            out.len(),
            widths.len(),
            "dpi={dpi} [{label}]: 出力長＝入力長（空虚一致封じ）"
        );

        // 期待 X は確定規則の畳み込みで生成する（等幅・不等幅で同一式・scg 2.5）
        let mut expected_x = wa.right;
        for (n, &w) in widths.iter().enumerate() {
            expected_x -= w;
            assert_eq!(
                out[n].char_pos.x, expected_x,
                "dpi={dpi} [{label}]: scope{n} は char_x(n)=char_x(n−1)−w(n)（隣接・gap 0（scg 2.1/2.2））"
            );
        }
        // 全隣接ペアの隙間 0（scope n の右端＝scope n−1 の左端）
        for n in 1..widths.len() {
            assert_eq!(
                out[n - 1].char_pos.x - (out[n].char_pos.x + widths[n]),
                0,
                "dpi={dpi} [{label}]: scope{}/scope{n} の隙間 0（隣接・gap 0（scg 2.1/2.2））",
                n - 1
            );
        }
        out
    }

    for dpi in DPIS {
        let wa = work_area(dpi);

        // (1) 不等幅 3 スコープ: 幅差（w(n−1)−w(n)）が隙間へ漏れない（scg 2.2/3.1/3.2）
        let unequal = [px(400, dpi), px(320, dpi), px(200, dpi)];
        let out_unequal = assert_chain_is_adjacent(dpi, "不等幅 400/320/200", &unequal);
        // 欠陥式（前スコープの幅を引く）は不等幅でのみ判別できる（scg 3.2）
        let x0 = wa.right - unequal[0];
        assert_ne!(
            out_unequal[1].char_pos.x,
            x0 - unequal[0],
            "dpi={dpi}: 前スコープ幅を引く旧式へ戻ってはならない（隣接・gap 0（scg 2.1/2.2））"
        );

        // (2) 等幅 2 スコープ: (1) と**同一のヘルパ＝同一式・同一 assert 形**で検定する
        //     （scg 2.5＝等幅を特殊扱いしない）
        let equal = [px(320, dpi), px(320, dpi)];
        let out_equal = assert_chain_is_adjacent(dpi, "等幅 320/320", &equal);
        // 等幅では是正式（自幅を引く）と欠陥式（前スコープ幅を引く）が数値的に一致する。
        // すなわち等幅入力では本欠陥を構造的に観測できない——不等幅入力を必須とする理由
        // そのものであり、(1) の assert_ne! を等幅側へ置けない根拠（scg 3.2）。
        assert_eq!(
            out_equal[1].char_pos.x,
            (wa.right - equal[0]) - equal[0],
            "dpi={dpi}: 等幅では是正式と欠陥式が一致（判別には不等幅入力が要る・scg 3.2）"
        );
    }
}

/// T-R2 補: 後続スコープの `defaultx` は「自スコープの基準位置（前スコープの
/// 左隣＝`char_x(n−1) − w(n)`）からの左方向オフセット」（DD3・scg 2.1/2.2）。
#[test]
fn t_r2_chain_defaultx_offsets_leftward_from_base() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w0, h0) = (px(400, dpi), px(600, dpi));
        let (w1, h1) = (px(320, dpi), px(480, dpi));
        let (dx0, dx1) = (px(16, dpi), px(48, dpi));
        let cfg = cfg_of(vec![
            (0, scope_cfg(Alignment::Bottom, Some(dx0), None)),
            (1, scope_cfg(Alignment::Bottom, Some(dx1), None)),
        ]);

        let out = resolve_placement(&cfg, wa, &[input(0, w0, h0), input(1, w1, h1)]);

        let x0 = wa.right - w0 - dx0;
        assert_eq!(out[0].char_pos.x, x0, "dpi={dpi}");
        assert_eq!(
            out[1].char_pos.x,
            x0 - w1 - dx1,
            "dpi={dpi}: char_x(1)=base_x(1)−defaultx(1)（base_x(1)=char_x(0)−w1・scg 2.1/2.2）"
        );
    }
}

// ------------------------------------------------------------------
// T-R3: defaulttop 無視（P1・2.4）
// ------------------------------------------------------------------

/// T-R3: bottom 時に `default_y` を与えても出力は不変（2.4）。
#[test]
fn t_r3_default_y_ignored_under_bottom() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h, dx) = (px(400, dpi), px(600, dpi), px(40, dpi));
        let with_y = cfg_of(vec![(
            0,
            scope_cfg(Alignment::Bottom, Some(dx), Some(px(100, dpi))),
        )]);
        let without_y = cfg_of(vec![(0, scope_cfg(Alignment::Bottom, Some(dx), None))]);

        let out_with = resolve_placement(&with_y, wa, &[input(0, w, h)]);
        let out_without = resolve_placement(&without_y, wa, &[input(0, w, h)]);

        assert_eq!(out_with, out_without, "dpi={dpi}: defaulttop は完全無視");
        assert_eq!(
            out_with[0].char_pos.y,
            wa.bottom - h,
            "dpi={dpi}: Y は work area 下端固定"
        );
    }
}

// ------------------------------------------------------------------
// T-R5: シーム値＝bottom 同一幾何出力（2.8・DD9）
// ------------------------------------------------------------------

/// T-R5: `Alignment::Seam(値)` は値によらず Bottom と同一**幾何**出力
/// （位置・寸法・バルーン・DD9＝挙動出力不変）。ただし `anchor` は 5 値解釈で
/// 相違する（`top`/`left`/`right`→対応辺・未知値→`Bottom` フォールバック・4.2）。
/// 旧版は構造体全体一致だったが、`bottom_snap:bool`→`anchor:Anchor` のフィールド化
/// （task 1.2）で幾何一致＋anchor 別検証へ意味を保って更新した。
#[test]
fn t_r5_seam_output_identical_to_bottom() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w0, h0) = (px(400, dpi), px(600, dpi));
        let (w1, h1) = (px(320, dpi), px(480, dpi));
        let dx = px(40, dpi);
        let inputs = [input(0, w0, h0), input(1, w1, h1)];

        let bottom = cfg_of(vec![
            (0, scope_cfg(Alignment::Bottom, Some(dx), None)),
            (1, scope_cfg(Alignment::Bottom, Some(0), None)),
        ]);
        let expected = resolve_placement(&bottom, wa, &inputs);
        // 空 Vec 同士の空虚一致（RED スタブで観測）を封じる
        assert_eq!(expected.len(), 2, "dpi={dpi}: 比較基準が空では無意味");

        for (seam_value, expected_anchor) in [
            ("top", Anchor::Top),
            ("left", Anchor::Left),
            ("right", Anchor::Right),
            ("unknown-value", Anchor::Bottom),
        ] {
            let seam = cfg_of(vec![
                (
                    0,
                    scope_cfg(Alignment::Seam(seam_value.to_owned()), Some(dx), None),
                ),
                (
                    1,
                    scope_cfg(Alignment::Seam(seam_value.to_owned()), Some(0), None),
                ),
            ]);
            let out = resolve_placement(&seam, wa, &inputs);
            assert_eq!(out.len(), 2, "dpi={dpi} seam={seam_value}");
            for (s, b) in out.iter().zip(&expected) {
                // 幾何（位置・寸法・バルーン）は Bottom と同一（DD9・挙動出力不変）
                assert_eq!(s.scope, b.scope, "dpi={dpi} seam={seam_value}");
                assert_eq!(
                    s.char_pos, b.char_pos,
                    "dpi={dpi} seam={seam_value}: Seam の char_pos は Bottom と同一"
                );
                assert_eq!(s.char_size, b.char_size, "dpi={dpi} seam={seam_value}");
                assert_eq!(
                    s.balloon_pos, b.balloon_pos,
                    "dpi={dpi} seam={seam_value}: Seam の balloon_pos は Bottom と同一"
                );
                assert_eq!(
                    s.balloon_size, b.balloon_size,
                    "dpi={dpi} seam={seam_value}"
                );
                assert_eq!(
                    s.balloon_offset, b.balloon_offset,
                    "dpi={dpi} seam={seam_value}"
                );
                // anchor は 5 値解釈で相違（top/left/right→対応辺・未知→Bottom・4.2）
                assert_eq!(
                    s.anchor, expected_anchor,
                    "dpi={dpi} seam={seam_value}: anchor は 5 値解釈で解決"
                );
            }
        }
    }
}

// ------------------------------------------------------------------
// T-R6: クランプ（P4・DD12）
// ------------------------------------------------------------------

/// T-R6: 過大 `defaultx` で `x = work_area.left` に止まる（左端クランプ）。
/// 原点非 (0,0) の work area で `left` 依存を固定する。
#[test]
fn t_r6_oversized_defaultx_clamps_to_left_edge() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let cfg = cfg_of(vec![(
            0,
            scope_cfg(Alignment::Bottom, Some(px(4000, dpi)), None),
        )]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

        assert_eq!(out[0].char_pos.x, wa.left, "dpi={dpi}: 左端で停止");
        assert_eq!(out[0].char_pos.y, wa.bottom - h, "dpi={dpi}: Y は不干渉");
    }
}

/// T-R6 補: 負の過大 `defaultx`（右方向へのはみ出し）は `right − w` に止まる。
#[test]
fn t_r6_negative_defaultx_clamps_to_right_edge() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let cfg = cfg_of(vec![(
            0,
            scope_cfg(Alignment::Bottom, Some(-px(4000, dpi)), None),
        )]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

        assert_eq!(out[0].char_pos.x, wa.right - w, "dpi={dpi}: 右端で停止");
    }
}

/// T-R6 補: work area より大きい surface 寸では区間が逆転するため
/// left／top 側を優先し、少なくとも左上は work area 内に収まる（DD12）。
#[test]
fn t_r6_oversized_surface_pins_to_top_left() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        // 幅・高さとも work area を超過
        let (w, h) = (px(2400, dpi), px(1600, dpi));
        let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Bottom, Some(0), None))]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

        assert_eq!(
            out[0].char_pos,
            PointPx {
                x: wa.left,
                y: wa.top
            },
            "dpi={dpi}: 逆転区間は left/top 優先"
        );
    }
}

/// T-R6 補: クランプはキャラ窓の連鎖にも波及する（後続スコープは
/// クランプ後の実配置の左隣＝P2 連鎖は実位置基準・2.7）。
///
/// 左端クランプの区間（前段）は両スコープとも左端へ潰れるため、
/// 「クランプ前とクランプ後のどちらを連鎖基準にしたか」を判別できない。
/// 判別には後段の右端クランプ区間が要る（そちらが 2.7 の実効的な檻）。
#[test]
fn t_r6_chain_uses_clamped_previous_position() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w0, h0) = (px(400, dpi), px(600, dpi));
        let (w1, h1) = (px(320, dpi), px(480, dpi));
        // scope0 が過大 defaultx で左端クランプ → scope1 の基準はクランプ後の x0
        let cfg = cfg_of(vec![
            (0, scope_cfg(Alignment::Bottom, Some(px(4000, dpi)), None)),
            (1, scope_cfg(Alignment::Bottom, Some(0), None)),
        ]);

        let out = resolve_placement(&cfg, wa, &[input(0, w0, h0), input(1, w1, h1)]);

        assert_eq!(out[0].char_pos.x, wa.left, "dpi={dpi}");
        // base_x(1) = char_x(0) − w1 は左外 → scope1 も左端クランプ（scg 2.1/2.2）
        assert_eq!(out[1].char_pos.x, wa.left, "dpi={dpi}: 連鎖先もクランプ");

        // ここまでは両基準が左端へ潰れて同値になるため 2.7 を判別しない。
        // 右端クランプ区間を併置して判別力を持たせる: scope0 を free の過大な
        // defaultleft で右端クランプさせると、scope1 はクランプされない位置へ落ち、
        // クランプ前基準なら w0 ぶん右（右端クランプ）へずれて差が出る。
        let cfg = cfg_of(vec![
            (
                0,
                scope_cfg(Alignment::Free, Some(px(2000, dpi)), Some(px(80, dpi))),
            ),
            (1, scope_cfg(Alignment::Bottom, Some(0), None)),
        ]);

        let out = resolve_placement(&cfg, wa, &[input(0, w0, h0), input(1, w1, h1)]);

        let clamped_x0 = wa.right - w0;
        assert_eq!(
            out[0].char_pos.x, clamped_x0,
            "dpi={dpi}: scope0 は free 指定が画面外ゆえ右端クランプ"
        );
        assert_eq!(
            out[1].char_pos.x,
            clamped_x0 - w1,
            "dpi={dpi}: base_x(1)=クランプ後 char_x(0)−w1（2.7・scg 2.1/2.2）"
        );
        // クランプ前の位置（wa.left+2000）を連鎖基準にすると scope1 は右端クランプへ
        // 落ちる。退行すればこの否定 assert が落ちる＝2.7 を判別する檻の本体。
        assert_ne!(
            out[1].char_pos.x,
            wa.right - w1,
            "dpi={dpi}: クランプ前の位置を連鎖基準にしてはならない（2.7）"
        );
        assert_eq!(
            out[0].char_pos.x - (out[1].char_pos.x + w1),
            0,
            "dpi={dpi}: クランプが挟まっても隣接ペアの隙間は 0（scg 2.1/2.2）"
        );
    }
}
