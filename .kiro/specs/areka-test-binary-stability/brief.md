# Brief: areka-test-binary-stability

> 2026-10-11 `/kiro-discovery` で起票（開発者の指示「バグの起票は粒度が小さい。修正範囲がかぶらない範囲でいくつかに集約」）。`areka` のテストの実行ファイル（`cargo test -p areka --bin areka`）のバグの起票 2 本を 1 本にまとめた。元の brief は下の「件 1〜2」に全文を写してある（見出しを 1 段下げただけ・中身は変えていない）。場所と行数は元の起票と棚卸㉓（main `ee3af616`）のときのもの＝着手時に引き直す。

## Problem

`areka` のテストの実行ファイルに、負荷の下で赤になる・落ちる原因が 2 件残っている。2 件は同じ実行ファイルを回し、足場のファイルも重なり、もともと「件 1 を先に・同時に走らせない」と決まっていた。別々の spec にすると、静かな机を 2 回借り、負荷つきの全体の走行（1 回 約 50 分）を 2 回回すことになる。

- **件 1（旧 `areka-test-threads-av`）**: テストのプロセスが `STATUS_ACCESS_VIOLATION`（`0xc0000005`）で落ちる。`--test-threads=4` で再現し、負荷の下では既定のスレッドの数でも 3 度落ちた。
- **件 2（旧 `test-wait-marker-gaps`）**: テストの待ちのうち、steering `tech.md` の「テストの待ちと、負荷の下で許す赤」に揃っていないものが 5 種類残っている。

## Current State

- 件 1 は原因が分かっていない（落ちる頃に走っていたのは `emo2_boot::frame` の族＝各スレッドで `CoInitializeEx(MULTITHREADED)` を呼び MTA の Compositor を作るテスト）。GPU の装置の許可（同時に 4 つまで）が入った後でも落ちる。
- 件 2 は数が確かめてある（目印の無い `spin_wait_until` の直接の呼び出し 15 か所・`join_bounded` の写し 3 つ・上限の無い `GpuSlots::take` ほか）。
- 重なる足場: `crates/areka/src/emo2_boot/spine_wait.rs`・`ghost_switch_test_support.rs`・`ghost_session_restart_tests.rs`・`shell_balloon_switch_session_lap_tests.rs`。
- 細部は各件の節。

## Desired Outcome

- 件 1: 落ちる組と原因が特定され、テストの土台か本番のコードが直っている。再現しなければ、条件と結果を記録して閉じる。
- 件 2: 5 種類の待ちが steering の線に揃っている（揃えられないものは、理由つきの例外として記録するか、開発者の裁定で許す赤の範囲を広げる）。
- 負荷つきの全体の走行（5 回）が、許さないと決めた赤もプロセスの落ちも無しで通る。**この最後の確かめは 2 件で 1 度だけ**。

## Approach

**件 1 → 件 2 の順**（件 1 の落ちが件 2 の確かめの回を無効にする。件 1 の直しが許可の数を変えるなら、件 2 の「許可の待ちの上限」はその後に決める）。

1. 件 1: 落ちた瞬間のスレッドの名前（＝テストの名前）と呼び出しの積み上げを 1 行残す仕掛けを先に入れ、`emo2_boot::frame` の族に絞って再現する。許可を 1 つにした対照を採る。
2. 件 2: 負荷なしで直せる項（`join_bounded`・`areka-nar` の短い掴みの檻・目印の無い `spin_wait_until`）から直し、打ち切りの文言は時計を注入した檻で見る。
3. 最後に、負荷つきの全体を 1 度だけ回して 2 件をまとめて確かめる。途中は赤が出たテストに絞って短く回す。

## Scope

- **In**: 再現の手順・原因の特定・テストの土台か本番のコードの修正（件 1）／5 種類の待ちと、そのための足場・偽の SHIORI の観測口・檻（件 2）／負荷の再現の記録。
- **Out**: 本番のコードの振る舞いのうち件 1 の原因でないもの（`rename_patiently` の試し直しの間隔と上限を含む）・許す赤の線そのものの書き換え（件 2 の項目 2 の裁定で広げる場合を除く）・`tools/test-all.ps1` の並列度・wintf の graphics の一度だけの AV（覚え書きのまま）・テストの一時フォルダの置き場（`test-roots-under-target`）。

## Boundary Candidates

- 件 1: テストの土台（スレッドに縛られる資源の扱い・GPU の装置の許可）／本番側の欠陥（見つかった場合だけ＝`crates/wintf/src/ecs/graphics/`）。
- 件 2: 待ちの部品（`spine_wait.rs`）と足場／spine の族のテスト／`install/` の 2 本／`areka-nar` の `install_commit_tests.rs`。

## Out of Boundary

- 本番のファイルの振る舞いの変更（件 1 で本番の欠陥が見つかった場合と、`#[cfg(test)]` の部分を除く）。
- 許す赤の線の理由の書き換え。

## Upstream / Downstream

- **Upstream**: `ghost-session-test-load-flake`（✅ 10-10・待ちの部品と許す赤の線・GPU の装置の許可）。
- **Downstream**: 実装の手順（`/kiro-impl` の査読）で負荷を下げる手が使えるようになる。以後のテストは steering の線に従って書く。

## Existing Spec Touchpoints

- **Extends**: 完了 `ghost-session-test-load-flake`（待ちの部品・steering `tech.md` の線）。
- **Adjacent**: `test-roots-under-target`（同じテストを書き換える・一時フォルダの置き場が動くと比べる基準が変わる＝同じウェーブに置かない）・`actor-thread-log-capture`（背景のスレッドの記録が赤の文言に出れば読み分けが楽になる）・`emo2-real-run-stability`（実機の窓が出る検査＝負荷の走行と同じ時間に回さない・壁時計の待ちを観測の待ちへ置き換える方針は同じ）・`install-live-target-hazards`（`session_end_deadline_tests.rs`）・`mcp/dump_surface.rs` を触る MCP の spec。

## Constraints

- 段は**バグ**・区分 B。**測ることが本体＝ウェーブに並べず、静かな机で 1 本だけ**。
- 規模の見立て: 13〜20 タスク（件 1＝5〜9・件 2＝8〜11）。最後の負荷の確かめを 1 度にまとめる分は減る。**上限 20 を超える見込みになったら、要件の段で件 2 を切り出す**（件 1 の原因が本番のコードに在ったとき）。
- 高負荷のテストを何度も回さない（全体は記録の最初と最後だけ・2 回目からは赤のテストに絞る・回す前に所要時間を伝える）。
- GPU のテストは実の GPU のまま（WARP などへ替えない）。sleep を足す・締切を延ばすだけ・1 フレーム遅らせる形では直さない。
- 1 ファイル 1,000 行の番人（`spine_conformance_support_tests.rs` 959 行・`assets_tests.rs` 979 行は先に分ける）。
- 要件定義のモデル: Fable（スレッドの競り合い・件 2 の項目 2 に開発者の裁定の分かれ目）。

---

## 件 1: 旧 `areka-P0-areka-test-threads-av`（元の brief の全文）

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-mouse-drag-events` のタスク 2.2 の検証と完了時の棚卸（`.kiro/specs/completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes の「範囲外」）。

### Problem

- **開発者・実装の手順**: `cargo test -p areka --bin areka -- --test-threads=4` が `STATUS_ACCESS_VIOLATION`（不正なメモリアクセス）でテストのプロセスごと落ちる。どのテストで落ちたかが分からず、スレッド数を絞って負荷を下げる手が使えない。メモリの壊れ方が本番の欠陥の兆しかもしれない。

### Current State

- 2026-10-04〜05、`mouse-drag-events` の 2.2 の検証で観測した。新しく足した drag のテストを外しても落ちる（本 spec の前からある）。既定のスレッド数では落ちない（`tools/test-all.ps1` の全体も緑）。
- roadmap の覚え書き「一度だけ落ちた試験」にある `wintf --test graphics` の `STATUS_ACCESS_VIOLATION`（10-04・`mcp-expression-table`）とは別の入口（こちらは `areka` の bin のテストで、条件を揃えると再現する）。同じ根かは分からない。
- 疑い: スレッドを跨いで共有してはいけない Windows の資源（COM の部屋・窓・GPU の資源）を、テストの並びしだいで別スレッドから触っている。

### Desired Outcome

- 落ちるテスト（または組）を特定する手順が 1 つあり、記録に残っている。
- 原因がテストの土台（スレッドに縛られる資源を共有している）なら、土台を直すか、そのテストの族をスレッド 1 本で回す印を付ける。本番のコードの欠陥なら、それを直すテストを添えて直す。
- 再現しなければ、試した条件を記録して据え置きへ移す。

### Approach

`--test-threads=4` で回すテストの集合を二分探索で絞り（モジュールの単位で `cargo test -p areka --bin areka <モジュール> -- --test-threads=4`）、落ちる組を特定する。WinDbg などで落ちた場所の積み上げを採れるなら採る。

### Scope

- **In**: 再現の手順・原因の特定・テストの土台か本番のコードの修正・記録。
- **Out**: `ghost-session-test-load-flake`（壁時計の締切で負荷のときに赤になる族）・`wintf` の graphics の一度だけの AV（覚え書きのまま）。

### Boundary Candidates

- テストの土台（スレッドに縛られる資源の扱い）
- 本番側の欠陥（見つかった場合だけ）

### Out of Boundary

- `tools/test-all.ps1` の並列度の変更。

### Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: 実装の手順（`/kiro-impl` のレビュー）で負荷を下げる手が使えるようになる。

### Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `ghost-session-test-load-flake`（同じ `areka` のテストの族を触るなら同時に走らせない）。

### Constraints

- 段は**バグ**（テストのプロセスが落ちる・原因不明のメモリの壊れ）。

### 2026-10-10 `ghost-session-test-load-flake` からの申し送り（既定のスレッドの数でも落ちた）

- 負荷の再現（論理 CPU 22・負荷の子 44 本・`--test-threads` は渡さない＝既定の数）の下で、`cargo test -p areka --bin areka` が `0xc0000005`（`STATUS_ACCESS_VIOLATION`）で 3 度落ちた。panic も「60 秒を越えた」の行も無く、どのテストかは出ない。
  - `load-repro.md` の 4.7 の 4 回目（94 秒・ok 522 本・`frame::transition_branch_tests` の後）
  - 4.8 の 3 回目（18.7 秒・`frame::text_scale_tests` の後）
  - 4.9 の 5 回目（65.9 秒）
- 落ちる頃に走っていたのは、名前の順で `frame::visibility_integration_tests`（各スレッドで `CoInitializeEx(MULTITHREADED)` を呼び、`GraphicsCore::new` から MTA の Compositor を作る）の先頭の十数本。上の「`--test-threads=4` でだけ再現」の前提は崩れた＝既定のスレッドの数でも、負荷でスレッドの重なりが増えると起きる。
- 記録の置き場: `.kiro/specs/completed/areka-P0-ghost-session-test-load-flake/load-repro.md` の 4.7〜4.9（`target\load-flake\after-20261009-*\round-N.log`）。

---

## 件 2: 旧 `areka-P0-test-wait-marker-gaps`（元の brief の全文）

> 起票: 2026-10-10（`areka-P0-ghost-session-test-load-flake` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票。`completed/areka-P0-ghost-session-test-load-flake/tasks.md` の Implementation Notes の「範囲外の申し送り」⑷⑹⑺・`load-repro.md` の 4.8 と、完了時の棚卸で見つけた 2 件）。コードの読みで確かめた。

### Problem

`ghost-session-test-load-flake` は、テストの待ちを待ちの部品（`crates/areka/src/emo2_boot/spine_wait.rs` の `wait_until` ほか）へ集め、負荷の下で許す赤の線を steering `tech.md` の「テストの待ちと、負荷の下で許す赤」に書いた。その線では、負荷の下でも待ちの文言の無い赤・`［進みは不明］`・`［進んではいた］` は許さない。ところが、まだ部品の形に揃っていない待ちが残っていて、負荷の下でそれらの赤が出うる、または許す赤と欠陥を見分けられない。

### Current State

2026-10-10 の本枝（`ghost-session-test-load-flake` の完了の直前）で数えた。

1. **目印が UI の側の仕事を数えない待ち**（申し送り ⑷）: `shell_balloon_switch_session_abort_tests.rs` の `ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` の 2 つの待ち（`lap.frames_until(CREEP, …)` と、捕捉の中の `lap.frames_until(UNBOUNDED, …)`）。目印は足場の `SwitchRig::progress_probe`（ゴーストを作った数と SHIORI の呼び出しの数）で、台詞の表示・B の定常・取りやめの記録を数えない。そのため `［止まった］` が実質 30 秒の壁時計の打ち切りになり、負荷と欠陥を見分けられない（`load-repro.md` の 4.8）。同じ足場の段の駆動器には、台詞の起動と表示の数を目印に足した前例がある（`spine_conformance_lap_tests.rs` の `progress_probe`）。
2. **`join_bounded` の目印が条件と同時にしか動かない**（申し送り ⑹）: spine の `join_bounded`（`spine_wait.rs`）と、その写しの `install/desk_pick_tests.rs`・`install/fetch_url_tests.rs` の `join_bounded`。目印は `is_finished`（0→1）で、条件と同じ時にしか増えないので、打ち切りは実質 30 秒の総時間で、文言は進み 0 回の `［止まった］` になる。`desk_pick`・`fetch_url` は素の World のテストで、ゴーストの足場でも GPU の足場でもない。負荷の下でここが赤になると、要件 6.1 の許す赤の範囲の字面に当たらない。写しが 3 つある（spine のは `ActorHandle`、install の 2 つは `JoinHandle<()>` を受ける）。
3. **`areka-nar` の短い掴みの檻**（申し送り ⑺）: `install_commit_tests.rs` の `a_destination_held_briefly_by_another_process_still_commits`。別のスレッドが約 200 ms 眠ってから掴みを放し、本番の `rename_patiently`（`install.rs`・os error 5・32 だけを 50 ms 刻みで約 2 秒まで試し直す）が待ち切ることを見る。負荷の下で放す側のスレッドの起動が大きく遅れると、2 秒を越えて揺れうる（`areka-nar` の実行ファイルには GPU のドライバの DLL が載らないので見込みは低い）。
4. **GPU の許可の待ちに上限が無い**: `GpuPermit::take`（`spine_wait.rs` の `GpuSlots::take`）は `Condvar::wait_while` で空きが出るまで眠り、打ち切りが無い。要件 3.3（どの待ちにも打ち切りの上限を持つ）の字面から外れている。許可を 2 つ取るテストが並ぶと互いに待ち合って止まりうることは、`GpuPermit` の説明に書いてある。
5. **目印の無い `spin_wait_until` の直接の呼び出し 15 か所**（`git grep -n "spin_wait_until(" -- crates/areka/src` から定義の 1 行を除いた数）。打ち切ると `［進みは不明］` になり、負荷の下では許さない赤になる。
   - spine の族 9 か所: `emo2_boot/spine_boot_smoke_tests.rs` 2・`spine_close_wiring_tests.rs` 1・`spine_conformance_support_tests.rs` 4・`spine_hold_tests.rs` 1・`spine_seriko_loop_tests.rs` 1
   - `session_end_deadline_tests.rs` 4（偽の SHIORI の `holding()`・`unblock_calls()` を待つ）
   - `mcp/dump_surface.rs` 1（`#[cfg(test)]` の `WaitAnswer::wait_answer`）
   - `ghost_session_restart_tests.rs` 1（`run_input_until` の中・作業プールの進みを数える口がテストの側に無いと説明にある）
   - あわせて見る所: 段の駆動器の目印 `StageSink::progress_probe`（`spine_conformance_support.rs`）は既定が `None`（`［進みは不明］`）で、上書きしているのは lap の足場だけ。`SpineHarness` で段を回す `spine_conformance_support_tests.rs` の檻は目印なしで待つ。

### Desired Outcome

- 上の待ちがすべて、steering `tech.md` の線のとおり、負荷の下で出る赤が許す赤（`［止まった］`）か、待ちの文言で欠陥と読み分けられる形になる。`［進みは不明］` を出しうる待ちが `crates/areka/src` のテストに残らない（残すなら理由を記録した例外にする）。
- 2 の `desk_pick`・`fetch_url` は、目印を良くするか、開発者の裁定で許す赤の範囲を広げるかのどちらかで決着する（議題）。
- 3 の檻が負荷で揺れない形になる（本番の `rename_patiently` の振る舞いは変えない）。
- 4 の待ちに上限が付き、打ち切りは文言で分かる。
- 静かな机と `tools/test-all.ps1` は今までどおり緑。所要時間を大きく延ばさない。

### Approach

- 1 は足場の目印に UI の側の仕事（台詞の起動・表示・定常の知らせなど）を足す。lap の段の駆動器の前例に合わせる。`LapRig::frames_until` はほかの lap のテストも使うので、目印を変える影響を全部の呼び手で見る。
- 2 は、`join_bounded` が待つ相手の進み（スレッドの中の仕事の数など）を目印にできるかを先に調べ、できなければ開発者に範囲の広げ方を諮る。写し 3 つを 1 つにまとめるかもここで決める。
- 3 は、放す時機を時刻でなく「1 度拒まれた」ことの観測に結ぶなど、待ちに頼らない形を探す。本番の試し直しの間隔と上限は変えない。
- 4 は、許可の待ちを待ちの部品へ載せるか、`wait_timeout_while` で上限と文言を付ける。
- 5 は、相手の進みを数える口が在る所（偽の SHIORI の呼び出しの数など）から目印つきの部品へ移す。口の無い所は、口を足すか、理由つきの例外として記録する。
- 確かめは `tools/load-flake.ps1` の負荷の再現で行う。全体の回は記録の最初と最後だけにし、2 回目からは赤が出たテストに絞って短く回す。回す前に所要時間を伝え、静かな机で回す。

### Scope

- **In**: 上の 1〜5 の待ちと、そのための足場・偽の SHIORI の観測口の追加・檻・負荷の再現の記録。
- **Out**: 本番のコードの振る舞い（`rename_patiently`・`GpuPermit` の数を含む）・許す赤の線そのものの書き換え（2 の裁定で広げる場合を除く）・テストの確かめの中身。

### Boundary Candidates

- 待ちの部品（`spine_wait.rs`）と足場（`ghost_switch_test_support.rs`・lap の足場）
- spine の族のテストファイル・`session_end_deadline_tests.rs`・`mcp/dump_surface.rs` のテストの部分・`ghost_session_restart_tests.rs`
- `install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`
- `areka-nar` の `install_commit_tests.rs`

### Out of Boundary

- 本番のファイル（`#[cfg(test)]` の部分を除く）
- 許す赤の線の理由の書き換え

### Upstream / Downstream

- **Upstream**: `ghost-session-test-load-flake`（✅ 2026-10-10 完了・待ちの部品と許す赤の線）。
- **Downstream**: なし（以後のテストは steering の線に従って書く）。

### Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-ghost-session-test-load-flake`（待ちの部品・要件 3.3・6.1・steering `tech.md` の線）。
- **Adjacent**: `areka-test-threads-av`（同じテストの実行ファイル `--bin areka` を回す＝同時に走らせない）・`test-roots-under-target`（同じテストを書き換える・同時に走らせない）・`actor-thread-log-capture`（背景のスレッドの記録が赤の文言に出れば読み分けが楽になる）・`emo2-real-run-wrap-timeout`（壁時計の待ちを観測の待ちへ置き換える方針が同じ）。

### Constraints

- `areka-test-threads-av` と同時に走らせない（同じテストの実行ファイル）。
- 負荷の再現は静かな机で回す。高負荷のテストを何度も回さない（全体は記録の最初と最後だけ・2 回目からは赤のテストに絞る・回す前に所要時間を伝える）。
- GPU のテストは実の GPU のまま（WARP などへ替えない）。sleep を足す・締切を延ばすだけ・1 フレーム遅らせる形では直さない。
- 1 ファイル 1,000 行の番人を守る（`spine_wait.rs` に足すなら行数を先に測る）。
- 規模の見立て: S〜M（6〜10）。

## 2026-10-23 追記: 負荷の下で止まった会話のテスト（`areka-P0-dot-sakura-specimen` の最終検証・完了時の棚卸）

- `tools/test-all.ps1 -Format -License` の x64 の段で、`emo2_boot::spine::third_scope_talk_tests::spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits` だけが赤（3335 本緑・1 本赤）。打ち切りの文は `待ちの打ち切り［止まった］: 「起動の会話」— 相手が 30.0 秒のあいだ状態を進めなかった（待ち始めから 31.3 秒・それまでの進み 10 回）`（`crates/areka/src/emo2_boot/spine_third_scope_talk_tests.rs:141`）。
- その 1 本だけを回し直すと 1.22 秒で緑。`dot-sakura-specimen` は `areka` のコードに触れていない＝負荷の下でだけ止まる会話の待ち。steering `tech.md` の「負荷の下で許す赤」に当たるか、止まる原因が足場の側にあるかは確かめていない。
- 起票でなくここへ足した訳: 確かめるのに負荷つきの全体の走行を何度も回す要があり（完了の棚卸の例外の 4 つ目）、本 spec の件 2（テストの待ち）と同じ足場（`spine_wait.rs` の族）を触る見込み。

## 2026-10-24 `descript-window-keys` の完了の棚卸で足した観測

- `cargo test -p areka-seriko` の lib の全体の走行で、`bind::tests::bind_resolver_from_groups_reads_three_kinds_of_scope`（ログも共有状態も使わない純関数のテスト・`bind.rs` は `descript-window-keys` で触っていない）が 1 回だけ赤になった。単独で 300 回・lib の全体で 7 回回して再現しなかった（赤の出力の本文は残っていない）。areka の実行ファイルの外（areka-seriko）だが、「並べて回したときだけ落ちる試験」の材料として残す。
- 起票でなくここへ足した訳: 再現の手がかりが無く、確かめるのに全体の走行を何度も回す必要がある（kiro-complete 冒頭ステップの手順 3 の 4 つ目）。同じ「試験の揺れ」の持ち場に寄せた。
- 2026-10-30 `default-balloon-selfmade` の完了の全体テスト（`-Format -License`）でも同じ 1 本だけが赤（3771 本緑・1 本赤・`待ちの打ち切り［止まった］: 「起動の会話」— 相手が 30.0 秒のあいだ状態を進めなかった（待ち始めから 31.1 秒・それまでの進み 10 回）`・141 行目）。単独では 3 回とも緑（1.5〜2.6 秒）・前日の同じコードの全体テストも緑。負荷の下で許す赤の字面に当たる。

## 2026-10-31 棚卸㉛の再測定（main 4068422f）
- **前提の変化**: 10-23（`aa8da8a7`）の後の着地 12 本は、この spec の足場（`emo2_boot/spine_wait.rs` 462・`tools/load-flake.ps1`・`doc/testing.md`）に触れていない。待ちの数は同じ（目印の無い `spin_wait_until` の直の呼び出し 15 か所・`join_bounded` の写し 3 つ＝`emo2_boot/spine_wait.rs` 385・`install/desk_pick_tests.rs` 60・`install/fetch_url_tests.rs` 80 行目・GPU の装置の許可の待ち＝`spine_wait.rs` 420 行目の `take`〔424 行目の `wait_while`〕は上限なし）。`crates/areka/src/` の下の `#[test]` は約 3,570→約 4,088。
- **件 1 の母集団**: COM の部屋（MTA）か GPU の装置を自分で開くファイルは 26→28（中の `#[test]` は 134→141）。足したのは `balloon-scroll-arrows` の `emo2_boot/assets_markers_tests.rs`（新・1 本・絵の読みの COM だけ）と、`emo2_boot/frame_test_support.rs`（974 行＝上限の近く）の `manual_balloon_runtime`。後者は 928 行目で `GraphicsCore::new()` を `GpuPermit::take()` を取らずに呼ぶ＝steering `tech.md` 93 行目の足場の決まりから外れ、同時に 4 つの許可の外で装置を作る（呼ぶのは `emo2_boot/balloon_visibility_phase_box_tests.rs`・`emo2_boot/frame/balloon_face_tests.rs`・`input_events/balloon_pressed_tests.rs`・`input_events/balloon_wheel_tests.rs` の 4 本）。許可を 1 つにした対照はこの 4 本を許可の外に残す＝対照を採る前に件 2 の側で揃える。
- **件 1 の段取り**（棚卸㉓から）: ① 落ちた瞬間のスレッドの名前（＝テストの名前）と呼び出しの積み上げを 1 行残す仕掛けを、テストだけの新しいファイルへ（`main.rs` は 986 行＝足さない）② `emo2_boot::frame` の族に絞って再現し、許可を 1 つにした対照と、`offline-surface-dump` の MTA の族（`offline_dump/` の 36 本）を外した組を採る ③ 負荷つきの全体（5 回で約 50 分）は最後に 1 度・回す前に所要時間を伝える。
- **触るファイル**（テストと足場だけ・本番は `#[cfg(test)]` の部分だけ）: 件 1＝GPU・COM を使うテストと足場（`emo2_boot/{spine.rs,frame_attach_tests.rs,frame_visibility_integration_tests.rs,film_playback_e2e_tests.rs,frame_test_support.rs,ghost_switch_test_support.rs}`〔722〕・`mcp/dump_surface_gpu_test_support.rs`・`placement/placement_shared_test_support.rs`・`crates/areka/src/shell_balloon_switch_session_lap_tests.rs`〔792〕）・本番の欠陥なら `crates/wintf/src/ecs/graphics/`。件 2＝`emo2_boot/{spine_wait.rs,spine_wait_tests.rs,spine_conformance_support.rs,spine_conformance_support_tests.rs,spine_boot_smoke_tests.rs,spine_close_wiring_tests.rs,spine_hold_tests.rs,spine_seriko_loop_tests.rs}`・`crates/areka/src/` 直下の `shell_balloon_switch_session_abort_tests.rs`・`session_end_deadline_tests.rs`・`ghost_session_restart_tests.rs`・`mcp/dump_surface.rs`（テストの部分）・`install/{desk_pick_tests,fetch_url_tests}.rs`・`crates/areka-nar/src/install_commit_tests.rs`（857）。上限の近く: `spine_conformance_support_tests.rs` 959（目印つきへ移すと増える＝先に分ける）・`spine_conformance_lap_tests.rs` 914・`frame_test_support.rs` 974。
- **規模**: 13〜20 のまま（件 1＝5〜9・件 2＝8〜11）。
- **先に要るもの**: 働きの前提は無い。測ることが本体＝ウェーブに並べず、静かな机で 1 本だけ（排他テストの机とひとりの机・測る前に開発機の `areka-impl-watch.exe` が新しいかを確かめる）。測る間に、同じテストのファイルや一時の根の置き場を動かす spec を着地させない（`test-roots-under-target`・`install-live-target-hazards`〔`session_end_deadline_tests.rs`〕・`actor-thread-log-capture`・`background-result-tick-wake`・`balloon-element-order`・`mcp-user-response`・`zorder-chain-residue`）。本物の窓が出る試験（`AREKA_EMO2_REAL_RUN=1` の結合テスト・環境変数なしでも子の areka を起こす `crates/areka/tests/mcp_get_log_real_run.rs`）とは同じ時間に回さない。依存は足さない・文字の層のクレートに触らない。
- **優先度の区分**: B のまま（テストのプロセスが落ちる）。版は v0.1.0（全体テストの偽の赤が開発の速さのゴールに直に効く・棚卸㉗）。
- **要件定義のモデル**: Opus のまま（件 1 の原因が「スレッドに縛られる資源の新しい決まり」を作る形と分かったときだけ、設計の段を Fable にする）。
- **分割の案**: 上限 20 を超える見込みになったら件 2 を切り出す（本文のまま・切り出した 1 本は Opus）。
- **見つけた穴・古くなった記述**:
  - 足場の説明の「WARP 可」は 9 ファイル 13 か所（棚卸㉙が挙げた 4 ファイルより多い・数は 10-23 から同じ）: `emo2_boot/frame_attach_tests.rs` 225・232・`frame_visibility_integration_tests.rs` 9・42・`frame_shell_box_integration_tests.rs` 4・`spine.rs` 4・439・644・`spine_boot_smoke_tests.rs` 17・`spine_talk_close_tests.rs` 16・`spine_test_support.rs` 8・`mcp/dump_surface_gpu_test_support.rs` 4・`crates/areka/src/shell_balloon_switch_session_lap_tests.rs` 6 行目。開発の決まりは「GPU のテストは本物の GPU」＝説明の直し候補（その場で直せる）。`doc/testing.md` に WARP の語は無い。
  - 本文の決まりの節の「`assets_tests.rs` 979 行は先に分ける」は古い（今 891 行）。件 2 の `shell_balloon_switch_session_*_tests.rs` は `emo2_boot/` でなく `crates/areka/src/` の直下に在る。
  - 別の実行ファイルにも同じ名の `join_bounded` がある（`crates/areka-ghost/tests/ghost/` の 3 本・`crates/areka-kanade/tests/kanade/common/` の 2 本）＝範囲に入れるかは要件で決める。`GpuPermit` の説明（`spine_wait.rs` 446 行目）は 1 つのテストで許可を 2 つ取ると待ち合いうると書く＝上限を付けるとき、その形も檻に入れる。
