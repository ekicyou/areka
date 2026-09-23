# 設計レビュー: areka-P0-wintf-drag-state-rest-contract

> 実施日: 2026-09-23（本ブランチ・main `92f5f448` 相当）。非対話で実施し、設計の主張はすべて `Read`／`Grep` で実物のソースと突き合わせた。行番号ではなく「何の定義か」で指す。

## レビュー要約

設計は案 A（説明を実装に合わせる）を一貫して貫き、挙動の変更を相乗り 2 件の判断分岐（預かりの窓の比較・説明書の実在チェック）だけに閉じている。設計が引用するコードの事実（`reset_to_idle` の製品の呼び手 0・`JustStarted` の出口は `dispatch_drag_events`・`WM_ACTIVATE` の非対称・`decide` の見本が scope 1／0・`is_available` が `missing_logged` を消費する・`JustEnded` が `CaptureGuard` を持たない）は **すべて実物と一致**し、食い違いは 0 件だった。実装へ進める水準にある。

## 実物との突き合わせ（主要な主張）

| 設計の主張 | 実物 | 結果 |
|---|---|---|
| `reset_to_idle` の製品の呼び手 0（定義・再輸出・テスト 3 か所のみ） | `state/mod.rs` の定義・`drag/mod.rs` の `pub use`・`state/tests.rs` 2 本・`controller_tests.rs` の後片付け 1 か所。`trigger.rs` の doc コメントに語のみ | 一致 |
| `JustStarted → Dragging` は次の tick の `dispatch_drag_events`（`Started` の腕が `update_dragging`） | `dispatch.rs` の `DragTransition::Started` の腕の末尾で `super::state::update_dragging(start_pos, ctx_res)` | 一致 |
| `mouse_move.rs` に `JustStarted` の腕は無い（`Preparing`・`Dragging` のみ） | `WM_MOUSEMOVE` の `match` は `Preparing`（閾値判定→`start_dragging`）と `Dragging` の 2 腕 | 一致 |
| 「押している間か」だけを聞く読み手 3 か所 | `start_preparing`・`cached_nchittest`（`read_drag_state` で `is_dragging`）・`handle_release`（`Idle \| JustEnded` でなければ無視） | 一致 |
| `resolve_transition` は `Dragging \| JustStarted` を抑止、`Idle \| Preparing \| JustEnded` を同じ腕 | `controller.rs` の `match drag` のとおり | 一致 |
| `WM_CAPTURECHANGED` は可変で `mark_released` を呼ぶ／`WM_ACTIVATE` は `JustStarted` で `cancel_dragging` を呼ばない | `keyboard.rs` のとおり（`WM_ACTIVATE` は `Dragging`・`Preparing` の 2 腕＋`_ => {}`） | 一致 |
| `decide` の見本は預かり scope 1・要求 scope 0 | `trigger_tests.rs` の `pending_double_click()`＝scope 1・`request()`＝scope 0 | 一致 |
| `is_available` は `missing_logged.replace(true)` を消費する | `readme.rs` の `is_available` のとおり | 一致 |
| `JustEnded` は `CaptureGuard` を持たない | `DragState::JustEnded { entity, position, cancelled }` | 一致 |
| `start_preparing` を呼ぶのは左押下のみ（述語 doc の「左ボタン」の根拠） | `mouse_click.rs` の `handle_button_message` は `button == Left && drag_config.left_button` のときだけ呼ぶ | 一致 |
| 「直前 1 フレーム」は `controller.rs` の判定順序 1 と `docs/click_through.md` の「ドラッグ中の透過抑止」の 2 か所 | 両方に「直前 1 フレームの `JustStarted`」の語がある。`runtime/mod.rs` の「1 フレーム遅延」は別文脈で対象外 | 一致 |
| テストの道具（`log_capture_kit`・`temp_path_kit`・`wired`・`capture`・`defer_double_click`・`take_query`・`menu_lines`）は既存 | `areka/Cargo.toml` の `[dev-dependencies]` と各テストファイルに実在 | 一致 |
| 触るファイルは既存 11（製品 6・テスト 4・文書 1）・新規 0 | 列挙どおり | 一致 |

## 指摘事項の確認（依頼で指定された観点）

- **「1 フレーム遅らせる解」を避けているか**: 避けている。案 B（フレームの終わりで `Idle` へ戻す）を却下し、製品の遷移を 1 つも足さない。状態図は今日の実装（`JustEnded` で休む）そのものである。
- **`trigger_tests.rs` の `decide` テストが不変か（要件 6.5）**: 不変。C1（`decide` へ渡す前に `poll_once` で窓を絞る）を採るので `decide` の引数・本文・既存 2 本は触らない。
- **`is_button_held` の真偽表が 5 つの状態を覆うか**: 覆う。`Preparing`・`JustStarted`・`Dragging`＝真／`Idle`・`JustEnded`＝偽。真偽表テストは 5 状態を製品の関数で順に踏み、両型（`DragState`・`DragStateSnapshot`）で確かめる。
- **説明書の設計が `is_available` を避けているか**: 避けている。`open_from_world` に `wiring.path.exists()` を直に書き、`missing_logged` を消費しない（要件 5.4）。
- **`controller_tests.rs` の後片付けの置き換えは安全か**: 安全。そのテストは冒頭で状態を `JustEnded` に直接置き、`evaluate_targets` は読むだけなので、後片付けの時点で `CaptureGuard` は存在しない。借用の中で `*s = DragState::Idle` を代入しても `ReleaseCapture` は走らず、`RefCell` の再借用も起きない。

## 重大な指摘

**0 件。** 設計の主張と実物の食い違いは無く、既存の構造（`matches!` 1 行の述語・兄弟テスト・`log_capture_kit`・thread_local の後片付け）から外れる新設も無い。

## 設計の強み

1. **契約を「実装が今そうである形」に固定している。** 契約テストは手で組んだ状態から始めず、製品と同じ `start_preparing` → `end_dragging`／`cancel_dragging` を踏む。2026-09-19 の実害（テストが欠陥を合格条件に固定していた）の再発を、テストの作り方そのもので防いでいる。
2. **既存テストとの衝突を先に潰している。** `decide` の見本が scope 1／0 で組まれている事実を実測してから C1 を選び、`is_available` が初回記録を消費する事実から `exists()` の直書きを選んでいる。どちらも「実装してから既存テストが赤くなって直す」手戻りを設計段階で消している。

## 軽微な観察（ディスカッションの題材・妨げにはならない）

1. **表示に決着したときの窓違いの預かりの記録の行。** 絞り込みを返事の種類より前に置くため、「別の窓の預かり＋表示」では `menu_deferred_double_click_dropped` ではなく `menu_deferred_double_click_scope_mismatch` の行が出る。要件 4.3 は「送らない」だけを求めるので違反ではなく、設計は System Flows に明記している。記録を読む側が「表示で捨てた」と「窓が違うので捨てた」を区別できるという利点もあるので、このままで良いと考えるが、確認事項として一度は開発者に見せる価値がある。
2. **後片付けの代入の頑健さ。** `controller_tests.rs` の `update_drag_state(|s| *s = DragState::Idle)` は、状態が `JustEnded` である前提（同じテストの冒頭で保証）に依る。将来そのテストの途中に押下を挟む改変が入ると借用中の `CaptureGuard` の解放で落ちるが、それは同じテストの冒頭の `JustEnded` 直接代入にも同じことが言える既存の前提であり、本仕様で新たに増える危険ではない。`state/tests.rs` の `force_idle` と同じ「取り出して借用の外で落とす」形にすればその前提も消えるが、areka 側の `ResetDragState` と作法を揃える要件 3.5 の趣旨からは現状の案で足りる。
3. **「OS を呼んでいない」の証明は間接。** 相乗り 2 のテストは `readme_open_failed` 0 行＋`readme_opened` 0 行で `open` に到達しなかったことを示す。`open` はどちらかの行を必ず出す（実装を確認済み）ので推論は健全であり、既存テスト `open_records_a_missing_file_as_an_error_and_returns_err` が捕捉の道具が `readme_open_failed` を拾えることを較正している。加えて `readme_open_skipped_missing` 1 行の正の確認で新しい分岐を踏んだことも示すので、沈黙による恒真にはならない。

## 最終判定

**GO。**

根拠: 設計が引用するコードの事実は 12 項目すべて実物と一致し、要件 1〜6 の各項が Components と Testing Strategy へ 1 対 1 で対応している。挙動の変更は相乗り 2 件の判断分岐に限られ、どちらも決定論テストで到達できる。開発規律（1 フレーム遅らせる解の禁止・既存テスト不変・記録の無い失敗経路の禁止・1 ファイル 1,000 行）にも反しない。

次の手順: `/kiro-spec-tasks areka-P0-wintf-drag-state-rest-contract` でタスクを生成する。タスク分割は research.md §8 の 5〜7 本の見立て（①doc＋述語＋撤去 ②読み手 3 か所 ③契約テスト＋既存テスト追随 ④相乗り 1＋テスト ⑤相乗り 2＋テスト ⑥透過制御の語彙 2 か所）で足りる。①と③は同じコミットにまとめる（`reset_to_idle` の撤去とテストの追随を分けると中間状態で compile が落ちる）。
