//! モニタ work area 解決（[`MonitorSnapshot`]・[`work_area_for_window`]・[`WorkAreaResolution`]）。

use bevy_ecs::prelude::*;
use wintf::ecs::window::monitor::Monitor;

use super::RectPx;

// =============================================================================
// MonitorSnapshot（task 8.1・DD15 基盤・4.7）
// =============================================================================

/// 全モニタの work area 集合（物理 px・**実行時に同期される**）。
///
/// 起動時に seam（main.rs）／example が [`MonitorSources::from_monitors`] で実モニタ
/// から忠実転写して Resource 挿入し、bottom 吸着ドラッグ（task 8.2）が
/// [`work_area_for_window`] で「窓が現在属するモニタの work area」を引くのに使う。
/// 中身は `RectPx` のみの純粋データで、headless テストは合成値を直接構築して
/// 注入する（偽装境界・wintf に触れるのは挿入サイトだけ）。
///
/// # 「セッション内固定」の撤回（areka-P0-dpi-transition-atomicity task 5.1・DD15 撤回）
///
/// 以前ここには「snapshot はセッション内固定＝M1 受容（`WM_DISPLAYCHANGE` 追随は後続・
/// DD15）」と書いてあった。**その受容は撤回された**——拡大率を下げるとタスクバーの物理高が
/// 縮んで実際の作業領域下端は下がるのに、起動時の下端が焼き付いたままキャラ窓が接地し
/// 続け、実機で 6/6 の遷移が接地点 −48px の浮きを出した（確定台帳 L3）。
/// 現在は毎フレーム先頭の同期段（`emo2_boot::frame::work_area_sync`）が実行時のモニタ表
/// から本 Resource を作り直す。**変化したフレームだけ**差し替わり、同じ表なら無操作である
/// （順序の揺れは [`same_monitors`] が吸収する）。
///
/// ただし**保存位置の復元判定は起動時に 1 度だけ読む契約**を維持する（要件 5.7・
/// `main.rs` の復元マージ）——拡大率をまたいで保存位置を追従させる裁定は採っていない。
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MonitorSnapshot {
    /// モニタ列挙順の work area（物理 px）。
    pub work_areas: Vec<RectPx>,
}

impl MonitorSnapshot {
    /// 実モニタ列挙結果から全 work area（`RECT`・物理 px）を列挙順のまま
    /// **単位変換なしで忠実転写**する（mod.rs `primary_work_area` と同じ U 契約:
    /// どちらも物理 px 通貨）。0 台は空 snapshot（panic しない・消費側が
    /// [`work_area_for_window`] の `None` で防御）。
    pub fn from_monitors(monitors: &[Monitor]) -> Self {
        Self {
            work_areas: monitors
                .iter()
                .map(|m| RectPx {
                    left: m.work_area.left,
                    top: m.work_area.top,
                    right: m.work_area.right,
                    bottom: m.work_area.bottom,
                })
                .collect(),
        }
    }
}

/// 窓矩形の中心が属するモニタの work area を引く純粋ヘルパ（4.7・DD15）。
///
/// 決定論規則（テストで固定）:
/// - 中心は `((left+right)/2, (top+bottom)/2)`（i64 演算・ゼロ方向切り捨て）
/// - 帰属判定は half-open（`left ≤ cx < right`・`top ≤ cy < bottom`）＝共有辺上の
///   中心は右／下側のモニタへ属する。複数矩形が含む（重複）場合は昇順 index 先勝ち
/// - どのモニタにも属さない場合は最近傍（中心→矩形 clamp 点の自乗距離最小・
///   等距離は昇順 index 先勝ち＝`min_by_key` の先頭優先）
/// - 空 snapshot は `None`（架空の既定矩形を発明しない・resolver と同方針）
///
/// 距離は i128 で自乗和を取り、極端な仮想スクリーン座標でも溢れない
/// （panic しない契約・resolver の saturating 演算と同じ防波堤）。
///
/// # 判別付き版への委譲（task 2.2・D6）
///
/// 本関数は [`work_area_for_window_with_origin`] の**戻り値から判別を落とすだけ**の
/// 薄いラッパである（契約不変＝既存呼出元の挙動は 1 bit も変わらない。等価性は
/// `work_area_for_window_delegates_to_with_origin` が檻で固定する）。
pub fn work_area_for_window(snapshot: &MonitorSnapshot, window: RectPx) -> Option<RectPx> {
    work_area_for_window_with_origin(snapshot, window).map(|(wa, _)| wa)
}

// =============================================================================
// 可視性の遷移ガード＋work area 解決の判別（task 2.2・D6/S3′・Req 3.1/3.2/5.3）
// =============================================================================

/// [`work_area_for_window_with_origin`] が work area を決めた**規則**（D6・Req 3.2）。
///
/// 最近傍フォールバックは「どのモニタにも属さない」＝モニタ構成情報と実画面の
/// 食い違い／窓が可視領域外という**異常の兆候**でありながら、[`work_area_for_window`]
/// の戻り値だけでは正常な帰属と区別できない（S3 の後半＝「最近傍フォールバックが
/// 異常を無観測で吸収する」）。本 enum はその区別を呼出側へ返すためだけに在る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkAreaResolution {
    /// 窓中心がその work area に帰属した（half-open 判定・正常）。
    Contains,
    /// どのモニタにも属さず、最近傍フォールバックで選ばれた（Req 3.2 の観測点）。
    NearestFallback,
}

/// [`work_area_for_window`] の判別付き版（D6・Req 3.2）。
///
/// 解決規則そのものは [`work_area_for_window`] の doc が定める決定論規則と**完全に
/// 同一**（中心の半開帰属・昇順 index 先勝ち・最近傍は clamp 点自乗距離最小・空
/// snapshot は `None`）。違いは「どちらの規則で決まったか」を
/// [`WorkAreaResolution`] として併せて返す点だけである。
///
/// # 消費側（実在の配線・file:line で辿れる形にしてある）
///
/// 判別を消費するのは [`super::visibility::evaluate_visibility_guard`] **1 箇所だけ**で、
/// そこが `NearestFallback` を非ドラッグ経路でのみ `warn!` へ昇格させる（ドラッグ経路は
/// そもそも本関数を通らないので、毎イベント発火の spam にならない）。昇格は**2 つの矩形**に
/// 対して別々に行う——射影が**決めた位置**（`[visibility-guard] NearestFallback`・
/// areka-P0-dpi-window-vanish が配線）と、射影の**入力**＝Y を決めるのに使った矩形
/// （`[visibility-guard] OffscreenPull`・areka-P0-dpi-transition-atomicity
/// task 5.1 が追加）である。後者が要るのは、入力が帰属しない窓でも決めた位置がモニタ内へ
/// 収まれば前者の腕に入らず、**画面外から最近傍モニタへ引き寄せられた事実が無観測で
/// 消える**ためである（実測で 0 行だった）。
///
/// # 帰属できない窓の**位置**（開発者の裁定 2026-08-20・現行挙動が正）
///
/// 本関数は空でない snapshot に対して `None` を返さない——どの矩形にも属さない中心は
/// 必ず最近傍へ寄せられる（`None` になるのは `work_areas` が空のときだけ＝下の
/// `min_by_key` が `None` を返す経路）。**この挙動が正である**というのが裁定である:
/// 副モニタを引き抜いたときに現状維持を選ぶと、ゴーストは画面外に取り残されて見えず
/// 触れなくなる——判断の軸は**ゴーストが触れなくなる事態を避ける**ことで、主モニタへ
/// 引き寄せられる方が安全である。
///
/// ゆえに `NearestFallback` は「解決できなかった」ではなく「**最近傍で解決した**」であり、
/// areka-P0-dpi-transition-atomicity 要件 5.5 の「位置を変更せずに現状を維持」が効くのは
/// **モニタ表が空のときに限る**。同 要件 5.6 が守る位置権威との衝突は本裁定で解消した。
/// 記録だけは残す（上の警告）——勝手に飛んだことを後から追えるようにするためで、位置は
/// 変えない。全文は同 spec の `requirements.md` 要件 5 の項目 5 直下の裁定の注記にある。
pub fn work_area_for_window_with_origin(
    snapshot: &MonitorSnapshot,
    window: RectPx,
) -> Option<(RectPx, WorkAreaResolution)> {
    let cx = (window.left as i64 + window.right as i64) / 2;
    let cy = (window.top as i64 + window.bottom as i64) / 2;

    // 帰属（half-open）・昇順 index 先勝ち
    if let Some(wa) = snapshot.work_areas.iter().find(|wa| {
        (wa.left as i64) <= cx
            && cx < (wa.right as i64)
            && (wa.top as i64) <= cy
            && cy < (wa.bottom as i64)
    }) {
        return Some((*wa, WorkAreaResolution::Contains));
    }

    // どこにも属さない → 最近傍（clamp 点との自乗距離最小・等距離は先勝ち）
    snapshot
        .work_areas
        .iter()
        .min_by_key(|wa| {
            // `i64::clamp` は逆転区間（万一の退化矩形）で panic するため min/max で書く
            // （resolver `clamp_axis` と同じ非 panic 流儀）
            let px = cx.min(wa.right as i64).max(wa.left as i64);
            let py = cy.min(wa.bottom as i64).max(wa.top as i64);
            let dx = (cx - px) as i128;
            let dy = (cy - py) as i128;
            dx * dx + dy * dy
        })
        .copied()
        .map(|wa| (wa, WorkAreaResolution::NearestFallback))
}

// =============================================================================
// モニタ別拡大率表と 2 源の同時構築（task 5.1・C6・要件 5.1/5.5/5.6）
// =============================================================================

/// モニタ 1 台ぶんの拡大率と画面矩形（物理 px・列挙順のまま忠実転写）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorDpiEntry {
    /// モニタの矩形（`work_area` ではなく `bounds`＝タスクバーを含む全体）。
    pub bounds: RectPx,
    /// モニタの拡大率。
    pub dpi: u32,
}

/// モニタ別の拡大率表（[`MonitorSnapshot`] と対になる実行時同期の Resource）。
///
/// 作業領域源が「どこに置けるか」を答えるのに対し、本表は「そこはどの拡大率か」を答える。
/// 2 つは**同一の `Monitor` 列から同時に**作られる（[`MonitorSources::from_monitors`]）——
/// 別々に作ると、片方だけが古い運転が生まれる。
///
/// # 引き当ての規則はまだ持たない
///
/// 窓中心からモニタを引き当てる規則（表示基盤側の `monitor_containing` と共有すべきもの）は
/// 拡大率と表の整合待ち（task 5.4）が所有する。本表は task 5.1 の時点では**純データ**であり、
/// 引き当て規則を先に発明して二重権威にしない。
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct MonitorDpiTable {
    /// モニタ列挙順の拡大率と矩形。
    pub entries: Vec<MonitorDpiEntry>,
}

impl MonitorDpiTable {
    /// 実モニタ列挙結果から拡大率と矩形を列挙順のまま忠実転写する
    /// （[`MonitorSnapshot::from_monitors`] と同じ U 契約＝物理 px・単位変換なし）。
    pub fn from_monitors(monitors: &[Monitor]) -> Self {
        Self {
            entries: monitors
                .iter()
                .map(|m| MonitorDpiEntry {
                    bounds: RectPx {
                        left: m.bounds.left,
                        top: m.bounds.top,
                        right: m.bounds.right,
                        bottom: m.bounds.bottom,
                    },
                    dpi: m.dpi,
                })
                .collect(),
        }
    }
}

/// モニタ表から作られる 2 つの源（作業領域源＋モニタ別拡大率表）の組。
///
/// **起動時（`main.rs` のシーム）も実行時の同期（`emo2_boot::frame::work_area_sync`）も
/// この 1 つの構築関数を通る**（要件 5.1・設計 C6「構築関数は同一＝二重権威にならない」）。
/// 起動時だけが別の作り方をすると、同期が入った後も起動時の値だけが違う形になり得る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorSources {
    /// 全モニタの作業領域集合。
    pub snapshot: MonitorSnapshot,
    /// モニタ別の拡大率表。
    pub dpi_table: MonitorDpiTable,
}

impl MonitorSources {
    /// 実モニタ列挙結果から 2 源を**同時に**作る（列挙順は両者で一致する）。
    pub fn from_monitors(monitors: &[Monitor]) -> Self {
        Self {
            snapshot: MonitorSnapshot::from_monitors(monitors),
            dpi_table: MonitorDpiTable::from_monitors(monitors),
        }
    }

    /// 台数（両源で同じ＝同一の列から作られる）。
    pub fn len(&self) -> usize {
        self.snapshot.work_areas.len()
    }

    /// 1 台も無い（列挙異常・[`MonitorSnapshot::from_monitors`] の 0 台契約と同義）。
    // 同期段は 0 台を `Monitor` 列の側で弾く（表を作る前に警告して現状維持する）ので、
    // 本判定を名前で呼ぶのは今のところ檻だけである。`len` を持つ型に `is_empty` が無いのは
    // API として不自然なので残す（areka は bin crate ゆえ `pub` でも dead_code 免除されない）。
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 比較用の正規形——**台ごとの (作業領域, 矩形, 拡大率) を昇順に並べた列**。
    ///
    /// `RectPx` は順序を持たないので、成分をそのまま組にして並べる（型へ `Ord` を足すと
    /// 矩形に「大小」という意味を与えることになり、幾何の型としては誤解を招く）。
    fn normalized(&self) -> Vec<(i32, i32, i32, i32, i32, i32, i32, i32, u32)> {
        let mut rows: Vec<_> = self
            .snapshot
            .work_areas
            .iter()
            .zip(self.dpi_table.entries.iter())
            .map(|(wa, entry)| {
                (
                    wa.left,
                    wa.top,
                    wa.right,
                    wa.bottom,
                    entry.bounds.left,
                    entry.bounds.top,
                    entry.bounds.right,
                    entry.bounds.bottom,
                    entry.dpi,
                )
            })
            .collect();
        rows.sort_unstable();
        rows
    }
}

/// 2 つのモニタ源が**順序に依らず**同じ内容か（要件 5.4・設計 C6「順序非依存の比較」）。
///
/// 実行時のモニタ表（`Monitor` entity 群）の走査順は列挙順と一致する保証が無く、
/// 素の `==` で比べると**中身が同じなのに毎フレーム作り直す**運転が生まれ得る
/// （定常フレームの窓書込ゼロという契約を、順序という無関係な理由で壊す）。
/// ゆえに比較は台の集合として行い、順序の揺れをここで吸収する。
///
/// 比べるのは台ごとの (作業領域, 矩形, 拡大率) の全成分である——作業領域だけを比べると、
/// 拡大率だけが変わった構成変更（表の値が変わって作業領域は同じ）を取りこぼす。
pub fn same_monitors(a: &MonitorSources, b: &MonitorSources) -> bool {
    a.len() == b.len() && a.normalized() == b.normalized()
}
