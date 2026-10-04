//! 箱の束の受け取りと、箱を隠す印（`actor.rs` の子・design.md「箱の登録と同期」）。
//!
//! 箱の束（置き場所の表・`\s` の解決の閉包・フォントを探す場所の順）は結線がシェルを読むたびに
//! （装着の相とシェルの切替）渡す。受け取ると前の箱の面を片付け、箱の場所の文字を捨て、
//! 新しい表を純粋状態へ入れる（要件 6.8）。普通のバルーンの場所には触れない。
//!
//! 毎フレームの箱の同期（`sync_boxes`）は、文字を持つ箱の場所ごとに、結線が渡す「シェルの窓が
//! いま表示している絵の番号」から「あるべき置き場所」を導いて登録（装着先・配置の入力・置き場所）を
//! 合わせる。面そのものは次の提示が作る。台本の `\s` で受け取った番号は文字の行き先だけに使い
//! （`state_route.rs`）、この部品は読まない。
//!
//! 窓の子の entity を消すので `World` に触れる（純粋な走査の外）。

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use areka_emo_compose::{BoxLayout, BoxName, BoxPlacement};
use areka_emo_present::TextSlotView;
use areka_sakura::contract::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use tracing::{debug, warn};

use super::{
    ResolvedBalloonText, SurfaceKeyResolver, TextLayerRuntime, TextSlotBinding, attach, decoration,
};
use crate::choice::HitRectPx;
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
        self.box_definition_warned.clear();
        self.shown_boxes.clear();

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
        self.shown_boxes.remove(actor);
    }

    /// 最後に提示したフレームで文字が 1 字以上見えていた箱の四角（手前＝element番号の大きい順・
    /// シェルの窓の物理 px）。`\c`・台詞の頭・箱を隠す印・箱の束の差し替え・絵の切替と非表示
    /// （箱の同期が登録を外す）で出なくなった箱は、次の提示を待たずに外れている。台本の `\s` の
    /// 受け取りだけでは外れない（絵が替わるまで箱は前の絵の置き場所に出ている）。
    pub fn shown_boxes(&self, actor: &ActorKey) -> &[ShownBox] {
        self.shown_boxes.get(actor).map_or(&[], Vec::as_slice)
    }

    /// 普通のバルーンの窓に今出ている文字の数（要件 5.1・5.2）: スコープの今のサーフェスに箱が
    /// あれば 0（窓を出さない）、無ければ普通のバルーンの場所の見えている文字の数。
    pub fn balloon_shown_glyphs(&self, actor: &ActorKey, talk_time: f64) -> usize {
        let has_boxes = self
            .state
            .current_surface(actor)
            .is_some_and(|surface| !self.box_layout.placements(surface).is_empty());
        if has_boxes {
            0
        } else {
            self.state.visible_glyphs(actor, talk_time)
        }
    }
}

/// 文字の出ている箱 1 つ（[`TextLayerRuntime::shown_boxes`] の要素）。
#[derive(Clone, Debug, PartialEq)]
pub struct ShownBox {
    /// 箱の名前（`balloon.*`ブレスの名前）。
    pub name: BoxName,
    /// 今のサーフェスでの element番号（大きいほど手前・要件 3.9）。
    pub element: u32,
    /// 箱の四角（シェルの窓の物理 px＝`(X, Y, X+幅, Y+高さ) × 拡大率`）。
    pub rect: HitRectPx,
}

impl TextLayerRuntime {
    /// 場所の左上の、窓の中の位置（image px）: 箱は登録済みの置き場所の (X,Y)、普通のバルーンと
    /// 登録の無い場所は (0,0)。面の装着位置と選択肢の当たり行へ足す。
    pub(super) fn box_origin(&self, place: &PlaceKey) -> (f32, f32) {
        self.box_sites
            .get(place)
            .map_or((0.0, 0.0), |site| (site.x as f32, site.y as f32))
    }

    /// 提示のたびに写しを作り直す（`present_frame` の最後）。`presented` はこのフレームで提示に
    /// 成功した場所。箱の場所で、今も出ていてよく（[`box_still_shown`](Self::box_still_shown)）、
    /// 文字が 1 字以上見えているものだけを、スコープごとに手前から並べる。
    pub(super) fn refresh_shown_boxes(&mut self, presented: &[PlaceKey], talk_time: f64) {
        let mut shown: HashMap<ActorKey, Vec<ShownBox>> = HashMap::new();
        for key in presented {
            let TextPlace::Box(name) = &key.place else {
                continue;
            };
            let visible = self
                .state
                .place_state(key)
                .map_or(0, |s| s.reveal().visible(talk_time));
            if visible == 0 || !self.box_still_shown(key) {
                continue;
            }
            // `box_still_shown` が登録（置き場所）を確かめ済み。装着先と定義は登録と対で在る。
            let (Some(site), Some(binding), Some(def)) = (
                self.box_sites.get(key),
                self.routing.get(key),
                self.box_layout.def(name),
            ) else {
                warn!(
                    name = name.as_str(),
                    "登録済みの箱の装着先か定義が無い（構造不変の破れ）——写しに載せない"
                );
                continue;
            };
            let contract = ScaleContract::new(binding.scale, None);
            let at = |v: i64| contract.to_physical(ImagePx(v as f32)).0;
            shown.entry(key.actor.clone()).or_default().push(ShownBox {
                name: name.clone(),
                element: site.element,
                rect: HitRectPx {
                    left: at(site.x),
                    top: at(site.y),
                    right: at(site.x + i64::from(def.size.0)),
                    bottom: at(site.y + i64::from(def.size.1)),
                },
            });
        }
        for boxes in shown.values_mut() {
            boxes.sort_by_key(|b| std::cmp::Reverse(b.element));
        }
        self.shown_boxes = shown;
    }

    /// 出なくなった箱を写しからその場で外す（cue の適用のたび）。
    pub(super) fn prune_shown_boxes(&mut self) {
        let mut shown = std::mem::take(&mut self.shown_boxes);
        for (actor, boxes) in shown.iter_mut() {
            boxes.retain(|b| {
                self.box_still_shown(&PlaceKey {
                    actor: actor.clone(),
                    place: TextPlace::Box(b.name.clone()),
                })
            });
        }
        shown.retain(|_, boxes| !boxes.is_empty());
        self.shown_boxes = shown;
    }

    /// 箱の場所が今も描かれたままでよいか: 登録があり、隠す印が無く、文字を持つ。登録は直近の
    /// 同期が絵の番号から導いた置き場所で、絵はフレームの外では替わらないので、登録があること
    /// 自体が「絵と合っている」ことを表す。台本の `\s` の受け取り（行き先だけが替わる）と
    /// `\b[名前]`（要件 4.5）では外さない。`\c`・台詞の頭は文字が無くなるので外れ、絵の切替・
    /// 非表示は次の同期が登録を外す（`unregister_box`）ので外れる。
    fn box_still_shown(&self, key: &PlaceKey) -> bool {
        self.box_sites.contains_key(key)
            && !self.hidden_boxes.contains(&key.actor)
            && self
                .state
                .place_state(key)
                .is_some_and(|s| !s.items().is_empty())
    }
}

/// 箱の場所の「あるべき置き場所」（登録と比べる値・はみ出しの判定の材料）。
struct DesiredBox {
    /// 装着先（シェルの窓・差し込み口・拡大率。画像の大きさは箱の大きさ）。
    binding: TextSlotBinding,
    /// シェルの窓がいま表示している絵での置き場所。
    site: BoxPlacement,
    /// シェルの窓がいま表示している絵のサーフェス番号（はみ出しの警告の鍵）。
    surface: u32,
    /// シェルの窓のサーフェスの画像の大きさ（native 原寸）。
    surface_size: (u32, u32),
}

impl TextLayerRuntime {
    /// 毎フレームの箱の同期（UI スレッドから・結線のテキストの拡大率の相が呼ぶ）。
    ///
    /// `shell_views` はスコープごとの「シェルの窓がいま表示している絵」＝差し込み口の
    /// [`TextSlotView`] と絵のサーフェス番号の組（絵が非表示・窓が未確立なら `None`・渡されない
    /// スコープも `None` と同じ）。文字を持つ箱の場所すべてについて、その絵の番号だけから
    /// 「あるべき置き場所」を導き（台本の `\s` で受け取った番号は読まない）、登録済みと違えば
    /// 面を片付けて登録し直し（文字の進み具合は保つ）、同じなら何もしない。
    pub fn sync_boxes(
        &mut self,
        world: &mut World,
        shell_views: &[(ActorKey, Option<(TextSlotView, u32)>)],
    ) {
        let shells: Vec<(ActorKey, Option<(TextSlotBinding, u32)>)> = shell_views
            .iter()
            .map(|(actor, shown)| {
                (
                    actor.clone(),
                    shown
                        .as_ref()
                        .map(|(view, surface)| (TextSlotBinding::from_view(view), *surface)),
                )
            })
            .collect();
        self.sync_box_bindings(world, &shells);
    }

    /// [`sync_boxes`](Self::sync_boxes) の内側（view を binding に読み替えた後）。`TextSlotView` は
    /// emo-present の私有の欄の型でクレートの中の檻から作れないので、判断をこの層で檻に入れる。
    pub(super) fn sync_box_bindings(
        &mut self,
        world: &mut World,
        shells: &[(ActorKey, Option<(TextSlotBinding, u32)>)],
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

    /// あるべき置き場所: シェルの窓が絵を表示していて（絵の番号が渡され）、名前がその絵の箱の列に
    /// あり、箱を隠す印が立っていないときだけ在る。
    fn desired_box(
        &self,
        key: &PlaceKey,
        shells: &[(ActorKey, Option<(TextSlotBinding, u32)>)],
    ) -> Option<DesiredBox> {
        let TextPlace::Box(name) = &key.place else {
            return None;
        };
        if self.hidden_boxes.contains(&key.actor) {
            return None;
        }
        let (shell, surface) = shells
            .iter()
            .find(|(actor, _)| *actor == key.actor)
            .and_then(|(_, shown)| *shown)?;
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
    /// （サーフェス番号, 名前）ごとに 1 度だけ警告する。定義の 2 つの警告（折り返しの基準・無視した
    /// 書き出し位置）は名前の欄を `balloon.*`ブレスの名前にして、箱の名前ごとに 1 度だけ出す
    /// （登録し直しには前の配置の入力が無いので、毎回出さないよう名前で覚える・要件 3.11）。
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
        if self.box_definition_warned.insert(want.site.name.clone()) {
            let name = want.site.name.as_str();
            attach::warn_coarse_wrap_threshold(name, &resolved, None);
            decoration::warn_ignored_origin(name, &resolved, None);
        }
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
        if let (Some(boxes), TextPlace::Box(name)) =
            (self.shown_boxes.get_mut(&key.actor), &key.place)
        {
            boxes.retain(|b| b.name != *name);
        }
        if let Some(render) = self.surfaces.remove(key) {
            render.surface.despawn_window_child(world);
        }
        debug!(actor = %key.actor, place = ?key.place, "箱の置き場所の登録を外した（文字は保つ）");
    }

    /// 箱の面を窓の `Children`（`children`）のどこへ挿すか: 差し込み口の直後から、element番号の
    /// 大きい順（絵より前＝手前・要件 3.7・3.9）。同じスコープで面を持つ、より大きい element番号の
    /// 箱の数だけ後ろへずらす。差し込み口が子に無い・登録されていない箱は `None`。
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
