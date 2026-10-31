# ギャップ分析: alert-dialog-per-monitor-dpi

> 2026-10-31・本ブランチ（main `9604677a` の上）のコードを読んだ結果。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> 対象は要件 1〜5（件 1＝告知の画面のにじみ）。件 2（モニタを移した後のバルーン）は見ていない（要件 5・ウェーブ C15 の約束）。

## 分析の要約

- **直す所は 1 か所に集まっている**: 3 つの画面は `crates/areka/src/alert.rs` の `raise`・`ask_yes_no`・`warn_ok` が `MessageBoxW(None, …)` を直に呼ぶだけ。呼び手（`main.rs` の `main`・`write_failure.rs` の `stop_boot_with`／`tell_at_close`・`install/before_boot_ports.rs` の `BootPorts`・`install/worker.rs` の `DeskPorts`）は口を呼ぶだけなので、口の中を変えれば全部に届く（要件 1.3・1.4 は 1 か所の直しで満たせる形）。
- **前例が在る**: `crates/areka/src/ghost_session_scope_rect.rs` の `os_window_rect` が「呼ぶ間だけスレッドの DPI の扱いをモニタごとの v2 にして、戻す・替えられなければ今のまま読んで記録」をすでにやっている。案 A はこれと同じ形。ただし前例の記録は `debug`、要件 1.5 は `warn` を求める（形は写せるが水準は違う）。
- **いちばん大きい分からないこと**: 2026-10-31 の実機で窓が返した awareness=1（system aware）。areka のコードには system aware を立てる所が無く、この機械の互換の設定（`AppCompatFlags\Layers`）にも areka の行は無い。「誰が system を選んだか」は OS の画面の作り（`MessageBoxW` の中）か、測り方の側の見込みが強い。前者なら **案 A（スレッドを囲む）は効かない**見込みがあり、案 B／C へ行く。設計の最初の実機の走行でこれを決める。
- **案 B の前提はそろっている**: comctl32 の版 6 の申告（`crates/areka/areka.manifest`・`build.rs`）も、`windows` クレートの機能 `Win32_UI_Controls`（ワークスペースの `Cargo.toml`）もすでに在る。`TaskDialogIndirect` に替えるのに依存の設定は増えない見込み。ただし最前面の指定（`MB_TOPMOST`・`MB_SETFOREGROUND`）にそのままの旗が無く、要件 2.1（見た目と指定を今と同じ）とぶつかる所がある。
- **規模**: S（3〜5 タスク）。**危うさ**: 中（案 A が効くかが実機の 1 回で決まるまで分からない・効かなければ案 B へ広がる）。

## 1. 今あるもの

| 物 | 今 | 定義点 |
|---|---|---|
| 3 つの画面の口 | `MessageBoxW(None, 本文, 題名, 旗)`。エラー＝`MB_OK|MB_ICONERROR`、はい／いいえ＝`MB_YESNO|MB_ICONQUESTION|MB_SETFOREGROUND|MB_TOPMOST`、警告＝`MB_OK|MB_ICONWARNING|MB_SETFOREGROUND|MB_TOPMOST`。戻り 0 で `Error::from_thread()` を読み `*_box_failed` を error で残す | `alert.rs` の `raise`・`ask_yes_no`・`warn_ok` |
| 口を呼ぶスレッド | メインのスレッド（起動の解決の失敗・記憶の確かめの失敗・起動窓の失敗・後始末の告知・起動の前のインストール）と、背景のスレッド `install`（窓口のインストールの利用条件と警告） | `main.rs` の `main`・`write_failure.rs`・`install/before_boot_ports.rs`・`install/worker.rs` の `spawn_worker`／`DeskPorts` |
| プロセスの DPI の扱い | `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` を `Once` で 1 回。呼ぶのは `WinApp::with_exit_policy` と `BootPorts::before_dialog`（抑止でないときだけ） | `crates/wintf/src/runtime/mod.rs` の `ensure_process_dpi_awareness`・`install/before_boot_ports.rs` の `BootPorts::before_dialog` |
| 窓のアプリを作る前の画面 | `resolve_boot` の失敗の `alert::raise` と `write_failure::check_boot` の失敗の `stop_boot` は `WinApp::with_exit_policy` より前。起動の前のインストールが画面を出していなければ、プロセスの扱いはまだ立っていない（既定＝非対応）。どちらも画面の後はそのまま終わる（`Err(E_FAIL)` を返す） | `main.rs` の `main` |
| 囲んで戻す前例 | `SetThreadDpiAwarenessContext(PER_MONITOR_AWARE_V2)` の戻り（前の扱い）が無効なら替わらなかったとして debug、有効なら読んだ後に戻す | `ghost_session_scope_rect.rs` の `os_window_rect` |
| 前例のテスト | 窓なしで「呼ぶ前と後のスレッドの扱いが同じ」を `GetThreadDpiAwarenessContext`＋`AreDpiAwarenessContextsEqual` で見るテストと、実の窓 1 枚を使う `#[ignore]` のテスト。マニフェストの無いテストの exe でもスレッドを v2 にすれば窓は v2 で作られる（同テストの注釈と `assert_ne!(dpi, 96)`） | `ghost_session_scope_rect_tests.rs` |
| 告知のテスト | どれも `suppressed = true` で呼び、記録だけを `log-capture-kit` で見る（画面は出さない）。抑止なしの呼び出しは決定論のテストに入れない決まり | `alert_tests.rs`・`alert_memory_write_tests.rs` |
| 申告と機能 | comctl32 の版 6 の申告を bin（と bin のユニットテスト）に埋める。`tests/` の結合テストの exe には入らない。`windows` の機能 `Win32_UI_Controls`・`Win32_UI_HiDpi`・`Win32_UI_WindowsAndMessaging` はワークスペースで有効 | `crates/areka/areka.manifest`・`crates/areka/build.rs`・ルートの `Cargo.toml` |
| ウェーブの約束 | ③＝`crates/areka/src/alert.rs` だけ。件 2 は取らない（`emo2_boot/frame/dpi.rs` に触らない）。`alert.rs` を挙げるほかの spec は C15 に無い | `roadmap.md` の「ウェーブ編成」の C15 の行 |

決まりごと（steering）: `unsafe` は Windows の API の呼び出しに限り、安全の理由を書く（`tech.md`）。テストは実装の隣の兄弟のファイル（`#[path = "…_tests.rs"]`）。記録は `tracing` の構造化の欄（`logging.md`）・テストの捕捉は `log-capture-kit` を通す。

## 2. 要件ごとの対応

| 要件 | 今あるもので足りるか | 印 |
|---|---|---|
| 1.1・1.2 出たモニタの拡大率で描く（主でも副でも） | 足りない。窓は system aware で作られている（実測）。主（200%＝system DPI）では見た目に差が出ず、副（150%）で縮めて描かれる見込み | **Missing**・**Unknown**（案 A で直るか） |
| 1.3 窓のアプリの前と後の両方 | 口が 1 つなので、口の中の直しなら両方に届く。スレッドを囲む形はプロセスの扱いが立っていなくても効く（前例のテストの注釈が裏づけ） | Missing（直しは 1 か所） |
| 1.4 3 つの画面・どのスレッドでも同じ扱い | 3 つの呼び出しを 1 つの関数に通せば満たせる。背景のスレッド `install` も同じ関数を通る | Missing（構造の直し） |
| 1.5 替えられなければ今と同じに出し warn を 1 件 | 前例は debug で残す。warn の語（event 名）は未定 | Missing・**Constraint**（前例と水準が違う） |
| 2.1 題名・本文・ボタン・アイコン・前面と最前面・答え | 案 A なら旗も戻り値もそのまま。案 B は最前面の旗が無い・見た目（大きい見出しと本文の分け方）が変わる | Constraint（案 B／C で効く） |
| 2.2 抑止と記録の名前・水準・欄 | 囲む所を「抑止の判断と記録の後・`MessageBoxW` の直前」に置けば変わらない | 足りる（置き場所の注意だけ） |
| 2.3 出る場所は OS 任せのまま | 案 A は持ち主の窓を渡さないのでそのまま | 足りる |
| 3.1・3.2 閉じた後／出せなかった後にスレッドの扱いを戻す | 前例の形（戻りを取っておいて戻す）で満たせる。出せなかったときも、戻してから `*_box_failed` を残す順にする（今は `from_thread()` を呼んですぐ記録するので、戻す呼び出しが最後のエラーの値を上書きしないよう**先にエラーを読んでから戻す**か、戻してから読むなら値が変わらないことを確かめる要がある） | Missing・**Constraint**（GetLastError の順） |
| 3.3 ほかの窓の大きさ・位置・座標の読み取りが変わらない | ゴーストとバルーンの窓は作った時の扱いで動き、窓のプロシージャはその扱いへ自動で切り替わる（Microsoft の説明）。スレッドを戻せば響かない見込み | Unknown（実機 4.5 で見る） |
| 4.1 画面なしの決定論のテスト | 前例の「前と後が同じ」テストの形が使える。ただし「替えられなかったとき」を本物の API で起こす方法が無い＝差し込みの口（切り替える関数を引数で受ける形など）が要る。「3 つが同じ扱いを通る」は振る舞いでなく構造の主張なので、ソースを読むテスト（`readonly_tests.rs` の `main.rs` の走査と同じ形）か、閉じた関数の引数で確かめる形になる | Missing |
| 4.2〜4.5 実機の短い 1 回 | 机の順取り・`GetDpiForWindow` の読みは前例あり（7.2）。副のモニタに画面を出す手順が未定 | **Unknown**（手順） |
| 5 件 2 を渡す | 完了の手続きの側。コードの直しは無い | 足りる（完了の時の作業） |

## 3. 「v2 に立てた後でも system aware と返った」件の見込み

7.2 の記録: プロセスの扱いを立てた行は画面より前に 1 件・失敗 0 件。画面の窓は `GetDpiForWindow`=192（主のモニタ 200%）・`GetWindowDpiAwarenessContext` の awareness=1。

手がかり: 返った値が **0（非対応）でも 2（モニタごと）でもなく 1（system）**。何もしなければ非対応（0）になるので、1 はどこかが system を選んだ印。

- areka のソースで `SetThreadDpiAwarenessContext`／`SetProcessDpiAwarenessContext` に渡す値は v2 と非対応（テストだけ）しか無い（grep 済み）。
- この機械の `HKCU`／`HKLM` の `Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers` に areka の行は無い（OneDrive の行だけ・読んで確かめた）。互換の上書きで system に倒された見込みは低い。

見込み（強い順）:

1. **OS の標準のメッセージボックスが、中で窓を system aware で作っている**。もしそうなら、スレッドを v2 に囲んでも同じ結果になり、案 A は効かない。Microsoft の文書は「モニタごとの v2 では `CreateDialog` 系のダイアログが自分で拡大を追う」とは書くが、`MessageBoxW` そのものの扱いは書いていない（下の出典）。外の掲示板には「v2 にしたらほかの画面は鮮明だがメッセージボックスはぼやける」という報告が 1 件ある（AutoHotkey の掲示板・第三者の製品の話で裏づけとしては弱い）。
2. **測り方の側**: 別のプロセスから窓を探して読んだ（クラス名や題名で探して、別の窓を拾った・プロセスをまたいだ読み）。areka の中から読んでいないので、取り違えを除けていない。
3. **呼んだスレッドの扱いが、そのとき v2 でなかった**（依存のクレートがスレッドの扱いを替えた等）。ワークスペースには見当たらないので弱いが、`GetThreadDpiAwarenessContext` を画面の直前に読めば 1 回で除ける。

見分け方（設計の最初の実機の走行・案 A の確かめと同じ走行で済む）: areka の中で ⑴ 画面の直前に呼んだスレッドの扱いを読み、⑵ 画面の窓（同じスレッドに出るので、フックかタイマーで取れる）の `GetWindowDpiAwarenessContext` と `GetDpiForWindow` を読み、⑶ 囲む／囲まないの 2 通りで比べる。囲んでも 1 なら見込み 1 が当たり＝案 A は捨てる。

## 4. 直し方の案

### 案 A: 画面を出す間だけスレッドを v2 に囲む（今ある物を広げる）

- **触る所**: `alert.rs` だけ。`MessageBoxW` の 3 か所を、囲んで戻す 1 つの関数に通す（`unsafe` はその関数とその 3 か所に閉じる）。テストは兄弟のテストのファイル（今の `alert_tests.rs` に足すか、新しい兄弟を足すか）。
- **良い所**: 差分が小さい・前例と同じ形・旗も戻り値も変わらない（要件 2 をそのまま守れる）・窓のアプリの前の画面も同じ道で直る（プロセスの扱いに頼らない）・背景のスレッドでも同じ。
- **悪い所**: 見込み 1 が当たれば効かない。効くかは実機の 1 回でしか分からない。
- **注意**: 戻す呼び出しが最後のエラーの値を上書きしうる（要件 3.2 の順）。替えられなかったときの記録の水準（warn）と語は前例と違う。

### 案 A′: 口の中でプロセスの扱いも先に立てる（A に添える小さな足し）

- `alert.rs` の口が、出す直前に `wintf::ensure_process_dpi_awareness()` を呼ぶ（`BootPorts::before_dialog` と同じ）。窓のアプリの前の画面でもプロセスが v2 になる。
- 7.2 ではプロセスが v2 でも 1 が返っているので、**これだけでは直らない**。A が効いたときは要らない（スレッドを囲めば足りる）。A が効かず B へ行くときに、B の画面をモニタごとに描かせるのにプロセスかスレッドの扱いが要るかで意味が出る。
- 気をつける点: プロセスの扱いは UI を作る前に 1 回だけ立てる決まり。今の窓のアプリの前の画面はどれも画面の後に終わるので崩れないが、将来「画面を出して起動を続ける」道が増えると順が効く。

### 案 B: comctl32 の版 6 の `TaskDialogIndirect` へ替える（新しい部品へ）

- **触る所**: `alert.rs` だけで収まる見込み（申告も `windows` の機能も在る）。
- **良い所**: comctl32 の版 6 の画面で、モニタごとの扱いのスレッドから出せば拡大を追う見込み（要確かめ）。見込み 1 が当たっても使える道。
- **悪い所**: 要件 2.1 とぶつかる。最前面（`MB_TOPMOST`）と前面（`MB_SETFOREGROUND`）にそのままの旗が無く、作られた知らせ（`TDN_CREATED`）の中で窓に自分で指定する形になる。見出しと本文の分け方・ボタンの並び・アイコンの大きさが変わる（「見た目は同じ」を言えない）。はい／いいえで閉じるボタンを効かなくする扱いも確かめが要る。版 6 の申告が無い exe（`tests/` の結合テストの exe など）では関数が見つからず失敗する（本番の bin には入っているので画面を出す道には響かない見込み）。スレッドの扱いは結局 v2 で呼ぶ要がある見込み（A の囲みと一緒に使う）。

### 案 C: 自前の窓で画面を作る（wintf）

- **良い所**: 見た目も DPI も自分で持てる。
- **悪い所**: 規模が大きい（窓・文字・ボタン・キーの扱い・前面の扱い）。`alert.rs` の外（wintf）へ広がり、ウェーブの約束に収まらない。窓のアプリを作る前（`WinApp` が無い）にも出せる形が要る。この spec の規模（3〜5）を大きく超える。

### 組み合わせの見立て

| 実機の 1 回の結果 | 進む道 |
|---|---|
| 囲むと画面の窓が v2 になり、副のモニタの DPI と同じになる | 案 A（A′ は要らない） |
| 囲んでも system のまま | 案 B（＋A の囲み）を開発者に諮る。要件 2.1 の「見た目は同じ」をどこまで緩めるかが要る |
| B でも system のまま | 案 C しか残らない＝この spec の規模とウェーブの約束を超えるので、開発者に戻す |

## 5. 規模と危うさ

- **規模: S（3〜5 タスク）**。案 A なら口の中の 1 つの関数・テスト・実機の 1 回。案 B に替わっても `alert.rs` の中で 5〜7 程度に収まる見込み。案 C なら L 以上（この spec の外）。
- **危うさ: 中**。前例の形と知った API で、触る所も 1 ファイル。ただし案 A が効くかが分からず、効かなければ画面の見た目が変わる案 B の裁定が要る。

## 6. 設計への申し送り

### 推奨の進め方（決めるのは討議と設計）

- 設計の最初に、areka の中から測る短い実機の走行を 1 回置き、§3 の見分けと案 A の効き目を同時に決める。効かなければ設計を進めずに開発者へ戻す（要件の討議の点 1 と同じ）。
- 囲む関数は前例 `os_window_rect` の形を写し、記録の水準だけを warn にする。

### 調べること（Research Needed）

1. `MessageBoxW` が呼んだスレッドの DPI の扱いに従って窓を作るか（§3 の見込み 1）。Microsoft の文書に `MessageBoxW` についての記述は見つからなかった。
2. 持ち主の窓の無い `MessageBoxW` がどのモニタに出るか（主のモニタか・前面の窓やカーソルのあるモニタか）。副のモニタに出す手順（要件 4.3）がこれで決まる。手順の候補: ⑴ 150% のモニタを一時的に主にする（system DPI はサインインの時に決まるので、主を替えても system DPI は 200% のまま＝「system DPI と違うモニタに出す」条件がそのまま作れる見込み・要確かめ）、⑵ 前面の窓やカーソルを副のモニタに置いてから出す、⑶ 出した後に手で動かす（これは「出た後に動かしたときの描き直し」で、境界の外の振る舞いを測ることになる）。
3. スレッドの扱いを戻す呼び出しが、`MessageBoxW` が残した最後のエラーの値を変えるか（要件 3.2 の記録の中身）。
4. 案 B に進むとき: v2 のスレッドから出した `TaskDialogIndirect` がモニタごとに描かれるか・最前面と前面の指定の再現・閉じるボタンの扱い。
5. 「替えられなかった」道を決定論のテストで踏む形（本物の API では起こせないので、切り替える関数を差し込む口が要る）。

### 設計で決めること（要件の討議で設計の段へ回した点）

6. 替えられなかったときの warn の記録の語（3 つの画面で 1 つの語か、画面ごとか）。前例 `scope_rect_dpi_context_unchanged` は debug で語も別。
7. テストの置き場（今の `alert_tests.rs` に足すか、`alert.rs` の新しい兄弟のテストのファイルか）。どちらも `alert.rs` の兄弟なのでウェーブ C15 の約束（`alert.rs` だけ）の内と読む。
8. 要件 4.4 が当たるか（窓のアプリの前と後で扱いを分ける設計になるか）。案 A だけなら分けない＝当たらない。案 A′ を添えるなら当たる。

### ほかに見えたこと（この spec の外・起票はしていない）

- フォルダや書庫を選ぶ画面（`crates/areka/src/roots/pick.rs`・`crates/areka/src/install/pick.rs` の OS のファイル選択の画面）も OS の標準の画面。こちらは窓のアプリの後に UI のスレッドから出るので v2 の扱いで出ている見込みだが、確かめていない。要件の範囲（`alert.rs` の 3 つ）の外。

## 7. 討議に出す点（答えで作業が変わるもの）

1. **直し方**: 案 A を先に実機で確かめ、効かなければ止めて戻す（要件の「討議で決める点 1」）。§3 の手がかり（返った値が 1）から、案 A が効かない見込みが小さくないことを踏まえて、効かなかったときに案 B（見た目が変わる）へ進むのを討議の時点で許すか、そのときに改めて諮るか。
2. **替えられなかったときの記録**: 要件 1.5 の warn の event の語を決める（前例 `scope_rect_dpi_context_unchanged` は debug で別の語）。3 つの画面で 1 つの語にするか、画面ごとに分けるか。
3. **テストの置き場**: ウェーブの約束は「`alert.rs` だけ」。囲む関数のテストを今の `alert_tests.rs` に足すか、新しい兄弟のファイルを足すか（新しいファイルが約束の内か）。
4. **副のモニタに出す手順**（要件 4.3）: §6 の調べること 2 の候補 ⑴〜⑶ のどれにするか。⑴（主を一時的に替える）は機械の設定を替えて戻す操作が要る。
5. **窓のアプリの前の画面の確かめ**（要件 4.4）: 案 A ならスレッドの扱いだけで前と後を同じ道で直すので、扱いを分けない＝4.4 は当たらない、と読んでよいか。案 A′ を添えるなら前と後で道が変わり、両方を 1 つずつ見る要がある。

## 出典

- [Mixed-Mode DPI Scaling and DPI-aware APIs（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-improvements-for-desktop-applications) — 窓の扱いは作った時の呼んだスレッドの扱いで決まる・窓のプロシージャはその扱いへ自動で切り替わる・`GetDpiForWindow` は system aware の窓には system DPI を返す
- [SetDialogDpiChangeBehavior（Microsoft Learn）](https://learn.microsoft.com/ko-kr/windows/win32/api/winuser/nf-winuser-setdialogdpichangebehavior)・[DIALOG_DPI_CHANGE_BEHAVIORS](https://learn.microsoft.com/en-gb/windows/win32/api/winuser/ne-winuser-dialog_dpi_change_behaviors) — v2 のダイアログは自分で拡大を追う（`MessageBoxW` についての記述は無い）
- [SetProcessDpiAwarenessContext（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/desktop/api/winuser/nf-winuser-setprocessdpiawarenesscontext) — DPI の扱いに頼る API より前に呼ぶ決まり
- [Supporting Per-Monitor (V2) DPI Awareness（Appeon・第三者）](https://docs.appeon.com/pb2025r2/application_techniques/Supporting_PerMonitorV2.html) — v2 で Win32 のダイアログ（その製品の MessageBox を含む）が拡大を追うとする
- [AutoHotkey の掲示板](https://www.autohotkey.com/boards/viewtopic.php?p=70977) — v2 にしてもメッセージボックスがぼやけるという利用者の報告（第三者・裏づけは弱い）

## 設計の段の調べと決めたこと（2026-11-01・`/kiro-spec-design -y`）

### 調べの範囲

- **種類**: 既にある仕組みの手直し（Extension）＝軽い調べ（light discovery）。触るのは `crates/areka/src/alert.rs` と新しい兄弟のテストのファイルだけ。新しい依存は無い。
- **使った手引き**: `kiro-spec-design` の決まり（設計の原則・軽い調べ・まとめ・審査の関門）、steering の `tech.md`（`unsafe` は Windows の API の呼び出しだけ・安全の理由を書く）・`logging.md`（構造化の欄・`log-capture-kit`）・`doc/impl-watch.md`（`#[ignore]` の実機テストを `--ignored` で回すのは排他テストの机）。

### 分かったこと

1. **機能はそろっている**: 囲みと関門 0 の測りに要る API（`SetThreadDpiAwarenessContext`・`GetWindowDpiAwarenessContext`・`GetAwarenessFromDpiAwarenessContext`・`GetDpiForWindow`・`GetDpiForMonitor`・`GetDpiForSystem`・`SetTimer`・`EnumThreadWindows`・`PostMessageW`・`MonitorFromWindow`・`GetCurrentThreadId`）は、ワークスペースの `Cargo.toml` で既に有効な `Win32_UI_HiDpi`・`Win32_UI_WindowsAndMessaging`・`Win32_Graphics_Gdi`・`Win32_System_Threading` にある。`Cargo.toml` は触らない。
2. **申告に DPI の行は無い**: `crates/areka/areka.manifest` は comctl32 の版 6 の申告だけ。areka.exe もテストの実行ファイルも、プロセスの既定は非対応で始まる。bin のユニットテストの実行ファイルにも同じ申告が埋まる（`build.rs`）ので、関門 0 は本番と同じメッセージボックスの部品で測れる。
3. **2026-10-31 の値 1 が正しければ、案 A は効かない見込みが強い**: 7.2 では画面の前にプロセスを v2 に立てていた。スレッドは何も替えていなければプロセスの既定を受けるので、そのときスレッドは既に v2 だった見込みが高い。その状態で画面の窓が system（1）だったのなら、スレッドを v2 に囲んでも同じ状態を作り直すだけで、結果は変わらない。外の報告にも「古い `MessageBox` は v2 に合わせて直されておらず、タスクダイアログは正しく拡大される」という Stack Overflow の答えと、AutoHotkey の掲示板の報告がある（どちらも第三者・公式の記述ではない）。一方、7.2 の値は別のプロセスから読んだもので、窓の取り違えを除けていない。だから関門 0 で、7.2 と同じ形（プロセスを v2 に立てて、スレッドは替えない）も areka のコードの中から測る（場合 `process_v2_default`）。
4. **最後のエラーの順（調べること 3）は問わずに済む**: `MessageBoxW` が戻った直後、スレッドを戻す前に最後のエラーを読んで持ち帰る形にすれば、戻す呼び出しが値を上書きしても記録の中身は今と同じ。
5. **画面の窓は同じスレッドから見つけられる**: 持ち主の窓の無いメッセージボックスもモーダルの間に同じスレッドのメッセージを配るので、スレッドのタイマーの呼び戻しから `EnumThreadWindows` でクラス名 `#32770` の窓を見つけ、読んで、`WM_COMMAND`＋`IDOK` で閉じられる。見つからないときは `PostQuitMessage` でモーダルを抜けさせる。

### 決めたこと

| 点 | 決めたこと | 理由 |
|---|---|---|
| 形（まとめ: 一般化） | 3 つの口の `MessageBoxW` を 1 つの `show_box` に集め、その中で `with_thread_per_monitor` が囲む | 3 つの画面は同じ問題の変種。1 つの道にすれば 1.3・1.4 が形で満たされる |
| 差し込みの口（まとめ: 簡素化） | `with_thread_per_monitor` だけが切り替えの関数を引数で受ける。`MessageBoxW` は差し込まない | 「替えられなかった」道は本物の API では起こせない（4.1）。画面の方は決定論のテストで出さないので差し込む要が無い |
| 作るか取り入れるか | OS の標準のメッセージボックスをそのまま使い、スレッドの扱いだけを替える（案 A）。タスクダイアログ（案 B）と自前の窓（案 C）は関門 0 で効かなかったときの開発者の裁定 | 要件の討議で決めた。見た目と旗を変えずに済む（2.1） |
| 案 A′（プロセスも立てる） | 取らない | スレッドを囲めば窓のアプリの前でも足りる（関門 0 の `wrapped_from_unaware` で確かめる）。プロセスの扱いを立てる時を増やすと「UI を作る前に 1 回だけ」の決まりに順が効く |
| 設計で決めること 6（warn の語） | 3 つの画面で 1 つの語 `alert_dpi_context_unchanged`（warn）・どの画面かは欄 `dialog`（"alert"・"ask"・"install_warning_dialog"） | 拾う側は語 1 つで足りる。前例 `scope_rect_dpi_context_unchanged` は debug で水準が違うので混ぜない |
| 設計で決めること 7（テストの置き場） | 新しい兄弟 `crates/areka/src/alert_dpi_tests.rs`（`alert.rs` の `#[path]`） | `alert.rs` の兄弟なのでウェーブ C15 の約束の内。今の `alert_tests.rs` は文面と抑止のテストで、見るものが違う |
| 設計で決めること 8（要件 4.4） | 当たらない。窓のアプリの前と後で道を分けない | スレッドだけを替えるので、プロセスが非対応でも v2 でも同じ道。プロセスが非対応の前の画面は関門 0 の `wrapped_from_unaware` が中から確かめる |
| 要件 4.6 の「設計の最初」 | 実装のタスクの 1 番目（関門 0）で測る。関門 0 は囲みの関数と測りだけを作り、3 つの口へは繋がない。通るまで先へ進まない | 設計を書く段では画面を出せない。測りが本番の `show_box` を通るには、その関数が先に要る |
| 実装の後の実機の画面 | 起動の前のインストールの利用条件の画面（7.2 と同じ形）。窓の DPI はプロセスの番号で絞った窓をその場限りのスクリプトで読む | 「はい」で起動が続くので 4.5 を同じ 1 回で見られる。番号で絞れば 7.2 の取り違えの見込みを除ける |

### 危うさと手当て

- **案 A が効かない見込みが小さくない**（上の 3）— 関門 0 を実装の最初に置き、効かなければ 3 つの口へ繋ぐ前に止めて開発者へ戻す。本番の振る舞いはその時点で変わっていない。
- **測りが別の窓を拾う**— 同じスレッドの窓だけを、クラス名と測りの題名で絞って読む。
- **測りの画面が閉じずに止まる**— 20 回（およそ 6 秒）で `PostQuitMessage` で抜け、赤として返す。
- **プロセスを v2 に立てる場合は戻せない**— 最後の場合に置き、名指しと `--test-threads=1` でこの測りだけを走らせる。

### 出典（設計の段で足した物）

- [Stack Overflow の答え（kiwix の写し）](https://kiwix.lema.org/stackoverflow.com_en_all_2023-11/a/43234355) — 古い MessageBox は v2 に合わせて直されておらず、タスクダイアログは正しく拡大されるという報告（第三者）
- [DPI_AWARENESS_CONTEXT handle（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/desktop/hidpi/dpi-awareness-context) — v2 の扱いと、v2 で作ったダイアログが拡大を追うこと
- [AutoHotkey の掲示板](https://www.autohotkey.com/boards/viewtopic.php?p=70977) — 既出・v2 でもメッセージボックスがぼやける報告（第三者）
