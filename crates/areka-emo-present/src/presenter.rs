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
use areka_emo_compose::{BindSet, ComposeError, Composer, EmoWorld};

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
    /// surface id → (composed, mask) 対の全保持キャッシュ。
    cache: ComposeCache,
    /// 装着先の窓 Entity（R1.3・遅延装着の対象）。
    window: Entity,
    /// 窓装着ハンドル（初回表示で生成）。
    mount: Option<VisualMount>,
    /// 自前供給面（初回表示で原寸確定後に生成）。
    chain: Option<SwapChainPresenter>,
    /// 現在可視か（`Hide`／全透明退化で false・`ShowSurface` 成功で true）。
    visible: bool,
}

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
    /// 手順（design §System Flows）: (1) 未装着なら error! ＋ `Err(TargetNotAttached)`。(2) キャッシュ
    /// ヒットなら再合成しない（R4.2）。(3) ミスなら合成し、`SurfaceNotFound` は error! ＋表示不変＋
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

        // (1) 引き当て: キャッシュヒットは再合成しない（R4.2）。ミスのみ合成する。
        let cache_hit = target.cache.get(surface_id).is_some();
        if !cache_hit {
            match target
                .composer
                .compose(&target.emo_world, &target.atlas, surface_id, &binds)
            {
                Ok(composed) => {
                    // 挿入時にマスクを 1 回だけ生成し、表示バッファと対で束ねる（R2.1/R2.4）。
                    target.cache.insert(surface_id, composed);
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
                let entry = target.cache.get(surface_id).expect("直前に引き当て済み");
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
        let entry = target.cache.get(surface_id).expect("直前に引き当て済み");
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
    use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};

    use wintf::ecs::{GraphicsCore, WucGraphicsResource};
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
            .compose(&world, &atlas, 1000, &BindSet::default())
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
}
