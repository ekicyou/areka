# 設計の確かめ: alert-dialog-per-monitor-dpi（案 B）

> 2026-11-01・案 B（タスクダイアログ）で作り直した設計（コミット `12ffa196`）を、本ブランチのコードと照らして読んだ。読んだ物: `crates/areka/src/alert.rs`・`alert_dpi_tests.rs`・`alert_tests.rs`・`alert_memory_write_tests.rs`、`windows` 0.62.2 のソース（`Win32/UI/Controls/mod.rs`）、`tasks.md` の実装の記録（関門 0 と案 B の測りの 7 行）、`research.md`、steering。画面は出していない。案 A の設計への前の確かめは、この文書で置き換えた。

## まとめ

設計は小さく閉じていて、要件 1〜5 のすべてに部品か手順が当たっている。タスクダイアログへの写し方（ボタン・アイコン・閉じるボタンの旗・呼び戻し・HRESULT の失敗の道・答えの読み方）は Win32 の決まりと `windows` 0.62.2 の型に合っており、触る所は `alert.rs` と兄弟のテストだけで `Cargo.toml` は動かない。気になるのは、要件 2.1 の「前面と最前面」をどこでも確かめていないことと、本番の 3 つの口を 1 本のスレッドで続けて出す測りの作りが今の測りの道具のままでは回らないことの 2 点で、どれも実装とタスクの段で直せる。**GO**。

## 照らし合わせで確かめたこと

- **API の形**（`windows` 0.62.2 のソースで確かめた）: `TaskDialogIndirect(*const TASKDIALOGCONFIG, Option<*mut i32>, Option<*mut i32>, Option<*mut BOOL>) -> Result<()>`（中は HRESULT の `.ok()`）。`TASKDIALOGCONFIG` に `Default` があり、欄 `cbSize`・`hwndParent`・`dwFlags`・`dwCommonButtons`・`pszWindowTitle`・`Anonymous1`（`pszMainIcon`）・`pszMainInstruction`・`pszContent`・`nDefaultButton`・`pfCallback` が在る。`PFTASKDIALOGCALLBACK` は `Option<unsafe extern "system" fn(HWND, TASKDIALOG_NOTIFICATIONS, WPARAM, LPARAM, isize) -> HRESULT>` で、設計の `bring_to_front` の署名と同じ。`TDN_CREATED`＝0・`TDF_ALLOW_DIALOG_CANCELLATION`＝8・`TD_INFORMATION_ICON` は `PCWSTR`。`TASKDIALOG_COMMON_BUTTON_FLAGS` に `BitOr` があるので `TDCBF_YES_BUTTON | TDCBF_NO_BUTTON` も書ける。
- **旗と閉じるボタン**: タスクダイアログは取り消しのボタンが無いと、閉じるボタン・Esc・Alt+F4 を効かなくする。OK だけの 2 つに `TDF_ALLOW_DIALOG_CANCELLATION` を立て、はい／いいえには立てない設計は、今の `MB_OK`（閉じられる）・`MB_YESNO`（閉じられない）と同じ効きになる。閉じたときの `IDCANCEL` は、OK だけの口がボタンを見ないので答えに響かない。
- **既定のボタン**: `nDefaultButton`＝0 は「先頭のボタン」。共通のボタンは OK・はい・いいえ・取り消しの順に並ぶので、はい／いいえの既定は「はい」で、今の `MB_YESNO` と同じ。
- **答えの読み方と失敗の道**: `yes_no_answer` は `IDYES` だけを「はい」と読み、思わぬボタンと `Err` を `Unavailable` へ倒す。今の `ask_yes_no` の「0 なら失敗」と同じ側へ倒れ、同意を思わぬ値へ倒さない。失敗は戻り値の HRESULT に乗るので、スレッドを戻す呼び出しの後でも値は変わらず、`BoxOutcome` を消せる。`*_box_failed` の語・水準・欄 `error` は残り、変わるのは文の `MessageBoxW failed` の語と欄の値の出どころ（`GetLastError` → HRESULT）だけ。文を名前で拾うテストや道具はリポジトリに無い（`crates`・`tools`・`doc` を探した）。
- **版 6 の申告**: `TaskDialogIndirect` は `windows_core::link!` で comctl32 を読み込みの時に結ぶ。`alert.rs` が入るのは bin と bin のユニットテストの実行ファイルだけで、どちらも `build.rs` が申告を埋める。案 B の測りがテストの実行ファイルで S_OK（`box=0`）を返したのが証。申告が外れたときの危うさは Revalidation Triggers に置いてある。
- **囲みの部品**: 既にある `with_thread_per_monitor` は、関門 0 と案 B の測りで本物の道として通っている（`taskdialog_wrapped_from_unaware` が `thread_during=2 window=2 window_dpi=192`）。`is_invalid()` が -1（＝`DPI_AWARENESS_CONTEXT_UNAWARE`）と null を無効と読む点を、D1〜D3 の偽の値の選び方と D4 の本物の切り替えで押さえているのは正しい。
- **境界**: 呼び手・`Cargo.toml`・マニフェスト・`build.rs`・`emo2_boot/frame/dpi.rs` に触らない。`show_box`・`with_thread_per_monitor` を呼ぶのは `alert.rs` と `alert_dpi_tests.rs` だけ（確かめた）。既にある `alert_tests.rs`・`alert_memory_write_tests.rs` は抑止ありで呼び `show_box` を使わないので、書き換えなしで 2.2 の檻のまま残る。
- **窓のアプリの前の道**（1.3）: プロセスが非対応のままでも、スレッドだけを替えて窓が v2 になることは案 B の測りが areka のコードの中から確かめている。前と後で道を分けないので要件 4.4 が当たらないという読みも筋が通る。

## 大事な点（3 つまで）

### 1. 「前面と最前面」をどこでも確かめていない

- **気になること**: 要件 2.1 は、はい／いいえの画面と警告の画面を今と同じく前面・最前面に出すことを求める。タスクダイアログには `MB_TOPMOST`・`MB_SETFOREGROUND` に当たる旗が無く、設計は `TDN_CREATED` の呼び戻しで `SetWindowPos(HWND_TOPMOST)` と `SetForegroundWindow` を呼ぶ新しい道を作る。ところが決定論のテストは配線として外し（議題 2）、実機の測り `probe_alert_dialogs_production` が見るのは窓の扱いと DPI だけ、撮った画面（4.7）で見るのは題名・本文・ボタン・アイコン・閉じるボタンだけで、最前面になったかはどこにも出てこない。`TDN_CREATED` は窓を見せる前に届く知らせなので、そこでの前面の指定が効くかも、今の設計では見ないまま終わる。
- **響き**: 呼び戻しを付け忘れる・種類の表の「最前面と前面」の列を取り違える・`TDN_CREATED` の時点の指定が効かない、のどれが起きても、利用条件の問いがほかの窓の後ろに隠れ、起動の前のインストールが答えを待って止まったように見える。撮った画面では、最前面かどうかは見分けられない。
- **直し方**: 実機の測り `probe_alert_dialogs_production` が窓を見つけたときに、拡張の様式（`GetWindowLongPtrW(GWL_EXSTYLE)`）の `WS_EX_TOPMOST` と、前面の窓（`GetForegroundWindow() == hwnd`）も読んで 1 行に載せ、`ask`・`install_warning_dialog` は最前面、`alert` は最前面でないことを assert する。前面は OS の決まりで揺れうるので、前面は行に載せて読むだけにし、最前面だけを判定にする。タスクの記録には ⑹ の結果として残す。
- **要件**: 2.1
- **出どころ**: design.md「Dialog」の表・「bring_to_front」・「alert_dpi_tests.rs」の実機の測りの表・「実機の確かめ」の ⑹

### 2. 本番の 3 つの口を 1 本のスレッドで続けて出す測りが、今の測りの道具のままでは回らない

- **気になること**: `probe_alert_dialogs_production` は、1 本のスレッドで 3 つの口を順に呼び、画面をおよそ 8 秒残してから閉じる設計になっている。今の測りの道具（`alert_dpi_tests.rs` の `on_tick`）は「場合ごとに新しいスレッドなので戻さない」前提で作られていて、(a) スレッドの局所の置き場 `TICKS`・`FOUND`・`CLOSE` を画面ごとに戻さない、(b) 窓を見つけたらすぐ閉じてタイマーを止める（次の画面のためのタイマーが無い）、(c) 見つからないときの上限が 20 回×300 ミリ秒＝およそ 6 秒で、8 秒残す待ちと別の数えになっていない、(d) 見つからないときに `PostQuitMessage` を投げるので、残った `WM_QUIT` が同じスレッドの次の画面のモーダルを即座に抜けさせる。設計は「題名を場合ごとに替えられるようにする」とだけ書き、これらに触れていない。
- **響き**: 2 つ目と 3 つ目の画面が読めずに「見つからない」になる、すぐ閉じて撮れない、または 1 つ目で見つからなかったときに残りが連鎖して壊れる。要件 1.4（3 つの口が同じ道）と 4.7（3 枚を撮る）の確かめが、測りの道具の不具合で赤や撮り損ないになる。
- **直し方**: 測りの道具を、画面ごとに「置き場を戻す → タイマーを立てる → 口を呼ぶ → タイマーを止める」の形にし、見つけた後の「残す回数」と見つからないときの上限を別の数えにする。見つからないときは `WM_QUIT` を投げずに測りを赤で終える（または画面ごとに新しいスレッドを立て、そのスレッドを非対応にしてから口を 1 つ呼ぶ。こちらなら今の道具をそのまま使える）。どちらを取るかはタスクの段で決め、タスクの完了の条件に書く。
- **要件**: 1.4, 4.7
- **出どころ**: design.md「alert_dpi_tests.rs（兄弟のテスト）」の実機の測りの表と、その下の注記・`crates/areka/src/alert_dpi_tests.rs` の `TICKS`・`on_tick`・`probe_case`

## 小さな点（大事な点に数えない）

- `alert_tests.rs` の先頭の注釈「抑止なしの呼び出し（`MessageBoxW`）は…」が古くなる。設計は `alert_tests.rs` を「今のまま」とするが、触ってよい兄弟のテストなので、注釈の 1 語を「タスクダイアログ（`TaskDialogIndirect`）」へ直すのを 2.2 のタスクに入れてよい（テストの中身は替えない）。
- `TaskDialogIndirect(&config, Some(&mut button), None, None)` の `Some(&mut button)` は `Option<*mut i32>` を期待する所への受け渡しで、写しの強制が効く。書けないときは `Some(&mut button as *mut i32)` と書く。

## よい所

- **測ってから作り直した**: 関門 0 で案 A が効かないことを本物の道（areka のコードの中から、囲む／囲まない・プロセスの既定の 4 通り）で確かめ、案 B も同じ測りの道具と既にある囲みの部品で主のモニタの値（窓の DPI＝モニタの DPI＝192・窓の扱い＝2）を取ってから設計に入っている。前の部品（`with_thread_per_monitor`・`set_thread_dpi_context`）をそのまま使い、中身だけを替えるので、変える量が小さい。
- **失敗の道が型で決まる**: 失敗を HRESULT の戻り値で受けることで「最後のエラーを戻す前に読む」順の決まりと `BoxOutcome` が消え、「戻してから記録する」（3.2）が関数の形から決まる。はい／いいえを `IDYES` だけで「はい」と読む純粋な関数を D5 で固定し、同意を思わぬ値へ倒さない。

## 判定

**GO**。Win32 の写し方・要件の当て方・境界はどれも成り立っていて、残る 2 点は実機の測りの作りの話なので、タスクの段で次を書き込めば足りる。

- 大事な点 1: `probe_alert_dialogs_production` の 1 行に最前面と前面を足し、最前面を判定にする（2.1）。
- 大事な点 2: 画面ごとに測りの置き場とタイマーを戻す形（または画面ごとに新しいスレッド）と、残す数えと上限の分け方を、タスクの完了の条件に書く（1.4・4.7）。

次は `/kiro-spec-tasks alert-dialog-per-monitor-dpi`（既にある `tasks.md` の 2 と 3 を案 B に合わせて作り直す）。
