//! actor の子: 1 コマの描画の流れ（未解決の見送り・初回の装着・配置→装飾→描画→提示）。
//! 足す予定の spec: text-reveal-fade（`present_actor`）・balloon-markers・anchor-tag-canon。

use bevy_ecs::prelude::World;
use tracing::{debug, error, info, warn};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use crate::TextLayerError;
use crate::canvas::ContentCanvas;
use crate::choice::{
    annotate_lines, decorate_canvas, derive_hit_rows, line_bands, to_window_physical,
};
use crate::layout::{LayoutEngine, WrapPlan};
use crate::place::PlaceKey;
use crate::region::{ImagePx, ScaleContract};
use crate::segment::segment_plan;
use crate::surface::TextSurface;
use crate::wrap::WrapMode;

use super::decoration;
use super::{ActorRender, ChoiceHitRow, TextLayerRuntime};

/// フレーム提示ステップ（毎フレーム UI スレッドで呼ぶ・example/emo2-boot が駆動）:
/// `talk_time` は注入時刻（talk 起点相対秒・実時間 sleep 不使用・R3.3）。
///
/// actor ごとに「リビール進行の解決（純粋）→ レイアウト決定（純粋）→ viewbox ダーティ矩形
/// スクロール描画（COM）→ 変化ありのフレームだけ供給面を提示（Present のみ）」を駆動する:
///
/// - **未解決 actor**（binding 未登録）: 状態は蓄積のみ・描画スキップ・次フレーム再試行
///   （actor ごと初回 `warn!`＋以降 `debug!`——frame の `Err` にはしない）。
/// - **初回解決フレーム**: World 資源（[`GraphicsCore`]／[`WucGraphicsResource`]）から
///   供給面/描画実行部を構築し予約スロットへ装着する（`ActorRender` 不在時のみ・`info!`）。
///   通常は actor ごと初回の 1 回だが、k 再追従（[`TextLayerRuntime::refresh_actor_scale`]）で
///   `ActorRender` を破棄した直後のフレームでも再発火する（新 k の物理寸で再生成・R8.2）。
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
    //
    // 「状態を持つ」の判定は **中身か既に作った供給面のどちらかがある**こと（task 7.2）。
    // 装着（[`TextLayerRuntime::register_actor`]）が 2 層を差し込む時点でスコープの器は
    // 生まれる——器だけを提示対象に数えると、一度も発話していないスコープにまで供給面を
    // 割り当ててしまう。逆に、既に供給面を持つスコープは中身が空でも外せない（`Clear`／
    // `ClearAll` の後の 1 フレームで実際に画面を消すのがこの走査だから）。
    //
    // 走査するのは普通のバルーンの場所だけ（箱の場所の提示は箱の面の登録が担う）。
    let places: Vec<PlaceKey> = runtime
        .state
        .actors()
        .map(|(key, state)| (PlaceKey::balloon(key), state))
        .filter(|(place, state)| !state.items().is_empty() || runtime.surfaces.contains_key(place))
        .map(|(place, _)| place)
        .collect();
    let mut first_err: Option<TextLayerError> = None;
    for place in &places {
        let actor = &place.actor;
        match present_actor(runtime, world, place, talk_time) {
            Ok(()) => {}
            // 未解決 actor: 蓄積継続・描画スキップ・次フレーム再試行（正常経路・Err にしない）。
            Err(TextLayerError::SlotNotAttached { .. }) => {
                if runtime.unresolved_warned.insert(place.clone()) {
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

/// 1 場所分のフレーム提示（[`present_frame`] の内訳・log-first）。
///
/// binding 未解決は [`TextLayerError::SlotNotAttached`]（呼び手が skip 写像）。
fn present_actor(
    runtime: &mut TextLayerRuntime,
    world: &mut World,
    place: &PlaceKey,
    talk_time: f64,
) -> Result<(), TextLayerError> {
    let actor = &place.actor;
    // ── 解決確認: binding＋layout 入力が揃うまでは蓄積のみ（描画スキップ） ──
    let (Some(binding), Some(resolved)) = (
        runtime.routing.get(place).copied(),
        runtime.layout_input.get(place).cloned(),
    ) else {
        return Err(TextLayerError::SlotNotAttached {
            actor: actor.to_string(),
        });
    };

    let contract = ScaleContract::new(binding.scale, None);

    // ── 初回解決フレーム: World 資源から描画資源を構築し予約スロットへ装着（初回のみ） ──
    if !runtime.surfaces.contains_key(place) {
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
            |world,
             core: bevy_ecs::world::Mut<GraphicsCore>|
             -> Result<ActorRender, TextLayerError> {
                let surface = TextSurface::attach(
                    world,
                    &binding,
                    &compositor,
                    &core,
                    physical_size,
                    physical_offset,
                )?;
                decoration::build_actor_render(
                    &core,
                    surface,
                    &resolved.font,
                    resolved.mode,
                    &config,
                    actor,
                )
            },
        )?;
        runtime.surfaces.insert(place.clone(), render);
        info!(
            actor = %actor,
            slot = ?binding.slot,
            ?physical_size,
            wrap = ?resolved.wrap,
            "テキスト供給面を予約スロットへ装着した（ActorRender 不在時のみ・以降は Present のみ）"
        );
    }

    // ── リビール進行（純粋）→ レイアウト決定（純粋）→ viewbox ダーティ矩形描画 → 提示（Present のみ） ──
    let Some(render) = runtime.surfaces.get_mut(place) else {
        // 直前の insert 直後に到達するため構造上起こらない——防御（panic 禁止・log-first）。
        error!(actor = %actor, "present_frame: 装着済み描画資源の引き当てに失敗（構造不変の破れ）");
        return Err(TextLayerError::Device {
            hresult: 0,
            context: "ActorRender missing after attach",
        });
    };
    let Some(actor_state) = runtime.state.place_state(place) else {
        // present_frame は state 由来の場所だけを渡す——防御的に空フレーム扱い。
        return Ok(());
    };
    let visible = actor_state.reveal().visible(talk_time);
    // 折返し計画: ON（BudouxWordWrap）のときだけ分かち書き境界を全 items から計算し
    // layout へ供給する。OFF（CharByChar）は plan を計算すらしない（R4.2 の構造保証——
    // segment_plan を呼ぶのは Segmented アームだけ）。`plan` は ON アームでのみ束縛され、
    // 借用 `&plan` が layout 呼出まで生存するよう遅延初期化パターンで宣言する。
    let plan;
    let wrap = match resolved.wrap {
        WrapMode::CharByChar => WrapPlan::CharByChar,
        WrapMode::BudouxWordWrap => {
            plan = segment_plan(actor_state.items());
            WrapPlan::Segmented(&plan)
        }
    };
    // `\_l` の座標解決の縮退（`CursorDegrade`・5.1〜5.3）の warn-once を production で有効化する持続 guard を渡す
    // （純挙動は `layout` と完全同一——差は縮退ログの有無のみ・task 4.2 が本配線へ委譲）。
    // 装飾入りの配置の入口（要件 14.1／14.2・番号列が既定だけなら従来と同一の出力）。
    let lines = LayoutEngine::layout_styled(
        actor_state.items(),
        visible,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &render.metrics,
        wrap,
        decoration::glyph_styles_of(actor_state, &resolved),
        actor,
        &mut runtime.cursor_warn,
    );
    let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);

    // ── 選択肢パイプライン: 同一 lines を単一の源に 注釈→装飾→描画（表示とヒットの単一導出・R3.3/5.2） ──
    // 注釈は layout 直後の同一 lines を消費する（可視窓調整後の行へ再適用しない——design Precondition）。
    let spans = actor_state.choices();
    let segments = annotate_lines(&lines, spans);
    // ハイライト帯／ヒット帯のブロック軸寸と寄せ量を**行ごとに 1 度だけ**決め（**単一の源**・
    // R3.3/R13.1/R13.2）、装飾（描画帯）とヒット導出（照会帯）の両方へ同じ列を配る。丈は実 font
    // metrics の行ボックス丈（descent 込み）を行送りピッチで頭打ちにした値——em ボックス丈で切ると
    // 和文フォントの descent インクが帯の外へ出る（実機不具合「選択肢の文字の下が切れる」の真因）。
    // 基準の em は**その行に置かれた文字のうち最も大きい em**（行矩形のブロック軸寸・要件 7.9）で、
    // アクターに 1 つの既定の大きさではない——既定固定だと `[height,40]` の選択肢が表示 42 画素でも
    // 帯 14 画素になり、文字の上下がクリックできない（要件 11.5）。装飾の無い行では行矩形のブロック軸寸が
    // 既定の大きさに等しいので、従来と 1 画素も変わらない。
    let bands = line_bands(&lines, resolved.mode, &render.metrics);
    // hover 印は per-actor 保持値（未注入＝None＝ハイライト無し・8.1）。
    let hover = runtime.choice_hover.get(place).copied().flatten();
    // 装飾: hover 行へ塗り/文字色を焼く。セグメント空（選択肢無し）は decorate が恒等＝canvas 無変更（非退行）。
    let canvas = ContentCanvas::from_layout(&lines, &resolved.region, resolved.mode);
    let canvas = decorate_canvas(
        canvas,
        &segments,
        hover,
        resolved.choice_style,
        resolved.font.color,
        &resolved.region,
        resolved.mode,
        &bands,
    );
    // 装飾入りの描画の入口（要件 14.2・既定だけの行は従来と同一の呼出列）。
    let changed = render.executor.render_styled(
        &canvas,
        &window,
        &resolved.font,
        resolved.mode,
        &contract,
        &mut render.surface,
        actor_state.styles(),
    )?;
    // 装着済み actor のグリフ更新は供給面の提示のみで完結（emo-compose 再駆動なし・R9.3）。
    // 変化ありのフレームだけ提示する（`FramePlan::NoChange` は blit も描画も present も省く——
    // readback は front を読むため観測述語に影響しない・R1.1/R3.1）。
    if changed {
        // ── ヒット行スナップショット更新（present 成功時のみ・表示と同一 lines/segments の単一導出・5.2） ──
        // committed は既存 visible_window→executor が確定した面反映済みスクロールをそのまま消費する
        // （新規のスクロール可視判定は追加しない・6.3）。NoChange フレームはこの更新を丸ごと省き
        // 直前スナップショットを不変のまま保つ。
        let committed = render.executor.scroll_state().committed;
        // 帯は装飾（描画）へ渡したのと**同一の行ごとの列**——描画とヒットの
        // 座標整合（R3.3/R13.2）。
        let hit_rows = derive_hit_rows(&lines, &segments, resolved.mode, &resolved.region, &bands);
        // 各ヒット行を配送順序数で対応スパンへ突き合わせ、窓物理 px 矩形＋下流構成材料を同梱する。
        let snapshot: Vec<ChoiceHitRow> = hit_rows
            .iter()
            .filter_map(|row| {
                spans
                    .iter()
                    .find(|span| span.ordinal == row.ordinal)
                    .map(|span| ChoiceHitRow {
                        ordinal: row.ordinal,
                        id: span.id.clone(),
                        label: span.label.clone(),
                        references: span.references.clone(),
                        rect: to_window_physical(
                            row,
                            &resolved.region,
                            resolved.mode,
                            committed,
                            &contract,
                        ),
                    })
            })
            .collect();
        runtime.choice_snapshot.insert(place.clone(), snapshot);
        render.surface.present()?;
    }
    Ok(())
}
