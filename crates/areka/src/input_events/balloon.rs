//! バルーン選択肢対話配線（areka-P0-choice-interact）。
//!
//! バルーン窓のポインタイベントを捉え、選択肢ヒットの判定・ハイライト追従・クリック確定を
//! kanade／文字層 runtime へ橋渡しするサブモジュール。単一責務＝バルーン選択肢対話配線。
//!
//! 本 task（1）はモジュール雛形のみを確立する。以降の task（2.x〜6.x）で本 mod へ増設される:
//! - 契約型（`ChoiceSelection` ほか）
//! - NonSend 資源（`BalloonWiring`・`ChoiceSelectionInbox`）
//! - 純関数判定核（選択肢ヒット・ハイライト遷移の決定的判定）
//! - 配線層（`on_balloon_pointer_moved`／`on_balloon_pointer_pressed`・
//!   `attach_balloon_pointer_handlers`・`wire_balloon_choice`）
//!
//! 上流契約（collision-geometry の resolver・`Emo2Wiring::runtime()` の読み口）は消費のみ行い、
//! 逆方向依存（上流が balloon.rs を知る）は禁止（design「依存方向」）。

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};

use areka_emo_text::actor::ChoiceHitRow;
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::find_owner_window;
use wintf::ecs::pointer::{
    OnPointerMoved, OnPointerPressed, Phase, PointerLeave, PointerState, dispatch_pointer_events,
};

use crate::emo2_boot::frame::Emo2Wiring;
use crate::placement::spawn::BalloonWindowMarker;

/// 選択確定のワイヤ形（本 spec 契約正本・2.2）。
///
/// 下流 W6 が表示層へ再照会せず選択解決とカスケード発火を組み立てられる自己完結データ。
/// 解決キーは `id`（`SakuraMsg::ResolveChoice { id }` と整合）であり、表示層内部の主キーである
/// `ordinal` はワイヤ形に含めない（漏洩防止・design 2.6）。本 task（2.1）は型定義のみで、
/// 発行・`resolve_choice` 呼出は行わない（後続 task の範囲）。フィールド消費は下流 task で結線される。
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChoiceSelection {
    /// `\q` ID（選択解決の主キー・不透明転写）。
    pub id: String,
    /// 表示ラベル（不透明転写）。
    pub label: String,
    /// 発生元 scope（`BalloonWindowMarker.scope` 由来）。
    pub scope: usize,
    /// `\q` 第 3 引数以降（参照列・不透明転写）。
    pub references: Vec<String>,
}

/// バルーン選択肢対話の配線資源（NonSend・donor `MouseWiring` 同型・2.2）。
///
/// UI スレッド所有の資源として `World` へ NonSend 挿入し（`insert_non_send_resource`）、Input
/// スケジュール排他システム内でのみ借用する（donor `MouseWiring` と同型）。mpsc `Sender` は
/// `Send` だが `hover` 追跡と一体で UI スレッド固定運用ゆえ NonSend 1 個に束ねる。
///
/// 2 つの用途を束ねる:
/// - `selection_tx`: 選択確定 [`ChoiceSelection`] の発行シンク（C-1・std mpsc）。発行は下流 W6 が
///   受信処理へ置換する [`ChoiceSelectionInbox`] seam へ流れる（`resolve_choice` は本 crate から
///   直接呼ばない・5.3/5.4）。
/// - `hover`: scope→本仕様が最後に注入した hover ordinal の自前追跡（表示層に getter が無いため・
///   B-2）。表示状態の正本ではなく、(a) 同値再注入の遷移検出、(b) 選択肢消滅時の自前状態整合
///   （`None` 上書き・R3.4）のみに用いる。
///
/// 本 task（2.2）は資源定義と発行シンク＋seam の確立まで。ハンドラ結線（`on_balloon_pointer_*`）と
/// hover 遷移・click 発行の消費は後続 task（4.x/6.x）の範囲。
///
/// `#[allow(dead_code)]`: フィールド／メソッドは後続 task（4.x/6.x）のハンドラ結線で初めて本番消費される
/// （M1 では単体檻のみが到達・task 2.1 `ChoiceSelection` と同型の暫定抑止）。
#[allow(dead_code)]
pub(crate) struct BalloonWiring {
    /// [`ChoiceSelection`] 発行シンク（C-1・mpsc）。
    selection_tx: Sender<ChoiceSelection>,
    /// scope → 最後に注入した hover ordinal（getter 不在の自前追跡・B-2）。
    hover: HashMap<usize, Option<usize>>,
}

#[allow(dead_code)] // 全 API は後続 task（4.x/6.x）で本番結線される（M1 は単体檻のみ到達）。
impl BalloonWiring {
    /// 発行シンク [`Sender`] から構築する（`hover` は空 map で初期化・donor `MouseWiring::new` 同型）。
    pub(crate) fn new(selection_tx: Sender<ChoiceSelection>) -> Self {
        Self {
            selection_tx,
            hover: HashMap::new(),
        }
    }

    /// 選択確定 [`ChoiceSelection`] を発行シンクへ送る（一度きり発行・2.4／log-first）。
    ///
    /// [`ChoiceSelectionInbox`] の `Receiver` が生存する限り成功する（`resolve_choice` は呼ばない・
    /// 5.3/5.4）。送出失敗（受け口消滅後の [`Sender`] エラー）は warn＋no-op（`false` 返し・log-first）。
    pub(crate) fn send_selection(&self, selection: ChoiceSelection) -> bool {
        let scope = selection.scope;
        if self.selection_tx.send(selection).is_err() {
            tracing::warn!(
                event = "choice_selection_send_failed",
                scope,
                "ChoiceSelection 発行シンク送出失敗（受け口消滅後）: no-op で継続"
            );
            return false;
        }
        true
    }

    /// scope の最後に注入した hover ordinal を回収する（未注入は `None`・B-2）。
    ///
    /// 後続 task（4.x）の遷移検出（同値再注入の抑制）と消滅時整合（R3.4）で参照される。
    pub(crate) fn hover(&self, scope: usize) -> Option<usize> {
        self.hover.get(&scope).copied().flatten()
    }

    /// scope の hover ordinal を記録する（`None` 上書きは消滅時整合・R3.4）。
    ///
    /// 注入は「本仕様が最後に注入した値」の記録であり表示状態の正本ではない（正本は上流）。
    pub(crate) fn set_hover(&mut self, scope: usize, ordinal: Option<usize>) {
        self.hover.insert(scope, ordinal);
    }
}

/// M1 の暫定受け口（W6 `choice-select-events` が受信処理へ置換する seam・5.3）。
///
/// `Receiver` 生存により [`BalloonWiring::send_selection`] は `Err` にならず、発行の mpsc 観測と
/// 実機ログが成立する。M1 では受信処理を持たない（下流 W6 の領分・単体檻で送信値を観測するのみ）。
///
/// `#[allow(dead_code)]`: M1 は seam の定義と単体観測まで。本番構築・受信は下流 W6 の範囲。
#[allow(dead_code)]
pub(crate) struct ChoiceSelectionInbox(pub(crate) Receiver<ChoiceSelection>);

/// 点包含 hit 判定（純関数・R1.1/1.5/2.3）。
///
/// 包含は半開区間 `[left, right) × [top, bottom)`（whole-pixel 行矩形と整合）——
/// `left`/`top` 辺は包含・`right`/`bottom` 辺は非包含。座標 `x`/`y`（バルーン窓 client
/// **物理 px**・f32）を `HitRectPx` の各辺へ**そのまま**比較する（DPI 変換・スケール係数を
/// 一切挟まない・k=1.0 素通し・R4.2／8.6・DD-IE-10 整合）。
///
/// 判定対象は上流が供給する選択肢行ジオメトリ（`rows`）のみ。選択肢以外のバルーン内リンク
/// （`\_a` 等）は `rows` に含まれず本関数の対象外（R5.2）。
///
/// 重なり時は**逆順走査の最初の一致＝スライス最終一致**の index を返す（`choice_hit_rows` は
/// ordinal 昇順×行昇順ゆえ「後定義が手前」＝画家のアルゴリズムと整合・DD-CI-5）。病的重なり入力
/// でも決定的に高々 1 つの index を返す（R1.1／1.5）。非ヒットは `None`（R2.3）。空 `rows` も `None`。
///
/// 戻り値はスライス index（呼び手が `rows[i].ordinal` 等へ展開する）。同一入力→同一出力（純粋・
/// 決定論・失敗経路なし）。
///
/// `#[allow(dead_code)]`: 本番消費（配線層 task 4.x）まで到達者なし——単体檻のみ到達（M1 暫定抑止）。
#[allow(dead_code)]
pub(crate) fn hit_choice_row(rows: &[ChoiceHitRow], x: f32, y: f32) -> Option<usize> {
    // 逆順走査の最初の一致＝スライス最終一致（後定義が手前・画家のアルゴリズム・DD-CI-5）。
    // 半開区間 [left, right) × [top, bottom)：left/top は包含・right/bottom は非包含。
    // 座標は物理 px 素通し——スケール係数を掛けない（R4.2／8.6）。
    rows.iter()
        .enumerate()
        .rev()
        .find(|(_, row)| {
            let r = &row.rect;
            x >= r.left && x < r.right && y >= r.top && y < r.bottom
        })
        .map(|(i, _)| i)
}

/// hover 遷移の決定（純関数・R1.2/1.3/1.4/3.4）。
///
/// 表示中フラグ・hit 結果（ordinal 展開済）・前回注入値の 3 入力から hover 遷移を
/// 決める副作用なしの決定的関数。World・runtime 借用・GPU・sleep 一切不要——入力→
/// `HoverAction` のみ。呼び手（配線層 task 4.1）が action を解釈する
/// （`Inject` で `inject_choice_hover`・`BalloonWiring.hover` 更新等）。
///
/// - `active == false`（choice 非表示）:
///   - `last_injected == None` → [`HoverAction::NoopInactive`]（未注入ゆえ何もしない・R1.4）。
///   - `last_injected == Some(_)` → [`HoverAction::ResetOwnState`]（注入済ハイライトを消滅時に
///     自前状態のみ `None` 整合・inject はしない＝上流原子性が正本・R3.4）。
///   - この分岐で `hit_ordinal` は無視される（非表示中は hover 追従なし・R1.4）。
/// - `active == true`（choice 表示中）:
///   - `hit_ordinal == last_injected` → [`HoverAction::Keep`]（同値既注入・遷移なし・
///     `Some==Some`／`None==None` 双方を含む）。
///   - `hit_ordinal != last_injected` → [`HoverAction::Inject`]`(hit_ordinal)`（遷移・新値注入。
///     `Some(ordinal)`＝行ハイライト・R1.2／`None`＝ハイライト解除・R1.3）。
///
/// `#[allow(dead_code)]`: 本番消費（配線層 task 4.1）まで到達者なし——単体檻のみ到達（M1 暫定抑止）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum HoverAction {
    /// choice 非表示かつ自前状態も None——何もしない（R1.4）。
    NoopInactive,
    /// choice 非表示だが自前状態が残っている——自前状態のみ None へ整合
    /// （inject はしない・上流原子性が正本・R3.4）。
    ResetOwnState,
    /// 表示中・hover 対象が前回注入値と同一——再注入しない（遷移なし）。
    Keep,
    /// 表示中・hover 対象が変化——`inject_choice_hover(actor, value)` を行う
    /// （`Some(ordinal)`＝行ハイライト・`None`＝ハイライト無し・R1.2/1.3）。
    Inject(Option<usize>),
}

#[allow(dead_code)]
pub(crate) fn hover_action(
    active: bool,
    hit_ordinal: Option<usize>,   // hit_choice_row の結果を ordinal へ展開した値
    last_injected: Option<usize>, // BalloonWiring.hover[scope]
) -> HoverAction {
    if !active {
        // choice 非表示中は hover 追従なし（hit_ordinal は無視・R1.4）。
        return match last_injected {
            None => HoverAction::NoopInactive,
            Some(_) => HoverAction::ResetOwnState, // 消滅時は自前状態のみ None 整合（inject せず・R3.4）。
        };
    }
    // 表示中: 同値なら遷移なし・変化なら新値注入（Some=ハイライト/None=解除・R1.2/1.3）。
    if hit_ordinal == last_injected {
        HoverAction::Keep
    } else {
        HoverAction::Inject(hit_ordinal)
    }
}

/// クリック確定の決定（純関数・R2.1/2.2/2.3/3.1/3.2）。
///
/// クリック時点の**現行**行ジオメトリ（`rows`）のみから [`ChoiceSelection`] を構成する
/// 副作用なしの決定的関数。World・runtime 借用・GPU・send・logging 一切不要——入力→
/// `Option<ChoiceSelection>` のみ。発行シンクへの送出・一度きり制御・ログは呼び手
/// （配線層 task 4.2）の領分。
///
/// - `active == false`（choice 非表示）→ `None`（hit 判定より前に短絡・R3.1）。
///   choice 消滅時の stale／原子性ガード＝非表示中はたとえ矩形内座標でも発行しない。
/// - `active == true` かつ非ヒット → `None`（[`hit_choice_row`] が `None`・R2.3）。
/// - `active == true` かつヒット → **現行** `rows[i]`（`i` は [`hit_choice_row`] の
///   返す index）の各フィールドを clone 転写した [`ChoiceSelection`] を返す。
///   `id`/`label`/`references` は現行ヒット行から不透明転写（キャッシュ行からは決して
///   読まない・R2.5/3.2）。`scope` は引数由来（`BalloonWindowMarker.scope`）。
///   `ordinal` はワイヤ形に含めない（漏洩防止・design 2.6）。
///
/// stale 棄却（R3.2）は本関数が**現行 rows のみ**を読むことで成立する: 以前 hover した
/// 座標に現行 rows のどの行も無ければ非ヒット＝`None`、別行が現れていればその現行行から
/// 構成される（キャッシュではなく現行ジオメトリが正本）。
///
/// `#[allow(dead_code)]`: 本番消費（配線層 task 4.2）まで到達者なし——単体檻のみ到達（M1 暫定抑止）。
#[allow(dead_code)]
pub(crate) fn click_selection(
    active: bool,
    rows: &[ChoiceHitRow],
    x: f32,
    y: f32,
    scope: usize,
) -> Option<ChoiceSelection> {
    // 非表示中は発行しない（hit 判定より前に短絡・消滅時 stale／原子性ガード・R3.1）。
    if !active {
        return None;
    }
    // 現行 rows のヒット判定を再利用（非ヒットは None・R2.3）。stale 棄却は現行 rows のみを
    // 読むことで自然に成立する（キャッシュ行は参照しない・R2.5/3.2）。
    let i = hit_choice_row(rows, x, y)?;
    let hit = &rows[i];
    // 現行ヒット行から不透明転写（id/label/references は clone・scope は arg・ordinal 非含有）。
    Some(ChoiceSelection {
        id: hit.id.clone(),
        label: hit.label.clone(),
        scope,
        references: hit.references.clone(),
    })
}

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
/// **物理 px**・i32）を `as f32` で（**スケール係数を掛けず**＝k=1.0 素通し・R4.2/8.6・DD-IE-10 整合）
/// 純関数核へ渡し、hover 遷移を上流 `TextLayerRuntime` へ注入する。moved は非侵襲ゆえ**常に `false`**。
///
/// **借用規律（固定順序・design §204）**:
/// 1. `Emo2Wiring` 共有借用→`runtime()` アクセサで `Rc` clone→world 側借用解放。
///    `Emo2Wiring` 不在（boot 前／失敗）は**正常縮退**＝`debug!`＋no-op（donor presenter=None 同型・R4.1）。
/// 2. `BalloonWiring` から `last_injected` を copy（共有借用即解放）。`BalloonWiring` 不在は結線漏れ＝
///    **構成異常** `error!(event = "balloon_wiring_missing")`＋no-op（配線存在檻が開発時に検出）。
/// 3. runtime `try_borrow`（不変）でスナップショット（`choice_active`＋現行 `choice_hit_rows` を純関数
///    評価・move はここで完結）。`try_borrow` 失敗は構成異常 `error!(event = "balloon_runtime_borrow_failed")`。
/// 4. 借用解放後に runtime `try_borrow_mut` で `inject_choice_hover`（`Inject` アームのみ）。
/// 5. `BalloonWiring` 可変借用で自前 `hover` 更新。
///
/// `RefCell` は `try_borrow`／`try_borrow_mut` を用い、失敗時は `error!`＋no-op（panic しない・log-first）。
/// hover 遷移注入時は `debug!(event = "choice_hover_inject")` を発行する（DD-CI-7・トラブルシュート用）。
/// クリック確定・`send`・`info!` は本ハンドラの範囲外（押下ハンドラ＝task 4.2）。
///
/// `#[allow(dead_code)]`: 本番結線（`attach_balloon_pointer_handlers`＝task 6.1）まで到達者なし——
/// 単体檻のみ到達（M1 暫定抑止）。
#[allow(dead_code)]
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

    // (2) client 物理 px（i32）を f32 へ——スケール係数を掛けない（k=1.0 素通し・R4.2/8.6・DD-IE-10）。
    let x = state.client_point.x as f32;
    let y = state.client_point.y as f32;

    // ── 借用規律 ① Emo2Wiring 共有借用→runtime() で Rc clone→world 側借用解放 ─────────────────
    // Emo2Wiring 不在（boot 前／失敗）は正常縮退＝debug!＋no-op（donor presenter=None 同型・R4.1）。
    let Some(runtime) = world
        .get_non_send_resource::<Emo2Wiring>()
        .map(|w| Rc::clone(w.runtime()))
    else {
        tracing::debug!(
            event = "choice_moved_no_emo2",
            scope,
            "Emo2Wiring 不在（boot 前／失敗）: hover 追従を no-op 縮退"
        );
        return false;
    };

    // ── 借用規律 ② BalloonWiring から last_injected を copy（共有借用即解放）─────────────────────
    // BalloonWiring 不在は結線漏れ＝構成異常 error!（配線存在檻が開発時に検出）＋no-op。
    let Some(last_injected) = world
        .get_non_send_resource::<BalloonWiring>()
        .map(|bw| bw.hover(scope))
    else {
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
                .get_non_send_resource_mut::<BalloonWiring>()
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
                .get_non_send_resource_mut::<BalloonWiring>()
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

/// バルーン窓のポインタ押下ハンドラ（Bubble のみ処理・確定クリック発行・R2.1/2.3/2.4/2.5/2.6/3.1/3.2/4.2/5.1/8.4）。
///
/// wintf `PointerEventHandler` 署名（移動ハンドラの鏡写し）。**Bubble 相のみ処理し Tunnel は伝播続行の
/// ため即 `false`**。**左シングルクリック限定**＝`state.left_down` のみを確定として扱い、`double_click`
/// フィールドは**一切参照しない**（DBLCLK 2 打目も独立 press として扱う・DD-CI-9）。右・中ボタン down は
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
/// 3. runtime `try_borrow`（不変）でスナップショット——`choice_active`＋**現行** `choice_hit_rows` を純関数
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
/// `RefCell` は `try_borrow` のみ（panic しない・log-first）。座標は物理 px 素通し（k=1.0・R4.2/8.6）。
///
/// **戻り値**: `ChoiceSelection` を発行したときのみ `true`（棄却・縮退・非左押下・Tunnel 時は `false`）。
///
/// `#[allow(dead_code)]`: 本番結線（`attach_balloon_pointer_handlers`＝task 6.1）まで到達者なし——
/// 単体檻のみ到達（M1 暫定抑止）。
#[allow(dead_code)]
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
    // false 素通し。double_click フィールドは参照しない（DBLCLK 2 打目も独立 press 扱い・DD-CI-9）。
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

    // client 物理 px（i32）を f32 へ——スケール係数を掛けない（k=1.0 素通し・R4.2/8.6・DD-IE-10）。
    let x = state.client_point.x as f32;
    let y = state.client_point.y as f32;

    // ── 借用規律 ① Emo2Wiring 共有借用→runtime() で Rc clone→world 側借用解放 ─────────────────
    // Emo2Wiring 不在（boot 前／失敗）は正常縮退＝debug!＋no-op（donor presenter=None 同型・R4.1）。
    let Some(runtime) = world
        .get_non_send_resource::<Emo2Wiring>()
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
    if world.get_non_send_resource::<BalloonWiring>().is_none() {
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
            let active = rt.choice_active(&actor);
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
    match selection {
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
                .get_non_send_resource::<BalloonWiring>()
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
    }
}

// ---------------------------------------------------------------------------
// clear_balloon_hover_on_leave 排他システム（tasks.md task 5・design「clear_balloon_hover_on_leave」）
//
// 窓外離脱（WM_MOUSELEAVE）時の hover 解除——`dispatch_pointer_events` は OnPointerExited/Entered を
// 配送しないため、離脱の hover-clear は `PointerLeave` マーカーを読む薄い排他システムが担う（design
// Existing Architecture Analysis point 1・WM_MOUSELEAVE bullet）。判断分岐そのものは純関数
// `hover_action` へ集約済みで、本システムは leave 対象選別（balloon 所有チェック）→snapshot→純関数→
// 適用のみを行う（自前描画なし・R1.6）。
// ---------------------------------------------------------------------------

/// バルーン窓外離脱時の hover 解除（排他システム・R1.3/3.4）。
///
/// Input スケジュールの `dispatch_pointer_events` 後・FrameFinalize（`PointerLeave` クリア）前に実行
/// される排他システム。`PointerLeave` マーカー保持 entity のうち、所有窓（wintf `find_owner_window` の
/// 親チェーン走査）が `BalloonWindowMarker` を持つものだけを対象に scope を解決し、既存純関数
/// `hover_action(active, None, last_injected)` を再利用して hover 状態を解除する（hit は `None`＝ポインタが
/// 窓外へ出た高速離脱ゆえエッジ非採取・R1.3）。
///
/// **借用規律・縮退はハンドラ（task 4.1）と同一**:
/// - `Emo2Wiring` 不在（boot 前／失敗）は**正常縮退**＝`debug!(event = "choice_leave_no_emo2")`＋no-op
///   （donor presenter=None 同型・R4.1）。runtime `Rc` は 1 度 clone し全 scope で共有する（同一資源）。
/// - `BalloonWiring` 不在は結線漏れ＝**構成異常** `error!(event = "balloon_wiring_missing")`＋skip。
/// - runtime `try_borrow`（不変）で `choice_active` を控え、借用解放後に `try_borrow_mut` で
///   `inject_choice_hover`（`Inject` アームのみ・値は必ず `None`）。`RefCell` 借用失敗は
///   `error!(event = "balloon_runtime_borrow_failed")`＋skip（panic しない・log-first）。
/// - `Inject(None)`（表示中・注入済→解除）: `inject_choice_hover(actor, None)`＋自前状態 `None`＋
///   `debug!(event = "choice_hover_inject")`。`ResetOwnState`（非表示・注入済）: 自前状態のみ `None`・
///   inject はしない（上流原子性が正本・R3.4）。`Keep`／`NoopInactive`: 何もしない。
///
/// `PointerLeave` の除去は行わない（除去は既存 FrameFinalize `clear_transient_pointer_state` の責務——
/// 機構不変）。マーカー不在フレーム・バルーン所有 leave 皆無フレームは**完全 no-op**（design Risks）。
///
/// `#[allow(dead_code)]`: スケジュール登録（`main.rs`・task 6.1）まで本番到達者なし——単体檻のみ到達
/// （M1 暫定抑止）。
#[allow(dead_code)]
pub(crate) fn clear_balloon_hover_on_leave(world: &mut World) {
    // (1) PointerLeave マーカー保持 entity を収集（query→即 collect で World 可変借用と分離）。
    let leaving: Vec<Entity> = world
        .query_filtered::<Entity, With<PointerLeave>>()
        .iter(world)
        .collect();
    if leaving.is_empty() {
        // マーカー不在フレームは完全 no-op（design Risks・Emo2Wiring にも触れない）。
        return;
    }

    // (2) 所有窓（find_owner_window の親チェーン走査）が BalloonWindowMarker を持つ leave のみを対象に
    // scope を解決する。非バルーン窓（marker 不在）・所有窓不在の leave は無視（key assertion）。
    // 複数 entity が同一バルーン窓へ写る場合は scope で dedup（1 scope につき高々 1 回処理）。
    let mut scopes: Vec<usize> = Vec::new();
    for e in leaving {
        if let Some(win) = find_owner_window(world, e) {
            if let Some(scope) = world.get::<BalloonWindowMarker>(win).map(|m| m.scope) {
                if !scopes.contains(&scope) {
                    scopes.push(scope);
                }
            }
        }
    }
    if scopes.is_empty() {
        // バルーン所有 leave が皆無なら完全 no-op（非バルーン leave は無視・key assertion）。
        return;
    }

    // ── 借用規律 ① Emo2Wiring 共有借用→runtime() で Rc clone→world 側借用解放 ─────────────────
    // Emo2Wiring 不在（boot 前／失敗）は正常縮退＝debug!＋no-op（donor presenter=None 同型・R4.1）。
    // runtime は単一グローバル資源ゆえ 1 度 clone して全 scope で共有する（ハンドラの per-event clone と
    // 等価・per-scope 再取得は不要）。
    let Some(runtime) = world
        .get_non_send_resource::<Emo2Wiring>()
        .map(|w| Rc::clone(w.runtime()))
    else {
        tracing::debug!(
            event = "choice_leave_no_emo2",
            "Emo2Wiring 不在（boot 前／失敗）: バルーン離脱 hover 解除を no-op 縮退"
        );
        return;
    };

    // (3) 各バルーン scope へハンドラ（task 4.1）と同一の借用規律・縮退で hover 解除を適用する。
    for scope in scopes {
        let actor = ActorKey::from(scope.to_string());

        // ── 借用規律 ② BalloonWiring から last_injected を copy（共有借用即解放）─────────────────
        // BalloonWiring 不在は結線漏れ＝構成異常 error!（配線存在檻が開発時に検出）＋skip。
        let Some(last_injected) = world
            .get_non_send_resource::<BalloonWiring>()
            .map(|bw| bw.hover(scope))
        else {
            tracing::error!(
                event = "balloon_wiring_missing",
                scope,
                "BalloonWiring 不在（結線漏れ）: バルーン離脱 hover 解除を no-op 縮退"
            );
            continue;
        };

        // ── 借用規律 ③ runtime 不変借用で choice_active スナップショット（hit は None＝窓外離脱・R1.3）──
        // try_borrow 失敗（理論上不到達の構成異常）は error!＋skip（panic しない・log-first）。
        let action = match runtime.try_borrow() {
            Ok(rt) => hover_action(rt.choice_active(&actor), None, last_injected),
            Err(_) => {
                tracing::error!(
                    event = "balloon_runtime_borrow_failed",
                    scope,
                    "runtime try_borrow 失敗（離脱・active スナップショット）: no-op 縮退"
                );
                continue;
            }
        };
        // ここで不変借用（Ref）は解放済み——④ の可変借用と同時に持たない。

        // 純関数の決定を適用する（hit=None ゆえ Inject は常に None＝ハイライト解除・R1.3）。
        match action {
            // 非表示・未注入／同値既注入（None==None）は何もしない。
            HoverAction::NoopInactive | HoverAction::Keep => {}
            // 消滅時整合（R3.4）: 自前状態のみ None 整合・inject はしない（上流原子性が正本）。
            HoverAction::ResetOwnState => {
                let mut bw = world
                    .get_non_send_resource_mut::<BalloonWiring>()
                    .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）");
                bw.set_hover(scope, None);
            }
            // 離脱遷移（R1.3）: ④ runtime 可変借用で inject(None)→⑤ BalloonWiring 自前状態を None 更新。
            HoverAction::Inject(value) => {
                // hit=None ゆえ value は必ず None（ハイライト解除注入・描画 API は呼ばない）。
                match runtime.try_borrow_mut() {
                    Ok(mut rt) => rt.inject_choice_hover(&actor, value),
                    Err(_) => {
                        tracing::error!(
                            event = "balloon_runtime_borrow_failed",
                            scope,
                            "runtime try_borrow_mut 失敗（離脱 inject）: no-op 縮退"
                        );
                        continue;
                    }
                }
                // ⑤ 可変借用は上で解放済み。BalloonWiring の自前 last-injected を更新する。
                let mut bw = world
                    .get_non_send_resource_mut::<BalloonWiring>()
                    .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）");
                bw.set_hover(scope, value);
                // hover 遷移注入の marker（DD-CI-7・トラブルシュート用・info ではない）。
                tracing::debug!(
                    event = "choice_hover_inject",
                    scope,
                    ordinal = ?value,
                    "窓外離脱で hover 解除を上流 runtime へ注入"
                );
            }
        }
    }
    // PointerLeave は除去しない（除去は FrameFinalize clear_transient_pointer_state の責務・機構不変）。
}

// ---------------------------------------------------------------------------
// post-spawn 装着・NonSend 結線・スケジュール登録（tasks.md task 6.1・design
// 「attach_balloon_pointer_handlers / wire_balloon_choice」＋「clear_balloon_hover_on_leave」Validation）
//
// donor `attach_char_pointer_handlers`（input_events/mod.rs）／`wire_mouse_input`（同）／main.rs の
// clickthrough 登録スロットの鏡写し。`spawn.rs` は不改変＝post-spawn 装着のみ（4.4）。上流 input-events
// 成果（ハンドラ／排他システム／資源型）を消費し結線するだけで判断分岐は増設しない（5.5）。本番呼出は
// main.rs シーム（task 6.2）——`wire_mouse_input` と同型に schedule 実行外で 1 回同期呼出する。
// ---------------------------------------------------------------------------

/// `BalloonWindowMarker` 全窓へ `OnPointerMoved`＋`OnPointerPressed` を post-spawn 挿入する
/// （donor `attach_char_pointer_handlers` の鏡写し・spawn.rs 不改変・R4.3/4.4）。
///
/// 前提: `spawn_ghost_windows` 完了後（`BalloonWindowMarker` 窓が存在する状態）に同一 `&mut World`
/// クロージャ内で呼ぶ（キャラ窓ハンドラ装着と同型のタイミング契約・main.rs の spawn 直後結線）。
/// `&mut World` 借用中はクエリで別の可変借用を取れないため、まず対象 entity を収集してから 1 件ずつ
/// 挿入する（donor 同型）。標的は `BalloonWindowMarker` 窓のみ——キャラ窓・その他 entity は一切
/// 触らない（配線の非退行・R4.3）。
///
/// `#[allow(dead_code)]`: 本番結線（main.rs＝task 6.2）まで到達者なし——配線存在檻のみ到達（M1 暫定抑止）。
#[allow(dead_code)]
pub(crate) fn attach_balloon_pointer_handlers(world: &mut World) {
    let balloon_windows: Vec<Entity> = world
        .query_filtered::<Entity, With<BalloonWindowMarker>>()
        .iter(world)
        .collect();
    for e in balloon_windows {
        world.entity_mut(e).insert((
            OnPointerMoved(on_balloon_pointer_moved),
            OnPointerPressed(on_balloon_pointer_pressed),
        ));
    }
}

/// mpsc チャネルを生成し `BalloonWiring`＋`ChoiceSelectionInbox` を NonSend 挿入し、さらに
/// `clear_balloon_hover_on_leave` を Input スケジュール（`dispatch_pointer_events` 後）へ登録する
/// （donor `wire_mouse_input`＋main.rs clickthrough 登録の合成・design Option A・R5.5/6.6）。
///
/// main.rs シーム（task 6.2）から `wire_mouse_input` と同型に **1 回・同期**（schedule 実行外）で
/// 呼ばれる。同期呼出ゆえ実行中スケジュールを触らず、`Schedules` 資源が既在の World（`EcsWorld` 内
/// World）で成立する（donor clickthrough 登録＝`app.world().add_systems(...)` と同じ作法）。発行シンク
/// `ChoiceSelectionInbox` は下流 W6 `choice-select-events` が受信処理へ置換する seam（5.3）。
///
/// `#[allow(dead_code)]`: 本番呼出（main.rs＝task 6.2）まで到達者なし——結線檻のみ到達（M1 暫定抑止）。
#[allow(dead_code)]
pub(crate) fn wire_balloon_choice(world: &mut World) {
    let (tx, rx) = channel::<ChoiceSelection>();
    world.insert_non_send_resource(BalloonWiring::new(tx));
    world.insert_non_send_resource(ChoiceSelectionInbox(rx));
    register_balloon_leave_system(world);
}

/// `clear_balloon_hover_on_leave` を Input スケジュール（`dispatch_pointer_events` 後）へ登録する
/// （main.rs clickthrough 登録スロットの donor 同型・design Integration Test 7・R6.6）。
///
/// 高速離脱時の hover 残置は登録漏れとして実機目視でしか検出できないため、登録は本関数に集約し
/// スケジュール登録檻が開発時に捕捉する（design Testing Strategy Integration Test 7）。ordering は
/// `dispatch_pointer_events` の後（FrameFinalize の `clear_transient_pointer_state` による `PointerLeave`
/// 除去より前は Input スケジュール内であることで成立）。`wire_balloon_choice` から呼ばれる私有ヘルパ。
///
/// `#[allow(dead_code)]`: 呼び手 `wire_balloon_choice` の本番到達が task 6.2 のため間接に未到達（M1 暫定抑止）。
#[allow(dead_code)]
fn register_balloon_leave_system(world: &mut World) {
    world.resource_mut::<Schedules>().add_systems(
        Input,
        clear_balloon_hover_on_leave.after(dispatch_pointer_events),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;
    use std::sync::mpsc;

    fn sample() -> ChoiceSelection {
        ChoiceSelection {
            id: "q0".to_string(),
            label: "はい".to_string(),
            scope: 0,
            references: vec!["ref0".to_string(), "ref1".to_string()],
        }
    }

    #[test]
    fn identical_field_contents_compare_equal() {
        let a = sample();
        let b = sample();
        assert_eq!(a, b, "同一フィールド内容の ChoiceSelection は等価であるべき");
    }

    #[test]
    fn differing_field_contents_compare_unequal() {
        let base = sample();

        let mut different_id = sample();
        different_id.id = "q1".to_string();
        assert_ne!(base, different_id, "id が異なれば非等価であるべき");

        let mut different_refs = sample();
        different_refs.references = vec!["ref0".to_string()];
        assert_ne!(base, different_refs, "references が異なれば非等価であるべき");
    }

    #[test]
    fn clone_equals_original_and_debug_is_usable() {
        let original = sample();
        let cloned = original.clone();
        assert_eq!(original, cloned, "clone は元と等価であるべき（Clone 導出の証跡）");

        let rendered = format!("{original:?}");
        assert!(!rendered.is_empty(), "Debug 出力は非空であるべき（Debug 導出の証跡）");
    }

    /// NonSend 挿入＋シーム観測檻（2.2・design「NonSend 資源」）: `BalloonWiring` を `World` へ
    /// **NonSend 挿入**でき、`ChoiceSelectionInbox` の `Receiver` 経由で発行シンクへ送った
    /// `ChoiceSelection` を一度だけ観測できる（送信値と等価・2 度目は `Empty`）。
    ///
    /// mpsc `Sender`/`Receiver` は `!Sync` ゆえ NonSend 資源として挿入する（`insert_non_send_resource`）。
    /// 受信処理は M1 未消費（下流 W6 `choice-select-events` が置換する seam・5.3）。ここでは
    /// 発行が mpsc 上で観測できることのみを固定し、`resolve_choice` は一切呼ばない（5.4）。
    #[test]
    fn wiring_inserts_non_send_and_selection_observed_via_inbox() {
        let (tx, rx) = mpsc::channel::<ChoiceSelection>();
        let wiring = BalloonWiring::new(tx);

        let mut world = World::new();
        world.insert_non_send_resource(wiring);
        world.insert_non_send_resource(ChoiceSelectionInbox(rx));

        assert!(
            world.get_non_send_resource::<BalloonWiring>().is_some(),
            "BalloonWiring は NonSend 挿入されている"
        );

        // 発行シンク経由で ChoiceSelection を送る。
        let sel = sample();
        let sent = world
            .get_non_send_resource::<BalloonWiring>()
            .expect("直上で存在確認済み")
            .send_selection(sel.clone());
        assert!(sent, "Receiver 生存中の発行は成功する（Err にならない・5.3）");

        // seam の Receiver 経由で送信値を観測する。
        let inbox = world
            .get_non_send_resource::<ChoiceSelectionInbox>()
            .expect("ChoiceSelectionInbox は NonSend 挿入されている");
        let received = inbox.0.try_recv().expect("発行した ChoiceSelection が届く");
        assert_eq!(received, sel, "受信値は送信値と等価（task 2.1 の PartialEq 再利用）");
        assert!(
            inbox.0.try_recv().is_err(),
            "発行は一度きり（2 度目の try_recv は Empty・2.4）"
        );
    }

    /// hover 自前追跡（B-2・R3.4）: `set_hover`/`hover` で scope 別の last-injected ordinal を
    /// 記録・回収でき、消滅時整合の `None` 上書きが反映される。
    #[test]
    fn hover_tracks_last_injected_ordinal_per_scope() {
        let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
        let mut wiring = BalloonWiring::new(tx);

        assert_eq!(wiring.hover(0), None, "未注入 scope の hover は None");

        wiring.set_hover(0, Some(2));
        wiring.set_hover(1, Some(5));
        assert_eq!(wiring.hover(0), Some(2), "scope 0 の last-injected を回収");
        assert_eq!(wiring.hover(1), Some(5), "scope 1 は独立に保持");

        // 消滅時整合（R3.4）: None 上書きで自前状態を整える。
        wiring.set_hover(0, None);
        assert_eq!(wiring.hover(0), None, "None 上書きが反映される");
        assert_eq!(wiring.hover(1), Some(5), "他 scope は影響を受けない");
    }

    // -------------------------------------------------------------------------
    // hit_choice_row 純関数檻（task 3.1・design「純関数判定核」／Testing Strategy item 1/4）
    //
    // 上流実型 `ChoiceHitRow`／`HitRectPx`（全 pub フィールド）で fixture を組み、
    // 包含／境界（半開）／行外／空／複数行／病的重なり／座標素通し（DPI 非適用）を
    // GPU・実窓不要で決定的に判定する（R1.1/1.5/2.3/4.2/5.2/8.6）。
    // -------------------------------------------------------------------------

    use areka_emo_text::actor::HitRectPx;

    /// 窓物理 px の行矩形を持つ `ChoiceHitRow` を組む（ordinal は入力順昇順を模す）。
    /// rect 以外のフィールドは 3.1 の判定に無関係——不透明転写の placeholder。
    fn row(ordinal: usize, left: f32, top: f32, right: f32, bottom: f32) -> ChoiceHitRow {
        ChoiceHitRow {
            ordinal,
            id: format!("q{ordinal}"),
            label: format!("label{ordinal}"),
            references: Vec::new(),
            rect: HitRectPx {
                left,
                top,
                right,
                bottom,
            },
        }
    }

    /// 内側包含（R1.1）: 矩形の内部点はその行 index を返す。
    #[test]
    fn hit_inside_rect_returns_index() {
        let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];
        assert_eq!(
            hit_choice_row(&rows, 30.0, 30.0),
            Some(0),
            "矩形内部の点はヒット index を返す"
        );
    }

    /// 空 rows（R2.3）: 判定対象が無ければ常に None。
    #[test]
    fn hit_empty_rows_returns_none() {
        let rows: [ChoiceHitRow; 0] = [];
        assert_eq!(hit_choice_row(&rows, 30.0, 30.0), None, "空 rows は None");
    }

    /// 行外（R2.3・非ヒット→None）: 矩形外の点は None。四方向すべて外側を確認。
    #[test]
    fn hit_outside_rect_returns_none() {
        let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];
        assert_eq!(hit_choice_row(&rows, 5.0, 30.0), None, "左外");
        assert_eq!(hit_choice_row(&rows, 60.0, 30.0), None, "右外");
        assert_eq!(hit_choice_row(&rows, 30.0, 10.0), None, "上外");
        assert_eq!(hit_choice_row(&rows, 30.0, 50.0), None, "下外");
    }

    /// 半開区間の境界（design Testing Strategy item 1）: `left`/`top` 辺は包含・
    /// `right`/`bottom` 辺は非包含（whole-pixel 行矩形と整合）。
    #[test]
    fn hit_half_open_boundary_edges() {
        let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];

        // left/top 辺は包含。
        assert_eq!(hit_choice_row(&rows, 10.0, 30.0), Some(0), "left 辺は包含");
        assert_eq!(hit_choice_row(&rows, 30.0, 20.0), Some(0), "top 辺は包含");
        assert_eq!(hit_choice_row(&rows, 10.0, 20.0), Some(0), "左上角は包含");

        // right/bottom 辺は非包含。
        assert_eq!(hit_choice_row(&rows, 50.0, 30.0), None, "right 辺は非包含");
        assert_eq!(hit_choice_row(&rows, 30.0, 40.0), None, "bottom 辺は非包含");
        assert_eq!(hit_choice_row(&rows, 50.0, 40.0), None, "右下角は非包含");
    }

    /// 複数の非重複行のうち正しい行（design Testing Strategy item 1）: 点を含む行の
    /// index が返る。各行が独立に判定される。
    #[test]
    fn hit_multiple_non_overlapping_rows_returns_correct_index() {
        let rows = [
            row(0, 0.0, 0.0, 100.0, 20.0),   // index 0
            row(1, 0.0, 20.0, 100.0, 40.0),  // index 1
            row(2, 0.0, 40.0, 100.0, 60.0),  // index 2
        ];
        assert_eq!(hit_choice_row(&rows, 50.0, 10.0), Some(0), "1 行目内");
        assert_eq!(hit_choice_row(&rows, 50.0, 30.0), Some(1), "2 行目内");
        assert_eq!(hit_choice_row(&rows, 50.0, 50.0), Some(2), "3 行目内");
        assert_eq!(hit_choice_row(&rows, 50.0, 70.0), None, "全行外");
    }

    /// 病的重なり→最終一致（R1.5・DD-CI-5）: 点を含む行が複数あっても、逆順走査の最初の
    /// 一致＝スライス**最終** index を決定的に返す（後定義が手前・画家のアルゴリズム）。
    /// 期待 index を各行で一意にするため、重なり方を非対称にして「最初一致」なら別 index に
    /// なる配置を用いる（最初一致=1／最終一致=2 を弁別）。
    #[test]
    fn hit_pathological_overlap_returns_last_match_deterministically() {
        // 3 行が (50, 30) を共通に含む。スライス順（index 昇順）で最後の index 2 が返るべき。
        let rows = [
            row(0, 0.0, 0.0, 100.0, 100.0),  // index 0: 点を含む
            row(1, 40.0, 20.0, 60.0, 40.0),  // index 1: 点を含む
            row(2, 45.0, 25.0, 80.0, 60.0),  // index 2: 点を含む（最後定義＝手前）
        ];
        assert_eq!(
            hit_choice_row(&rows, 50.0, 30.0),
            Some(2),
            "重なり時はスライス最終一致 index（逆順走査の最初一致）"
        );

        // 2 行重なりでも最終一致（index 1）が返る——最初一致 index 0 と弁別。
        let two = [
            row(0, 0.0, 0.0, 100.0, 100.0), // index 0: 点を含む
            row(1, 10.0, 10.0, 90.0, 90.0), // index 1: 点を含む（最後定義）
        ];
        assert_eq!(
            hit_choice_row(&two, 50.0, 50.0),
            Some(1),
            "2 行重なりでも最終一致 index を返す（最初一致 0 ではない）"
        );
    }

    /// 座標素通し＝DPI 非適用（design Testing Strategy item 4・R4.2／8.6）: k=1.0 では
    /// ヒットする点が、もし DPI スケール（>1）を座標へ掛けていれば矩形外へ出て miss する
    /// 配置を用い、実際には**スケールを掛けず**ヒットすることを固定する（falsifying fixture）。
    #[test]
    fn hit_coordinate_passthrough_no_dpi_scaling() {
        // 矩形は右下に離れた帯。点は k=1.0 でその内側。
        let rows = [row(0, 100.0, 100.0, 140.0, 120.0)];
        let (x, y) = (110.0_f32, 110.0_f32);

        // k=1.0 素通し: ヒットする。
        assert_eq!(
            hit_choice_row(&rows, x, y),
            Some(0),
            "物理 px 素通し（k=1.0）ではヒットする"
        );

        // もし DPI スケール（例 1.5）を座標へ掛けていたら (165, 165) となり矩形外＝miss。
        // 実装がスケールを掛けていないことを、スケール後座標が miss になることで補強する。
        let scale = 1.5_f32;
        assert_eq!(
            hit_choice_row(&rows, x * scale, y * scale),
            None,
            "スケールを掛けた座標なら矩形外（＝実装はスケールを掛けていない反証 fixture）"
        );
    }

    // -------------------------------------------------------------------------
    // hover_action 純関数檻（task 3.2・design「純関数判定核」／Observable 全分岐）
    //
    // active・hit_ordinal・last_injected の 3 入力から hover 遷移を決める純関数の
    // 全分岐（非表示時無処理／消滅時自前整合／同値維持／新規注入／解除注入）を
    // World・runtime 不要で決定的に網羅する（R1.2/1.3/1.4/3.4）。
    // -------------------------------------------------------------------------

    /// 非表示時無処理（R1.4）: `active == false` かつ `last_injected == None` は
    /// 何もしない。hit_ordinal（Some/None いずれも）は無視される。
    #[test]
    fn hover_inactive_no_prior_injection_is_noop() {
        assert_eq!(
            hover_action(false, None, None),
            HoverAction::NoopInactive,
            "非表示・未注入・hit なしは NoopInactive"
        );
        assert_eq!(
            hover_action(false, Some(3), None),
            HoverAction::NoopInactive,
            "非表示・未注入は hit があっても NoopInactive（hit_ordinal 無視・R1.4）"
        );
    }

    /// 消滅時自前整合（R3.4）: `active == false` かつ `last_injected == Some(k)` は
    /// 自前状態を None 整合する `ResetOwnState`（inject はしない＝上流原子性が正本）。
    /// hit_ordinal（Some/None いずれも）は無視される。
    #[test]
    fn hover_inactive_with_prior_injection_resets_own_state() {
        assert_eq!(
            hover_action(false, None, Some(2)),
            HoverAction::ResetOwnState,
            "非表示・注入済・hit なしは ResetOwnState（R3.4）"
        );
        assert_eq!(
            hover_action(false, Some(5), Some(2)),
            HoverAction::ResetOwnState,
            "非表示・注入済は hit があっても ResetOwnState（hit_ordinal 無視・R3.4）"
        );
    }

    /// 同値維持（遷移なし）: 表示中で hit_ordinal == last_injected は再注入しない Keep。
    /// Some==Some と None==None の双方を確認する。
    #[test]
    fn hover_active_same_value_keeps() {
        assert_eq!(
            hover_action(true, Some(2), Some(2)),
            HoverAction::Keep,
            "表示中・同一 Some は Keep（再注入しない）"
        );
        assert_eq!(
            hover_action(true, None, None),
            HoverAction::Keep,
            "表示中・None 同値は Keep（ハイライト無しのまま遷移なし）"
        );
    }

    /// 新規注入（R1.2）: 表示中で hover 対象が変化したら新値を Inject する。
    /// None→Some の初回注入と Some→Some の遷移の双方を確認する。
    #[test]
    fn hover_active_new_row_injects() {
        assert_eq!(
            hover_action(true, Some(1), None),
            HoverAction::Inject(Some(1)),
            "表示中・None→Some(1) は Inject(Some(1))（初回ハイライト・R1.2）"
        );
        assert_eq!(
            hover_action(true, Some(3), Some(1)),
            HoverAction::Inject(Some(3)),
            "表示中・Some(1)→Some(3) は Inject(Some(3))（行遷移・R1.2）"
        );
    }

    /// 解除注入（R1.3）: 表示中で hover が行外へ出たら None を Inject する
    /// （Some→None＝ハイライト解除の遷移）。
    #[test]
    fn hover_active_leaves_row_injects_none() {
        assert_eq!(
            hover_action(true, None, Some(2)),
            HoverAction::Inject(None),
            "表示中・Some(2)→None は Inject(None)（ハイライト解除・R1.3）"
        );
    }

    // -------------------------------------------------------------------------
    // click_selection 純関数檻（task 3.3・design「純関数判定核」／R2.1/2.2/2.3/3.1/3.2）
    //
    // active・現行 rows・click 座標・scope の入力から確定 ChoiceSelection の構成を
    // 決める純関数を、World・runtime・send 不要で決定的に判定する。
    // ヒット時のフィールド一致（scope は arg 由来・ordinal 非含有）、非 hit／非表示／
    // stale 行での非構成（None）を網羅する（R2.1/2.2/2.3/3.1/3.2/6.2/6.3）。
    // -------------------------------------------------------------------------

    /// `references` を明示指定できる `ChoiceHitRow` を組む（転写忠実性の検証用）。
    fn row_with_refs(
        ordinal: usize,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        references: Vec<String>,
    ) -> ChoiceHitRow {
        ChoiceHitRow {
            ordinal,
            id: format!("q{ordinal}"),
            label: format!("label{ordinal}"),
            references,
            rect: HitRectPx {
                left,
                top,
                right,
                bottom,
            },
        }
    }

    /// ヒット確定（R2.1/2.2）: 表示中・ヒット座標では現行ヒット行の全フィールドを
    /// clone 転写した `ChoiceSelection` を返す。scope は arg 由来・ordinal は非含有。
    #[test]
    fn click_hit_builds_selection_from_current_row() {
        let rows = [
            row_with_refs(0, 0.0, 0.0, 100.0, 20.0, vec!["a".to_string()]),
            row_with_refs(
                1,
                0.0,
                20.0,
                100.0,
                40.0,
                vec!["r0".to_string(), "r1".to_string()],
            ),
        ];
        // (50, 30) は index 1 の行内。
        let sel = click_selection(true, &rows, 50.0, 30.0, 7)
            .expect("表示中・ヒット座標では ChoiceSelection を構成する");
        assert_eq!(sel.id, "q1", "id は現行ヒット行から転写");
        assert_eq!(sel.label, "label1", "label は現行ヒット行から転写");
        assert_eq!(sel.scope, 7, "scope は引数由来（BalloonWindowMarker.scope）");
        assert_eq!(
            sel.references,
            vec!["r0".to_string(), "r1".to_string()],
            "references は現行ヒット行から忠実転写"
        );
    }

    /// 非ヒット→非発行（R2.3）: 表示中でも全矩形外の座標なら None。
    #[test]
    fn click_non_hit_returns_none() {
        let rows = [row_with_refs(0, 10.0, 20.0, 50.0, 40.0, Vec::new())];
        assert_eq!(
            click_selection(true, &rows, 5.0, 5.0, 0),
            None,
            "全矩形外の click は非構成（None・R2.3）"
        );
    }

    /// 非表示中は非発行（R3.1）: `active == false` は、たとえ矩形内座標でも None。
    /// hit 判定より前に短絡する（消滅時の stale／原子性ガード）。
    #[test]
    fn click_inactive_returns_none_even_inside_rect() {
        let rows = [row_with_refs(0, 10.0, 20.0, 50.0, 40.0, Vec::new())];
        // (30, 30) は矩形内だが active=false ゆえ None。
        assert_eq!(
            click_selection(false, &rows, 30.0, 30.0, 0),
            None,
            "非表示中（active=false）は矩形内でも非構成（None・R3.1）"
        );
    }

    /// stale 行棄却（R3.2/6.3）: 以前ヒットしていた座標に、現行 rows ではどの行も
    /// 存在しない（レイアウト差替後）場合、同座標の click は None。
    /// 現行ジオメトリのみを読むことを、キャッシュ座標を覆わない現行 rows で固定する。
    #[test]
    fn click_stale_coords_not_in_current_rows_returns_none() {
        // hover 時代には座標 (30, 30) に行があったが、現行 rows はその座標を覆わない
        // （行が消滅／別位置へ差し替わった）。
        let current = [row_with_refs(0, 200.0, 200.0, 240.0, 220.0, Vec::new())];
        assert_eq!(
            click_selection(true, &current, 30.0, 30.0, 0),
            None,
            "現行 rows がクリック座標を覆わなければ stale 棄却で None（R3.2）"
        );
    }

    /// stale 差替（R3.2/2.5）: 同座標に別行が現れた場合、確定は必ず**現行**行から
    /// 構成される（キャッシュではなく現行ジオメトリが正本）。
    #[test]
    fn click_replaced_row_builds_from_current_not_cached() {
        // 現行 rows: 座標 (30, 30) を覆うのは ordinal 9 の別行のみ。
        let current = [row_with_refs(9, 10.0, 20.0, 50.0, 40.0, vec!["z".to_string()])];
        let sel = click_selection(true, &current, 30.0, 30.0, 3)
            .expect("現行行がヒットするので構成される");
        assert_eq!(sel.id, "q9", "確定は現行ヒット行（差替後）から構成される");
        assert_eq!(sel.label, "label9", "label も現行行から");
        assert_eq!(sel.references, vec!["z".to_string()], "references も現行行から");
        assert_eq!(sel.scope, 3, "scope は引数由来");
    }

    /// 空 references の忠実転写: 参照列が空でも空 Vec として構成される。
    #[test]
    fn click_empty_references_transcribed_as_empty() {
        let rows = [row_with_refs(0, 0.0, 0.0, 100.0, 20.0, Vec::new())];
        let sel = click_selection(true, &rows, 50.0, 10.0, 0).expect("ヒットするので構成される");
        assert!(sel.references.is_empty(), "空 references は空 Vec として転写");
    }

    // -------------------------------------------------------------------------
    // on_balloon_pointer_moved 配線層檻（task 4.1・design「配線層 > balloon ハンドラ」／
    // Error Handling／System Flows・R1.1/1.2/1.3/1.4/1.6/3.1/3.3/4.1/4.2/8.4）
    //
    // 合成 `PointerState` を直接ハンドラへ与え、(a) Tunnel 素通し、(b) 資源不在の縮退経路を
    // 正しい**ログレベル**（Emo2Wiring 不在=debug／BalloonWiring 不在=error）で、(c) 適用アーム
    // （Inject／ResetOwnState／Noop）を実 `TextLayerRuntime` で決定的に檻へ入れる。判断分岐そのもの
    // （active×hit×last の全組合せ）は hover_action 純関数檻（task 3.2）が網羅済み。
    //
    // NOTE（runtime constructibility）: `choice_hit_rows` は `present_frame`（GPU）でしか
    // `choice_snapshot` を埋めないため、headless では現行 rows が常に空＝hit=None。ゆえに
    // 「Some(ordinal) ハイライト追従」は本層では実演できず hover_action 純関数檻（3.2）＋
    // task 7.1 の pass-through 檻に委ねる。本檻はハンドラ経由の実注入として Inject(None) 遷移
    // （active かつ hit=None かつ last=Some → ハイライト解除注入）を観測する（inject_choice_hover を
    // 実 runtime へ実際に呼び、BalloonWiring.hover の更新と debug marker を固定する）。
    // -------------------------------------------------------------------------

    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    use areka_emo_atlas::AtlasTable;
    use areka_emo_compose::{BindSet, EmoWorld};
    use areka_emo_present::{EmoPresenter, PresentCommand};
    use areka_emo_text::actor::TextLayerRuntime;
    use areka_emo_text::state::TextLayerConfig;
    use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
    use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;
    use wintf::ecs::Point;
    use wintf::ecs::pointer::{Phase, PointerState};

    use crate::emo2_boot::assets::{BootAssets, LoopTables, ScopeAssets};
    use crate::emo2_boot::frame::Emo2Wiring;
    use crate::emo2_boot::move_cue::MoveDirective;
    use crate::emo2_boot::talk_clock::TalkClock;
    use crate::placement::spawn::BalloonWindowMarker;

    /// 空 `EmoWorld`（空 shell から build・COM/GPU 不要の寛容契約・frame.rs 檻同型）。
    fn empty_world() -> EmoWorld {
        EmoWorld::build(&areka_parsers::shell::parse(""))
    }

    /// 空アトラス（headless 構築・frame.rs 檻同型）。
    fn empty_atlas() -> AtlasTable {
        AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
    }

    /// 合成 `BootAssets`（scope0 の最小形・COM/GPU/fixture 不要の純合成・frame.rs synth_assets 同型）。
    fn synth_boot_assets() -> BootAssets {
        BootAssets {
            shells: vec![ScopeAssets {
                scope: 0,
                emo_world: empty_world(),
                atlas: empty_atlas(),
                initial_surface_id: 0,
            }],
            balloons: vec![(0, empty_world(), empty_atlas())],
            balloon_model: areka_parsers::balloon::parse_str("", None),
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: BindSet::default(),
            bind_resolver: BindResolver::empty(),
            shell_author_dpi: 96,
            balloon_author_dpi: 96,
            loop_tables: LoopTables {
                shell: AnimationTable::empty(),
                balloon: AnimationTable::empty(),
            },
        }
    }

    /// headless な `Emo2Wiring`（実 `EmoPresenter`／与えた runtime／合成 `BootAssets`・COM/GPU 不要）。
    ///
    /// ハンドラが `Emo2Wiring::runtime()` から借りる runtime を、テスト側が事前に populate した実体で
    /// 差し込むための最小結線（frame.rs の headless_wiring_with と同型）。
    fn headless_emo2_wiring(runtime: Rc<RefCell<TextLayerRuntime>>) -> Emo2Wiring {
        Emo2Wiring::new(
            EmoPresenter::new(),
            mpsc::channel::<PresentCommand>().1,
            mpsc::channel::<MoveDirective>().1,
            runtime,
            TalkClock::new(Arc::new(|| 0.0)),
            synth_boot_assets(),
        )
    }

    /// 選択肢スパンを 1 つ載せて `choice_active(actor)==true` にした実 runtime を組む（GPU 不要）。
    /// `choice_hit_rows` は `present_frame`（GPU）未実行ゆえ空のまま（headless の既知制約）。
    fn runtime_with_active_choice(actor: &str) -> Rc<RefCell<TextLayerRuntime>> {
        let rt = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        rt.borrow_mut().apply_cue(&TalkCue {
            at: 0.0,
            actor: ActorKey::from(actor),
            command: CueCommand::Choice {
                id: "OnYes".into(),
                text: "はい".into(),
                references: Vec::new(),
            },
            duration: 0.0,
        });
        rt
    }

    /// Bubble 相の合成 `PointerState`（client 物理 px）を組む（moved ハンドラは double_click/ctrl 非参照）。
    fn bubble_move(x: i32, y: i32) -> Phase<PointerState> {
        Phase::Bubble(PointerState {
            client_point: Point { x, y },
            ..Default::default()
        })
    }

    // ── スレッドローカル tracing capture（frame.rs 檻の最小複製・ログレベル決定論観測用）─────────
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            ev: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let meta = ev.metadata();
            let mut line = format!("level={}", meta.level());
            struct V<'a>(&'a mut String);
            impl Visit for V<'_> {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    use std::fmt::Write;
                    let _ = write!(self.0, " {}={:?}", f.name(), v);
                }
            }
            ev.record(&mut V(&mut line));
            self.0.lock().unwrap().push(line);
        }
    }

    /// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
    fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
        let cap = Capture::default();
        let logs = cap.0.clone();
        let subscriber = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(subscriber, f);
        let guard = logs.lock().unwrap();
        guard.clone()
    }

    /// Tunnel 素通し（R1・donor 同型）: Tunnel 相は資源に一切触れず即 `false`（副作用なし）。
    #[test]
    fn moved_tunnel_phase_is_noop_false() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
        // 資源未挿入でも Tunnel は最初に短絡するため到達しない（副作用なしの担保）。
        let tunnel = Phase::Tunnel(PointerState {
            client_point: Point { x: 10, y: 20 },
            ..Default::default()
        });
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &tunnel),
            "Tunnel 相は即 false（伝播続行・非侵襲）"
        );
    }

    /// Emo2Wiring 不在＝正常縮退（R4.1・donor presenter=None 同型）: **DEBUG** レベルで no-op・`false`。
    #[test]
    fn moved_emo2_absent_degrades_with_debug_not_error() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
        let ev = bubble_move(10, 20);

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_moved(&mut world, e, e, &ev),
                "Emo2Wiring 不在は no-op false"
            );
        });

        assert!(
            logs.iter()
                .any(|l| l.contains("level=DEBUG") && l.contains("choice_moved_no_emo2")),
            "Emo2Wiring 不在は DEBUG で正常縮退する（event=choice_moved_no_emo2）: {logs:?}"
        );
        assert!(
            !logs.iter().any(|l| l.contains("level=ERROR")),
            "Emo2Wiring 不在は構成異常ではない＝ERROR を出さない: {logs:?}"
        );
    }

    /// BalloonWiring 不在＝構成異常（結線漏れ）: Emo2Wiring は present でも **ERROR** で no-op・`false`。
    /// （借用規律の固定順序＝Emo2Wiring→BalloonWiring ゆえ Emo2Wiring present が前提。）
    #[test]
    fn moved_balloon_wiring_absent_degrades_with_error() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
        // Emo2Wiring は present（空 runtime）・BalloonWiring は挿入しない。
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        let ev = bubble_move(10, 20);

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_moved(&mut world, e, e, &ev),
                "BalloonWiring 不在は no-op false"
            );
        });

        assert!(
            logs.iter()
                .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
            "BalloonWiring 不在は ERROR の構成異常（event=balloon_wiring_missing）: {logs:?}"
        );
    }

    /// BalloonWindowMarker 不在（理論上不到達の構成異常）: **ERROR** で no-op・`false`（silent 禁止）。
    #[test]
    fn moved_missing_marker_errors_and_noop() {
        let mut world = World::new();
        let e = world.spawn_empty().id(); // BalloonWindowMarker を持たない entity
        let ev = bubble_move(10, 20);

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_moved(&mut world, e, e, &ev),
                "marker 不在は no-op false"
            );
        });

        assert!(
            logs.iter()
                .any(|l| l.contains("level=ERROR") && l.contains("balloon_marker_missing")),
            "marker 不在は ERROR で縮退（silent failure 禁止）: {logs:?}"
        );
    }

    /// hover 遷移注入（Inject アーム・R1.3）: choice 表示中・現行 hit=None（headless snapshot 空）・
    /// 前回注入 Some(2) → `Inject(None)`。ハンドラが実 runtime へ `inject_choice_hover(actor, None)` を
    /// 呼び、自前状態 `BalloonWiring.hover[scope]` を None へ更新し、`choice_hover_inject` を DEBUG 発火する。
    #[test]
    fn moved_active_choice_transition_injects_and_updates_own_state() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        let runtime = runtime_with_active_choice("0");
        assert!(
            runtime.borrow().choice_active(&ActorKey::from("0")),
            "前提: choice_active=true（選択肢スパンあり）"
        );
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));

        // 前回注入値 Some(2) を仕込む（遷移検出のため・現行 hit=None ゆえ Inject(None) へ遷移）。
        let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
        let mut bw = BalloonWiring::new(tx);
        bw.set_hover(0, Some(2));
        world.insert_non_send_resource(bw);

        let ev = bubble_move(10, 20);
        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_moved(&mut world, e, e, &ev),
                "moved は常に false（非侵襲）"
            );
        });

        assert_eq!(
            world
                .get_non_send_resource::<BalloonWiring>()
                .unwrap()
                .hover(0),
            None,
            "Inject(None) 遷移で BalloonWiring.hover[scope] が None へ更新される（⑤）"
        );
        assert!(
            logs.iter()
                .any(|l| l.contains("level=DEBUG") && l.contains("choice_hover_inject")),
            "hover 遷移注入で choice_hover_inject を DEBUG 発火（DD-CI-7）: {logs:?}"
        );
        // 実 runtime へ inject 済みでも借用/poison を残さない（try_borrow_mut が成功する）。
        assert!(
            runtime.try_borrow_mut().is_ok(),
            "inject 後も runtime に借用/poison を残さない"
        );
    }

    /// 消滅時整合（ResetOwnState アーム・R3.4）: choice 非表示・前回注入 Some(3) → 自前状態のみ
    /// None 整合し、**inject はしない**（上流原子性が正本＝choice_hover_inject を発火しない）。
    #[test]
    fn moved_inactive_with_prior_injection_resets_own_state_without_inject() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        // 選択肢無しの runtime（choice_active=false）。
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        assert!(
            !runtime.borrow().choice_active(&ActorKey::from("0")),
            "前提: choice_active=false（選択肢スパン無し）"
        );
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));

        let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
        let mut bw = BalloonWiring::new(tx);
        bw.set_hover(0, Some(3));
        world.insert_non_send_resource(bw);

        let ev = bubble_move(10, 20);
        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_moved(&mut world, e, e, &ev),
                "moved は常に false"
            );
        });

        assert_eq!(
            world
                .get_non_send_resource::<BalloonWiring>()
                .unwrap()
                .hover(0),
            None,
            "消滅時は自前状態のみ None 整合（ResetOwnState・Some(3)→None）"
        );
        assert!(
            !logs.iter().any(|l| l.contains("choice_hover_inject")),
            "ResetOwnState は inject しない（choice_hover_inject を出さない・上流原子性が正本）: {logs:?}"
        );
    }

    /// 完全 no-op（NoopInactive アーム・R1.4）: choice 非表示・前回注入なし → 自前状態を触らず
    /// inject もしない（非表示中は hover 追従なし）。
    #[test]
    fn moved_inactive_no_prior_injection_is_full_noop() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));

        let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
        world.insert_non_send_resource(BalloonWiring::new(tx)); // hover 空（未注入）

        let ev = bubble_move(10, 20);
        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_moved(&mut world, e, e, &ev),
                "moved は常に false"
            );
        });

        assert_eq!(
            world
                .get_non_send_resource::<BalloonWiring>()
                .unwrap()
                .hover(0),
            None,
            "NoopInactive は自前状態を触らない（未注入のまま None）"
        );
        assert!(
            !logs.iter().any(|l| l.contains("choice_hover_inject")),
            "NoopInactive は inject しない: {logs:?}"
        );
    }

    // -------------------------------------------------------------------------
    // on_balloon_pointer_pressed 配線層檻（task 4.2・design「配線層 > balloon ハンドラ」／
    // Error Handling／System Flows・R2.1/2.3/2.4/2.5/2.6/3.1/3.2/4.2/5.1/8.4）
    //
    // 合成 `PointerState` を直接ハンドラへ与え、(a) Tunnel 素通し、(b) 非左押下（右/中 down）素通し、
    // (c) 資源不在の縮退経路を正しい**ログレベル**（Emo2Wiring 不在=debug／BalloonWiring 不在=error）で、
    // (d) 棄却経路（非表示=inactive／非ヒット=no_hit の reason 弁別・零 send）を実 `TextLayerRuntime` で
    // 決定的に檻へ入れる。確定発行のフィールド一致・stale 棄却は click_selection 純関数檻（task 3.3）が
    // 網羅済み。
    //
    // NOTE（runtime constructibility）: `choice_hit_rows` は `present_frame`（GPU）でしか
    // `choice_snapshot` を埋めないため headless では現行 rows が常に空＝hit=None。ゆえに
    // 「ヒット→send→choice_selected info」の full pass-through は本層では実演できず、design Testing
    // Strategy item 6 の設計裁定どおり task 7.1/7.3 の pass-through 檻へ委ねる。send 機構自体は
    // task 2.2 の `send_selection`／`ChoiceSelectionInbox` 檻で、`Some` 構成は click_selection 檻
    // （3.3）で構造的に網羅済み。本檻はハンドラ経由の棄却・縮退・零 send を観測する。
    // -------------------------------------------------------------------------

    /// 左シングルクリックの合成 `PointerState`（client 物理 px・left_down=true）を組む。
    /// `double_click` フィールドは既定 `None` のまま——押下ハンドラは**参照しない**（DD-CI-9）。
    fn bubble_left_press(x: i32, y: i32) -> Phase<PointerState> {
        Phase::Bubble(PointerState {
            client_point: Point { x, y },
            left_down: true,
            ..Default::default()
        })
    }

    /// `BalloonWiring` と生存 `Receiver`（零 send 観測用）を組む。
    fn wiring_with_inbox() -> (BalloonWiring, Receiver<ChoiceSelection>) {
        let (tx, rx) = mpsc::channel::<ChoiceSelection>();
        (BalloonWiring::new(tx), rx)
    }

    /// Tunnel 素通し（R5.1・donor 同型）: Tunnel 相は資源に一切触れず即 `false`（副作用なし・零 send）。
    #[test]
    fn pressed_tunnel_phase_is_noop_false() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
        // 資源未挿入でも Tunnel は最初に短絡するため到達しない（副作用なしの担保）。
        let tunnel = Phase::Tunnel(PointerState {
            client_point: Point { x: 10, y: 20 },
            left_down: true,
            ..Default::default()
        });
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &tunnel),
            "Tunnel 相は即 false（伝播続行・非侵襲）"
        );
    }

    /// 非左押下は素通し（R5.1）: 右/中ボタン down（left_down=false）は確定でないため `false`・零 send。
    /// `double_click` を一切参照しないことを、既定 None のまま処理が left_down のみで分岐することで担保する。
    #[test]
    fn pressed_non_left_button_is_noop_false() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        // Emo2Wiring/BalloonWiring を present にしても、left_down=false なら手前で false 短絡する。
        let runtime = runtime_with_active_choice("0");
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        let (bw, rx) = wiring_with_inbox();
        world.insert_non_send_resource(bw);

        // 右ボタン down（left_down=false）——確定ではない。
        let right = Phase::Bubble(PointerState {
            client_point: Point { x: 10, y: 20 },
            left_down: false,
            right_down: true,
            ..Default::default()
        });
        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_pressed(&mut world, e, e, &right),
                "非左押下（右 down）は false 素通し"
            );
        });

        assert!(
            rx.try_recv().is_err(),
            "非左押下では send しない（Inbox は Empty）"
        );
        assert!(
            !logs.iter().any(|l| l.contains("choice_selected")),
            "非左押下では choice_selected を出さない: {logs:?}"
        );
    }

    /// Emo2Wiring 不在＝正常縮退（R4.1・donor presenter=None 同型）: **DEBUG** で no-op・`false`・零 send。
    #[test]
    fn pressed_emo2_absent_degrades_with_debug_not_error() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
        let ev = bubble_left_press(10, 20);

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_pressed(&mut world, e, e, &ev),
                "Emo2Wiring 不在は no-op false"
            );
        });

        assert!(
            logs.iter()
                .any(|l| l.contains("level=DEBUG") && l.contains("choice_pressed_no_emo2")),
            "Emo2Wiring 不在は DEBUG で正常縮退（event=choice_pressed_no_emo2）: {logs:?}"
        );
        assert!(
            !logs.iter().any(|l| l.contains("level=ERROR")),
            "Emo2Wiring 不在は構成異常ではない＝ERROR を出さない: {logs:?}"
        );
    }

    /// BalloonWiring 不在＝構成異常（結線漏れ）: Emo2Wiring は present でも **ERROR** で no-op・`false`。
    /// （借用規律の固定順序＝Emo2Wiring→BalloonWiring ゆえ Emo2Wiring present が前提。）
    #[test]
    fn pressed_balloon_wiring_absent_degrades_with_error() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        let ev = bubble_left_press(10, 20);

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_pressed(&mut world, e, e, &ev),
                "BalloonWiring 不在は no-op false"
            );
        });

        assert!(
            logs.iter()
                .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
            "BalloonWiring 不在は ERROR の構成異常（event=balloon_wiring_missing）: {logs:?}"
        );
    }

    /// BalloonWindowMarker 不在（理論上不到達の構成異常）: **ERROR** で no-op・`false`（silent 禁止）。
    #[test]
    fn pressed_missing_marker_errors_and_noop() {
        let mut world = World::new();
        let e = world.spawn_empty().id(); // BalloonWindowMarker を持たない entity
        let ev = bubble_left_press(10, 20);

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_pressed(&mut world, e, e, &ev),
                "marker 不在は no-op false"
            );
        });

        assert!(
            logs.iter()
                .any(|l| l.contains("level=ERROR") && l.contains("balloon_marker_missing")),
            "marker 不在は ERROR で縮退（silent failure 禁止）: {logs:?}"
        );
    }

    /// 非表示中クリックは棄却（R3.1）: choice_active=false の実 runtime へ左クリック → 発行ゼロ・
    /// `debug!(choice_click_rejected, reason="inactive")`。Inbox の try_recv は Empty。
    #[test]
    fn pressed_inactive_choice_rejected_with_reason_inactive() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        // 選択肢無しの runtime（choice_active=false）。
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        assert!(
            !runtime.borrow().choice_active(&ActorKey::from("0")),
            "前提: choice_active=false（選択肢スパン無し）"
        );
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        let (bw, rx) = wiring_with_inbox();
        world.insert_non_send_resource(bw);

        let ev = bubble_left_press(10, 20);
        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_pressed(&mut world, e, e, &ev),
                "非表示中クリックは棄却＝false（非発行）"
            );
        });

        assert!(
            rx.try_recv().is_err(),
            "非表示中クリックは send しない（Inbox は Empty・R3.1）"
        );
        assert!(
            logs.iter().any(|l| l.contains("level=DEBUG")
                && l.contains("choice_click_rejected")
                && l.contains("inactive")),
            "非表示中は reason=inactive の choice_click_rejected を DEBUG 発火: {logs:?}"
        );
        assert!(
            !logs.iter().any(|l| l.contains("choice_selected")),
            "棄却では choice_selected を出さない: {logs:?}"
        );
    }

    /// 表示中・非ヒットは棄却（R2.3）: choice_active=true でも headless では choice_hit_rows が空ゆえ
    /// 常に非ヒット → 発行ゼロ・`debug!(choice_click_rejected, reason="no_hit")`。
    #[test]
    fn pressed_active_non_hit_rejected_with_reason_no_hit() {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        let runtime = runtime_with_active_choice("0");
        assert!(
            runtime.borrow().choice_active(&ActorKey::from("0")),
            "前提: choice_active=true（選択肢スパンあり）"
        );
        assert!(
            runtime.borrow().choice_hit_rows(&ActorKey::from("0")).is_empty(),
            "前提: headless では choice_hit_rows は空（GPU 未実行）＝常に非ヒット"
        );
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        let (bw, rx) = wiring_with_inbox();
        world.insert_non_send_resource(bw);

        let ev = bubble_left_press(10, 20);
        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_pressed(&mut world, e, e, &ev),
                "非ヒットは棄却＝false（非発行）"
            );
        });

        assert!(
            rx.try_recv().is_err(),
            "非ヒットでは send しない（Inbox は Empty・R2.3）"
        );
        assert!(
            logs.iter().any(|l| l.contains("level=DEBUG")
                && l.contains("choice_click_rejected")
                && l.contains("no_hit")),
            "表示中・非ヒットは reason=no_hit の choice_click_rejected を DEBUG 発火: {logs:?}"
        );
        assert!(
            !logs.iter().any(|l| l.contains("choice_selected")),
            "棄却では choice_selected を出さない: {logs:?}"
        );
    }

    // -------------------------------------------------------------------------
    // clear_balloon_hover_on_leave 排他システム檻（task 5・design「clear_balloon_hover_on_leave」／
    // R1.3/3.4）
    //
    // bare World で `PointerLeave` マーカー保持 entity を組み、(a) バルーン所有 leave のみ hover を
    // 解除する（親チェーン→`BalloonWindowMarker`）、(b) 非バルーン窓の leave は無視、(c) マーカー不在は
    // 完全 no-op、(d) Emo2Wiring 不在は DEBUG 縮退（hover 不変）を決定的に檻へ入れる。判断分岐そのもの
    // （active×hit=None×last の全組合せ）は hover_action 純関数檻（task 3.2）が網羅済み。`PointerLeave`
    // の除去は本システムの責務外（FrameFinalize の機構不変）ゆえ検証しない。
    // -------------------------------------------------------------------------

    use bevy_ecs::hierarchy::ChildOf;
    use wintf::ecs::Window;
    use wintf::ecs::pointer::PointerLeave;

    /// バルーン窓（`BalloonWindowMarker`＋`Window`）を組み、その子 entity へ `PointerLeave` を載せる。
    /// 子の親チェーンは window へ届く（`find_owner_window` 相当）。
    fn spawn_balloon_leave_child(world: &mut World, scope: usize) -> Entity {
        let win = world
            .spawn((BalloonWindowMarker { scope }, Window::default()))
            .id();
        world.spawn((PointerLeave, ChildOf(win))).id()
    }

    /// `hover[scope]=Some(k)`（`ordinal=Some`）を仕込んだ `BalloonWiring` を World へ NonSend 挿入する。
    fn insert_wiring_with_hover(world: &mut World, scope: usize, ordinal: Option<usize>) {
        let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
        let mut bw = BalloonWiring::new(tx);
        if let Some(o) = ordinal {
            bw.set_hover(scope, Some(o));
        }
        world.insert_non_send_resource(bw);
    }

    /// バルーン所有 leave→hover 解除（Inject(None) アーム・R1.3）: choice 表示中・現行 hit=None
    /// （窓外離脱ゆえエッジ非採取）・前回注入 Some(2) → `inject_choice_hover(actor, None)`＋自前状態
    /// `BalloonWiring.hover[scope]` を None 更新＋`choice_hover_inject` を DEBUG 発火する。
    #[test]
    fn leave_balloon_owned_clears_hover_via_inject_none() {
        let mut world = World::new();
        spawn_balloon_leave_child(&mut world, 0);

        let runtime = runtime_with_active_choice("0");
        assert!(
            runtime.borrow().choice_active(&ActorKey::from("0")),
            "前提: choice_active=true（選択肢スパンあり）"
        );
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        insert_wiring_with_hover(&mut world, 0, Some(2));

        let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

        assert_eq!(
            world
                .get_non_send_resource::<BalloonWiring>()
                .unwrap()
                .hover(0),
            None,
            "バルーン所有 leave で hover[scope] が None へ解除される（Inject(None)・R1.3）"
        );
        assert!(
            logs.iter()
                .any(|l| l.contains("level=DEBUG") && l.contains("choice_hover_inject")),
            "離脱 hover 解除注入で choice_hover_inject を DEBUG 発火（DD-CI-7）: {logs:?}"
        );
        assert!(
            runtime.try_borrow_mut().is_ok(),
            "inject 後も runtime に借用/poison を残さない"
        );
    }

    /// バルーン所有 leave・非表示（ResetOwnState アーム・R3.4）: choice 非表示・前回注入 Some(3) →
    /// 自前状態のみ None 整合し、**inject はしない**（上流原子性が正本＝choice_hover_inject を出さない）。
    #[test]
    fn leave_balloon_owned_inactive_resets_own_state_without_inject() {
        let mut world = World::new();
        spawn_balloon_leave_child(&mut world, 0);

        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        assert!(
            !runtime.borrow().choice_active(&ActorKey::from("0")),
            "前提: choice_active=false（選択肢スパン無し）"
        );
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        insert_wiring_with_hover(&mut world, 0, Some(3));

        let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

        assert_eq!(
            world
                .get_non_send_resource::<BalloonWiring>()
                .unwrap()
                .hover(0),
            None,
            "消滅時は自前状態のみ None 整合（ResetOwnState・Some(3)→None・R3.4）"
        );
        assert!(
            !logs.iter().any(|l| l.contains("choice_hover_inject")),
            "ResetOwnState は inject しない（choice_hover_inject を出さない・上流原子性が正本）: {logs:?}"
        );
    }

    /// 非バルーン窓の leave は無視（key assertion）: `BalloonWindowMarker` を持たない窓の子が
    /// `PointerLeave` を保持しても、別 scope のバルーン hover は一切変化しない。
    #[test]
    fn leave_non_balloon_window_is_ignored() {
        let mut world = World::new();
        // 非バルーン窓（Window だが BalloonWindowMarker 無し）の子へ PointerLeave。
        let win = world.spawn(Window::default()).id();
        world.spawn((PointerLeave, ChildOf(win)));

        let runtime = runtime_with_active_choice("0");
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        insert_wiring_with_hover(&mut world, 0, Some(5));

        let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

        assert_eq!(
            world
                .get_non_send_resource::<BalloonWiring>()
                .unwrap()
                .hover(0),
            Some(5),
            "非バルーン窓の leave は hover を触らない（balloon 所有チェックの key assertion）"
        );
        assert!(
            !logs.iter().any(|l| l.contains("choice_hover_inject")),
            "非バルーン leave では注入しない: {logs:?}"
        );
    }

    /// マーカー不在は完全 no-op（R1.4 の離脱版）: `PointerLeave` を一切持たない World では hover を
    /// 触らず、Emo2Wiring にも触れずに即 return する（design Risks: 完全 no-op）。
    #[test]
    fn leave_no_marker_is_full_noop() {
        let mut world = World::new();
        // バルーン窓はあるが PointerLeave マーカーは一切無い。
        world.spawn((BalloonWindowMarker { scope: 0 }, Window::default()));

        let runtime = runtime_with_active_choice("0");
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
        insert_wiring_with_hover(&mut world, 0, Some(1));

        clear_balloon_hover_on_leave(&mut world);

        assert_eq!(
            world
                .get_non_send_resource::<BalloonWiring>()
                .unwrap()
                .hover(0),
            Some(1),
            "マーカー不在フレームは完全 no-op（hover 不変）"
        );
    }

    /// Emo2Wiring 不在＝正常縮退（R4.1・donor presenter=None 同型）: バルーン所有 leave があっても
    /// **DEBUG** で no-op・hover 不変（`event=choice_leave_no_emo2`・ERROR は出さない）。
    #[test]
    fn leave_emo2_absent_degrades_with_debug_and_leaves_hover() {
        let mut world = World::new();
        spawn_balloon_leave_child(&mut world, 0);
        // Emo2Wiring は挿入しない。BalloonWiring のみ present。
        insert_wiring_with_hover(&mut world, 0, Some(1));

        let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

        assert_eq!(
            world
                .get_non_send_resource::<BalloonWiring>()
                .unwrap()
                .hover(0),
            Some(1),
            "Emo2Wiring 不在では hover を触らず縮退（no-op）"
        );
        assert!(
            logs.iter()
                .any(|l| l.contains("level=DEBUG") && l.contains("choice_leave_no_emo2")),
            "Emo2Wiring 不在は DEBUG で正常縮退（event=choice_leave_no_emo2）: {logs:?}"
        );
        assert!(
            !logs.iter().any(|l| l.contains("level=ERROR")),
            "Emo2Wiring 不在は構成異常ではない＝ERROR を出さない: {logs:?}"
        );
    }

    // -------------------------------------------------------------------------
    // post-spawn 装着・NonSend 結線・スケジュール登録檻（task 6.1・design
    // 「attach_balloon_pointer_handlers / wire_balloon_choice」＋Integration Test 7・
    // R4.3/4.4/5.5/6.6）
    //
    // (1) 配線存在檻（6.6/4.3）: bare World → spawn_ghost_windows → attach_char（キャラ窓 baseline）
    //     → attach_balloon_pointer_handlers で、全バルーン窓に OnPointerMoved＋OnPointerPressed が
    //     装着され、キャラ窓のハンドラ集合は不変であること（非退行・donor attach 檻同型）。
    // (2) NonSend 結線檻: wire_balloon_choice 後に BalloonWiring＋ChoiceSelectionInbox が NonSend
    //     資源として存在すること（donor wire_mouse_input 同型）。
    // (3) スケジュール登録檻（Integration Test 7）: wire_balloon_choice が clear_balloon_hover_on_leave を
    //     Input スケジュールへ登録すること——構造（systems_len）と行動（Input 実行で hover 解除）の双方で
    //     固定する。登録漏れは高速離脱時の hover 残置として実機目視でしか検出できないため（1.3, 6.6）。
    // -------------------------------------------------------------------------

    use bevy_ecs::schedule::Schedules;
    use wintf::ecs::Input;
    use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed};

    use crate::input_events::attach_char_pointer_handlers;
    use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
    use crate::placement::source::GhostTitles;
    use crate::placement::spawn::{CharWindowMarker, spawn_ghost_windows};

    /// emo2 相当 2 スコープぶんの解決済み配置（spawn.rs 檻の two_scope_placements と同値）。
    fn two_scopes() -> Vec<ScopePlacement> {
        vec![
            ScopePlacement {
                scope: 0,
                char_pos: PointPx { x: 1483, y: 733 },
                char_size: SizePx { w: 434, h: 687 },
                balloon_pos: PointPx { x: 1071, y: 708 },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: PointPx { x: -412, y: -25 },
                anchor: Anchor::Bottom,
            },
            ScopePlacement {
                scope: 1,
                char_pos: PointPx { x: 1049, y: 1063 },
                char_size: SizePx { w: 278, h: 357 },
                balloon_pos: PointPx { x: 1334, y: 1044 },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: PointPx { x: 285, y: -19 },
                anchor: Anchor::Bottom,
            },
        ]
    }

    fn ghost_titles() -> GhostTitles {
        GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())])
    }

    fn balloon_window_entities(world: &mut World) -> Vec<Entity> {
        world
            .query_filtered::<Entity, With<BalloonWindowMarker>>()
            .iter(world)
            .collect()
    }

    fn char_window_entities(world: &mut World) -> Vec<Entity> {
        world
            .query_filtered::<Entity, With<CharWindowMarker>>()
            .iter(world)
            .collect()
    }

    fn has_pointer_handlers(world: &World, e: Entity) -> bool {
        world.get::<OnPointerMoved>(e).is_some() && world.get::<OnPointerPressed>(e).is_some()
    }

    /// 配線存在檻（6.6/4.3）: 全バルーン窓へハンドラ装着・キャラ窓集合不変（非退行）。
    #[test]
    fn attach_installs_handlers_on_all_balloon_windows_and_leaves_char_unchanged() {
        let mut world = World::new();
        spawn_ghost_windows(&mut world, &two_scopes(), &ghost_titles());
        // キャラ窓へ donor ハンドラを装着（R4.3 非退行の対象 baseline を非自明にする）。
        attach_char_pointer_handlers(&mut world);

        let balloons = balloon_window_entities(&mut world);
        let chars = char_window_entities(&mut world);
        assert_eq!(balloons.len(), 2, "2 スコープぶんのバルーン窓が spawn される");
        assert_eq!(chars.len(), 2, "2 スコープぶんのキャラ窓が spawn される");

        // 装着前: バルーン窓はハンドラ未装着（spawn.rs はバルーンに付けない・DD-IE-12）。
        for &e in &balloons {
            assert!(
                !has_pointer_handlers(&world, e),
                "attach 前のバルーン窓はポインタハンドラ未装着"
            );
        }
        // キャラ窓は donor で装着済み（baseline）。
        for &e in &chars {
            assert!(
                has_pointer_handlers(&world, e),
                "キャラ窓は donor（attach_char）でハンドラ装着済み（baseline）"
            );
        }

        attach_balloon_pointer_handlers(&mut world);

        // 装着後: 全バルーン窓に OnPointerMoved＋OnPointerPressed。
        for &e in &balloons {
            assert!(
                has_pointer_handlers(&world, e),
                "attach 後は全バルーン窓に OnPointerMoved＋OnPointerPressed が装着される（6.6）"
            );
        }
        // キャラ窓のハンドラ集合は不変（非退行・R4.3）。
        for &e in &chars {
            assert!(
                has_pointer_handlers(&world, e),
                "キャラ窓のハンドラ集合は attach_balloon で不変（非退行・R4.3）"
            );
        }
    }

    /// NonSend 結線檻（R5.5）: wire_balloon_choice が BalloonWiring＋ChoiceSelectionInbox を NonSend 挿入。
    #[test]
    fn wire_inserts_both_non_send_resources() {
        let mut world = World::new();
        world.init_resource::<Schedules>(); // wire は schedule 登録も行うため Schedules 既在が前提。

        assert!(
            world.get_non_send_resource::<BalloonWiring>().is_none(),
            "挿入前は BalloonWiring 不在"
        );
        assert!(
            world.get_non_send_resource::<ChoiceSelectionInbox>().is_none(),
            "挿入前は ChoiceSelectionInbox 不在"
        );

        wire_balloon_choice(&mut world);

        assert!(
            world.get_non_send_resource::<BalloonWiring>().is_some(),
            "wire 後は BalloonWiring が NonSend 挿入されている"
        );
        assert!(
            world.get_non_send_resource::<ChoiceSelectionInbox>().is_some(),
            "wire 後は ChoiceSelectionInbox が NonSend 挿入されている（発行 seam・5.3）"
        );
    }

    /// スケジュール登録檻・構造（Integration Test 7・R6.6）: wire が clear_balloon_hover_on_leave を
    /// Input スケジュールへ 1 件登録する。
    #[test]
    fn wire_registers_leave_system_into_input_schedule() {
        let mut world = World::new();
        world.init_resource::<Schedules>();

        wire_balloon_choice(&mut world);

        let schedules = world.resource::<Schedules>();
        assert!(schedules.contains(Input), "Input スケジュールが存在する");
        let input = schedules.get(Input).expect("Input schedule は存在する");
        assert_eq!(
            input.systems_len(),
            1,
            "Input スケジュールに 1 システム（clear_balloon_hover_on_leave）が登録される（6.6）"
        );
    }

    /// スケジュール登録檻・行動（Integration Test 7・R1.3/6.6）: 登録済みシステムが Input 実行で走り、
    /// バルーン所有 leave の hover を解除する（登録が「clear_balloon_hover_on_leave」であることの行動的証明・
    /// debug feature 非依存）。
    #[test]
    fn registered_leave_system_runs_in_input_schedule_and_clears_balloon_hover() {
        let mut world = World::new();
        world.init_resource::<Schedules>();

        // バルーン所有 leave（scope 0）＋表示中 choice の実 runtime。
        spawn_balloon_leave_child(&mut world, 0);
        let runtime = runtime_with_active_choice("0");
        world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));

        // wire で BalloonWiring／Inbox 挿入＋leave system 登録。
        wire_balloon_choice(&mut world);
        // wire が挿入した BalloonWiring に前回注入値 Some(2) を仕込む（解除対象）。
        world
            .get_non_send_resource_mut::<BalloonWiring>()
            .expect("wire で BalloonWiring 挿入済み")
            .set_hover(0, Some(2));
        assert_eq!(
            world.get_non_send_resource::<BalloonWiring>().unwrap().hover(0),
            Some(2),
            "前提: hover[0]=Some(2)"
        );

        // Input スケジュールを実行 → 登録済み clear_balloon_hover_on_leave が走り hover を解除する。
        world.run_schedule(Input);

        assert_eq!(
            world.get_non_send_resource::<BalloonWiring>().unwrap().hover(0),
            None,
            "Input 実行で登録済み clear_balloon_hover_on_leave が走り hover を解除する（登録の行動的証明）"
        );
    }

    // -------------------------------------------------------------------------
    // 貫通ポインタ列の決定論檻（task 7.1・design Testing Strategy Integration Test 6
    // 「貫通檻（R6.2 字義・条件付き）」／R6.1/6.2/6.3/6.4/6.5）
    //
    // STEP 0 裁定（本 task 冒頭で確認・GPU-gated）: `TextLayerRuntime::choice_hit_rows` は
    // `choice_snapshot`（HashMap）のみを読み（actor.rs:389-393）、その population は present_actor が
    // GPU present 成功時にのみ行う（actor.rs:204-205・688-709）。`apply_cue(Choice)` は逆に
    // `choice_snapshot` を remove する（actor.rs:297）ため、headless では `choice_active==true` でも
    // `choice_hit_rows` は常に空＝hit=None（既存檻 pressed_active_non_hit_rejected_with_reason_no_hit が
    // 実証済み）。population 経路 present_frame は WucGraphicsResource/Compositor・GraphicsCore・
    // dwrite_factory を要求し（actor.rs:532-576）headless では Err(Device) を返す——GPU 無しに snapshot を
    // populate する手段は無い。ゆえに実ハンドラ経由で「Some(ordinal) 追従」「hit→send→choice_selected」の
    // full pass-through を実演することは構造的に不可能。
    //
    // 設計裁定（Integration Test 6・設計バリデーション Issue 1 の非決定例外）どおり本檻は**分解形**で R6 を
    // 満たす: 合成 `ChoiceHitRow`（上流契約型・pub フィールドで直接構築可＝task 3.1 と同じ正当性。fake
    // runtime で hit を捏造するのではなく、上流照会契約の型そのものを純関数へ供給する）を純関数パイプライン
    // （hit_choice_row→hover_action／click_selection）へ**注入座標列**（move 群＋click）で通し、発行は
    // **実 mpsc seam**（`BalloonWiring::send_selection`→`ChoiceSelectionInbox::try_recv`）で観測する。
    // GPU・実窓・sleep を一切用いない（R6.4）。判断分岐の実行網羅は 3.1/3.2/3.3、発行 seam は 2.2、配線存在／
    // スケジュール登録は 6.1 の各檻が閉じており、本檻はそれらの上に「注入座標列→hover 追従→一度きり発行／
    // 非発行」を貫通で固定する（R6.1/6.2/6.3/6.5）。
    // -------------------------------------------------------------------------

    /// 2 行の選択肢ヒット行（ordinal 0/1・非重複帯・窓物理 px）。上流契約型を直接構築する（task 3.1 同型）。
    fn two_choice_rows() -> Vec<ChoiceHitRow> {
        vec![
            row(0, 0.0, 0.0, 100.0, 20.0),  // ordinal 0（上帯 y∈[0,20)）
            row(1, 0.0, 20.0, 100.0, 40.0), // ordinal 1（下帯 y∈[20,40)）
        ]
    }

    /// 移動ハンドラ（`on_balloon_pointer_moved`）の自前 last-injected 更新（⑤・design §204）を鏡写す:
    /// `Inject` は注入値、`ResetOwnState` は None、`Keep`/`NoopInactive` は据え置き。
    fn apply_last(action: HoverAction, last: Option<usize>) -> Option<usize> {
        match action {
            HoverAction::Inject(v) => v,
            HoverAction::ResetOwnState => None,
            HoverAction::Keep | HoverAction::NoopInactive => last,
        }
    }

    /// 注入座標列→hover 追従→hit クリック一度きり発行（貫通・R6.1/6.2/6.4/6.5・GPU/実窓/sleep 不要）。
    ///
    /// 合成 2 行へ move 点列 [row0 内, row1 内, row1 内（同値再入）, 全行外] を注入し、各 move で
    /// `hit_choice_row→hover_action` を評価しつつ移動ハンドラ同型に last-injected を更新して hover 軌跡を
    /// 採取する。軌跡は注入列に追従する（Some(0)→Some(1)→Some(1)→None）。中間で「同値再入は Keep（再 Inject
    /// しない）」「行外遷移は Inject(None)＝ハイライト解除」を明示 assert し、hover 追跡が誤って last を
    /// 更新しなければ落ちる非自明性を担保する。続いて row1 上の click 点で `click_selection`→**実**
    /// `BalloonWiring::send_selection`→`ChoiceSelectionInbox::try_recv` が発行を一度だけ観測する
    /// （2 度目は Empty・R6.2）。
    #[test]
    fn injected_pointer_sequence_hover_follows_then_click_issues_once_via_inbox() {
        let rows = two_choice_rows();
        let scope = 7usize;

        // ── 注入 move 点列（バルーン窓 client 物理 px・k=1.0 素通し・R6.4）─────────────────────────
        let moves = [
            (50.0_f32, 10.0_f32), // row0 内 → Some(0)（初回 Inject）
            (50.0, 30.0),         // row1 内 → Some(1)（行遷移 Inject）
            (50.0, 30.0),         // row1 内（同値再入）→ Some(1)（Keep・再注入しない）
            (200.0, 200.0),       // 全行外 → None（Inject(None) 解除）
        ];

        let mut last: Option<usize> = None; // 移動ハンドラ ⑤ の自前 last-injected（初期未注入）。
        let mut trajectory: Vec<Option<usize>> = Vec::new();
        let mut actions: Vec<HoverAction> = Vec::new();
        for (x, y) in moves {
            // choice 表示中（active=true）——注入座標に対する現行 rows のヒットを ordinal へ展開し
            // 純関数核で遷移を決め、ハンドラ同型に last を更新する（毎 move 現行 rows を読む）。
            let hit = hit_choice_row(&rows, x, y).map(|i| rows[i].ordinal);
            let action = hover_action(true, hit, last);
            last = apply_last(action, last);
            actions.push(action);
            trajectory.push(last);
        }

        // hover 軌跡が注入座標列に追従する（R6.1）。
        assert_eq!(
            trajectory,
            vec![Some(0), Some(1), Some(1), None],
            "hover は注入座標列に追従する（row0→row1→row1 同値→行外解除）"
        );
        // 非自明性①: 同値再入（3 手目）は再注入せず Keep。last 更新が誤って Some(1) を保持しなければ
        // ここが Inject になり落ちる。
        assert_eq!(
            actions[2],
            HoverAction::Keep,
            "同値再入は Keep（hover 追跡が誤れば Inject になり落ちる非自明 assertion）"
        );
        // 非自明性②: 行外遷移（4 手目）は last=Some(1)→None のハイライト解除 Inject(None)（R1.3）。
        // 追跡が誤って last を None のままにしていれば Keep になり落ちる。
        assert_eq!(
            actions[3],
            HoverAction::Inject(None),
            "行外遷移はハイライト解除 Inject(None)（追跡が誤れば Keep になり落ちる非自明 assertion）"
        );

        // ── row1 上の click を実 mpsc seam へ通す（一度きり発行の貫通観測・R6.2）─────────────────────
        let (tx, rx) = mpsc::channel::<ChoiceSelection>();
        let wiring = BalloonWiring::new(tx);
        let inbox = ChoiceSelectionInbox(rx);

        // click 点 (50, 30) は row1（ordinal 1）内。click_selection が**現行**行から Some を構成する。
        let sel = click_selection(true, &rows, 50.0, 30.0, scope)
            .expect("row1 上の click は ChoiceSelection を構成する");
        // 押下ハンドラ同型に「Some のときだけ 1 回 send」する（発行の一度きり制御はエッジ検出＝1 send）。
        assert!(
            wiring.send_selection(sel),
            "Receiver 生存中の発行は成功（send_selection→mpsc）"
        );

        // seam の Receiver 経由で発行を一度だけ観測する（送信値は click 行から構成される・R6.2）。
        let received = inbox.0.try_recv().expect("発行した ChoiceSelection が届く");
        assert_eq!(received.id, "q1", "発行値は click 行（ordinal 1）から構成される");
        assert_eq!(received.label, "label1", "label も click 行から転写される");
        assert_eq!(
            received.scope, scope,
            "scope は引数（BalloonWindowMarker.scope）由来"
        );
        assert!(
            inbox.0.try_recv().is_err(),
            "発行は一度きり（2 度目の try_recv は Empty・R6.2）"
        );
    }

    /// 非 hit・stale・非表示（消滅）click は実 seam へ何も流さない（貫通・R6.2/6.3/6.4・GPU/実窓/sleep 不要）。
    ///
    /// 単一の実 mpsc seam を張り、注入 click を `click_selection`→（`None` のとき非 send）で通す:
    /// (a) 表示中・全行外座標＝非 hit → None → 非 send（R6.3）、(b) 現行 rows がクリック座標を覆わない
    /// stale（レイアウト差替後）→ None → 非 send（現行ジオメトリのみ読む・R6.3）、(c) 非表示（消滅・
    /// active=false）→ 矩形内座標でも短絡 None → 非 send（R6.3）。いずれの後も `try_recv` は Empty。
    #[test]
    fn injected_non_hit_stale_and_inactive_clicks_issue_nothing_via_inbox() {
        let rows = two_choice_rows();
        let scope = 7usize;

        let (tx, rx) = mpsc::channel::<ChoiceSelection>();
        let wiring = BalloonWiring::new(tx);
        let inbox = ChoiceSelectionInbox(rx);

        // 押下ハンドラ同型: Some のときだけ send する薄い注入経路（None は非 send＝棄却）。
        let try_click = |active: bool, current: &[ChoiceHitRow], x: f32, y: f32| {
            if let Some(sel) = click_selection(active, current, x, y, scope) {
                wiring.send_selection(sel);
            }
        };

        // (a) 表示中・全行外＝非 hit（R6.3）。
        try_click(true, &rows, 200.0, 200.0);
        // (b) stale: 現行 rows は (50,30) を覆わない別位置の行のみ（キャッシュではなく現行のみ読む・R6.3）。
        let stale_rows = [row(0, 500.0, 500.0, 540.0, 520.0)];
        try_click(true, &stale_rows, 50.0, 30.0);
        // (c) 非表示（消滅・active=false）——row1 内座標でも hit 判定より前に短絡 None（R6.3）。
        try_click(false, &rows, 50.0, 30.0);

        assert!(
            inbox.0.try_recv().is_err(),
            "非 hit／stale／非表示の click は一切 send されない（Inbox は Empty・R6.2/6.3）"
        );
    }

    // -------------------------------------------------------------------------
    // 資源縮退・Tunnel 素通し 副作用ゼロ回帰檻（task 7.2・design Testing Strategy Integration
    // Tests item 3/4・Error Handling／R8.1/8.4）
    //
    // 判断分岐そのもの（Tunnel 短絡／Emo2Wiring 不在=debug 縮退／BalloonWiring 不在=error 縮退／
    // marker 不在=error）の**実行網羅**は task 4.1（moved）／4.2（pressed）の各縮退檻が既に閉じている
    // （moved_tunnel_phase_is_noop_false／moved_emo2_absent_degrades_with_debug_not_error／
    // moved_balloon_wiring_absent_degrades_with_error ほか・pressed 側も同型）。ただしそれらは
    // 「戻り値 false＋ログレベル」までで、**観測資源を同居させた副作用ゼロ三点**——(a) send なし
    // （present の `ChoiceSelectionInbox.try_recv()==Empty`）、(b) `BalloonWiring.hover` 不変、
    // (c) runtime へ `inject_choice_hover` 未適用（借用/poison を残さず `choice_active` 不変・moved は
    // `choice_hover_inject` を出さない）——までは固定していない（Tunnel 檻は資源を一切挿入せず、Emo2 不在
    // 檻は BalloonWiring を挿入しないため観測点が無かった）。
    //
    // 本 7.2 檻はその副作用ゼロ次元を**両ハンドラ×{Tunnel, Emo2Wiring 不在, BalloonWiring 不在}**へ
    // first-class に追試する（縮退時に対応ログ＝debug／error が出つつ、いかなる観測可能状態も動かない
    // ことを、観測資源を同居させて固定する）。判断分岐の重複網羅ではなく副作用ゼロ観測の追加ゆえ非重複。
    //
    // 8.4（スレッド親和・NonSend 借用は Input スケジュール排他システム内）: 本檻は各ハンドラを単一スレッドの
    // `&mut World` 排他呼出で直接叩く——NonSend 資源（`Emo2Wiring`／`BalloonWiring`・`Rc<RefCell>`）の借用が
    // 排他システム内でのみ起きるという構造契約は、ハンドラの `&mut World` シグネチャ自体が強制する
    // （別途スレッドテストは不要）。
    // -------------------------------------------------------------------------

    /// 縮退シナリオ（観測資源を同居させた副作用ゼロ回帰の分類）。
    #[derive(Clone, Copy)]
    enum Degrade {
        /// Tunnel 相（全資源 present でも即 false・一切触れない）。
        Tunnel,
        /// `Emo2Wiring` 不在（`BalloonWiring` は present）＝debug 正常縮退。
        Emo2Absent,
        /// `BalloonWiring` 不在（`Emo2Wiring` は present）＝error 構成異常縮退。
        BalloonWiringAbsent,
    }

    /// moved: Tunnel／Emo2Wiring 不在／BalloonWiring 不在のいずれも副作用ゼロで縮退する（8.1/8.4）。
    ///
    /// 各シナリオで観測資源（Emo2Wiring の active runtime・hover=Some(2) 仕込みの BalloonWiring・Inbox）を
    /// 可能な限り同居させ、(1) 戻り値 false、(2) 対応ログレベル（Tunnel=無縮退ログ・Emo2 不在=DEBUG・
    /// BalloonWiring 不在=ERROR）、(3) 副作用ゼロ三点（choice_hover_inject なし／hover 不変／Inbox Empty／
    /// runtime 借用・active 不変）を固定する。判断分岐網羅は 4.1、本檻は副作用ゼロ観測を担う。
    #[test]
    fn moved_tunnel_and_resource_degrade_are_side_effect_free() {
        for scenario in [Degrade::Tunnel, Degrade::Emo2Absent, Degrade::BalloonWiringAbsent] {
            let mut world = World::new();
            let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

            // Emo2Wiring（active choice runtime）は Tunnel／BalloonWiringAbsent で present。
            let runtime = match scenario {
                Degrade::Tunnel | Degrade::BalloonWiringAbsent => {
                    let rt = runtime_with_active_choice("0");
                    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&rt)));
                    Some(rt)
                }
                Degrade::Emo2Absent => None,
            };

            // BalloonWiring（hover=Some(2) 仕込み）＋Inbox は Tunnel／Emo2Absent で present。
            let inbox_rx = match scenario {
                Degrade::Tunnel | Degrade::Emo2Absent => {
                    let (tx, rx) = mpsc::channel::<ChoiceSelection>();
                    let mut bw = BalloonWiring::new(tx);
                    bw.set_hover(0, Some(2));
                    world.insert_non_send_resource(bw);
                    Some(rx)
                }
                Degrade::BalloonWiringAbsent => None,
            };

            // 事象: Tunnel は Tunnel 相（資源 present でも短絡）・それ以外は通常 Bubble move。
            let ev = match scenario {
                Degrade::Tunnel => Phase::Tunnel(PointerState {
                    client_point: Point { x: 10, y: 20 },
                    ..Default::default()
                }),
                _ => bubble_move(10, 20),
            };

            let logs = capture_logs(|| {
                assert!(
                    !on_balloon_pointer_moved(&mut world, e, e, &ev),
                    "縮退時 moved は常に false（非侵襲）"
                );
            });

            // ── (2) 縮退時の対応ログレベル（8.1 の観測条件）─────────────────────────────────
            match scenario {
                Degrade::Tunnel => assert!(
                    !logs
                        .iter()
                        .any(|l| l.contains("choice_hover_inject") || l.contains("level=ERROR")),
                    "Tunnel は短絡ゆえ inject も error も出さない: {logs:?}"
                ),
                Degrade::Emo2Absent => {
                    assert!(
                        logs.iter()
                            .any(|l| l.contains("level=DEBUG") && l.contains("choice_moved_no_emo2")),
                        "Emo2Wiring 不在は DEBUG で正常縮退（choice_moved_no_emo2）: {logs:?}"
                    );
                    assert!(
                        !logs.iter().any(|l| l.contains("level=ERROR")),
                        "Emo2Wiring 不在は構成異常でない＝ERROR を出さない: {logs:?}"
                    );
                }
                Degrade::BalloonWiringAbsent => assert!(
                    logs.iter()
                        .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
                    "BalloonWiring 不在は ERROR の構成異常（balloon_wiring_missing）: {logs:?}"
                ),
            }

            // ── (3) 副作用ゼロ①: runtime へ inject_choice_hover 未適用（全シナリオ共通）───────────
            assert!(
                !logs.iter().any(|l| l.contains("choice_hover_inject")),
                "縮退では runtime へ hover を注入しない（choice_hover_inject なし）: {logs:?}"
            );

            // ── (3) 副作用ゼロ②③: BalloonWiring.hover 不変＋send なし（present の場合）─────────────
            if let Some(rx) = &inbox_rx {
                assert_eq!(
                    world
                        .get_non_send_resource::<BalloonWiring>()
                        .unwrap()
                        .hover(0),
                    Some(2),
                    "縮退では BalloonWiring.hover[scope] を動かさない（仕込み Some(2) 不変）"
                );
                assert!(
                    rx.try_recv().is_err(),
                    "縮退では ChoiceSelection を発行しない（Inbox は Empty）"
                );
            }

            // ── (3) 副作用ゼロ④: runtime 未変更（present の場合）借用/poison を残さず active 不変 ─────
            if let Some(rt) = &runtime {
                assert!(
                    rt.try_borrow_mut().is_ok(),
                    "縮退では runtime に借用/poison を残さない"
                );
                assert!(
                    rt.borrow().choice_active(&ActorKey::from("0")),
                    "縮退では runtime の choice 状態を動かさない（active 不変）"
                );
            }
        }
    }

    /// pressed: Tunnel／Emo2Wiring 不在／BalloonWiring 不在のいずれも副作用ゼロで縮退する（8.1/8.4）。
    ///
    /// moved 版と対称。pressed の副作用は発行（send）ゆえ副作用ゼロ観測は choice_selected info の非発火＋
    /// present な Inbox の `try_recv()==Empty`＋BalloonWiring.hover 不変＋runtime 借用/active 不変で固定する。
    /// 判断分岐網羅は 4.2、本檻は副作用ゼロ観測を担う。
    #[test]
    fn pressed_tunnel_and_resource_degrade_are_side_effect_free() {
        for scenario in [Degrade::Tunnel, Degrade::Emo2Absent, Degrade::BalloonWiringAbsent] {
            let mut world = World::new();
            let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

            let runtime = match scenario {
                Degrade::Tunnel | Degrade::BalloonWiringAbsent => {
                    let rt = runtime_with_active_choice("0");
                    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&rt)));
                    Some(rt)
                }
                Degrade::Emo2Absent => None,
            };

            let inbox_rx = match scenario {
                Degrade::Tunnel | Degrade::Emo2Absent => {
                    let (tx, rx) = mpsc::channel::<ChoiceSelection>();
                    let mut bw = BalloonWiring::new(tx);
                    bw.set_hover(0, Some(2));
                    world.insert_non_send_resource(bw);
                    Some(rx)
                }
                Degrade::BalloonWiringAbsent => None,
            };

            // 事象: Tunnel は Tunnel 相（left_down=true でも Tunnel 短絡が先）・それ以外は左シングル押下。
            let ev = match scenario {
                Degrade::Tunnel => Phase::Tunnel(PointerState {
                    client_point: Point { x: 10, y: 20 },
                    left_down: true,
                    ..Default::default()
                }),
                _ => bubble_left_press(10, 20),
            };

            let logs = capture_logs(|| {
                assert!(
                    !on_balloon_pointer_pressed(&mut world, e, e, &ev),
                    "縮退時 pressed は常に false（発行なし）"
                );
            });

            // ── (2) 縮退時の対応ログレベル（8.1 の観測条件）─────────────────────────────────
            match scenario {
                Degrade::Tunnel => assert!(
                    !logs
                        .iter()
                        .any(|l| l.contains("choice_selected") || l.contains("level=ERROR")),
                    "Tunnel は短絡ゆえ choice_selected も error も出さない: {logs:?}"
                ),
                Degrade::Emo2Absent => {
                    assert!(
                        logs.iter().any(
                            |l| l.contains("level=DEBUG") && l.contains("choice_pressed_no_emo2")
                        ),
                        "Emo2Wiring 不在は DEBUG で正常縮退（choice_pressed_no_emo2）: {logs:?}"
                    );
                    assert!(
                        !logs.iter().any(|l| l.contains("level=ERROR")),
                        "Emo2Wiring 不在は構成異常でない＝ERROR を出さない: {logs:?}"
                    );
                }
                Degrade::BalloonWiringAbsent => assert!(
                    logs.iter()
                        .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
                    "BalloonWiring 不在は ERROR の構成異常（balloon_wiring_missing）: {logs:?}"
                ),
            }

            // ── (3) 副作用ゼロ①: 選択発行 info（choice_selected）を出さない（全シナリオ共通）───────
            assert!(
                !logs.iter().any(|l| l.contains("choice_selected")),
                "縮退では選択を発行しない（choice_selected info なし）: {logs:?}"
            );

            // ── (3) 副作用ゼロ②③: BalloonWiring.hover 不変＋send なし（present の場合）─────────────
            if let Some(rx) = &inbox_rx {
                assert_eq!(
                    world
                        .get_non_send_resource::<BalloonWiring>()
                        .unwrap()
                        .hover(0),
                    Some(2),
                    "縮退では BalloonWiring.hover[scope] を動かさない（pressed は元来非更新だが回帰固定）"
                );
                assert!(
                    rx.try_recv().is_err(),
                    "縮退では ChoiceSelection を発行しない（Inbox は Empty）"
                );
            }

            // ── (3) 副作用ゼロ④: runtime 未変更（present の場合）借用/poison を残さず active 不変 ─────
            if let Some(rt) = &runtime {
                assert!(
                    rt.try_borrow_mut().is_ok(),
                    "縮退では runtime に借用/poison を残さない"
                );
                assert!(
                    rt.borrow().choice_active(&ActorKey::from("0")),
                    "縮退では runtime の choice 状態を動かさない（active 不変）"
                );
            }
        }
    }
}
