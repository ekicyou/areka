//! 純粋 resolver: 物理 px 値型と配置規則（P1 bottom 基準・P2 スコープ連鎖・
//! P4 クランプ）。
//!
//! 座標単位契約（design 正本 U1〜U5）に従い、入出力は**すべて物理 px**
//! （論理 DIP は持ち込まない）。std＋tracing のみに依存する規律は配置式 P1〜P5 に
//! ついては不変である——**幾何は表示 DPI を 1 度も読まない**。
//!
//! 唯一の例外は追従オフセットの**基準対**（[`ScopePlacement::balloon_offset_base`]・
//! areka-P0-balloon-offset-dpi 要件 3.1／design D15）である。基準対は「この物理 px 値は
//! どの表示 DPI の空間に属するか」という札であり、値型 `wintf::ecs::DPI` を運ぶだけで
//! 配置式の入力にはならない（[`resolve_placement`] は受け取った値を素通しで刻む）。
//! U5 が守ろうとした「幾何が DPI に依存しない」性質はそのまま保たれる。
//!
//! 配置規則 P1〜P5（design「placement::resolver」正本）と
//! `virtual_desktop_union`（4.6・DD8）を持つ。座標演算は `saturating_add`/
//! `saturating_sub` で行う（極端値でも debug オーバーフロー panic しない＝
//! 「パニックしない」契約の防波堤。通常入力では通常の加減算と同値）。

use tracing::warn;
use wintf::ecs::DPI;

use super::config::{Alignment, BalloonSide, BalloonXMode, PlacementConfig, ScopeConfig};
use super::follow::OffsetBase;

/// 物理 px の矩形（スクリーン座標系・wintf 非依存）。
#[allow(dead_code)]
// scaffold（task 3.1）: main.rs シーム（task 6）が結線するまで非テストビルドでは未使用
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
    /// バルーン窓の左上位置（物理 px・P5 の暫定 offset 適用済み）。
    ///
    /// **resolver 自身はクランプしない**——work area 外へ素直にはみ出した値を返す。
    /// `windowposition.limit` による作業領域内への補正は下流の関門が所有する
    /// （起動時＝`main.rs` の `restore_merged_placements` が呼ぶ
    /// `balloon_limit::apply_balloon_limit`／実行時＝
    /// `follow::window_move::enqueue_window_set_pos` の runtime 関門）。
    pub balloon_pos: PointPx,
    /// バルーン窓寸（入力の転記）。
    pub balloon_size: SizePx,
    /// `balloon_pos − char_pos`（追従用に配置時確定・物理 px）。
    ///
    /// 恒等式 `balloon_offset ≡ balloon_pos − char_pos` は **resolver 出力時点の
    /// 事後条件**であって、以降ずっと成り立つ不変量ではない（design Postconditions・
    /// windowposition-limit DD6）。下流の関門を通った後は `balloon_pos` が
    /// 補正後の**表示位置**、`balloon_offset` が補正を焼き付けない**論理相対位置**
    /// （作者指定・保存値の系譜）という役割分離になる。
    pub balloon_offset: PointPx,
    /// 追従オフセットの**基準対**——値と、その値が属する表示 DPI
    /// （areka-P0-balloon-offset-dpi 要件 3.1・design D15）。
    ///
    /// 本関数の出力では常に `OffsetBase { offset: balloon_offset, dpi: Some(採寸 DPI) }`
    /// ——配置式が出した既定のオフセットは、採寸に使った表示 DPI の空間の値だからである。
    /// 保存値を採用した scope だけが未係留（`dpi: None`）になる（`persist::merge_scope`）。
    ///
    /// 基準対は拡大率遷移でオフセットを引き直すための札であり、**配置式の入力ではない**
    /// （P1〜P5 は基準対を 1 度も読まない）。
    pub balloon_offset_base: OffsetBase,
    /// このスコープの `windowposition.limit` 解決値（正典既定 `true`＝画面内へ維持する）。
    ///
    /// resolver は**判定も補正もしない**——`ScopeConfig.balloon_limit` を転記して
    /// 下流（起動時関門・spawn の `BalloonLimit` Component）へ運ぶだけの欄である
    /// （design C4 Responsibilities「resolver は判定せず運ぶだけ」）。
    pub balloon_limit: bool,
    /// このスコープの解決済みアンカー種別（5 値・単一真実源）。
    ///
    /// `alignment` の cascade 解決結果を `Anchor::from_alignment` で解釈した値
    /// （どの辺を work area の対応辺へ固定するか・`Free`＝アンカー辺なし＝非吸着・
    /// 4.2／DD15）。spawn がキャラ窓 entity へ焼き込み、ドラッグ／リサイズの射影 T
    /// が消費する。二値の吸着フラグ（旧 `bottom_snap`）はこの単一値から
    /// `!anchor.is_free()`（＝`!matches!(anchor, Anchor::Free)`）で導出する
    /// ——二つ目の格納表現を作らない（単一真実源・Req1.6）。
    pub anchor: Anchor,
    /// キーワード由来のバルーン基本位置を実表示寸確定時に一度だけ導出し直すための素材。
    ///
    /// `Some((mode, adjust))` は「この scope の `balloon_offset` はキーワード由来の
    /// **初期既定位置**であり、実表示寸が確定したら `mode` で導出し直してよい」を意味する
    /// （`adjust` は作者指定の調整量＝`windowposition.y` の数値と `balloon.offsetx/offsety`
    /// が合流した値・4.4）。`Side`（数値指定・未指定）は `None`——数値指定の分岐は
    /// 1 ビットも変えない（4.5/5.2）。保存値が効いている scope も `None` へ落ちる
    /// （要件 4.7「保存値優先・キーワードの適用は初期既定位置の供給にとどめる」・
    /// `persist::merge_scope`）。要件 4.7 は While 節（状態）ゆえ、**セッション中に**
    /// 保存値が生まれたとき（バルーン単独ドラッグの DragEnd 書込）も同じ観測点で素材が
    /// 退役する（`follow::drag_follow::retire_keyword_base_on_save`）——起動時の規則
    /// だけでは状態条件を満たさない（task 4.6）。
    ///
    /// # なぜ「採寸した寸」ではなく「実表示寸」で導出し直す必要があるのか
    ///
    /// キーワードの中央揃えは `(char_w − balloon_w) / 2` ゆえ、`char_w` が採寸値と
    /// 実表示値でずれるとバルーンがその差の半分だけ横へずれる。実機（2026-08-14 の
    /// サインオフ）では採寸 434 に対し実表示 382 で 26px ずれ、右端で作業領域を越えて
    /// 関門のクランプまで誘発した。導出し直す契機は**キャラ窓の寸が実際に変わった最初の
    /// 書込**であり、そこで `spawn::BalloonKeywordBase` を消費して `BalloonFollow.offset`
    /// を導出し直す（`follow::keyword_base::rederive_keyword_balloon_offset`）。
    ///
    /// 特定の route（`PlacementRoute::ReportedSizeReconcile` など）で絞る案は**却下した**
    /// ——実表示寸を最初に運ぶ route は `DpiReproject`／`ReportedSizeReconcile`／`Resnap`
    /// のいずれにもなり得て、どれが先に来るかは frame の相順に依存する。route を条件に
    /// すると相順が変わるたびに静かに壊れるため、寸の変化そのものを条件に置いている。
    pub balloon_keyword_base: Option<(BalloonXMode, PointPx)>,
}

/// キーワード由来のバルーン基本位置（`CenterTop`／`CenterBottom`）＋調整量の**唯一の式**
/// （要件 4.2/4.3/4.4・DD8）。
///
/// `Side` は `None` を返す——数値指定・未指定の分岐はこの関数を通らず、P5 の既存式が
/// そのまま担う（4.5/5.2 の bit 同一）。
///
/// # なぜ関数として切り出すのか
///
/// 消費者が 2 つあるからである: P5（[`resolve_placement`]・採寸寸での初期解決）と、
/// 実表示寸確定時の一度きりの再導出（`follow::window_move::resize_window_to`）。
/// 幾何を書き写すと片方だけ直したときに静かに割れる（`clamp_axis` を
/// `balloon_limit.rs` へ逐語再掲した件で既に登記済みのドリフト面と同型）。
///
/// - 水平: `char_x + (char_w − balloon_w) / 2`（中点は整数除算＝0 方向切り捨て・DD8。
///   丸め権威の新設ではない）
/// - 垂直: `CenterTop` は `char_y − balloon_h`（バルーン下端がシェル画像上端に接する）・
///   `CenterBottom` は `char_y + char_h`（バルーン上端がシェル画像下端に接する）
/// - `adjust`（`windowposition.y` の数値＋`balloon.offsetx/offsety`）を基本位置へ加算（4.4）
///
/// 座標演算は P5 と同じく `saturating_add`／`saturating_sub`（panic しない契約）。
/// `char_pos` に原点 `(0,0)` を渡せば戻り値はそのまま**キャラ窓左上相対の offset**になる
/// （`balloon_pos − char_pos` の定義そのもの）。
pub fn keyword_balloon_pos(
    mode: BalloonXMode,
    char_pos: PointPx,
    char_size: SizePx,
    balloon_size: SizePx,
    adjust: PointPx,
) -> Option<PointPx> {
    let base_y = match mode {
        // 数値指定・未指定はこの式を通さない（P5 の既存分岐が担う・4.5/5.2）
        BalloonXMode::Side => return None,
        // 下端がシェル画像上端に接する（4.2）
        BalloonXMode::CenterTop => char_pos.y.saturating_sub(balloon_size.h),
        // 上端がシェル画像下端に接する（4.3）
        BalloonXMode::CenterBottom => char_pos.y.saturating_add(char_size.h),
    };
    // 水平中央: 中点は整数除算（0 方向切り捨て・DD8）
    let base_x = char_pos
        .x
        .saturating_add(char_size.w.saturating_sub(balloon_size.w) / 2);
    Some(PointPx {
        x: base_x.saturating_add(adjust.x),
        y: base_y.saturating_add(adjust.y),
    })
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
/// - **P5（バルーン暫定 offset・DD7）**: 基本位置は `balloon_x_mode`（`windowposition.x`
///   の語彙解決値）で分岐する。
///   - `Side`（数値指定・未指定＝現行挙動・windowposition-limit 4.5/5.2 で**不変**）:
///     `balloon.alignment=Left`（既定）→ `balloon_x = char_x − balloon_w`、
///     `Right` → `balloon_x = char_x + char_w`。`balloon_y = char_y`（上端揃え）。
///   - `CenterTop`（`windowposition.x=center|top`・4.2）: シェル画像の中央上。
///     `balloon_x = char_x + (char_w − balloon_w) / 2`（中点は整数除算＝0 方向切り捨て・
///     DD8。丸め権威の新設ではない）・`balloon_y = char_y − balloon_h`
///     （バルーン下端がシェル画像上端に接する）。
///   - `CenterBottom`（`windowposition.x=bottom`・4.3）: 同 x ・
///     `balloon_y = char_y + char_h`（バルーン上端がシェル画像下端に接する）。
///
///   いずれのモードでも `balloon.offsetx/offsety` と `windowposition.y` 由来の調整量
///   （`balloon_offset` 欄へ合流済み）を基本位置へ加算する（4.4）。「シェル画像の
///   上端／下端／中央」はキャラ窓 rect で読む（窓寸＝採寸したシェル画像寸——
///   `measure.rs` の `char_size` が spawn の窓寸そのもの）。
///   **本関数はバルーンをクランプしない**（バルーンは work area 外へ素直にはみ出す）
///   ——`windowposition.limit` の補正は下流の関門が所有する（`ScopePlacement.balloon_pos`
///   の doc 参照）。offset は
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
    measure_dpi: DPI,
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

        // P5: バルーン暫定 offset（DD7・ここではクランプしない＝limit 補正は下流の関門が
        //     所有）。left（既定）＝キャラ左隣・
        //     right＝キャラ右隣・上端揃え・balloon.offsetx/offsety があれば加算
        // `windowposition.y` の数値調整量は `balloon_offset` 欄へ合流済み——モードに
        // よらず基本位置へ加算する（4.4）。
        let (ox, oy) = sc.balloon_offset.unwrap_or((0, 0));
        let adjust = PointPx { x: ox, y: oy };
        let char_pos = PointPx { x, y };

        // P5 キーワード幾何（windowposition-limit 4.2/4.3・DD3）: キーワードのときだけ
        // [`keyword_balloon_pos`]（唯一の式・実表示寸確定時の再導出と共有）へ委ねる。
        // `Side` は下の既存式をそのまま採る（4.5/5.2＝数値指定・未指定の分岐は
        // 1 ビットも変えない）。
        let balloon_pos = match keyword_balloon_pos(
            sc.balloon_x_mode,
            char_pos,
            input.char_size,
            input.balloon_size,
            adjust,
        ) {
            Some(pos) => pos,
            None => {
                let balloon_base_x = match sc.balloon_alignment {
                    BalloonSide::Left => x.saturating_sub(input.balloon_size.w),
                    BalloonSide::Right => x.saturating_add(w),
                };
                PointPx {
                    x: balloon_base_x.saturating_add(ox),
                    y: y.saturating_add(oy),
                }
            }
        };
        // 事後条件の値を 1 度だけ組む（欄と基準対で同じ式を書き写さない）。
        let balloon_offset = PointPx {
            x: balloon_pos.x.saturating_sub(char_pos.x),
            y: balloon_pos.y.saturating_sub(char_pos.y),
        };
        out.push(ScopePlacement {
            scope: input.scope,
            char_pos,
            char_size: input.char_size,
            balloon_pos,
            balloon_size: input.balloon_size,
            // 事後条件: balloon_offset ≡ balloon_pos − char_pos（design Postconditions）。
            // 成立するのは**本関数の出力時点**まで（DD6）——下流の関門が balloon_pos だけを
            // 補正した後は、この欄は補正を含まない論理相対位置として残る。
            balloon_offset,
            // 基準対（要件 3.1・D15）: 配置式が出した既定の offset に**採寸 DPI**を刻む。
            // 幾何は 1 ビットも変えない——札を貼るだけの代入である。
            balloon_offset_base: OffsetBase {
                offset: balloon_offset,
                dpi: Some(measure_dpi),
            },
            // limit は判定せず転記するだけ（design C4・下流の関門が所有する）
            balloon_limit: sc.balloon_limit,
            // 4.2/DD15: cascade 解決済み alignment を 5 値アンカーへ解釈（単一真実源・
            // Req1.6）。旧 bottom_snap（二値）はここでは格納せず、使用点で
            // !anchor.is_free() として導出する
            anchor: Anchor::from_alignment(&sc.alignment),
            // 実表示寸確定時の一度きりの再導出の素材（要件 4.7・実機サインオフ是正）。
            // `Side` は素材を持たない＝再導出が構造的に起こらない（4.5/5.2）。
            balloon_keyword_base: match sc.balloon_x_mode {
                BalloonXMode::Side => None,
                mode => Some((mode, adjust)),
            },
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
#[allow(dead_code)]
// scaffold（task 1.1）: ScopePlacement への結線は task 1.2・射影消費は follow（task）
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
#[path = "resolver_balloon_keyword_tests.rs"]
mod balloon_keyword_tests;
#[cfg(test)]
#[path = "resolver_from_alignment_tests.rs"]
mod from_alignment_tests;
#[cfg(test)]
#[path = "resolver_resolve_balloon_tests.rs"]
mod resolve_balloon_tests;
#[cfg(test)]
#[path = "resolver_resolve_contract_tests.rs"]
mod resolve_contract_tests;
#[cfg(test)]
#[path = "resolver_resolve_free_tests.rs"]
mod resolve_free_tests;
#[cfg(test)]
#[path = "resolver_resolve_test_support.rs"]
mod resolve_test_support;
#[cfg(test)]
#[path = "resolver_resolve_tests.rs"]
mod resolve_tests;
#[cfg(test)]
#[path = "resolver_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "resolver_union_tests.rs"]
mod union_tests;
