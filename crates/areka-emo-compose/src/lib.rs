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
pub use error::ComposeError;
pub mod method;

pub use method::{BlendKind, BlendMode, ComposeMethod};
pub mod bind;
pub mod composed;
pub mod normalized;
pub mod pattern;
pub use bind::BindSet;
pub use pattern::{PatternFrame, PatternState};
pub use composed::ComposedSurface;
pub use normalized::{NormalizedElement, SurfaceMaster, Transform};
pub mod world;
pub use world::{AliasMap, AtlasBinding, EmoWorld, ShellSettings, SurfaceId, SurfaceIndex};
pub mod fold;
pub mod atlas_bind;
pub mod plan;
pub use plan::BlitOp;
pub mod blit;
pub mod hit;
pub use hit::{hit_region, RegionPriority};
pub mod scale;
pub use scale::{ScaleRatio, resample};

use areka_emo_atlas::AtlasTable;

/// 合成ファサード: fold 済み・`bind_atlas` 済みの [`EmoWorld`] から surface を静的合成し
/// [`ComposedSurface`]（premultiplied BGRA・要件 9.1）を返す（三段 fold→plan→execute の結線点）。
///
/// design「Composer facade（lib.rs）」に厳密に従う:
///
/// - **同期・値返し**（要件 9.3/10.4）: スレッド生成・async・channel を一切持たず、合成結果を
///   値（[`compose`]）または呼び手のバッファ（[`compose_into`]）へ直接書いて返す。wintf の
///   visual/window/WUC 描画 API には依存しない。
/// - **結果を保持しない**（要件 9.4）: `Composer` の内部フィールドは走査スクラッチ（`ops`／`visited`）
///   のみで、[`ComposedSurface`] を一切キャッシュしない。前回の合成結果は次回呼び出しへ持ち越さず、
///   surface→結果のキャッシュ・無効化は下流 `emo-present` の責務である。
/// - **buffer 再利用でゼロアロケーション定常状態**（要件 10.3）: [`compose_into`] は呼び手の `out`
///   と自身のスクラッチ（`ops`／`visited`）を再利用するため、外形が変わらない毎フレーム経路で
///   途中アロケーションを発生させず O(elements) で合成する。
/// - **アトラス解決は構築時一度きり**（要件 4.3・design 主要決定）: hot path（[`compose_into`]/
///   [`compose`]）は [`AtlasTable::resolve`] を一切呼ばず、束縛済み `ElementId` の
///   [`AtlasTable::entry`] O(1) 引きのみを行う（束縛は事前の [`EmoWorld::bind_atlas`] で完了済み）。
///
/// # Preconditions
///
/// `world` は fold 済み（[`EmoWorld::build`]）かつ [`EmoWorld::bind_atlas`] 済みであること。
/// 未束縛 element は plan 段でスキップされる（非パニック）。
///
/// # 失敗経路（要件 10.5）
///
/// 合成対象 surface 不在 → [`ComposeError::SurfaceNotFound`]、定義層皆無で外形 0×0 の退化 →
/// [`ComposeError::EmptyComposition`]。いずれも `error` ログ＋`Err`（[`crate::plan::build_plan`]
/// が分類・ログ済み）で、パニックしない（要件 1.4）。描画可能命令ゼロでも定義層があれば正常系
/// （外形どおりの全透明 [`ComposedSurface`] を `Ok` で返す・要件 6.6）。
#[derive(Debug, Default)]
pub struct Composer {
    /// plan 段が積む転写命令のスクラッチ（毎回 `build_plan` 入口で clear・再利用）。
    ops: Vec<BlitOp>,
    /// 入れ子 flatten の循環検出用祖先スタックのスクラッチ（`build_plan` が走査入口で clear・再利用）。
    visited: Vec<u32>,
}

impl Composer {
    /// 空スクラッチの `Composer` を構築する（合成結果は保持しない・要件 9.4）。
    pub fn new() -> Self {
        Composer {
            ops: Vec::new(),
            visited: Vec::new(),
        }
    }

    /// 合成先バッファ再利用形（毎フレーム経路・定常状態アロケーションなし・要件 10.3）。
    ///
    /// `world`／`atlas`／`surface_id`／`active_binds` から plan を導出（[`crate::plan::build_plan`]）し、
    /// 命令列を `out` へ転写（[`crate::blit::execute`]）する。`out` と自身のスクラッチ（`ops`／`visited`）
    /// を再利用するため、外形が変わらない反復呼び出しで再割り当てを起こさない。
    ///
    /// `build_plan` が `Err`（surface 不在・定義層皆無退化）を返せば、そのまま伝播する（既に `error`
    /// ログ済み・要件 10.5）。`Ok(extent)` なら `execute` が `out` を `extent` へ `resize_and_clear`
    /// してから転写し、`Ok(())` を返す。描画可能命令ゼロ（全透明・空 bind 集合）でも正常系として
    /// 外形どおりの全透明バッファを書く（要件 6.6）。
    ///
    /// # hot path 不変条件（要件 4.3）
    ///
    /// 本経路は [`AtlasTable::resolve`] を一切呼ばない。plan/blit は束縛済み `ElementId` の
    /// [`AtlasTable::entry`] O(1) 引きのみを用いる（アトラス束縛は事前の
    /// [`EmoWorld::bind_atlas`] で一度きり実施済み）。
    pub fn compose_into(
        &mut self,
        out: &mut ComposedSurface,
        world: &EmoWorld,
        atlas: &AtlasTable,
        surface_id: u32,
        active_binds: &BindSet,
        pattern: &PatternState,
    ) -> Result<(), ComposeError> {
        // plan: 命令列＋外形を導出（Err は既に error ログ済み・そのまま伝播・要件 10.5）。
        // ops／visited は build_plan 入口で clear され再利用される（要件 10.3）。
        let extent = crate::plan::build_plan(
            &mut self.ops,
            &mut self.visited,
            world,
            atlas,
            surface_id,
            active_binds,
            pattern,
        )?;
        // execute: 命令列を out へ premultiplied SourceOver 転写（out は resize_and_clear で再利用）。
        crate::blit::execute(out, extent, &self.ops, atlas);
        Ok(())
    }

    /// 新規割り当て形（初回・テスト向け便宜）。
    ///
    /// 新しい [`ComposedSurface`] を確保し、[`compose_into`] へ委譲して合成結果を書き、値で返す。
    /// `Composer` はこの結果を保持しない（要件 9.4）。毎フレーム経路では代わりに [`compose_into`]
    /// でバッファを再利用すること（要件 10.3）。
    ///
    /// [`compose_into`]: Composer::compose_into
    pub fn compose(
        &mut self,
        world: &EmoWorld,
        atlas: &AtlasTable,
        surface_id: u32,
        active_binds: &BindSet,
        pattern: &PatternState,
    ) -> Result<ComposedSurface, ComposeError> {
        // 便宜上 0×0 で確保し、compose_into 内の resize_and_clear が外形へ合わせて伸長する。
        let mut out = ComposedSurface::new(0, 0);
        self.compose_into(&mut out, world, atlas, surface_id, active_binds, pattern)?;
        Ok(out)
    }
}

/// テスト専用の `tracing` ログ捕捉ハーネス（本番影響ゼロ・要件 1.4/3.3/7.3/8.4/10.5 の
/// ログ発火経路を檻に入れるための共有土台）。`#[cfg(test)]` 限定ゆえ出荷クロージャに現れない。
#[cfg(test)]
mod log_capture;

#[cfg(test)]
mod golden_tests;

#[cfg(test)]
mod composer_tests;

#[cfg(test)]
mod log_firing_tests;

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
