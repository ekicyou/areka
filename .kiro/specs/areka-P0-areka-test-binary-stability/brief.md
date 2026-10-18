# Brief: areka-P0-areka-test-binary-stability

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

### 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S（3〜6）のまま。切らない。原因が本番のコードにあれば +2〜3（直しと檻）。
- 前提の状態: 上流なし・今すぐ着手できる。起票（`1bb449ae`）の後に入ったのは `mcp-dump-images` だけで、新しい観測（落ちた・落ちない）の記録は無い。roadmap の覚え書きにも本件の再発は載っていない。
- 崩れた前提／古くなった位置:
  - **実際の GPU を使うテストが `areka` の実行ファイルに 15 本増えた**: `mcp-dump-images` の `mcp/dump_surface_gpu_tests.rs`（9 本）・`mcp/dump_balloon_gpu_tests.rs`（6 本）と土台 `mcp/dump_surface_gpu_test_support.rs`（WARP を含む GPU 資源・偽の窓・自分のスレッドでのメッセージの汲み出し）。起票のときより「スレッドに縛られる資源」を使うテストの母集団が広い＝二分探索はこの 2 本を外した対照も採ること。
  - 前から GPU・COM を使うテストのファイル（`emo2_boot/{assets_tests, assets_shell_tests, frame_attach_tests, frame_visibility_integration_tests, switch_assets_tests}.rs`・`ghost_session_restart_tests.rs`・`shell_balloon_switch_session_lap_tests.rs`・`placement/{measure_tests.rs, placement_shared_test_support.rs}`・`input_events/balloon_pass_through_tests.rs`）は、どれもテスト同士を順番に並べる錠を持たない（`areka` の中に `Mutex<()>` の錠は 0。wintf には `TICK_WAKE_TEST_LOCK` がある）。`assets_tests.rs`・`assets_shell_tests.rs` は `CoInitializeEx(MULTITHREADED)` を自前で呼ぶ。
  - wintf `--test graphics` の一度だけの AV（覚え書き）は、負荷の高いときに 60 秒止まった後に落ちた形で、完了 `wintf-gpu-test-crash` と同じ。こちらは「スレッド 4 本で再現する」ので別の入口のまま。
- 触るファイル（見込み・原因しだい）: 上の GPU・COM を使うテストのファイルとその土台（`emo2_boot/ghost_switch_test_support.rs`・`mcp/dump_surface_gpu_test_support.rs`・`placement/placement_shared_test_support.rs`）。順番に並べる錠を足すなら新しい小さな共有ファイル 1 本。wintf の GPU 資源の作り方が原因なら `crates/wintf/src/ecs/graphics/` も。
- 議題: なし（直し方は原因が分かってから決まる）。
- 同時に走らせない: `ghost-session-test-load-flake`（同じ実行ファイル・負荷を互いに汚す）。

### 2026-10-10 `ghost-session-test-load-flake` からの申し送り（既定のスレッドの数でも落ちた）

- 負荷の再現（論理 CPU 22・負荷の子 44 本・`--test-threads` は渡さない＝既定の数）の下で、`cargo test -p areka --bin areka` が `0xc0000005`（`STATUS_ACCESS_VIOLATION`）で 3 度落ちた。panic も「60 秒を越えた」の行も無く、どのテストかは出ない。
  - `load-repro.md` の 4.7 の 4 回目（94 秒・ok 522 本・`frame::transition_branch_tests` の後）
  - 4.8 の 3 回目（18.7 秒・`frame::text_scale_tests` の後）
  - 4.9 の 5 回目（65.9 秒）
- 落ちる頃に走っていたのは、名前の順で `frame::visibility_integration_tests`（各スレッドで `CoInitializeEx(MULTITHREADED)` を呼び、`GraphicsCore::new` から MTA の Compositor を作る）の先頭の十数本。上の「`--test-threads=4` でだけ再現」の前提は崩れた＝既定のスレッドの数でも、負荷でスレッドの重なりが増えると起きる。
- 記録の置き場: `.kiro/specs/completed/areka-P0-ghost-session-test-load-flake/load-repro.md` の 4.7〜4.9（`target\load-flake\after-20261009-*\round-N.log`）。

### 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: `ghost-session-test-load-flake`（10-10 着地）で、GPU の装置を作る足場は作る直前に「GPU の装置の許可」（`crates/areka/src/emo2_boot/spine_wait.rs` の `GpuPermit`・同時に 4 つまで）を取る形になった。取る所は 6 ファイル（`emo2_boot/spine.rs`・`frame_visibility_integration_tests.rs`・`frame_attach_tests.rs`・`film_playback_e2e_tests.rs`・`mcp/dump_surface_gpu_test_support.rs`・`shell_balloon_switch_session_lap_tests.rs`）。上の申し送りの 3 度の落ちは、この許可が入った後の記録＝同時に 4 つまでに絞っても起きる。起票のときの条件「スレッド 4 本」と数が同じなので、許可を 1 つにした対照が安い手がかりになる。
- GPU・COM を使うテストは C4 でさらに増えた（`emo2_boot/film_playback_e2e_tests.rs`＝`animated-image-playback`・`frame_balloon_timeout_notice_e2e_tests.rs`＝`balloon-lifecycle-events`）。10-05 の一覧に無かった `emo2_boot/frame_shell_box_integration_tests.rs`（`shell-balloon`）も同じ足場を使う。`areka` の実行ファイルのテストは 2,897 本。テスト同士を順番に並べる錠（`Mutex<()>`）は今も 0。待ちの部品が打ち切りの文言を出すようになったので、3 度とも「止まってから」でなく「いきなり」落ちたと記録から読める。
- **触るファイル**（原因しだい）: 上の 6 ファイル・`emo2_boot/assets_tests.rs`（979 行＝上限の近く）・`assets_shell_tests.rs`・`spine_wait.rs`（462 行）。落ちたスレッドの名前を残す仕掛けを足すなら、テストだけの新しい小さなファイル 1 本（`crates/areka/src/main.rs` は 950 行なので足さない）。本番の欠陥なら `crates/wintf/src/ecs/graphics/`。
- **規模**: S〜M（5〜9）。切らない。
- **仕事の芯は再現と測定**＝静かな机が要り、ほかの spec と並べない。重い回を減らす段取り: ① 先に「落ちた瞬間のスレッドの名前（＝テストの名前）と呼び出しの積み上げを 1 行残す仕掛け」をテストの実行ファイルへ入れる（二分探索の何十回を、1 回の再現に置き換える）。② 再現は `emo2_boot::frame` の族に絞って回す（3 度とも、始まりから 19〜94 秒・この族の頃に落ちた。絞れば 1 回は数分以内の見込み）。③ 全体を負荷つきで回すのは最後の確かめの 1 度だけ（5 回で約 50 分・回す前に所要時間を伝える）。静かな机の全体は 1 回 約 70 秒。
- **先に要るもの**: なし。`test-wait-marker-gaps` とは同じテストの実行ファイル（`--bin areka`）を回し、足場のファイル（`spine_wait.rs`・`emo2_boot/ghost_switch_test_support.rs`・`ghost_session_restart_tests.rs`・`shell_balloon_switch_session_lap_tests.rs`）も重なるので、同じウェーブに置かない。**こちらを先に**（負荷の下の 5 回に 1 回がプロセスごと落ちて、あちらの確かめの回を無効にする。こちらの直しが許可の数を変えるなら、あちらの「許可の待ちの上限」はその後に決める）。`test-roots-under-target` とも同じウェーブに置かない（一時フォルダの置き場が動くと比べる基準が変わる）。
- **優先度の区分**: B（バグ。テストのプロセスが落ちる・本番のメモリの壊れの疑いが消えていない）。
- **要件定義のモデル**: Fable（スレッドの競り合い・原因しだいで直し方が分かれる）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 本文の「`--test-threads=4` で二分探索」は、既定のスレッドの数でも落ちると分かった今は遠回り（上の ①②）。足場の説明（`emo2_boot/spine.rs`・`frame_attach_tests.rs`・`frame_visibility_integration_tests.rs`）に「WARP 可」とあるが、steering `tech.md` は GPU のテストを実の GPU で行うと決めた（注記の直し候補）。

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

### 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 起票（10-10・着地の直前の枝）の数は main でも同じ。目印の無い `spin_wait_until` の直接の呼び出しは 15 か所（spine の族 9・`session_end_deadline_tests.rs` 4・`mcp/dump_surface.rs` 1・`ghost_session_restart_tests.rs` 1）。`areka` の中の `join_bounded` の写しは 3 つ（`emo2_boot/spine_wait.rs`・`install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`）。GPU の装置の許可の待ち（`spine_wait.rs` の `GpuSlots::take`）は今も上限なし。崩れた前提は無い。
- **触るファイル**（どれも `crates/areka/src/` の下・テストと足場だけ）: `emo2_boot/spine_wait.rs`（462 行）・`spine_wait_tests.rs`・`ghost_switch_test_support.rs`（672）・`spine_conformance_support.rs`（773）・`spine_conformance_support_tests.rs`（**959 行＝上限の近く**。目印つきへ移すと増えるので先に分ける）・`spine_boot_smoke_tests.rs`・`spine_close_wiring_tests.rs`・`spine_hold_tests.rs`・`spine_seriko_loop_tests.rs`・`shell_balloon_switch_session_abort_tests.rs`（536）・足場の目印を変えるなら `shell_balloon_switch_session_lap_tests.rs`（791）・`session_end_deadline_tests.rs`（523）・`ghost_session_restart_tests.rs`・`mcp/dump_surface.rs`（テストの部分だけ）・`install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`。ほかに `crates/areka-nar/src/install_commit_tests.rs`（802）。`emo2_boot/spine_conformance_lap_tests.rs` は 911 行。
- **規模**: M（8〜11）。待ちの種類 5 つ＋負荷の記録。切らない。
- **仕事の芯は書き換え**で、測定は確かめのためだけ。項目 2・3・5 は負荷なしで直せる（打ち切りの文言は時計を注入した檻で見る。静かな机の全体は 1 回 約 70 秒）。重い回は 2 度だけにする: 途中は赤が出たテストの名前で絞って回し（`tools/load-flake.ps1 -Filter`・1 回 数分）、全体の負荷つき 5 回（約 50 分）は最後に 1 度。回す前に所要時間を伝える。
- **先に要るもの**: 働きの依存は済み（`ghost-session-test-load-flake`）。`areka-test-threads-av` の後に置く（同じテストの実行ファイル・足場のファイルが重なる・あちらの落ちが負荷の回を無効にする）。ほかのファイルの重なり: `install-live-target-hazards`（`session_end_deadline_tests.rs`・`areka-nar` の `install.rs` の近く）・`mcp/dump_surface.rs` を触る MCP の spec・`test-roots-under-target`（呼び出しを 1 つずつ書き換える案のときだけ `install/fetch_url_tests.rs` ほか）。
- **優先度の区分**: B（バグ＝テストの穴。負荷の下で、許さないと決めた赤が出うる）。
- **要件定義のモデル**: Fable（待ちと並行・項目 2 に開発者の裁定の分かれ目がある）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 同じ名前の `join_bounded` が、別の実行ファイルのテストにもある（`crates/areka-ghost/tests/ghost/` の 3 本〔接続の失敗・helper の生死・切断〕と `crates/areka-kanade/tests/kanade/common/` の 2 本）。待ちの部品は `areka` の実行ファイルの中にしか無いので、これらは steering `tech.md` の線の外に居る＝範囲に入れるかを要件で決める。`GpuPermit` の説明は「許可を 2 つ取るテストは 1 本だけ」と書く＝上限を付けるとき、その 1 本が待ち合いで打ち切られる形も檻に入れる。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 版: **v0.1.0（棚卸㉗で引き上げた）**・**測ることが本体＝ウェーブに並べず、静かな机で 1 本だけ**（本文のまま）。理由: 全体テスト（負荷つきで 1 回 約 50 分）が、テストのプロセスの落ち（`0xc0000005`）と目印の無い待ちで偽の赤を出すと、そのたびに回し直しになり、開発の速さのゴールに直に効く。「SSP並みの自動試験」を足すほど、土台のテストの実行ファイルが安定していることが要る。
- 前提の変化（棚卸㉓から・本 brief は棚卸㉕・㉖で測っていない）: `areka` のテストの実行ファイルの `#[test]` は約 2,897 → **約 3,156**（`crates/areka/src/` の下で数えた）。目印の無い `spin_wait_until` の直接の呼び出しは今も約 15 か所。`join_bounded` の写しは 3 つ（`emo2_boot/spine_wait.rs`・`install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`）のまま。上限の近いファイル: `emo2_boot/assets_tests.rs` 979・`spine_conformance_support_tests.rs` 959・`spine_conformance_lap_tests.rs` 912（変わらず）・`shell_balloon_switch_session_lap_tests.rs` 792・`ghost_switch_test_support.rs` 675。`spine_wait.rs` は 462 のまま。
- 触るファイル: 本文と件 1・件 2 の棚卸㉓の節のとおり。
- 規模: 13〜20 のまま（件 1＝5〜9・件 2＝8〜11）。20 を超える見込みになったら件 2 を切り出す（本文のまま）。
- 先に要るもの: 働きの前提なし。**理想は `impl-watch-test-desk` の後**（負荷テストの机を「排他テストの机＋実装の机のひとり占め」に組み直してから、静かな机で測る。今の机でも測れるが、ほかのセッションの全体テストや実機の UI テストと重なりうる）。
- 同じ時間に回さない相手: 実機の窓が出る試験（`emo2-real-run-stability`・`real-run-mcp-kit`・`pasta-in-windows-verification`）。ファイルの上では `test-roots-under-target`（版に縛らない列）と同じウェーブに置かない（本文のまま）。
- 優先度の区分: B のまま。
- 要件定義のモデル: **Fable**（本文のまま・新しく並行の保証を決める＝原因の分からないスレッドの競り合い〔件 1〕。件 2 の項目 2 は開発者の裁定の分かれ目）。件 2 だけを切り出すなら、その 1 本は Opus で足りる。
- 分割の案: 上の条件つきの 1 つだけ。
- 見つけた穴・古くなった記述: 本 brief には棚卸㉕・㉖の節が無い（v0.1.0 の外だった）。棚卸㉓の節の「`main.rs` は 950 行」は今 1,000 行（上限ちょうど・落ちたスレッドの名前を残す仕掛けは新しいファイルへ置く方針は変わらない）。
