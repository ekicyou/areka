//! `EmoWorld`: emo 専用 per-ghost `bevy_ecs` World（wintf 本体 World とは分離）。
//!
//! 正規化 Surface 定義の常駐点。`SurfaceIndex`／`AliasMap`／`ShellSettings` をリソースとして、
//! `SurfaceId`／`SurfaceMaster`／`AtlasBinding` をコンポーネントとして保持する。スケール前提で
//! 定義・構造のみを保持し、合成済みビットマップ（大容量）は World に永続保持しない。本 spec では
//! Schedule/System を持たず、fold/compose が `&mut World`／`&World` を取る受動データストアとして使う。

use std::collections::{BTreeMap, HashMap};

use areka_parsers::shell::{Shell, SortOrder};
use bevy_ecs::prelude::{Component, Entity, Resource};
use bevy_ecs::world::World;

use crate::normalized::SurfaceMaster;

/// surface 番号（entity のドメインキー・疎 id の明示・要件 1.1）。
///
/// `SurfaceMaster.id` と同値だが、疎 id 集合を entity のクエリ／ソート単位として
/// 明示するために独立 component として持つ（design「Domain Model」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct SurfaceId(pub u32);

/// element と平行なアトラス束縛結果（`resolve` は構築時一度きり・要件 4.3）。
///
/// `SurfaceMaster.elements` と同じ添字で並ぶ `Option<ElementId>`（`None`＝未解決）。
/// 型スキーマは本 task で定義し、実際の束縛（`bind_atlas`）は AtlasBinder（task 4）が挿入する。
#[derive(Debug, Clone, PartialEq, Component)]
pub struct AtlasBinding(pub Vec<Option<areka_emo_atlas::ElementId>>);

/// 疎 surface id → `Entity` の O(1) 解決表（要件 1.1）。
///
/// entity 集合と常に一対一（design「不変条件」）。画素バッファは保持しない（要件 10.6）。
#[derive(Debug, Default, Resource)]
pub struct SurfaceIndex(pub HashMap<u32, Entity>);

/// `kero.surface.alias` の alias キー → 順序付き id リスト（要件 3.1/3.2）。
///
/// 同一キーは後勝ち確定済み（fold が上書きする・design「Domain Model」）。キーで決定的に
/// 並ぶよう `BTreeMap` を用いる。
#[derive(Debug, Default, Resource)]
pub struct AliasMap(pub BTreeMap<String, Vec<u32>>);

/// `animation-sort`／`collision-sort` の実効ソート順キー（要件 1.6/5.6）。
///
/// いずれも未指定は `None`。既定の適用（`animation_sort` 未指定→`Descend` 等）は
/// クエリ側（[`EmoWorld::animation_sort`]）で行い、リソースは転記値をそのまま保持する。
#[derive(Debug, Default, Resource)]
pub struct ShellSettings {
    /// `animation-sort` の転記値（未指定は `None`・既定解釈はクエリ側）。
    pub animation_sort: Option<SortOrder>,
    /// `collision-sort` の転記値（未指定は `None`・既定 none＝記述順はクエリ側）。
    pub collision_sort: Option<SortOrder>,
}

/// emo 専用 per-ghost `bevy_ecs` World（wintf 本体 World とは分離・要件 1.8/10.4）。
///
/// surface 1件＝entity 1件で正規化 Surface 定義を常駐させ、公開クエリを提供する受動データ
/// ストア。本 spec では Schedule/System を所有せず（design 決定3）、fold/compose が
/// `&mut World`／`&World` を排他/共有で受けて処理する。ゴースト unload ＝本型 drop で全 entity
/// を一掃する（per-ghost lifecycle）。
///
/// **不変条件（要件 10.6）**: 画素バッファを保持する component/Resource を持たない。
/// `SurfaceIndex` と entity 集合は常に一対一。
pub struct EmoWorld {
    world: World,
}

impl EmoWorld {
    /// `Shell` から single-pass fold で構築する（寛容・非パニック・欠落は warn ログ）。
    ///
    /// 本 task は骨組み: 既定リソースを備えた空 World を用意し、fold による entity 常駐は
    /// 後続 task（3.2）が内部 population 段（[`EmoWorld::populate_from_shell`]）へ差し込む。
    /// 空 `Shell` に対しては entity ゼロの空 World を返す（`surface_ids()` 空・`surface(_)` None）。
    pub fn build(shell: &Shell) -> EmoWorld {
        let mut world = World::new();
        world.insert_resource(SurfaceIndex::default());
        world.insert_resource(AliasMap::default());
        world.insert_resource(ShellSettings {
            animation_sort: shell.animation_sort,
            collision_sort: shell.collision_sort,
        });

        let mut emo = EmoWorld { world };
        emo.populate_from_shell(shell);
        emo
    }

    /// fold による entity 常駐段（single-pass fold への唯一の呼び出し口）。
    ///
    /// 空 `Shell`（surface/append/alias 皆無）に対しては何も積まず、World を空のまま保つ。
    /// plain `surface` ヘッダの展開＝全 id 新設（task 3.2）は [`crate::fold::fold_shell`] が担う。
    /// append/alias の意味論は後続 task が同 fold 経路へ差し込む。
    fn populate_from_shell(&mut self, shell: &Shell) {
        crate::fold::fold_shell(&mut self.world, shell);
    }

    /// アトラス束縛（`resolve` は本呼び出しの一度きり・要件 4.3）。
    ///
    /// `build` 後・compose 前に一度呼ぶ（design Preconditions）。全 surface entity の
    /// `SurfaceMaster.elements` を `AtlasTable::resolve` で `ElementId` へ解決し、平行な
    /// [`AtlasBinding`] を各 entity へ挿入する。未解決 element はパニックせず `warn`＋`None`。
    /// 束縛後の compose 経路に `resolve` は現れない（毎フレーム O(1) の `entry` 引き）。
    pub fn bind_atlas(&mut self, atlas: &areka_emo_atlas::AtlasTable, set: areka_emo_atlas::SetId) {
        crate::atlas_bind::bind_atlas(&mut self.world, atlas, set);
    }

    /// 正規化 Surface 定義の公開クエリ（要件 1.3・存在しない id は `None`）。
    pub fn surface(&self, id: u32) -> Option<&SurfaceMaster> {
        let entity = *self.world.resource::<SurfaceIndex>().0.get(&id)?;
        self.world.get::<SurfaceMaster>(entity)
    }

    /// 常駐する surface id を昇順（決定的）で列挙する。
    pub fn surface_ids(&self) -> impl Iterator<Item = u32> + '_ {
        let mut ids: Vec<u32> = self.world.resource::<SurfaceIndex>().0.keys().copied().collect();
        ids.sort_unstable();
        ids.into_iter()
    }

    /// alias 解決表を所有権ごと取り出す（`resolve_alias` と同一の `AliasMap` を複製・要件 2.2/2.3/7.3）。
    ///
    /// 別スレッドで動く seriko アクターが所有する Send スナップショット（[`EmoWorld`] 自身は
    /// `World`（bevy_ecs）を内包し `Send`／`Sync` を実装しないため move 不可）。追加のみで
    /// 既存の借用ベース [`EmoWorld::resolve_alias`] は不変。二重定義せず [`AliasMap`] を直接
    /// クローンするため、借用解決結果からドリフトしない（design 決定1）。
    pub fn alias_snapshot(&self) -> BTreeMap<String, Vec<u32>> {
        self.world.resource::<AliasMap>().0.clone()
    }

    /// alias 解決（要件 3.1/3.3・未解決は warn ログ＋`None`）。
    pub fn resolve_alias(&self, key: &str) -> Option<&[u32]> {
        match self.world.resource::<AliasMap>().0.get(key) {
            Some(ids) => Some(ids.as_slice()),
            None => {
                tracing::warn!(target: "areka_emo_compose", key, "alias キー未解決");
                None
            }
        }
    }

    /// 実効 animation ソート順（未指定は ukadoc 既定 `Descend`・要件 1.6/5.6）。
    pub fn animation_sort(&self) -> SortOrder {
        self.world
            .resource::<ShellSettings>()
            .animation_sort
            .unwrap_or(SortOrder::Descend)
    }

    /// 実効 collision ソート順（未指定は `None`＝記述順・要件 1.6/5.6）。
    pub fn collision_sort(&self) -> Option<SortOrder> {
        self.world.resource::<ShellSettings>().collision_sort
    }

    /// 将来の seriko system 統合用の脱出口（本 spec では未使用・design 決定3）。
    pub fn world(&self) -> &World {
        &self.world
    }

    /// 将来の seriko system 統合用の脱出口（可変・fold/bind が `&mut World` を受ける口）。
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空 `Shell`（surface/append/alias 皆無・sort 未指定）。
    fn empty_shell() -> Shell {
        Shell {
            surfaces: Vec::new(),
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions: Vec::new(),
        }
    }

    /// 空 `Shell` から構築した World は entity ゼロ: `surface_ids()` 空・`surface(_)` None（要件 1.1/1.3）。
    #[test]
    fn empty_shell_yields_empty_world() {
        let world = EmoWorld::build(&empty_shell());
        assert_eq!(world.surface_ids().count(), 0);
        assert_eq!(world.surface(0), None);
        assert_eq!(world.surface(1000), None);
    }

    /// `animation_sort` 未指定は ukadoc 既定 `Descend`・`collision_sort` 未指定は `None`（要件 1.6/5.6）。
    #[test]
    fn default_sort_orders() {
        let world = EmoWorld::build(&empty_shell());
        assert_eq!(world.animation_sort(), SortOrder::Descend);
        assert_eq!(world.collision_sort(), None);
    }

    /// 転記された sort 値はそのまま実効値になる（既定に潰さない）。
    #[test]
    fn explicit_sort_orders_are_preserved() {
        let mut shell = empty_shell();
        shell.animation_sort = Some(SortOrder::Ascend);
        shell.collision_sort = Some(SortOrder::Descend);
        let world = EmoWorld::build(&shell);
        assert_eq!(world.animation_sort(), SortOrder::Ascend);
        assert_eq!(world.collision_sort(), Some(SortOrder::Descend));
    }

    /// 未解決 alias キーは `None`（パニックしない・要件 3.3）。
    #[test]
    fn resolve_missing_alias_returns_none() {
        let world = EmoWorld::build(&empty_shell());
        assert_eq!(world.resolve_alias("nope"), None);
    }

    /// emo2 fixture（実上流シェル）の `surfaces.txt` を UTF-8 で読み parse して `Shell` を得る。
    ///
    /// `fold.rs` の `emo2_shell()` と同一の読込経路（`read_to_string`＋`areka_parsers::shell::parse`）。
    /// COM／WIC／atlas bake は経由しない純粋な parse ゆえ headless で走る。
    fn emo2_shell() -> Shell {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
        areka_parsers::shell::parse(&content)
    }

    /// `alias_snapshot()` が返す所有 `BTreeMap` は、同一 `EmoWorld` の全キーに対し
    /// 借用ベース `resolve_alias(key)` と完全一致する（スナップショットが `AliasMap` から
    /// 静かにドリフトしないための檻・要件 2.2/2.3/7.3）。
    #[test]
    fn alias_snapshot_matches_resolve_alias() {
        let world = EmoWorld::build(&emo2_shell());
        let snap = world.alias_snapshot();

        // 非自明性: emo2 は alias を含む（空マップの vacuous pass を防ぐ）。
        assert!(!snap.is_empty(), "emo2 fixture は alias を含む");

        // 全キーを走査（spot-check しない）: 各キーで借用解決結果と所有スナップショットが一致。
        for (key, ids) in &snap {
            let borrowed = world
                .resolve_alias(key)
                .unwrap_or_else(|| panic!("スナップショットのキー `{key}` は resolve_alias でも解決する"));
            assert_eq!(
                borrowed,
                ids.as_slice(),
                "キー `{key}` の借用解決結果と所有スナップショットが要素等価",
            );
        }
    }
}
