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
}
