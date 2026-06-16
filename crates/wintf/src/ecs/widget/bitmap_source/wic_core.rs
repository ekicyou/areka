//! WicCore - WIC関連リソース（Device Lostの影響を受けない）
//!
//! WICはCPUベースのイメージ処理のため、GPUのDevice Lostとは独立。
//! GraphicsCore.invalidate()時もWicCoreは有効なまま。

use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Imaging::D2D::*;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::*;
use windows::core::Result;

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
// 依拠する: ファクトリは `CLSCTX_INPROC_SERVER` で生成され、本プロセスは MTA
// （`CoInitializeEx(COINIT_MULTITHREADED)`）で初期化されるため、`IWICImagingFactory2` は
// 跨スレッドで直接アクセス可能。実利用上、`WicCore` は clone されて `WintfTaskPool` の
// バックグラウンドワーカーへ move され（Send）、`factory()` の読み取り参照でデコードに
// 用いられる（Sync）。
unsafe impl Send for WicCore {}
unsafe impl Sync for WicCore {}

impl WicCore {
    /// WicCoreを作成
    pub fn new() -> Result<Self> {
        let factory: IWICImagingFactory2 =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory2, None, CLSCTX_INPROC_SERVER)? };
        Ok(Self { factory })
    }

    /// WICファクトリへの参照を取得
    pub fn factory(&self) -> &IWICImagingFactory2 {
        &self.factory
    }
}
