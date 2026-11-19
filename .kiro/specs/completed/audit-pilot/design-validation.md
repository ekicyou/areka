# 設計の検証: audit-pilot

- 日付: 2026-11-17
- 対象: `design.md`（605 行）・`requirements.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`two-tunnel.md`・`roadmap.md` の C22 の行）
- やり方: `kiro-validate-design`（`rules/design-review.md` の 4 段＝分析 → 大事な問題 → 良い点 → GO／NO-GO）。対話なしで判定まで出した。設計の主張のうち効くものは `crates/pilot/` の今のコードで確かめた。

## 1. まとめ

設計は、ギャップ分析の 27 件の全部に「どこを・どう直すか」を 1 行ずつ割り当て、直したバグ・危ない所（1・2・3・4・6・18）には「壊れたら落ちる」テストを名前つきで置いている。先進坑の決まり（葉の隔離・空の `lib.rs`）とウェーブ C22 の約束（`main.rs` を割らない・番人の表は申し合わせの後・依存を足さない）にも沿う。大きな作りの食い違いは無い。残る問題は、要件の文面との 1 か所のずれと、タスクの見取り図の順番の 2 か所で、どれも設計の討議とタスク生成で直せる。

## 2. コードで確かめたこと

| 設計の主張 | 確かめた結果 |
|---|---|
| `pilot-clickthrough-alpha-toggle/main.rs` は 1,006 行で、直しの後は約 990 行（1,000 行を下回る） | 正しい。今 1,006 行。15 は `applied_done` の引数（697）・確かめ（711-713）・`applier_done` の 2 行（876・881）・後片付けの 7 行（898-904）と説明の 3〜4 行で約 −15〜16、16 は内外の判定 4 行（290-293）が 1 行になり −3（ログが使う `ccx`／`ccy` の 2 行は残る）、27 は `allow` の 1 行、1 は +2〜3、テストの接続宣言 +3。合わせて約 989〜993 行。15 が −10 行を越えれば必ず 1,000 行を下回るので、結論は揺れない |
| 消す名前（`MsgTag::Load`・`PeerGone`・`HelperState`・`module_loaded`・`PAYLOAD_HEADER_LEN`・`poll_exit`・`wait_clean`・`encode_hwnd_le`／`decode_hwnd_le`・`GHOSTDIR`／`PARENT_HWND` ほか）は範囲の外に読み手が無い | 正しい。`git grep` で範囲の外に当たるのは、本坑の `crates/shiori-host32-*`・`areka-ghost`・`areka-kanade` が持つ同じ名前の別物だけ（`crates/pilot` を取り込んでいない）。名指しの注記（`real_helper_test.rs:43`・`emo-present.rs:152`・`win_style.rs:45`・番人 2 本）はファイルを消さず名前も変えないので壊れない |
| `HelperHandle` に `Drop` を足しても今の待ち方と両立する | 正しい。`wait_kind` は `handle.child.wait()` を借りで呼ぶだけで、欄を外へ動かさない（`Drop` を持つ型から欄を動かす E0509 にならない）。テストの `HelperHandle { child, .. }` の組み立ても通る |
| 待ちを `SetTimer` にできる | 正しい。`MessageLoop::run_loop` は `GetMessageW(None, 0, 0)` で全部のメッセージを受け、目覚ましの印でないものをフィルタへ渡す（`wintf-winmsg-executor/src/lib.rs:182-205`）。UNLOAD・HELLO は別プロセスからの `SendMessageTimeoutW`＝`GetMessageW` の中で窓の手続きへ配られてフィルタに来ないので、50 ms の `WM_TIMER` がフィルタを起こす今の起こし役の代わりになる |
| `WM_CAPTURECHANGED` が閉包に届き、1 のテストが見えない窓で書ける | 正しい。`make_window(Rc<AppState>)` は `ShowWindow` を呼ばず、`AppState::new(state_changed, dragging)` を外から渡せる。`SendMessageW` で直に腕を通せる |
| 使う Win32 の関数は今の features で足りる | 正しい。根の `Cargo.toml` に `Win32_System_Threading`・`Win32_UI_WindowsAndMessaging`・`Win32_System_LibraryLoader` があり、`pilot` の 4 つの追加と合わせて足りる |
| `[[example]]` を `name` と `test = true` だけで書ける | 正しい。cargo は `path` が無いとき `examples/<name>/main.rs` を推し量る。`shiori-host-32-helper` は今の項目に `test = true` を足すだけ |
| 全体テストに乗るテストが画面・GPU・検体を使わない | 正しい。`capture.rs`・`swap.rs`・`main.rs`（バルーン差し替え）・`dropfiles.rs` のテストは純粋な関数だけ。`wintf-winmsg-executor/main.rs` はテストを持たない |
| 番人の表の 1 件を外せる時期 | `audit-test-kits` は 2026-11-17 の `origin/main`（`5749a4c7`）にまだ着地していない。要件 7.3 のとおり、相手の着地を待つことになる |

## 3. 大事な問題（3 件）

🔴 **問題 1: 要件 4.6 の「ファイルは足さない」と、設計が足すテストのファイル 3 本が文面で食い違う**
- **何か**: 要件 4.6（討議の決め 2）は「新しい依存とファイルは足さない」と書く。設計は `main_drag_tests.rs`・`helper_window_tests.rs`・`parent_window_tests.rs` の 3 本を足す。設計はこれを「配線についての言葉」と読み、OPEN QUESTIONS の 6 に仮定として置いた。
- **困ること**: 実装の後の検証（`/kiro-validate-impl`）やかけ直しが要件の文面どおりに読むと、足した 3 本を要件違反と数える。足さない形に戻すと、`main.rs` は 1,000 行を越え直し（要件 6.1）、`helper_window.rs` の新しいテストは本番のファイルに本体を書くことになって `structure.md`「Unit Tests」（新しいテストのモジュールは兄弟のファイル）に反する。
- **直し方**: 設計の討議で仮定 6 を確定し、要件 4.6 の文を「全体テストへ乗せる配線のために新しい依存・道具の段・ファイルは足さない（要件 6.1 と `structure.md` が求めるテストの兄弟のファイルは別）」に直す。設計の Non-Goals にも同じ 1 行を置く。
- **要件**: 4.6・6.1・4.3
- **設計の箇所**: 「Boundary Commitments／This Spec Owns」のテストのファイル 3 本・「ClickthroughDrag」の Implementation Notes・`research.md` 15.5 の 6

🔴 **問題 2: タスクの見取り図で、消す名前の読み手が後のタスクに割り振られていて、途中のタスクの終わりで組み立てが通らない**
- **何か**: 見取り図は「どのタスクも終わった時点で `cargo test -p pilot --examples --no-run` が通る・名前を消すタスクは呼び元の行も同じタスクで直す」と決める。ところが ⑴ タスク 2 は「11」（`DEFAULT_TIMEOUT` を含む）と「10（ipc の分）」を持つのに、`ipc::DEFAULT_TIMEOUT` の読み手は `helper.rs:105`（タスク 8 の 11⑺）・`helper_window.rs:90`・`:226`（後継の `SEND_TIMEOUT` はタスク 7）・`main.rs:73`（タスク 6 の 11⑺）にあり、`helper.rs` はタスク 2 の触るファイルに無い。⑵ タスク 4 は `ipc::send_request` と `ResponseSlot` を `parent_window.rs` へ移すが、err(1)（`main.rs:522`・`:530`）は親の窓を持たずに `send_request` を呼んでいる。err(1) を `send_copydata` へ替えるのはタスク 6 に置かれている。
- **困ること**: タスク 2・タスク 4 を書いてあるとおりに実装すると、そのタスクの終わりで組み立てが赤になる。実装係とレビュー係が、どちらのタスクの持ち物かで迷い、直しが行き来する。
- **直し方**: ⑴ `DEFAULT_TIMEOUT` を消すのはタスク 8（helper の最後の読み手 `helper.rs` を直すタスク）に移し、タスク 2 の「11」を「11 の ⑴〜⑹（`DEFAULT_TIMEOUT` と 11⑺ を除く）」と書く。⑵ err(1) を `send_copydata` へ替える直し（指摘 3 の err(1) の分）をタスク 4 へ移すか、タスク 4 の触るファイルの `main.rs` に「err(1) の送りを `send_copydata` へ替える」を明記する。タスク生成（`/kiro-spec-tasks`）はこの直した見取り図から作る。
- **要件**: 3.1・3.2
- **設計の箇所**: 「タスクの切り分けの見取り図（15 本）」の表のタスク 2・4・6・7・8、「指摘と直しの対応表」の 11

🔴 **問題 3: タスク 10 の後は、このワークツリーで番人のテストが分かっていて赤になるのに、その赤を実装とレビューの段にどう扱わせるかが決まっていない**
- **何か**: タスク 10 で `main.rs` が 1,000 行を下回ると、`file_length_guard_test.rs` の `every_over_limit_exception_is_still_over_the_limit` が赤になる。表の 1 件を外すのは `audit-test-kits` の着地の後（要件 7.3）で、その着地はまだ無い。設計は「外すまでの間は赤」と書くだけで、タスク 10 をいつ入れるか（見取り図では「1 の後」で前の方）と、その間のレビューで赤をどう読むかを決めていない。
- **困ること**: `/kiro-impl` の自動の流れでは、タスクごとのレビュー係や最後の検証が `log-capture-kit` の赤を「直しが壊した」と読み、差し戻しや調べ直しを重ねうる。赤の期間が長いほど、ほかのタスクの赤と見分けにくくなる。
- **直し方**: ⑴ タスク 10 の入れどきを、`shiori-host-32` の直し（2〜9）とバルーン差し替え・文書（12）の後、実機の確かめ（13）の前に下げ、赤の期間を短くする。⑵ タスク 10 の完了の条件に「この時点で番人の `every_over_limit_exception_is_still_over_the_limit` だけが赤になるのは正しい（要件 7.3 の待ち）。ほかの番人とテストは緑」と書き、レビュー係が見る赤の名前を 1 つに絞る。⑶ タスク 11 の止まり方に、`audit-test-kits` の着地を待つ間はタスク 14（かけ直し）を先に進めてよいことを書く。
- **要件**: 7.1・7.3・4.4
- **設計の箇所**: 「LengthGuardEntry」・「タスクの切り分けの見取り図」のタスク 10・11・15・「Open Questions / Risks」の 2 つ目

## 4. 良い点

- **27 件の全部に、今のコードで確かめた直しが付いている**: 「指摘と直しの対応表」が 1 件 1 行で直す所と直し方を決め、ギャップ分析が残した調べもの（実行例のテストが今も組めるか・`WM_CAPTURECHANGED` が届くか・`SetTimer` で起こせるか・helper を `test = true` にできるか）を、組み立ての実測とライブラリのコードの行番号つきで解いている。行番号がずれた指摘も名前で指し直しており、実装係が迷わない。
- **ふやさずに減らす形で、共有の決まりを 1 か所へ寄せている**: `ipc.rs` には親と helper の両方が使う物だけを残し、親だけの受け皿セルと 1 往復を `parent_window.rs` へ移すことで、6 つの `#![allow(dead_code)]` を消した後も警告を出さない形を、項目ごとの `allow` に逃げずに作っている。起こし役のスレッドを Windows の `SetTimer` に、子の後始末を std の `kill`／`wait` に任せ、新しい仕組みを足していない。

## 5. 判定

- **判定: GO**
- **理由**: 作りは今の先進坑の形のまま直すもので、steering とウェーブの約束に沿い、27 件の全部とテストの置き場が決まっている。3 件の問題は要件の文の直しとタスクの順番の直しで片付き、作りを組み直す必要は無い。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion audit-pilot`）で、上の 3 件と `research.md` 15.5 の仮定 1〜9 を確かめる。問題 1 は要件 4.6 の文の直し、問題 2・3 は「タスクの切り分けの見取り図」の直しになる。
  2. 討議で決まった形で `design.md` を直し、`/kiro-spec-tasks audit-pilot` でタスクを作る。
