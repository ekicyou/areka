//! balloon の子: ホバーの追従（`on_balloon_pointer_moved`・選択肢の行のハイライトとバルーン滞在の記録）。
//! 足す予定の spec: anchor-tag-canon（アンカーのホバー）。

use std::rc::Rc;

use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use wintf::ecs::pointer::{Phase, PointerState};

use crate::emo2_boot::frame::Emo2Wiring;
use crate::placement::spawn::BalloonWindowMarker;

use super::{BalloonWiring, HoverAction, hit_choice_row, hover_action};

// ---------------------------------------------------------------------------
// 配線層（tasks.md task 4.1・design「配線層（input_events/balloon.rs）> balloon ハンドラ」）
//
// wintf `PointerEventHandler` 署名の移動ハンドラ。Bubble 相のみ処理し、固定順の借用規律
// （共有借用→スナップショット→借用解放→可変借用で inject・design §204）に従って hover 追従を
// 上流 runtime へ橋渡しする薄い結線。判断分岐は純関数核（hit_choice_row／hover_action）へ集約済み
// で、本ハンドラは snapshot→純関数→適用のみを行う（自前描画なし・R1.6／R4.1）。
// ---------------------------------------------------------------------------

/// バルーン窓のポインタ移動ハンドラ（Bubble のみ処理・hover 追従駆動・R1.1/1.2/1.3/1.4/1.6/3.1/3.3/4.1/4.2/8.4）。
///
/// wintf `PointerEventHandler` 署名（donor `on_char_pointer_moved` 同型）。**Bubble 相のみ処理し
/// Tunnel は伝播続行のため即 `false`**。`BalloonWindowMarker.scope` を取り、`client_point`（窓 client
/// **物理 px**・i32）を `as f32` で**無変換のまま**（行ヒット矩形が `to_window_physical` で既に実適用
/// k ×済みの窓物理 px であり点と同一空間ゆえ——k=1.0 だからではない。÷k の追加は二重縮約・
/// R5.6/5.7・R6.4）純関数核へ渡し、hover 遷移を上流 `TextLayerRuntime` へ注入する。moved は非侵襲ゆえ
/// **常に `false`**。
///
/// **借用規律（固定順序・design §204）**:
/// 1. `Emo2Wiring` 共有借用→`runtime()` アクセサで `Rc` clone→world 側借用解放。
///    `Emo2Wiring` 不在（boot 前／失敗）は**正常縮退**＝`debug!`＋no-op（donor presenter=None 同型・R4.1）。
/// 2. `BalloonWiring` へバルーン滞在（`balloon_hover`）を記録し `last_injected` を copy（可変借用即解放）。
///    `BalloonWiring` 不在は結線漏れ＝**構成異常** `error!(event = "balloon_wiring_missing")`＋no-op
///    （配線存在檻が開発時に検出）。滞在の記録は選択肢の有無・行ヒットの有無に依存しない独立の軸
///    （areka-P0-balloon-visibility 5.2）。
/// 3. runtime `try_borrow`（不変）でスナップショット（`choice_active`＋現行 `choice_hit_rows` を純関数
///    評価・move はここで完結）。`try_borrow` 失敗は構成異常 `error!(event = "balloon_runtime_borrow_failed")`。
/// 4. 借用解放後に runtime `try_borrow_mut` で `inject_choice_hover`（`Inject` アームのみ）。
/// 5. `BalloonWiring` 可変借用で自前 `hover` 更新。
///
/// `RefCell` は `try_borrow`／`try_borrow_mut` を用い、失敗時は `error!`＋no-op（panic しない・log-first）。
/// hover 遷移注入時は `debug!(event = "choice_hover_inject")` を発行する（DD-CI-7・トラブルシュート用）。
/// クリック確定・`send`・`info!` は本ハンドラの範囲外（押下ハンドラ＝task 4.2）。
///
/// 本番到達済み——[`attach_balloon_pointer_handlers`]（`ghost_session::prepare_ghost_windows` から
/// 呼ばれる）が `BalloonWindowMarker` 窓へ `OnPointerMoved` として挿入する。
pub(crate) fn on_balloon_pointer_moved(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // (1) Bubble 相のみ処理。Tunnel は伝播続行のため即 false（donor 同型・非侵襲）。
    let state = match ev {
        Phase::Tunnel(_) => return false,
        Phase::Bubble(s) => s,
    };

    // scope は BalloonWindowMarker から読む（donor char_scope の鏡写し・R-3）。attach は marker 窓のみを
    // 標的とするため不在は理論上不到達の構成異常＝error!＋no-op（silent failure 禁止・panic しない）。
    let Some(scope) = world.get::<BalloonWindowMarker>(entity).map(|m| m.scope) else {
        tracing::error!(
            event = "balloon_marker_missing",
            "BalloonWindowMarker 不在の entity へ移動ハンドラが着火（理論上不到達）: no-op 縮退"
        );
        return false;
    };
    let actor = ActorKey::from(scope.to_string());

    // (2) client 物理 px（i32）を f32 へ——無変換のまま渡す。行矩形が to_window_physical で既に実適用
    // k ×済みの窓物理 px ゆえ同一空間で一致する（k=1.0 だからではない）。÷k 追加は二重縮約（R6.4）。
    let x = state.client_point.x as f32;
    let y = state.client_point.y as f32;

    // ── 借用規律 ① Emo2Wiring 共有借用→runtime() で Rc clone→world 側借用解放 ─────────────────
    // Emo2Wiring 不在（boot 前／失敗）は正常縮退＝debug!＋no-op（donor presenter=None 同型・R4.1）。
    let Some(runtime) = world
        .get_non_send::<Emo2Wiring>()
        .map(|w| Rc::clone(w.runtime()))
    else {
        tracing::debug!(
            event = "choice_moved_no_emo2",
            scope,
            "Emo2Wiring 不在（boot 前／失敗）: hover 追従を no-op 縮退"
        );
        return false;
    };

    // ── 借用規律 ② BalloonWiring へ滞在を記録し last_injected を copy（可変借用即解放）───────────
    // 滞在の記録（areka-P0-balloon-visibility 5.2）は選択肢機構と**独立の軸**——選択肢の表示有無・
    // 行ヒットの有無に依らず、バルーン窓上の移動そのもので当該 scope を真にする（ゆえに選択肢
    // スナップショット ③ より前で行い、③ の縮退に巻き込まれない）。解除は窓外離脱
    // （`clear_balloon_hover_on_leave`）と非表示遷移時の掃除（`BalloonWiring::clear_balloon_hover`）。
    // 読み口 `is_balloon_hovered` はバルーン可視性の相が毎フレーム読む（抑止条件の観測）。
    // BalloonWiring 不在は結線漏れ＝構成異常 error!（配線存在檻が開発時に検出）＋no-op。
    let Some(last_injected) = world.get_non_send_mut::<BalloonWiring>().map(|mut bw| {
        bw.set_balloon_hover(scope);
        bw.hover(scope)
    }) else {
        tracing::error!(
            event = "balloon_wiring_missing",
            scope,
            "BalloonWiring 不在（結線漏れ）: hover 追従を no-op 縮退"
        );
        return false;
    };

    // ── 借用規律 ③ runtime 不変借用でスナップショット（純関数評価・move はここで完結）────────────
    // 毎イベント現行 choice_hit_rows に対して hit 判定する（新選択肢集合へ持ち越さない・R3.3）。
    // try_borrow 失敗（理論上不到達の構成異常）は error!＋no-op（panic しない・log-first）。
    let action = match runtime.try_borrow() {
        Ok(rt) => {
            let active = rt.choice_active(&actor);
            let rows = rt.choice_hit_rows(&actor);
            let hit_ordinal = hit_choice_row(rows, x, y).map(|i| rows[i].ordinal);
            hover_action(active, hit_ordinal, last_injected)
        }
        Err(_) => {
            tracing::error!(
                event = "balloon_runtime_borrow_failed",
                scope,
                "runtime try_borrow 失敗（不変・スナップショット）: no-op 縮退"
            );
            return false;
        }
    };
    // ここで不変借用（Ref）は解放済み——④ の可変借用と同時に持たない。

    // 純関数の決定を適用する（自前描画なし＝inject_choice_hover のみ・R1.6）。
    match action {
        // 非表示・未注入（R1.4）／表示中・同値既注入（遷移なし）は何もしない。
        HoverAction::NoopInactive | HoverAction::Keep => {}
        // 消滅時整合（R3.4）: 自前状態のみ None 整合・inject はしない（上流原子性が正本）。
        HoverAction::ResetOwnState => {
            let mut bw = world
                .get_non_send_mut::<BalloonWiring>()
                .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）");
            bw.set_hover(scope, None);
        }
        // 遷移（R1.2/1.3）: ④ runtime 可変借用で inject→⑤ BalloonWiring 可変借用で自前状態更新。
        HoverAction::Inject(value) => {
            // ④ try_borrow_mut で inject_choice_hover（Some=行ハイライト／None=解除・描画 API は呼ばない）。
            match runtime.try_borrow_mut() {
                Ok(mut rt) => rt.inject_choice_hover(&actor, value),
                Err(_) => {
                    tracing::error!(
                        event = "balloon_runtime_borrow_failed",
                        scope,
                        "runtime try_borrow_mut 失敗（inject）: no-op 縮退"
                    );
                    return false;
                }
            }
            // ⑤ 可変借用は上で解放済み。BalloonWiring の自前 last-injected を更新する。
            let mut bw = world
                .get_non_send_mut::<BalloonWiring>()
                .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）");
            bw.set_hover(scope, value);
            // hover 遷移注入の marker（DD-CI-7・トラブルシュート用・info ではない）。
            tracing::debug!(
                event = "choice_hover_inject",
                scope,
                ordinal = ?value,
                "hover 遷移を上流 runtime へ注入"
            );
        }
    }

    // moved は常に false（非侵襲・伝播継続）。
    false
}
