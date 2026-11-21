//! バルーン選択肢対話配線。
//!
//! バルーン窓のポインタイベントを捉え、選択肢ヒットの判定・ハイライト追従・クリック確定を
//! kanade（dispatcher の inbox 経由）／文字層 runtime へ橋渡しする。
//!
//! 構成: 契約型（`ChoiceSelection` ほか）・NonSend 資源（`BalloonWiring`・`ChoiceSelectionInbox`）・
//! 純関数判定核（選択肢ヒット・ハイライト遷移）・配線層（`on_balloon_pointer_moved`／
//! `on_balloon_pointer_pressed`・`attach_balloon_pointer_handlers`・`wire_balloon_choice`）。
//!
//! 本番の入口は 3 箇所: 離脱の系の登録 `register_balloon_leave_system`（`ghost_session::register_systems`
//! からプロセスに 1 回）・状態の載せ替え `wire_balloon_choice`（`ghost_session::boot_ghost` から
//! ゴーストごとに同期呼出）・`attach_balloon_pointer_handlers`（`ghost_session::prepare_ghost_windows`
//! の窓 spawn 直後クロージャ内）。
//! バルーンの上の滞在の集合は runtime の型 `BalloonHover`（`crates/areka-runtime/src/balloon_hover.rs`）
//! へ分けてあり、書くのはこの mod・読むのはバルーン可視性の相（`run_balloon_visibility_phase`）。
//!
//! 上流契約（collision-geometry の resolver・`RuntimeWiring::runtime()` の読み口）は消費のみ行い、
//! 上流が balloon.rs を知る逆方向の依存は作らない。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};

use areka_emo_text::actor::{ChoiceHitRow, TextLayerRuntime};
use areka_emo_text::place::{PlaceKey, TextPlace};
use areka_emo_text::state::SpanKind;
#[cfg(test)]
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed, dispatch_pointer_events};

use areka_placement::spawn::BalloonWindowMarker;

use super::user_break;

#[path = "balloon_exit.rs"]
mod exit;
#[path = "balloon_moved.rs"]
mod moved;
#[path = "balloon_pressed.rs"]
mod pressed;

pub(crate) use exit::clear_balloon_hover_on_leave;
pub(crate) use moved::on_balloon_pointer_moved;
pub(crate) use pressed::{finished_talking, on_balloon_pointer_pressed};

/// 選択確定のワイヤ形。
///
/// 下流が表示層へ再照会せず選択解決とカスケード発火を組み立てられる自己完結データ。
/// 解決キーは `id`（`SakuraMsg::ResolveChoice { id }` と整合）であり、表示層内部の主キーである
/// `ordinal` はワイヤ形に含めない（漏洩防止）。
///
/// 発行は [`on_balloon_pointer_pressed`]→[`BalloonWiring::send_selection`]、全フィールドの消費は
/// `crates/areka-input/src/choice_drain.rs` の `to_choice_input`（`ChoiceInput` へ不透明転写）と、
/// 種類がアンカーのときの `to_anchor_input`（`AnchorInput` へ同じく転写）。`resolve_choice` は本 crate
/// から呼ばない（発行までが本 mod の範囲・カスケードは kanade 側）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceSelection {
    /// 当たった範囲の種類（選択肢かアンカー・当たりの行から写す）。型の名前は選択肢だけだった
    /// 頃のままで、アンカーの知らせも同じ形で運ぶ（その場合 `id`・`label`・`references` は
    /// `\_a` の ID・範囲の文字・2 番目以降の引数）。
    pub kind: SpanKind,
    /// `\q` ID（選択解決の主キー・不透明転写）。
    pub id: String,
    /// 表示ラベル（不透明転写）。
    pub label: String,
    /// 発生元 scope（`BalloonWindowMarker.scope` 由来）。
    pub scope: usize,
    /// `\q` 第 3 引数以降（参照列・不透明転写）。
    pub references: Vec<String>,
}

/// 選択の発行の記録の名前（送れたとき・送れなかったときの順）。
///
/// 選択肢は今までの名前のまま。アンカーは `anchor_` で始まる名前にして、ログで種類を見分ける
/// （1 回の発行に両方の名前は出さない）。押下の 2 か所（普通のバルーン・箱）と送り口が同じ表を引く。
pub(super) fn selection_events(kind: SpanKind) -> (&'static str, &'static str) {
    match kind {
        SpanKind::Choice => ("choice_selected", "choice_selection_send_failed"),
        SpanKind::Anchor => ("anchor_selected", "anchor_selection_send_failed"),
    }
}

/// 選択を送れた押下の後に呼ぶ。
///
/// 種類が選択肢のときだけ、押した場所の選択肢を済みにする（済んだ行は光らず、押しても選択に
/// ならない）。アンカーはトークが終わっても押せるので済みにしない。選択待ちに入る前の押下では
/// 文字の層の口が何もしない（kanade が捨てる押下なので押し直せるまま残す）。呼び手は普通のバルーンの
/// 押下（`balloon_pressed.rs`）と箱の押下（`shell_box_handler.rs`）で、送出に成功したときだけ呼ぶ。
///
/// 文字の層を借りられなければ `error!` を残して何もしない（選択は送れているので、その場所の
/// 選択肢はトークの終わりで済む）。
pub(super) fn settle_after_selection(
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    place: &PlaceKey,
    kind: SpanKind,
) {
    if kind != SpanKind::Choice {
        return;
    }
    match runtime.try_borrow_mut() {
        Ok(mut rt) => rt.settle_choices_at(place),
        Err(_) => tracing::error!(
            event = "balloon_runtime_borrow_failed",
            actor = %place.actor,
            "runtime try_borrow_mut 失敗（押下の後の済み）: 選択肢を済みにせず縮退"
        ),
    }
}

/// 選択を送り、送れたら場所の選択肢を済みにして記録する。送れたら `true`。
pub(super) fn dispatch_selection(
    wiring: &BalloonWiring,
    runtime: Option<&Rc<RefCell<TextLayerRuntime>>>,
    place: &PlaceKey,
    sel: ChoiceSelection,
) -> bool {
    let (selected, send_failed) = selection_events(sel.kind);
    let box_name = match &place.place {
        TextPlace::Box(name) => Some(name.as_str()),
        TextPlace::Balloon => None,
    };
    let kind = sel.kind;
    let scope = sel.scope;
    let id = sel.id.clone();
    let label = sel.label.clone();
    let references_len = sel.references.len();
    if !wiring.send_selection(sel) {
        tracing::error!(
            event = send_failed,
            scope,
            id = %id,
            "ChoiceSelection 発行シンク送出失敗（受け口消滅後）: no-op 縮退"
        );
        return false;
    }
    if let Some(runtime) = runtime {
        settle_after_selection(runtime, place, kind);
    }
    let message = if box_name.is_some() {
        "選択確定: ChoiceSelection を発行（箱）"
    } else {
        "選択確定: ChoiceSelection を発行"
    };
    tracing::info!(
        event = selected,
        scope,
        id = %id,
        label = %label,
        references_len,
        r#box = box_name,
        "{message}"
    );
    true
}

/// バルーン選択肢対話の配線資源（NonSend）。
///
/// UI スレッド所有の資源として `World` へ NonSend 挿入し（`insert_non_send`）、Input
/// スケジュール排他システム内でのみ借用する。mpsc `Sender` は `Send` だが `hover` 追跡と一体で
/// UI スレッド固定運用ゆえ NonSend 1 個に束ねる。
///
/// - `selection_tx`: 選択確定 [`ChoiceSelection`] の発行シンク（std mpsc）。発行は
///   [`ChoiceSelectionInbox`] へ流れ、drain が kanade へ転送する（`resolve_choice` は本 crate から
///   直接呼ばない）。
/// - `hover`: scope→最後に注入した hover ordinal の自前追跡（表示層に getter が無いため）。
///   表示状態の正本ではなく、(a) 同値再注入の遷移検出、(b) 選択肢消滅時の自前状態整合
///   （`None` 上書き）のみに用いる。
///
/// 挿入は [`wire_balloon_choice`]、消費はポインタハンドラ（[`on_balloon_pointer_moved`]／
/// [`on_balloon_pointer_pressed`]・[`attach_balloon_pointer_handlers`] で装着）と離脱システム
/// [`clear_balloon_hover_on_leave`]。バルーンの上の滞在の集合は `BalloonHover` に分けてある。
pub struct BalloonWiring {
    /// [`ChoiceSelection`] 発行シンク（mpsc）。
    selection_tx: Sender<ChoiceSelection>,
    /// scope → 最後に注入した hover ordinal（getter 不在の自前追跡）。
    hover: HashMap<usize, Option<usize>>,
}

impl BalloonWiring {
    /// 発行シンク [`Sender`] から構築する（`hover` は空 map で初期化）。
    pub fn new(selection_tx: Sender<ChoiceSelection>) -> Self {
        Self {
            selection_tx,
            hover: HashMap::new(),
        }
    }

    /// 発行シンクへ送る（記録はしない）。受け口が無ければ `false`。
    pub(crate) fn send_selection(&self, selection: ChoiceSelection) -> bool {
        self.selection_tx.send(selection).is_ok()
    }

    /// scope の最後に注入した hover ordinal を回収する（未注入は `None`）。
    ///
    /// [`on_balloon_pointer_moved`] の遷移検出（同値再注入の抑制）と
    /// [`clear_balloon_hover_on_leave`] の消滅時整合が参照する。
    pub(crate) fn hover(&self, scope: usize) -> Option<usize> {
        self.hover.get(&scope).copied().flatten()
    }

    /// scope の hover ordinal を記録する（`None` 上書きは消滅時整合）。
    ///
    /// 注入は「最後に注入した値」の記録であり表示状態の正本ではない（正本は上流）。
    pub(crate) fn set_hover(&mut self, scope: usize, ordinal: Option<usize>) {
        self.hover.insert(scope, ordinal);
    }
}

/// 選択確定通知の受け口。
///
/// `Receiver` 生存により [`BalloonWiring::send_selection`] は `Err` にならず、発行の mpsc 観測と
/// 実機ログが成立する。構築は [`wire_balloon_choice`]、受信は drain 排他システム `choice_drain.rs` の
/// `drain_choice_selections`（系の登録は `register_choice_drain` を `ghost_session::register_systems`
/// からプロセスに 1 回・送り口は `wire_choice_drain` が `boot_ghost` からゴーストごとに置く）。
pub(crate) struct ChoiceSelectionInbox(pub(crate) Receiver<ChoiceSelection>);

/// 点包含 hit 判定（純関数）。
///
/// 包含は半開区間 `[left, right) × [top, bottom)`（whole-pixel 行矩形と整合）——
/// `left`/`top` 辺は包含・`right`/`bottom` 辺は非包含。座標 `x`/`y`（バルーン窓 client
/// **物理 px**・f32）を `HitRectPx` の各辺へ**そのまま（無変換で）**比較する。
///
/// # 無変換が正しい理由（k=1.0 だからではない）
///
/// `HitRectPx` は `areka_emo_text::choice::to_window_physical` が**既に実適用 k を掛けて
/// バルーン窓物理 px へ持ち上げた**矩形である（行内軸＝`(region 原点 + inline) × k`・ブロック軸
/// ＝`… × k + committed`）。点も窓 client 物理 px ゆえ、**両者は既に同一空間**にあり無変換で一致
/// する。成立根拠は「矩形側が ×k 済み」であって「k=1.0 だから」ではない——DPI追従により
/// k≠1.0 が実供給されても本経路は正しいままである。
///
/// シェル窓の当たり判定は逆向きで、「矩形は作者定義サーフェス px のまま・**点を ÷k**」する
/// （正準記述＝`areka_runtime::hit_region` の座標契約）。バルーンは「点はそのまま・**矩形を ×k**」
/// ——**逆向きだが等価に正しい**整合方式である。
///
/// **警告**: 本経路へ ÷k を追加すると **二重縮約**（矩形 ×k と点 ÷k の両掛け）になり、正常動作を
/// 破壊する。シェル側で k=1.0 限定契約が解除されたことを本経路へ一般化してはならない。
/// k=2.0 で持ち上げた行矩形×無変換点がヒットし、同点を ÷k すると外れることを、下段の in-source
/// 檻が固定する。
///
/// 判定対象は上流が供給する当たりの行（`rows`）のみ。行は選択肢（`\q`）とアンカー（`\_a`）が
/// 通し番号の順に混ざった列で、種類は各行の `kind` が持つ。
///
/// 判定は種類の 2 段: まず選択肢の行だけを見て、どれにも当たらなかったときにアンカーの行を見る。
/// 選択肢とアンカーが重なる所は、台本での定義の順に関わらず選択肢になる。
///
/// 同じ種類の中の重なりは**逆順走査の最初の一致＝スライス最終一致**の index を返す（`choice_hit_rows` は
/// ordinal 昇順×行昇順ゆえ「後定義が手前」＝画家のアルゴリズムと整合）。病的重なり入力
/// でも決定的に高々 1 つの index を返す。非ヒットは `None`。空 `rows` も `None`。
///
/// 戻り値はスライス index（呼び手が `rows[i].ordinal` 等へ展開する）。同一入力→同一出力（純粋・
/// 決定論・失敗経路なし）。
///
/// 呼び手は [`on_balloon_pointer_moved`]（hover 追従）と [`click_selection`] 経由の
/// [`on_balloon_pointer_pressed`]（クリック確定）。
pub(crate) fn hit_choice_row(rows: &[ChoiceHitRow], x: f32, y: f32) -> Option<usize> {
    // 逆順走査の最初の一致＝スライス最終一致（後定義が手前・画家のアルゴリズム）。
    // 半開区間 [left, right) × [top, bottom)：left/top は包含・right/bottom は非包含。
    // 座標は無変換で比較する——行矩形が to_window_physical で既に ×k 済みの窓物理 px ゆえ
    // 点と同一空間で一致する。ここへ ÷k を足すと二重縮約。
    let last_hit_of = |kind: SpanKind| {
        rows.iter()
            .enumerate()
            .rev()
            .find(|(_, row)| {
                let r = &row.rect;
                row.kind == kind && x >= r.left && x < r.right && y >= r.top && y < r.bottom
            })
            .map(|(i, _)| i)
    };
    // 選択肢が先。どの選択肢にも当たらなかったときだけアンカーを見る。
    last_hit_of(SpanKind::Choice).or_else(|| last_hit_of(SpanKind::Anchor))
}

/// hover 遷移の決定（純関数）。
///
/// 表示中フラグ・hit 結果（ordinal 展開済）・前回注入値の 3 入力から hover 遷移を
/// 決める副作用なしの決定的関数。呼び手が action を解釈する
/// （`Inject` で `inject_choice_hover`・`BalloonWiring.hover` 更新等）。
///
/// - `active == false`（choice 非表示）:
///   - `last_injected == None` → [`HoverAction::NoopInactive`]（未注入ゆえ何もしない）。
///   - `last_injected == Some(_)` → [`HoverAction::ResetOwnState`]（注入済ハイライトを消滅時に
///     自前状態のみ `None` 整合・inject はしない＝上流原子性が正本）。
///   - この分岐で `hit_ordinal` は無視される（非表示中は hover 追従なし）。
/// - `active == true`（choice 表示中）:
///   - `hit_ordinal == last_injected` → [`HoverAction::Keep`]（同値既注入・遷移なし・
///     `Some==Some`／`None==None` 双方を含む）。
///   - `hit_ordinal != last_injected` → [`HoverAction::Inject`]`(hit_ordinal)`（遷移・新値注入。
///     `Some(ordinal)`＝行ハイライト／`None`＝ハイライト解除）。
///
/// [`hover_action`] の返値を [`on_balloon_pointer_moved`] と [`clear_balloon_hover_on_leave`] の
/// 双方が解釈する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HoverAction {
    /// choice 非表示かつ自前状態も None——何もしない。
    NoopInactive,
    /// choice 非表示だが自前状態が残っている——自前状態のみ None へ整合
    /// （inject はしない・上流原子性が正本）。
    ResetOwnState,
    /// 表示中・hover 対象が前回注入値と同一——再注入しない（遷移なし）。
    Keep,
    /// 表示中・hover 対象が変化——`inject_choice_hover(actor, value)` を行う
    /// （`Some(ordinal)`＝行ハイライト・`None`＝ハイライト無し）。
    Inject(Option<usize>),
}

pub(crate) fn hover_action(
    active: bool, // 押せる範囲（選択肢かアンカー）があるか（`TextLayerRuntime::hit_active`）
    hit_ordinal: Option<usize>, // hit_choice_row の結果を ordinal へ展開した値
    last_injected: Option<usize>, // BalloonWiring.hover[scope]
) -> HoverAction {
    if !active {
        // choice 非表示中は hover 追従なし（hit_ordinal は無視）。
        return match last_injected {
            None => HoverAction::NoopInactive,
            Some(_) => HoverAction::ResetOwnState, // 消滅時は自前状態のみ None 整合（inject はしない）。
        };
    }
    // 表示中: 同値なら遷移なし・変化なら新値注入（Some=ハイライト/None=解除）。
    if hit_ordinal == last_injected {
        HoverAction::Keep
    } else {
        HoverAction::Inject(hit_ordinal)
    }
}

/// クリック確定の決定（純関数）。
///
/// クリック時点の**現行**行ジオメトリ（`rows`）のみから [`ChoiceSelection`] を構成する
/// 副作用なしの決定的関数。発行シンクへの送出・一度きり制御・ログは呼び手の領分。
///
/// `active` は「押せる範囲（選択肢かアンカー）があるか」（`TextLayerRuntime::hit_active`）。当たった
/// 行がアンカーなら、種類がアンカーの知らせを返す（[`hit_choice_row`] の順で選択肢が先）。以下の
/// 「choice」「非表示」は、アンカーだけが出ているときも同じに読む。
///
/// - `active == false`（choice 非表示）→ `None`（hit 判定より前に短絡）。
///   choice 消滅時の stale／原子性ガード＝非表示中はたとえ矩形内座標でも発行しない。
/// - `active == true` かつ非ヒット → `None`（[`hit_choice_row`] が `None`）。
/// - `active == true` かつヒット → **現行** `rows[i]`（`i` は [`hit_choice_row`] の
///   返す index）の各フィールドを clone 転写した [`ChoiceSelection`] を返す。
///   `id`/`label`/`references` は現行ヒット行から不透明転写（キャッシュ行からは決して
///   読まない）。`scope` は引数由来（`BalloonWindowMarker.scope`）。
///   `ordinal` はワイヤ形に含めない（漏洩防止）。
///
/// stale 棄却は本関数が**現行 rows のみ**を読むことで成立する: 以前 hover した
/// 座標に現行 rows のどの行も無ければ非ヒット＝`None`、別行が現れていればその現行行から
/// 構成される（キャッシュではなく現行ジオメトリが正本）。
///
/// [`on_balloon_pointer_pressed`] が押下ごとに呼ぶ。
pub(crate) fn click_selection(
    active: bool,
    rows: &[ChoiceHitRow],
    x: f32,
    y: f32,
    scope: usize,
) -> Option<ChoiceSelection> {
    // 非表示中は発行しない（hit 判定より前に短絡・消滅時 stale／原子性ガード）。
    if !active {
        return None;
    }
    // 現行 rows のヒット判定を再利用。stale 棄却は現行 rows のみを読むことで自然に成立する
    // （キャッシュ行は参照しない）。
    let i = hit_choice_row(rows, x, y)?;
    let hit = &rows[i];
    // 現行ヒット行から不透明転写（kind は写し・id/label/references は clone・scope は arg・ordinal 非含有）。
    Some(ChoiceSelection {
        kind: hit.kind,
        id: hit.id.clone(),
        label: hit.label.clone(),
        scope,
        references: hit.references.clone(),
    })
}

// ---------------------------------------------------------------------------
// post-spawn 装着・NonSend 結線・スケジュール登録
//
// donor `attach_char_pointer_handlers`（crates/areka-input/src/lib.rs）／`wire_mouse_input`（同）／main.rs の
// clickthrough 登録スロットの鏡写し。`spawn.rs` は不改変＝post-spawn 装着のみ。上流 input-events
// 成果（ハンドラ／排他システム／資源型）を消費し結線するだけで判断分岐は増設しない。
// ---------------------------------------------------------------------------

/// `BalloonWindowMarker` 全窓へ `OnPointerMoved`＋`OnPointerPressed` を post-spawn 挿入する
/// （donor `attach_char_pointer_handlers` の鏡写し・spawn.rs 不改変）。
///
/// 前提: `spawn_ghost_windows` 完了後（`BalloonWindowMarker` 窓が存在する状態）に同一 `&mut World`
/// クロージャ内で呼ぶ（キャラ窓ハンドラ装着と同型のタイミング契約）。
/// `&mut World` 借用中はクエリで別の可変借用を取れないため、まず対象 entity を収集してから 1 件ずつ
/// 挿入する。標的は `BalloonWindowMarker` 窓のみ——キャラ窓・その他 entity は一切触らない。
pub fn attach_balloon_pointer_handlers(world: &mut World) {
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

/// mpsc チャネルを生成し `BalloonWiring`＋`ChoiceSelectionInbox` を NonSend 挿入する
/// （`clear_balloon_hover_on_leave` の登録は [`register_balloon_leave_system`]）。
///
/// `ghost_session::boot_ghost` から `wire_mouse_input` と同型にゴーストごとに
/// **同期**（schedule 実行外）で呼ばれる（状態の載せ替えだけで、系は登録しない）。
/// `ChoiceSelectionInbox` の送り口は `ghost_session_runtime::rewire_runtime_consumers` の
/// `wire_choice_drain` が置き、毎フレームの drain の系は `register_choice_drain` が載せる。
pub fn wire_balloon_choice(world: &mut World) {
    let (tx, rx) = channel::<ChoiceSelection>();
    world.insert_non_send(BalloonWiring::new(tx));
    // バルーンの上の滞在の集合は runtime の型（可視性の相が読む）。`BalloonWiring` と同じ時に入れ直す。
    world.insert_non_send(areka_runtime::balloon_hover::BalloonHover::default());
    world.insert_non_send(ChoiceSelectionInbox(rx));
    // 箱の上の滞在もゴーストごとに入れ直す。
    world.insert_non_send(super::shell_box::ShellBoxHover::default());
}

/// `clear_balloon_hover_on_leave` を Input スケジュール（`dispatch_pointer_events` 後）へ登録する。
///
/// 高速離脱時の hover 残置は登録漏れとして実機目視でしか検出できないため、登録は本関数に集約し
/// スケジュール登録檻が開発時に捕捉する。ordering は `dispatch_pointer_events` の後
/// （FrameFinalize の `clear_transient_pointer_state` による `PointerLeave` 除去より前は Input
/// スケジュール内であることで成立）。
pub fn register_balloon_leave_system(world: &mut World) {
    world.resource_mut::<Schedules>().add_systems(
        Input,
        clear_balloon_hover_on_leave.after(dispatch_pointer_events),
    );
}

#[cfg(test)]
#[path = "balloon_choice_settle_tests.rs"]
mod choice_settle_tests;
#[cfg(test)]
#[path = "balloon_hover_flag_tests.rs"]
mod hover_flag_tests;
#[cfg(test)]
#[path = "balloon_leave_tests.rs"]
mod leave_tests;
#[cfg(test)]
#[path = "balloon_pass_through_tests.rs"]
mod pass_through_tests;
#[cfg(test)]
#[path = "balloon_pointer_handler_tests.rs"]
mod pointer_handler_tests;
#[cfg(test)]
#[path = "balloon_pure_core_tests.rs"]
mod pure_core_tests;
#[cfg(test)]
#[path = "balloon_test_support.rs"]
pub(super) mod test_support;
#[cfg(test)]
#[path = "balloon_wiring_tests.rs"]
mod wiring_tests;
