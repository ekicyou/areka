//! 回帰檻（C7 RegressionCage・Requirement 5.1/5.2/5.3）
//!
//! 「同一プロセス内で WUC グラフィックススタック（`GraphicsCore` + `WucGraphicsResource`＝
//! `Compositor`）を 2 回以上生成する」パターンが、クラッシュなく決定論的に完走することを
//! 恒久監視する。
//!
//! # 根本原因と是正（正本: research.md「根本原因記録」節）
//! 本環境（Intel Arc / Win 26200）では:
//! - **別スレッド上の 2 個目 `Compositor` 生成** → `STATUS_ACCESS_VIOLATION`（プロセス死）。
//! - **同一スレッド上での DispatcherQueueController 再生成の反復** → `0x8001010E`
//!   （"already created on this thread"）＋リーク。
//!
//! 是正 = ①共有 GPU オーナースレッドへ全 WUC 生成を集約（別スレッド 2 個目を排除）＋
//! ②`WucGraphicsResource::new` が既存スレッド DispatcherQueue を再利用（controller 再生成を排除）。
//!
//! # 檻としての検出保証（Requirement 5.3・サイレント成功は構造的に不可能）
//! 本檻は是正の正準経路（オーナースレッド上での再生成）を通す。是正が退行した場合:
//! - オーナースレッド集約が壊れる（別スレッドで Compositor 生成に戻る）→ 2 個目で
//!   `STATUS_ACCESS_VIOLATION`＝**プロセス死でバイナリ全体が失敗**（アサーション不要で検出）。
//! - DispatcherQueue 再利用が壊れる → `WucGraphicsResource::new` が `0x8001010E` で
//!   `Err` → `expect` で **panic＝テスト失敗**。
//!
//! いずれも「静かに成功」できない。本番の同一 UI スレッド上での再生成（ゴースト再ロード等）が
//! 安全であることの回帰保証も兼ねる。

use windows::UI::Composition::Visual as WucVisual;
use windows::core::Interface;
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use crate::common::on_gpu_owner_thread;

/// フルスタック生成 → 最小合成操作（`CreateSpriteVisual`）→ 解放を 1 サイクル行う。
///
/// 呼び出しは**必ずオーナースレッド上**で行うこと（`on_gpu_owner_thread` 経由）。
fn full_stack_cycle() {
    let core = GraphicsCore::new().expect("GraphicsCore::new（フルスタック生成）");
    let d2d = core.d2d_device().expect("d2d_device が None");
    let wuc =
        WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new（2 個目でも成功すべき）");

    // 最小合成操作: Compositor から SpriteVisual を 1 個生成する（実 WUC 操作を通す）。
    let _visual: WucVisual = wuc
        .compositor()
        .expect("compositor")
        .CreateSpriteVisual()
        .expect("CreateSpriteVisual")
        .cast()
        .expect("cast to Visual");

    // スコープ終端で wuc / core が（オーナースレッド上で）解放される。
}

/// 形態(i)-A: 独立した 2 `#[test]` の 1 本目。
///
/// libtest は `#[test]` ごとに別スレッドを spawn するが、`on_gpu_owner_thread` で
/// オーナースレッドへ集約されるため、A・B のどちらが「プロセス内 2 個目のスタック生成」に
/// なっても、その生成はオーナースレッド上での再生成（＝安全経路）に帰着する。
#[test]
fn wuc_stack_a_full_cycle() {
    on_gpu_owner_thread(full_stack_cycle);
}

/// 形態(i)-B: 独立した 2 `#[test]` の 2 本目（A と対称）。
#[test]
fn wuc_stack_b_full_cycle() {
    on_gpu_owner_thread(full_stack_cycle);
}

/// 形態(ii): 単一 `#[test]` 内で 生成→解放→再生成 を 3 サイクル行う（Requirement 5.1
/// 「単一テストプロセス内で 2 回以上連続生成」を単独実行でも決定論的に検証）。
///
/// 既存 in-source `wuc_graphics_resource_lifecycle`（同一関数内・1 回再生成）が覆えない
/// 「完全解放後の複数回再生成」を明示検証する。
#[test]
fn wuc_stack_recreate_in_single_test() {
    on_gpu_owner_thread(|| {
        full_stack_cycle(); // 1 回目
        full_stack_cycle(); // 2 回目（再生成）
        full_stack_cycle(); // 3 回目（2 回以上を明示）
    });
}
