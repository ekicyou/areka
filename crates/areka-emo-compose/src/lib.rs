//! # areka-emo-compose — emo ⑥ render engine 合成コア（三段直列チェーン 2/3）
//!
//! `areka-emo-atlas`（1/3・アトラス正本）と `emo-present`（3/3・WUC アップロード）に
//! 挟まれた中段。上流 `areka-parsers::shell` の忠実転記モデル（`Shell`）と
//! `areka-emo-atlas::AtlasTable`（premultiplied BGRA・`Placement` 解決済み）を入力に、
//! 描く計画（[`DrawPlan`]）を組む純粋層である。描いた絵の受け渡しの型 `ComposedSurface` もここに置く。
//!
//! ## パイプライン: fold → plan
//!
//! - **fold**（[`fold`]）: parser の登場順定義ストリームを single-pass で畳み込み、
//!   ターゲット展開＋除外・create/append 意味論・alias 収集を経て emo 専用 per-ghost
//!   `bevy_ecs` World（[`world`]）へ正規化定義を常駐させる。
//! - **plan**（[`plan`]）: 正規化定義から、描く順・重ね方・範囲・まとめを持つ計画（[`DrawPlan`]）を導出する。
//!   レイヤ順・`animation-sort`→ID 順の bind 合成規則・入れ子 surface・
//!   循環検出・キャンバス外形算出をここで確定する。
//!
//! 計画を絵にするのは下流（`areka-emo-present` の節と画面の外の描き手）で、この層は画素を作らない。
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
pub mod draw_plan;
pub use draw_plan::{DrawPlan, IRect, Mix, PlanItem};
pub mod bind;
mod bind_steps;
mod budget;
pub mod composed;
pub mod film;
pub use film::{FilmSheet, FilmSheets, FilmSkip, FilmSkipReason};
pub mod nesting;
pub mod normalized;
pub mod pattern;
pub mod pattern_word;
pub use bind::BindSet;
pub use composed::ComposedSurface;
pub use nesting::{
    ElementKind, FilmId, NestIssue, NestReport, NestTable, PartKey, SurfaceParts, element_kind,
    is_always_interval, rest_index,
};
pub use normalized::{NormalizedElement, SurfaceMaster, Transform};
pub use pattern::{Cell, PatternFrame, PatternState};
pub use pattern_word::{PatternWord, UnreadableTarget, call_targets, pattern_word};
pub mod world;
pub use world::{AliasMap, AtlasBinding, EmoWorld, ShellSettings, SurfaceId, SurfaceIndex};
pub mod atlas_bind;
pub mod base_image;
mod base_surface;
pub use base_image::{BaseImageReport, SurfaceImages};
pub mod boxes;
pub use boxes::{
    BoxDef, BoxIssue, BoxLayout, BoxName, BoxPlacement, BoxReport, FontFollow, fold_boxes,
};
pub mod fold;
mod outline_walk;
pub mod plan;
mod plan_always;
mod plan_base;
mod plan_bounds;
mod plan_cap;
mod plan_issues;
mod plan_mix;
use plan::BlitOp;
pub use plan_bounds::{ShellBounds, shell_bounds};
pub use plan_issues::{ShapeIssue, shape_issues};
pub mod hit_mask;
pub use hit_mask::{RegionMask, RegionMasks, region_mask};
mod hit_import;
pub use hit_import::HitRegions;
pub mod scale;
pub use scale::ScaleRatio;
pub mod text_route;
pub use text_route::{BoxSelect, SurfaceKeyOutcome, TextPlace, TextRoute, box_index};

use areka_emo_atlas::AtlasTable;

/// 計画の口: fold 済み・`bind_atlas` 済みの [`EmoWorld`] から surface の描く計画（[`DrawPlan`]）を組む。
///
/// 次の性質を持つ:
///
/// - **同期・値返し**: スレッド生成・async・channel を一切持たず、wintf の
///   visual/window/WUC 描画 API には依存しない。
/// - **結果を保持しない**: 内部フィールドは走査スクラッチ（`ops`／`scratch`・`scratch` は記録済みの印＝
///   面ごとに 1 度だけ記録するための面の番号の集合も持つ）だけで、計画をキャッシュしない。
///   surface→計画のキャッシュ・無効化は下流 `emo-present` の責務である。
/// - **buffer 再利用**: [`plan_into`] は呼び手の `out` と自身のスクラッチを再利用し、外形は束縛のときの
///   表から引く（合成のたびにたどらない）。
/// - **アトラス解決は構築時一度きり**: [`plan_into`]／[`plan`] は [`AtlasTable::resolve`] を一切呼ばず、
///   束縛済み `ElementId` の [`AtlasTable::entry`] O(1) 引きのみを行う（束縛は事前の
///   [`EmoWorld::bind_atlas`] で完了済み）。
///
/// # Preconditions
///
/// `world` は fold 済み（[`EmoWorld::build`]）かつ [`EmoWorld::bind_atlas`] 済みであること。
/// 未束縛 element は plan 段でスキップされる（非パニック）。
///
/// # 失敗経路
///
/// 合成対象 surface 不在 → [`ComposeError::SurfaceNotFound`]、外形の幅か高さが 0 の退化 →
/// [`ComposeError::EmptyComposition`]、外形が一辺か面積の上限を越える → [`ComposeError::ExtentTooLarge`]。
/// いずれも `error` ログ＋`Err`（[`crate::plan::build_plan`]
/// が分類・ログ済み）で、パニックしない。描く項が 0 でも定義層があれば正常系
/// （外形どおりの項の無い計画を `Ok` で返す）。
///
/// [`plan_into`]: Composer::plan_into
/// [`plan`]: Composer::plan
#[derive(Debug, Default)]
pub struct Composer {
    /// plan 段が積む命令のスクラッチ（毎回 `build_plan` 入口で clear・再利用）。
    ops: Vec<BlitOp>,
    /// 命令列の走査の作業場（`build_plan` が走査入口で空へ戻して再利用）。
    scratch: crate::plan::PlanScratch,
}

impl Composer {
    /// 空スクラッチの `Composer` を構築する（計画は保持しない）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 計画を組む再利用形: `surface_id` の描く順の項の列を `out` へ書く（`out.items` は入口で空にする）。
    ///
    /// 重ね方・範囲・まとめは語ごとの規則で決め、まとめの中の位置はまとめの左上から。
    /// 外形は外形の表の答えのまま。`build_plan` の `Err`（surface 不在・外形の幅か高さが 0 の退化・
    /// 外形の上限越え）はそのまま返す（記録済み＝合成器 1 つにつき面ごとに初めは `error`・後は `debug`）。
    pub fn plan_into(
        &mut self,
        out: &mut DrawPlan,
        world: &EmoWorld,
        atlas: &AtlasTable,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
    ) -> Result<(), ComposeError> {
        out.items.clear();
        let extent = crate::plan::build_plan(
            &mut self.ops,
            &mut self.scratch,
            world,
            atlas,
            surface_id,
            binds,
            pattern,
        )?;
        out.surface_id = surface_id;
        out.base_surface = self.scratch.root_base.unwrap_or(surface_id);
        out.extent = extent;
        crate::plan_mix::push_items(&mut out.items, &self.ops, &self.scratch.marks, world, atlas);
        Ok(())
    }

    /// 計画を組む新規割り当て形（[`plan_into`] へ委ねる）。
    ///
    /// [`plan_into`]: Composer::plan_into
    pub fn plan(
        &mut self,
        world: &EmoWorld,
        atlas: &AtlasTable,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
    ) -> Result<DrawPlan, ComposeError> {
        let mut out = DrawPlan {
            surface_id,
            base_surface: surface_id,
            extent: crate::plan::Extent { w: 0, h: 0 },
            items: Vec::new(),
        };
        self.plan_into(&mut out, world, atlas, surface_id, binds, pattern)?;
        Ok(out)
    }
}

/// テスト専用の `tracing` ログ捕捉ハーネス（本番影響ゼロ。
/// ログ発火経路を檻に入れるための共有土台）。`#[cfg(test)]` 限定ゆえ出荷クロージャに現れない。
#[cfg(test)]
mod log_capture;

// 検体を窓口から取得しプロセス寿命で保持する共有の受け口。
#[cfg(test)]
mod sample_test_support;

// 命令列ほかのテストが引く部品（面の組み立て・コマ・頁・焼き・記録の読み取り・命令列を引く口）。
#[cfg(test)]
pub(crate) mod plan_test_support;

#[cfg(test)]
mod golden_tests;

#[cfg(test)]
mod composer_tests;

#[cfg(test)]
mod log_firing_tests;

// 深い鎖・菱形・insert の連鎖を、入れ子をたどる走査へ同じ検体で通す。
#[cfg(test)]
mod walk_limits_tests;

// 線形の探索で 2 乗に伸びていた所を、大きな検体で通す。
#[cfg(test)]
mod linear_lookup_tests;

// 予算が数えなかった仕事の量（なめる行・重ねる画素・範囲の写し・根をまたぐ持ち込み）。
#[cfg(test)]
mod work_limits_tests;

// 予算を面の行の全部で数える（どの走査も入るときに引く）・重なりの記録・持ち込みの切り方。
#[cfg(test)]
mod row_budget_tests;

#[cfg(test)]
mod contract_tests {
    use super::{BindSet, ComposedSurface};

    /// 型 `T` が `Send` であることをコンパイル時に要求するヘルパ。
    fn _assert_send<T: Send>() {}

    /// `BindSet` / `ComposedSurface` が `Send` 所有であることをコンパイル時に固定する。
    ///
    /// いずれかが `Send` でなくなるとこのテストのコンパイルが失敗し、公開データ契約の
    /// スレッド越え受け渡し不変条件の逸脱を静的に検出する。
    #[test]
    fn public_contracts_are_send() {
        _assert_send::<BindSet>();
        _assert_send::<ComposedSurface>();
    }
}
