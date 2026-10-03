//! 箱の束の受け取りと、箱を隠す印（`actor.rs` の子・design.md「箱の登録と同期」）。
//!
//! 箱の束（置き場所の表・`\s` の解決の閉包・フォントを探す場所の順）は結線がシェルを読むたびに
//! （装着の相とシェルの切替）渡す。受け取ると前の箱の面を片付け、箱の場所の文字を捨て、
//! 新しい表を純粋状態へ入れる（要件 6.8）。普通のバルーンの場所には触れない。
//!
//! 窓の子の entity を消すので `World` に触れる（純粋な走査の外）。

use std::collections::BTreeMap;
use std::path::PathBuf;

use areka_emo_compose::{BoxLayout, BoxName};
use areka_sakura::contract::ActorKey;
use bevy_ecs::prelude::World;
use tracing::debug;

use super::{SurfaceKeyResolver, TextLayerRuntime};
use crate::draw::{DEFAULT_BALLOON_BACKGROUND, ResolvedFont};
use crate::place::{PlaceKey, TextPlace};
use crate::state::BoxTraits;

impl TextLayerRuntime {
    /// 箱の束を受け取る（UI スレッドから・装着の相とシェルの切替が呼ぶ）。
    ///
    /// 1. 前の箱の面（シェルの窓の子）を片付け、箱の場所ごとの実行時の表も捨てる。
    /// 2. 箱の名前ごとの既定の 2 層と種類を状態へ渡す。2 層はブレスの定義を白を混色の相手として
    ///    解いた `font.looks`（拡大率に依らない・design.md「装飾の持ち運び」）。
    /// 3. 新しい表を状態へ入れる。状態は箱の場所の文字を捨て、各スコープの今のサーフェス番号を保ち、
    ///    行き先を新しい表での既定へ引き直す（要件 6.8）。種類は 3 の持ち運びの判定が引くので、
    ///    **2 を 3 より先に**行う（tasks.md Implementation Notes 5.4）。
    /// 4. `\s` の解決の閉包とフォントを探す場所の順（要件 3.10）を入れ替える。
    pub fn set_box_layout(
        &mut self,
        world: &mut World,
        layout: BoxLayout,
        resolve: SurfaceKeyResolver,
        font_dirs: Vec<PathBuf>,
    ) {
        let is_box = |key: &PlaceKey| key.place != TextPlace::Balloon;
        let stale: Vec<PlaceKey> = self
            .surfaces
            .keys()
            .filter(|k| is_box(k))
            .cloned()
            .collect();
        for key in &stale {
            if let Some(render) = self.surfaces.remove(key) {
                render.surface.despawn_window_child(world);
            }
        }
        self.routing.retain(|k, _| !is_box(k));
        self.layout_input.retain(|k, _| !is_box(k));
        self.unresolved_warned.retain(|k| !is_box(k));
        self.choice_hover.retain(|k, _| !is_box(k));
        self.choice_snapshot.retain(|k, _| !is_box(k));

        let index: BTreeMap<u32, Vec<BoxName>> = layout
            .surfaces()
            .map(|(id, placements)| (id, placements.iter().map(|p| p.name.clone()).collect()))
            .collect();
        // 置き場所の表は名前のブレスが採られたものだけを載せる（`fold_boxes`）ので、定義は必ず引ける。
        let traits: BTreeMap<BoxName, BoxTraits> = index
            .values()
            .flatten()
            .filter_map(|name| {
                let def = layout.def(name);
                if def.is_none() {
                    tracing::warn!(
                        name = name.as_str(),
                        "emo-text: 箱の名前に定義が無いので既定の見た目を作らない"
                    );
                }
                def.map(|def| (name.clone(), def))
            })
            .map(|(name, def)| {
                // 2 層は大きさに依らないので、フォントの解決だけを通す
                // （`ResolvedBalloonText::resolve_with_background` の `font` と同じ導出）。
                let looks =
                    ResolvedFont::resolve_with_background(&def.model, DEFAULT_BALLOON_BACKGROUND)
                        .looks;
                (
                    name,
                    BoxTraits {
                        looks,
                        follow: def.follow,
                    },
                )
            })
            .collect();
        debug!(
            dropped_surfaces = stale.len(),
            boxes = traits.len(),
            surfaces = index.len(),
            font_dirs = ?font_dirs,
            "箱の束を受け取った（前の箱の面を片付け、表を差し替える）"
        );
        self.state.set_box_traits(traits);
        self.state.set_box_index(index);
        self.surface_resolver = Some(resolve);
        self.box_font_dirs = font_dirs;
    }

    /// 箱の `font.name` のフォントファイルを探す場所の順（シェル → ゴースト・要件 3.10）。
    /// 束が来るまでは空。読み込みそのものは `balloon-font-file` が行う。
    pub fn box_font_dirs(&self) -> &[PathBuf] {
        &self.box_font_dirs
    }

    /// スコープに「箱を隠す印」を立てる（時間切れ・利用者の中断の発行が呼ぶ）。次の台詞の頭
    /// （`ClearAll`）で文字の層が自分で下ろす。
    pub fn hide_boxes(&mut self, actor: &ActorKey) {
        debug!(actor = %actor, "箱を隠す印を立てた（次の台詞の頭まで）");
        self.hidden_boxes.insert(actor.clone());
    }
}
