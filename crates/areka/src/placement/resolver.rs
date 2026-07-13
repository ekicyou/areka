//! 純粋 resolver: 物理 px 値型と配置規則（P1 bottom 基準・P2 スコープ連鎖・
//! P4 クランプ）。
//!
//! 座標単位契約（design 正本 U1〜U5）に従い、入出力は**すべて物理 px**
//! （論理 DIP・DPI は署名に登場しない）。std＋tracing のみに依存し wintf 型を
//! import しない（DPI パラメタ化単体テストの前提・U5）。
//!
//! 配置規則 P1〜P5（design「placement::resolver」正本）と
//! `virtual_desktop_union`（4.6・DD8）を持つ。座標演算は `saturating_add`/
//! `saturating_sub` で行う（極端値でも debug オーバーフロー panic しない＝
//! 「パニックしない」契約の防波堤。通常入力では通常の加減算と同値）。

use tracing::warn;

use super::config::{Alignment, BalloonSide, PlacementConfig, ScopeConfig};

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
    /// バルーン窓の左上位置（物理 px・P5 の暫定 offset 適用済み・クランプなし）。
    pub balloon_pos: PointPx,
    /// バルーン窓寸（入力の転記）。
    pub balloon_size: SizePx,
    /// `balloon_pos − char_pos`（追従用に配置時確定・物理 px）。
    ///
    /// 恒等式 `balloon_offset ≡ balloon_pos − char_pos` は恒久の事後条件
    /// （design Postconditions）。
    pub balloon_offset: PointPx,
    /// このスコープの解決済みアンカー種別（5 値・単一真実源）。
    ///
    /// `alignment` の cascade 解決結果を `Anchor::from_alignment` で解釈した値
    /// （どの辺を work area の対応辺へ固定するか・`Free`＝アンカー辺なし＝非吸着・
    /// 4.2／DD15）。spawn がキャラ窓 entity へ焼き込み、ドラッグ／リサイズの射影 T
    /// が消費する。二値の吸着フラグ（旧 `bottom_snap`）はこの単一値から
    /// `!anchor.is_free()`（＝`!matches!(anchor, Anchor::Free)`）で導出する
    /// ——二つ目の格納表現を作らない（単一真実源・Req1.6）。
    pub anchor: Anchor,
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
/// - **P3（free・DD10）**: `alignment=Free` のとき原点は **work area 左上**。
///   `char_x = work_area.left + default_x`／`char_y = work_area.top + default_y`。
///   未指定成分は bottom 相当値（X→P2 連鎖値・Y→P1 値）へフォールバック（2.6）。
/// - **P4（クランプ）**: キャラ窓のみ `x ∈ [left, right−w]`・`y ∈ [top, bottom−h]`
///   （DD12・free 含む全 alignment）。窓が work area より大きく区間が逆転する場合は
///   left／top 側を優先。P2 連鎖の基準はクランプ後の実配置（alignment 不問）。
/// - **P5（バルーン暫定 offset・DD7）**: `balloon.alignment=Left`（既定）→
///   `balloon_x = char_x − balloon_w`、`Right` → `balloon_x = char_x + char_w`。
///   `balloon_y = char_y`（上端揃え）。`balloon.offsetx/offsety` があれば加算。
///   **クランプなし**（バルーンは work area 外へ素直にはみ出す）。offset は
///   配置時に確定し以後静的（4.4: 正式規則は balloon 表示系の後続へ委ねる）。
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
        let SizePx { w, h } = input.char_size;

        // P1: bottom 相当の Y＝work area 下端固定・default_y は無視（2.4）。
        //     free の Y 未指定成分のフォールバック先でもある（2.6）
        let bottom_y = work_area.bottom.saturating_sub(h);

        // P2: base_x(0)=right−w0・base_x(n≥1)=char_x(n−1)−w(n−1)（2.9）・
        //     char_x(n)=base_x(n)−defaultx(n).unwrap_or(0)（左方向オフセット・2.10/DD3）。
        //     free の X 未指定成分のフォールバック先でもある（2.6・その場合
        //     default_x は None ゆえオフセット項は 0）
        let base_x = match prev {
            None => work_area.right.saturating_sub(w),
            Some((prev_x, prev_w)) => prev_x.saturating_sub(prev_w),
        };
        let bottom_x = base_x.saturating_sub(sc.default_x.unwrap_or(0));

        // P3: free は work area **左上**原点＋defaultleft/defaulttop（DD10）。
        //     指定成分のみ適用し、未指定成分は bottom 相当値へ（2.6）
        let (x, y) = if sc.alignment == Alignment::Free {
            (
                sc.default_x
                    .map_or(bottom_x, |dx| work_area.left.saturating_add(dx)),
                sc.default_y
                    .map_or(bottom_y, |dy| work_area.top.saturating_add(dy)),
            )
        } else {
            (bottom_x, bottom_y)
        };

        // P4: キャラ窓のみ work area 内へクランプ（DD12・free 含む全 alignment）
        let x = clamp_axis(x, work_area.left, work_area.right.saturating_sub(w));
        let y = clamp_axis(y, work_area.top, work_area.bottom.saturating_sub(h));

        prev = Some((x, w));

        // P5: バルーン暫定 offset（DD7・クランプなし）。left（既定）＝キャラ左隣・
        //     right＝キャラ右隣・上端揃え・balloon.offsetx/offsety があれば加算
        let balloon_base_x = match sc.balloon_alignment {
            BalloonSide::Left => x.saturating_sub(input.balloon_size.w),
            BalloonSide::Right => x.saturating_add(w),
        };
        let (ox, oy) = sc.balloon_offset.unwrap_or((0, 0));
        let balloon_pos = PointPx {
            x: balloon_base_x.saturating_add(ox),
            y: y.saturating_add(oy),
        };

        let char_pos = PointPx { x, y };
        out.push(ScopePlacement {
            scope: input.scope,
            char_pos,
            char_size: input.char_size,
            balloon_pos,
            balloon_size: input.balloon_size,
            // 事後条件: balloon_offset ≡ balloon_pos − char_pos（design Postconditions）
            balloon_offset: PointPx {
                x: balloon_pos.x.saturating_sub(char_pos.x),
                y: balloon_pos.y.saturating_sub(char_pos.y),
            },
            // 4.2/DD15: cascade 解決済み alignment を 5 値アンカーへ解釈（単一真実源・
            // Req1.6）。旧 bottom_snap（二値）はここでは格納せず、使用点で
            // !anchor.is_free() として導出する
            anchor: Anchor::from_alignment(&sc.alignment),
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

/// 全モニタ bounds の和（仮想デスクトップ・物理 px・4.6/DD8）。空入力は `None`
/// （モニタ 0 面に架空の既定矩形を発明しない）。
///
/// M1 では `DragConstraint` を付与しない（DD8: 無制約＝仮想デスクトップ全域
/// ドラッグ可）ため未結線だが、制約を適用する将来の消費側の正規算出規則として
/// ここで提供・テストする（07-05 の単一モニタ誤釘付けの欠陥面を消す規則）。
#[allow(dead_code)] // DD8: M1 は DragConstraint 非付与＝将来の消費側向け正規規則（テストのみが消費）
pub fn virtual_desktop_union(monitor_bounds: &[RectPx]) -> Option<RectPx> {
    monitor_bounds.iter().copied().reduce(|a, b| RectPx {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    })
}

/// 5 値アンカー種別（純粋値・`seriko.alignmenttodesktop` の解決済み解釈結果・4.2）。
///
/// 「シェル座標系のどの辺を work area の対応辺へ固定するか」を表す不変値。
/// wintf/bevy 非依存で `resolver` に在住し（U5・純粋 DPI 檻が wintf 非依存で走る）、
/// 後続で `ScopePlacement.anchor` として spawn へ運ばれ `Anchored(Anchor)` Component
/// として char 窓へ焼き込まれる。射影 T（`project_anchor`）は follow 層が所有する。
#[allow(dead_code)] // scaffold（task 1.1）: ScopePlacement への結線は task 1.2・射影消費は follow（task）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// 上端固定（`y = wa.top`・X 保持）。
    Top,
    /// 下端固定（既定・`y = wa.bottom − h`・X 保持）。既存 `BottomSnapPolicy` の一般化元。
    Bottom,
    /// 左端固定（`x = wa.left`・Y 保持）。
    Left,
    /// 右端固定（`x = wa.right − w`・Y 保持）。
    Right,
    /// アンカー辺なし（position 保持・寸法のみ反映・全方向ドラッグ自由）。
    Free,
}

#[allow(dead_code)] // scaffold（task 1.1）: 結線は task 1.2 以降
impl Anchor {
    /// cascade 解決済み `Alignment` を 5 値アンカーへ解釈する消費写像（4.2）。
    ///
    /// **優先度チェーンの読取り・解決は行わない**（config.rs の領分・Req6.3）。
    /// 既に解決済みの `Alignment` を解釈消費するのみ:
    /// - `Bottom`→`Bottom`・`Free`→`Free`
    /// - `Seam(s)`: `s.trim().to_ascii_lowercase()` で正規化し `"top"`→`Top`・
    ///   `"left"`→`Left`・`"right"`→`Right`・それ以外（未知値）→`Bottom`（フォールバック・
    ///   window-placement DD9「未知は bottom 相当」を継承）＋`warn!`。正規化は防御
    ///   （parsers 側で正規化済み前提だが大小文字・前後空白へ念のため備える）。
    pub fn from_alignment(alignment: &Alignment) -> Anchor {
        match alignment {
            Alignment::Bottom => Anchor::Bottom,
            Alignment::Free => Anchor::Free,
            Alignment::Seam(value) => match value.trim().to_ascii_lowercase().as_str() {
                "top" => Anchor::Top,
                "left" => Anchor::Left,
                "right" => Anchor::Right,
                // 未知アンカー指定は bottom 相当へフォールバック（DD9）＋警告を残す
                _ => {
                    warn!(
                        value = %value,
                        "alignmenttodesktop の未知アンカー指定（bottom 相当で解釈・4.2/DD9）"
                    );
                    Anchor::Bottom
                }
            },
        }
    }

    /// アンカー辺を持たない（＝`Free`）か（格納でなく導出・可読性用述語）。
    /// bottom_snap 等の boolean が要る使用点は `!is_free()` で導出できる。
    pub fn is_free(self) -> bool {
        matches!(self, Anchor::Free)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::config::{
        build_placement_config, Alignment, BalloonSide, PlacementConfig, ScopeConfig,
    };

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
                    assert_eq!(s.balloon_size, b.balloon_size, "dpi={dpi} seam={seam_value}");
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
    // T-R4: free 配置（P3・2.6・DD10）
    // ------------------------------------------------------------------

    /// T-R4: free で `defaultleft`/`defaulttop` 両指定 → work area **左上**原点で
    /// `char_x = left + dx`・`char_y = top + dy`（DD10）。原点非 (0,0) の work area
    /// で left/top 依存を固定する（bottom の right 基準と混同しない檻）。
    #[test]
    fn t_r4_free_applies_default_left_top_from_work_area_origin() {
        for dpi in DPIS {
            let wa = offset_work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let (dx, dy) = (px(120, dpi), px(80, dpi));
            let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Free, Some(dx), Some(dy)))]);

            let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

            assert_eq!(
                out[0].char_pos,
                PointPx {
                    x: wa.left + dx,
                    y: wa.top + dy
                },
                "dpi={dpi}: free は work area 左上原点＋オフセット"
            );
        }
    }

    /// T-R4: free で Y 未指定 → Y のみ bottom 相当（`bottom − h`）へフォールバック
    /// （2.6）。X は左上原点適用のまま。
    #[test]
    fn t_r4_free_unspecified_y_falls_back_to_bottom() {
        for dpi in DPIS {
            let wa = offset_work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let dx = px(120, dpi);
            let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Free, Some(dx), None))]);

            let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

            assert_eq!(
                out[0].char_pos,
                PointPx {
                    x: wa.left + dx,
                    y: wa.bottom - h
                },
                "dpi={dpi}: Y 未指定は bottom 相当"
            );
        }
    }

    /// T-R4: free で X 未指定 → X のみ bottom 相当（P2 連鎖値＝scope0 は右端密着）へ
    /// フォールバック（2.6）。Y は左上原点適用のまま。
    #[test]
    fn t_r4_free_unspecified_x_falls_back_to_bottom_chain() {
        for dpi in DPIS {
            let wa = offset_work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let dy = px(80, dpi);
            let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Free, None, Some(dy)))]);

            let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

            assert_eq!(
                out[0].char_pos,
                PointPx {
                    x: wa.right - w,
                    y: wa.top + dy
                },
                "dpi={dpi}: X 未指定は P2 の bottom 相当値（scope0＝右端密着）"
            );
        }
    }

    /// T-R4 補: free で両成分未指定 → **幾何**（位置・寸法・バルーン）は Bottom と
    /// 完全同一（2.6 の極限）。ただし `anchor` だけは alignment 由来で相違する
    /// （free は幾何が bottom へフォールバックしても全方向移動可＝非吸着＝`Anchor::Free`・
    /// 4.2。task 8.1 で構造体全体一致から幾何一致＋anchor 相違へ更新）。
    #[test]
    fn t_r4_free_both_unspecified_equals_bottom() {
        for dpi in DPIS {
            let wa = offset_work_area(dpi);
            let inputs = [
                input(0, px(400, dpi), px(600, dpi)),
                input(1, px(320, dpi), px(480, dpi)),
            ];
            let free = cfg_of(vec![
                (0, scope_cfg(Alignment::Free, None, None)),
                (1, scope_cfg(Alignment::Free, None, None)),
            ]);
            let bottom = cfg_of(vec![
                (0, scope_cfg(Alignment::Bottom, None, None)),
                (1, scope_cfg(Alignment::Bottom, None, None)),
            ]);

            let out_free = resolve_placement(&free, wa, &inputs);
            let out_bottom = resolve_placement(&bottom, wa, &inputs);
            assert_eq!(out_free.len(), 2, "dpi={dpi}: 空虚一致封じ");
            for (f, b) in out_free.iter().zip(&out_bottom) {
                assert_eq!(f.scope, b.scope, "dpi={dpi}");
                assert_eq!(f.char_pos, b.char_pos, "dpi={dpi}: 全未指定 free ≡ bottom（幾何）");
                assert_eq!(f.char_size, b.char_size, "dpi={dpi}");
                assert_eq!(f.balloon_pos, b.balloon_pos, "dpi={dpi}");
                assert_eq!(f.balloon_size, b.balloon_size, "dpi={dpi}");
                assert_eq!(f.balloon_offset, b.balloon_offset, "dpi={dpi}");
                // アンカー情報だけは alignment 由来で相違（free＝非吸着・4.2）
                assert_eq!(
                    f.anchor,
                    Anchor::Free,
                    "dpi={dpi}: free は幾何フォールバックでも非吸着（Anchor::Free）"
                );
                assert_eq!(b.anchor, Anchor::Bottom, "dpi={dpi}: bottom は吸着（Anchor::Bottom）");
            }
        }
    }

    /// T-R4 補: free もキャラ窓クランプ（P4・DD12）の対象（過大 offset は
    /// work area 右下で停止＝画面内出現の安全弁は alignment を選ばない）。
    #[test]
    fn t_r4_free_is_clamped_into_work_area() {
        for dpi in DPIS {
            let wa = offset_work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let huge = px(40000, dpi);
            let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Free, Some(huge), Some(huge)))]);

            let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

            assert_eq!(
                out[0].char_pos,
                PointPx {
                    x: wa.right - w,
                    y: wa.bottom - h
                },
                "dpi={dpi}: free も P4 クランプで画面内"
            );
        }
    }

    /// T-R4 補: free で配置した scope0 の実位置が後続の P2 連鎖基準になる
    /// （連鎖は alignment を選ばず「前スコープの実配置の左隣」）。
    #[test]
    fn t_r4_free_position_feeds_scope_chain() {
        for dpi in DPIS {
            let wa = offset_work_area(dpi);
            let (w0, h0) = (px(400, dpi), px(600, dpi));
            let (w1, h1) = (px(320, dpi), px(480, dpi));
            let (dx, dy) = (px(800, dpi), px(80, dpi));
            let cfg = cfg_of(vec![
                (0, scope_cfg(Alignment::Free, Some(dx), Some(dy))),
                (1, scope_cfg(Alignment::Bottom, Some(0), None)),
            ]);

            let out = resolve_placement(&cfg, wa, &[input(0, w0, h0), input(1, w1, h1)]);

            let x0 = wa.left + dx;
            assert_eq!(out[0].char_pos.x, x0, "dpi={dpi}");
            assert_eq!(
                out[1].char_pos,
                PointPx {
                    x: x0 - w0,
                    y: wa.bottom - h1
                },
                "dpi={dpi}: base_x(1)=char_x(0)−w0（free 実位置基準）"
            );
        }
    }

    // ------------------------------------------------------------------
    // T-R8: バルーン暫定 offset（P5・4.4・DD7）
    // ------------------------------------------------------------------

    /// バルーン構成つき ScopeConfig（T-R8 用）。
    fn scope_cfg_balloon(
        default_x: Option<i32>,
        balloon_alignment: BalloonSide,
        balloon_offset: Option<(i32, i32)>,
    ) -> ScopeConfig {
        ScopeConfig {
            alignment: Alignment::Bottom,
            default_x,
            default_y: None,
            balloon_alignment,
            balloon_offset,
        }
    }

    /// T-R8: `balloon.alignment=left`（既定） → `balloon_x = char_x − balloon_w`・
    /// `balloon_y = char_y`（上端揃え・DD7）。恒等式も確認。
    #[test]
    fn t_r8_balloon_left_places_left_of_char() {
        for dpi in DPIS {
            let wa = work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let inp = input(0, w, h);
            let bw = inp.balloon_size.w;
            let cfg = cfg_of(vec![(
                0,
                scope_cfg_balloon(Some(px(40, dpi)), BalloonSide::Left, None),
            )]);

            let out = resolve_placement(&cfg, wa, &[inp]);

            let cp = out[0].char_pos;
            assert_eq!(
                out[0].balloon_pos,
                PointPx {
                    x: cp.x - bw,
                    y: cp.y
                },
                "dpi={dpi}: left はキャラ左隣・上端揃え"
            );
            assert_eq!(
                out[0].balloon_offset,
                PointPx { x: -bw, y: 0 },
                "dpi={dpi}: balloon_offset ≡ balloon_pos − char_pos"
            );
        }
    }

    /// T-R8: `balloon.alignment=right` → `balloon_x = char_x + w`（キャラ右隣・DD7）。
    /// scope0 右端密着では `balloon_x = work_area.right`＝work area 外だが、
    /// バルーンはクランプなし（P5）＝そのままの幾何値を返す。
    #[test]
    fn t_r8_balloon_right_places_right_of_char_without_clamp() {
        for dpi in DPIS {
            let wa = work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let cfg = cfg_of(vec![(0, scope_cfg_balloon(Some(0), BalloonSide::Right, None))]);

            let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

            let cp = out[0].char_pos;
            assert_eq!(cp.x, wa.right - w, "dpi={dpi}: 前提＝右端密着");
            assert_eq!(
                out[0].balloon_pos,
                PointPx {
                    x: wa.right,
                    y: cp.y
                },
                "dpi={dpi}: right はキャラ右隣・クランプなしで work area 外可"
            );
            assert_eq!(
                out[0].balloon_offset,
                PointPx { x: w, y: 0 },
                "dpi={dpi}: balloon_offset ≡ balloon_pos − char_pos"
            );
        }
    }

    /// T-R8: `balloon.offsetx/offsety` は alignment 由来の幾何値へ加算（DD7）。
    #[test]
    fn t_r8_balloon_offsetx_offsety_added() {
        for dpi in DPIS {
            let wa = work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let inp = input(0, w, h);
            let bw = inp.balloon_size.w;
            let (ox, oy) = (px(24, dpi), -px(32, dpi));
            let cfg = cfg_of(vec![(
                0,
                scope_cfg_balloon(Some(px(40, dpi)), BalloonSide::Left, Some((ox, oy))),
            )]);

            let out = resolve_placement(&cfg, wa, &[inp]);

            let cp = out[0].char_pos;
            assert_eq!(
                out[0].balloon_pos,
                PointPx {
                    x: cp.x - bw + ox,
                    y: cp.y + oy
                },
                "dpi={dpi}: offsetx/y 加算"
            );
            assert_eq!(
                out[0].balloon_offset,
                PointPx {
                    x: -bw + ox,
                    y: oy
                },
                "dpi={dpi}: balloon_offset ≡ balloon_pos − char_pos"
            );
        }
    }

    /// T-R8 補: バルーンにクランプなし（P5）＝キャラが左端クランプされた状態の
    /// left バルーンは work area 左外（負方向）へ素直にはみ出す。
    #[test]
    fn t_r8_balloon_never_clamped_even_outside_work_area() {
        for dpi in DPIS {
            let wa = offset_work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let inp = input(0, w, h);
            let bw = inp.balloon_size.w;
            // キャラを左端クランプへ追い込む（T-R6 と同型）
            let cfg = cfg_of(vec![(
                0,
                scope_cfg_balloon(Some(px(40000, dpi)), BalloonSide::Left, None),
            )]);

            let out = resolve_placement(&cfg, wa, &[inp]);

            assert_eq!(out[0].char_pos.x, wa.left, "dpi={dpi}: 前提＝左端クランプ");
            assert_eq!(
                out[0].balloon_pos.x,
                wa.left - bw,
                "dpi={dpi}: バルーンは work area 左外でもクランプしない"
            );
        }
    }

    // ------------------------------------------------------------------
    // T-R7: virtual_desktop_union（4.6・DD8）
    // ------------------------------------------------------------------

    /// T-R7: 複数モニタ矩形の和＝各辺の min/max。プライマリ左に負座標モニタ・
    /// 上に高さ違いモニタを置いた 3 面構成で固定する。
    #[test]
    fn t_r7_union_spans_monitors_including_negative_coords() {
        for dpi in DPIS {
            let primary = RectPx {
                left: 0,
                top: 0,
                right: px(1920, dpi),
                bottom: px(1080, dpi),
            };
            // プライマリの左（負座標・少し上へずれた縦位置）
            let left_monitor = RectPx {
                left: -px(1920, dpi),
                top: -px(40, dpi),
                right: 0,
                bottom: px(1040, dpi),
            };
            // プライマリの上（小型・右へ寄せ）
            let top_monitor = RectPx {
                left: px(480, dpi),
                top: -px(720, dpi),
                right: px(480 + 1280, dpi),
                bottom: 0,
            };

            let union = virtual_desktop_union(&[primary, left_monitor, top_monitor]);

            assert_eq!(
                union,
                Some(RectPx {
                    left: -px(1920, dpi),
                    top: -px(720, dpi),
                    right: px(1920, dpi),
                    bottom: px(1080, dpi),
                }),
                "dpi={dpi}: 和＝min(left,top)/max(right,bottom)"
            );
        }
    }

    /// T-R7 補: 単一モニタの和はその矩形そのもの。
    #[test]
    fn t_r7_union_single_monitor_is_identity() {
        for dpi in DPIS {
            let m = work_area(dpi);
            assert_eq!(virtual_desktop_union(&[m]), Some(m), "dpi={dpi}");
        }
    }

    /// T-R7 補: 空入力は `None`（モニタ 0 面に架空の既定矩形を発明しない）。
    #[test]
    fn t_r7_union_empty_input_is_none() {
        assert_eq!(virtual_desktop_union(&[]), None);
    }

    // ------------------------------------------------------------------
    // anchor 伝搬（4.2・DD15 基盤・task 1.2）
    //
    // 旧 `bottom_snap`（二値）伝搬テストを 5 値アンカー検証へ意味を保って差し替え
    // （単一真実源＝`ScopePlacement.anchor`・Req1.6）。二値の吸着フラグは
    // `!anchor.is_free()` で導出可能ゆえ格納しない。
    // ------------------------------------------------------------------

    /// 4.2: cascade 解決済み `alignment` が 5 値アンカーとして `ScopePlacement.anchor`
    /// へ伝搬する（`Bottom`→`Bottom`・`Seam("top"/"left"/"right")`→対応辺・
    /// `Seam(未知)`→`Bottom`（フォールバック）・`Free`→`Free`）。吸着ドラッグ／
    /// リサイズの射影 T（後続 task）が消費する情報伝搬の檻。
    #[test]
    fn anchor_propagates_five_values_from_resolved_alignment() {
        for dpi in DPIS {
            let wa = work_area(dpi);
            let (w, h) = (px(400, dpi), px(600, dpi));
            let cases = [
                (Alignment::Bottom, Anchor::Bottom),
                (Alignment::Seam("top".to_owned()), Anchor::Top),
                (Alignment::Seam("left".to_owned()), Anchor::Left),
                (Alignment::Seam("right".to_owned()), Anchor::Right),
                (Alignment::Seam("unknown-value".to_owned()), Anchor::Bottom),
                (Alignment::Free, Anchor::Free),
            ];
            for (alignment, expected) in cases {
                let cfg = cfg_of(vec![(0, scope_cfg(alignment.clone(), Some(0), None))]);
                let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);
                assert_eq!(
                    out[0].anchor, expected,
                    "dpi={dpi} alignment={alignment:?}: anchor の伝搬"
                );
            }
        }
    }

    /// 4.2 補: 混在スコープでスコープごとに独立に伝搬し、`cfg.scopes` 未収載
    /// スコープは既定 `ScopeConfig`（＝Bottom）ゆえ `Anchor::Bottom`。
    #[test]
    fn anchor_mixed_scopes_and_missing_config_defaults_to_bottom() {
        for dpi in DPIS {
            let wa = work_area(dpi);
            let cfg = cfg_of(vec![
                (0, scope_cfg(Alignment::Bottom, Some(0), None)),
                (
                    1,
                    scope_cfg(Alignment::Free, Some(px(100, dpi)), Some(px(80, dpi))),
                ),
            ]);
            let out = resolve_placement(
                &cfg,
                wa,
                &[
                    input(0, px(400, dpi), px(600, dpi)),
                    input(1, px(320, dpi), px(480, dpi)),
                    input(2, px(200, dpi), px(400, dpi)), // 未収載 → 既定 Bottom
                ],
            );
            assert_eq!(out[0].anchor, Anchor::Bottom, "dpi={dpi}: Bottom → Anchor::Bottom");
            assert_eq!(out[1].anchor, Anchor::Free, "dpi={dpi}: Free → Anchor::Free");
            assert_eq!(
                out[2].anchor,
                Anchor::Bottom,
                "dpi={dpi}: 未収載＝既定 Bottom → Anchor::Bottom"
            );
        }
    }

    /// 4.2 完了条件: **4 層優先度カスケード**（`build_placement_config`）で解決された
    /// 各アンカー設定（bottom・free・未指定＝既定 bottom・混在スコープ）が、
    /// `resolve_placement` を通って対応する 5 値アンカーとして `ScopePlacement.anchor`
    /// へ正しく伝搬する（KV → config → resolver → anchor の実経路）。
    ///
    /// - scope0: shell scope 接頭層 `sakura.seriko.alignmenttodesktop=bottom` が
    ///   ghost scope 接頭層 `sakura.seriko.alignmenttodesktop=free` を**上書き**
    ///   （shell ＞ ghost の層優先で解決）→ `Bottom`
    /// - scope1: shell scope 接頭層 `kero.seriko.alignmenttodesktop=free` → `Free`
    ///   （スコープごとに独立伝搬）
    /// - scope2: `char2.` 接頭キーでスコープ検出のみ・alignment はどの層にも不在
    ///   → 既定 `Bottom`（未指定＝既定）
    ///
    /// cascade 解決自体は config.rs の領分（改変しない・Req6.3）。ここでは解決済み
    /// alignment が anchor として正しく運ばれることのみを固定する。
    #[test]
    fn anchor_propagates_through_build_placement_config_cascade() {
        use std::collections::BTreeMap;

        fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }

        let ghost_kv = kv(&[
            ("kero.name", "エモ"),                        // scope1 検出
            ("sakura.seriko.alignmenttodesktop", "free"), // scope0 の弱層（shell に負ける）
        ]);
        let shell_kv = kv(&[
            ("sakura.seriko.alignmenttodesktop", "bottom"), // scope0 強層（ghost free を上書き）
            ("kero.seriko.alignmenttodesktop", "free"),     // scope1
            ("char2.defaultx", "0"),                        // scope2 検出のみ（alignment 未指定）
        ]);
        let cfg = build_placement_config(&ghost_kv, &shell_kv);

        for dpi in DPIS {
            let wa = work_area(dpi);
            let out = resolve_placement(
                &cfg,
                wa,
                &[
                    input(0, px(400, dpi), px(600, dpi)),
                    input(1, px(320, dpi), px(480, dpi)),
                    input(2, px(200, dpi), px(400, dpi)),
                ],
            );
            assert_eq!(out.len(), 3, "dpi={dpi}: 3 スコープ解決（空虚一致封じ）");
            assert_eq!(
                out[0].anchor,
                Anchor::Bottom,
                "dpi={dpi}: scope0 は shell 強層 bottom が ghost 弱層 free を上書き → Anchor::Bottom"
            );
            assert_eq!(
                out[1].anchor,
                Anchor::Free,
                "dpi={dpi}: scope1 接頭層 free → Anchor::Free（スコープ独立伝搬）"
            );
            assert_eq!(
                out[2].anchor,
                Anchor::Bottom,
                "dpi={dpi}: scope2 alignment どの層にも不在＝既定 → Anchor::Bottom"
            );
        }
    }

    // ------------------------------------------------------------------
    // 事後条件（design Postconditions・恒久不変条件）
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

    // ------------------------------------------------------------------
    // Anchor::from_alignment（4.2・design Testing Strategy Unit #4）
    // ------------------------------------------------------------------

    /// 4.2: cascade 解決済み `Alignment` の 5 値アンカーへの解釈写像を全分岐固定する。
    /// `Bottom`→`Bottom`・`Free`→`Free`・`Seam("top"/"left"/"right")`→対応値・
    /// 未知 `Seam` →`Bottom`（フォールバック・DD9「未知は bottom 相当」を継承）。
    #[test]
    fn from_alignment_maps_all_branches() {
        assert_eq!(
            Anchor::from_alignment(&Alignment::Bottom),
            Anchor::Bottom,
            "Bottom → Bottom"
        );
        assert_eq!(
            Anchor::from_alignment(&Alignment::Free),
            Anchor::Free,
            "Free → Free"
        );
        assert_eq!(
            Anchor::from_alignment(&Alignment::Seam("top".to_owned())),
            Anchor::Top,
            "Seam(top) → Top"
        );
        assert_eq!(
            Anchor::from_alignment(&Alignment::Seam("left".to_owned())),
            Anchor::Left,
            "Seam(left) → Left"
        );
        assert_eq!(
            Anchor::from_alignment(&Alignment::Seam("right".to_owned())),
            Anchor::Right,
            "Seam(right) → Right"
        );
        assert_eq!(
            Anchor::from_alignment(&Alignment::Seam("unknown-value".to_owned())),
            Anchor::Bottom,
            "Seam(未知) → Bottom（フォールバック）"
        );
    }

    /// 4.2 補: `Seam` 値は `trim().to_ascii_lowercase()` で正規化してから解釈する
    /// （parsers 側で正規化済み前提だが防御・design Implementation Notes Risks）。
    #[test]
    fn from_alignment_normalizes_case_and_whitespace() {
        assert_eq!(
            Anchor::from_alignment(&Alignment::Seam(" TOP ".to_owned())),
            Anchor::Top,
            "前後空白＋大文字も Top へ"
        );
        assert_eq!(
            Anchor::from_alignment(&Alignment::Seam("Right".to_owned())),
            Anchor::Right,
            "大文字混じりも Right へ"
        );
    }
}
