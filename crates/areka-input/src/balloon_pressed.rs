//! balloon の子: クリック（`on_balloon_pointer_pressed`・話し終えた後の矢印の押下を先に取り、選択肢とアンカーの確定の発行・送れた選択肢の場所の選択肢を済みにすることと、末尾で利用者の中断へ渡す）。

use std::cell::RefCell;
use std::rc::Rc;

use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::place::PlaceKey;
use areka_emo_text::scroll_marker::ARROW_STEP_LINES;
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use wintf::ecs::pointer::{Phase, PointerState};

use areka_placement::spawn::BalloonWindowMarker;
use areka_runtime::frame::RuntimeWiring;

use super::user_break::UserBreakWiring;
use super::{BalloonWiring, click_selection, dispatch_selection};

/// バルーン窓のポインタ押下ハンドラ（Bubble のみ処理・確定クリック発行）。
///
/// Tunnel は伝播続行のため即 `false`。**左シングルクリック限定**＝`state.left_down` のみを確定として扱い、
/// 選択の確定は `double_click` を見ない（DBLCLK 2 打目も独立 press として扱う）。`double_click` は
/// 末尾で利用者の中断の入口 `on_left_press` へ渡すだけである。右・中ボタン down は `false` 素通し。
/// 単一クリックの二重発行は wintf dispatch のエッジ検出（dispatch 後 `left_down` クリア）が防ぎ、
/// 本ハンドラは 1 dispatch＝高々 1 send を守る。
///
/// 借用の順は移動ハンドラと同じ（`RuntimeWiring` 共有借用で `Rc` を clone→world 側の借用を解放→runtime を
/// 不変借用してスナップショット）。現行 rows のみを [`click_selection`] へ渡すことで stale 棄却が成立する。
/// 棄却は `debug!(event = "choice_click_rejected", reason)`（`inactive`／`no_hit`）、選択は
/// [`dispatch_selection`] で高々 1 回発行する。`RuntimeWiring` 不在は正常縮退（`debug!`＋no-op）、
/// `BalloonWiring` 不在は結線漏れの `error!`＋no-op、`RefCell` の借用失敗も `error!`＋no-op（panic しない）。
/// `resolve_choice` は本 crate から呼ばない（発行まで・カスケードは kanade 側）。
/// 座標は窓 client 物理 px を**無変換**で照合する（理由は [`hit_choice_row`] の座標契約 doc）。
///
/// **戻り値**: `ChoiceSelection` を発行したとき、または利用者の中断を受け入れたとき `true`
/// （棄却・縮退・非左押下・Tunnel 時は `false`。途中の早期復帰では中断も作らない）。
///
/// [`attach_balloon_pointer_handlers`] が `BalloonWindowMarker` 窓へ `OnPointerPressed` として挿入する。
pub(crate) fn on_balloon_pointer_pressed(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let state = match ev {
        Phase::Tunnel(_) => return false,
        Phase::Bubble(s) => s,
    };

    if !state.left_down {
        return false;
    }

    // attach は marker 窓のみを標的とするため、scope の不在は理論上不到達の構成異常＝error!＋no-op（panic しない）。
    let Some(scope) = world.get::<BalloonWindowMarker>(entity).map(|m| m.scope) else {
        tracing::error!(
            event = "balloon_marker_missing",
            "BalloonWindowMarker 不在の entity へ押下ハンドラが着火（理論上不到達）: no-op 縮退"
        );
        return false;
    };
    let actor = ActorKey::from(scope.to_string());

    // client 物理 px（i32）を f32 へ——無変換のまま渡す。行矩形が to_window_physical で既に実適用
    // k ×済みの窓物理 px ゆえ同一空間で一致する。÷k 追加は二重縮約。
    let x = state.client_point.x as f32;
    let y = state.client_point.y as f32;

    // RuntimeWiring 不在（boot 前／失敗）は正常縮退＝debug!＋no-op。
    let Some(runtime) = world
        .get_non_send::<RuntimeWiring>()
        .map(|w| Rc::clone(w.runtime()))
    else {
        tracing::debug!(
            event = "choice_pressed_no_runtime",
            scope,
            "RuntimeWiring 不在（boot 前／失敗）: クリック確定を no-op 縮退"
        );
        return false;
    };

    // ── 矢印──────────────────────────────────
    // 話し終えた後だけ、矢印の当たり（α ≥ 0.5 の画素）を先に取る。当たれば送って押下を消費し、
    // 閉じる働き・選択肢やリンクの確定へ流さない。話している最中・当たらないなら今の流れへ。
    // 矢印かどうかは送り手の entity でなく押下の点で決める（不透明な絵の上では送り手は絵の entity）。
    if finished_talking(world) && press_arrow(&runtime, &actor, (x, y), scope) {
        return true;
    }

    // BalloonWiring 不在は結線漏れ＝構成異常 error!＋no-op。
    if world.get_non_send::<BalloonWiring>().is_none() {
        tracing::error!(
            event = "balloon_wiring_missing",
            scope,
            "BalloonWiring 不在（結線漏れ）: クリック確定を no-op 縮退"
        );
        return false;
    }

    // 現行 choice_hit_rows のみを読む。active は棄却理由（inactive／no_hit）の弁別のために別途控える。
    // try_borrow 失敗（理論上不到達の構成異常）は error!＋no-op（panic しない）。
    let (active, selection) = match runtime.try_borrow() {
        Ok(rt) => {
            // 押せる範囲（選択肢かアンカー）があるか。アンカーだけのバルーンでも押下を範囲と照合する。
            let active = rt.hit_active(&actor);
            let rows = rt.choice_hit_rows(&actor);
            let selection = click_selection(active, rows, x, y, scope);
            (active, selection)
        }
        Err(_) => {
            tracing::error!(
                event = "balloon_runtime_borrow_failed",
                scope,
                "runtime try_borrow 失敗（不変・click スナップショット）: no-op 縮退"
            );
            return false;
        }
    };

    let selected_now = match selection {
        // 非表示（active=false）or 非ヒット → 棄却（非発行）。理由を弁別して debug 発火。
        None => {
            let reason = if !active { "inactive" } else { "no_hit" };
            tracing::debug!(
                event = "choice_click_rejected",
                scope,
                reason,
                "クリック確定を棄却（非表示中 or 非ヒット）: 非発行"
            );
            false
        }
        Some(sel) => dispatch_selection(
            world
                .get_non_send::<BalloonWiring>()
                .expect("BalloonWiring は直上で存在確認済み"),
            Some(&runtime),
            &PlaceKey::balloon(&actor),
            sel,
        ),
    };

    // 利用者の中断。選択を確定した押下とその続きは中断にしない。
    // 当たった範囲がアンカーでも `selected_now` は真（種類を問わず「選択で使った」押下）。
    let accepted = super::user_break::on_left_press(world, scope, state.double_click, selected_now);
    selected_now || accepted
}

/// 話し終えているか（矢印の押下を先に取ってよいか・普通のバルーンの窓と箱の押下が共有する）。
///
/// 利用者の中断の入口（[`super::user_break::on_left_press`]）・箱の押下（`on_box_press`）が
/// 早送り・中断へ回すかを決めるのと同じ状態（`UserBreakWiring` の「話している最中」）から引く。
/// 話している最中かはゴーストに 1 つ（スコープごとには持たない）。持ち物が無い（結線前）は箱の
/// 押下と同じく話していない扱い。
pub(crate) fn finished_talking(world: &World) -> bool {
    world
        .get_non_send::<UserBreakWiring>()
        .is_none_or(|w| !w.talking())
}

/// 押下の点（窓のクライアント物理 px）がそのスコープの普通のバルーンの矢印に当たれば、その向きへ
/// [`ARROW_STEP_LINES`] 行送って `true`。当たらない（矢印が無い・透けた画素）なら `false`。
fn press_arrow(
    runtime: &RefCell<TextLayerRuntime>,
    actor: &ActorKey,
    point: (f32, f32),
    scope: usize,
) -> bool {
    let Ok(mut rt) = runtime.try_borrow_mut() else {
        tracing::error!(
            event = "balloon_runtime_borrow_failed",
            scope,
            "runtime try_borrow_mut 失敗（矢印の押下）: 矢印として扱わない"
        );
        return false;
    };
    let place = PlaceKey::balloon(actor);
    let Some(kind) = rt.marker_at_point(&place, point) else {
        return false;
    };
    let change = rt.scroll_manual_at(&place, kind.dir(), ARROW_STEP_LINES);
    tracing::trace!(
        event = "balloon_arrow_pressed",
        scope,
        ?kind,
        ?change,
        "矢印の押下: 送りだけをしてクリックにしない"
    );
    true
}

#[cfg(test)]
#[path = "balloon_pressed_tests.rs"]
mod tests;
