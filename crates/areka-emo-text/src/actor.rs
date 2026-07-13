//! # actor — UI ドレインとフレーム提示ステップ（結線層）
//!
//! `spawn_emo_text`（`spawn_ui` 結線・UI ドレイン起動）・`TextLayerRuntime`
//! （UI スレッド所有の集約ルート）・`TextSlotBinding`・`present_frame`
//! （毎フレームの注入時刻駆動：リビール進行→レイアウト→描画→装着）を担う。
//!
//! **層規律**: 結線層。終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`、
//! 全 `UiSender` drop＝drain 正常終了（いずれも error ログなし）。個別メッセージの処理失敗は
//! `Err` 戻し→基盤が `error!`＋継続（log-first・ループを殺さない）。

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use areka_actor::UiSpawnError;
use areka_emo_present::TextSlotView;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use tracing::{debug, error, info, warn};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use crate::canvas::ContentCanvas;
use crate::draw::{DWriteMetrics, ResolvedFont};
use crate::layout::LayoutEngine;
use crate::region::{ImagePx, ScaleContract, TextRegion};
use crate::sink::{handle_text_msg, EmoTextSink, TextMsg};
use crate::state::{TextLayerConfig, TextLayerState};
use crate::surface::TextSurface;
use crate::viewbox_draw::{DrawStats, ViewboxExecutor};
use crate::writing::WritingMode;
use crate::TextLayerError;

/// actor の装着先（結線側が emo-present `TextSlotView` から構築して routing へ登録する）。
///
/// [`crate::surface::TextSurface::attach`] の入力。emo-present は actor を知らない
/// （層純度維持・R9.5）——`ActorKey → TargetId` の対応は結線側（example/emo2-boot）が所有し、
/// `text_slot_view(target)` で得た view の値から本型を [`Self::new`] で組む。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSlotBinding {
    /// 予約スロット（`emo-text-layer-slot` Visual entity・emo-present `VisualMount` が予約）。
    pub slot: Entity,
    /// 装着先の窓 entity。
    pub window: Entity,
    /// 合成スケール k（`TextSlotView.scale` 由来・現行契約 1.0 恒常）。
    /// 不正値は構築時に [`ScaleContract`] の縮退規約（warn!＋1.0）で正規化済み。
    pub scale: f32,
    /// バルーン surface の物理原寸（TextSurface/swapchain の物理化に使用）。
    pub surface_size: (u32, u32),
    /// 画像座標空間の原寸（負値=反対辺解決・`TextRegion::resolve` の入力）。
    ///
    /// **構築時に一点導出**: `image_size = round(surface_size / k)`（k=1.0 恒常の現行契約では
    /// `surface_size` と同値）。`TextRegion::resolve` へ物理 px を渡すのはレビューエラー
    /// （2 空間モデルの綻び目をここで構造閉塞——design.md「DPI/スケール契約」）。
    pub image_size: (u32, u32),
}

impl TextSlotBinding {
    /// `TextSlotView` の読み値（slot/window/scale/surface_size）から binding を構築し、
    /// `image_size = round(surface_size / k)` をここで**一点導出**する。
    ///
    /// k の正規化（0 以下・非有限→warn!＋1.0 縮退）は [`ScaleContract::new`] に委譲し、
    /// 保持する `scale` と `image_size` の導出が常に同一の k で自己整合するようにする
    /// （k の多重適用・混在の構造排除——design.md 不変条件 (3)）。
    pub fn new(slot: Entity, window: Entity, scale: f32, surface_size: (u32, u32)) -> Self {
        let contract = ScaleContract::new(scale, None);
        TextSlotBinding {
            slot,
            window,
            scale: contract.scale,
            surface_size,
            image_size: contract.image_size(surface_size),
        }
    }

    /// emo-present の読み取り専用増分 `TextSlotView` からの一点変換（結線の正準口・R9.1/R9.2）。
    ///
    /// view の読み値（slot/window/scale/surface_size）を [`Self::new`] へ透過するだけで、
    /// `image_size = round(surface_size / k)` の一点導出は `new` に集約されたまま変わらない
    /// （k の多重適用・混在の構造排除——design.md「DPI/スケール契約」）。
    pub fn from_view(view: &TextSlotView) -> Self {
        TextSlotBinding::new(view.slot(), view.window(), view.scale(), view.surface_size())
    }
}

/// actor 1 人分の layout 入力——`writing_mode`／領域／フォントの解決済み束
/// （design.md `TextLayerRuntime.layout_input` の値型）。
///
/// 解決はすべて既存の一点解決口（[`WritingMode::resolve`]／[`TextRegion::resolve`]／
/// [`ResolvedFont::resolve`]）の合成であり、本型は束ねるだけで独自解釈を持たない。
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedBalloonText {
    /// 2 層マージ済み `writing_mode` の解釈結果。
    pub mode: WritingMode,
    /// 解決済みテキスト領域（validrect 絶対矩形・描画開始点・折返し閾値——全値 image px）。
    pub region: TextRegion,
    /// 解決済みフォント一式（欠落は ukadoc 既定で充足済み）。
    pub font: ResolvedFont,
}

impl ResolvedBalloonText {
    /// balloon model（2 層マージ済み）とバルーン画像原寸（**image px**・
    /// [`TextSlotBinding::image_size`] の一点導出値）から解決する。
    /// 物理 px を渡すのはレビューエラー（2 空間モデル——design.md「DPI/スケール契約」）。
    pub fn resolve(model: &BalloonModel, image_size: (u32, u32)) -> ResolvedBalloonText {
        let mode = WritingMode::resolve(model);
        ResolvedBalloonText {
            mode,
            region: TextRegion::resolve(model, image_size, mode),
            font: ResolvedFont::resolve(model),
        }
    }
}

/// actor 1 人分の描画資源（供給面＋描画実行部＋実測 metrics——font/mode に束縛されるため
/// actor 別に持つ。行 TextLayout キャッシュの index 衝突・format 組み直しの構造回避）。
struct ActorRender {
    /// 自前 swapchain 供給面（初回のみ予約スロットへ brush 装着・以降 Present のみ）。
    surface: TextSurface,
    /// viewbox ダーティ矩形スクロールの実行部（保持ピクセルの面内 blit ＋ ダーティ矩形限定描画・
    /// 行 TextLayout キャッシュは actor の行 index に束縛）。
    executor: ViewboxExecutor,
    /// 計測専用 probe 由来の実測 metrics（actor の font/mode に束縛）。
    metrics: DWriteMetrics,
}

/// UI スレッド所有の集約ルート（NonSend・design.md「TextLayerRuntime」正本）。
///
/// 純粋状態（[`TextLayerState`]）・binding（結線側が登録）・COM 資源（actor 別の
/// 供給面/描画実行部——World 資源から遅延構築）を束ねる。`Rc<RefCell<_>>` で
/// [`spawn_emo_text`] の UI ドレインとフレーム提示ステップ（[`present_frame`]）が
/// 同一 UI スレッド上で共有する（`!Send`——スレッドを跨がない）。
///
/// - cue 適用（[`apply_cue`](Self::apply_cue)）は純粋状態の更新のみで World に触れない。
/// - World への装着・描画はフレーム提示ステップ（[`present_frame`]）が担う。
/// - 時刻は常に注入（`talk_time`）——内部で `Instant::now()` を読まない（R3.3）。
pub struct TextLayerRuntime {
    /// 純粋状態機械（actor 別・cue 列→行/グリフ状態）。
    state: TextLayerState,
    /// actor → 装着先（予約スロット）。結線側が [`register_actor`](Self::register_actor) で登録。
    routing: HashMap<ActorKey, TextSlotBinding>,
    /// actor → layout 入力（writing_mode/region/font の解決済み束）。routing と対で登録。
    layout_input: HashMap<ActorKey, ResolvedBalloonText>,
    /// actor 別の描画資源（遅延生成——初回解決フレームで World 資源から構築・装着）。
    surfaces: HashMap<ActorKey, ActorRender>,
    /// 調整値（char_wait／line_pitch 係数）。
    config: TextLayerConfig,
    /// 未解決 actor の warn を actor ごと初回のみに抑える記録（以降は debug!——
    /// design Error Handling「未 binding actor の cue」）。
    unresolved_warned: BTreeSet<ActorKey>,
}

impl TextLayerRuntime {
    /// 空のランタイムを構築する（COM 資源は初回解決フレームで World 資源から遅延構築）。
    pub fn new(config: TextLayerConfig) -> TextLayerRuntime {
        TextLayerRuntime {
            state: TextLayerState::default(),
            routing: HashMap::new(),
            layout_input: HashMap::new(),
            surfaces: HashMap::new(),
            config,
            unresolved_warned: BTreeSet::new(),
        }
    }

    /// actor と装着先（予約スロット）＋layout 入力の対応を登録する（結線側の口——
    /// `ActorKey → TargetId` の対応は結線側が所有し、emo-present `TextSlotView` の読み値
    /// から [`TextSlotBinding::new`]／[`ResolvedBalloonText::resolve`] で組んで渡す・R9.5）。
    ///
    /// 未解決のまま蓄積していた actor は次の [`present_frame`] で再試行され装着される。
    pub fn register_actor(
        &mut self,
        actor: ActorKey,
        binding: TextSlotBinding,
        resolved: ResolvedBalloonText,
    ) {
        debug!(actor = %actor, slot = ?binding.slot, "actor の装着先（予約スロット）を登録した");
        self.routing.insert(actor.clone(), binding);
        self.layout_input.insert(actor, resolved);
    }

    /// 統合配線の一点口（R9.5・task 8）: `ActorKey → TargetId` 対応の解決結果
    /// （`EmoPresenter::text_slot_view(target)` の view——対応関係の所有は結線側・
    /// example/emo2-boot）と 2 層マージ済み balloon model から、binding
    /// （[`TextSlotBinding::from_view`]）と layout 入力（[`ResolvedBalloonText::resolve`]・
    /// 入力は必ず binding の `image_size`＝**image px**）を導出して
    /// [`register_actor`](Self::register_actor) へ登録する。
    ///
    /// 物理 px を領域解決へ渡す誤配線をこの口で構造閉塞する（2 空間モデル——design.md
    /// 「DPI/スケール契約」）。行レイアウト（`PositionedLine`）・クリック可能範囲の
    /// choice-render 再利用シーム（R9.4）には手を触れない——本口は入力の供給のみで、
    /// レイアウト出力の形を変えない。
    ///
    /// `text_slot_view` は初回 `ShowSurface` 前は `None` を返すため、呼び手は表示確立後に
    /// 本口で登録する（未登録の間に届いた cue は蓄積され、登録後の次フレームで装着・描画される）。
    pub fn register_actor_view(
        &mut self,
        actor: ActorKey,
        view: &TextSlotView,
        model: &BalloonModel,
    ) {
        let binding = TextSlotBinding::from_view(view);
        let resolved = ResolvedBalloonText::resolve(model, binding.image_size);
        self.register_actor(actor, binding, resolved);
    }

    /// cue を actor 別の純粋状態機械へ適用する（UI ドレインの適用点・World に触れない）。
    ///
    /// `Clear` は全域リセット要求点でもある（planner 初期化＋確定行 TextLayout キャッシュの全破棄——
    /// design「Clear で全破棄」・破棄はこの口だけ・次フレームは `FramePlan::FullClear`＝全域透明・R4.3）。
    pub fn apply_cue(&mut self, cue: &TalkCue) {
        if matches!(cue.command, CueCommand::Clear) {
            if let Some(render) = self.surfaces.get_mut(&cue.actor) {
                render.executor.request_clear();
            }
        }
        self.state.apply_cue(cue, &self.config);
    }

    /// 純粋状態機械（可視グリフ数・actor 状態の読み取り口）。
    pub fn state(&self) -> &TextLayerState {
        &self.state
    }

    /// 調整値（char_wait／line_pitch 係数）。
    pub fn config(&self) -> &TextLayerConfig {
        &self.config
    }

    /// actor の供給面が予約スロットへ装着済みか。
    pub fn is_attached(&self, actor: &ActorKey) -> bool {
        self.surfaces.contains_key(actor)
    }

    /// 装着済み actor の供給面（readback 等の観測口・未装着は `None`）。
    pub fn surface(&self, actor: &ActorKey) -> Option<&TextSurface> {
        self.surfaces.get(actor).map(|render| &render.surface)
    }

    /// 装着済み actor の決定論観測統計（[`ViewboxExecutor::stats`]・未装着は `None`）。
    ///
    /// [`surface`](Self::surface) と同型の additive アクセサ（R9.2 非抵触——emo2-boot 消費経路の
    /// 再定義ではない）。example／統合テストがこの口から actor 別の [`DrawStats`]
    /// （blit・`DrawTextLayout` 実行回数・行 TextLayout 生成回数・FullClear 回数）を読み、
    /// 「可視窓のみ移動フレームで確定 content の再描画が起きない」等を決定論的に観測する
    /// （R3.5/R10.3・目視非依存）。`ViewboxExecutor::stats()` は runtime 内部の `ActorRender` に
    /// 抱えられており、この読み口がないと example から R10.3 checkpoint が成立しない。
    pub fn draw_stats(&self, actor: &ActorKey) -> Option<DrawStats> {
        self.surfaces.get(actor).map(|render| render.executor.stats())
    }
}

/// 結線 API（UI スレッド＝pump スレッドから呼ぶ・design.md「TextLayerActor」正本）:
/// `spawn_ui` で UI ドレインを起動し、受信口 [`EmoTextSink`] と drain の join ハンドルを返す。
///
/// handler は `runtime` の `Rc` clone を捕捉し（`!Send` handler・基盤許容）、
/// [`handle_text_msg`]（終了規律の正準写像）へ委譲して cue を純粋状態へ適用する。
/// 終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`・全 `UiSender`
/// （＝全 [`EmoTextSink`] クローン）drop＝drain 正常終了（R1.4・error ログなし）。
/// 個別 cue の適用失敗（runtime 借用競合など）は `Err` 戻し→基盤が `error!`＋継続する
/// （R1.5——失敗は終了経路ではない・panic しない）。
///
/// # 前提
///
/// UI（pump）スレッドから呼ぶこと。誤用は呼出時検出不能（基盤既知リスク——`spawn_ui` の
/// Risks 参照・spawn 時 debug! 診断で緩和）。
pub fn spawn_emo_text(
    runtime: Rc<RefCell<TextLayerRuntime>>,
) -> Result<(EmoTextSink, wintf_winmsg_executor::JoinHandle<()>), UiSpawnError> {
    let (tx, handle) = areka_actor::spawn_ui("emo-text", move |msg: TextMsg| {
        handle_text_msg(msg, |cue| match runtime.try_borrow_mut() {
            Ok(mut rt) => {
                rt.apply_cue(&cue);
                Ok(())
            }
            // 借用競合（UI スレッド上の別処理が runtime を保持中）——panic せず Err 戻しで
            // 基盤の error!＋継続に乗せる（当該 cue は失われるが後続の受理は破壊しない・R1.5）。
            Err(_) => Err(format!(
                "TextLayerRuntime が借用中のため cue を適用できない（actor={}, at={}）——当該 cue は失われる",
                cue.actor, cue.at
            )),
        })
    })?;
    Ok((EmoTextSink::new(tx), handle))
}

/// フレーム提示ステップ（毎フレーム UI スレッドで呼ぶ・example/emo2-boot が駆動）:
/// `talk_time` は注入時刻（talk 起点相対秒・実時間 sleep 不使用・R3.3）。
///
/// actor ごとに「リビール進行の解決（純粋）→ レイアウト決定（純粋）→ viewbox ダーティ矩形
/// スクロール描画（COM）→ 変化ありのフレームだけ供給面を提示（Present のみ）」を駆動する:
///
/// - **未解決 actor**（binding 未登録）: 状態は蓄積のみ・描画スキップ・次フレーム再試行
///   （actor ごと初回 `warn!`＋以降 `debug!`——frame の `Err` にはしない）。
/// - **初回解決フレーム**: World 資源（[`GraphicsCore`]／[`WucGraphicsResource`]）から
///   供給面/描画実行部を構築し予約スロットへ装着する（actor ごと初回のみ・`info!`）。
/// - **装着済み actor のグリフ更新**: viewbox ダーティ矩形スクロール描画→（変化ありのフレームだけ）
///   swapchain Present で完結し、バルーン surface 本体の再合成（emo-compose 再駆動）を要求しない（R9.3）。
/// - **デバイス失敗**: 失敗源で `error!` 済み（log-first）。当該 actor の当該フレーム提示を
///   skip して他 actor の処理は継続し、最初の失敗を `Err` として返す（次フレーム再試行）。
pub fn present_frame(
    runtime: &mut TextLayerRuntime,
    world: &mut World,
    talk_time: f64,
) -> Result<(), TextLayerError> {
    // 状態を持つ actor だけが提示対象（binding 登録済みでも cue が無ければ描くものがない）。
    let actors: Vec<ActorKey> = runtime.state.actors().map(|(key, _)| key.clone()).collect();
    let mut first_err: Option<TextLayerError> = None;
    for actor in &actors {
        match present_actor(runtime, world, actor, talk_time) {
            Ok(()) => {}
            // 未解決 actor: 蓄積継続・描画スキップ・次フレーム再試行（正常経路・Err にしない）。
            Err(TextLayerError::SlotNotAttached { .. }) => {
                if runtime.unresolved_warned.insert(actor.clone()) {
                    warn!(
                        actor = %actor,
                        talk_time,
                        "actor の装着先（予約スロット）が未解決——状態を蓄積し描画をスキップして次フレームで再試行する"
                    );
                } else {
                    debug!(
                        actor = %actor,
                        talk_time,
                        "actor の装着先が未解決のまま——蓄積継続・描画スキップ・次フレーム再試行"
                    );
                }
            }
            // デバイス失敗: 失敗源で error! 済み。他 actor は継続し、最初の失敗を返す。
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// 1 actor 分のフレーム提示（[`present_frame`] の内訳・log-first）。
///
/// binding 未解決は [`TextLayerError::SlotNotAttached`]（呼び手が skip 写像）。
fn present_actor(
    runtime: &mut TextLayerRuntime,
    world: &mut World,
    actor: &ActorKey,
    talk_time: f64,
) -> Result<(), TextLayerError> {
    // ── 解決確認: binding＋layout 入力が揃うまでは蓄積のみ（描画スキップ） ──
    let (Some(binding), Some(resolved)) = (
        runtime.routing.get(actor).copied(),
        runtime.layout_input.get(actor).cloned(),
    ) else {
        return Err(TextLayerError::SlotNotAttached {
            actor: actor.to_string(),
        });
    };

    let contract = ScaleContract::new(binding.scale, None);

    // ── 初回解決フレーム: World 資源から描画資源を構築し予約スロットへ装着（初回のみ） ──
    if !runtime.surfaces.contains_key(actor) {
        let region = &resolved.region;
        // 物理寸＝ceil(validrect 寸 × k)・offset＝validrect 原点 × k（DPI/スケール契約）。
        let physical_size = (
            contract.physical_extent(ImagePx(region.right() - region.left())),
            contract.physical_extent(ImagePx(region.bottom() - region.top())),
        );
        let physical_offset = (
            contract.to_physical(ImagePx(region.left())).0,
            contract.to_physical(ImagePx(region.top())).0,
        );

        // Compositor は所有クローンで取り出し、以後の &mut World 装着と借用衝突しない
        // ようにする（emo-present presenter.rs と同じ規律）。
        let Some(compositor) = world
            .get_resource::<WucGraphicsResource>()
            .and_then(|resource| resource.compositor().cloned())
        else {
            error!(
                actor = %actor,
                "present_frame: WucGraphicsResource/Compositor 不在（供給面を生成できない）"
            );
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "WucGraphicsResource::compositor",
            });
        };
        if !world.contains_resource::<GraphicsCore>() {
            error!(
                actor = %actor,
                "present_frame: GraphicsCore 不在（供給面を生成できない）"
            );
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "GraphicsCore resource",
            });
        }
        // GraphicsCore は resource_scope で一時取り外し——attach の &mut World と
        // 資源借用の衝突を構造回避する。
        let config = runtime.config;
        let render = world.resource_scope(
            |world, core: bevy_ecs::world::Mut<GraphicsCore>| -> Result<ActorRender, TextLayerError> {
                let surface = TextSurface::attach(
                    world,
                    &binding,
                    &compositor,
                    &core,
                    physical_size,
                    physical_offset,
                )?;
                let executor = ViewboxExecutor::new(&core)?;
                let Some(factory) = core.dwrite_factory() else {
                    error!(actor = %actor, "present_frame: dwrite_factory 不在（metrics を構築できない）");
                    return Err(TextLayerError::Device {
                        hresult: 0,
                        context: "GraphicsCore::dwrite_factory",
                    });
                };
                let metrics = DWriteMetrics::new(factory, &resolved.font, resolved.mode, &config)?;
                Ok(ActorRender {
                    surface,
                    executor,
                    metrics,
                })
            },
        )?;
        runtime.surfaces.insert(actor.clone(), render);
        info!(
            actor = %actor,
            slot = ?binding.slot,
            ?physical_size,
            "テキスト供給面を予約スロットへ装着した（actor ごと初回のみ・以降は Present のみ）"
        );
    }

    // ── リビール進行（純粋）→ レイアウト決定（純粋）→ viewbox ダーティ矩形描画 → 提示（Present のみ） ──
    let Some(render) = runtime.surfaces.get_mut(actor) else {
        // 直前の insert 直後に到達するため構造上起こらない——防御（panic 禁止・log-first）。
        error!(actor = %actor, "present_frame: 装着済み描画資源の引き当てに失敗（構造不変の破れ）");
        return Err(TextLayerError::Device {
            hresult: 0,
            context: "ActorRender missing after attach",
        });
    };
    let Some(actor_state) = runtime.state.actor_state(actor) else {
        // present_frame は state.actors() 由来の actor だけを渡す——防御的に空フレーム扱い。
        return Ok(());
    };
    // レイアウトへは可視 **item** prefix 長を渡す（グリフ＋リビール済み改行を通算・item 単位モデル）。
    // 未リビールの改行は prefix 外に落ちて適用されない——`\n` 直前の `\w` を尊重する（#4）。
    let visible = runtime.state.visible_items(actor, talk_time);
    let lines = LayoutEngine::layout(
        actor_state.items(),
        visible,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &render.metrics,
    );
    let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);
    let canvas = ContentCanvas::from_layout(&lines, &resolved.region, resolved.mode);
    let changed = render.executor.render(
        &canvas,
        &window,
        &resolved.font,
        resolved.mode,
        &contract,
        &mut render.surface,
    )?;
    // 装着済み actor のグリフ更新は供給面の提示のみで完結（emo-compose 再駆動なし・R9.3）。
    // 変化ありのフレームだけ提示する（`FramePlan::NoChange` は blit も描画も present も省く——
    // readback は front を読むため観測述語に影響しない・R1.1/R3.1）。
    if changed {
        render.surface.present()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::TextSlotBinding;
    use bevy_ecs::prelude::World;

    /// k=1.0（現行契約の物理 1:1）: `image_size` は `surface_size` と同値、
    /// slot/window/scale/surface_size は透過保持される。
    #[test]
    fn binding_identity_scale_keeps_surface_size() {
        let mut world = World::new();
        let slot = world.spawn_empty().id();
        let window = world.spawn_empty().id();

        let binding = TextSlotBinding::new(slot, window, 1.0, (434, 687));
        assert_eq!(binding.slot, slot);
        assert_eq!(binding.window, window);
        assert_eq!(binding.scale, 1.0);
        assert_eq!(binding.surface_size, (434, 687));
        assert_eq!(
            binding.image_size,
            (434, 687),
            "k=1.0 恒常の現行契約では image_size == surface_size"
        );
    }

    /// 一点導出 `image_size = round(surface_size / k)` の端数檻（Implementation Notes 2.4:
    /// floor/ceil 変異を殺す値で檻化する）。
    ///
    /// k=1.25・surface=(127, 94): 127/1.25=101.6→**102**（floor 変異=101 を殺す）、
    /// 94/1.25=75.2→**75**（ceil 変異=76 を殺す）。
    #[test]
    fn binding_derives_image_size_by_round_fractional() {
        let mut world = World::new();
        let slot = world.spawn_empty().id();
        let window = world.spawn_empty().id();

        let binding = TextSlotBinding::new(slot, window, 1.25, (127, 94));
        assert_eq!(
            binding.image_size,
            (102, 75),
            "round(127/1.25)=102（floor では 101）・round(94/1.25)=75（ceil では 76）"
        );
        assert_eq!(binding.surface_size, (127, 94), "物理原寸はそのまま保持");
    }

    /// k=2.0 の .5 境界: 101/2=50.5→51・51/2=25.5→26（round half away from zero）。
    #[test]
    fn binding_derives_image_size_half_boundary() {
        let mut world = World::new();
        let slot = world.spawn_empty().id();
        let window = world.spawn_empty().id();

        let binding = TextSlotBinding::new(slot, window, 2.0, (101, 51));
        assert_eq!(binding.image_size, (51, 26));
    }

    /// 不正な k（0 以下・非有限）は ScaleContract の縮退規約（warn!＋1.0）へ乗り、
    /// binding の scale / image_size とも物理 1:1 で自己整合する（k の多重適用・混在の構造排除）。
    #[test]
    fn binding_degrades_invalid_scale_to_identity() {
        let mut world = World::new();
        let slot = world.spawn_empty().id();
        let window = world.spawn_empty().id();

        let binding = TextSlotBinding::new(slot, window, 0.0, (320, 240));
        assert_eq!(binding.scale, 1.0, "不正 k は 1.0 へ縮退（log-first・panic なし）");
        assert_eq!(binding.image_size, (320, 240));
    }
}

#[cfg(test)]
mod runtime_tests {
    // task 7.2 の檻: UI スレッド常駐ランタイム（TextLayerRuntime／spawn_emo_text／
    // present_frame）。実 pump 上の終了指示・全送信元切断・個別失敗継続の 3 経路
    // （R1.2/R1.3/R1.4）と、注入時刻フレーム駆動での可視グリフ進行・未解決 actor の
    // 蓄積＋スキップ＋再試行・装着済み actor の Present 完結（R9.3）を檻化する。

    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };
    use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
    use areka_sakura::TextSink;
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_ecs::name::Name;
    use bevy_ecs::prelude::World;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
    use wintf::ecs::{GraphicsCommandList, GraphicsCore, Visual, VisualGraphics, WucGraphicsResource};
    use wintf_winmsg_executor::{FilterResult, MessageLoop};

    use super::{present_frame, spawn_emo_text, ResolvedBalloonText, TextLayerRuntime, TextSlotBinding};
    use crate::state::TextLayerConfig;

    // ── ログ檻（WARN/ERROR 件数を数える最小 Subscriber・sink.rs の檻パターン踏襲） ──

    struct LevelCounter {
        warns: Arc<AtomicUsize>,
        errors: Arc<AtomicUsize>,
    }

    impl tracing::Subscriber for LevelCounter {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            match *event.metadata().level() {
                tracing::Level::WARN => {
                    self.warns.fetch_add(1, Ordering::SeqCst);
                }
                tracing::Level::ERROR => {
                    self.errors.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    /// クロージャをログ檻の中で実行し、（結果, WARN 件数, ERROR 件数）を返す。
    fn with_log_cage<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
        let warns = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let subscriber = LevelCounter {
            warns: Arc::clone(&warns),
            errors: Arc::clone(&errors),
        };
        let out = tracing::subscriber::with_default(subscriber, f);
        (
            out,
            warns.load(Ordering::SeqCst),
            errors.load(Ordering::SeqCst),
        )
    }

    // ── テスト土台 ──

    /// テスト用 cue。
    fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
        TalkCue {
            at,
            actor: ActorKey::from(actor),
            command,
        }
    }

    /// 事前 queue 済みメッセージを全て処理し queue が空になった時点で抜ける bounded pump
    /// （sink.rs テストの決定論パターン——WM_QUIT は posted メッセージが尽きた後にのみ配送）。
    fn pump_until_idle() {
        // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけの
        // 無害な Win32 呼び出し（テストスレッド＝pump スレッド）。
        unsafe { PostQuitMessage(0) };
        MessageLoop::run(|_, _| FilterResult::Forward);
    }

    /// テスト用 BalloonModel（幾何のみ・font 未指定＝既定 ＭＳ ゴシック 12px・
    /// validrect 未指定＝画像全域・wordwrap 未指定＝右辺折返し）。
    fn geo_model() -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(Some(0), Some(0)),
            WordWrapPoint::new(None, None),
            ValidRect::new(None, None, None, None),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
        )
    }

    /// emo-present `VisualMount` と同型の予約スロット（surface.rs テストと同型）。
    /// 返り値は (window, slot)。
    fn spawn_reserved_slot(world: &mut World) -> (bevy_ecs::entity::Entity, bevy_ecs::entity::Entity) {
        let window = world.spawn_empty().id();
        let slot = world
            .spawn((
                Name::new("emo-text-layer-slot"),
                Visual::default(),
                ChildOf(window),
            ))
            .id();
        world.flush();
        (window, slot)
    }

    /// 非透明ピクセル数（BGRA 密配列の α ≠ 0・draw.rs 檻と同じ述語）。
    fn opaque_count(bytes: &[u8]) -> usize {
        bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
    }

    // ══ 実 pump 3 経路（R1.2/R1.3/R1.4/R1.5——観測可能な完了状態の前半） ══

    /// ケース1: 終了指示（`close()`）——実 pump 上で cue が UI ドレイン経由で状態機械へ
    /// 適用され（R1.2/R1.3）、Close 受領で drain が error ログなしにクリーン終了する（R1.4）。
    /// drain 終了は「handler クロージャ（runtime の Rc clone を捕捉）の drop」を
    /// `Rc::strong_count` で決定論観測する。
    #[test]
    fn close_terminates_drain_cleanly_after_applying_cues_on_real_pump() {
        let ((), _warns, errors) = with_log_cage(|| {
            let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
            let (mut sink, _handle) =
                spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
            assert_eq!(Rc::strong_count(&runtime), 2, "drain handler が runtime を保持する");

            sink.emit(cue("0", 0.0, CueCommand::Text("アヒル".into())));
            sink.emit(cue("0", 0.2, CueCommand::NewLine { ratio: 1.0 }));
            sink.close();

            pump_until_idle();

            // cue は UI ドレイン経由で状態機械へ適用済み（R1.2——描画状態の更新）。
            {
                let rt = runtime.borrow();
                let actor = ActorKey::from("0");
                let state = rt.state().actor_state(&actor).expect("actor state が生成される");
                assert_eq!(state.items().len(), 4, "グリフ 3＋改行マーカー 1 が追記される");
                // 注入時刻でのリビール進行（既定 char_wait=0.05・丸め安全マージン付き時刻）。
                assert_eq!(rt.state().visible_glyphs(&actor, 0.0), 1);
                assert_eq!(rt.state().visible_glyphs(&actor, 0.06), 2);
                assert_eq!(rt.state().visible_glyphs(&actor, 0.11), 3);
            }

            // 送信元（sink）は生存したまま＝終了は Close（Ok(Break)）経路のみでありうる。
            assert_eq!(
                Rc::strong_count(&runtime),
                1,
                "Close 受領で drain future が drop されクリーン終了する"
            );
            drop(sink);
        });
        assert_eq!(errors, 0, "終了指示によるクリーン終了は error ログを伴わない");
    }

    /// ケース2: 全送信元切断（全 `EmoTextSink` clone drop）——queue 済み cue を適用し切った
    /// 上で error ログなしにクリーン終了する（R1.4）。
    #[test]
    fn dropping_all_sinks_terminates_drain_cleanly_after_applying_queued_cues() {
        let ((), _warns, errors) = with_log_cage(|| {
            let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
            let (mut sink, _handle) =
                spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
            let sink2 = sink.clone();

            sink.emit(cue("1", 0.0, CueCommand::Text("残置".into())));
            drop(sink);
            drop(sink2);

            pump_until_idle();

            let rt = runtime.borrow();
            let actor = ActorKey::from("1");
            let state = rt.state().actor_state(&actor).expect("切断前に queue 済みの cue は届く");
            assert_eq!(state.items().len(), 2, "切断前の cue は破棄されず適用される");
            drop(rt);

            assert_eq!(
                Rc::strong_count(&runtime),
                1,
                "全送信元切断で drain future が drop されクリーン終了する"
            );
        });
        assert_eq!(errors, 0, "全送信元切断によるクリーン終了は error ログを伴わない");
    }

    /// ケース3: 個別失敗継続——runtime が借用中（UI スレッド上の別処理が保持）で cue 適用に
    /// 失敗しても panic せず error 1 件で drain は継続し（R1.5）、借用解放後の後続 cue は
    /// 適用され、Close でクリーン終了する。
    #[test]
    fn cue_failure_is_logged_and_drain_continues_processing_subsequent_cues() {
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        let (mut sink, _handle) =
            spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");

        // 借用を保持したまま pump——drain handler は try_borrow_mut に失敗し Err（基盤が
        // error!＋継続）。当該 cue は失われるが drain は死なない。
        let ((), _warns, errors) = with_log_cage(|| {
            let guard = runtime.borrow_mut();
            sink.emit(cue("0", 0.0, CueCommand::Text("失われる".into())));
            pump_until_idle();
            drop(guard);
        });
        assert_eq!(errors, 1, "個別適用失敗はちょうど 1 件の error ログとして記録される");
        assert_eq!(
            Rc::strong_count(&runtime),
            2,
            "個別失敗は終了経路ではない——drain は生き続ける"
        );

        // 借用解放後の後続 cue は受理・適用され、Close でクリーン終了する。
        let ((), _warns, errors) = with_log_cage(|| {
            sink.emit(cue("0", 0.5, CueCommand::Text("後続".into())));
            sink.close();
            pump_until_idle();
        });
        assert_eq!(errors, 0, "失敗後の後続 cue 適用と Close 終了は error ログを伴わない");

        let rt = runtime.borrow();
        let actor = ActorKey::from("0");
        let state = rt.state().actor_state(&actor).expect("後続 cue が適用される");
        assert_eq!(state.items().len(), 2, "失敗 cue は失われ、後続 cue のみ適用される");
        drop(rt);
        assert_eq!(Rc::strong_count(&runtime), 1, "Close でクリーン終了する");
    }

    // ══ フレーム提示: 未解決 actor の蓄積＋スキップ＋再試行（純粋・COM 不要） ══

    /// 未解決（binding 未登録）actor の cue は状態を蓄積し、present_frame は Ok のまま
    /// 描画をスキップして次フレームで再試行する（actor ごと初回のみ warn!・以降 debug!——
    /// design Error Handling）。
    #[test]
    fn present_frame_accumulates_and_skips_unresolved_actor_with_warn_once() {
        let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
        rt.apply_cue(&cue("0", 0.0, CueCommand::Text("蓄積".into())));
        let mut world = World::new(); // COM 資源なし——未解決 skip は資源に触れない

        let ((), warns, errors) = with_log_cage(|| {
            present_frame(&mut rt, &mut world, 0.0).expect("未解決 actor は skip＝frame は Ok");
            present_frame(&mut rt, &mut world, 0.1).expect("再試行フレームも Ok");
        });
        assert_eq!(warns, 1, "未解決 actor の warn は actor ごと初回のみ（2 フレーム目は debug）");
        assert_eq!(errors, 0, "未解決 skip は error ではない（蓄積＋再試行の正常経路）");

        let actor = ActorKey::from("0");
        assert!(!rt.is_attached(&actor), "未解決のまま装着されない");
        let state = rt.state().actor_state(&actor).expect("状態は蓄積継続（無損失）");
        assert_eq!(state.items().len(), 2, "skip 中も cue 状態は失われない");
    }

    /// binding 登録済みだが COM 資源（GraphicsCore/Compositor）が World に無い場合、
    /// present_frame は panic せず `Device` エラーを返す（log-first・次フレーム再試行可能）。
    #[test]
    fn present_frame_reports_device_error_without_panic_when_resources_missing() {
        let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
        rt.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));

        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        let binding = TextSlotBinding::new(slot, window, 1.0, (60, 40));
        rt.register_actor(
            ActorKey::from("0"),
            binding,
            ResolvedBalloonText::resolve(&geo_model(), (60, 40)),
        );

        let (result, _warns, errors) = with_log_cage(|| present_frame(&mut rt, &mut world, 0.0));
        let err = result.err().expect("COM 資源不在は Err（panic しない）");
        assert!(
            matches!(err, crate::TextLayerError::Device { .. }),
            "資源不在は Device エラー（log-first）: {err:?}"
        );
        assert!(errors >= 1, "失敗は真因文脈付き error ログを伴う");
        // 状態は無傷＝次フレーム再試行可能。
        assert!(rt.state().actor_state(&ActorKey::from("0")).is_some());
        assert!(!rt.is_attached(&ActorKey::from("0")));
    }

    // ══ フレーム提示: 装着＋注入時刻駆動の進行＋Present 完結（COM・headless） ══

    /// 観測可能な完了状態（後半）: 登録後の present_frame が予約スロットへ装着し、
    /// 注入時刻フレームごとに可視グリフ（非透明ピクセル）が進行する。装着は初回のみで、
    /// 以降のグリフ更新は供給面の提示のみ（World の構造変更なし・GraphicsCommandList
    /// 不挿入＝バルーン surface 本体の再合成を要求しない・R9.3）。Clear 後は全透明へ戻る。
    #[test]
    fn present_frame_attaches_once_and_progresses_glyphs_per_injected_frame() {
        // 本番 UI スレッド（MTA）を再現（WucGraphicsResource::new は DQTAT_COM_NONE を使う
        // ため COM 未初期化スレッドでは失敗する——wintf wuc_resource.rs テストと同一方針）。
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
        let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
            .expect("WucGraphicsResource::new 失敗");

        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        world.insert_resource(core);
        world.insert_resource(wuc);

        let actor = ActorKey::from("0");
        let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
        rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));

        // 未登録フレーム: 蓄積＋スキップ（COM 資源があっても binding 未解決なら触れない）。
        present_frame(&mut rt, &mut world, 0.0).expect("未解決 skip フレーム");
        assert!(!rt.is_attached(&actor));

        // 登録→次フレームで再試行が成功し装着される。
        let image = (120u32, 60u32);
        let binding = TextSlotBinding::new(slot, window, 1.0, image);
        rt.register_actor(
            actor.clone(),
            binding,
            ResolvedBalloonText::resolve(&geo_model(), image),
        );
        present_frame(&mut rt, &mut world, 0.0).expect("装着フレーム");
        assert!(rt.is_attached(&actor), "登録後の再試行で装着される");

        // 装着の構造契約: 有効な VisualGraphics・GraphicsCommandList 不挿入（R9.3 構造）。
        let vg = world
            .get::<VisualGraphics>(slot)
            .expect("slot に VisualGraphics（emo 自前 brush）が装着される");
        assert!(vg.is_valid());
        assert!(
            world.get::<GraphicsCommandList>(slot).is_none(),
            "GraphicsCommandList は挿入しない（wintf 描画系と競合しない）"
        );

        // 注入時刻フレームごとの進行（既定 char_wait=0.05・r=[0.0, 0.05, 0.10]）。
        let read = |rt: &TextLayerRuntime| -> usize {
            opaque_count(
                &rt.surface(&actor)
                    .expect("装着済み actor の供給面")
                    .read_back()
                    .expect("read_back"),
            )
        };
        let n1 = read(&rt);
        assert!(n1 > 0, "t=0.0 で 1 グリフ目が可視（非透明ピクセルあり）");

        let entities_after_attach = world.entities().len();
        present_frame(&mut rt, &mut world, 0.06).expect("フレーム t=0.06");
        let n2 = read(&rt);
        assert!(n2 > n1, "t=0.06 で 2 グリフ目まで可視（ピクセル単調増加）: {n1} -> {n2}");

        present_frame(&mut rt, &mut world, 0.11).expect("フレーム t=0.11");
        let n3 = read(&rt);
        assert!(n3 > n2, "t=0.11 で 3 グリフ目まで可視: {n2} -> {n3}");

        // 装着済み actor の更新は Present のみで完結——World の構造は変わらない（R9.3）。
        assert_eq!(
            world.entities().len(),
            entities_after_attach,
            "グリフ更新フレームは World に entity を追加しない（供給面の提示のみ）"
        );
        assert!(
            world.get::<GraphicsCommandList>(slot).is_none(),
            "更新後も GraphicsCommandList は不挿入のまま"
        );

        // Clear cue → 次フレームで全透明へ戻る（clear_cache 経路込み）。
        rt.apply_cue(&cue("0", 0.15, CueCommand::Clear));
        present_frame(&mut rt, &mut world, 0.2).expect("Clear 後フレーム");
        assert_eq!(read(&rt), 0, "Clear 後の供給面は全域透明へ戻る");
    }

    /// 観測可能な完了状態（task 9・R3.5/R10.3）: 登録済み actor に対して present_frame 後の
    /// 決定論観測統計を `TextLayerRuntime::draw_stats(actor)`（`surface(actor)` と同型の
    /// additive アクセサ）から外部が読み出せる。未装着 actor は `None`。
    #[test]
    fn draw_stats_readable_after_present_frame() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
        let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
            .expect("WucGraphicsResource::new 失敗");

        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        world.insert_resource(core);
        world.insert_resource(wuc);

        let actor = ActorKey::from("0");
        let mut rt = TextLayerRuntime::new(TextLayerConfig::default());

        // 未装着 actor は None（読み口の存在自体は装着に依存しない）。
        assert!(
            rt.draw_stats(&actor).is_none(),
            "未装着 actor の draw_stats は None"
        );

        rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
        let image = (120u32, 60u32);
        let binding = TextSlotBinding::new(slot, window, 1.0, image);
        rt.register_actor(
            actor.clone(),
            binding,
            ResolvedBalloonText::resolve(&geo_model(), image),
        );

        // 装着＋初回描画フレーム（全域ダーティ）→ 統計が読み出せて描画が計上されている。
        present_frame(&mut rt, &mut world, 0.0).expect("装着フレーム");
        let stats = rt
            .draw_stats(&actor)
            .expect("装着済み actor の draw_stats は Some");
        assert!(
            stats.draw_text_layout_calls >= 1,
            "初回フレームで DrawTextLayout が計上される: {stats:?}"
        );
        assert!(
            stats.line_layout_creations >= 1,
            "初回フレームで行 TextLayout 生成が計上される: {stats:?}"
        );

        // 後続フレームで観測値が単調増加する（リビール進行＝現在行の再描画）。
        present_frame(&mut rt, &mut world, 0.06).expect("フレーム t=0.06");
        let stats2 = rt.draw_stats(&actor).expect("draw_stats（t=0.06）");
        assert!(
            stats2.draw_text_layout_calls >= stats.draw_text_layout_calls,
            "描画呼び出し回数は単調非減少: {} -> {}",
            stats.draw_text_layout_calls,
            stats2.draw_text_layout_calls
        );
    }
}
