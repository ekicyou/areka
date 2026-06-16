//! BitmapSource リソースコンポーネント
//!
//! CPU側（BitmapSourceResource）とGPU側（BitmapSourceGraphics）のリソース。

use super::alpha_mask::AlphaMask;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap1;
use windows::Win32::Graphics::Imaging::IWICBitmapSource;

/// CPU側画像リソース（WIC BitmapSource + αマスク）
///
/// # Thread Safety
/// IWICBitmapSourceはthread-free marshaling対応のため
/// Send + Syncを手動実装する。
#[derive(Component)]
pub struct BitmapSourceResource {
    source: IWICBitmapSource,
    alpha_mask: Option<AlphaMask>,
}

// SAFETY 条件: windows-rs 0.62.2 は WIC インターフェイス（`IWICBitmapSource` 等）に
// 対し Send/Sync を自動生成しない（DirectWrite/Direct2D 型とは異なり、Imaging モジュール
// には `unsafe impl Send` が一切存在しない）ため、この手動 impl は冗長ではなく必須である。
// 健全性は WIC オブジェクトのスレッドモデルに依拠する: WIC は free-threaded（thread-free
// marshaling）であり、本プロセスは MTA（`CoInitializeEx(COINIT_MULTITHREADED)`、
// win_thread_mgr.rs）で初期化されるため、`IWICBitmapSource` は跨スレッド移動（Send）・
// 参照共有（Sync）とも直接アクセス可能。実利用上、本リソースはバックグラウンド
// （`WintfTaskPool` ワーカー）でデコード生成され mpsc 経由でメインスレッドへ移送（Send）、
// 以後は `source()` による読み取り専用参照（D2D ビットマップ生成・αマスク生成の
// `get_size`/`copy_pixels`）でのみ用いられる。`alpha_mask` フィールドはプレーンデータ。
unsafe impl Send for BitmapSourceResource {}
unsafe impl Sync for BitmapSourceResource {}

impl BitmapSourceResource {
    /// WIC BitmapSourceから作成
    pub fn new(source: IWICBitmapSource) -> Self {
        Self {
            source,
            alpha_mask: None,
        }
    }

    /// BitmapSourceへの参照を取得
    pub fn source(&self) -> &IWICBitmapSource {
        &self.source
    }

    /// αマスクへの参照を取得
    pub fn alpha_mask(&self) -> Option<&AlphaMask> {
        self.alpha_mask.as_ref()
    }

    /// αマスクを設定（非同期生成完了時に呼び出し）
    pub fn set_alpha_mask(&mut self, mask: AlphaMask) {
        self.alpha_mask = Some(mask);
    }
}

/// GPU側画像リソース（D2D Bitmap）
///
/// BitmapSourceのon_add時にOption::Noneで作成され、
/// BitmapSourceResourceが追加されたらD2D Bitmapを生成する。
///
/// # Device Lost対応
/// 既存のVisualGraphics/SurfaceGraphicsと同じパターン:
/// - invalidate_dependent_componentsシステムがDevice Lost時にinvalidate()を呼ぶ
/// - 次フレームでis_valid() == falseを検出しBitmapを再生成
#[derive(Component, Default)]
pub struct BitmapSourceGraphics {
    bitmap: Option<ID2D1Bitmap1>,
}

// SAFETY 条件: 保持する `ID2D1Bitmap1` は GraphicsCore の MULTI_THREADED D2D ファクトリ
// 系列のオブジェクトであり、跨スレッド移動・参照共有とも D2D が内部直列化するため安全。
// なお windows-rs 0.62.2 は `ID2D1Bitmap1` に Send+Sync を自動付与済み（Direct2D/mod.rs）
// のため本 impl は厳密には冗長だが、crate 全域の COM 保持コンポーネント（components.rs /
// core.rs / dcomp_resource.rs / command_list.rs）が一貫して明示 impl を採る慣習に合わせ、
// かつ将来の windows-rs 仕様変更に対する明示の保険として残置する。
unsafe impl Send for BitmapSourceGraphics {}
unsafe impl Sync for BitmapSourceGraphics {}

impl BitmapSourceGraphics {
    /// 空のBitmapSourceGraphicsを作成
    pub fn new() -> Self {
        Self { bitmap: None }
    }

    /// Bitmapへの参照を取得
    pub fn bitmap(&self) -> Option<&ID2D1Bitmap1> {
        self.bitmap.as_ref()
    }

    /// Bitmapを設定
    pub fn set_bitmap(&mut self, bitmap: ID2D1Bitmap1) {
        self.bitmap = Some(bitmap);
    }

    /// Device Lost時にBitmapを無効化
    pub fn invalidate(&mut self) {
        self.bitmap = None;
    }

    /// Bitmapが有効か判定
    pub fn is_valid(&self) -> bool {
        self.bitmap.is_some()
    }
}

impl std::fmt::Debug for BitmapSourceGraphics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitmapSourceGraphics")
            .field("bitmap", &self.bitmap.is_some())
            .finish()
    }
}
