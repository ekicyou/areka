//! `EmoPresenter`（presenter.rs）: 提示段の統括ハブ（合成・キャッシュ・表示・マスクの一点結線）。
//!
//! 上流が組んだ部品（[`ComposeCache`]・[`SwapChainPresenter`]・[`VisualMount`]・emo-compose の
//! [`Composer`]）を target ごとに束ね、指令 [`PresentCommand`] を UI スレッド上で適用する。合成そのもの
//! は emo-compose、供給面バイト転送は `SwapChainPresenter`、当たり判定マスクは wintf hit-test が担い、
//! 本型は「指令を受けて、キャッシュ引き当て or 合成 → 供給面アップロード → AlphaMask 同期 → 可視制御」を
//! **一続きの UI スレッド呼び出し**として結線する（design §EmoPresenter・§System Flows 指令適用）。
//!
//! # UI スレッドアフィニティ（型で強制・R7.1）
//!
//! `EmoPresenter` は COM/GPU 資源（`SwapChainPresenter` 内の DXGI/D3D11・`VisualMount` の WUC visual）を
//! 内包するため **`!Send`**（NonSend）である。`unsafe impl Send` は置かず、`PhantomData<*const ()>` を
//! 併せ持つことで「他スレッドへ移動できない」ことを**構造（型）で**担保する。wintf World へ NonSend
//! 資源として登録するか example が直接所有し、`apply`/`attach_target`/`read_back` は必ず UI スレッド
//! （NonSend 到達可能スレッド）から呼ばれる（design §Responsibilities & Constraints）。
//!
//! # 原子入替（R2.4）
//!
//! 表示バッファ（`chain.upload`）と当たり判定マスク（`AlphaMaskResource::set`）の更新は**同一 `apply`
//! 呼び出し内**で連続して起き、hit-test も同一 UI スレッドで走るため中間状態は観測不能である。ゆえに
//! surface 切替に伴う「表示とマスクの対入替」は構造的に原子化される（別途ロック不要）。
//!
//! # 失敗経路のログ規律（silent failure 禁止）
//!
//! 全失敗分岐は返す前に `tracing::error!`/`warn!` を出す。`ComposeError::SurfaceNotFound`（解決不能 id）は
//! **error! ＋ 表示不変 ＋ reply `Err`**（R3.4）、`ComposeError::EmptyComposition`（全透明退化）は
//! **warn! ＋ Hide 縮退 ＋ reply `Ok`**（設計ディスカッション #1: 許容される正常退化・skip 解釈は採らない）、
//! デバイス層失敗は `PresentError::Device`（HRESULT ＋文脈）で `Err`。panic は用いない。

use std::collections::HashMap;
use std::marker::PhantomData;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

use areka_actor::ReplySender;
use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, ComposeError, Composer, EmoWorld, PatternState, RegionPriority};

use wintf::ecs::{AlphaMaskResource, GraphicsCore, WucGraphicsResource};

use crate::cache::ComposeCache;
use crate::chain::SwapChainPresenter;
use crate::command::{PresentCommand, PresentError, PresentOutcome, TargetId};
use crate::mount::VisualMount;

/// target ごとの表示コンテキスト（シェル・バルーンで同一機構・R5.1 の統一原則）。
///
/// `chain`／`mount` は **初回 `ShowSurface` で原寸が確定してから遅延生成**する（0×0 の供給面は作れない・
/// 全透明退化では生成しない）。`emo_world`／`atlas`（構築時 `bind_atlas` 済み）と `composer`／`cache` は
/// 合成・引き当ての入力側、`window` は装着先の窓ハンドル（R1.3）である。
struct PresentTarget {
    /// 合成入力（構築時 `bind_atlas` 済み・不変）。
    emo_world: EmoWorld,
    /// アトラス正本（不変・`InvalidateCache` の再合成源）。
    atlas: AtlasTable,
    /// 合成器（状態非保持・スクラッチのみ再利用）。
    composer: Composer,
    /// 合成入力（surface id＋bind 集合）→ (composed, mask) 対の容量 1 メモ化スロット。
    cache: ComposeCache,
    /// 装着先の窓 Entity（R1.3・遅延装着の対象）。
    window: Entity,
    /// 窓装着ハンドル（初回表示で生成）。
    mount: Option<VisualMount>,
    /// 自前供給面（初回表示で原寸確定後に生成）。
    chain: Option<SwapChainPresenter>,
    /// 現在可視か（`Hide`／全透明退化で false・`ShowSurface` 成功で true）。
    visible: bool,
    /// 現在表示中のサーフェス id ＝「**最後に表示が成立した id**」（CurrentSurfaceRead・R3.1-3.3）。
    ///
    /// 画面の絵ではなく表示成立の結果を刻む（全透明合成でも表示成立＝その id が正・α 非依存で collision
    /// 解決の単一真実源）。`Hide`／`EmptyComposition` 縮退で `None`。書き込みは既存 `visible` 更新点と
    /// 同一の3箇所のみ（表示成立＝`Some(surface_id)`／縮退・Hide＝`None`）で、失敗経路は表示成立点より
    /// 手前で early return するため前値を保持する（`ComposeKey` からは導出しない＝`invalidate_all` で
    /// キーが消えても表示は残るため画面と乖離する）。
    current_surface_id: Option<u32>,
}

/// 予約 text 層スロットへの読み取り専用の到達手段（emo-text-layer が消費する additive 公開増分・R9.1/9.2）。
///
/// [`EmoPresenter::text_slot_view`] が返すスナップショット値。フィールドは非公開（`#[non_exhaustive]`
/// 相当）で accessor のみを公開し、スロット状態の変更手段を一切持たない（読み取り専用 view）。
/// 装着 API 形（emo-present が描画物を受け取る）は emo-text の描画型が本 crate へ逆流し依存方向
/// （emo-present → emo-text 禁止）と衝突するため採らない（design §TextSlotView）。
///
/// mount は初回 `ShowSurface` で遅延生成されるため、それ以前は取得できない（`text_slot_view` が
/// `None`）。呼び手は表示確立後に取得するか再取得を試みる（runtime 前提条件）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSlotView {
    /// 予約済み text 層スロット entity（`Name("emo-text-layer-slot")`・内容なしの seam）。
    slot: Entity,
    /// スロットが属する装着先の窓 Entity。
    window: Entity,
    /// バルーン/シェル surface の物理 px 原寸（取得時点のスナップショット）。
    surface_size: (u32, u32),
    /// バルーン surface と同一の合成スケール k（現行の物理 1:1 表示契約では恒常 1.0）。
    scale: f32,
}

impl TextSlotView {
    /// 予約済み text 層スロット entity（emo-text-layer が描画を装着する先）。
    pub fn slot(&self) -> Entity {
        self.slot
    }

    /// スロットが属する装着先の窓 Entity。
    pub fn window(&self) -> Entity {
        self.window
    }

    /// バルーン surface の物理 px 原寸。
    pub fn surface_size(&self) -> (u32, u32) {
        self.surface_size
    }

    /// バルーン surface と同一の合成スケール k（現行 1.0 恒常・DPI 契約の共有点）。
    ///
    /// 将来 emo-present が DPI スケーリング（k=モニタ DPI ÷ author_dpi）を導入したら、供給値の
    /// 変更点はここ 1 点である（design §TextSlotView Revalidation Trigger）。
    pub fn scale(&self) -> f32 {
        self.scale
    }
}

/// 現行の物理 1:1 表示契約における合成スケール k の恒常値（design §DPI/スケール契約）。
const CURRENT_COMPOSE_SCALE: f32 = 1.0;

/// 指令適用の統括ハブ（合成・キャッシュ・表示・マスクの一点結線・UI スレッド専有）。
///
/// target を [`Self::attach_target`] で登録し、[`Self::apply`] で [`PresentCommand`] を適用する。
/// COM/GPU 資源を内包するため `!Send`（`PhantomData<*const ()>` で型強制・R7.1）。`unsafe impl Send`
/// は置かない。
pub struct EmoPresenter {
    /// target 識別子 → 表示コンテキスト。
    targets: HashMap<TargetId, PresentTarget>,
    /// `!Send`/`!Sync` を型で担保するマーカー（UI スレッドアフィニティの構造的強制・R7.1）。
    _not_send: PhantomData<*const ()>,
}

impl EmoPresenter {
    /// 空の統括ハブを構築する（target は未登録）。
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            _not_send: PhantomData,
        }
    }

    /// target を登録し、窓 Entity を装着先として記録する（窓生成は呼び手＝placement/example の責務）。
    ///
    /// 供給面（`SwapChainPresenter`）と装着（`VisualMount`）は**初回 `ShowSurface` で原寸が確定してから
    /// 遅延生成**するため、本メソッドは skeleton（`chain=None`/`mount=None`/`visible=false`）を組んで登録
    /// するのみで World には触れない。既存 id への再登録は表示コンテキストごと置換する。
    ///
    /// `world` は将来の system 化（`&mut World` を要する装着タイミング）へ向けた API 一貫性のために受ける
    /// が、遅延生成方針ゆえ本メソッドでは参照しない。
    pub fn attach_target(
        &mut self,
        _world: &mut World,
        target: TargetId,
        window: Entity,
        emo_world: EmoWorld,
        atlas: AtlasTable,
    ) -> Result<(), PresentError> {
        self.targets.insert(
            target,
            PresentTarget {
                emo_world,
                atlas,
                composer: Composer::new(),
                cache: ComposeCache::new(),
                window,
                mount: None,
                chain: None,
                visible: false,
                current_surface_id: None,
            },
        );
        Ok(())
    }

    /// 指令を適用する（UI スレッド上で呼ぶ）。reply 同梱時は完了/失敗を高々 1 回返信する。
    ///
    /// 戻り値は持たず、結果は各 variant の `reply`（`Some` のとき）へ送る。失敗経路も含め、全分岐が
    /// ログを出したうえで reply する（silent failure 禁止）。
    pub fn apply(&mut self, world: &mut World, cmd: PresentCommand) {
        match cmd {
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds,
                reply,
            } => self.apply_show(world, target, surface_id, binds, reply),
            PresentCommand::Hide { target, reply } => self.apply_hide(world, target, reply),
            PresentCommand::InvalidateCache { target, reply } => {
                self.apply_invalidate(target, reply)
            }
        }
    }

    /// `ShowSurface` の適用（キャッシュ引き当て or 合成 → 供給面アップロード → マスク同期 → 可視化）。
    ///
    /// 手順（design §System Flows）: (1) 未装着なら error! ＋ `Err(TargetNotAttached)`。(2) 合成入力
    /// （surface id＋bind 集合）が直前と完全一致するヒットなら再合成しない（R4.2）——bind 集合が
    /// 1 要素でも異なれば必ずミス＝再合成する（着せ替え・まばたきの正しさの担保）。(3) ミスなら合成し、
    /// `SurfaceNotFound` は error! ＋表示不変＋
    /// `Err`（R3.4）、`EmptyComposition` は warn! ＋ Hide 縮退＋`Ok`（設計ディスカッション #1）、`Ok` なら
    /// `cache.insert`（マスクを 1 回だけ生成）。(4) 使えるエントリで、`chain`/`mount` 未生成なら原寸確定
    /// 後に遅延生成し、`chain.upload` ＋ `AlphaMaskResource::set` ＋ 可視化を同一呼び出し内で行う（R2.4）。
    fn apply_show(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        surface_id: u32,
        binds: BindSet,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(?target_id, surface_id, "apply(ShowSurface): 未装着ターゲット");
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        // (1) 引き当て: 合成入力（id＋binds）の完全一致のみヒット＝再合成しない（R4.2）。ミスのみ合成する。
        // task 8.1: pattern はキー要素だが、ShowSurface.pattern からの実スレッドは task 8.2 が置換する。
        // ここでは空 PatternState を既定で用いる（拡張前と観測等価・R5.4）。
        let cache_hit = target
            .cache
            .get(surface_id, &binds, &PatternState::default())
            .is_some();
        if !cache_hit {
            match target
                .composer
                // task 7.1 keep-compiling: pattern を既定（空）で埋める。ShowSurface.pattern からの
                // 実スレッドは task 8.2 が置換する（空 PatternState は拡張前と観測等価・R5.4）。
                .compose(&target.emo_world, &target.atlas, surface_id, &binds, &PatternState::default())
            {
                Ok(composed) => {
                    // 挿入時にマスクを 1 回だけ生成し、表示バッファと対で束ねる（R2.1/R2.4）。
                    // task 8.1: pattern はキー要素・実スレッドは 8.2（ここは既定の空 PatternState）。
                    target
                        .cache
                        .insert(surface_id, binds.clone(), PatternState::default(), composed);
                }
                Err(ComposeError::EmptyComposition(id)) => {
                    // 全透明退化（外形 0×0）: 許容される正常退化として Hide 縮退＋reply Ok（skip ではない）。
                    tracing::warn!(
                        ?target_id,
                        surface_id = id,
                        was_visible = target.visible,
                        "apply(ShowSurface): 全透明退化（EmptyComposition）→ Hide 縮退（reply Ok）"
                    );
                    if let Some(mount) = target.mount.as_ref() {
                        mount.set_visible(world, false);
                    }
                    target.visible = false;
                    // EmptyComposition 縮退は Hide と同じ表示結果ゆえ現サーフェス無し（R3.2・Key decisions (b)）。
                    target.current_surface_id = None;
                    Self::reply(reply, Ok(()));
                    return;
                }
                Err(e) => {
                    // 解決不能 id（SurfaceNotFound 等）: error! ＋ 表示不変 ＋ reply Err（R3.4）。
                    tracing::error!(
                        ?target_id,
                        surface_id,
                        error = %e,
                        "apply(ShowSurface): 合成失敗 → 表示は適用前のまま（reply Err）"
                    );
                    Self::reply(reply, Err(PresentError::Compose(e)));
                    return;
                }
            }
        }

        // (2) 供給面・装着の遅延生成（初回表示・原寸確定後）。
        if target.chain.is_none() {
            let (w, h) = {
                let entry = target
                    .cache
                    .get(surface_id, &binds, &PatternState::default())
                    .expect("直前に引き当て済み");
                (entry.composed.width(), entry.composed.height())
            };

            // Compositor は所有クローンで取り出し、以後の &mut World 装着と借用衝突しないようにする。
            let Some(compositor) = world
                .get_resource::<WucGraphicsResource>()
                .and_then(|r| r.compositor().cloned())
            else {
                tracing::error!(
                    ?target_id,
                    "apply(ShowSurface): WucGraphicsResource/Compositor 不在（供給面を生成できない）"
                );
                Self::reply(
                    reply,
                    Err(PresentError::Device {
                        hresult: 0,
                        context: "WucGraphicsResource::compositor",
                    }),
                );
                return;
            };

            // GraphicsCore は生成呼び出しの間だけ借用する（surface は所有で返るため借用は閉じる）。
            let new_chain = {
                let Some(gfx) = world.get_resource::<GraphicsCore>() else {
                    tracing::error!(
                        ?target_id,
                        "apply(ShowSurface): GraphicsCore 不在（供給面を生成できない）"
                    );
                    Self::reply(
                        reply,
                        Err(PresentError::Device {
                            hresult: 0,
                            context: "GraphicsCore resource",
                        }),
                    );
                    return;
                };
                SwapChainPresenter::new(gfx, &compositor, w, h)
            };
            let (chain, surface) = match new_chain {
                Ok(pair) => pair,
                // SwapChainPresenter::new は内部で error! 済み（chain.rs device_err）。ここは reply のみ。
                Err(e) => {
                    Self::reply(reply, Err(e));
                    return;
                }
            };

            let window = target.window;
            let mount = match VisualMount::attach(world, window, &surface, &compositor, (w, h)) {
                Ok(m) => m,
                // VisualMount::attach も内部で error! 済み（mount.rs device_err）。
                Err(e) => {
                    Self::reply(reply, Err(e));
                    return;
                }
            };

            target.chain = Some(chain);
            target.mount = Some(mount);
        }

        // (3) 供給面アップロード ＋ マスク同期 ＋ 可視化（同一呼び出し内＝原子入替・R2.4）。
        let entry = target
            .cache
            .get(surface_id, &binds, &PatternState::default())
            .expect("直前に引き当て済み");
        let size = (entry.composed.width(), entry.composed.height());

        let chain = target.chain.as_mut().expect("直上で生成済み");
        if let Err(e) = chain.upload(&entry.composed) {
            // upload は内部で error! 済み（chain.rs）。表示は前状態を保つ（成功まで旧状態不変）。
            Self::reply(reply, Err(e));
            return;
        }

        let mount = target.mount.as_ref().expect("直上で生成済み");
        if let Some(mut mask_res) = world.get_mut::<AlphaMaskResource>(mount.surface_entity()) {
            // 表示バッファと同一 bytes 由来のマスクを hit-test へ供給する（R2.2/R2.5）。
            mask_res.set(entry.mask.clone());
        } else {
            tracing::warn!(
                ?target_id,
                entity = ?mount.surface_entity(),
                "apply(ShowSurface): surface entity に AlphaMaskResource が無い（当たり判定は矩形/前状態）"
            );
        }
        mount.set_visible(world, true);
        mount.set_bounds(world, size);
        target.visible = true;
        // 表示成立＝この id が現サーフェス（全透明でも成立・α 非依存の単一真実源・R3.1/3.3・Key decisions）。
        target.current_surface_id = Some(surface_id);

        tracing::info!(
            ?target_id,
            surface_id,
            cache_hit,
            width = size.0,
            height = size.1,
            "apply(ShowSurface): 表示・マスクを更新"
        );
        Self::reply(reply, Ok(()));
    }

    /// `Hide`（`\s[-1]` 相当）の適用: visual 非表示＋当たり判定停止。swap chain・キャッシュは保持する（R3.3）。
    fn apply_hide(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(?target_id, "apply(Hide): 未装着ターゲット");
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        if let Some(mount) = target.mount.as_ref() {
            mount.set_visible(world, false);
        }
        tracing::debug!(?target_id, was_visible = target.visible, "apply(Hide): 非表示へ");
        target.visible = false;
        // Hide（`\s[-1]` 相当）は表示していない＝現サーフェス無し（R3.2/4.4・Key decisions (a)）。
        target.current_surface_id = None;
        Self::reply(reply, Ok(()));
    }

    /// `InvalidateCache` の適用: 合成キャッシュ全破棄（R4.3）。表示中バッファ/マスクは反映済みゆえ表示は継続。
    fn apply_invalidate(&mut self, target_id: TargetId, reply: Option<ReplySender<PresentOutcome>>) {
        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(?target_id, "apply(InvalidateCache): 未装着ターゲット");
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        target.cache.invalidate_all();
        tracing::debug!(?target_id, "apply(InvalidateCache): キャッシュ全破棄（表示は継続）");
        Self::reply(reply, Ok(()));
    }

    /// target の予約 text 層スロットへの読み取り専用の到達手段（mount 未生成なら `None`・R9.1/9.2）。
    ///
    /// mount（と供給面）は初回 `ShowSurface` で原寸確定後に遅延生成されるため、未登録 target・
    /// 初回表示確立前は取得不可（`None`）である。呼び手（結線側）は表示確立後に取得するか再取得を
    /// 試みる。返る値はスナップショット（読み取り専用 view）で、スロット状態は変更できない。
    pub fn text_slot_view(&self, target: TargetId) -> Option<TextSlotView> {
        let t = self.targets.get(&target)?;
        let mount = t.mount.as_ref()?;
        let chain = t.chain.as_ref()?;
        Some(TextSlotView {
            slot: mount.text_slot(),
            window: t.window,
            surface_size: chain.size(),
            scale: CURRENT_COMPOSE_SCALE,
        })
    }

    /// target がいま表示しているサーフェス id（CurrentSurfaceRead・R3.1-3.3）。
    ///
    /// 「最後に表示が成立したサーフェス id」を返す（画面の絵ではなく表示成立の結果・α 非依存）。
    /// 未表示（一度も `ShowSurface` していない）・`Hide` 済み・空合成へ縮退した場合、および未登録
    /// target は `None`。単一真実源は `PresentTarget.current_surface_id`（`ComposeKey` から導出しない・
    /// design §CurrentSurfaceRead State Management）。既存の表示ロジックへ分岐を足さない additive な
    /// 読み取りのみ（R3.4）。
    pub fn current_surface_id(&self, target: TargetId) -> Option<u32> {
        self.targets.get(&target)?.current_surface_id
    }

    /// 現サーフェスの当たり判定領域名を解決する（`current_surface_id` → `EmoWorld::surface` → 純関数・R4.1/4.4）。
    ///
    /// 座標はサーフェス px（＝窓 client 物理 px・k=1.0 契約）。現サーフェス無し（未表示／`Hide`／空合成
    /// 縮退／未登録 target）は `None`（R4.4）。重なりは画家のアルゴリズム（後定義が手前・[`RegionPriority::Painter`]）で
    /// 解決する。`EmoWorld` を presenter 外へ露出しない（`&SurfaceMaster` を外へ出さない）ため純関数
    /// [`areka_emo_compose::hit_region`] の呼出は本メソッド内で閉じ、戻り値の寿命は `&self` に従う
    /// （マウス移動ごとの割当を生まない・design §CurrentSurfaceRead Service Interface）。
    pub fn hit_region(&self, target: TargetId, x: i64, y: i64) -> Option<&str> {
        let t = self.targets.get(&target)?;
        let master = t.emo_world.surface(t.current_surface_id?)?;
        areka_emo_compose::hit_region(master, x, y, RegionPriority::Painter)
    }

    /// target の表示中画素を CPU へ読み戻す（R6.2/R8.3・検証・将来の直読みヒットテスト基盤）。
    ///
    /// 未装着、または供給面未生成（未表示）なら [`PresentError::TargetNotAttached`] を返す。
    pub fn read_back(&self, target: TargetId) -> Result<Vec<u8>, PresentError> {
        let Some(t) = self.targets.get(&target) else {
            tracing::error!(?target, "read_back: 未装着ターゲット");
            return Err(PresentError::TargetNotAttached(target));
        };
        match t.chain.as_ref() {
            Some(chain) => chain.read_back(),
            None => {
                tracing::error!(?target, "read_back: 供給面が未生成（未だ表示していない）");
                Err(PresentError::TargetNotAttached(target))
            }
        }
    }

    /// reply 同梱時に結果を高々 1 回送る。受信端が既に drop 済みなら撃ちっぱなし扱い（debug ログ）。
    fn reply(reply: Option<ReplySender<PresentOutcome>>, outcome: PresentOutcome) {
        if let Some(tx) = reply {
            if tx.send(outcome).is_err() {
                tracing::debug!("reply: 受信端が既に drop 済み（撃ちっぱなし扱い・無視）");
            }
        }
    }
}

impl Default for EmoPresenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;
    use std::time::Duration;

    use areka_actor::reply_channel;
    use areka_emo_atlas::{
        AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
    };
    use areka_emo_compose::BindSet;
    use areka_parsers::shell::{
        Animation, AppendTarget, DefRef, DrawMethod, Element, ElementPath, Interval, Pattern, Shell,
        Surface,
    };

    use wintf::ecs::{GraphicsCore, HitTest, HitTestMode, Visual, WucGraphicsResource};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    // ── GPU/WUC フィクスチャ（chain.rs / mount.rs / wuc_resource.rs テストと同一方針）──────────
    // 本番 UI スレッドは MTA（メモリ「areka WUC は MTA スレッドで動く」）。WucGraphicsResource::new は
    // DQTAT_COM_NONE（apartment 不変）でディスパッチャを組むため、COM を MTA 初期化してから呼ぶ。

    /// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World を組む。
    ///
    /// `EmoPresenter` は供給面生成時に World から両資源を読む（compositor は `WucGraphicsResource` 由来）。
    /// ゆえに本番同様、World へ両者を挿入した状態を作る。
    fn make_world_with_gpu() -> World {
        // 各テストは専用スレッドで走る。MTA を初期化（S_FALSE/RPC_E_CHANGED_MODE は無視）。
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
        let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
        let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");

        let mut world = World::new();
        world.insert_resource(core);
        world.insert_resource(wuc);
        world
    }

    // ── ComposedSurface 生成補助（chain.rs テストと同技法）──────────────────────────────────
    // `ComposedSurface::bytes_mut` は emo-compose の pub(crate) ゆえ本クレートから画素を直接焼けない。
    // 上流公開 API（atlas bake → EmoWorld → Composer::compose）で本物を合成して得る。

    fn elem(path: &str, x: i64, y: i64) -> Element {
        Element {
            layer: 0,
            path: ElementPath::new(path.to_string()),
            x,
            y,
        }
    }

    fn surface(id: u32, elements: Vec<Element>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations: Vec::new(),
        }
    }

    fn shell_of(surfaces: Vec<Surface>) -> Shell {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions,
        }
    }

    /// surface 1000 = 単一 element（`w×h` 全不透明・座標由来グラデーション）の `(EmoWorld, AtlasTable)`
    /// と、同一入力を `Composer::compose` で直接合成した golden バイト列を返す。
    ///
    /// α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は正確に `w×h`。golden は presenter が
    /// 内部で辿るのと同一の world/atlas から作るため、readback とのバイト一致が二重に決定論的になる。
    fn build_target_assets(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable, Vec<u8>) {
        let base = Path::new("shell/master");
        let surfaces = vec![surface(1000, vec![elem("p.png", 0, 0)])];

        let mut dec = MemoryDecoder::new();
        let stride = w * 4;
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let a: u8 = 0xFF;
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.push(b);
                img.push(g);
                img.push(r);
                img.push(a);
            }
        }
        dec.insert(base.join("p.png"), w, h, stride, img, true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        let atlas = baked.table;

        // golden: presenter と同一入力を直接合成（move 前に計算する）。
        let mut composer = Composer::new();
        let golden = composer
            .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
            .expect("静的 element 単体の合成は Ok");
        let golden_bytes = golden.bytes().to_vec();

        (world, atlas, golden_bytes)
    }

    /// R2.4/R3.2/R8.2 観測完了（golden 一致）: `attach_target` → `apply(ShowSurface 有効 id)` で reply が
    /// `Ok(())`、かつ `read_back` が同一入力の直接合成 golden と**全バイト一致**する。
    ///
    /// 供給面は D2D 非経由の純バイト転送ゆえ、readback と `ComposedSurface.bytes()` のバイト一致が
    /// 決定論的に成立する（WARP でも可＝CI 決定論）。
    #[test]
    fn golden_match_read_back_equals_direct_compose() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();

        let (emo_world, atlas, golden) = build_target_assets(3, 2, 0x11);
        assert!(golden.iter().any(|&b| b != 0), "golden は非退化（全 0 でない）");

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx),
            },
        );

        let outcome = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（ShowSurface）を受信できない");
        assert!(
            matches!(outcome, Ok(())),
            "有効 id の ShowSurface は Ok を返す: {outcome:?}"
        );

        let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
        assert_eq!(
            rb, golden,
            "readback が直接合成 golden とバイト一致しない（表示・供給面の恒等転送が壊れている）"
        );
    }

    /// R3.4 観測完了（表示不変）: 有効 id で表示を確立後、**解決不能 id** の `ShowSurface` は reply が
    /// `Err(Compose(SurfaceNotFound))` で、`read_back` バイトは**適用前と不変**（表示を乱さない）。
    #[test]
    fn invalid_surface_id_replies_err_and_leaves_display_unchanged() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();

        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x5A);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        // まず有効 id で表示を確立（供給面生成＋表示バイト確定）。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );
        let before = presenter.read_back(TargetId(0)).expect("read_back（前）失敗");

        // 解決不能 id: error! ＋ 表示不変 ＋ reply Err（R3.4）。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 9999,
                binds: BindSet::default(),
                reply: Some(tx1),
            },
        );
        let outcome = rx1
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（無効 id）を受信できない");
        assert!(
            matches!(
                outcome,
                Err(PresentError::Compose(ComposeError::SurfaceNotFound(9999)))
            ),
            "無効 id は Err(Compose(SurfaceNotFound(9999))) を返す: {outcome:?}"
        );

        let after = presenter.read_back(TargetId(0)).expect("read_back（後）失敗");
        assert_eq!(
            before, after,
            "無効 id の適用で表示中バイトが変化した（表示不変の不変条件を破っている）"
        );
    }

    /// 有効 surface 1000（`w×h` 全不透明 element）＋ 定義層皆無で外形 0×0 に退化する surface 7000
    /// （element/animation ゼロ）を**同一 target**へ載せる `(EmoWorld, AtlasTable)` を返す。
    ///
    /// surface 7000 は composer_tests.rs の `no_layers_degenerate_propagates_empty_composition`
    /// と同型の構成（bind 済み world に存在するが合成外形 0×0）で、`Composer::compose` が
    /// `Err(ComposeError::EmptyComposition(7000))` を返す。単なる全透明 element は非ゼロ外形の
    /// `Ok`（`all_transparent_surface_is_ok_transparent_nonzero_extent`）で EmptyComposition では
    /// ないため、退化は「定義層皆無 → 0×0」の経路で作る。
    fn build_assets_with_valid_and_empty(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable) {
        let base = Path::new("shell/master");
        // surface 1000: 全不透明 element 1 本。surface 7000: element/animation ゼロ（定義層皆無）。
        let surfaces = vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(7000, Vec::new()),
        ];

        let mut dec = MemoryDecoder::new();
        let stride = w * 4;
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let a: u8 = 0xFF;
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.push(b);
                img.push(g);
                img.push(r);
                img.push(a);
            }
        }
        dec.insert(base.join("p.png"), w, h, stride, img, true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        (world, baked.table)
    }

    /// R3.4 観測完了（skip＋表示不変の回帰檻）: 有効 id で表示・マスクを確立後、**解決不能 id** の
    /// `ShowSurface` は reply が `Err(Compose(SurfaceNotFound))`、かつ (a) `read_back` バイト、
    /// (b) surface entity の `HitTest`（`AlphaMask`）、(c) `AlphaMaskResource`（設定済みマスク）の
    /// いずれも**適用前と不変**（表示＋マスクを一切乱さない）。
    ///
    /// 4.1 の `invalid_surface_id_replies_err_and_leaves_display_unchanged` はバイト不変のみを見るが、
    /// 本テストは「skip＝表示器（visual/mask/hit-test）を触らない」を独立・自己完結に固定する。
    #[test]
    fn invalid_surface_skips_and_leaves_display_and_mask_unchanged() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();

        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x37);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        // 有効 id で表示・マスク・hit-test を確立。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );

        // 表示器の適用前状態を捕捉（bytes ＋ HitTest ＋ mask 寸法/有無）。
        let surface_entity = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("有効表示後は mount が生成済み")
            .surface_entity();

        let bytes_before = presenter.read_back(TargetId(0)).expect("read_back（前）失敗");
        let hit_before = world
            .get::<HitTest>(surface_entity)
            .expect("surface entity に HitTest が無い")
            .mode;
        assert_eq!(hit_before, HitTestMode::AlphaMask, "有効表示後は αマスク判定");
        let mask_dims_before = world
            .get::<AlphaMaskResource>(surface_entity)
            .and_then(|r| r.mask().map(|m| (m.width(), m.height())));
        assert!(mask_dims_before.is_some(), "有効表示後は AlphaMask が供給済み");
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "有効表示後は可視"
        );

        // 解決不能 id: error! ＋ skip（表示器不触）＋ reply Err（R3.4）。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 4242,
                binds: BindSet::default(),
                reply: Some(tx1),
            },
        );
        let outcome = rx1
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（無効 id）を受信できない");
        assert!(
            matches!(
                outcome,
                Err(PresentError::Compose(ComposeError::SurfaceNotFound(4242)))
            ),
            "無効 id は Err(Compose(SurfaceNotFound(4242))) を返す: {outcome:?}"
        );

        // (a) 表示バイト不変。
        let bytes_after = presenter.read_back(TargetId(0)).expect("read_back（後）失敗");
        assert_eq!(
            bytes_before, bytes_after,
            "無効 id の skip で表示中バイトが変化した（表示を乱さない不変条件違反）"
        );
        // (b) HitTest 不変（None へ落ちていない＝当たり判定が生きたまま）。
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::AlphaMask,
            "無効 id の skip で HitTest が変化した（マスク/当たり判定を乱している）"
        );
        // (c) AlphaMaskResource 不変（供給済みマスクが消えていない）。
        let mask_dims_after = world
            .get::<AlphaMaskResource>(surface_entity)
            .and_then(|r| r.mask().map(|m| (m.width(), m.height())));
        assert_eq!(
            mask_dims_before, mask_dims_after,
            "無効 id の skip で AlphaMaskResource が変化した（マスクを乱している）"
        );
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "無効 id の skip で可視状態が変化した（表示を乱している）"
        );
    }

    /// 設計ディスカッション #1 観測完了（EmptyComposition → Hide 縮退＋reply Ok）: 有効表示で mount を
    /// 確立後、**外形 0×0 に退化する既存 surface**（定義層皆無）を `ShowSurface` すると reply は
    /// **`Ok(())`**（`Err` ではない）で、target は Hidden へ縮退（`Visual` 不可視＋`HitTest::none()`）し、
    /// 0×0 供給面を作ろうとして panic しない（既存 chain は破棄されず保持）。
    ///
    /// 前段で `Composer::compose(7000)` が `EmptyComposition(7000)` を返すことを直接確認し、退化経路が
    /// 「不在 surface（SurfaceNotFound）」ではなく「存在するが 0×0」であることを固定する。
    #[test]
    fn empty_composition_degrades_to_hidden_and_replies_ok() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();

        let (emo_world, atlas) = build_assets_with_valid_and_empty(5, 4, 0x22);

        // 前提固定: 7000 は「存在するが外形 0×0」＝EmptyComposition（SurfaceNotFound ではない）。
        {
            let mut composer = Composer::new();
            let direct = composer.compose(&emo_world, &atlas, 7000, &BindSet::default(), &PatternState::default());
            assert_eq!(
                direct.err(),
                Some(ComposeError::EmptyComposition(7000)),
                "surface 7000 は定義層皆無で EmptyComposition を返す前提でなければならない"
            );
        }

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        // 有効 1000 で mount/chain を確立し可視化。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );
        let surface_entity = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("有効表示後は mount が生成済み")
            .surface_entity();
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "有効表示後は可視"
        );
        let bytes_len_before = presenter
            .read_back(TargetId(0))
            .expect("read_back（前）失敗")
            .len();

        // EmptyComposition 退化: warn! ＋ Hide 縮退 ＋ reply Ok（skip でも Err でもない）。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 7000,
                binds: BindSet::default(),
                reply: Some(tx1),
            },
        );
        let outcome = rx1
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（EmptyComposition）を受信できない");
        assert!(
            matches!(outcome, Ok(())),
            "EmptyComposition は Hide 縮退＋reply Ok（Err ではない）: {outcome:?}"
        );

        // Hidden へ縮退: Visual 不可視 ＋ HitTest::none()。
        assert!(
            !world.get::<Visual>(surface_entity).unwrap().is_visible,
            "EmptyComposition は Hidden へ縮退（Visual 不可視）でなければならない"
        );
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::None,
            "EmptyComposition は当たり判定停止（HitTest::none）でなければならない"
        );
        assert!(
            !presenter.targets.get(&TargetId(0)).unwrap().visible,
            "EmptyComposition 後は target.visible=false"
        );

        // 0×0 供給面は作らない: 既存 chain は破棄されず保持（read_back は旧外形の長さのまま成立）。
        let bytes_len_after = presenter
            .read_back(TargetId(0))
            .expect("EmptyComposition 後も既存 chain は保持され read_back できる")
            .len();
        assert_eq!(
            bytes_len_before, bytes_len_after,
            "EmptyComposition で 0×0 chain へ差し替わった（既存 chain 保持の不変条件違反）"
        );
        // 7000 は非合成ゆえキャッシュへ載らない（0×0 を挿入しない）。
        assert!(
            presenter
                .targets
                .get(&TargetId(0))
                .unwrap()
                .cache
                .get(7000, &BindSet::default(), &PatternState::default())
                .is_none(),
            "EmptyComposition は cache へ 0×0 を挿入しない"
        );
    }

    /// R3.3 観測完了（Hide → 再 ShowSurface 復帰）: 有効表示 → `Hide`（不可視＋`HitTest::none()`＋
    /// chain/cache 保持）→ 同一有効 id を再 `ShowSurface` で表示復帰（可視＋`HitTest::alpha_mask()`）。
    /// 再表示はキャッシュヒットで再合成せず、`read_back` が初回表示バイトと一致する（キャッシュからの復帰）。
    #[test]
    fn hide_then_reshow_recovers_display_from_cache() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();

        let (emo_world, atlas, _golden) = build_target_assets(6, 5, 0x4D);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        // 初回表示（可視・αマスク判定確立）。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "初回 ShowSurface が Ok でない"
        );
        let surface_entity = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("初回表示後は mount が生成済み")
            .surface_entity();
        let bytes_shown = presenter.read_back(TargetId(0)).expect("read_back（初回）失敗");
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::AlphaMask,
            "初回表示後は αマスク判定"
        );

        // Hide: 不可視 ＋ HitTest::none() ＋ chain/cache 保持。
        let (txh, rxh) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::Hide {
                target: TargetId(0),
                reply: Some(txh),
            },
        );
        assert!(
            matches!(rxh.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "Hide が Ok でない"
        );
        assert!(
            !world.get::<Visual>(surface_entity).unwrap().is_visible,
            "Hide 後は Visual 不可視"
        );
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::None,
            "Hide 後は当たり判定停止（HitTest::none）"
        );
        {
            let target = presenter.targets.get(&TargetId(0)).unwrap();
            assert!(target.chain.is_some(), "Hide は swap chain を保持する（R3.3）");
            assert!(
                target.cache.get(1000, &BindSet::default(), &PatternState::default()).is_some(),
                "Hide は合成キャッシュを保持する（R3.3）"
            );
            assert!(!target.visible, "Hide 後は target.visible=false");
        }

        // 再 ShowSurface（同一有効 id）: キャッシュヒットで再合成せず表示復帰。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx1),
            },
        );
        assert!(
            matches!(rx1.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "再 ShowSurface が Ok でない"
        );

        // 表示復帰: 可視 ＋ αマスク判定へ戻る。
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "再表示後は可視へ復帰"
        );
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::AlphaMask,
            "再表示後は αマスク判定へ復帰"
        );
        assert!(
            presenter.targets.get(&TargetId(0)).unwrap().visible,
            "再表示後は target.visible=true"
        );

        // 観測可能な復帰: read_back が初回表示バイトと一致（キャッシュからの復帰）。
        let bytes_reshown = presenter.read_back(TargetId(0)).expect("read_back（再表示）失敗");
        assert_eq!(
            bytes_shown, bytes_reshown,
            "再表示のバイトが初回表示と一致しない（キャッシュからの表示復帰が壊れている）"
        );
    }

    /// R9.1/9.2 観測完了（mount 未生成＝取得不可）: 未登録 target・登録済みだが初回 `ShowSurface` 前
    /// （mount 遅延生成前）のいずれも `text_slot_view` が `None` を返す（取得結果が空）。
    ///
    /// mount 未生成経路は World に GPU 資源を要しない（`attach_target` は skeleton 登録のみ）ため、
    /// 素の `World` で決定論的に固定する。
    #[test]
    fn text_slot_view_is_none_before_display_established() {
        let mut world = World::new();
        let window = world.spawn_empty().id();
        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x66);

        let mut presenter = EmoPresenter::new();
        // 未登録 target: 取得結果は空。
        assert!(
            presenter.text_slot_view(TargetId(0)).is_none(),
            "未登録 target の text_slot_view は None"
        );

        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");
        // 登録済みでも初回 ShowSurface 前（mount 未生成）は空（design: mount は遅延生成・R9.2）。
        assert!(
            presenter.text_slot_view(TargetId(0)).is_none(),
            "初回 ShowSurface 前（mount 未生成）の text_slot_view は None"
        );
    }

    /// R9.1/9.2 観測完了（表示確立後の正値）: 有効 `ShowSurface` で表示確立後、`text_slot_view` が
    /// `Some` を返し、(a) `slot()` ＝ mount の予約スロット（`Name("emo-text-layer-slot")` を持つ）、
    /// (b) `window()` ＝ 装着先窓 Entity、(c) `surface_size()` ＝ バルーン/シェル surface の物理 px 原寸、
    /// (d) `scale()` ＝ 現行の物理 1:1 表示契約の恒常値 1.0、をすべて満たす。
    #[test]
    fn text_slot_view_returns_slot_window_size_scale_after_display() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();

        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x77);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );

        let view = presenter
            .text_slot_view(TargetId(0))
            .expect("表示確立後の text_slot_view は Some");

        // (a) slot ＝ mount の予約スロット（Name で二重に裏取り）。
        let expected_slot = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("表示確立後は mount が生成済み")
            .text_slot();
        assert_eq!(view.slot(), expected_slot, "slot() が予約スロット entity と一致しない");
        let name = world
            .get::<bevy_ecs::name::Name>(view.slot())
            .expect("予約スロットに Name が無い");
        assert_eq!(name.as_str(), "emo-text-layer-slot");

        // (b) window ＝ attach_target で渡した装着先窓。
        assert_eq!(view.window(), window, "window() が装着先窓 entity と一致しない");

        // (c) surface_size ＝ 合成原寸（物理 px・本 fixture は 3×2）。
        assert_eq!(view.surface_size(), (3, 2), "surface_size() が物理原寸と一致しない");

        // (d) scale ＝ 現行契約の恒常値 1.0（物理 1:1・DPI 契約の共有点）。
        assert_eq!(view.scale(), 1.0, "scale() は現行契約で恒常 1.0");
    }

    /// surface 1000（`w×h` 全不透明 element ＋ bind animation 2000 が surface 5000 を (0,0) に重ねる）
    /// の `(EmoWorld, AtlasTable)` と、bind 無し／bind 有りそれぞれの直接合成 golden を返す。
    ///
    /// 5000 の part（1×1 不透明・base と異色）は base 内に収まるため、bind 有無で**外形は不変・
    /// バイトのみ変わる**（供給面リサイズ経路を踏まずに bind 差分の表示反映だけを固定できる）。
    fn build_target_assets_with_bind(
        w: u32,
        h: u32,
        salt: u8,
    ) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
        let base = Path::new("shell/master");
        let bind_part = Surface {
            id: 5000,
            targets: vec![AppendTarget::Single(5000)],
            elements: vec![elem("q.png", 0, 0)],
            collisions: Vec::new(),
            animations: Vec::new(),
        };
        let base_surface = Surface {
            id: 1000,
            targets: vec![AppendTarget::Single(1000)],
            elements: vec![elem("p.png", 0, 0)],
            collisions: Vec::new(),
            animations: vec![Animation {
                id: 2000,
                interval: Interval::Bind,
                patterns: vec![Pattern {
                    index: 0,
                    method: DrawMethod::new("overlay".to_string()),
                    surface_id: 5000,
                    wait: 0,
                    x: 0,
                    y: 0,
                }],
            }],
        };
        let surfaces = vec![base_surface, bind_part];

        let mut dec = MemoryDecoder::new();
        let stride = w * 4;
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.extend_from_slice(&[b, g, r, 0xFF]);
            }
        }
        dec.insert(base.join("p.png"), w, h, stride, img, true);
        // 1×1 の不透明 part（base 左上と必ず異なる色 → bind 有無でバイトが必ず変わる）。
        dec.insert(base.join("q.png"), 1, 1, 4, vec![0xFF, 0xFF, 0xFF, 0xFF], true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        let atlas = baked.table;

        let mut composer = Composer::new();
        let golden_plain = composer
            .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
            .expect("bind 無し合成は Ok")
            .bytes()
            .to_vec();
        let golden_bound = composer
            .compose(&world, &atlas, 1000, &BindSet::from_ids([2000]), &PatternState::default())
            .expect("bind 有り合成は Ok")
            .bytes()
            .to_vec();
        assert_ne!(
            golden_plain, golden_bound,
            "fixture 前提: bind 有無で合成バイトが異ならなければ回帰檻にならない"
        );

        (world, atlas, golden_plain, golden_bound)
    }

    /// 回帰檻（キャッシュ仕様バグ・実表示レベル）: **同一 surface id で bind 集合だけ変えた**
    /// `ShowSurface` が必ず再合成され、`read_back` が各 bind 状態の直接合成 golden とバイト一致する。
    ///
    /// 旧設計（surface id のみキー）では 2 回目以降が古い合成にヒットし、着せ替え・まばたきの
    /// bind 差分が表示に反映されなかった（2026-07-09 まばたきデモで顕在化）。往復（無し→有り→無し）
    /// で両方向の再合成を固定する。
    #[test]
    fn bind_change_on_same_surface_updates_display() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();

        let (emo_world, atlas, golden_plain, golden_bound) =
            build_target_assets_with_bind(4, 3, 0x2B);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        let show = |presenter: &mut EmoPresenter, world: &mut World, binds: BindSet| {
            let (tx, rx) = reply_channel::<PresentOutcome>();
            presenter.apply(
                world,
                PresentCommand::ShowSurface {
                    target: TargetId(0),
                    surface_id: 1000,
                    binds,
                    reply: Some(tx),
                },
            );
            assert!(
                matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
                "ShowSurface が Ok でない"
            );
        };

        // bind 無し → golden_plain。
        show(&mut presenter, &mut world, BindSet::default());
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            golden_plain,
            "bind 無し表示が直接合成 golden と一致しない"
        );

        // 同一 surface・bind 有り → 再合成されて golden_bound（旧設計はここで古い絵を返した）。
        show(&mut presenter, &mut world, BindSet::from_ids([2000]));
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            golden_bound,
            "bind 追加が表示へ反映されない（合成入力キーの回帰＝着せ替えバグ再発）"
        );

        // bind 無しへ戻す → 再合成されて golden_plain（往復の両方向を固定）。
        show(&mut presenter, &mut world, BindSet::default());
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            golden_plain,
            "bind 除去が表示へ反映されない（合成入力キーの回帰＝着せ替えバグ再発）"
        );
    }

    /// surface 1000／3000 = 同 `w×h`・全不透明・**別バイト**（別 element・別 salt）を持つ単一
    /// world の `(EmoWorld, AtlasTable)` と、各面の直接合成 golden 2 本を返す（build_target_assets の
    /// 複面版）。
    ///
    /// 両面とも α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は両面とも正確に `w×h`
    /// （＝同寸）。ゆえに供給面（chain）リサイズ経路を踏まずに「同寸・異 id 再 Show」だけを固定できる。
    /// golden は presenter が内部で辿るのと同一 world/atlas から作るため readback とのバイト一致が
    /// 二重に決定論的。2 面の golden が別物であることを fixture 自身が assert する（R6.1 の回帰檻前提）。
    fn build_two_face_assets(w: u32, h: u32) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
        let base = Path::new("shell/master");
        let surfaces = vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(3000, vec![elem("q.png", 0, 0)]),
        ];

        let stride = w * 4;
        let gradient = |salt: u8| -> Vec<u8> {
            let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
            for y in 0..h {
                for x in 0..w {
                    let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                    let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                    let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                    img.extend_from_slice(&[b, g, r, 0xFF]);
                }
            }
            img
        };

        let mut dec = MemoryDecoder::new();
        dec.insert(base.join("p.png"), w, h, stride, gradient(0x11), true);
        dec.insert(base.join("q.png"), w, h, stride, gradient(0x77), true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        let atlas = baked.table;

        let mut composer = Composer::new();
        let golden_1000 = composer
            .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
            .expect("面 1000 の合成は Ok")
            .bytes()
            .to_vec();
        let golden_3000 = composer
            .compose(&world, &atlas, 3000, &BindSet::default(), &PatternState::default())
            .expect("面 3000 の合成は Ok")
            .bytes()
            .to_vec();
        assert_ne!(
            golden_1000, golden_3000,
            "fixture 前提: 同寸でも 2 面のバイトが異ならなければ再表示の回帰檻にならない"
        );

        (world, atlas, golden_1000, golden_3000)
    }

    /// R6.1 観測完了（同寸・異 id 再 Show ＝ 新面提示 ＋ 文字スロット安定）: バルーン target が既に
    /// ある面（1000）を表示中に、**同寸の異なる面 id（3000）**を `ShowSurface` すると——(a) reply Ok・
    /// (b) 可視維持・(c) `HitTest::AlphaMask` 維持・(d) `read_back` が **新面 3000 の golden** と一致
    /// （新面が実際に提示された証跡）・(e) `text_slot_view()`（slot/window/surface_size/scale）が切替の
    /// 前後で**完全一致**（文字スロットが安定＝TextSlotView が不変）——をすべて満たす。
    ///
    /// 同寸ゆえ供給面（chain）と装着（mount）は再生成されず（apply_show の `chain.is_none()` 分岐を
    /// 踏まない）、予約 text スロット entity は据え置かれる＝emo-text の描画資源を破壊しない
    /// （design §emo-present 回帰・文字層＝同寸保持）。本 crate 本体は無改変（test-only・R6.3）。
    #[test]
    fn reshow_same_size_different_face_keeps_text_slot_stable() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();

        let (emo_world, atlas, golden_1000, golden_3000) = build_two_face_assets(6, 5);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        // 面 1000 を表示確立（可視・αマスク判定・供給面/装着を遅延生成）。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "面 1000 の初回 ShowSurface が Ok でない"
        );
        // 前提: 初回表示は面 1000 の golden（切替前の基準）。
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back（面 1000）失敗"),
            golden_1000,
            "初回表示が面 1000 の golden と一致しない（前提が崩れている）"
        );

        let surface_entity = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("初回表示後は mount が生成済み")
            .surface_entity();

        // 切替前の文字スロット表示スナップショット（TextSlotView は Copy＝値で退避）。
        let slot_before = presenter
            .text_slot_view(TargetId(0))
            .expect("表示確立後の text_slot_view は Some");

        // 同寸・異 id 再 Show（面 3000）。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 3000,
                binds: BindSet::default(),
                reply: Some(tx1),
            },
        );
        // (a) reply Ok。
        assert!(
            matches!(rx1.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "同寸・異 id（面 3000）の再 ShowSurface が Ok でない"
        );

        // (b) 可視維持。
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "同寸・異 id 再表示後も可視のまま"
        );
        assert!(
            presenter.targets.get(&TargetId(0)).unwrap().visible,
            "同寸・異 id 再表示後も target.visible=true"
        );
        // (c) HitTest::AlphaMask 維持。
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::AlphaMask,
            "同寸・異 id 再表示後も αマスク判定を維持"
        );

        // (d) read_back が新面 3000 の golden と一致（新面が実際に提示された証跡・R6.1）。
        let rb = presenter.read_back(TargetId(0)).expect("read_back（面 3000）失敗");
        assert_eq!(
            rb, golden_3000,
            "再表示のバイトが新面 3000 の golden と一致しない（新面が提示されていない）"
        );
        assert_ne!(
            rb, golden_1000,
            "再表示のバイトが旧面 1000 のまま（面切替が表示へ反映されていない）"
        );

        // (e) 文字スロット表示が切替の前後で完全一致（slot/window/surface_size/scale が不変・R6.1）。
        let slot_after = presenter
            .text_slot_view(TargetId(0))
            .expect("再表示後の text_slot_view は Some");
        assert_eq!(
            slot_before, slot_after,
            "同寸・異 id 再表示で文字スロット表示（slot/window/surface_size/scale）が変化した（TextSlotView が不安定）"
        );
    }

    // ── CurrentSurfaceRead: 現サーフェス id 状態のライフサイクル固定（Task 2・R3.1-3.4）───────────
    // 現サーフェス id は「最後に表示が成立したサーフェス id」（画面の絵でなく表示成立の結果・α非依存）。
    // 書き込みは既存 `visible` 更新点と同一の3箇所のみ（表示成立/EmptyComposition 縮退/Hide）＝additive。

    /// テスト 10・R3.2 観測完了（未表示→None）: `attach_target` 直後（一度も `ShowSurface` していない）は
    /// `current_surface_id` が `None`。`hit_region` も現サーフェス無しゆえ `None`（純関数へ届かない）。
    ///
    /// `attach_target` は skeleton 登録のみで World に触れないため、GPU 不要の素の `World` で決定論固定する。
    #[test]
    fn current_surface_id_is_none_before_first_show() {
        let mut world = World::new();
        let window = world.spawn_empty().id();
        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x10);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            None,
            "attach_target 直後（未表示）は現サーフェス無し（3.2）"
        );
        assert_eq!(
            presenter.hit_region(TargetId(0), 0, 0),
            None,
            "未表示 target の hit_region は現サーフェス無しゆえ None"
        );
    }

    /// テスト 11・R3.1 観測完了（表示後→直近 id）: 有効 `ShowSurface(1000)` 適用後、`current_surface_id`
    /// が `Some(1000)`（直近に表示が成立したサーフェス id）。
    #[test]
    fn current_surface_id_is_last_shown_after_display() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();
        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x11);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );

        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "表示成立後は直近に表示した id（3.1）"
        );
    }

    /// テスト 12・R3.3 観測完了（切替→新 id）: 面 1000 表示中に同寸の別 id 3000 を `ShowSurface` すると、
    /// `current_surface_id` が `Some(3000)` へ追随する（以後の問い合わせは新 id）。
    #[test]
    fn current_surface_id_follows_surface_switch() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();
        let (emo_world, atlas, _g1, _g3) = build_two_face_assets(6, 5);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        let show = |presenter: &mut EmoPresenter, world: &mut World, id: u32| {
            let (tx, rx) = reply_channel::<PresentOutcome>();
            presenter.apply(
                world,
                PresentCommand::ShowSurface {
                    target: TargetId(0),
                    surface_id: id,
                    binds: BindSet::default(),
                    reply: Some(tx),
                },
            );
            assert!(
                matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
                "ShowSurface が Ok でない"
            );
        };

        show(&mut presenter, &mut world, 1000);
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "初回表示成立後は Some(1000)"
        );

        show(&mut presenter, &mut world, 3000);
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(3000),
            "別 id へ切替後は新 id を返す（3.3）"
        );
    }

    /// テスト 13・R3.2/4.4 観測完了（Hide→None）: 有効表示後の `Hide` で `current_surface_id` が `None`
    /// （「未表示等」に Hide が含まれる＝`\s[-1]` 相当で表示していない）。
    #[test]
    fn current_surface_id_is_none_after_hide() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x13);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "Hide 前は Some(1000)（前提）"
        );

        let (txh, rxh) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::Hide {
                target: TargetId(0),
                reply: Some(txh),
            },
        );
        assert!(
            matches!(rxh.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "Hide が Ok でない"
        );

        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            None,
            "Hide 後は現サーフェス無し（3.2/4.4）"
        );
        assert_eq!(
            presenter.hit_region(TargetId(0), 0, 0),
            None,
            "Hide 後は hit_region も現サーフェス無しゆえ None"
        );
    }

    /// テスト 14 観測完了（InvalidateCache→不変）: 有効表示後に `InvalidateCache` を適用しても
    /// `current_surface_id` は不変（キャッシュ無効化は表示を変えない）。単一真実源が `ComposeKey` 由来では
    /// なくフィールドであることの回帰檻（`invalidate_all` でキーが消えても現サーフェス id は残る）。
    #[test]
    fn current_surface_id_unchanged_by_invalidate_cache() {
        let mut world = make_world_with_gpu();
        let window = world.spawn_empty().id();
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x14);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas)
            .expect("attach_target 失敗");

        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );

        let (txi, rxi) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::InvalidateCache {
                target: TargetId(0),
                reply: Some(txi),
            },
        );
        assert!(
            matches!(rxi.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "InvalidateCache が Ok でない"
        );

        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "InvalidateCache は表示を変えないため現サーフェス id は不変（ComposeKey 由来案の棄却根拠）"
        );
    }

    /// テスト 15・R3.2 観測完了（未登録 target→None）: 一度も `attach_target` していない target に対し
    /// `current_surface_id`／`hit_region` の両アクセサが `None`（未登録＝現サーフェス無し）。
    ///
    /// 両アクセサとも `HashMap` 引きのみで GPU/World を要さないため、`EmoPresenter::new()` 単体で固定する。
    #[test]
    fn unregistered_target_returns_none_for_both_accessors() {
        let presenter = EmoPresenter::new();
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            None,
            "未登録 target の current_surface_id は None"
        );
        assert_eq!(
            presenter.hit_region(TargetId(0), 10, 20),
            None,
            "未登録 target の hit_region は None"
        );
    }
}
