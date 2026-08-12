//! ゴースト窓ペアの重なり関係——宣言・再断行要求・確立記録・実行時ストラテジ
//!
//! 「同一スコープにおいてバルーン窓は自分のキャラ窓のすぐ手前に居る」という不変条件
//! （要件 1.1）を、**entity 参照だけの宣言**として持つ層である。宣言する側（areka）は
//! scope を知る唯一の層であり、HWND も Win32 も知らない。反映する側（wintf）は HWND と
//! Win32 を知る唯一の層である——この責務分界ゆえ、宣言は wintf 側に住み、`Entity` を
//! 取る（wintf → areka の import は禁止ゆえ scope 型は受け取れない）。
//!
//! - `KeepDirectlyAbove`: 永続宣言（バルーン窓へ付与・peer はキャラ窓）
//! - `ReassertZOrder`: 重なりを断行し直す一回限りの要求（未適用／適用済み・検証待ちの 2 段階）
//! - `ExpectedOrder`: 適用済み要求が次巡で照合する期待隣接
//! - `OwnerLink`: 案 A の owner を張った事実の記録（切離しに使う）
//! - `ZOrderPairStrategy`: 案 A／案 B を実行時に切り替える設定
//!
//! 判断の純関数（`decide_pair_fix`）・確立系・維持系・診断ログ語彙は本フィーチャーの
//! 後続タスクが同ファイルへ足す。本ファイルは現時点では状態定義のみを持つ。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;

// ============================================================================
// KeepDirectlyAbove - ペア関係の永続宣言
// ============================================================================

/// ペア宣言（バルーン窓へ付与・peer はキャラ窓 entity）。
/// 「この窓は peer 窓のすぐ手前に居るべき」の永続宣言。
///
/// 付与するのは scope を知る層（areka の spawn）。宣言は片側（手前に居るべき窓＝
/// バルーン窓）にのみ付き、対になる窓は `peer` で指す。スコープ間には宣言を張らない
/// ——これが「スコープ間の上下関係を固定の規則で決めない」（要件 3.1）と「是正時に
/// 当該スコープの 2 窓しか動かさない」（要件 3.4）の構造的な根拠である。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeepDirectlyAbove {
    pub peer: Entity,
}

// ============================================================================
// ExpectedOrder / ReassertZOrder - 再断行の一時要求と検証段階
// ============================================================================

/// 適用済みの再断行要求が**次巡で照合する期待隣接**。
///
/// 「`above` が `below` のすぐ手前に居る」——是正の 2 つの腕
/// （バルーンを手前へ／キャラを直後へ）はどちらも最終的にこの同一の隣接へ収束するため、
/// 期待値はこの 1 形で足りる。照合は `get_window_below(above) == Some(below)`
/// （`GetWindow(GW_HWNDNEXT)` 実測）で行い、不一致なら `verify-failed` を `error!` で
/// 記録する（要件 6.2）。同レコードの `expected` がこの値、`measured` が実測値である。
///
/// 座標・寸法のフィールドは**持たない**——是正は表示順のみを動かす（要件 1.6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedOrder {
    /// 手前側に居るべき窓（通常はバルーン窓）
    pub above: HWND,
    /// 背後側に居るべき窓（通常はキャラ窓）
    pub below: HWND,
}

// SAFETY: `ExpectedOrder` は 2 つの `HWND`（windows-rs では `*mut c_void` の newtype）を
// 保持するため、自動では Send/Sync が導出されない（windows-rs 0.62.2 は HWND に
// Send/Sync を実装しない）。よってこの手動 impl は冗長ではなく必須である
// （これが無いと [`ReassertZOrder`] が `Component` の `Send + Sync` 要求を満たせない）。
// 健全性: ここでの HWND は**期待隣接を記録した値**であり、窓の所有権も解放責務も伴わない
// 不透明なウィンドウ識別子にすぎない（読み書きは値のコピーのみ）。この値を実際に Win32 へ
// 渡すのは維持系の system であり、Win32 を呼ぶ system は NonSend パラメータで UI スレッド
// 固定されている——他スレッドが HWND を用いて窓を操作する経路は存在しない。
// `ZOrder::InsertAfter(HWND)` と同根の crate 標準の HWND 取り扱い方針。
unsafe impl Send for ExpectedOrder {}
unsafe impl Sync for ExpectedOrder {}

/// 重なりの再断行の一時要求（one-shot）。挿入元:
///   ① `establish_owner_links`（初期隣接の確定）
///   ② balloon-visibility（要件 2.6: show 後に挿入）★相互登記の契約点
///   ③ `WM_WINDOWPOSCHANGED` z 変化検知（案 B の B2）
/// 維持系が消費し、適用→検証の完了で remove する。
///
/// 段階は `pending_verify` が表す——`None` は「挿入済み・未適用」、`Some(_)` は
/// 「適用済みで実測検証待ち」。検証を次巡へ遅らせるのは、適用（`SetWindowPosCommand`）の
/// flush が tick 後であり、同巡の実測では指令が反映されていないためである。
/// この 2 段階が「指令は出したが効かなかった」を切り分ける（要件 6.2）。
///
/// 他クレート（表示制御を所有する仕様）から挿入されるため公開到達性を持つ。
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReassertZOrder {
    /// 適用済みで実測検証待ち（expected は検証時の期待隣接）
    pub pending_verify: Option<ExpectedOrder>,
}

// NOTE: `ReassertZOrder` 自身に手動 Send/Sync は不要——`Option<ExpectedOrder>` は
// [`ExpectedOrder`] の Send/Sync を通じて自動導出される。冗長な `unsafe` は置かない。

// ============================================================================
// OwnerLink - 案 A の owner 確立記録
// ============================================================================

/// 案 A の owner 確立済み記録（バルーン窓へ付与・切離しに使う）。
///
/// `set_window_owner` が成功した事実そのものであり、再確立の抑止（冪等性）と、
/// 破棄経路での `clear_window_owner` 呼出に使う（要件 5.9）。owner を張れなかった
/// ペアにはこの記録が付かない——「張った」と「張っていない」がここで区別される。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerLink {
    pub owner_hwnd: HWND,
}

// SAFETY: `OwnerLink` は `HWND`（`*mut c_void` の newtype）を保持するため、自動では
// Send/Sync が導出されず、この手動 impl は `Component`（ECS は Send+Sync を要求）に
// するために必須である。健全性: 保持するのは owner 窓の不透明な識別子の**値の写し**で
// あり、所有権も破棄責務も持たない（窓の破棄は `WindowRegistry`／`Window` drop が担う）。
// この HWND を Win32 へ渡すのは owner 確立系・切離し系であり、いずれも NonSend パラメータで
// UI スレッド固定された system である。`ZOrder::InsertAfter(HWND)` と同根。
unsafe impl Send for OwnerLink {}
unsafe impl Sync for OwnerLink {}

// ============================================================================
// ZOrderPairStrategy - 実行時ストラテジ
// ============================================================================

/// 実行時ストラテジ（areka main が挿入。既定は `OwnerLink { raise_assist: false }`）。
///
/// 案 A（Win32 owner の OS 保証）を本線とし、実機ゲートで要件 5.1〜5.5 の毀損が
/// 判明した場合のみ案 B へ切り替える（要件 5.6）。宣言の付与コードは両案で同一であり、
/// 切り替わるのは wintf 側の反映手段だけである。
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZOrderPairStrategy {
    /// 案 A: owner 保証。`raise_assist` はゲート G7 FAIL 時のみ true（要件 1.3 の明示実装）
    OwnerLink { raise_assist: bool },
    /// 案 B: B2 検知＋B3 同乗の明示維持
    ExplicitMaintenance,
}

impl Default for ZOrderPairStrategy {
    /// 既定は案 A・raise assist 無効（design.md「State（コンポーネント契約）」）。
    ///
    /// バリアントがフィールドを持つため `#[derive(Default)]` の `#[default]` 属性は
    /// 使えず、手書きの impl とする。
    fn default() -> Self {
        Self::OwnerLink {
            raise_assist: false,
        }
    }
}

#[cfg(test)]
#[path = "zorder_pair_tests.rs"]
mod tests;
