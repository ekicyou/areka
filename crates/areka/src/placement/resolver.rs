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
///   `base_x(n≥1) = char_x(n−1) − w(n)`（**自スコープの幅**を引く＝隣接・隣接ペアの
///   隙間 0・scg 2.1/2.2）。
///   `char_x(n) = base_x(n) − default_x(n).unwrap_or(0)`（左方向オフセット・
///   0＝基準密着・2.10・DD3）。連鎖の `char_x(n−1)` は **P4 クランプ後**の実配置
///   （後続スコープは前スコープの実際の位置の左隣に置く）。
///   なお旧 `window-placement` R2.9（前スコープの幅を引く）は本仕様
///   areka-P0-scope-chain-gap で上書きされた（`doc/COMPAT_ARCHITECTURE.md` §8 参照）。
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
pub fn resolve_placement(
    cfg: &PlacementConfig,
    work_area: RectPx,
    scopes: &[ScopeInput],
) -> Vec<ScopePlacement> {
    let default_scope_cfg = ScopeConfig::default();
    let mut out = Vec::with_capacity(scopes.len());
    // P2 連鎖の前スコープ状態: クランプ後 char_x のみ（自スコープ幅を引く是正式では
    // 前スコープの幅が不要・scg 2.1/2.2）
    let mut prev: Option<i32> = None;

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

        // P2: base_x(0)=right−w0・base_x(n≥1)=char_x(n−1)−w(n)（自スコープの幅＝
        //     隣接・隙間 0・scg 2.1/2.2）・
        //     char_x(n)=base_x(n)−defaultx(n).unwrap_or(0)（左方向オフセット・2.10/DD3）。
        //     free の X 未指定成分のフォールバック先でもある（2.6・その場合
        //     default_x は None ゆえオフセット項は 0）
        let base_x = match prev {
            None => work_area.right.saturating_sub(w),
            Some(prev_x) => prev_x.saturating_sub(w),
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

        prev = Some(x);

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
#[path = "resolver_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "resolver_resolve_tests.rs"]
mod resolve_tests;
#[cfg(test)]
#[path = "resolver_union_tests.rs"]
mod union_tests;
#[cfg(test)]
#[path = "resolver_from_alignment_tests.rs"]
mod from_alignment_tests;
