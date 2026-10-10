//! バルーン選択肢対話配線（areka-P0-choice-interact）。
//!
//! バルーン窓のポインタイベントを捉え、選択肢ヒットの判定・ハイライト追従・クリック確定を
//! kanade／文字層 runtime へ橋渡しするサブモジュール。単一責務＝バルーン選択肢対話配線。
//!
//! 本 mod の構成要素はいずれも確立済みで、本番の起動（`ghost_session`）から結線されている:
//! - 契約型（`ChoiceSelection` ほか）
//! - NonSend 資源（`BalloonWiring`・`ChoiceSelectionInbox`）
//! - 純関数判定核（選択肢ヒット・ハイライト遷移の決定的判定）
//! - 配線層（`on_balloon_pointer_moved`／`on_balloon_pointer_pressed`・
//!   `attach_balloon_pointer_handlers`・`wire_balloon_choice`）
//!
//! 本番の入口は 3 箇所——離脱の系の登録 `register_balloon_leave_system`（`ghost_session::register_systems`
//! からプロセスに 1 回）・状態の載せ替え `wire_balloon_choice`（`ghost_session::boot_ghost` から
//! ゴーストごとに n 回・同期呼出）・`attach_balloon_pointer_handlers`（`ghost_session::prepare_ghost_windows`
//! の窓 spawn 直後クロージャ内）。
//! 本 mod の公開項目はこの 3 入口のいずれかから到達する（唯一の例外は
//! [`BalloonWiring::is_balloon_hovered`] で、こちらはバルーン可視性の相
//! ——`emo2_boot/frame.rs:170` から呼ばれる `run_balloon_visibility_phase`——が読む）。
//!
//! 上流契約（collision-geometry の resolver・`Emo2Wiring::runtime()` の読み口）は消費のみ行い、
//! 逆方向依存（上流が balloon.rs を知る）は禁止（design「依存方向」）。

use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};

use areka_emo_text::actor::ChoiceHitRow;
use areka_emo_text::state::SpanKind;
#[cfg(test)]
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed, dispatch_pointer_events};

use crate::placement::spawn::BalloonWindowMarker;

use super::user_break;

#[path = "balloon_exit.rs"]
mod exit;
#[path = "balloon_moved.rs"]
mod moved;
#[path = "balloon_pressed.rs"]
mod pressed;

pub(crate) use exit::clear_balloon_hover_on_leave;
pub(crate) use moved::on_balloon_pointer_moved;
pub(crate) use pressed::on_balloon_pointer_pressed;

/// 選択確定のワイヤ形（本 spec 契約正本・2.2）。
///
/// 下流 W6 が表示層へ再照会せず選択解決とカスケード発火を組み立てられる自己完結データ。
/// 解決キーは `id`（`SakuraMsg::ResolveChoice { id }` と整合）であり、表示層内部の主キーである
/// `ordinal` はワイヤ形に含めない（漏洩防止・design 2.6）。
///
/// 本番到達済み——発行は [`on_balloon_pointer_pressed`]→[`BalloonWiring::send_selection`]、
/// 全フィールドの消費は `input_events/choice_drain.rs` の `to_choice_input`（`ChoiceInput` へ
/// 不透明転写）と、種類がアンカーのときの `to_anchor_input`（`AnchorInput` へ同じく転写）。
/// `resolve_choice` を本 crate から呼ばない点は現在も変わらない（発行までが本 mod の
/// 範囲・カスケードは kanade 側）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChoiceSelection {
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

/// バルーン選択肢対話の配線資源（NonSend・donor `MouseWiring` 同型・2.2）。
///
/// UI スレッド所有の資源として `World` へ NonSend 挿入し（`insert_non_send`）、Input
/// スケジュール排他システム内でのみ借用する（donor `MouseWiring` と同型）。mpsc `Sender` は
/// `Send` だが `hover` 追跡と一体で UI スレッド固定運用ゆえ NonSend 1 個に束ねる。
///
/// 2 つの用途を束ねる:
/// - `selection_tx`: 選択確定 [`ChoiceSelection`] の発行シンク（C-1・std mpsc）。発行は
///   [`ChoiceSelectionInbox`] へ流れ、W6 が入れた drain が kanade へ転送する（`resolve_choice` は
///   本 crate から直接呼ばない・5.3/5.4）。
/// - `hover`: scope→本仕様が最後に注入した hover ordinal の自前追跡（表示層に getter が無いため・
///   B-2）。表示状態の正本ではなく、(a) 同値再注入の遷移検出、(b) 選択肢消滅時の自前状態整合
///   （`None` 上書き・R3.4）のみに用いる。
///
/// 本番到達済み——挿入は [`wire_balloon_choice`]（`ghost_session::boot_ghost` からゴーストごと）、
/// 消費はポインタハンドラ（[`on_balloon_pointer_moved`]／[`on_balloon_pointer_pressed`]・
/// `ghost_session::prepare_ghost_windows` の
/// [`attach_balloon_pointer_handlers`] で装着）と離脱システム [`clear_balloon_hover_on_leave`]。
/// 3 フィールドはいずれもそれらの経路で読み書きされる。
pub(crate) struct BalloonWiring {
    /// [`ChoiceSelection`] 発行シンク（C-1・mpsc）。
    selection_tx: Sender<ChoiceSelection>,
    /// scope → 最後に注入した hover ordinal（getter 不在の自前追跡・B-2）。
    hover: HashMap<usize, Option<usize>>,
    /// ポインタがバルーン窓の上に居る scope の集合（areka-P0-balloon-visibility 5.2）。
    ///
    /// `hover` とは**別概念の独立した軸**——`hover` は選択肢行の追跡（どの行を光らせたか）であり、
    /// 本集合は「バルーンの上に居るか」だけを表す（選択肢の有無・行ヒットの有無に依存しない）。
    balloon_hover: HashSet<usize>,
}

impl BalloonWiring {
    /// 発行シンク [`Sender`] から構築する（`hover` は空 map で初期化・donor `MouseWiring::new` 同型）。
    pub(crate) fn new(selection_tx: Sender<ChoiceSelection>) -> Self {
        Self {
            selection_tx,
            hover: HashMap::new(),
            balloon_hover: HashSet::new(),
        }
    }

    /// 選択確定 [`ChoiceSelection`] を発行シンクへ送る（一度きり発行・2.4／log-first）。
    ///
    /// [`ChoiceSelectionInbox`] の `Receiver` が生存する限り成功する（`resolve_choice` は呼ばない・
    /// 5.3/5.4）。送出失敗（受け口消滅後の [`Sender`] エラー）は warn＋no-op（`false` 返し・log-first）。
    pub(crate) fn send_selection(&self, selection: ChoiceSelection) -> bool {
        let scope = selection.scope;
        let (_, send_failed) = selection_events(selection.kind);
        if self.selection_tx.send(selection).is_err() {
            tracing::warn!(
                event = send_failed,
                scope,
                "ChoiceSelection 発行シンク送出失敗（受け口消滅後）: no-op で継続"
            );
            return false;
        }
        true
    }

    /// scope の最後に注入した hover ordinal を回収する（未注入は `None`・B-2）。
    ///
    /// [`on_balloon_pointer_moved`] の遷移検出（同値再注入の抑制）と
    /// [`clear_balloon_hover_on_leave`] の消滅時整合（R3.4）が参照する（いずれも本番到達済み）。
    pub(crate) fn hover(&self, scope: usize) -> Option<usize> {
        self.hover.get(&scope).copied().flatten()
    }

    /// scope の hover ordinal を記録する（`None` 上書きは消滅時整合・R3.4）。
    ///
    /// 注入は「本仕様が最後に注入した値」の記録であり表示状態の正本ではない（正本は上流）。
    pub(crate) fn set_hover(&mut self, scope: usize, ordinal: Option<usize>) {
        self.hover.insert(scope, ordinal);
    }

    /// scope のバルーン窓上にポインタが居るかを照会する（areka-P0-balloon-visibility 5.2）。
    ///
    /// タイムアウト抑止の判断側（バルーン可視性コントローラ）が読む口。未観測の scope は偽。
    ///
    /// 本番到達済み（実測）——バルーン可視性の相が毎フレームの観測収集で呼ぶ
    /// （`emo2_boot/balloon_visibility_phase.rs` の `collect_observations`）。到達の起点は
    /// `emo2_boot/frame.rs:170` の `run_balloon_visibility_phase` 呼び出しである。
    pub(crate) fn is_balloon_hovered(&self, scope: usize) -> bool {
        self.balloon_hover.contains(&scope)
    }

    /// scope のバルーン窓上へポインタが入った（居る）ことを記録する（5.2）。
    ///
    /// 選択肢行の追跡（[`set_hover`](Self::set_hover)）とは独立——行に当たっていなくても記録する。
    pub(crate) fn set_balloon_hover(&mut self, scope: usize) {
        self.balloon_hover.insert(scope);
    }

    /// scope のバルーン滞在の記録を落とす（離脱・非表示遷移時の掃除・5.2/5.5）。
    ///
    /// 窓外離脱（`PointerLeave`）のほか、可視性コントローラが非表示遷移で呼ぶ掃除口でもある——
    /// 不可視の間は `PointerLeave` が届かず、放置すると滞在が真のまま固着して恒久抑止になる
    /// （Requirement 5.5 が禁じる側）。未記録 scope への呼出は no-op（冪等）。
    pub(crate) fn clear_balloon_hover(&mut self, scope: usize) {
        self.balloon_hover.remove(&scope);
    }
}

/// 選択確定通知の受け口（5.3 の seam。W6 `choice-select-events` が受信処理を入れて消費済み）。
///
/// `Receiver` 生存により [`BalloonWiring::send_selection`] は `Err` にならず、発行の mpsc 観測と
/// 実機ログが成立する。
///
/// 本番到達済み——構築は [`wire_balloon_choice`]（`ghost_session::boot_ghost` からゴーストごと）、
/// 受信は W6 が入れた drain 排他システム `choice_drain.rs` の `drain_choice_selections`（系の登録は
/// `register_choice_drain` を `ghost_session::register_systems` からプロセスに 1 回・送り口は
/// `wire_choice_drain` が `boot_ghost` からゴーストごとに置く）。「M1 では受信処理を持たない」という旧記述は
/// W6 の着地で無効になったため撤去した。
pub(crate) struct ChoiceSelectionInbox(pub(crate) Receiver<ChoiceSelection>);

/// 点包含 hit 判定（純関数・R1.1/1.5/2.3）。
///
/// 包含は半開区間 `[left, right) × [top, bottom)`（whole-pixel 行矩形と整合）——
/// `left`/`top` 辺は包含・`right`/`bottom` 辺は非包含。座標 `x`/`y`（バルーン窓 client
/// **物理 px**・f32）を `HitRectPx` の各辺へ**そのまま（無変換で）**比較する。
///
/// # 無変換が正しい理由（k=1.0 だからではない・R5.6/5.7・R6.4）
///
/// `HitRectPx` は `areka_emo_text::choice::to_window_physical` が**既に実適用 k を掛けて
/// バルーン窓物理 px へ持ち上げた**矩形である（行内軸＝`(region 原点 + inline) × k`・ブロック軸
/// ＝`… × k + committed`）。点も窓 client 物理 px ゆえ、**両者は既に同一空間**にあり無変換で一致
/// する。すなわち成立根拠は「矩形側が ×k 済み」であって「k=1.0 だから」ではない——DPI追従により
/// k≠1.0 が実供給されても本経路は正しいままである。
///
/// シェル窓の当たり判定は逆向きで、「矩形は作者定義サーフェス px のまま・**点を ÷k**」する
/// （正準記述＝`crate::emo2_boot::hit_region` の座標契約）。バルーンは「点はそのまま・**矩形を ×k**」
/// ——**逆向きだが等価に正しい**整合方式である。
///
/// **警告**: 本経路へ ÷k を追加すると **二重縮約**（矩形 ×k と点 ÷k の両掛け）になり、正常動作を
/// 破壊する。シェル側で k=1.0 限定契約が解除されたことを本経路へ一般化してはならない（R6.4 が
/// 明文で禁じる）。不変条件は下段の in-source 檻（k=2.0 で持ち上げた行矩形×無変換点＝ヒット／
/// 同点を ÷k すると外れる・R3.7）が固定する。
///
/// 判定対象は上流が供給する当たりの行（`rows`）のみ。行は選択肢（`\q`）とアンカー（`\_a`）が
/// 通し番号の順に混ざった列で、種類は各行の `kind` が持つ。
///
/// 判定は種類の 2 段（areka-P0-anchor-tag-canon 要件 3.5）: まず選択肢の行だけを見て、どれにも
/// 当たらなかったときにアンカーの行を見る。選択肢とアンカーが重なる所は、台本での定義の順に
/// 関わらず選択肢になる。
///
/// 同じ種類の中の重なりは**逆順走査の最初の一致＝スライス最終一致**の index を返す（`choice_hit_rows` は
/// ordinal 昇順×行昇順ゆえ「後定義が手前」＝画家のアルゴリズムと整合・DD-CI-5）。病的重なり入力
/// でも決定的に高々 1 つの index を返す（R1.1／1.5）。非ヒットは `None`（R2.3）。空 `rows` も `None`。
///
/// 戻り値はスライス index（呼び手が `rows[i].ordinal` 等へ展開する）。同一入力→同一出力（純粋・
/// 決定論・失敗経路なし）。
///
/// 本番到達済み——[`on_balloon_pointer_moved`]（hover 追従）と [`click_selection`] 経由の
/// [`on_balloon_pointer_pressed`]（クリック確定）が呼ぶ。両ハンドラは
/// `ghost_session::prepare_ghost_windows` の [`attach_balloon_pointer_handlers`] でバルーン窓へ装着される。
pub(crate) fn hit_choice_row(rows: &[ChoiceHitRow], x: f32, y: f32) -> Option<usize> {
    // 逆順走査の最初の一致＝スライス最終一致（後定義が手前・画家のアルゴリズム・DD-CI-5）。
    // 半開区間 [left, right) × [top, bottom)：left/top は包含・right/bottom は非包含。
    // 座標は無変換で比較する——行矩形が to_window_physical で既に実適用 k ×済みの窓物理 px ゆえ
    // 点と同一空間で一致する（k=1.0 だからではない）。ここへ ÷k を足すと二重縮約（R5.6/5.7・R6.4）。
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
/// 本番到達済み——[`hover_action`] の返値を [`on_balloon_pointer_moved`]（`ghost_session::prepare_ghost_windows`
/// で装着）と [`clear_balloon_hover_on_leave`]（`ghost_session::register_systems` から Input へ登録）の
/// 双方が解釈する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

pub(crate) fn hover_action(
    active: bool, // 押せる範囲（選択肢かアンカー）があるか（`TextLayerRuntime::hit_active`）
    hit_ordinal: Option<usize>, // hit_choice_row の結果を ordinal へ展開した値
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
/// `active` は「押せる範囲（選択肢かアンカー）があるか」（`TextLayerRuntime::hit_active`）。当たった
/// 行がアンカーなら、種類がアンカーの知らせを返す（[`hit_choice_row`] の順で選択肢が先）。以下の
/// 「choice」「非表示」は、アンカーだけが出ているときも同じに読む。
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
/// 本番到達済み——[`on_balloon_pointer_pressed`]（`ghost_session::prepare_ghost_windows` の
/// [`attach_balloon_pointer_handlers`] でバルーン窓へ装着）が押下ごとに呼ぶ。
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
// post-spawn 装着・NonSend 結線・スケジュール登録（tasks.md task 6.1・design
// 「attach_balloon_pointer_handlers / wire_balloon_choice」＋「clear_balloon_hover_on_leave」Validation）
//
// donor `attach_char_pointer_handlers`（input_events/mod.rs）／`wire_mouse_input`（同）／main.rs の
// clickthrough 登録スロットの鏡写し。`spawn.rs` は不改変＝post-spawn 装着のみ（4.4）。上流 input-events
// 成果（ハンドラ／排他システム／資源型）を消費し結線するだけで判断分岐は増設しない（5.5）。本番呼出は
// 結線済み——系の登録（`register_balloon_leave_system`）は `ghost_session::register_systems` から
// プロセスに 1 回、状態の載せ替え（`wire_balloon_choice`）は `ghost_session::boot_ghost` から
// `wire_mouse_input` と同型に schedule 実行外でゴーストごとに n 回同期呼出され、
// `attach_balloon_pointer_handlers` は `ghost_session::prepare_ghost_windows`（窓 spawn 直後の同一
// `&mut World` クロージャ内）から呼ばれる。
// ---------------------------------------------------------------------------

/// `BalloonWindowMarker` 全窓へ `OnPointerMoved`＋`OnPointerPressed` を post-spawn 挿入する
/// （donor `attach_char_pointer_handlers` の鏡写し・spawn.rs 不改変・R4.3/4.4）。
///
/// 前提: `spawn_ghost_windows` 完了後（`BalloonWindowMarker` 窓が存在する状態）に同一 `&mut World`
/// クロージャ内で呼ぶ（キャラ窓ハンドラ装着と同型のタイミング契約・`ghost_session::prepare_ghost_windows`
/// の spawn 直後結線）。
/// `&mut World` 借用中はクエリで別の可変借用を取れないため、まず対象 entity を収集してから 1 件ずつ
/// 挿入する（donor 同型）。標的は `BalloonWindowMarker` 窓のみ——キャラ窓・その他 entity は一切
/// 触らない（配線の非退行・R4.3）。
///
/// 本番到達済み——`ghost_session::prepare_ghost_windows`（`attach_char_pointer_handlers` の直後・
/// `spawn_ghost_windows` と同一 `&mut World` クロージャ内）から呼ばれる。
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

/// mpsc チャネルを生成し `BalloonWiring`＋`ChoiceSelectionInbox` を NonSend 挿入する
/// （`clear_balloon_hover_on_leave` の登録は [`register_balloon_leave_system`]・donor `wire_mouse_input`＋main.rs clickthrough 登録の合成・design Option A・R5.5/6.6）。
///
/// 本番到達済み——`ghost_session::boot_ghost` から `wire_mouse_input` と同型にゴーストごとに
/// **同期**（schedule 実行外）で呼ばれる（状態の載せ替え＝n 回。系は登録しない＝離脱の系の登録は
/// `ghost_session::register_systems` からプロセスに 1 回）。同期呼出ゆえ実行中スケジュールを触らない。
/// 発行シンク `ChoiceSelectionInbox` の受信は W6 `choice-select-events` が着地済みで、送り口は
/// `ghost_session::boot_ghost` の `wire_choice_drain` が置き、毎フレームの drain の系は
/// `register_choice_drain`（`register_systems` から 1 回）が載せる（5.3 の seam は消費済み）。
pub(crate) fn wire_balloon_choice(world: &mut World) {
    let (tx, rx) = channel::<ChoiceSelection>();
    world.insert_non_send(BalloonWiring::new(tx));
    world.insert_non_send(ChoiceSelectionInbox(rx));
    // 箱の上の滞在（areka-P0-shell-balloon）もゴーストごとに入れ直す。
    world.insert_non_send(super::shell_box::ShellBoxHover::default());
}

/// `clear_balloon_hover_on_leave` を Input スケジュール（`dispatch_pointer_events` 後）へ登録する
/// （main.rs clickthrough 登録スロットの donor 同型・design Integration Test 7・R6.6）。
///
/// 高速離脱時の hover 残置は登録漏れとして実機目視でしか検出できないため、登録は本関数に集約し
/// スケジュール登録檻が開発時に捕捉する（design Testing Strategy Integration Test 7）。ordering は
/// `dispatch_pointer_events` の後（FrameFinalize の `clear_transient_pointer_state` による `PointerLeave`
/// 除去より前は Input スケジュール内であることで成立）。
///
/// 本番到達済み——呼び手は `ghost_session::register_systems`（プロセスに 1 回）。
pub(crate) fn register_balloon_leave_system(world: &mut World) {
    world.resource_mut::<Schedules>().add_systems(
        Input,
        clear_balloon_hover_on_leave.after(dispatch_pointer_events),
    );
}

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
