# 設計の確かめ: alert-dialog-per-monitor-dpi

> 2026-11-01・本ブランチのコードと照らして読んだ（`crates/areka/src/alert.rs`・`ghost_session_scope_rect.rs` の `os_window_rect` とそのテスト・`alert_tests.rs`・`build.rs`・`areka.manifest`・`wintf` の `ensure_process_dpi_awareness`・ルートの `Cargo.toml` の `windows` の機能・`windows` 0.62.2 の定義）。画面は出していない。

## まとめ

設計は小さく閉じていて、要件 1〜5 のすべてに部品か手順が当たっている。頼る API は `windows` 0.62.2 にすべて在り、要る機能（`Win32_UI_HiDpi`・`Win32_UI_WindowsAndMessaging`・`Win32_Graphics_Gdi`・`Win32_System_Threading`）もワークスペースで有効なことを確かめた。関門 0 の判定（`wrapped_from_unaware`）の作りは正しい。気になるのは、比べるための場合 `process_v2_default` の置き方と、実装の後の実機の確かめで出す画面の選び方の 2 点と、字面を見るテストの書き方の 1 点で、どれも関門 0 の判定そのものは崩さない。**GO**。

## 照らし合わせで確かめたこと

- **API の形**: `SetThreadDpiAwarenessContext`・`GetThreadDpiAwarenessContext`・`AreDpiAwarenessContextsEqual`・`GetWindowDpiAwarenessContext`・`GetAwarenessFromDpiAwarenessContext`・`GetDpiForWindow`・`GetDpiForMonitor`（`HMONITOR` は `Win32_Graphics_Gdi`）・`GetDpiForSystem`・`MessageBoxW`（戻りは `MESSAGEBOX_RESULT`）・`SetTimer`（`TIMERPROC` は `Option<unsafe extern "system" fn(HWND, u32, usize, u32)>`）・`KillTimer`・`EnumThreadWindows`・`GetClassNameW`・`PostMessageW`・`PostQuitMessage` は、どれも設計の使い方どおりの形で在る。`DPI_AWARENESS_CONTEXT` は `Copy`・`PartialEq` なので、偽の切り替えが受けた値を並べて残す D1 も書ける。`wintf::ensure_process_dpi_awareness` は `pub use runtime::*` で外から呼べる。
- **前例との形**: `os_window_rect` の「v2 にする → 前の値が無効なら debug で今のまま → 有効なら戻す」を `with_thread_per_monitor` が写し、水準だけ warn にする。切り替えを引数で受けるので、本物では起きない「替えられなかった」道を D2 で踏める。
- **最後のエラーを戻す前に読む**: 今の 3 つの口は `MessageBoxW` の直後に `Error::from_thread()` を読む。設計は戻す呼び出しの前に読んで `BoxOutcome` で返すので、`*_box_failed` の欄 `error` の中身は今と変わらない（2.2）。
- **関門 0 の測り**: bin のユニットテストの exe にも comctl32 の版 6 の申告が入る（`build.rs` の注釈どおり `rustc-link-arg-bins` が bin のユニットテストにも当たる）ので、本番と同じ部品で測れる。持ち主の窓の無いスレッドのタイマー（`SetTimer(None, …)`）はメッセージボックスのモーダルの間も同じスレッドで配られ、`EnumThreadWindows` で `#32770` を探して `WM_COMMAND`／`IDOK` で閉じる形は筋が通る。見つからないときに `PostQuitMessage` で抜ける道も、場合ごとに新しいスレッドなので後の場合へ `WM_QUIT` が残らない。
- **ほかの窓への響き（3.3）**: 窓のアプリの後はメインのスレッドがもともとプロセスの既定（v2）を受けているので、囲んでも扱いは変わらない。モーダルの間にゴーストの窓へ配られるメッセージも、OS が窓の扱いへ合わせて配る。窓のアプリの前と `install` のスレッドには、ほかの窓が無い。

## 大事な点（3 つまで）

### 1. 実装の後の実機の確かめが、直した道を通らないことがある

- **何が気になるか**: 実装の後に出す画面は「起動の前のインストールの利用条件の画面」だけ。この画面の前に `BootPorts::before_dialog` がプロセスを v2 に立てるので、スレッドは囲む前から v2 を受けている。関門 0 の `process_v2_default` が 2（＝10-31 の値 1 は測り違い）なら、この画面は直さなくても初めからにじまない。そうなると、直しで描き方が変わる唯一の道（プロセスがまだ非対応のまま出る、窓のアプリの前の画面＝根が無い・ゴーストが無い・起動の時点で記憶を書けない）は、副のモニタで実際に見ないまま終わる。関門 0 の `wrapped_from_unaware` は窓の扱いの値を読むだけで、にじみは見ない。
- **なぜ困るか**: 要件 4.2 の「にじまないことを実物で見る」が、直しの効かない画面で済んでしまうおそれがある（1.2・1.3 の副のモニタの側）。
- **案**: 関門 0 の結果で出す画面を決めると設計に書く。`process_v2_default` が 1 なら今の設計どおり利用条件の画面だけ。2 なら、同じ机の番のうちに、窓のアプリの前の画面を 1 つ（例: `target\` の下の空のフォルダを `AREKA_ROOT` に渡した根が無い画面。`AREKA_APP_SMOKE_EXIT_MS` は付けない）も副のモニタに出し、窓の DPI とにじみを見る。4.5 は今どおり利用条件の画面の後で見る。
- **要件**: 1.2・1.3・4.2・4.4
- **設計の場所**: 「実装の後の実機の確かめ（4.2〜4.5）」の「出す画面」と「4.4 について」

### 2. `process_v2_default` を最後に置くと、10-31 と同じ形にならないおそれがある

- **何が気になるか**: この場合は、同じプロセスで先に 3 つの場合がメッセージボックスを作り、`GetDpiForSystem` などを読んだ後に `ensure_process_dpi_awareness()` を呼ぶ。research.md も引いているとおり、`SetProcessDpiAwarenessContext` は UI を作る前・DPI の扱いに頼る API より前に呼ぶ決まりなので、この位置では失敗するか（`wintf` は warn を出すだけで続く）、10-31（UI を作る前にプロセスを立てた）と同じ状態にならないおそれがある。設計の見分け方は「2 なら測り違い・1 なら囲めば直る形」の 2 通りだけで、「プロセスが v2 にならなかった」の枝が無い。
- **なぜ困るか**: 関門の判定には使わないが、この値で 10-31 の件を見分け、さらに大事な点 1 の画面の選び方もこの値に頼る。見分けを取り違えると、要るはずの確かめを落とす。
- **案**: `process_v2_default` を別の `#[ignore]` のテスト（例 `probe_alert_box_window_dpi_process_v2`）に分け、別の `cargo test` の 1 回で、UI を作る前の新しいプロセスの最初に呼ぶ。あわせて、その行に `GetDpiAwarenessContextForProcess` で読んだプロセスの扱いを足し、`thread_before=2` でなければその行を見分けに使わない（測りの不具合として走らせ直す）と判定に書く。
- **要件**: 4.6（と 10-31 の見分け）
- **設計の場所**: 「関門 0」の「測る場合」の表と、その下の注、「出すもの・判定」の最後の段

### 3. ソースの字面を見る D5・D6 を、整形と注釈に強い形で決めておく

- **何が気になるか**: D5 は「`//` で始まる行を除いて `MessageBoxW(` が 1 回・`show_box(` が 4 回」、D6 は「各口の `show_box` の呼び出しが渡す名」を字面で見る。rustfmt は `show_box(` の引数を次の行へ折るので、`show_box("alert"` の形では一致しない。字下げのある注釈行や行末の注釈の扱いも決まっていない。あわせて、偽の切り替えが「有効な前の扱い」として返す値に `DPI_AWARENESS_CONTEXT_UNAWARE` を使うと、`windows` 0.62 の `is_invalid()` は値 -1 を無効と読むので、D1・D3 が別の道を踏む（設計の D1 は `SYSTEM_AWARE`＝-2 なので今は当たらない）。
- **なぜ困るか**: 実装の係が字面の合わせ方で手戻りし、整形だけでテストが赤になる。偽の値の取り違えは、テストが緑のまま別の枝を固定する。
- **案**: D5・D6 の読み方を「行の頭の空白を除いてから `//` を見る・空白と改行をすべて除いてから `show_box("alert"` などを数える・関数の本文は `fn raise(` から次の `fn ` まで」と設計に書く。D1・D3 の偽の前の扱いは `SYSTEM_AWARE` に限り、`UNAWARE`（-1）と null を使わないと注に書く。
- **要件**: 1.4・1.5・3.1・3.2・4.1
- **設計の場所**: 「alert_dpi_tests.rs（兄弟のテスト）」の D1〜D6 の表

## よいところ

- **関門 0 を実装の最初に置き、3 つの口へ繋ぐ前に止まれる作り**: 判定が 1 行の ASCII と assert で機械的に決まり、効かなければ本番の振る舞いを変えないまま測った 4 行を添えて戻せる（4.6）。10-31 の値の出どころの疑い（別のプロセスから読んだ）を、areka の中から読む形で除いている。
- **小さく閉じた変更**: 触るのは `alert.rs` と兄弟のテストのファイルだけで、`Cargo.toml`・マニフェスト・`build.rs`・呼び手・`wintf` に触らない。切り替えを引数で受ける形で、画面を出さない決定論のテストが囲む順・戻し・warn の 1 件をすべて押さえる。

## 判定

- **GO**
- **理由**: 既にある形（`os_window_rect`）に沿い、頼る API と機能は在ることを確かめた。関門 0 の判定の場合（`wrapped_from_unaware`）は正しく作られている。大事な点 1〜3 は、確かめの手順とテストの書き方の詰めで、設計の骨は変わらない。
- **次にすること**: 設計の討議（`/kiro-design-discussion alert-dialog-per-monitor-dpi`）で大事な点 1〜3 を議題にし、決まった分を design.md に足す。そのあと `/kiro-spec-tasks alert-dialog-per-monitor-dpi`。
