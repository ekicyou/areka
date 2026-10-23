# Brief: talk-thread-count-growth

> 起票: 2026-10-19（`areka-P0-mcp-kanade-tools` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-mcp-kanade-tools/verification/signoff.md` の「気付いたこと（本 spec の範囲の外）」と、同 `tasks.md` の Implementation Notes の 6.3。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

- **見たもの**: 実機（emo2・debug ビルド）で、10 秒ごとのスレッド別 CPU の報告の `threads=` が、起動直後の 15 から終了前の 35 まで増えた。増えた分はすべて `sakura-talk-1`〜`sakura-talk-<番号>`（トーク 1 つにつき 1 本）。MCP を呼ぶ前に終わった起動の挨拶の `sakura-talk-1` も残っていたので、MCP の呼び出しとは関係のない、前からある問題。
- **起票の時点の見立て（未確定）**: この数は「生きているスレッドの数」ではなく、**スレッド名簿の項目の数**かもしれない。
  - 報告器 `perf_thread_report`（`crates/areka/src/perf_thread_report.rs`）の `threads=` は、名簿 `thread_registry::snapshot()`（`crates/wintf/src/ecs/world/thread_registry.rs`）の項目の数で、OS から数えた数ではない。
  - 名簿は、スレッドが走り始めに `register_current_thread` で自分のハンドルを複製して預ける作りで、**スレッドが終わった後も項目と複製したハンドルを持ち続ける**（モジュールの説明に「そのスレッドが終了した後も CPU 時間を読み出せる」とある・項目を消すのは同じ TID が登録し直したときだけ）。
  - トークのスレッドは、自然に終わると `settle_after_tick` が、中断されると `on_close` が `ControlFlow::Break` を返し、`run_inbox` が抜けて終わる作り（`crates/areka-sakura/src/drive.rs`）。dispatcher は自然終端の `on_done` で `TalkHandle` を join せずに落とす（`crates/areka-ghost/src/dispatcher.rs`・切り離し＝スレッドは自分で終わる）。
- **どちらでも直す価値がある**: 生きたスレッドが残っているなら、トークのたびにスレッドが 1 本ずつ増える。名簿だけなら、トークのたびに名簿の項目と OS のハンドルが 1 つずつ増え続け、報告の数も実際と食い違う。マスコットは何日も起動したまま喋り続けるので、どちらも上限なく積み上がる。

## Current State

- トークのスレッドは `spawn_talk`（`crates/areka-sakura/src/drive.rs`）が `spawn_actor("sakura-talk-{talk_id}", …)` で 1 トークにつき 1 本起こし、`TalkHandle { inbox, actor }` を返す。
- 持ち主は dispatcher の `ActiveTalk`（`crates/areka-ghost/src/dispatcher.rs` の `handle: TalkHandle`・`on_start` で起こす）。差し替えと停止（`close_active_if_any`・`on_close`）は `Close` を送って join する。自然終端（`on_done`）は join せずに `active = None` で落とす。
- 走り始めのフック（`crates/areka/src/thread_roles.rs` の `on_actor_thread_start`）が、`spawn_actor` で起きた全スレッドを名簿へ `actor:<name>` で載せる。
- 報告器は名簿の全項目の CPU 時間を読み、`threads=` に項目の数を出す。名簿に載らない分は「プロセス全体 − 名簿の合計」の差（`unregistered_rest`）として 1 行で出す約束。
- 生きたスレッドの数を OS から数えた記録は、まだ無い。

## Desired Outcome

- 長く起動していても、トークの数に比例してスレッド・名簿の項目・OS のハンドルが増えない。
- 報告の数が、実際に生きているスレッドと食い違わない（終わったスレッドを含めるなら、そう分かる形で出す）。
- 終わったスレッドの CPU 時間が報告から黙って消えない（役割ごとの合計へ畳むなど・形は要件で決める）。
- 決定論テストで固定し、実機で数が増えないことを 1 回確かめる。

## Approach

1. **最初のタスクは切り分け**: 実機で、トークを何度か流しながら、報告の `threads=` と、OS から見たプロセスのスレッドの数（PowerShell の `(Get-Process -Id <pid>).Threads.Count` など・areka の側に ToolHelp の機能は足さない）を並べて数える。
2. 名簿だけが増えているなら、名簿の側で直す（終わったスレッドの項目を、最後の CPU 時間を役割ごとの合計へ畳んでから外す、など）。生きたスレッドが残っているなら、トークのスレッドが抜けない理由（`Break` の後の後始末で止まっている所など）を突き止めて直す。両方なら両方。
3. 直した形を決定論テストで固定し、実機でもう一度数える。

## Scope

- **In**: 実機の切り分け・名簿の終わったスレッドの扱い・報告の数の意味・（残っているなら）トークのスレッドの後始末・決定論テスト・実機の確かめ 1 回。
- **Out**: 名簿の作りの置き換え（OS からの列挙へ替えること・`Cargo.toml` の機能を足すこと）・トークの再生の振る舞い（台本・刻み・中断の意味）・トーク以外のアクターの寿命の見直し（同じ形で増えるものが見つかったら記録して要件で決める）。

## Boundary Candidates

- 名簿（`crates/wintf/src/ecs/world/thread_registry.rs` と兄弟のテスト）と報告器（`crates/areka/src/perf_thread_report.rs`）。
- トークのスレッドの起こし方と終わり方（`crates/areka-sakura/src/drive.rs`）と持ち主（`crates/areka-ghost/src/dispatcher.rs`）＝生きたスレッドが残っていると分かったときだけ。

## Out of Boundary

- `areka-actor` の `spawn_actor`・`run_inbox` の約束（終わり方は `Break` と切断の 2 つ）。
- 走り始めのフックの型（`ThreadStartHook`・`actor-thread-log-capture` が扱う）。

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手できる）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし（名簿と報告器を作った `draw-load-parity` も、見つけた `mcp-kanade-tools` も完了済み＝新しい spec）。
- **Adjacent**:
  - `talk-anchor-interrupt-ticks`（据え置き）・`talk-fast-forward`（版に縛らない）: `dispatcher.rs`・`drive.rs` に触る。トークの側を直すことになったら同じウェーブに置かない。
  - `actor-thread-log-capture`（版に縛らない）: `spawn_actor` の走り始めの差し込み口と `thread_roles` を扱う。名簿の側だけなら重ならない見込み。
  - `instant-to-interrupt-ticks`（v0.2.0）: `perf_thread_report.rs` の時計を替える。同じウェーブに置かない。

## Constraints

- 段は**バグ**・区分 B・v0.2.0（致命的でない新しいバグは次の版へ。今の版は v0.1.0）。
- **触らない約束**: 起票の時点で走っているウェーブ C9 の spec は、名簿・報告器・`drive.rs`・`dispatcher.rs` のどれも触る予定に挙げていない（`dispatcher.rs` は `mcp-kanade-tools` が自分の触らない一覧に挙げていたファイル）。着手時に、走っている spec の触るファイルともう一度照らす。
- 実機の確かめは、負荷テストの机と画面の操作をタスクの区切りの中だけで持つ。実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ。止めてよいプロセスは、自分が起こしたと確かめられるものだけ。
- ログの無い失敗の道を作らない（名簿から外すときも、読めなかった CPU 時間は今と同じく記録に残す）。
- 依存は足さない。
- 規模の見込み: S〜M（5〜10。切り分け 1〜2・名簿と報告器 2〜4・トークの側〔残っていたときだけ〕2〜3・実機 1）。
- 要件定義のモデル: Opus（切り分けの結果で直す所が決まるだけで、新しい並行の設計は要らない見込み）。

## 優先度の区分

B（バグ。長く起動したまま喋り続けると、トークの数だけスレッドか名簿の項目と OS のハンドルが積み上がり、報告の数も実際と食い違う）。
