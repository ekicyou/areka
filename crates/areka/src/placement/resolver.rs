//! 純粋 resolver: 物理 px 値型と配置規則（P1 bottom 基準・P2 スコープ連鎖・
//! P4 クランプ）。
//!
//! 座標単位契約（design 正本 U1〜U5）に従い、入出力は**すべて物理 px**
//! （論理 DIP・DPI は署名に登場しない）。std＋tracing のみに依存し wintf 型を
//! import しない（DPI パラメタ化単体テストの前提・U5）。
//!
//! task 3.1 の範囲は P1／P2／P4（＋Seam=Bottom 同一出力・DD9）。
//! P3（free の defaultleft/defaulttop 適用・DD10）・P5（バルーン暫定 offset・DD7）・
//! `virtual_desktop_union`（4.6・DD8）は task 3.2 で実装する。

use tracing::warn;

use super::config::{Alignment, PlacementConfig, ScopeConfig};

/// 物理 px の矩形（スクリーン座標系・wintf 非依存）。
#[allow(dead_code)] // scaffold（task 3.1）: main.rs シーム（task 6）が結線するまで非テストビルドでは未使用
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectPx {
    /// 左端（物理 px）。
    pub left: i32,
    /// 上端（物理 px）。
    pub top: i32,
    /// 右端（物理 px・排他側）。
    pub right: i32,
    /// 下端（物理 px・排他側）。
    pub bottom: i32,
}

/// 物理 px の点（スクリーン座標系）。
#[allow(dead_code)] // scaffold（task 3.1）: 結線は task 6
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointPx {
    /// X 座標（物理 px）。
    pub x: i32,
    /// Y 座標（物理 px）。
    pub y: i32,
}

/// 物理 px の寸法。
#[allow(dead_code)] // scaffold（task 3.1）: 結線は task 6
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizePx {
    /// 幅（物理 px）。
    pub w: i32,
    /// 高さ（物理 px）。
    pub h: i32,
}

/// スコープ 1 体ぶんの採寸入力（物理 px）。
#[allow(dead_code)] // scaffold（task 3.1）: 結線は task 6
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeInput {
    /// スコープ番号（0=本体・1=相方・…）。
    pub scope: usize,
    /// キャラ surface の原寸（物理 px・emo 採寸由来）。
    pub char_size: SizePx,
    /// バルーン surface の原寸（物理 px）。
    pub balloon_size: SizePx,
}

/// 解決済み配置（物理 px・スクリーン座標）。
#[allow(dead_code)] // scaffold（task 3.1）: 結線は task 6
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopePlacement {
    /// スコープ番号（入力の転記）。
    pub scope: usize,
    /// キャラ窓の左上位置（物理 px・P4 クランプ後）。
    pub char_pos: PointPx,
    /// キャラ窓寸（入力の転記）。
    pub char_size: SizePx,
    /// バルーン窓の左上位置（物理 px）。
    ///
    /// P5（`balloon.alignment` 由来の暫定 offset・DD7）は task 3.2 で実装する。
    /// それまでは `char_pos` と同値（offset ゼロの暫定・意味論の先取りはしない）。
    pub balloon_pos: PointPx,
    /// バルーン窓寸（入力の転記）。
    pub balloon_size: SizePx,
    /// `balloon_pos − char_pos`（追従用に配置時確定・物理 px）。
    ///
    /// 恒等式 `balloon_offset ≡ balloon_pos − char_pos` は恒久の事後条件
    /// （design Postconditions）。task 3.1 時点の値は暫定ゼロ。
    pub balloon_offset: PointPx,
}

/// 既定位置解決（純粋関数・パニックしない・入力順のまま返す・出力長＝入力長）。
///
/// 配置規則（design「placement::resolver」正本）:
/// - **P1（Y・bottom）**: `alignment=Bottom|Seam(_)` のとき `y = work_area.bottom − h`。
///   `default_y`（defaulttop/defaulty）は無視（2.4）。
/// - **P2（X・bottom・連鎖基準）**: `base_x(0) = work_area.right − w(0)`、
///   `base_x(n≥1) = char_x(n−1) − w(n−1)`（2.9）。
///   `char_x(n) = base_x(n) − default_x(n).unwrap_or(0)`（左方向オフセット・
///   0＝基準密着・2.10・DD3）。連鎖の `char_x(n−1)` は **P4 クランプ後**の実配置
///   （後続スコープは前スコープの実際の位置の左隣に置く）。
/// - **P4（クランプ）**: キャラ窓のみ `x ∈ [left, right−w]`・`y ∈ [top, bottom−h]`
///   （DD12）。窓が work area より大きく区間が逆転する場合は left／top 側を優先。
/// - **P3（free・DD10）は task 3.2**: それまで `Alignment::Free` は未指定成分
///   フォールバック（2.6）と同じ bottom 相当で配置する（emo2 は free 未使用）。
/// - **Seam の warn**: `Alignment::Seam` の警告ログは config 側から本関数
///   （シーム値を実際に消費する層）へ委ねられている（config.rs の Alignment doc・
///   DD9）。tracing への `warn!` は I/O を持たない決定論的な副チャネルであり、
///   `config::parse_i32` の `warn!` と同じリポジトリ規約に従う。挙動出力は
///   Bottom と同一（T-R5 で固定）。
///
/// `scopes` 入力に `cfg.scopes` 未収載のスコープ番号が来た場合は
/// `ScopeConfig::default()`（＝Bottom・オフセットなし）で配置する（2.2 の既定と
/// 同じ意味論・テストで固定）。
#[allow(dead_code)] // scaffold（task 3.1）: 結線は task 6
pub fn resolve_placement(
    cfg: &PlacementConfig,
    work_area: RectPx,
    scopes: &[ScopeInput],
) -> Vec<ScopePlacement> {
    let default_scope_cfg = ScopeConfig::default();
    let mut out = Vec::with_capacity(scopes.len());
    // P2 連鎖の前スコープ状態: (クランプ後 char_x, char 幅)
    let mut prev: Option<(i32, i32)> = None;

    for input in scopes {
        let sc = cfg.scopes.get(&input.scope).unwrap_or(&default_scope_cfg);

        // Seam の warn はシーム値を消費する本層で発する（config.rs から委任・DD9）
        if let Alignment::Seam(value) = &sc.alignment {
            warn!(
                scope = input.scope,
                value = %value,
                "alignmenttodesktop の未使用値（bottom として配置・2.8/DD9）"
            );
        }
        // Alignment::Free の P3（DD10）は task 3.2。それまでは Bottom|Seam と
        // 同じ bottom 相当（未指定成分フォールバック・2.6）で配置する。

        let SizePx { w, h } = input.char_size;

        // P1: Y は work area 下端固定・default_y（defaulttop/defaulty）は無視（2.4）
        let y = work_area.bottom - h;

        // P2: base_x(0)=right−w0・base_x(n≥1)=char_x(n−1)−w(n−1)（2.9）・
        //     char_x(n)=base_x(n)−defaultx(n).unwrap_or(0)（左方向オフセット・2.10/DD3）
        let base_x = match prev {
            None => work_area.right - w,
            Some((prev_x, prev_w)) => prev_x - prev_w,
        };
        let x = base_x - sc.default_x.unwrap_or(0);

        // P4: キャラ窓のみ work area 内へクランプ（DD12）
        let x = clamp_axis(x, work_area.left, work_area.right - w);
        let y = clamp_axis(y, work_area.top, work_area.bottom - h);

        prev = Some((x, w));

        let char_pos = PointPx { x, y };
        out.push(ScopePlacement {
            scope: input.scope,
            char_pos,
            char_size: input.char_size,
            // P5（balloon.alignment 由来の暫定 offset・DD7）は task 3.2。
            // それまでは offset ゼロの暫定（恒等式 balloon_offset≡balloon_pos−char_pos 維持）。
            balloon_pos: char_pos,
            balloon_size: input.balloon_size,
            balloon_offset: PointPx { x: 0, y: 0 },
        });
    }

    out
}

/// 1 軸クランプ（`lo ≤ v ≤ hi`）。窓が work area より大きく `hi < lo` に逆転する
/// 場合は `lo`（left／top）側を優先する（DD12「画面内に正しく出現」の安全弁）。
/// `i32::clamp` は逆転区間で panic するため使わない（resolver は panic しない契約）。
fn clamp_axis(v: i32, lo: i32, hi: i32) -> i32 {
    v.min(hi).max(lo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::config::{Alignment, BalloonSide, PlacementConfig, ScopeConfig};

    /// DPI パラメタ水準（3.4・U5）。
    const DPIS: [i32; 4] = [96, 120, 144, 192];

    /// 論理基準値 → 各 DPI の物理 px（整数演算のみ・厳密整除を強制）。
    ///
    /// resolver 自体は物理 px しか見ない（U1）。テストは「同じ論理形状を
    /// 各 DPI の物理値で与えたとき、期待値も同じ物理式から出る」ことを固定する
    /// （隠れた `/96` 変換があれば 96 以外の水準で崩れる＝07-05 欠陥の檻）。
    fn px(logical: i32, dpi: i32) -> i32 {
        assert_eq!(
            (logical * dpi) % 96,
            0,
            "テスト入力は厳密整除になる論理値（4 の倍数）で構築する"
        );
        logical * dpi / 96
    }

    /// プライマリモニタ相当の work area（左上原点・物理 px）。
    fn work_area(dpi: i32) -> RectPx {
        RectPx {
            left: 0,
            top: 0,
            right: px(1920, dpi),
            bottom: px(1080, dpi),
        }
    }

    /// 左上が原点でない work area（クランプ境界の left/top 依存を暴く）。
    fn offset_work_area(dpi: i32) -> RectPx {
        RectPx {
            left: px(64, dpi),
            top: px(40, dpi),
            right: px(64 + 1920, dpi),
            bottom: px(40 + 1080, dpi),
        }
    }

    fn scope_cfg(
        alignment: Alignment,
        default_x: Option<i32>,
        default_y: Option<i32>,
    ) -> ScopeConfig {
        ScopeConfig {
            alignment,
            default_x,
            default_y,
            balloon_alignment: BalloonSide::Left,
            balloon_offset: None,
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

    fn input(scope: usize, w: i32, h: i32) -> ScopeInput {
        ScopeInput {
            scope,
            char_size: SizePx { w, h },
            balloon_size: SizePx { w: w / 2, h: h / 2 },
        }
    }

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
    // T-R2: スコープ連鎖（P2・2.9/2.10・DD3）
    // ------------------------------------------------------------------

    /// T-R2: `base_x(n≥1) = char_x(n−1) − w(n−1)`・`kero.defaultx=0` は基準密着で
    /// あって右端に戻らない（DD3 の檻）。3 スコープで一般連鎖も固定する。
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
                x0 - w0,
                "dpi={dpi}: base_x(1)=char_x(0)−w0・defaultx=0＝密着（2.9）"
            );
            assert_ne!(
                out[1].char_pos.x,
                wa.right - w1,
                "dpi={dpi}: kero.defaultx=0 が右端に戻ってはならない（DD3）"
            );
            assert_eq!(
                out[2].char_pos.x,
                (x0 - w0) - w1,
                "dpi={dpi}: base_x(2)=char_x(1)−w1（一般連鎖）"
            );
            // Y は各スコープの h で独立に bottom 基準
            assert_eq!(out[1].char_pos.y, wa.bottom - h1, "dpi={dpi}");
            assert_eq!(out[2].char_pos.y, wa.bottom - h2, "dpi={dpi}");
        }
    }

    /// T-R2 補: 後続スコープの `defaultx` は「自スコープの基準位置（前スコープの
    /// 左隣）からの左方向オフセット」（DD3）。
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
                x0 - w0 - dx1,
                "dpi={dpi}: char_x(1)=base_x(1)−defaultx(1)"
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
    // T-R5: シーム値＝bottom 同一出力（2.8・DD9）
    // ------------------------------------------------------------------

    /// T-R5: `Alignment::Seam(値)` は値によらず Bottom と完全同一の出力（DD9）。
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

            for seam_value in ["top", "left", "right", "unknown-value"] {
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
                assert_eq!(
                    resolve_placement(&seam, wa, &inputs),
                    expected,
                    "dpi={dpi} seam={seam_value}: Seam は Bottom と同一出力"
                );
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
    /// クランプ後の実配置の左隣＝P2 連鎖は実位置基準）。
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
            // base_x(1) = char_x(0) − w0 は左外 → scope1 も左端クランプ
            assert_eq!(out[1].char_pos.x, wa.left, "dpi={dpi}: 連鎖先もクランプ");
        }
    }

    // ------------------------------------------------------------------
    // 事後条件（design Postconditions・恒久不変条件のみ。P5 の意味論は task 3.2）
    // ------------------------------------------------------------------

    /// 事後条件: 出力長＝入力長・入力順保存・寸法転記・
    /// `balloon_offset ≡ balloon_pos − char_pos`（恒等式は task 3.2 以降も恒久）。
    #[test]
    fn postconditions_order_length_and_offset_identity() {
        for dpi in DPIS {
            let wa = work_area(dpi);
            let inputs = [
                input(0, px(400, dpi), px(600, dpi)),
                input(1, px(320, dpi), px(480, dpi)),
            ];
            let cfg = cfg_of(vec![
                (0, scope_cfg(Alignment::Bottom, Some(0), None)),
                (1, scope_cfg(Alignment::Bottom, Some(0), None)),
            ]);

            let out = resolve_placement(&cfg, wa, &inputs);

            assert_eq!(out.len(), inputs.len(), "dpi={dpi}: 出力長＝入力長");
            for (o, i) in out.iter().zip(&inputs) {
                assert_eq!(o.scope, i.scope, "dpi={dpi}: 入力順保存");
                assert_eq!(o.char_size, i.char_size, "dpi={dpi}");
                assert_eq!(o.balloon_size, i.balloon_size, "dpi={dpi}");
                assert_eq!(
                    o.balloon_offset,
                    PointPx {
                        x: o.balloon_pos.x - o.char_pos.x,
                        y: o.balloon_pos.y - o.char_pos.y
                    },
                    "dpi={dpi}: balloon_offset ≡ balloon_pos − char_pos"
                );
            }
        }
    }
}
