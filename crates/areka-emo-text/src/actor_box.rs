//! 箱の束の受け取りと、箱を隠す印（`actor.rs` の子・design.md「箱の登録と同期」）。
//!
//! 箱の束（置き場所の表・`\s` の解決の閉包・フォントを探す場所の順）は結線がシェルを読むたびに
//! （装着の相とシェルの切替）渡す。受け取ると前の箱の面を片付け、箱の場所の文字を捨て、
//! 新しい表を純粋状態へ入れる（要件 6.8）。普通のバルーンの場所には触れない。
//!
//! 毎フレームの箱の同期（`sync_boxes`）は、文字を持つ箱の場所ごとに「あるべき置き場所」を導いて
//! 登録（装着先・配置の入力・置き場所）を合わせる。面そのものは次の提示が作る。
//!
//! 窓の子の entity を消すので `World` に触れる（純粋な走査の外）。

use std::collections::BTreeMap;
use std::path::PathBuf;

use areka_emo_compose::{BoxLayout, BoxName, BoxPlacement};
use areka_emo_present::TextSlotView;
use areka_sakura::contract::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use tracing::{debug, warn};

use super::{ResolvedBalloonText, SurfaceKeyResolver, TextLayerRuntime, TextSlotBinding};
use crate::draw::{DEFAULT_BALLOON_BACKGROUND, ResolvedFont};
use crate::place::{PlaceKey, TextPlace};
use crate::region::{ImagePx, ScaleContract};
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
        self.box_sites.clear();
        self.box_overflow_warned.clear();

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
        self.box_layout = layout;
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

/// 箱の場所の「あるべき置き場所」（登録と比べる値・はみ出しの判定の材料）。
struct DesiredBox {
    /// 装着先（シェルの窓・差し込み口・拡大率。画像の大きさは箱の大きさ）。
    binding: TextSlotBinding,
    /// 今のサーフェスでの置き場所。
    site: BoxPlacement,
    /// 今のサーフェス番号。
    surface: u32,
    /// シェルの窓のサーフェスの画像の大きさ（native 原寸）。
    surface_size: (u32, u32),
}

impl TextLayerRuntime {
    /// 毎フレームの箱の同期（UI スレッドから・結線のテキストの拡大率の相が呼ぶ）。
    ///
    /// `shell_views` はスコープごとのシェルの窓の [`TextSlotView`]（未確立は `None`・渡されない
    /// スコープも未確立と同じ）。文字を持つ箱の場所すべてについて「あるべき置き場所」を導き、
    /// 登録済みと違えば面を片付けて登録し直し（文字の進み具合は保つ）、同じなら何もしない。
    pub fn sync_boxes(
        &mut self,
        world: &mut World,
        shell_views: &[(ActorKey, Option<TextSlotView>)],
    ) {
        let shells: Vec<(ActorKey, Option<TextSlotBinding>)> = shell_views
            .iter()
            .map(|(actor, view)| (actor.clone(), view.as_ref().map(TextSlotBinding::from_view)))
            .collect();
        self.sync_box_bindings(world, &shells);
    }

    /// [`sync_boxes`](Self::sync_boxes) の内側（view を binding に読み替えた後）。`TextSlotView` は
    /// emo-present の私有の欄の型でクレートの中の檻から作れないので、判断をこの層で檻に入れる。
    pub(super) fn sync_box_bindings(
        &mut self,
        world: &mut World,
        shells: &[(ActorKey, Option<TextSlotBinding>)],
    ) {
        let desired: BTreeMap<PlaceKey, DesiredBox> = self
            .state
            .places()
            .filter(|(key, state)| key.place != TextPlace::Balloon && !state.items().is_empty())
            .filter_map(|(key, _)| Some((key.clone(), self.desired_box(key, shells)?)))
            .collect();
        let stale: Vec<PlaceKey> = self
            .box_sites
            .keys()
            .filter(|key| {
                desired.get(*key).is_none_or(|want| {
                    self.routing.get(*key) != Some(&want.binding)
                        || self.box_sites.get(*key) != Some(&want.site)
                })
            })
            .cloned()
            .collect();
        for key in &stale {
            self.unregister_box(world, key);
        }
        for (key, want) in desired {
            if !self.box_sites.contains_key(&key) {
                self.register_box(key, want);
            }
        }
    }

    /// あるべき置き場所: 名前が今のサーフェスの箱の列にあり、箱を隠す印が立っておらず、
    /// シェルの窓が確立しているときだけ在る。
    fn desired_box(
        &self,
        key: &PlaceKey,
        shells: &[(ActorKey, Option<TextSlotBinding>)],
    ) -> Option<DesiredBox> {
        let TextPlace::Box(name) = &key.place else {
            return None;
        };
        if self.hidden_boxes.contains(&key.actor) {
            return None;
        }
        let shell = shells
            .iter()
            .find(|(actor, _)| *actor == key.actor)
            .and_then(|(_, shell)| *shell)?;
        let surface = self.state.current_surface(&key.actor)?;
        let site = self
            .box_layout
            .placements(surface)
            .iter()
            .find(|p| p.name == *name)?
            .clone();
        let size = self.box_layout.def(name)?.size;
        let contract = ScaleContract::new(shell.scale, None);
        let physical = (
            contract.physical_extent(ImagePx(size.0 as f32)),
            contract.physical_extent(ImagePx(size.1 as f32)),
        );
        Some(DesiredBox {
            binding: TextSlotBinding::new(shell.slot, shell.window, shell.scale, physical, size),
            site,
            surface,
            surface_size: shell.image_size,
        })
    }

    /// 箱の場所を登録する。配置の入力は普通のバルーンと同じ式で、箱の大きさを画像の大きさとし、
    /// 背景は白を混色の相手とする（要件 3.1〜3.3）。はみ出す置き場所は採ったうえで、
    /// （サーフェス番号, 名前）ごとに 1 度だけ警告する。
    fn register_box(&mut self, key: PlaceKey, want: DesiredBox) {
        let Some(def) = self.box_layout.def(&want.site.name) else {
            // `desired_box` が定義の在る名前だけを通すので構造上起こらない。
            warn!(
                name = want.site.name.as_str(),
                "箱の定義が無いので登録しない（構造不変の破れ）"
            );
            return;
        };
        let resolved = ResolvedBalloonText::resolve_with_background(
            &def.model,
            def.size,
            DEFAULT_BALLOON_BACKGROUND,
        );
        let (w, h) = (i64::from(def.size.0), i64::from(def.size.1));
        let (sw, sh) = (
            i64::from(want.surface_size.0),
            i64::from(want.surface_size.1),
        );
        let site = &want.site;
        let overflows = site.x < 0 || site.y < 0 || site.x + w > sw || site.y + h > sh;
        if overflows
            && self
                .box_overflow_warned
                .insert((want.surface, site.name.clone()))
        {
            warn!(
                surface = want.surface,
                name = site.name.as_str(),
                rect = ?(site.x, site.y, w, h),
                surface_size = ?want.surface_size,
                "箱がサーフェスの画像からはみ出している——採ったうえで、はみ出した部分は窓の端で切れる"
            );
        }
        debug!(
            actor = %key.actor,
            name = site.name.as_str(),
            surface = want.surface,
            element = site.element,
            x = site.x,
            y = site.y,
            k = want.binding.scale,
            "箱の置き場所を登録した（面は次の提示で作る）"
        );
        self.routing.insert(key.clone(), want.binding);
        self.layout_input.insert(key.clone(), resolved);
        self.box_sites.insert(key, want.site);
    }

    /// 箱の場所の登録を外し、面があれば窓の子ごと片付ける（純粋状態の文字には触れない）。
    fn unregister_box(&mut self, world: &mut World, key: &PlaceKey) {
        self.routing.remove(key);
        self.layout_input.remove(key);
        self.box_sites.remove(key);
        // 見えない箱の当たり行を残さない（次の提示で再び導く）。
        self.choice_snapshot.remove(key);
        if let Some(render) = self.surfaces.remove(key) {
            render.surface.despawn_window_child(world);
        }
        debug!(actor = %key.actor, place = ?key.place, "箱の置き場所の登録を外した（文字は保つ）");
    }

    /// 箱の面を窓の `Children`（`children`）のどこへ挿すか: 差し込み口の直後から、element番号の
    /// 大きい順（絵より前＝手前・要件 3.7・3.9）。同じスコープで面を持つ、より大きい element番号の
    /// 箱の数だけ後ろへずらす。差し込み口が子に無い・登録されていない箱は `None`。
    #[cfg_attr(not(test), allow(dead_code))] // 箱の面の提示（7.5）が使う
    pub(super) fn box_child_index(&self, children: &[Entity], place: &PlaceKey) -> Option<usize> {
        let element = self.box_sites.get(place)?.element;
        let slot = self.routing.get(place)?.slot;
        let slot_at = children.iter().position(|child| *child == slot)?;
        let in_front = self
            .box_sites
            .iter()
            .filter(|(key, site)| {
                key.actor == place.actor
                    && site.element > element
                    && self.surfaces.contains_key(*key)
            })
            .count();
        Some(slot_at + 1 + in_front)
    }
}
