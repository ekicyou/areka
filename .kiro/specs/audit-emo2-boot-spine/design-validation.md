# 設計の確かめ: audit-emo2-boot-spine

- 日付: 2026-11-17
- 対象: `design.md`（102 件の直し方・束 T1〜T20）・`requirements.md`・`research.md`・steering
- 作法: `kiro-validate-design`（分析 → 重大な問題 → よい所 → GO／NO-GO）。対話なしで判定まで出した。

## まとめ

設計は 102 件の指摘をすべて番号で受け、直し方・縛るテスト・束を 1 行ずつ決めている。直さない 2 件（25・81 番）と預かった件 1〜8 の扱いは、要件の討議の裁定と合っている。C22 の約束（`frame/`・`frame*.rs`・`areka-placement`・`file_length_guard_test.rs`・emo-text の新しいファイル・新しい依存）にも触れていない。重いところの主張をコードで確かめたところ、実装を止める食い違いは無かった。ただし、3 番の呼び手の数と、仕上げで差分を確かめる条件に、直しておくべき食い違いが 2 つある。

## コードで確かめた主張

| 主張 | 結果 | 根拠 |
|---|---|---|
| 1 番: 上限は入口の `scope_of` 1 か所で足りる | 合う | 台本の道で `scope_of` を呼ぶのは `adapter.rs` だけ（L54〜L99・L221）。範囲の外の MCP は `dump_surface_judge.rs:L77` で既に `i32::try_from` で切っており、上限 `i32::MAX` と同じ |
| 2 番: 門を開けるのは `TalkEnded` を見た時 | 合う | `drive_talk` は、汲み終えた後に `done` を 1 反復に 1 回だけ呼ぶ。門が閉じている間は目印の面が汲まれないので、直す前は毎回赤になる。直した後は、次の反復から目印が届く。`TalkEnded` は seriko を通らずに `lifecycle_rx` へ届く（`talk_lifecycle.rs`）。seriko の受け口は、切れ目のない mpsc で送るだけ |
| 3 番: shiori のスレッドが panic すると `shutdown` が `Err` を返す | 合う | `GhostRuntime::shutdown` は join の `Err` を集め、1 つでもあれば `Err` を返す（`areka-ghost/src/runtime.rs:L327〜L332`）。kanade は応答の oneshot が切れると `Failed(Ipc)` として扱うので、止まったままにならない（`areka-kanade/src/actor.rs` の頭の説明）。`run_bounded_watching` は、相手が panic して居なくなると `［相手が居ない］` で落ちる（`spine_wait.rs:L181`・`L331〜L343`）。`GpuPermit` は毒された錠も受け取る（`spine_wait.rs:L625〜L626`）ので、`should_panic` で落としても、後に続く GPU のテストは止まらない |
| 4 番: 呼び手は 23 ファイル | 合う | `spawn_ui(`・`spawn_emo_text(`・`UiSpawnError` を grep すると 23 ファイル。内訳は設計の表と同じ |
| 48 番: `BootRuntimeFn` の読み手 | ほぼ合う | `RuntimeBootError` を名指すのは `session/runtime_boot.rs:L38・L115・L148` と `runtime_boot_tests.rs:L221` だけ。ただし、本番の `crates/areka/src/ghost_session.rs:L862` も `boot_runtime` を `wire_runtime` に渡している（コンパイルは通るので直しは要らない） |
| 30 番: `boot_finished`（basewareversion を待つ） | 合う | 起動の系列の最後は basewareversion（`spine_boot_smoke_tests.rs:L55〜L66`）。`spine_conformance_support_stage_tests.rs` の 2 本（L88・L260）は `OnBoot` が `Ok(None)` を返すので、`>= 6` だと永久に届かない |
| 6・21・66・97・102 番: 寄せる先の支え | 合う | `build_boot_assets` の呼び手は、全部本体のテスト（`test-support` の下）。`count_level`（`frame_test_support.rs:L130`）・`give_fake_handles`・`two_scope_placements`／`titles`（`spawn_test_support.rs`）・`code_only`・`listed_name` は今ある。`areka-runtime` のテストでも、プロセスの寿命の MTA が立つ（`crash_report_test_support.rs:L148〜L152`） |

## 重大な問題

🔴 **重大な問題 1: 3 番の呼び手の数と使い手が事実と違う**
**中身**: 設計は「使い手は spine 系のテスト 12 ファイルと `ghost_session_restart_tests.rs`（呼び出し 13 か所）」と書く（「見直しが要る変化」と 3 番の節）。今の `main` では、足場の `.shutdown_bounded()` の呼び出しは 11 ファイル・22 か所ある。`ghost_session_restart_tests.rs:L412〜L419` は自前の `shutdown_bounded` を持ち、すでに `.expect("降ろすのは成功する")` にしている。つまり足場の使い手ではなく、同じ直しの前例である。
**響き**: 完了の条件の「13 か所が全部緑」を、違う一覧で確かめることになる。そのうえ、`notify("OnClose")` を台本に持たない 5 つの台本（`spine_close_wiring_tests`・`spine_conformance_script`・`spine_conformance_support_stage_tests` の 2 本・`spine_third_scope_talk_tests`）は、降ろす前に kanade が自分で止まるのを待っているから緑で済んでいる。この前提が設計に書かれていない。書かないままだと、赤が出たときに、台本へ `OnClose` を足して黙らせる直しに流れやすい。
**直し方**: 3 番の節と「見直しが要る変化」の数を「11 ファイル・22 か所」に改める。`ghost_session_restart_tests.rs` は前例として挙げる。「`notify("OnClose")` を持たない台本は、kanade の自己停止を待ってから降ろす」を前提として 1 行足す。赤が出たら、それは降ろしの退行として扱う（台本を足して黙らせない）と書く。
**要件**: 4.1・4.2・4.7・6.3
**場所**: design.md「見直しが要る変化」・「3 番 足場の後片付けが降ろしの失敗で赤になる」

🔴 **重大な問題 2: 仕上げの差分の確かめが、設計自身が触るかもしれない範囲の外のファイルを許していない**
**中身**: T20 と「仕上げの確かめ」の 4 は、「範囲の外のファイルの差分が 2 項（4 番の呼び手・102 番の 1 ファイル）だけ」を合格の条件にしている。けれども設計自身が、次の 2 つを範囲の外で触るかもしれないとしている。
- 62 番の読み手 `crates/areka/src/session/ghost_switch_reload_tests.rs:L525`。持ち主は `audit-emo2-boot-switch` で、設計は「直しは要らない見込み・実装で確かめる」と書く。
- 未決 2 の別案。範囲の外の 4 ファイルの match の腕を消す。

102 番そのものも、未決 1 の答え次第で範囲の外の差分になったりならなかったりする。また 48 番の読み手の一覧（「`session/runtime_boot.rs`・`runtime_boot_tests.rs` だけ」）からは、本番で `boot_runtime` を渡す `ghost_session.rs:L862` が抜けている。
**響き**: 実装でどれかが要るとなったとき、要件 9.5 の確かめが落ちるか、確かめの条件を後から言い換えることになる。要件 7.2 の「要る変更だけ」も、記録で示せなくなる。
**直し方**: 範囲の外で触ってよいファイルを、未決 1・2 の答えと 62 番の実測で決まる「条件つきの一覧」にする。62 番で `ghost_switch_reload_tests.rs` に手が要るなら、持ち主が C22 で走っていない（要件 7.1）ことを記録して一覧に足す、と書く。48 番の「見直しが要る変化」に `ghost_session.rs:L862`（変えずに通ることを確かめる）を足す。
**要件**: 7.2・9.5（あわせて 3.2）
**場所**: design.md「見直しが要る変化」・「タスクの束と順」の T20・「仕上げの確かめ」の 4・「未決」1・2

## よい所

- 重い 3 件（1・2・3 番）は、どれも「直す前に赤・直した後に緑」までの道が具体的に書いてある。2 番は、`research.md` の「`drive_talk` が戻ってから門を開ける」案だと直した後に緑にならないことを見つけ、開ける時点を `TalkEnded` に替えている。確かめると、この組み方なら決定論で赤から緑へ移れる。
- 写しの寄せと置き場の移し替え（件 6・26・27・33・95・102 番）は、依存を足さないこと（要件 6.6）と、runtime-crate-split の設計 D4「テストの支えを 2 つの crate で写さない」を、両方とも同時に守る形を選んでいる。33 番では、`names_of` が `sample-ghost-kit` を使うので `test-support` の下には置けない、という理由まで押さえている。

## 判定

**GO**

- 理由: 102 件すべてに直し方と束が決まっており、重い主張はコードと合う。上の 2 つは数と確かめの条件の書き直しで済み、作りは変わらない。設計の討議で直してから、タスクの生成へ進めばよい。
- 次の段: 設計の討議（`kiro-design-discussion`）で、重大な問題 1・2 と、未決 1〜3 を議題にする → `/kiro-spec-tasks audit-emo2-boot-spine`。

## そのほかの気付き（議題にしなくてよい）

- 35 番: seriko のループを動かす（`LoopDriver::Live`）待ちでは、まばたきの指令が出続けるので、受けた数を目印にすると進みが止まらない。退行したときは、30 秒の `［止まった］` ではなく総時間（300 秒）で落ちる。これは議題 1 の裁定（`drive_talk` と同じ形）の範囲だが、タスクでは、各所の目印を「条件に向かう進み」に絞れるかを 1 行確かめるとよい。
- 「使ってよい依存」の `[dependencies]` の列挙から、`areka-placement` と `areka-sylphya` が抜けている（依存の向きの図には `areka-placement` がある）。足す・消すの話ではなく、書き漏れ。
