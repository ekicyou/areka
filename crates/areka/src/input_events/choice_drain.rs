//! 選択確定通知の受信結線（areka-P0-choice-select-events・design C1 ChoiceDrain）。
//!
//! UI スレッドの受信口 [`ChoiceSelectionInbox`]（`wire_balloon_choice` が挿入・所有は
//! choice-interact spec）を毎フレーム drain し、`KanadeMsg::Choice` へ写像して kanade へ
//! 転送する薄い配線層。**判断・フィルタ・重複排除は一切行わない**——一回性の調停は
//! kanade 側が単一真実源であり（Req1.1・design C4）、上流 balloon 押下ハンドラは既に
//! 一度きり発行を保証している。本層は**到着順に全件を素通し**する（Req1.2）。
//!
//! 判断分岐は「送出失敗（kanade 停止後）で warn 記録して継続する」1 点のみ（Req1.6）で、
//! それと写像純関数 [`to_choice_input`] だけが檻の対象（ECS 結線は再テストしない・steering
//! `test-only-decision-branches-not-proven-wiring`）。

use std::sync::mpsc::{Receiver, Sender};

use areka_kanade::{ChoiceInput, KanadeMsg};
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use super::balloon::{ChoiceSelection, ChoiceSelectionInbox};

/// kanade への投函端（NonSend・UI スレッド所有・donor `MouseWiring` 同型）。
///
/// `Sender<KanadeMsg>` 単体は `Send` だが、受信口 [`ChoiceSelectionInbox`]（`Receiver` ゆえ
/// `!Sync`）と対で UI スレッド固定運用ゆえ NonSend 資源として挿入する（`MouseWiring` 前例）。
pub(crate) struct ChoiceForwarder {
    /// `GhostRuntime::kanade()` クローン（main.rs 結線・std mpsc）。
    kanade: Sender<KanadeMsg>,
}

/// 選択確定通知 → kanade 入力への写像（純関数・不透明転写・Req1.5/3.7・DD-13）。
///
/// `id` / `label` / `references` は**改変せず move で転写**する——トリム・正規化・
/// 空要素除去・順序変更・表示層への再照会はいずれも行わない（Req1.5）。`scope` は
/// `usize`→`u32` の型合わせのみ（M1 の実値は {0,1}）。
///
/// `scope` は下流でも**検証・ログにのみ**用いられ Reference には載らない（Req3.7・DD-13）。
/// 本関数は搬送するだけで用途判断を持たない。
///
/// 同一入力→同一出力（純粋・決定論・失敗経路なし）。
fn to_choice_input(sel: ChoiceSelection) -> ChoiceInput {
    ChoiceInput {
        // id / label / references は move で素通し（clone も加工もしない＝改変の余地を作らない）。
        id: sel.id,
        label: sel.label,
        // 型合わせのみ（M1 の scope 実値は {0,1}）。用途限定の判断は下流（DD-13）。
        scope: sel.scope as u32,
        references: sel.references,
    }
}

/// 受信口を `try_recv` で drain し、到着順に全件を kanade へ転送する（Req1.2/1.6）。
///
/// ECS から切り離した純粋な形（`Receiver` と `Sender` のみを取る）——排他システム
/// [`drain_choice_selections`] はこの関数へ委譲するだけの配線。
///
/// - **判断・フィルタ・重複排除をしない**: 受領した通知は 1 件残らず送出を試みる（Req1.2）。
/// - **送出失敗で止まらない**: `warn!(event = "choice_forward_failed")` を記録して**次の件へ継続**
///   する（kanade 停止後は終了系で正常・Req1.6・log-first）。
///
/// 戻り値は送出に成功した件数（呼び手は使わないが檻の観測点）。
fn forward_all(inbox: &Receiver<ChoiceSelection>, kanade: &Sender<KanadeMsg>) -> usize {
    let mut forwarded = 0usize;
    // try_recv ループ＝到着順に全件（滞留は 1 件も残さない・Req1.2）。
    while let Ok(sel) = inbox.try_recv() {
        // scope は送出後に使えないため、ログ用へ先に控える（写像は所有権を消費する）。
        let scope = sel.scope;
        let input = to_choice_input(sel);
        let id = input.id.clone();
        if kanade.send(KanadeMsg::Choice(input)).is_err() {
            // kanade 停止後（終了系では正常）。無記録で打ち切らず、記録して次の件へ継続する（Req1.6）。
            tracing::warn!(
                event = "choice_forward_failed",
                scope,
                id = %id,
                "kanade Sender 送出失敗（actor 停止後）: 記録して次の通知へ継続"
            );
            continue;
        }
        forwarded += 1;
    }
    forwarded
}

/// 受信口 drain の排他システム（Input スケジュール・design C1）。
///
/// NonSend の [`ChoiceSelectionInbox`]（`wire_balloon_choice` 挿入）と [`ChoiceForwarder`]
/// （[`wire_choice_drain`] 挿入）をともに共有借用し（両方 `&World` 借用ゆえ両立・donor
/// `resolve_region_owned` と同じ借用作法）、[`forward_all`] へ委譲する。
///
/// 資源不在は self-gating の no-op（`trace!`＋return・donor `mouse_moved_no_wiring` 同型）。
/// [`wire_choice_drain`] は `wire_balloon_choice` の直後に呼ばれるため理論上不到達だが、
/// 毎フレーム走る排他システムゆえ panic せず無記録でもなく縮退する（log-first）。
pub(crate) fn drain_choice_selections(world: &mut World) {
    let Some(inbox) = world.get_non_send::<ChoiceSelectionInbox>() else {
        tracing::trace!(
            event = "choice_drain_no_inbox",
            "ChoiceSelectionInbox 不在（wire_balloon_choice 前）: drain を no-op 縮退"
        );
        return;
    };
    let Some(forwarder) = world.get_non_send::<ChoiceForwarder>() else {
        tracing::trace!(
            event = "choice_drain_no_forwarder",
            "ChoiceForwarder 不在（wire_choice_drain 前）: drain を no-op 縮退"
        );
        return;
    };
    forward_all(&inbox.0, &forwarder.kanade);
}

/// kanade 投函端を NonSend 挿入し、drain 排他システムを Input スケジュールへ登録する
/// （design C1 Contracts・donor `wire_mouse_input` 同型）。
///
/// main.rs の `wire_balloon_choice` 呼出**直後**から `wire_mouse_input` と同型に **1 回・同期**
/// （schedule 実行外の World 変更）で呼ばれる。同期呼出ゆえ実行中スケジュールを触らず、
/// `Schedules` 資源が既在の World で成立する。
///
/// ordering は `dispatch_pointer_events` の**後**——押下ハンドラが同一フレームで発行した
/// [`ChoiceSelection`] をそのフレームのうちに転送する（1 フレーム遅延を作らない・
/// `register_balloon_leave_system` と同じ ordering 作法）。
///
/// Precondition: `wire_balloon_choice` 済み（[`ChoiceSelectionInbox`] 存在）。
/// Postcondition: 以降のフレームで受信済み通知は全件送出試行済み・失敗は warn 記録。
pub(crate) fn wire_choice_drain(world: &mut World, kanade: Sender<KanadeMsg>) {
    world.insert_non_send(ChoiceForwarder { kanade });
    world.resource_mut::<Schedules>().add_systems(
        Input,
        drain_choice_selections.after(dispatch_pointer_events),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use log_capture_kit::{LineFormat, capture_lines};
    use std::sync::mpsc::{self, TryRecvError};

    /// 不透明転写の弁別に足る「汚い」値を持つ選択確定通知を組む。
    ///
    /// 非 ASCII・前後空白・空文字要素・記号・さくらスクリプト風文字列を含めることで、
    /// トリム／正規化／空要素除去が入れば必ず落ちる fixture にする。
    fn dirty_selection() -> ChoiceSelection {
        ChoiceSelection {
            id: "  Onおしゃべり頻度メニュー  ".to_string(),
            label: " \\q[もどる,back] ＞＞ ".to_string(),
            scope: 1,
            references: vec![
                "  前後に空白  ".to_string(),
                String::new(),
                "\\s[0]\\n\\e".to_string(),
                "ＡＢＣ・全角／記号".to_string(),
                "  ".to_string(),
            ],
        }
    }

    // ── ログ発火を決定論的に観測するための捕捉（捕捉層は共有機構へ委譲）──
    //
    // 捕捉層そのものはここに持たず、硬化機構の唯一の定義元 `log-capture-kit` へ委譲する。
    // 行の形（1 イベント 1 行・`level=…` に続けてフィールドを訪問順で ` name=value`）も
    // 呼出側の判定内容も、移行前と 1 バイト変わらない。
    //
    // 「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」は**誤り**である。差し替わる
    // のはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
    // interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点を最初に踏んだスレッドの
    // 判定が焼き付く（先着が勝つ）。捕捉窓を持たないスレッドの既定は `NoSubscriber` で判定は
    // 「不要」ゆえ、先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても
    // 取りこぼす。共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 捕捉窓の内側での interest 再計算
    // ⑶ 番兵イベントによる空振り検出 の 3 点でこれを塞ぐ（機序の逐条解説は `log_capture_kit` の
    // crate doc と同 crate の `src/probe.rs`）。

    /// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
    fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
        let ((), lines) = capture_lines(LineFormat::LevelFields, f);
        lines
    }

    /// `KanadeMsg` から `ChoiceInput` を取り出す（他 variant は檻失敗）。
    fn expect_choice(msg: KanadeMsg) -> ChoiceInput {
        match msg {
            KanadeMsg::Choice(c) => c,
            _ => panic!("KanadeMsg::Choice を期待"),
        }
    }

    // -------------------------------------------------------------------------
    // to_choice_input 純関数檻（design Testing Strategy / Unit Tests 5・Req1.5/3.7）
    // -------------------------------------------------------------------------

    /// 不透明転写（Req1.5）: `id`／`label` は受領した内容をそのまま——前後空白のトリムも
    /// 正規化も行わずバイト等価で転写される。
    #[test]
    fn to_choice_input_transcribes_id_and_label_verbatim() {
        let sel = dirty_selection();
        let (id, label) = (sel.id.clone(), sel.label.clone());

        let input = to_choice_input(sel);

        assert_eq!(
            input.id, id,
            "id は改変せず転写（トリム・正規化なし・Req1.5）"
        );
        assert_eq!(
            input.label, label,
            "label は改変せず転写（さくらスクリプト風・記号もそのまま・Req1.5）"
        );
    }

    /// 不透明転写（Req1.5）: `references` は**記述順のまま全要素**が転写される——
    /// 空文字要素の除去・空白のみ要素の除去・順序変更・重複排除のいずれも起きない。
    #[test]
    fn to_choice_input_preserves_reference_order_and_empty_elements() {
        let sel = dirty_selection();
        let references = sel.references.clone();
        assert_eq!(
            references.len(),
            5,
            "fixture 前提（空文字・空白のみ要素を含む 5 件）"
        );

        let input = to_choice_input(sel);

        assert_eq!(
            input.references, references,
            "references は記述順・全要素をバイト等価で転写（空要素除去・順序変更なし・Req1.5）"
        );
        assert_eq!(
            input.references.len(),
            5,
            "空文字要素・空白のみ要素も落とさない（Req1.5）"
        );
        assert_eq!(input.references[1], "", "空文字要素はそのまま空文字で残る");
    }

    /// `scope` の型合わせ（design C1・Req3.7 は下流の用途限定）: `usize`→`u32` へ値を保って写る。
    /// 本層は搬送のみで、Reference へ載せる／載せないの判断は持たない（DD-13）。
    #[test]
    fn to_choice_input_maps_scope_usize_to_u32() {
        for scope in [0usize, 1, 3] {
            let sel = ChoiceSelection {
                id: "q0".to_string(),
                label: "はい".to_string(),
                scope,
                references: Vec::new(),
            };
            assert_eq!(
                to_choice_input(sel).scope,
                scope as u32,
                "scope は usize→u32 で値を保って写る"
            );
        }
    }

    /// 参照列が空でも空 `Vec` のまま転写される（下流の Reference 位置省略は kanade の領分・Req3.5）。
    #[test]
    fn to_choice_input_keeps_empty_reference_list_empty() {
        let sel = ChoiceSelection {
            id: "q1".to_string(),
            label: "いいえ".to_string(),
            scope: 0,
            references: Vec::new(),
        };
        assert!(
            to_choice_input(sel).references.is_empty(),
            "空の参照列は空のまま（空文字で埋めない）"
        );
    }

    // -------------------------------------------------------------------------
    // forward_all 檻（design Testing Strategy / Unit Tests 5・Req1.2/1.6）
    // -------------------------------------------------------------------------

    /// 全件・到着順・無判断（Req1.2）: 滞留した通知は 1 件も捨てず到着順に転送される。
    /// **同一 id の重複を含める**ことで、本層が重複排除・フィルタを行わない（一回性の調停は
    /// kanade 側が単一真実源）ことを弁別する。
    #[test]
    fn forward_all_sends_every_selection_in_arrival_order_without_dedup() {
        let (sel_tx, sel_rx) = mpsc::channel::<ChoiceSelection>();
        let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();

        let arrivals = ["first", "second", "first", "third"];
        for id in arrivals {
            sel_tx
                .send(ChoiceSelection {
                    id: id.to_string(),
                    label: format!("ラベル:{id}"),
                    scope: 0,
                    references: vec![id.to_string()],
                })
                .expect("受信口は生存している");
        }

        let forwarded = forward_all(&sel_rx, &kanade_tx);
        assert_eq!(
            forwarded, 4,
            "滞留 4 件は全件送出される（暗黙に捨てない・Req1.2）"
        );

        for expected in arrivals {
            let got = expect_choice(kanade_rx.try_recv().expect("到着順に届く"));
            assert_eq!(got.id, expected, "到着順そのまま（並べ替えなし・Req1.2）");
            assert_eq!(got.label, format!("ラベル:{expected}"));
            assert_eq!(got.references, vec![expected.to_string()]);
        }
        assert!(
            kanade_rx.try_recv().is_err(),
            "送出は受領件数ちょうど（水増しなし）"
        );
        assert_eq!(
            sel_rx.try_recv().unwrap_err(),
            TryRecvError::Empty,
            "受信口は drain 済み（滞留を残さない）"
        );
    }

    /// 空の受信口では何もしない（毎フレーム走る排他システムの no-op 経路）。
    #[test]
    fn forward_all_on_empty_inbox_forwards_nothing() {
        let (_sel_tx, sel_rx) = mpsc::channel::<ChoiceSelection>();
        let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();

        assert_eq!(forward_all(&sel_rx, &kanade_tx), 0, "空 inbox は 0 件送出");
        assert!(kanade_rx.try_recv().is_err(), "何も届かない");
    }

    /// 転送失敗分岐（Req1.6）: kanade 受信端が drop 済みでも **panic せず**、
    /// `warn!(event = "choice_forward_failed")` を記録して**次の件へ継続**する
    /// （滞留全件が drain され、失敗件数ぶん warn が出る）。
    #[test]
    fn forward_all_warns_and_continues_when_kanade_receiver_dropped() {
        let (sel_tx, sel_rx) = mpsc::channel::<ChoiceSelection>();
        let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
        drop(kanade_rx); // kanade 停止後（終了系では正常・Req1.6）

        for id in ["a", "b", "c"] {
            sel_tx
                .send(ChoiceSelection {
                    id: id.to_string(),
                    label: id.to_string(),
                    scope: 1,
                    references: Vec::new(),
                })
                .expect("受信口は生存している");
        }

        let mut forwarded = 0usize;
        let logs = capture_logs(|| {
            forwarded = forward_all(&sel_rx, &kanade_tx);
        });

        assert_eq!(forwarded, 0, "受信端 drop 後は 1 件も送出成功しない");
        assert_eq!(
            sel_rx.try_recv().unwrap_err(),
            TryRecvError::Empty,
            "最初の失敗で打ち切らず全件 drain して継続する（Req1.6）"
        );

        let failures: Vec<&String> = logs
            .iter()
            .filter(|l| l.contains("choice_forward_failed"))
            .collect();
        assert_eq!(
            failures.len(),
            3,
            "失敗 3 件それぞれが記録される（無記録で打ち切らない・Req1.6）: {logs:?}"
        );
        for line in failures {
            assert!(
                line.contains("level=WARN"),
                "choice_forward_failed は warn レベル（design ログ語彙表）: {line}"
            );
        }
    }
}
