use super::init::format_entity_name;
use crate::com::wuc::DrawingSurfaceInteropExt;
use crate::ecs::graphics::composite::{SurfaceComposite, compose_planned};
use crate::ecs::graphics::damage::{ComposePlan, DAMAGE_TARGET};
use crate::ecs::graphics::visual::PxRect;
use crate::ecs::graphics::{
    DrawCommand, DrawTree, GraphicsCommandList, GraphicsCore, SurfaceGraphics, SurfaceGraphicsDirty,
};
use crate::ecs::layout::GlobalArrangement;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::collections::HashMap;
use std::fmt::Display;
use std::time::Instant;
use tracing::{Level, debug, error};
use windows::Win32::Foundation::RECT;
use windows::Win32::System::WinRT::Composition::ICompositionDrawingSurfaceInterop;
use windows::core::{HRESULT, Interface};

// ========== 描画システム ==========

/// 描画の係が触る部品（入れ物の欄だけを `bypass_change_detection` の下で書く）
type RenderSurfaceData = (
    Entity,
    &'static mut SurfaceGraphics,
    &'static GlobalArrangement,
    Option<&'static GraphicsCommandList>,
    Option<&'static DrawTree>,
    Option<&'static Name>,
);

/// 入れ物の作成・合成・写しの段の名（記録の欄）
const STAGE_CREATE: &str = "composite create";
const STAGE_COMPOSE: &str = "composite";
const STAGE_END_DRAW: &str = "EndDraw";

/// 失いでない失敗の記録の控え（設計討議 議題 1）。描画面ごとに直前の `(段, HRESULT)` を持つ。
// shortcut: 失敗のまま消えた実体の控えは残る（1 実体 1 件）、実体の数が膨らむなら消えた実体を掃く。
pub(crate) type FailureLog = HashMap<Entity, (&'static str, HRESULT)>;

/// 失敗を控え、`error!` の番なら true（同じ描画面で同じ `(段, HRESULT)` が続けば 2 回目から false）。
/// 成功した回は `log.remove(&entity)` で控えを消す（次の失敗はまた `error!`）。
pub(crate) fn note_failure(
    log: &mut FailureLog,
    entity: Entity,
    stage: &'static str,
    hr: HRESULT,
) -> bool {
    log.insert(entity, (stage, hr)) != Some((stage, hr))
}

/// 装置の失いで係を抜けるとき控えを全部捨てる（作り直した装置での最初の失敗をまた `error!` にする）。
pub(crate) fn forget_failures_on_device_lost(log: &mut FailureLog) {
    log.clear();
}

/// 次の回で描き直す（41 番）。係が自分で立てた変化の印は次の回の自分に見えないので
/// 命令で立てる。部品ごと消えていれば描き直しも要らない。
fn request_redraw(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .queue_silenced(|mut e: EntityWorldMut| {
            if let Some(mut dirty) = e.get_mut::<SurfaceGraphicsDirty>() {
                dirty.set_changed();
            }
        });
    crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::GRAPHICS);
}

/// 入れ物の作成・合成・写しの段の失敗を今の道に乗せる（4.6・4.7）。装置の失いなら true（係を抜ける）。
fn on_stage_failure(
    graphics_core: &mut ResMut<GraphicsCore>,
    commands: &mut Commands,
    log: &mut FailureLog,
    (entity, name): (Entity, Option<&Name>),
    stage: &'static str,
    err: &windows::core::Error,
) -> bool {
    // 失いの記録と無効化は note_draw_failure が残す。変化の印は失いのときだけ立てる。
    if graphics_core
        .bypass_change_detection()
        .note_draw_failure(err)
    {
        graphics_core.set_changed();
        forget_failures_on_device_lost(log);
        return true;
    }
    let hresult = format_args!("0x{:08X}", err.code().0);
    if note_failure(log, entity, stage, err.code()) {
        error!(entity = %format_entity_name(entity, name), stage, error = ?err, hresult, "[render_surface] draw stage failed");
    } else {
        debug!(entity = %format_entity_name(entity, name), stage, error = ?err, hresult, "[render_surface] draw stage failed again");
    }
    request_redraw(commands, entity);
    false
}

/// 描き直しの量の記録の 1 行の欄（実体名と所要を除く・3.4・5.4・6.1・6.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DamageRecord {
    /// `full`・`partial`・`present`・`skip`。
    pub mode: &'static str,
    /// 全部のときだけ（`FullReason::as_str`）。
    pub reason: Option<&'static str>,
    pub items_redrawn: u32,
    /// 描く列の件の数。
    pub items: u32,
    pub area: u64,
    pub surface_area: u64,
    pub shift_area: u64,
    pub exposed_area: u64,
    /// 描画面へ写した面積（`present_rect` が None なら全面）。
    pub present_area: u64,
}

/// 計画と写した矩形から記録の欄を組む（純粋）。
pub(crate) fn damage_record(
    plan: &ComposePlan,
    items: u32,
    surface: (u32, u32),
    present_rect: Option<PxRect>,
) -> DamageRecord {
    let surface_area = surface.0 as u64 * surface.1 as u64;
    let present_area = present_rect.map_or(surface_area, PxRect::area);
    let none = DamageRecord {
        mode: "skip",
        reason: None,
        items_redrawn: 0,
        items,
        area: 0,
        surface_area,
        shift_area: 0,
        exposed_area: 0,
        present_area: 0,
    };
    match plan {
        ComposePlan::Skip => none,
        ComposePlan::PresentOnly => DamageRecord {
            mode: "present",
            present_area,
            ..none
        },
        ComposePlan::Full(reason) => DamageRecord {
            mode: "full",
            reason: Some(reason.as_str()),
            items_redrawn: items,
            area: surface_area,
            present_area,
            ..none
        },
        ComposePlan::Partial(p) => DamageRecord {
            mode: "partial",
            items_redrawn: p.items_redrawn,
            area: p.area,
            shift_area: p.shift_area,
            exposed_area: p.exposed_area,
            present_area,
            ..none
        },
    }
}

/// 記録の 1 行（target `wintf::damage` の debug・`[tick]`・`perf(…)` とは別の行・6.4）。
pub(crate) fn log_damage(
    entity: impl Display,
    rec: &DamageRecord,
    compose_us: u64,
    present_us: u64,
) {
    debug!(
        target: DAMAGE_TARGET,
        entity = %entity,
        mode = rec.mode,
        reason = rec.reason,
        items_redrawn = rec.items_redrawn,
        items = rec.items,
        area = rec.area,
        surface_area = rec.surface_area,
        shift_area = rec.shift_area,
        exposed_area = rec.exposed_area,
        present_area = rec.present_area,
        compose_us,
        present_us,
        "[render_surface] damage"
    );
}

fn elapsed_us(start: Option<Instant>) -> u64 {
    start.map_or(0, |t| t.elapsed().as_micros() as u64)
}

/// Surfaceへの自己描画（Phase 4: 自己描画方式）
///
/// 各Entityが自分の入れ物（`SurfaceComposite`）へ自分のGraphicsCommandListと根の木（DrawTree）を
/// 合成し、WUC の描画面へ atlas の offset で写す（4.1）。入れ物が無いか大きさが違えばその回に作る（4.5）。
/// 子の描画は各子が自分のSurfaceで行い、WUC の見た目（Visual）の木で合成する。
///
/// - Changed<SurfaceGraphicsDirty>を持つEntityが対象
/// - GlobalArrangementのスケール成分を適用（DPIスケール対応）
/// - 装置が無い・無効なら何も描かずに戻る（描画面は作り直しで全部作り直される）
/// - `GraphicsCore` を可変で触るのは装置の失いのときだけ（ふだんの描画で変化の印を立てない）
/// - 入れ物への書き込みは `Changed<SurfaceGraphics>` を立てない（立てると毎回描き直しになる）
/// - 失いでない失敗（BeginDraw・入れ物の作成・合成・写し）は、その面の描き直しの印を命令で
///   立て直し、次の回で描き直す。合成に失敗した回は WUC の描画面に触らない（前の絵が残る）
pub fn render_surface(
    mut surfaces: Query<RenderSurfaceData, Changed<SurfaceGraphicsDirty>>,
    graphics_core: Option<ResMut<GraphicsCore>>,
    mut commands: Commands,
    mut log: Local<FailureLog>,
) {
    let Some(mut graphics_core) = graphics_core else {
        return;
    };
    // 共有の DC を複製して持つ（失いの記録で GraphicsCore を可変で借りるため）。無効なら None。
    let Some(shared_dc) = graphics_core.device_context().cloned() else {
        return;
    };
    // 記録の水準は係の頭で 1 度だけ見る。開いているときだけ所要を測る（計画と答えは変えない・6.3）。
    let record = tracing::enabled!(target: DAMAGE_TARGET, Level::DEBUG);
    let now = || record.then(Instant::now);

    for (entity, mut surface, global_arrangement, cmd_list_opt, tree, name) in surfaces.iter_mut() {
        let Some(surface_ref) = surface.surface() else {
            continue;
        };

        // WUC: CompositionDrawingSurface を ICompositionDrawingSurfaceInterop へ cast する。
        let surface_interop = match surface_ref.cast::<ICompositionDrawingSurfaceInterop>() {
            Ok(si) => si,
            Err(err) => {
                error!(
                    entity = %format_entity_name(entity, name),
                    error = ?err,
                    "cast to ICompositionDrawingSurfaceInterop failed"
                );
                continue;
            }
        };
        let who = (entity, name);

        // 入れ物へ合成する（WUC の BeginDraw より前）。変換は拡大だけ（atlas の offset は写しで足す）。
        let size = surface.size;
        let surface = surface.bypass_change_detection();
        // 入れ物を今作った回の理由（First か Recreated・3.1）。
        let mut fresh = None;
        if surface.composite().is_none_or(|c| c.size() != size) {
            let slot = surface.composite_slot();
            *slot = None;
            match SurfaceComposite::new(&shared_dc, size) {
                Ok(c) => {
                    *slot = Some(c);
                    fresh = Some(surface.note_composite_created());
                }
                Err(err) => {
                    let lost = on_stage_failure(
                        &mut graphics_core,
                        &mut commands,
                        &mut log,
                        who,
                        STAGE_CREATE,
                        &err,
                    );
                    if lost {
                        return;
                    }
                    continue;
                }
            }
        }
        let scale = windows_numerics::Matrix3x2 {
            M11: global_arrangement.scale_x(),
            M12: 0.0,
            M21: 0.0,
            M22: global_arrangement.scale_y(),
            M31: 0.0,
            M32: 0.0,
        };
        let root_list = cmd_list_opt.and_then(|c| c.command_list());
        let tree_commands = tree.map_or(&[][..], |t| &t.commands[..]);
        let slot = surface.composite_slot();
        // 合成の前に読む（合成した回は remember が偽に戻す）。
        let presented_before = slot.as_ref().is_some_and(|c| c.presented());
        let compose_start = now();
        // 計画を立てて合成する。失敗した入れ物は控えごと捨てる（途中まで描いた絵を読ませない・3.5）。
        let planned = match compose_planned(
            slot,
            &shared_dc,
            fresh,
            scale,
            root_list,
            tree_commands,
            size,
        ) {
            Ok((planned, _)) => planned,
            Err(err) => {
                let lost = on_stage_failure(
                    &mut graphics_core,
                    &mut commands,
                    &mut log,
                    who,
                    STAGE_COMPOSE,
                    &err,
                );
                if lost {
                    return;
                }
                continue;
            }
        };
        let compose_us = elapsed_us(compose_start);
        let items = || {
            tree_commands
                .iter()
                .filter(|c| matches!(c, DrawCommand::Draw(_)))
                .count() as u32
        };
        // 描画面へ写す矩形（None＝全面）。範囲だけで前の写しを終えている回だけ狭める（2.6・2.8）。
        let present_rect = match &planned {
            ComposePlan::Skip => {
                log.remove(&entity);
                if record {
                    let rec = damage_record(&planned, items(), size, None);
                    log_damage(format_entity_name(entity, name), &rec, compose_us, 0);
                }
                continue;
            }
            ComposePlan::Partial(p) if presented_before => Some(p.present_rect),
            _ => None,
        };

        let Some(composite) = slot.as_ref() else {
            continue;
        };
        // 部分の矩形は作りたての描画面では断られる（E_INVALIDARG）が、作りたての描画面は入れ物も
        // 作りたて＝必ず全部なのでここへは来ない（全面を 1 度描いた後は通る・2026-11-14 実測）。
        let update_rect = present_rect.map(|r| RECT {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        });
        let present_start = now();
        let (dc, offset) = match surface_interop.begin_draw(update_rect.as_ref()) {
            Ok(result) => result,
            Err(err) => {
                // 失いの記録と無効化は note_draw_failure が残す。残りの描画面も同じ装置なので描かない。
                // 変化の印は失いのときだけ立てる（失いでない失敗で下流に控えを捨てさせない）。
                if graphics_core
                    .bypass_change_detection()
                    .note_draw_failure(&err)
                {
                    graphics_core.set_changed();
                    forget_failures_on_device_lost(&mut log);
                    return;
                }
                error!(
                    entity = %format_entity_name(entity, name),
                    error = ?err,
                    hresult = format_args!("0x{:08X}", err.code().0),
                    "BeginDraw failed"
                );
                request_redraw(&mut commands, entity);
                continue;
            }
        };

        // 写し: 入れ物を atlas の offset（物理ピクセル）へ。失敗は EndDraw に出る。写しに失敗した回は
        // 控えを残し写し終えた印を付けない（次の回に写すだけで写す・2.8）。
        match present_rect {
            Some(r) => composite.copy_rect_to(&dc, r, offset),
            None => composite.copy_to(&dc, offset),
        }
        if let Err(err) = surface_interop.end_draw() {
            let lost = on_stage_failure(
                &mut graphics_core,
                &mut commands,
                &mut log,
                who,
                STAGE_END_DRAW,
                &err,
            );
            if lost {
                return;
            }
            continue;
        }
        if let Some(c) = slot.as_mut() {
            c.mark_presented();
        }
        log.remove(&entity);
        if record {
            let rec = damage_record(&planned, items(), size, present_rect);
            log_damage(
                format_entity_name(entity, name),
                &rec,
                compose_us,
                elapsed_us(present_start),
            );
        }
    }
}

// Note: DComp の `commit_composition` システムは WUC 移行で削除された（要件 7.1）。
// WUC は DispatcherQueue 経由で変更が暗黙反映されるため明示的な commit は不要。
// `CommitComposition` schedule 自体は空スケジュール（no-op）として残置される
// （ULW present system は wintf-ulw-removal で撤去済み）。
