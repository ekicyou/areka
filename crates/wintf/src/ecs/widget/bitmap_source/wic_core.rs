//! WicCore - WIC関連リソース（Device Lostの影響を受けない）
//!
//! WICはCPUベースのイメージ処理のため、GPUのDevice Lostとは独立。
//! GraphicsCore.invalidate()時もWicCoreは有効なまま。

use bevy_ecs::prelude::*;
use std::sync::OnceLock;
use windows::Win32::Graphics::Imaging::D2D::*;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::*;
use windows::core::Result;

/// プロセス寿命の暗黙 MTA を一度だけ確立するためのガード（cookie は保持しない＝leak）。
static MTA_KEEPER: OnceLock<()> = OnceLock::new();

/// WIC factory 生成に先立ち、本プロセスに **プロセス寿命の暗黙 MTA** を確立する（DD-9）。
///
/// 背景（根本原因）: layout 等のテストバイナリは `CoInitializeEx` を呼ばないため、
/// `WicCore::new()` の `CoCreateInstance` が「たまたま並走テストの副作用で立っている
/// 一時的 MTA」の窓で成功しうる。その借り物 MTA が解体されると COM ランタイムごと
/// factory が解放され、`EcsWorld` drop 時の `IUnknown::Release` が use-after-free
/// （0xC0000005）を起こす。
///
/// 処方（interest-keeper と同型）: `CoIncrementMTAUsage` で暗黙 MTA を**自ら**常駐させる。
/// 返る `CO_MTA_USAGE_COOKIE` は保持せず意図的に leak する（`CoDecrementMTAUsage` は
/// 呼ばない）＝MTA をプロセス寿命で生かし続けるのが目的。`OnceLock` で初回のみ確立し、
/// 以降は no-op。`CoIncrementMTAUsage` の失敗（極稀）は `?` で `Err` 伝播し、
/// panic しない（縮退は呼び元 `world/mod.rs` の log-first が担う）。
fn ensure_process_mta() -> Result<()> {
    if MTA_KEEPER.get().is_some() {
        return Ok(());
    }
    // SAFETY: Win32 境界。`CoIncrementMTAUsage` は暗黙 MTA をプロセス寿命で常駐させる
    // 純粋な COM ランタイム操作で、事前の apartment 初期化を要さない（任意スレッドから
    // 呼び出し可）。返る `CO_MTA_USAGE_COOKIE`（`!Send`）は **意図的に leak** する:
    // 目的はプロセス寿命 MTA の維持であり、`CoDecrementMTAUsage`（cookie を消費）は
    // 決して呼ばない。windows-rs 0.62.2 の `CO_MTA_USAGE_COOKIE` に Drop は無く、
    // cookie を drop しても MTA は解体されない。
    let _cookie = unsafe { CoIncrementMTAUsage()? };
    // 並行初回で複数スレッドが increment しても、decrement しないため過剰増分は無害。
    let _ = MTA_KEEPER.set(());
    Ok(())
}

/// WIC関連リソース
///
/// WICファクトリを保持し、画像デコード機能を提供する。
/// Device Lostの影響を受けない独立リソース。
#[derive(Resource, Clone)]
pub struct WicCore {
    factory: IWICImagingFactory2,
}

// SAFETY 条件: windows-rs 0.62.2 は WIC インターフェイス（`IWICImagingFactory2` 等）に
// Send/Sync を自動生成しない（Imaging モジュールに `unsafe impl Send` が存在しない）ため、
// この手動 impl は必須である。健全性は WIC の free-threaded（thread-free marshaling）特性に
// 依拠する: ファクトリは `CLSCTX_INPROC_SERVER` で生成され、跨スレッドで直接アクセス可能。
// 呼び元スレッドの apartment 状態に依存しない: `WicCore::new` は `CoCreateInstance` の前に
// `ensure_process_mta`（`CoIncrementMTAUsage`）を呼び、**自ら**プロセス寿命の暗黙 MTA を
// 強制する（cookie を leak＝decrement しない）。これにより factory は「呼び元が
// `CoInitializeEx(COINIT_MULTITHREADED)` 済みである前提」に依存せず、**常に生存する COM
// ランタイム上に在る**。借り物 MTA（並走テストの副作用で立つ一時的 MTA）の解体で COM が
// factory ごと解放され、drop 時の `IUnknown::Release` が use-after-free（0xC0000005）を
// 起こす経路を構造的に根絶する（DD-9）。実利用上、`WicCore` は clone されて `WintfTaskPool`
// のバックグラウンドワーカーへ move され（Send）、`factory()` の読み取り参照でデコードに
// 用いられる（Sync）。
unsafe impl Send for WicCore {}
unsafe impl Sync for WicCore {}

impl WicCore {
    /// WicCoreを作成
    pub fn new() -> Result<Self> {
        // factory 生成に先立ち、プロセス寿命の暗黙 MTA を自己確立する（DD-9・借り物 MTA
        // 解体に伴う use-after-free の根絶）。
        ensure_process_mta()?;
        let factory: IWICImagingFactory2 =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory2, None, CLSCTX_INPROC_SERVER)? };
        Ok(Self { factory })
    }

    /// WICファクトリへの参照を取得
    pub fn factory(&self) -> &IWICImagingFactory2 {
        &self.factory
    }
}
