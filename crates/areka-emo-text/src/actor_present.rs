//! actor の子: 1 コマの描画の流れ（未解決の見送り・描画範囲の潰れた場所の見送り・初回の装着・配置→装飾→描画→提示）。
//! 足す予定の spec: text-reveal-fade（`present_actor`）・balloon-markers。
//!
//! 断りの無い要件の番号と design.md の参照は spec `areka-P0-text-decoration-canon` のもの（別の spec のものはその場で名を書く）。

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::World;
use tracing::{debug, error, info, warn};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use crate::TextLayerError;
use crate::canvas::ContentCanvas;
use crate::choice::{
    CanvasHitRow, LineBand, LineChoiceSegment, annotate_lines, decorate_canvas, derive_hit_rows,
    glyph_cells, line_bands, to_window_physical,
};
use crate::cursor_tag::CursorWarnGuard;
use crate::layout::{GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::lookahead::{Basis, TalkLookahead};
use crate::place::{PlaceKey, TextPlace};
use crate::region::{ImagePx, ScaleContract, TextRegion};
use crate::scroll_marker::{ScrollChange, marker_visibility, window_for};
use crate::segment::{SegmentPlan, segment_plan};
use crate::state::{ChoiceSpan, TextItem, TextLayerState};
use crate::surface::TextSurface;
use crate::wrap::WrapMode;
use crate::writing::WritingMode;

use super::decoration;
use super::scroll;
use super::{
    ActorRender, ChoiceHitRow, MarkerSurface, PlacedMarker, ResolvedBalloonText, TextLayerRuntime,
    is_settled_ordinal,
};

/// 文字の面の物理寸の一辺の上限（Direct3D 11 の機能水準 11 の 2D テクスチャの一辺の最大）。
pub(crate) const MAX_SURFACE_SIDE: u32 = 16_384;
/// 文字の面の物理寸の面積の上限（8K の画面より大きい・`audit-emo-compose` の `MAX_AREA` と同じ）。
pub(crate) const MAX_SURFACE_AREA: u64 = 1 << 25;
/// 上限を越える面を作らなかったときの記録の文。
pub(crate) const SURFACE_LIMIT_MESSAGE: &str =
    "文字の面の物理寸が上限を越える——面を作らず、寸と位置が変わるまで作り直さない";

/// 上限を越える印の面を作らなかったときの記録の文。
pub(crate) const MARKER_LIMIT_MESSAGE: &str =
    "印の面の物理寸が上限を越える——面を作らず、寸が変わるまで作り直さない";

/// 物理寸が文字の面の上限（一辺・面積）を越えるか。
pub(crate) fn over_surface_limit((w, h): (u32, u32)) -> bool {
    w > MAX_SURFACE_SIDE || h > MAX_SURFACE_SIDE || u64::from(w) * u64::from(h) > MAX_SURFACE_AREA
}

/// フレーム提示ステップ（毎フレーム UI スレッドで呼ぶ・example/emo2-boot が駆動）:
/// `talk_time` は注入時刻（talk 起点相対秒・実時間 sleep 不使用・`areka-P0-emo-text-layer` R3.3）。
///
/// actor ごとに「リビール進行の解決（純粋）→ レイアウト決定（純粋）→ viewbox ダーティ矩形
/// スクロール描画（COM）→ 変化ありのフレームだけ供給面を提示（Present のみ）」を駆動する:
///
/// - **未解決 actor**（binding 未登録）: 状態は蓄積のみ・描画スキップ・次フレーム再試行
///   （actor ごと初回 `warn!`＋以降 `debug!`——frame の `Err` にはしない）。
/// - **初回解決フレーム**: World 資源（[`GraphicsCore`]／[`WucGraphicsResource`]）から
///   供給面/描画実行部を構築し予約スロットへ装着する（`ActorRender` 不在時のみ・`info!`）。
///   通常は actor ごと初回の 1 回だが、k 再追従（[`TextLayerRuntime::refresh_actor_scale`]）で
///   `ActorRender` を破棄した直後のフレームでも再発火する（新 k の物理寸で再生成・`areka-P0-emo-dpi-scaling` R8.2）。
/// - **装着済み actor のグリフ更新**: viewbox ダーティ矩形スクロール描画→（変化ありのフレームだけ）
///   swapchain Present で完結し、バルーン surface 本体の再合成（emo-compose 再駆動）を要求しない（`areka-P0-emo-text-layer` R9.3）。
/// - **描画範囲の潰れた場所**（areka-P0-balloon-text-bugs 件 2）: 装着せず描かず記録もせず、残った描画資源は外して捨てる。
///   「提示した場所」に数えない（箱の写しに載せない）。ほかの場所の提示は続ける。
/// - **デバイス失敗**: 失敗源で `error!` 済み（log-first）。当該 actor の当該フレーム提示を
///   skip して他 actor の処理は継続し、最初の失敗を `Err` として返す（次フレーム再試行）。
pub fn present_frame(
    runtime: &mut TextLayerRuntime,
    world: &mut World,
    talk_time: f64,
) -> Result<(), TextLayerError> {
    // 再追従が捨てた描き手を差し込み口から外す（World はここで初めて持つ）。
    for (render, slot) in std::mem::take(&mut runtime.retired_renders) {
        render.discard(world, Some(slot));
    }
    drop_renders_after_device_change(runtime, world);
    // 状態を持つ actor だけが提示対象（binding 登録済みでも cue が無ければ描くものがない）。
    //
    // 「状態を持つ」の判定は **中身か既に作った供給面のどちらかがある**こと。
    // 装着（[`TextLayerRuntime::register_actor`]）が 2 層を差し込む時点でスコープの器は
    // 生まれる——器だけを提示対象に数えると、一度も発話していないスコープにまで供給面を
    // 割り当ててしまう。逆に、既に供給面を持つスコープは中身が空でも外せない（`Clear`／
    // `ClearAll` の後の 1 フレームで実際に画面を消すのがこの走査だから）。
    //
    // 箱の場所は、毎フレームの箱の同期（`sync_boxes`）が登録した場所だけを走査する（文字を持ち、
    // 今のサーフェスに置かれ、隠されていない箱。登録を外すと面も片付くので、面だけが残る箱は無い）。
    let mut places: Vec<PlaceKey> = runtime
        .state
        .actors()
        .map(|(key, state)| (PlaceKey::balloon(key), state))
        .filter(|(place, state)| !state.items().is_empty() || runtime.surfaces.contains_key(place))
        .map(|(place, _)| place)
        .collect();
    let mut boxes: Vec<PlaceKey> = runtime.box_sites.keys().cloned().collect();
    boxes.sort();
    places.extend(boxes);
    let mut first_err: Option<TextLayerError> = None;
    let mut presented: Vec<PlaceKey> = Vec::new();
    for place in &places {
        let actor = &place.actor;
        // 印の面の失敗はその場所の矢印だけを出さない（文字・箱の写し・当たりの写しは成功と同じ）。
        let mut marker_err = None;
        let result = present_actor(runtime, world, place, talk_time, &mut marker_err);
        if let Some(e) = marker_err {
            first_err.get_or_insert(e);
        }
        match result {
            Ok(true) => presented.push(place.clone()),
            // 描画範囲の潰れた場所: 描かなかったので「提示した場所」に数えない（箱の写しに載せない）。
            Ok(false) => {}
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
    // 文字の出ている箱の四角の写しを、このフレームの提示の結果で作り直す。
    runtime.refresh_shown_boxes(&presented, talk_time);
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// `GraphicsCore` の変化の印（作り直しと本当の失いのときだけ立つ）を見たら、全部の描き手と印の面を
/// 差し込み口・窓の子から外して捨てる（audit-wintf-graphics-world 6 番・要件 3.1）。古い装置の
/// swap chain・文脈を使い続けない。作り直しは同じ回の作る門が、装置が有効なときだけ行う。
/// 初めて呼んだ回は印を控えるだけ（描き手はまだ無い）。
fn drop_renders_after_device_change(runtime: &mut TextLayerRuntime, world: &mut World) {
    let Some(changed) = world
        .get_resource_change_ticks::<GraphicsCore>()
        .map(|t| t.changed)
    else {
        return;
    };
    let now = world.read_change_tick();
    let seen = runtime.seen_graphics;
    if seen.is_some_and(|seen| !changed.is_newer_than(seen, now)) {
        return;
    }
    // 時刻を進めて控える: 同じ時刻のうちに後から立った印も、次の回に新しいと分かる。
    runtime.seen_graphics = Some(world.increment_change_tick());
    if seen.is_none() {
        return;
    }
    // 新しい装置では作れるかもしれない（作れなかった面の覚えを消す）。
    runtime.surface_failed.clear();
    let places: Vec<PlaceKey> = runtime.surfaces.keys().cloned().collect();
    debug!(
        dropped = places.len(),
        "present_frame: 装置の変化を見た（描き手と印の面を捨てる）"
    );
    for place in places {
        let Some(render) = runtime.surfaces.remove(&place) else {
            continue;
        };
        let slot = runtime.routing.get(&place).map(|binding| binding.slot);
        if slot.is_none() {
            // 結線の無い場所は差し込み口が分からない: 窓の子（箱の面）だけは外して残さない。
            debug!(actor = %place.actor, "装置の変化: 結線の無い場所の描き手を捨てた");
        }
        render.discard(world, slot);
    }
}

/// `GraphicsCore` を一時取り外して `f` へ貸す。`resource_scope` は戻すときに資源の変化の印を
/// 立てる（bevy 0.19）ので、印を前の値へ戻す——描き手を作るだけで装置の作り直しの合図に
/// 見せない（立てたままだと、ここと emo-present が毎回捨てて作り直す輪になる）。
pub(super) fn with_graphics_core<T>(
    world: &mut World,
    f: impl FnOnce(&mut World, &GraphicsCore) -> T,
) -> T {
    let before = world
        .get_resource_change_ticks::<GraphicsCore>()
        .map(|t| t.changed);
    let out =
        world.resource_scope(|world, core: bevy_ecs::world::Mut<GraphicsCore>| f(world, &core));
    if let (Some(before), Some(mut core)) = (before, world.get_resource_mut::<GraphicsCore>()) {
        core.set_last_changed(before);
    }
    out
}

/// 1 場所分のフレーム提示（[`present_frame`] の内訳・log-first）。
///
/// binding 未解決は [`TextLayerError::SlotNotAttached`]（呼び手が skip 写像）。
/// 描画範囲の潰れた場所は描かずに `Ok(false)`、それ以外の成功は `Ok(true)`（提示した場所）。
/// 印の面の失敗は `marker_err` へ置き、文字の提示の答えは変えない（その場所の矢印だけが出ない）。
fn present_actor(
    runtime: &mut TextLayerRuntime,
    world: &mut World,
    place: &PlaceKey,
    talk_time: f64,
    marker_err: &mut Option<TextLayerError>,
) -> Result<bool, TextLayerError> {
    let actor = &place.actor;
    // ── 解決確認: binding＋layout 入力が揃うまでは蓄積のみ（描画スキップ） ──
    let (Some(binding), Some(resolved)) = (
        runtime.routing.get(place).copied(),
        runtime.layout_input.get(place),
    ) else {
        return Err(TextLayerError::SlotNotAttached {
            actor: actor.to_string(),
        });
    };

    // 文字の面は画像（箱は箱とシェルの画像の重なり）の右と下で切る（外は窓に映らない）。左と上は
    // 面の原点なので切らない（近い辺は上限が止める）。切って空なら潰れた場所と同じく描かない（audit-emo-text-state-actor 指摘 3・要件 2.1）。
    let region = &resolved.region;
    let lim = runtime.image_limit(place, &binding);
    let clip = (region.right().min(lim.0), region.bottom().min(lim.1));

    // ── 描画範囲の潰れた場所: 装着せず描かず記録もしない（areka-P0-balloon-text-bugs 件 2・要件 2.1・2.2・2.7） ──
    // 描いた後に潰れた（再追従が描画資源を残した）なら、ここで差し込み口から外して前の字を残さない
    // （`areka-P0-balloon-text-bugs` 要件 2.9）。当たり行の写しも外す。記録は登録口が状態の変わり目に 1 回だけ出す。
    if region.is_collapsed() || clip.0 <= region.left() || clip.1 <= region.top() {
        if let Some(render) = runtime.surfaces.remove(place) {
            render.discard(world, Some(binding.slot));
        }
        runtime.choice_snapshot.remove(place);
        return Ok(false);
    }

    let contract = ScaleContract::new(binding.scale, None);
    // 場所の左上の窓の中の位置（image px）: 箱は置き場所の (X,Y)、普通のバルーンは (0,0)。
    let origin = runtime.box_origin(place);

    // ── 初回解決フレーム: World 資源から描画資源を構築し予約スロットへ装着（初回のみ） ──
    if !runtime.surfaces.contains_key(place) {
        // 描き手が無い＝前の字は映っていない。面を作れずに戻っても当たり行が残らないよう外す
        // （装着できればこの回は全部を描き直し、写しも作り直す）。
        runtime.choice_snapshot.remove(place);
        // 物理寸＝ceil(切った validrect 寸 × k)・offset＝validrect 原点 × k（DPI/スケール契約）。
        let physical_size = (
            contract.physical_extent(ImagePx(clip.0 - region.left())),
            contract.physical_extent(ImagePx(clip.1 - region.top())),
        );
        // 箱の面は `(箱の X + 領域の左, 箱の Y + 領域の上) × k`（シェルの窓の中の位置）。
        let physical_offset = (
            contract.to_physical(ImagePx(origin.0 + region.left())).0,
            contract.to_physical(ImagePx(origin.1 + region.top())).0,
        );
        // 入力で答えの決まる失敗（上限越え・その寸での装着の失敗）は、寸・位置・WUC の資源・結線
        // （差し込み口と窓）が変わるまで試さない（記録は 1 回）。資源が無いだけの失敗は覚えず毎フレーム試し直す。
        let wuc = world.get_resource_change_ticks::<WucGraphicsResource>();
        let wired = (binding.slot, binding.window);
        let attempt = (
            physical_size,
            physical_offset,
            wuc.map(|t| t.changed),
            wired,
        );
        if runtime.surface_failed.get(place) == Some(&attempt) {
            return Ok(false);
        }
        if over_surface_limit(physical_size) {
            error!(
                actor = %actor,
                place = ?place.place,
                ?physical_size,
                ?physical_offset,
                max_side = MAX_SURFACE_SIDE,
                max_area = MAX_SURFACE_AREA,
                "{SURFACE_LIMIT_MESSAGE}"
            );
            runtime.surface_failed.insert(place.clone(), attempt);
            return Ok(false);
        }
        // 箱の面はシェルの窓の直接の子として、差し込み口の直後から element番号の大きい順に挿す。
        let child_index = match &place.place {
            TextPlace::Balloon => None,
            TextPlace::Box(_) => {
                let children: Vec<bevy_ecs::entity::Entity> = world
                    .get::<Children>(binding.window)
                    .map(|c| c.iter().copied().collect())
                    .unwrap_or_default();
                let Some(index) = runtime.box_child_index(&children, place) else {
                    // 差し込み口がシェルの窓の子に無い: 未解決として次フレームで再試行する。
                    return Err(TextLayerError::SlotNotAttached {
                        actor: actor.to_string(),
                    });
                };
                Some(index)
            }
        };

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
        if !world
            .get_resource::<GraphicsCore>()
            .is_some_and(GraphicsCore::is_valid)
        {
            error!(
                actor = %actor,
                "present_frame: GraphicsCore 不在か無効（供給面を生成できない）"
            );
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "GraphicsCore resource",
            });
        }
        // GraphicsCore は一時取り外し——attach の &mut World と資源借用の衝突を構造回避する。
        let config = runtime.config;
        let mut attach_failed = false;
        let render = with_graphics_core(
            world,
            |world, core| -> Result<ActorRender, TextLayerError> {
                let attach = || {
                    match child_index {
                        None => TextSurface::attach(
                            world,
                            &binding,
                            &compositor,
                            core,
                            physical_size,
                            physical_offset,
                        ),
                        Some(index) => TextSurface::attach_window_child(
                            world,
                            binding.window,
                            index,
                            &compositor,
                            core,
                            physical_size,
                            physical_offset,
                        ),
                    }
                    .inspect_err(|_| attach_failed = true)
                };
                decoration::build_actor_render(
                    core,
                    attach,
                    &resolved.font,
                    resolved.mode,
                    &config,
                    actor,
                )
            },
        )
        .inspect_err(|_| {
            if attach_failed {
                runtime.surface_failed.insert(place.clone(), attempt);
            }
        })?;
        runtime.surface_failed.remove(place);
        runtime.surfaces.insert(place.clone(), render);
        info!(
            actor = %actor,
            place = ?place.place,
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
        return Ok(true);
    };
    let visible = actor_state.reveal().visible(talk_time);
    let spans = actor_state.choices();
    // hover 印は per-actor 保持値（未注入＝None＝ハイライト無し）。済んだ選択肢を指す強調は
    // 装飾へ渡さない（注釈は全部の範囲のまま＝字・色・印は変わらない・areka-P0-balloon-text-bugs 件 1 の要件 1.10／1.12）。
    let hover = runtime
        .choice_hover
        .get(place)
        .copied()
        .flatten()
        .filter(|&ordinal| !is_settled_ordinal(actor_state, ordinal));
    let key = FrameKey {
        visible,
        items: actor_state.items().len(),
        choices: spans.len(),
        settled: actor_state.settled(),
        clears: runtime.state.clears_at(place),
        hover,
        lookahead_rev: runtime.lookahead_rev,
    };
    // 鍵が前のフレームと同じなら、覚えた行・区切り・帯・キャンバスで描き手を回す（audit-emo-text-state-actor 指摘 8）。
    let mut memo = match render.frame_memo.take() {
        Some(memo) if memo.matches(&key, &resolved) => memo,
        _ => {
            // 「見える数 → 区切り → 配置」は [`arrange_lines`] の 1 回の呼び出し（検査も同じ関数を呼ぶ）。
            let Some(lines) = arrange_lines(
                &runtime.state,
                &mut runtime.lookahead,
                &mut runtime.cursor_warn,
                place,
                &resolved,
                &render.metrics,
                &mut render.break_memo,
                talk_time,
            ) else {
                // 直前で状態を引けている——構造上起こらない（防御的に空フレーム扱い）。
                return Ok(true);
            };
            // ── 選択肢パイプライン: 同一 lines を単一の源に 注釈→装飾→描画（表示とヒットの単一導出・`areka-P0-choice-render` R3.3/5.2） ──
            // 注釈は layout 直後の同一 lines を消費する（可視窓調整後の行へ再適用しない——`areka-P0-choice-render` の design.md の Precondition）。
            // 範囲と帯は choice_ranges が 1 度だけ出す（当たりは描き替えたときだけ同じ値から出す）。
            // ハイライト帯／ヒット帯のブロック軸寸と寄せ量を**行ごとに 1 度だけ**決め（**単一の源**・
            // `areka-P0-choice-render` R3.3・`areka-P0-emo2-conformance-e2e` R13.1/R13.2）、装飾（描画帯）とヒット導出（照会帯）の両方へ同じ列を配る。丈は実 font
            // metrics の行ボックス丈（descent 込み）を行送りピッチで頭打ちにした値——em ボックス丈で切ると
            // 和文フォントの descent インクが帯の外へ出る（実機不具合「選択肢の文字の下が切れる」の真因）。
            // 基準の em は**その行に置かれた文字のうち最も大きい em**（行矩形のブロック軸寸・要件 7.9）で、
            // アクターに 1 つの既定の大きさではない——既定固定だと `[height,40]` の選択肢が表示 42 画素でも
            // 帯 14 画素になり、文字の上下がクリックできない（要件 11.5）。装飾の無い行では行矩形のブロック軸寸が
            // 既定の大きさに等しいので、従来と 1 画素も変わらない。
            let ranges = choice_ranges(
                &lines,
                spans,
                resolved.mode,
                &resolved.region,
                &render.metrics,
            );
            // 装飾: hover 行へ塗り/文字色を焼く。セグメント空（選択肢無し）は decorate が恒等＝canvas 無変更（非退行）。
            let canvas = ContentCanvas::from_layout(&lines, &resolved.region, resolved.mode);
            let canvas = decorate_canvas(
                canvas,
                &ranges.segments,
                hover,
                resolved.choice_style,
                resolved.font.color,
                &resolved.region,
                resolved.mode,
                &ranges.bands,
            );
            let ChoiceRanges {
                segments, bands, ..
            } = ranges;
            #[cfg(test)]
            {
                render.frame_rebuilds += 1;
            }
            FrameMemo {
                key,
                input: resolved.clone(),
                lines,
                segments,
                bands,
                canvas,
            }
        }
    };
    // 見える字の数の観測を見える範囲の決めより**先に**呼ぶ（字が増えたフレームを自動の位置で
    // 描く・後に呼ぶと 1 フレーム遅れる・`balloon-scroll-arrows` 要件 8.1）。手動の位置が在ればその行から見せる（`balloon-scroll-arrows` 要件 8.5）。
    let scroll_state = runtime.scrolls.entry(place.clone()).or_default();
    if let Some(reason) = scroll_state.observe_visible(visible) {
        scroll::record(place, ScrollChange::Stopped(reason), "提示");
    }
    let (window, reached) = window_for(&memo.lines, &resolved.region, resolved.mode, scroll_state);
    if let Some(reason) = reached {
        scroll::record(place, ScrollChange::Stopped(reason), "提示");
    }
    let shown = scroll::placed_markers(
        &runtime.balloon_markers,
        &runtime.box_markers,
        place,
        marker_visibility(&window, scroll_state),
        resolved.markers,
    );

    // 装飾入りの描画の入口（要件 14.2・既定だけの行は従来と同一の呼出列）。
    let changed = render.executor.render_styled(
        &memo.canvas,
        &window,
        &resolved.font,
        resolved.mode,
        &contract,
        &mut render.surface,
        actor_state.styles(),
    )?;
    // 装着済み actor のグリフ更新は供給面の提示のみで完結（emo-compose 再駆動なし・`areka-P0-emo-text-layer` R9.3）。
    // 変化ありのフレームだけ提示する（`FramePlan::NoChange` は blit も描画も present も省く——
    // readback は front を読むため観測述語に影響しない・`areka-P0-emo-text-viewbox` R1.1/R3.1）。
    if changed {
        // ── ヒット行スナップショット更新（present 成功時のみ・表示と同一 lines/segments の単一導出・`areka-P0-choice-render` 5.2） ──
        // committed は既存 visible_window→executor が確定した面反映済みスクロールをそのまま消費する
        // （新規のスクロール可視判定は追加しない・同じ spec の 6.3）。NoChange フレームはこの更新を丸ごと省き
        // 直前スナップショットを不変のまま保つ。
        let committed = render.executor.scroll_state().committed;
        // 帯は装飾（描画）へ渡したのと**同一の行ごとの列**——描画とヒットの
        // 座標整合（`areka-P0-choice-render` R3.3・`areka-P0-emo2-conformance-e2e` R13.2）。
        let hit_rows = memo.hit_rows(&resolved.region, resolved.mode);
        // 各ヒット行を配送順序数で対応スパンへ突き合わせ、窓物理 px 矩形＋下流構成材料を同梱する。
        let snapshot: Vec<ChoiceHitRow> = hit_rows
            .iter()
            .filter_map(|row| {
                // 済んだ選択肢の行は写しに入れない＝押しても当たらない（アンカーの行は残る・areka-P0-balloon-text-bugs 件 1 の要件 1.11）。
                spans
                    .iter()
                    .find(|span| span.ordinal == row.ordinal)
                    .filter(|span| !actor_state.is_settled_choice(span))
                    .map(|span| ChoiceHitRow {
                        kind: span.kind,
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
                            origin,
                        ),
                    })
            })
            .collect();
        runtime.choice_snapshot.insert(place.clone(), snapshot);
        // 箱の文字の面は、表示されている字の矩形でポインタを受ける（`areka-P0-shell-balloon` 要件 9.4）。字の矩形は表示と
        // 同じ配置の結果（見えている字だけ）と面反映済みのスクロールから作る。
        if matches!(place.place, TextPlace::Box(_)) {
            let cells = glyph_cells(
                &memo.lines,
                &memo.bands,
                resolved.mode,
                &resolved.region,
                committed,
                &contract,
            );
            render.surface.set_hit_cells(world, cells);
        }
        render.surface.present()?;
    }
    render.frame_memo = Some(memo);
    *marker_err = present_markers(runtime, world, place, &binding, origin, &shown).err();
    Ok(true)
}

/// 提示の最後: 出す矢印・絵・置き場・拡大率が前と違うときだけ印の面を描き直す（当たりの写しも
/// 一緒に作り直す・`balloon-scroll-arrows` 要件 3.8）。印の面は初めて矢印を出すときに、面の画像（箱は箱とシェルの画像の
/// 重なり）全体の物理寸で文字の面の直前に挿す（`balloon-scroll-arrows` 要件 3.5・audit-emo-text-state-actor 指摘 3）。失敗は失敗源で `error!` 済み
/// （その場所の矢印だけが出ない）。上限越えとその寸での作成の失敗は寸を覚え、同じ寸では試さない。
fn present_markers(
    runtime: &mut TextLayerRuntime,
    world: &mut World,
    place: &PlaceKey,
    binding: &super::TextSlotBinding,
    origin: (f32, f32),
    shown: &[PlacedMarker],
) -> Result<(), TextLayerError> {
    let contract = ScaleContract::new(binding.scale, None);
    let Some(render) = runtime.surfaces.get(place) else {
        return Ok(());
    };
    match &render.markers {
        Some(markers) if markers.is_drawn(shown, contract.scale) => return Ok(()),
        // まだ一度も出していない: 面を作らない。
        None if shown.is_empty() => return Ok(()),
        _ => {}
    }
    if render.markers.is_none() {
        let lim = runtime.image_limit(place, binding);
        let physical_size = (
            contract.physical_extent(ImagePx(lim.0)),
            contract.physical_extent(ImagePx(lim.1)),
        );
        // 空の面（箱がシェルの外）は作らない。作れなかった寸は寸か WUC の資源が変わるまで試さない。
        let wuc = world
            .get_resource_change_ticks::<WucGraphicsResource>()
            .map(|t| t.changed);
        let attempt = (physical_size, wuc);
        if physical_size.0 == 0 || physical_size.1 == 0 || render.markers_failed == Some(attempt) {
            return Ok(());
        }
        if over_surface_limit(physical_size) {
            error!(
                actor = %place.actor,
                place = ?place.place,
                ?physical_size,
                max_side = MAX_SURFACE_SIDE,
                max_area = MAX_SURFACE_AREA,
                "{MARKER_LIMIT_MESSAGE}"
            );
            if let Some(render) = runtime.surfaces.get_mut(place) {
                render.markers_failed = Some(attempt);
            }
            return Ok(());
        }
        let children: Vec<bevy_ecs::entity::Entity> = world
            .get::<Children>(binding.window)
            .map(|c| c.iter().copied().collect())
            .unwrap_or_default();
        let Some(index) = runtime.marker_child_index(&children, place) else {
            error!(actor = %place.actor, place = ?place.place, "印の面: 文字の面が窓の子に無い（矢印を出せない）");
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "marker child index",
            });
        };
        let Some(compositor) = world
            .get_resource::<WucGraphicsResource>()
            .and_then(|resource| resource.compositor().cloned())
        else {
            error!(actor = %place.actor, "印の面: WucGraphicsResource/Compositor 不在");
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "WucGraphicsResource::compositor",
            });
        };
        // 装置が無いか無効なのは一時の失敗: 覚えず、次の回に試し直す（文字の面と同じ）。
        if !world
            .get_resource::<GraphicsCore>()
            .is_some_and(GraphicsCore::is_valid)
        {
            error!(actor = %place.actor, "印の面: GraphicsCore 不在か無効（矢印を出せない）");
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "GraphicsCore resource",
            });
        }
        let physical_offset = (
            contract.to_physical(ImagePx(origin.0)).0,
            contract.to_physical(ImagePx(origin.1)).0,
        );
        let mut surface_failed = false;
        let markers = with_graphics_core(world, |world, core| {
            MarkerSurface::attach_noting(
                world,
                (binding.window, index),
                &compositor,
                core,
                physical_size,
                physical_offset,
                &mut surface_failed,
            )
        })
        .inspect_err(|_| {
            if let (true, Some(render)) = (surface_failed, runtime.surfaces.get_mut(place)) {
                render.markers_failed = Some(attempt);
            }
        })?;
        debug!(actor = %place.actor, place = ?place.place, ?physical_size, "印の面を文字の面の直前に挿した");
        if let Some(render) = runtime.surfaces.get_mut(place) {
            render.markers = Some(markers);
        }
    }
    if let Some(markers) = runtime
        .surfaces
        .get_mut(place)
        .and_then(|r| r.markers.as_mut())
    {
        markers.draw(world, shown, &contract)?;
        debug!(
            actor = %place.actor,
            place = ?place.place,
            kinds = ?shown.iter().map(|m| m.kind).collect::<Vec<_>>(),
            "出す矢印が変わったので印の面を描き直した"
        );
    }
    Ok(())
}

/// 場所 1 つの行の割り当てを作る（「見える数 → 区切り → 配置」の 1 つの手順）。場所に状態が無ければ `None`。
///
/// 本番の提示（[`present_actor`]）と検査（`TextLayerRuntime::arrange_for_test`）が同じ関数を呼ぶ。
/// 字幅は引数で受けるので GPU の資源を要らない（検査は決まった字幅を渡す）。
/// 同じ（状態・区間の全文・配置の入力・字幅・時刻）なら同じ行の列を返す（`cursor_warn` は縮退の
/// warn の抑えだけで行の列には効かない）。
///
/// 見える数はいつも届いた内容の表示の時刻から出す（字の表示の時刻を変えない・`areka-P0-budoux-reveal-reflow` 要件 2.4）。文節の
/// 折り返しで区間の全文があり、届いた内容がその先頭と一致するときは、全文の字の列・区切り・装飾で
/// 配置する——見えている字の行は見える数だけで決まり、後から届く字で動かない（`areka-P0-budoux-reveal-reflow` 要件 1.1・1.2）。
///
/// 字は台本のタグとタグの間のひと続きごとに、それぞれの時刻に分かれて届くので、届いた内容は
/// 台詞の途中までのことが多い。区間の全文は、再生の前に知らされた台本の全部を空回しして作る
/// （持ち主は [`TalkLookahead`]）。全文が無い（先に知らされていない）か、届いた内容が全文の先頭と
/// 食い違うときは、届いた字の列で区切って配置する修正前の動きで続ける（記録は持ち主が場所・区間
/// ごとに 1 度だけ出す）。
#[allow(clippy::too_many_arguments)]
pub(super) fn arrange_lines(
    state: &TextLayerState,
    lookahead: &mut TalkLookahead,
    cursor_warn: &mut CursorWarnGuard,
    place: &PlaceKey,
    resolved: &ResolvedBalloonText,
    metrics: &dyn GlyphMetrics,
    breaks: &mut BreakMemo,
    talk_time: f64,
) -> Option<Vec<PositionedLine>> {
    let arrived = state.place_state(place)?;
    let visible = arrived.reveal().visible(talk_time);
    // 配置に使う内容（字の列・装飾）と折返し計画。1 字ずつ（CharByChar）は今の呼び方のまま
    // 区切りを計算せず、区間の全文にも触れない（`areka-P0-budoux-reveal-reflow` R4.2・要件 3.3）。文節（BudouxWordWrap）は
    // 区間の全文の持ち主に聞き、全文なら全文の側の字の列・区切り・装飾（番号・表）で、そうでなければ
    // 届いた字で区切る（修正前の動き・warn は持ち主が出す）。字の無い行の丈は今の見た目でなく後ろの字
    // から引く（`Scan::following_glyph_height`・後ろに字が無いときだけ `font_height`）。
    // 届いた字で区切る腕は、消した回数と項目の数が前と同じなら覚えた区切りを使う（audit-emo-text-state-actor 指摘 8）。
    let (content, wrap) = match resolved.wrap {
        WrapMode::CharByChar => (arrived, WrapPlan::CharByChar),
        WrapMode::BudouxWordWrap => match lookahead.basis(place, arrived) {
            Basis::Full { content, plan } => (content, WrapPlan::Segmented(plan)),
            Basis::Arrived => (
                arrived,
                WrapPlan::Segmented(breaks.plan(state.clears_at(place), arrived.items())),
            ),
        },
    };
    // `\_l` の座標解決の縮退（`CursorDegrade`・`areka-P0-cursor-tag-canon` 要件 5.1〜5.3）の warn-once を production で有効化する持続 guard を渡す
    // （純挙動は `layout` と完全同一——差は縮退ログの有無のみ）。
    // 装飾入りの配置の入口（要件 14.1／14.2・番号列が既定だけなら従来と同一の出力）。
    Some(LayoutEngine::layout_styled(
        content.items(),
        visible,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        metrics,
        wrap,
        decoration::glyph_styles_of(content, resolved),
        &place.actor,
        cursor_warn,
    ))
}

/// 届いた字の区切りの覚え（audit-emo-text-state-actor 指摘 8）: 鍵は（消した回数, 項目の数）。項目は消すまで増えるだけで、
/// 表を替えると箱の `ActorRender` は捨てられるので、この 2 つが同じなら区切りも同じ。
#[derive(Default)]
pub(super) struct BreakMemo {
    last: Option<((u64, usize), SegmentPlan)>,
    /// 区切りを計算し直した回数（テストだけの数）。
    #[cfg(test)]
    pub(super) runs: u32,
}

impl BreakMemo {
    /// 鍵が前と同じなら覚えた区切り、違えば区切り直して覚えた区切りを返す。
    fn plan(&mut self, clears: u64, items: &[TextItem]) -> &SegmentPlan {
        let key = (clears, items.len());
        let kept = self.last.take().filter(|(k, _)| *k == key);
        #[cfg(test)]
        if kept.is_none() {
            self.runs += 1;
        }
        &self
            .last
            .insert(kept.unwrap_or_else(|| (key, segment_plan(items))))
            .1
    }
}

/// 配置の覚えの鍵のうち数で比べる欄（配置の入力は [`FrameMemo`] が値で持って比べる）。
#[derive(Clone, Copy, PartialEq)]
struct FrameKey {
    /// 見える字の数（時刻で増える）。
    visible: usize,
    /// 項目の数（消すまで増えるだけ）。
    items: usize,
    /// 選択肢の数（`\_a` の開きは項目を増やさない）。
    choices: usize,
    /// 押した選択肢の境目。
    settled: usize,
    /// その場所の消した回数（同じフレームで消して同じ数を書き直すと項目の数が同じになる）。
    clears: u64,
    /// 済んだ選択肢で絞った後の強調。
    hover: Option<usize>,
    /// 全文の先読みを入れ直した回数。
    lookahead_rev: u64,
}

/// 配置の覚え（audit-emo-text-state-actor 指摘 8）: 鍵が前のフレームと同じなら、配置・選択肢の範囲・キャンバス・装飾を
/// 作り直さず、覚えた答え（行・区切り・帯・キャンバス）で描き手を回す。
///
/// **配置が読む値を足したら鍵にも足す**（足し忘れると古い字を見せる）。
///
/// 鍵が足りる理由: 配置から装飾までが読むのは、届いた項目・見える字の数・選択肢・押した選択肢・
/// 強調・配置の入力・全文の先読みと、`ActorRender` の寿命の間変わらない字幅だけ。
/// 項目と選択肢は消すまで増えるだけなので数で比べ、消して同じ数を書き直したときは消した回数が
/// 進む。登録し直しで描き手を捨てずに替わる配置の入力は値で比べる。今の見た目（`\f` で項目を
/// 増やさずに変わる）は鍵にしない——配置が読むのは字ごとの番号列と表だけで、今の見た目は字が届いた
/// ときに番号へ焼かれる（字が届けば項目の数が進む）。全文の先読みは入れ直す 3 か所で `lookahead_rev` が進む。場所ごとの区間の番号は
/// 鍵にしない——描き手を持つ場所は全消しのとき必ず消した回数が進み、箱の回数・区間・描き手は
/// `set_box_layout` の中で一緒に捨てられるので、`clears_at` と `lookahead_rev` で足りる。
/// スクロール・原点・`\_l` の手動の位置は飛ばす所より後で読むので鍵に入れない。
pub(super) struct FrameMemo {
    key: FrameKey,
    /// 配置の入力の複製（外れたときだけ複製し直す）。
    input: ResolvedBalloonText,
    lines: Vec<PositionedLine>,
    segments: Vec<LineChoiceSegment>,
    bands: Vec<LineBand>,
    canvas: ContentCanvas,
}

impl FrameMemo {
    fn matches(&self, key: &FrameKey, input: &ResolvedBalloonText) -> bool {
        self.key == *key && self.input == *input
    }

    /// 覚えた行・区切り・帯からクリックを受ける範囲を出す（[`ChoiceRanges::hit_rows`] を通す・
    /// 区切りと帯は貸して返す）。
    fn hit_rows(&mut self, region: &TextRegion, mode: WritingMode) -> Vec<CanvasHitRow> {
        let ranges = ChoiceRanges {
            lines: &self.lines,
            region,
            mode,
            segments: std::mem::take(&mut self.segments),
            bands: std::mem::take(&mut self.bands),
        };
        let rows = ranges.hit_rows();
        self.segments = ranges.segments;
        self.bands = ranges.bands;
        rows
    }
}

/// 1 つの場所の行の列から出した選択肢の範囲と帯。クリックを受ける範囲は同じ値から
/// [`ChoiceRanges::hit_rows`] で出す（描き替えたフレームでだけ呼ぶ）。
pub(crate) struct ChoiceRanges<'a> {
    lines: &'a [PositionedLine],
    region: &'a TextRegion,
    mode: WritingMode,
    /// 選択肢の範囲（強調を塗る所）。
    pub(crate) segments: Vec<LineChoiceSegment>,
    /// 行ごとの帯。強調の描画・クリックを受ける範囲・箱の字の矩形へ同じ列を配る。
    pub(crate) bands: Vec<LineBand>,
}

/// 行の列 → 選択肢の範囲・帯（本番の提示と検査が同じこの関数を呼ぶ・GPU の資源は要らない）。
/// 字幅は呼び出しの間だけ借りる（続く描画の面の可変の借用と重ならない）。
pub(crate) fn choice_ranges<'a>(
    lines: &'a [PositionedLine],
    spans: &[ChoiceSpan],
    mode: WritingMode,
    region: &'a TextRegion,
    metrics: &dyn GlyphMetrics,
) -> ChoiceRanges<'a> {
    ChoiceRanges {
        lines,
        region,
        mode,
        segments: annotate_lines(lines, spans),
        bands: line_bands(lines, mode, metrics),
    }
}

impl ChoiceRanges<'_> {
    /// 持っている行の列・範囲・帯からクリックを受ける範囲を出す（canvas-local image px）。
    /// 引数を取らないので、別の行の列や別の帯で当たりを作る道が無い。
    pub(crate) fn hit_rows(&self) -> Vec<CanvasHitRow> {
        derive_hit_rows(
            self.lines,
            &self.segments,
            self.mode,
            self.region,
            &self.bands,
        )
    }
}
