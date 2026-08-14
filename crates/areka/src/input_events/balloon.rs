//! バルーン選択肢対話配線（areka-P0-choice-interact）。
//!
//! バルーン窓のポインタイベントを捉え、選択肢ヒットの判定・ハイライト追従・クリック確定を
//! kanade／文字層 runtime へ橋渡しするサブモジュール。単一責務＝バルーン選択肢対話配線。
//!
//! 本 mod の構成要素はいずれも確立済みで、本番 `main.rs` から結線されている:
//! - 契約型（`ChoiceSelection` ほか）
//! - NonSend 資源（`BalloonWiring`・`ChoiceSelectionInbox`）
//! - 純関数判定核（選択肢ヒット・ハイライト遷移の決定的判定）
//! - 配線層（`on_balloon_pointer_moved`／`on_balloon_pointer_pressed`・
//!   `attach_balloon_pointer_handlers`・`wire_balloon_choice`）
//!
//! 本番の入口は 2 箇所——`wire_balloon_choice`（`main.rs` から 1 回・同期呼出）と
//! `attach_balloon_pointer_handlers`（`main.rs` の窓 spawn 直後クロージャ内）。
//! 本 mod の公開項目はこの 2 入口のいずれかから到達する（唯一の例外は
//! [`BalloonWiring::is_balloon_hovered`] で、こちらはバルーン可視性の相
//! ——`emo2_boot/frame.rs:170` から呼ばれる `run_balloon_visibility_phase`——が読む）。
//!
//! 上流契約（collision-geometry の resolver・`Emo2Wiring::runtime()` の読み口）は消費のみ行い、
//! 逆方向依存（上流が balloon.rs を知る）は禁止（design「依存方向」）。

use std::collections::{HashMap, HashSet};
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
/// `ordinal` はワイヤ形に含めない（漏洩防止・design 2.6）。
///
/// 本番到達済み——発行は [`on_balloon_pointer_pressed`]→[`BalloonWiring::send_selection`]、
/// 全フィールドの消費は `input_events/choice_drain.rs:42` の `to_choice_input`（`ChoiceInput` へ
/// 不透明転写）。`resolve_choice` を本 crate から呼ばない点は現在も変わらない（発行までが本 mod の
/// 範囲・カスケードは kanade 側）。
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
/// - `selection_tx`: 選択確定 [`ChoiceSelection`] の発行シンク（C-1・std mpsc）。発行は
///   [`ChoiceSelectionInbox`] へ流れ、W6 が入れた drain が kanade へ転送する（`resolve_choice` は
///   本 crate から直接呼ばない・5.3/5.4）。
/// - `hover`: scope→本仕様が最後に注入した hover ordinal の自前追跡（表示層に getter が無いため・
///   B-2）。表示状態の正本ではなく、(a) 同値再注入の遷移検出、(b) 選択肢消滅時の自前状態整合
///   （`None` 上書き・R3.4）のみに用いる。
///
/// 本番到達済み——挿入は [`wire_balloon_choice`]（`main.rs`）、消費はポインタハンドラ
/// （[`on_balloon_pointer_moved`]／[`on_balloon_pointer_pressed`]・`main.rs` の
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
/// 本番到達済み——構築は [`wire_balloon_choice`]（`main.rs`）、受信は W6 が入れた drain 排他
/// システム `input_events/choice_drain.rs:95` の `drain_choice_selections`（登録は同 `:126` の
/// `wire_choice_drain`・`main.rs` から呼ばれる）。「M1 では受信処理を持たない」という旧記述は
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
/// 本番到達済み——[`on_balloon_pointer_moved`]（hover 追従）と [`click_selection`] 経由の
/// [`on_balloon_pointer_pressed`]（クリック確定）が呼ぶ。両ハンドラは `main.rs` の
/// [`attach_balloon_pointer_handlers`] でバルーン窓へ装着される。
pub(crate) fn hit_choice_row(rows: &[ChoiceHitRow], x: f32, y: f32) -> Option<usize> {
    // 逆順走査の最初の一致＝スライス最終一致（後定義が手前・画家のアルゴリズム・DD-CI-5）。
    // 半開区間 [left, right) × [top, bottom)：left/top は包含・right/bottom は非包含。
    // 座標は無変換で比較する——行矩形が to_window_physical で既に実適用 k ×済みの窓物理 px ゆえ
    // 点と同一空間で一致する（k=1.0 だからではない）。ここへ ÷k を足すと二重縮約（R5.6/5.7・R6.4）。
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
/// 本番到達済み——[`hover_action`] の返値を [`on_balloon_pointer_moved`]（`main.rs` で装着）と
/// [`clear_balloon_hover_on_leave`]（`main.rs` 経由で Input スケジュールへ登録）の双方が解釈する。
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
/// 本番到達済み——[`on_balloon_pointer_pressed`]（`main.rs` の
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
/// 本番到達済み——[`attach_balloon_pointer_handlers`]（`main.rs` から呼ばれる）が
/// `BalloonWindowMarker` 窓へ `OnPointerMoved` として挿入する。
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

    // ── 借用規律 ② BalloonWiring へ滞在を記録し last_injected を copy（可変借用即解放）───────────
    // 滞在の記録（areka-P0-balloon-visibility 5.2）は選択肢機構と**独立の軸**——選択肢の表示有無・
    // 行ヒットの有無に依らず、バルーン窓上の移動そのもので当該 scope を真にする（ゆえに選択肢
    // スナップショット ③ より前で行い、③ の縮退に巻き込まれない）。解除は窓外離脱
    // （`clear_balloon_hover_on_leave`）と非表示遷移時の掃除（`BalloonWiring::clear_balloon_hover`）。
    // 読み口 `is_balloon_hovered` はバルーン可視性の相が毎フレーム読む（抑止条件の観測）。
    // BalloonWiring 不在は結線漏れ＝構成異常 error!（配線存在檻が開発時に検出）＋no-op。
    let Some(last_injected) = world
        .get_non_send_resource_mut::<BalloonWiring>()
        .map(|mut bw| {
            bw.set_balloon_hover(scope);
            bw.hover(scope)
        })
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
/// `RefCell` は `try_borrow` のみ（panic しない・log-first）。座標は窓 client 物理 px を**無変換**で
/// 照合する——行矩形が `to_window_physical` で既に実適用 k ×済みゆえ同一空間で一致する（k=1.0 だから
/// ではない）。シェルの「点 ÷k」とは逆向きだが等価に正しく、本経路へ ÷k を足すと二重縮約になり正常
/// 動作を壊す（R5.6/5.7・R6.4。詳細は [`hit_choice_row`] の座標契約 doc）。
///
/// **戻り値**: `ChoiceSelection` を発行したときのみ `true`（棄却・縮退・非左押下・Tunnel 時は `false`）。
///
/// 本番到達済み——[`attach_balloon_pointer_handlers`]（`main.rs` から呼ばれる）が
/// `BalloonWindowMarker` 窓へ `OnPointerPressed` として挿入する。
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

    // client 物理 px（i32）を f32 へ——無変換のまま渡す。行矩形が to_window_physical で既に実適用
    // k ×済みの窓物理 px ゆえ同一空間で一致する（k=1.0 だからではない）。÷k 追加は二重縮約（R6.4）。
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
/// 併せて当該 scope の**バルーン滞在**（`BalloonWiring::clear_balloon_hover`・
/// areka-P0-balloon-visibility 5.2）を解除する。滞在は選択肢行の追跡とは独立の軸であり、解除は
/// `Emo2Wiring` の可否より前・選択肢機構の縮退に依らず行う（記録側との非対称は本文中コメント参照・5.5）。
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
/// 本番到達済み——[`register_balloon_leave_system`]（[`wire_balloon_choice`] 経由・
/// `main.rs`）が Input スケジュールへ `dispatch_pointer_events` の後として登録するため、
/// 以降は毎フレーム走る。
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

    // ── バルーン滞在の解除（areka-P0-balloon-visibility 5.2）——選択肢機構より前 ────────────────
    // 記録側（`on_balloon_pointer_moved` の借用規律 ②）と縮退方向を意図的に非対称にする: 記録は
    // 上流 `Emo2Wiring` が揃うときだけ行い（抑止を増やす側は保守的）、解除はその可否に依らず行う
    // （抑止を解く側は積極的）。滞在が真のまま残ると恒久抑止へ固着し Requirement 5.5 に反するため。
    // BalloonWiring 不在は結線漏れ＝構成異常 error!＋no-op（silent failure 禁止）。
    if let Some(mut bw) = world.get_non_send_resource_mut::<BalloonWiring>() {
        for &scope in &scopes {
            bw.clear_balloon_hover(scope);
        }
    } else {
        tracing::error!(
            event = "balloon_wiring_missing",
            scopes = ?scopes,
            "BalloonWiring 不在（結線漏れ）: バルーン滞在の解除を no-op 縮退"
        );
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
// 結線済み——`wire_balloon_choice` は `main.rs` から `wire_mouse_input` と同型に schedule 実行外で
// 1 回同期呼出され、`attach_balloon_pointer_handlers` は `main.rs`（窓 spawn 直後の同一
// `&mut World` クロージャ内）から呼ばれる。
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
/// 本番到達済み——`main.rs`（`attach_char_pointer_handlers` の直後・`spawn_ghost_windows` と
/// 同一 `&mut World` クロージャ内）から呼ばれる。
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
/// 本番到達済み——`main.rs` から `wire_mouse_input` と同型に **1 回・同期**（schedule 実行外）で
/// 呼ばれる。同期呼出ゆえ実行中スケジュールを触らず、`Schedules` 資源が既在の World（`EcsWorld` 内
/// World）で成立する（donor clickthrough 登録＝`app.world().add_systems(...)` と同じ作法）。発行シンク
/// `ChoiceSelectionInbox` の受信は W6 `choice-select-events` が着地済みで、`main.rs` の
/// `wire_choice_drain` が毎フレームの drain を登録する（5.3 の seam は消費済み）。
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
/// 本番到達済み——唯一の呼び手 [`wire_balloon_choice`] が `main.rs` から呼ばれるため間接に到達する。
fn register_balloon_leave_system(world: &mut World) {
    world.resource_mut::<Schedules>().add_systems(
        Input,
        clear_balloon_hover_on_leave.after(dispatch_pointer_events),
    );
}

#[cfg(test)]
#[path = "balloon_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "balloon_pure_core_tests.rs"]
mod pure_core_tests;
#[cfg(test)]
#[path = "balloon_pointer_handler_tests.rs"]
mod pointer_handler_tests;
#[cfg(test)]
#[path = "balloon_leave_tests.rs"]
mod leave_tests;
#[cfg(test)]
#[path = "balloon_wiring_tests.rs"]
mod wiring_tests;
#[cfg(test)]
#[path = "balloon_pass_through_tests.rs"]
mod pass_through_tests;
#[cfg(test)]
#[path = "balloon_hover_flag_tests.rs"]
mod hover_flag_tests;
