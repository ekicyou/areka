# Implementation Plan

> 触るのは `crates/areka/src/alert.rs` と兄弟のテストのファイル `crates/areka/src/alert_dpi_tests.rs` だけ（ウェーブ C15 の約束）。`emo2_boot/frame/dpi.rs`・呼び手・`Cargo.toml`・マニフェスト・`build.rs` には触らない。既にある `alert_tests.rs`・`alert_memory_write_tests.rs` のテストは書き換えずに緑のまま保つ（要件 2.2 の檻・`alert_tests.rs` の先頭の注釈の古い語だけは 2.2 で直す）。
> 2026-11-01 の開発者の裁定「案 B で作り直す」で、2 と 3 をタスクダイアログの形に作り直した。1 は案 A の関門 0 の記録で、そこで足した部品（囲みの部品・本物の切り替えの関数）は 2 でそのまま使う。
> 実機の確かめ（3.1）で副のモニタの画面の窓が v2 でない、または窓の DPI がモニタの DPI と違えば、そこで止め、測った値を添えて開発者へ戻す（案 C へ進むかは開発者が決める）。

- [x] 1. 関門 0: 囲みの部品を足し、案 A が効くかを実機で測る
- [x] 1.1 画面を出す間だけスレッドの DPI の扱いを v2 に囲んで戻す部品と、画面を出す 1 つの道を足す
  - 切り替えの関数を引数で受ける囲みの部品を足す。前の扱いが有効なら処理を 1 回走らせた後に前の扱いへ戻し、無効なら warn `alert_dpi_context_unchanged`（欄 `dialog`）を 1 件残して処理を 1 回だけ走らせ、戻さない
  - 本物の切り替え（`SetThreadDpiAwarenessContext` を呼ぶ）を `alert.rs` の中の名前の付いた非公開の関数にし、画面を出す道とテストの D4（2.3）の両方から名前で呼べるようにする
  - 本物の切り替えとメッセージボックス（持ち主の窓なし・旗はそのまま）を組む道を足し、戻り値と、戻った直後・スレッドを戻す前に読んだ最後のエラーを一緒に返す
  - `alert.rs` の `unsafe` は本物の切り替えの関数とメッセージボックスの呼び出しの 2 か所だけにし、それぞれに安全の理由の注釈を書く
  - 3 つの口へはまだ繋がない。本番の組み立てで未使用の警告が出ないよう、足した物すべて（囲みの部品・画面を出す道・戻り値の型・本物の切り替えの関数）に未使用の許しを付ける（2.2 で全部外す）
  - 完了: `cargo build -p areka` が警告なしで通り、`cargo test -p areka --bin areka alert` の既にあるテストが緑のまま（本番の振る舞いは変わっていない）
  - _Requirements: 1.5, 2.3, 3.1, 3.2, 3.3_
- [x] 1.2 関門 0 の測りの 2 本を兄弟のテストのファイルに足す
  - 新しい兄弟のテストのファイルを作り、`alert.rs` の末尾にテストの時だけ読み込む宣言を足す
  - 1 本目は新しいスレッドごとに 3 つの場合（非対応で直に出す・非対応から囲んで出す・スレッドを明示で v2 にして直に出す）を順に測る。2 本目はプロセスの最初にプロセスを v2 に立て、スレッドは替えずに直に出す 1 つの場合だけを測る
  - 画面が出ている間に同じスレッドのタイマーの呼び戻しで、題名 `areka-alert-dpi-probe` の `#32770` の窓を探し、スレッドの扱い（出す前・出ている間）・窓の扱いと v2 か・窓の DPI・モニタの DPI・system DPI・プロセスの扱いを読み、OK を送って閉じる。20 回探して見つからなければモーダルを抜けて「見つからない」として残す
  - 測りが直に出すメッセージボックスと、タイマー・窓の読み取り・スレッドの局所の置き場の `unsafe` は兄弟のテストのファイルの中に置き、`alert.rs` には置かない（2.2 の「`MessageBoxW` は 1 か所」と先頭の注釈を崩さない）
  - 場合ごとに ASCII の `ALERT-DPI-PROBE case=... process=... thread_before=... thread_during=... window=... window_v2=... window_dpi=... monitor_dpi=... system_dpi=... box=...` を 1 行出し、1 本目は 3 行を出してから「囲んだ場合の窓が v2」を、2 本目は窓が見つかったことを assert する
  - 完了: 2 本とも `#[ignore]` 付きでコンパイルが通り、既定の `cargo test -p areka --bin areka` では走らない（画面が出ない）
  - _Requirements: 4.6_
- [x] 1.3 関門 0 を実機で 1 回走らせ、先へ進むかを決める（関門は通らず・案 B へ）
  - 排他テストの机を取ってから、画面がロックされていないときに 2 本を別々の `cargo test ... -- --ignored --nocapture --test-threads=1 --exact` で 1 回ずつ走らせ、終わったらすぐ机を返す
  - どれかの場合で窓が見つからなければ測りを直して走らせ直す（関門は通らない）
  - 4 行と、プロセスを v2 に立てた後の場合の見分け（窓が 2 なら当時の測り方の取り違え・1 ならプロセスの既定を受けたスレッドの画面が system で作られる形・プロセスが v2 にならなければ見分けに使わない）をタスクの記録に書く
  - 完了: 「非対応から囲んで出す」の行が `thread_during=2` かつ `window_v2=true` なら関門を通り 2 へ進む。そうでなければ 2 本の 4 行を添えて開発者へ戻し、ここで止まる（足した部品と測りは消さない・口へは繋いでいないので本番は変わらない）
  - 結果（2026-11-01 実機）: 「非対応から囲んで出す」は `thread_during=1`・`window_v2=false` で関門を通らず、4 行を添えて開発者へ戻した。同じ日に開発者の指示で案 B の測り（タスクダイアログ・`probe_alert_task_dialog_window_dpi`）を足して走らせ、囲めば非対応のスレッドからでも窓が v2（窓の DPI＝モニタの DPI＝192）と分かった。開発者の裁定「案 B で作り直す」（2026-11-01）で要件と設計を作り直した。値と見立ては下の「Implementation Notes」
  - _Requirements: 1.3, 4.6_

- [x] 2. 案 B の直しの本体: タスクダイアログへの置き換えと決定論のテスト
- [x] 2.1 メッセージボックスの測り 2 本を消し、残す測りの窓の探し方と閉じ方をタスクダイアログに揃える
  - 関門 0 のメッセージボックスの測り 2 本（スレッドを囲む・プロセスを v2 に立てる）と、それだけが使う直に出す関数・本番の道で出す関数・何も替えない関数を消す（関門 0 の値は下の記録と `research.md` に残る）
  - 案 B の測り（タスクダイアログを直に出す 3 つの場合）は残し、行の形と値の読み方を変えない
  - 窓を探す鍵の題名を、場合ごとにスレッドの局所の置き場へ置いて替えられるようにする（2.4 で本番の口の題名を探すため）。今の測りは今の題名を置いて今どおり見つける
  - 閉じ方をタスクダイアログのボタンを押す合図だけに揃え、メッセージボックスの閉じ方（`WM_CLOSE`）の分かれを消す。ファイルの先頭の注釈からメッセージボックスと関門 0 の 2 本の記述を外す
  - 完了: このファイルからメッセージボックスの呼び出しと、本番の画面を出す道への参照が無くなり（grep で `MessageBoxW`・`show_box`・`WM_CLOSE` が 0 件）、`cargo test -p areka --bin areka alert` が緑（残る測りは `#[ignore]` で走らない）で、`cargo fmt --all -- --check` が通る。`cargo clippy -p areka --tests` の警告は、テストの組み立てで唯一の使い手を失う `alert.rs` の古い画面を出す道・戻り値の型・旗の型の取り込みの「未使用」だけ（範囲の外なので 2.2 で消える）
  - 2.2 より先に済ませる: 2.2 は画面を出す道の署名を替えるので、古い道を呼ぶ測りが残っているとテストの組み立てが通らない
  - _Boundary: alert_dpi_tests.rs_
  - _Requirements: 4.6, 4.7_
- [x] 2.2 画面の種類を足し、画面を出す道をタスクダイアログに替えて 3 つの口を繋ぐ
  - 画面の種類（エラー・はい／いいえ・OK だけの警告）を足し、種類から記録の欄の名（"alert"・"ask"・"install_warning_dialog"）・ボタン・アイコン（エラー・情報・警告）・閉じるボタンの効き（OK だけの画面は効く・はい／いいえの画面は効かない）・前面と最前面の要否が決まる形にする（設計の表のとおり）
  - 画面を出す道を、囲みの部品と本物の切り替えの中でタスクダイアログを出す形へ替える。持ち主の窓・太字の見出し・位置の旗・リンクの旗は渡さず、題名は窓の題名・本文は本文の欄に置く。押されたボタンか、出せなかった HRESULT をそのまま返し、戻り値と最後のエラーを一緒に運ぶ型は消す
  - 画面ができた知らせで窓を最前面にして前面へ出す呼び戻しを、はい／いいえの画面と警告の画面にだけ付ける。最前面にできなければ warn `alert_topmost_failed`（欄 `error`）を 1 件残して画面はそのまま出す。前面にできなかったことは記録しない。呼び戻しの中で `panic` しない
  - はい／いいえの結果を答えへ写す純粋な関数を足す（はいのボタンだけを「はい」、いいえのボタンを「いいえ」、それ以外のボタンと失敗を「出せなかった」と読む）
  - 3 つの口が直に出していたメッセージボックスを画面を出す道の呼び出しへ替える。署名・抑止の判断・出す前の記録・答えの型は動かさず、失敗の記録（`*_box_failed`）は名前・水準・欄をそのままに、欄 `error` を戻った HRESULT（はい／いいえの思わぬボタンは `unexpected button <数>`）にし、文の「MessageBoxW failed」を「TaskDialogIndirect failed」に替える
  - 1.1 で付けた未使用の許しを全部外し、ファイルの先頭の注釈を「メッセージボックス（`MessageBoxW`）」から「タスクダイアログ（`TaskDialogIndirect`）」へ、`unsafe` の置き場を「本物の切り替えの関数・画面を出す道の中のタスクダイアログの呼び出し・最前面の呼び戻しの 3 か所」へ直し、3 か所それぞれに安全の理由の注釈を書く
  - 兄弟の `alert_tests.rs` の先頭の注釈（8 行目）の「抑止なしの呼び出し（`MessageBoxW`）」を「抑止なしの呼び出し（タスクダイアログ）」へ直す。注釈だけで、テストの中身（檻）は替えない
  - 消す物: 戻り値と最後のエラーを運ぶ型（`BoxOutcome`）・`MessageBoxW` と `MB_*` の旗と旗の型（`MESSAGEBOX_STYLE`）の取り込み・3 つの口の最後のエラーを読む道（`Error::from_thread()`）。押されたボタンの型と値（`MESSAGEBOX_RESULT`・`IDOK`・`IDYES`・`IDNO`）は残る
  - 完了: grep で `alert.rs` に `MessageBoxW`・`MB_`・`BoxOutcome`・`from_thread` が 0 件（古い旗と比べる注釈も `alert.rs` に残さない）、`alert_tests.rs` に `MessageBoxW` が 0 件、`cargo build -p areka` が警告なしで通り、`cargo test -p areka --bin areka` が緑（既にある `alert_tests.rs`・`alert_memory_write_tests.rs` が書き換えなしで緑）で、`cargo clippy -p areka` と `cargo fmt --all -- --check` が通る
  - _Boundary: alert.rs（と alert_tests.rs の先頭の注釈だけ）_
  - _Depends: 2.1_
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 2.1, 2.2, 2.3, 3.2, 5.1_
- [x] 2.3 囲みの部品と答えの読み方の決定論のテスト D1〜D5 を足す
  - D1: 偽の切り替えが有効な前の扱い（system aware に限る）を返すとき、切り替えが「v2」「前の扱い」の順にちょうど 2 回、処理が 1 回、処理の戻り値がそのまま返り、記録は 0 件
  - D2: 偽の切り替えが無効な値を返すとき、切り替えは 1 回（v2）だけで戻さず、処理は 1 回走り、warn `alert_dpi_context_unchanged` が欄 `dialog` 付きでちょうど 1 件
  - D3: 処理が失敗（`Err`）を返しても、切り替えが「v2」「前の扱い」の順に 2 回で、`Err` がそのまま返る
  - D4: 新しいスレッドで扱いを非対応にしてから、本物の切り替えの関数を名前で渡して囲み、処理の中のスレッドの扱いが v2・戻った後は非対応。v2 から始めた場合も同じ形で 1 本
  - D5: はい／いいえの答えの読み方に、はい・いいえ・閉じた（`IDCANCEL`）・失敗（`E_FAIL`）を渡し、はい・いいえ・出せなかった・出せなかった
  - どのテストも本物の画面を出さず（タスクダイアログを呼ばない）、記録は `log-capture-kit` で見る
  - 完了: `cargo test -p areka --bin areka alert_dpi_tests` で D1〜D5 の 6 本が緑、`#[ignore]` の測りは走らず、`cargo clippy -p areka --tests` と `cargo fmt --all -- --check` が通る
  - _Boundary: alert_dpi_tests.rs_
  - _Depends: 2.1, 2.2_
  - _Requirements: 1.3, 1.5, 2.1, 3.1, 3.2, 4.1_
- [x] 2.4 本番の 3 つの口を 1 つずつ出す実機の測り `probe_alert_dialogs_production` を足す
  - 画面ごとに新しいスレッドを立て、扱いを非対応にしてから、本番の口を抑止なしで 1 つ呼ぶ（エラーの画面は起動窓の場面、はい／いいえの画面と警告の画面は測りの題名と本文）。窓を探す鍵には、その画面の題名を置く（エラーの画面は文面の関数が決める題名）
  - 同じスレッドのタイマーの呼び戻しで窓を探し、今の測りの読みに加えて最前面か（拡張の様式）と前面か（前面の窓がその窓か）を読み、およそ 8 秒（見つからないときの上限とは別に数える）残してからボタンを押す合図で閉じる（エラーと警告は OK、はい／いいえははい）
  - 画面ごとに ASCII の `ALERT-DPI-PROBE case=<alert|ask|install_warning_dialog> ...` を 1 行出し、3 つの窓が v2 で、はい／いいえと警告の窓が最前面・エラーの窓が最前面でなく、はい／いいえの答えが「はい」であることを assert する。前面は行に載せて読むだけで assert しない
  - 走らせる前の注意（抑止の環境変数 `AREKA_APP_SMOKE_EXIT_MS`・`AREKA_NO_ALERT` が立っていると画面が出ない）を測りの注釈に書く
  - 完了: `#[ignore]` 付きでコンパイルが通り、既定の `cargo test -p areka --bin areka` では走らず（画面が出ない）D1〜D5 が緑のまま、`cargo clippy -p areka --tests` と `cargo fmt --all -- --check` が通る
  - 2.1 の窓を探す鍵の題名と閉じ方を使う。2.3 と同じファイルなので並べて進めない（2.3 の後）
  - _Boundary: alert_dpi_tests.rs_
  - _Depends: 2.1, 2.2, 2.3_
  - _Requirements: 1.4, 2.1, 4.7_

- [ ] 3. 実装の後の実機の確かめ
- [ ] 3.1 2 枚のモニタの実機で利用条件の画面を副のモニタに出し、DPI・にじみ・閉じた後の窓を見る
  - 排他テストの机を取ってから始め、実の GPU と主 200%・副 150% のモニタで、画面がロックされていないときに、根と `terms.txt` 付きの書庫をワークツリーの `target\` の下に作り、`--install` で起動の前のインストールの利用条件の画面を出す。SSP は動かさない
  - 副に出す手順（走行の前に決める）: SendInput で副のモニタの上の窓を前面にしカーソルも副に置いてから areka を起こす。主に出たらその走行をやめ、開発者に走行の間だけ副を主に替えてもらってから出し直す。出た後に画面を手で動かさない。どの手順で副に出たかと、出たモニタを書く（2026-10-31 の走行とは比べない）
  - 画面が出ている間に、作業用のフォルダに置いたその場限りのスクリプト（リポジトリに入れない）で、areka のプロセスの番号で絞った `#32770` の窓の DPI・そのモニタの実効の DPI・窓の扱いを読み、画面の文字を拡大して撮ってにじみを見る。窓が v2 でないか窓の DPI がモニタの DPI と違えば、ここで机を返して止め、値を添えて開発者へ戻す
  - SendInput で「はい」を押し、起きたゴーストの窓を拡大率の違うモニタへ動かし、`WM_DPICHANGED` が届いてゴーストとバルーンの窓が今どおり追う（0.75 倍）ことを記録で見る。終わったらすぐ机を返す
  - 要件 4.2 の 2 回目は関門 0 の `process_v2_default` が `window=1` だったので走らせないこと（窓のアプリの前の道は案 B の測りの `taskdialog_wrapped_from_unaware` が確かめた）と、窓のアプリの前と後で扱いを分けない設計なので要件 4.4 は当たらないことを、理由とともに書く
  - 完了: タスクの記録に、副に出した手順と出たモニタ・窓の DPI＝144＝モニタの DPI・窓の扱い＝2・撮った画面の見立て・閉じた後の追従の記録の抜き書き・2 回目と 4.4 を走らせない理由が書かれている
  - _Depends: 2.2, 2.3_
  - _Requirements: 1.1, 1.2, 1.3, 2.3, 3.3, 4.2, 4.3, 4.4, 4.5_
- [x] 3.2 本番の 3 つの口の画面を 1 つずつ出して撮り、替わった見た目を開発者に見せる
  - 排他テストの机を取ってから、画面がロックされていないときに、抑止の環境変数が立っていないことを確かめて `cargo test -p areka --bin areka alert_dpi_tests::probe_alert_dialogs_production -- --ignored --nocapture --test-threads=1 --exact` を 1 回走らせる（主のモニタでよい）。終わったらすぐ机を返す
  - 3 つの画面がそれぞれ残っている数秒の間に撮り、題名・本文・ボタン（OK／はい・いいえ）・アイコン（エラー・情報・警告）と、はい／いいえの画面の閉じるボタンが効かない（灰色）ことを見る
  - 3 行の `ALERT-DPI-PROBE` で、3 つの窓が v2 で窓の DPI＝モニタの DPI であること、最前面と前面の読みを見る
  - 完了: 測りが緑で、タスクの記録に 3 行と撮った 3 枚の見立てが書かれ、3 枚を開発者に見せている
  - _Depends: 2.4_
  - _Requirements: 1.4, 2.1, 4.7_

## 先送りする要件

- 要件 5.2（完了のときに件 2 を持ち主の決まった起票として渡す）はコードの変更を持たず、`/kiro-complete` の未解決問題の棚卸（`/kiro-discovery` での起票）の持ち物。実装のタスクには置かない。

## Implementation Notes

- 関門 0（1.3・2026-11-01・主 200%＝system DPI 192）の 4 行:
  - `case=unwrapped_unaware process=0 thread_before=0 thread_during=0 window=0 window_v2=false window_dpi=96 monitor_dpi=96 system_dpi=96 box=1`
  - `case=wrapped_from_unaware process=0 thread_before=0 thread_during=1 window=1 window_v2=false window_dpi=192 monitor_dpi=192 system_dpi=192 box=1`
  - `case=unwrapped_thread_v2 process=2 thread_before=2 thread_during=1 window=1 window_v2=false window_dpi=192 monitor_dpi=192 system_dpi=192 box=1`
  - `case=process_v2_default process=2 thread_before=2 thread_during=1 window=1 window_v2=false window_dpi=192 monitor_dpi=192 system_dpi=192 box=1`
- 見立て: スレッドが v2（囲んだ・明示・プロセスの既定のどれでも）で `MessageBoxW` を呼ぶと、画面が出ている間のスレッドの扱いが 1（system）に替わり、窓も system で作られる。`MessageBoxW` 自身が v2 を system へ落とす形で、案 A（スレッドを囲む）では直らない。非対応のスレッドからは非対応（96）のまま。
- 2026-10-31 の見分け: `process_v2_default` は `process=2`・`thread_before=2` で見分けに使え、`window=1`。当時の「v2 に立てた後でも system」は測り方の取り違えではなく、上と同じ `MessageBoxW` の振る舞い。3.2 の条件（`window=2`）には当たらない。
- 測りの不具合: 見つけた窓へ `PostMessageW(WM_COMMAND, IDOK, 0)` を送ってもメッセージボックスは閉じない（外から同じ合図を送って確かめた）。`WM_CLOSE` なら閉じる。今回は外から `WM_CLOSE` を送って 4 つとも閉じた（読みは閉じる前に画面の中で済んでいる）。測りを使い直す前に閉じ方を `WM_CLOSE` へ直す。
- 案 B の測り（2026-11-01・開発者の指示・`probe_alert_task_dialog_window_dpi`・comctl32 v6 の `TaskDialog`、OK だけ・エラーのアイコン）の 3 行:
  - `case=taskdialog_unaware process=0 thread_before=0 thread_during=0 window=0 window_v2=false window_dpi=96 monitor_dpi=96 system_dpi=96 box=0`
  - `case=taskdialog_wrapped_from_unaware process=0 thread_before=0 thread_during=2 window=2 window_v2=true window_dpi=192 monitor_dpi=192 system_dpi=192 box=0`
  - `case=taskdialog_thread_v2 process=2 thread_before=2 thread_during=2 window=2 window_v2=true window_dpi=192 monitor_dpi=192 system_dpi=192 box=0`
- 見立て: タスクダイアログは `MessageBoxW` と違いスレッドの扱いを落とさず、囲めば（非対応のスレッドからでも）窓が v2 で作られる。案 B は 1.1 の `with_thread_per_monitor` と組めば効く。副のモニタ（150%）での DPI とにじみはまだ見ていない（測ったのは主のモニタだけ）。
- 測りの閉じ方を直した: メッセージボックスは `WM_CLOSE`、タスクダイアログは `TDM_CLICK_BUTTON`（IDOK）。直した後の `probe_alert_box_window_dpi` は人の手なしで 2.6 秒で終わり、値は関門 0 と同じ（判定どおり赤）。
- 3.2 の実機（2026-11-01・主 200%＝system DPI 192・排他テストの机を取って 1 回・26.3 秒で緑）の 3 行:
  - `case=alert process=0 thread_before=0 thread_during=2 window=2 window_v2=true window_dpi=192 monitor_dpi=192 system_dpi=192 topmost=false foreground=true box=()`
  - `case=ask process=0 thread_before=0 thread_during=2 window=2 window_v2=true window_dpi=192 monitor_dpi=192 system_dpi=192 topmost=true foreground=false box=Yes`
  - `case=install_warning_dialog process=0 thread_before=0 thread_during=2 window=2 window_v2=true window_dpi=192 monitor_dpi=192 system_dpi=192 topmost=true foreground=false box=()`
- 3.2 の撮った 3 枚の見立て（`targetlert-dpi-shotslert-dialogs-taskdialog.png`・リポジトリに入れない）: 3 つとも主のモニタに出て、字はにじまない。エラーの画面は題名・本文 3 行・エラーのアイコン・OK・閉じるボタンあり。はい／いいえの画面は情報のアイコン・はい(Y)／いいえ(N)・閉じるボタンなし（設計どおり）。警告の画面は警告のアイコン・OK・閉じるボタンあり。
- 3.2 の前面の読み: はい／いいえと警告の画面は最前面になったが前面にはならなかった（題名の帯が灰色）。テストの実行ファイルが前面のプロセスでないので、OS が `SetForegroundWindow` を断ったと見る（今の `MB_SETFOREGROUND` も同じ OS の決まりに従う）。設計どおり記録しない・assert しない。
