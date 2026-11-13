# 設計の検証: audit-shiori-host32

- 日付: 2026-11-13
- 読んだもの: `spec.json`・`requirements.md`（要件 10＝討議の決め）・`design.md`・`research.md`（§4 の指摘 48 件・§14 の設計の段の調べ）・steering（`roadmap.md` の C19 の行の約束 ⑴〜⑸）
- ソースで確かめたもの: helper `main.rs`（REQUEST・LOAD・UNLOAD の腕と `RefCell` の借り方）・ipc `lib.rs`（`send_copydata` が `wParam` に送り手の窓を入れる所・`ResponseSlot::store` は後勝ち）・host `parent_window.rs`（HELLO と RESPONSE の受け方）・`process_host.rs`（`HelperHandle`・`spawn`・`CREATE_NO_WINDOW` は private）・`lifecycle.rs`（`HelperLifecycle` の Drop・`EXIT_POLL_INTERVAL` は公開の定数）・`log-capture-kit/tests/temp_path_guard_test.rs`（例外表）・`areka-ghost/src/shiori_wiring.rs`・`areka-ghost/tests/ghost/common.rs`（`real_pasta_turn` と 3 つの使い手）・ツリー全体の grep（消す項目の使い手・エラーの表示の文の比べ）

## 設計の検証のまとめ

設計は、要件の討議で決めた 46 件の直しを、指摘 1 件ごとに「直す場所・直し方・戻したら落ちるテスト」で書き切っていて、18 のまとまり（タスクの上限 20 の内）に無理なく割れている。公開の形・`crates/areka/`・`vendors/`・`Cargo.lock` の約束も、ソースと照らして守られている。直しておきたいのは 3 点で、どれも設計の文を少し直せば済み、作りの向きを変えるものではない。

## ソースで確かめた結果（設計の主張ごと）

| 主張 | 結果 |
|---|---|
| 指摘 8: REQUEST の腕が DLL の呼び出しのあいだ `s.proxy.borrow()` を持ち、入れ子の UNLOAD の `borrow_mut()` が窓の手続きの中で panic する | 本当（`main.rs:275` で借り、`:365` で `borrow_mut`）。設計の直し（`in_dll_call`・`unload_pending` の 2 つの旗・入れ子の UNLOAD は proxy に触れず ack [1]・戻ってから `close`）で panic の元は消える。入れ子の LOAD も、今の作りで共有の借りだけなので衝突しない |
| 指摘 38: HELLO を「1 度だけ・`wParam` と中身が同じ・`expect_helper_pid` を呼べば子のプロセスの窓だけ」で受ける | HELLO の乗っ取り（中身の窓の値の差し替え）は、この 3 つで防げる。ただし `wParam` は送り手が自分で入れる値（ipc `lib.rs:388`）なので、RESPONSE の確かめ（`wParam == helper_hwnd`）は差し替えを防げない（下の重大な指摘 1） |
| `HelperHandle` に Drop を置き、`HelperLifecycle` の Drop を消す | 今の `HelperLifecycle` の Drop は `terminate` だけ（`lifecycle.rs:255-264`）なので、`HelperHandle` の Drop へ移しても振る舞いは同じかそれ以上。`real_connect` は接続ごとに親の窓と `HelperHandle` を新しく作り、HELLO の時間切れで `HelperHandle` を捨てる＝指摘 14 は本物で、直しが効く |
| 標準エラーの読み手（UTF-8／CP_ACP・`warn`・Drop で 1 秒まで待つ） | 形は成り立つ。`spawn_command` の公開の形は変わらず、`HelperHandle` から子の標準エラーを読む道は外に無い。気になる組み合わせは下の重大な指摘 3 |
| `temp_path_guard_test.rs` の例外表から本範囲の 10 件を外す | 表に載る範囲のファイルは数えて 10 件（ProcessUnique 9・ReadOnly 1）。外すと件数の定数 `ALLOWED_COUNT` 13→3・`PROCESS_UNIQUE_COUNT` 11→2・`READ_ONLY_COUNT` 2→1 と、「入口が複数あるファイルは 2 件」の説明（`shiori_proxy.rs`・`process_host.rs` を名指し）も直す要がある。設計の「件数と説明を直す」で読める。このファイルは C19 のほかの 7 本の触るファイルに無い（`runtime-module-untangle` が触るのは `file_length_guard_test.rs`・`workspace_scan_test.rs`） |
| 指摘 26 と `EXIT_POLL_INTERVAL`・`PAYLOAD_HEADER_LEN` の消去 | 範囲の外（`pilot` を除く `crates/`・`tools/`・`doc/`）の使い手は 0（grep で確かめた）。`crates/areka/` が使うのは `ExitKind`・`HelperStatus`・`RequestError`・`ShutdownError`・`RequestHeaders`・`ValueReply` だけで、どれも形が変わらない |
| 指摘 6（表示の文を変える） | 文字列で比べる使い手は `areka-kanade/src/shiori/real_tests.rs` だけで、どれも同じ値の `to_string()` と比べる＝文を変えても緑のまま |
| タスクの数 | 18 のまとまり（U17 は待ちあり）＝上限 20 の内 |
| 1,000 行 | 今いちばん近いのは `client.rs` 957・`shiori3.rs` 879・`shiori_proxy.rs` 819。中のテストを兄弟へ出す手当てと見込みの表は筋が通る。`parent_window.rs` 694→約 820 が次に近い |
| `Cargo.lock` | 変わるのは U17（`dola-yaml-backend` の着地の後）だけ。機能の行の足し（`Win32_Globalization`・`Win32_System_DataExchange`）と試験用の DLL の `workspace = true` は `Cargo.lock` に載らない（R6・R8 の読みのとおり） |
| `crates/areka/`・`vendors/` | 触るファイルの一覧に無い。本体の実機の試験が探す `[helper] LOAD 失敗（観測・ack[0]）: …` の文は、`warn` の本文にそのまま入るので探せる（R2） |
| `real_pasta_turn` の 3 つの使い手 | 3 つとも `resolve_helper_exe` で helper を通す＝件 2 の直しの後は 9276 の取り合いが起きない。ただし錠を外す時期に注意が要る（下の重大な指摘 2） |

## 重大な指摘（3 件）

### 重大な指摘 1: 指摘 38 の RESPONSE の確かめは、応答の差し替えを防げない

- **何が問題か**: `WM_COPYDATA` の `wParam` は送り手が自分で入れる値で、OS は確かめない（ipc `lib.rs:388` も `WPARAM(self_hwnd)` を自分で入れている）。同じ利用者のほかのプロセスは、メッセージだけの窓を数え上げて helper の窓を見つけ（`FindWindowEx(HWND_MESSAGE)`＋`GetWindowThreadProcessId`）、`wParam` に helper の窓の値を入れた偽の RESPONSE を送れる。設計の「Security Considerations」と「親の窓の送り手の確かめ」は「応答も差し替わらない」と書いているが、そうはならない。HELLO の側は「`wParam` と中身が同じ」と「子のプロセスの窓」の組み合わせで中身の窓の値が子の窓に縛られるので、こちらは防げる。
- **なぜ効くか**: 要件 9 のかけ直しの audit が同じ所を「確かめが偽」として指し直すおそれがあり、0 件に収まらない元になる。設計の文が事実と違うまま残る。
- **直し方（推す）**: RESPONSE の確かめは残し（素朴な取り違えは防げる）、その場所に `// shortcut: wParam は送り手が入れる値で、同じ利用者のプロセスは helper の窓の値を名乗って RESPONSE を偽れる。閉じるには起こすたびの合言葉を枠に載せる（取り決めの変更）要がある。偽の RESPONSE が実際に問題になったら上げる` の注釈を置き、「Security Considerations」と「Revalidation Triggers」の文を「HELLO の乗っ取りは防ぐ・RESPONSE は取り違えだけを防ぐ」に直す。これは要件 2.2 ⑵（`shortcut:` の注釈で、書いた限界が今の負荷を越えていないもの）に当たる。本当に閉じたいなら合言葉（環境で helper へ渡し、HELLO と RESPONSE の枠に載せる）だが、取り決めの形が変わり規模が増えるので推さない。
- **要件**: 3.1・9.2・10.2
- **設計の場所**: 「host・窓」の「親の窓の送り手の確かめ（指摘 38）」・「Security Considerations」の 38・「Revalidation Triggers」

### 重大な指摘 2: `real_pasta_turn` を外す時期（U4）が、窓 2 組の確かめ（U8）より前にある

- **何が問題か**: 錠 `real_pasta_turn` は 9276 の取り合いを避けるためのものだが、同じテストの実行ファイルの中で、窓を作る実物の pasta のテストを 1 本ずつにする働きも兼ねている。設計は U4 で錠を外し、「1 プロセスに窓 1 組」の制約が本当に無いかを確かめるテスト（`two_parent_windows_live_side_by_side_on_two_threads`）は U8 で初めて回す。U8 が赤だったときの逃げ道は host の中の錠だけを書いていて、`areka-ghost` の 3 本が並んでぶつかる場合を書いていない。
- **なぜ効くか**: R4 の見立てが外れると、要件 10.5 の直しが、`HOST32_PASTA_DLL` を立てて回す開発者の手元のテストを揺らす（要件 4.2 の決定論）。直したあとで錠を戻すと、要件 10.5 と食い違う。
- **直し方（推す）**: `areka-ghost` の錠を消す作業（要件 10.5）を U4 から U8 の後ろ（U8 の緑を確かめた後）へ移す。または U8 の「赤なら」の逃げ道に「`areka-ghost` の `real_pasta_turn` も、名前と説明を窓の制約の錠に改めて残し、要件 10.5 は開発者への議題にする」と書き足す。前者のほうが小さい（まとまりの表の 1 行を動かすだけ）。
- **要件**: 10.5・4.2・4.5
- **設計の場所**: 「修正のまとまり」の U4・U8 の行・「窓を作るテストの並び（指摘 40）」の「赤なら」

### 重大な指摘 3: 標準エラーの管が SHIORI の起こしたプロセスに受け継がれると、ゴーストを閉じるたびに 1 秒止まり、読み手のスレッドが残る

- **何が問題か**: 指摘 13 の直しで、SHIORI が起こしたプロセスはゴーストを閉じても生き残る。SAORI や C の実行時の関数（`system()` など）は、取っ手を受け継ぐ形でプロセスを起こすことが多く、その子は helper の標準エラーの管の書き口を持ったまま残る。すると読み手は管の終わりを受け取れず、`HelperHandle` の Drop は毎回 1 秒待ってから読み手を切り離す。切り離した読み手は、その子が終わるまで残り、その子の標準エラーを「helper の行」として `warn` で書き続ける。設計はこの場合を「切り離して `debug` で 1 行」とだけ書いている（`research.md` §14.6 にも載っていない）。
- **なぜ効くか**: ゴーストの切り替えのたびに SHIORI のスレッドが 1 秒止まり、スレッドが積み上がる。終了の道（要件 5.3）と、areka の終わりの締め切りに響くおそれがある。指摘 4 と 13 の組み合わせで新しく生まれる道なので、要件 9 のかけ直しの audit が指すおそれもある。
- **直し方（推す）**: helper の始めに、自分の標準エラーの取っ手を受け継がせない印にする（`SetHandleInformation(GetStdHandle(STD_ERROR_HANDLE), HANDLE_FLAG_INHERIT, 0)`・helper の `main.rs` に数行・`windows` の今の機能で足りるかは実装で確かめる）。これで管の書き口を持つのは helper だけになり、helper が終わればすぐに終わりが届く。1 秒の待ちと切り離しは安全網として残す。テストは、helper が起こした子（例 `ping.exe`）が生きているあいだに helper を終わらせ、読み手が 1 秒を待たずに終わることを確かめる 1 本で縛れる。足さない場合は、少なくとも設計と `research.md` §14.6 にこの道と上限（1 回の閉じで 1 秒・残るスレッドは子が終わるまで）を書いておく。
- **要件**: 5.3・10.7・10.12
- **設計の場所**: 「host・起こし方」の「標準エラーの読み手」・「`HelperHandle` の Drop・`wait_for_exit`」・「job の設定」

## 設計のよい所

1. **指摘 1 件ごとに、戻したら落ちるテストの名前と中身まで決めてある**。試験用の DLL に試しの ID（`OnTestNull`・`OnTestLenOverflow`・`OnTestReenter`・`OnTestUnloadFalse`・`OnTestEnv`）を足して、再入・長さの食い違い・null・`unload` の偽・環境の受け継ぎを決定的に作る形は、時間に頼らず実物の道を通るので、要件 4.1・4.2・4.4 をまとめて満たす。件 2 の試験を「自分の実行ファイルを `PASTA_DEBUG=1` で起こし直す」形にして、自分のプロセスの環境を書き換えないのも、並んで走るテストを乱さない良い選び方である。
2. **C19 の約束と討議の決めを、ソースの事実で裏打ちしてある**。公開の形は足すだけ（`expect_helper_pid` と Drop）に抑え、消す項目は `runtime-module-untangle` との申し合わせを記録し、`Cargo.lock` に触る直し（指摘 28）だけを待ちつきの U17 に分け、本体の実機の試験が探す文を守る道（R2）を確かめてある。番人の例外表を増やさないように、別のスレッドの記録の全体の捕捉を使わず、行を渡す口で試す形にしたのも、決まりに沿った小さな作りである。

## 小さな気づき（重大ではない・実装で拾えばよい）

- 要件 10.6 と 10.8 は「設計の段で確かめる・測る」と書くが、設計は R1（実物の pasta が読めるか）と R5（SSP の値の空白と重なり）を実装の段へ回している。R1 は `research.md` §5 の静かな読み（依存は OS の DLL だけ）が設計の段の証跡になり、R5 は測れないときの既定が要件で決まっているので、進めてよい。
- `EXIT_POLL_INTERVAL` は要件 10.9 の一覧に無い公開の定数で、設計は「知らせてから消す・異議があれば残す」としている。要件 6.5 の「申し合わせ」として読めるが、知らせた結果は `research.md` に残すこと。
- `process_host_stderr_tests.rs` が `#[path = "../tests/common/mod.rs"]` で結合テストの道具を引くと、ライブラリのテストの組みで使わない関数の警告が出るおそれがある。`tests/common/mod.rs` の頭で `#![allow(dead_code)]` にするなど、組みが警告で落ちない形にしておくこと。
- 標準エラーの行は 1 行の長さに上限が無い（改行の無い大きな書き込みは丸ごと貯まる）。今の負荷では当たらないので、直すなら `shortcut:` の注釈で足りる。

## 最終の判断

- **判断: GO**
- **理由**: 要件 1〜10 の全部に受ける所があり、C19 の約束（公開の形・`crates/areka/`・`vendors/`・`Cargo.lock`）とタスクの数・1,000 行の見込みもソースと照らして守られている。重大な指摘の 3 件は、どれも設計の文と 1 つのまとまりの中身を直せば済み、作りの向きを変えない。
- **次の手順**: 設計の討議（`/kiro-design-discussion audit-shiori-host32`）で上の 3 件を議題にし、決めた直しを `design.md` に入れてから `/kiro-spec-tasks audit-shiori-host32` へ進む。
