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
use std::sync::mpsc::{Receiver, Sender};

use areka_emo_text::actor::ChoiceHitRow;

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
}
