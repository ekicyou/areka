//! # areka-emo-compose — emo ⑥ render engine 合成コア（三段直列チェーン 2/3）
//!
//! `areka-emo-atlas`（1/3・アトラス正本）と `emo-present`（3/3・WUC アップロード）に
//! 挟まれた中段。上流 `areka-parsers::shell` の忠実転記モデル（`Shell`）と
//! `areka-emo-atlas::AtlasTable`（premultiplied BGRA・`Placement` 解決済み）を入力に、
//! 静的合成済みビットマップ `ComposedSurface` を生成する純粋層である。
//!
//! ## パイプライン: fold → plan → execute
//!
//! - **fold**（[`fold`]）: parser の登場順定義ストリームを single-pass で畳み込み、
//!   ターゲット展開＋除外・create/append 意味論・alias 収集を経て emo 専用 per-ghost
//!   `bevy_ecs` World（[`world`]）へ正規化定義を常駐させる。
//! - **plan**（[`plan`]）: 正規化定義から、バックエンド非依存の命令列（`BlitOp`）を導出する。
//!   レイヤ順・`animation-sort`→ID 順の bind 合成規則・入れ子 surface の flatten・
//!   循環検出・キャンバス外形算出をここで確定する。
//! - **execute**（[`blit`]）: 命令列を CPU 整数演算で転写する。premultiplied SourceOver・
//!   `trim_offset` 補正・クリップを経て `ComposedSurface`（[`composed`]）を得る。
//!
//! plan（命令列データ）と execute（CPU 実装）の分離が、将来のバックエンド差替えシームを成す。
//!
//! ## 制約
//!
//! Rust 2024・tokio 不使用。依存はワークスペース既存基盤のみ
//! （`areka-parsers`・`areka-emo-atlas`・`bevy_ecs`・`tracing`・`thiserror`）。
//! wintf の visual/window/WUC/描画 API には依存しない決定的な純粋合成処理であり、
//! 自らスレッド生成・async・channel を持たず、UI スレッド上の emo 専用 World に常駐する。

pub mod error;
pub mod method;

pub use method::{BlendKind, BlendMode, ComposeMethod};
pub mod bind;
pub mod composed;
pub mod normalized;
pub use bind::BindSet;
pub use composed::ComposedSurface;
pub use normalized::{NormalizedElement, SurfaceMaster, Transform};
pub mod world;
pub use world::{AliasMap, AtlasBinding, EmoWorld, ShellSettings, SurfaceId, SurfaceIndex};
pub mod fold;
pub mod atlas_bind;
pub mod plan;
pub mod blit;

#[cfg(test)]
mod golden_tests;

#[cfg(test)]
mod contract_tests {
    use super::{BindSet, ComposedSurface};

    /// 型 `T` が `Send` であることをコンパイル時に要求するヘルパ。
    fn _assert_send<T: Send>() {}

    /// Req 9.2: `BindSet` / `ComposedSurface` が `Send` 所有であることをコンパイル時に固定する。
    ///
    /// いずれかが `Send` でなくなるとこのテストのコンパイルが失敗し、公開データ契約の
    /// スレッド越え受け渡し不変条件の逸脱を静的に検出する。
    #[test]
    fn public_contracts_are_send() {
        _assert_send::<BindSet>();
        _assert_send::<ComposedSurface>();
    }
}
