//! balloon の子: クリック（`on_balloon_pointer_pressed`・選択肢とアンカーの確定の発行と、末尾で利用者の中断へ渡す）。
//! 足す予定の spec: talk-fast-forward・balloon-markers（矢印のクリック）。

use std::rc::Rc;

use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use wintf::ecs::pointer::{Phase, PointerState};

use crate::emo2_boot::frame::Emo2Wiring;
use crate::placement::spawn::BalloonWindowMarker;

use super::{BalloonWiring, click_selection};

/// バルーン窓のポインタ押下ハンドラ（Bubble のみ処理・確定クリック発行・R2.1/2.3/2.4/2.5/2.6/3.1/3.2/4.2/5.1/8.4）。
///
/// wintf `PointerEventHandler` 署名（移動ハンドラの鏡写し）。**Bubble 相のみ処理し Tunnel は伝播続行の
/// ため即 `false`**。**左シングルクリック限定**＝`state.left_down` のみを確定として扱い、選択の確定は
/// `double_click` フィールドを見ない（DBLCLK 2 打目も独立 press として扱う・DD-CI-9）。`double_click` は
/// 末尾で利用者の中断の入口 `fn on_left_press`（`user_break.rs`）へ渡すだけである。右・中ボタン down は
/// 確定でないため `false` 素通し（wheel/keyboard は本 spec 未実装・R5.1）。
///
/// 単一クリック二重発行は wintf dispatch のエッジ検出（dispatch 後 `left_down` クリア）が構造的に防止し、
/// 本ハンドラは 1 dispatch＝高々 1 send を守る（`Some` 選択 1 つにつき `send_selection` を高々 1 回・R2.4）。
///
/// **借用規律（移動ハンドラと固定同順・design §204）**:
/// 1. `Emo2Wiring` 共有借用→`runtime()` で `Rc` clone→world 側借用解放。`Emo2Wiring` 不在（boot 前／失敗）は
///    **正常縮退**＝`debug!(event = "choice_pressed_no_emo2")`＋no-op（donor presenter=None 同型・R4.1）。
/// 2. `BalloonWiring` 存在確認（共有借用即解放）。不在は結線漏れ＝**構成異常**
///    `error!(event = "balloon_wiring_missing")`＋no-op。
/// 3. runtime `try_borrow`（不変）でスナップショット——`hit_active`（押せる範囲＝選択肢かアンカーが
///    あるか）＋**現行** `choice_hit_rows` を純関数
///    [`click_selection`]（task 3.3）へ渡し `Option<ChoiceSelection>` を得る（現行 rows のみ読むことで
///    stale 棄却が成立・R2.5/3.2）。`try_borrow` 失敗は構成異常
///    `error!(event = "balloon_runtime_borrow_failed")`＋no-op。
/// 4. `None`（非表示 or 非ヒット）→ `debug!(event = "choice_click_rejected", reason)`＋`false`（非発行・
///    R2.3/3.1・reason は `!active` なら `"inactive"`／それ以外は `"no_hit"`）。`Some(sel)` →
///    `BalloonWiring::send_selection` で高々 1 回発行。成功時 `info!(event = "choice_selected", scope, id,
///    label, references_len)` を **1 行**発火し `true`（DD-CI-7・R7.2 grep 対象）。送出失敗（受け口消滅）→
///    `error!(event = "choice_selection_send_failed", scope, id)`＋`false`。
///
/// `resolve_choice` は本 crate から呼ばない（発行まで・カスケードは W6・R2.6/5.4）。自前描画なし。
/// `RefCell` は `try_borrow` のみ（panic しない・log-first）。座標は窓 client 物理 px を**無変換**で
/// 照合する——行矩形が `to_window_physical` で既に実適用 k ×済みゆえ同一空間で一致する（k=1.0 だから
/// ではない）。シェルの「点 ÷k」とは逆向きだが等価に正しく、本経路へ ÷k を足すと二重縮約になり正常
/// 動作を壊す（R5.6/5.7・R6.4。詳細は [`hit_choice_row`] の座標契約 doc）。
///
/// **戻り値**: `ChoiceSelection` を発行したとき、または利用者の中断を受け入れたとき `true`
/// （棄却・縮退・非左押下・Tunnel 時は `false`。途中の早期復帰では中断も作らない）。
///
/// 本番到達済み——[`attach_balloon_pointer_handlers`]（`ghost_session::prepare_ghost_windows` から
/// 呼ばれる）が `BalloonWindowMarker` 窓へ `OnPointerPressed` として挿入する。
pub(crate) fn on_balloon_pointer_pressed(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // (1) Bubble 相のみ処理。Tunnel は伝播続行のため即 false（移動ハンドラ同型・非侵襲）。
    let state = match ev {
        Phase::Tunnel(_) => return false,
        Phase::Bubble(s) => s,
    };

    // (2) 左シングルクリック限定（R5.1）。left_down 以外（右・中 down 等の非左押下）は確定でないため
    // false 素通し。選択の確定は double_click を見ない（DBLCLK 2 打目も独立 press 扱い・DD-CI-9）。
    if !state.left_down {
        return false;
    }

    // scope は BalloonWindowMarker から読む（移動ハンドラの鏡写し・R-3）。attach は marker 窓のみを標的と
    // するため不在は理論上不到達の構成異常＝error!＋no-op（silent failure 禁止・panic しない）。
    let Some(scope) = world.get::<BalloonWindowMarker>(entity).map(|m| m.scope) else {
        tracing::error!(
            event = "balloon_marker_missing",
            "BalloonWindowMarker 不在の entity へ押下ハンドラが着火（理論上不到達）: no-op 縮退"
        );
        return false;
    };
    let actor = ActorKey::from(scope.to_string());

    // client 物理 px（i32）を f32 へ——無変換のまま渡す。行矩形が to_window_physical で既に実適用
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
            event = "choice_pressed_no_emo2",
            scope,
            "Emo2Wiring 不在（boot 前／失敗）: クリック確定を no-op 縮退"
        );
        return false;
    };

    // ── 借用規律 ② BalloonWiring 存在確認（共有借用即解放）──────────────────────────────────
    // BalloonWiring 不在は結線漏れ＝構成異常 error!（配線存在檻が開発時に検出）＋no-op。
    if world.get_non_send::<BalloonWiring>().is_none() {
        tracing::error!(
            event = "balloon_wiring_missing",
            scope,
            "BalloonWiring 不在（結線漏れ）: クリック確定を no-op 縮退"
        );
        return false;
    }

    // ── 借用規律 ③ runtime 不変借用でスナップショット（純関数 click_selection 評価）────────────
    // 現行 choice_hit_rows のみを読む（stale 棄却は現行 rows のみ読むことで成立・R2.5/3.2）。active は
    // 棄却理由（inactive／no_hit）の弁別のために別途控える。try_borrow 失敗（理論上不到達の構成異常）は
    // error!＋no-op（panic しない・log-first）。
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
    // ここで不変借用（Ref）は解放済み。

    // (4) 純関数の決定を適用する（resolve_choice は呼ばない＝発行まで・R2.6/5.4／自前描画なし）。
    let selected_now = match selection {
        // 非表示（active=false）or 非ヒット → 棄却（非発行・R2.3/3.1）。理由を弁別して debug 発火。
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
        // ヒット確定（R2.1/2.4）: 高々 1 回だけ発行する（send_selection は 1 send・二重発行なし）。
        Some(sel) => {
            // info! 用の値を send 前に控える（send_selection が selection の所有権を消費するため）。
            let id = sel.id.clone();
            let label = sel.label.clone();
            let references_len = sel.references.len();
            // ② で存在確認済みの BalloonWiring を借りて発行シンクへ送る（reuse・task 2.2）。
            let sent = world
                .get_non_send::<BalloonWiring>()
                .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）")
                .send_selection(sel);
            if sent {
                // 実機サインオフ導線（DD-CI-7・R7.2 grep 対象）: 発行 1 回につき 1 行。
                tracing::info!(
                    event = "choice_selected",
                    scope,
                    id = %id,
                    label = %label,
                    references_len,
                    "選択確定: ChoiceSelection を発行"
                );
                true
            } else {
                // 送出失敗（受け口消滅後の Sender エラー）は構成異常＝error!＋no-op 縮退
                // （design Error Handling・R7 grep 対象と別導線）。
                tracing::error!(
                    event = "choice_selection_send_failed",
                    scope,
                    id = %id,
                    "ChoiceSelection 発行シンク送出失敗（受け口消滅後）: no-op 縮退"
                );
                false
            }
        }
    };

    // (5) 利用者の中断（areka-P0-balloon-break）。選択を確定した押下とその続きは中断にしない。
    // 当たった範囲がアンカーでも `selected_now` は真（種類を問わず「選択で使った」押下）。
    let accepted = super::user_break::on_left_press(world, scope, state.double_click, selected_now);
    selected_now || accepted
}
