# Brief: areka-P0-areka-test-threads-av

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-mouse-drag-events` のタスク 2.2 の検証と完了時の棚卸（`.kiro/specs/completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes の「範囲外」）。

## Problem

- **開発者・実装の手順**: `cargo test -p areka --bin areka -- --test-threads=4` が `STATUS_ACCESS_VIOLATION`（不正なメモリアクセス）でテストのプロセスごと落ちる。どのテストで落ちたかが分からず、スレッド数を絞って負荷を下げる手が使えない。メモリの壊れ方が本番の欠陥の兆しかもしれない。

## Current State

- 2026-10-04〜05、`mouse-drag-events` の 2.2 の検証で観測した。新しく足した drag のテストを外しても落ちる（本 spec の前からある）。既定のスレッド数では落ちない（`tools/test-all.ps1` の全体も緑）。
- roadmap の覚え書き「一度だけ落ちた試験」にある `wintf --test graphics` の `STATUS_ACCESS_VIOLATION`（10-04・`mcp-expression-table`）とは別の入口（こちらは `areka` の bin のテストで、条件を揃えると再現する）。同じ根かは分からない。
- 疑い: スレッドを跨いで共有してはいけない Windows の資源（COM の部屋・窓・GPU の資源）を、テストの並びしだいで別スレッドから触っている。

## Desired Outcome

- 落ちるテスト（または組）を特定する手順が 1 つあり、記録に残っている。
- 原因がテストの土台（スレッドに縛られる資源を共有している）なら、土台を直すか、そのテストの族をスレッド 1 本で回す印を付ける。本番のコードの欠陥なら、それを直すテストを添えて直す。
- 再現しなければ、試した条件を記録して据え置きへ移す。

## Approach

`--test-threads=4` で回すテストの集合を二分探索で絞り（モジュールの単位で `cargo test -p areka --bin areka <モジュール> -- --test-threads=4`）、落ちる組を特定する。WinDbg などで落ちた場所の積み上げを採れるなら採る。

## Scope

- **In**: 再現の手順・原因の特定・テストの土台か本番のコードの修正・記録。
- **Out**: `ghost-session-test-load-flake`（壁時計の締切で負荷のときに赤になる族）・`wintf` の graphics の一度だけの AV（覚え書きのまま）。

## Boundary Candidates

- テストの土台（スレッドに縛られる資源の扱い）
- 本番側の欠陥（見つかった場合だけ）

## Out of Boundary

- `tools/test-all.ps1` の並列度の変更。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: 実装の手順（`/kiro-impl` のレビュー）で負荷を下げる手が使えるようになる。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `ghost-session-test-load-flake`（同じ `areka` のテストの族を触るなら同時に走らせない）。

## Constraints

- 段は**バグ**（テストのプロセスが落ちる・原因不明のメモリの壊れ）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S（3〜6）のまま。切らない。原因が本番のコードにあれば +2〜3（直しと檻）。
- 前提の状態: 上流なし・今すぐ着手できる。起票（`1bb449ae`）の後に入ったのは `mcp-dump-images` だけで、新しい観測（落ちた・落ちない）の記録は無い。roadmap の覚え書きにも本件の再発は載っていない。
- 崩れた前提／古くなった位置:
  - **実際の GPU を使うテストが `areka` の実行ファイルに 15 本増えた**: `mcp-dump-images` の `mcp/dump_surface_gpu_tests.rs`（9 本）・`mcp/dump_balloon_gpu_tests.rs`（6 本）と土台 `mcp/dump_surface_gpu_test_support.rs`（WARP を含む GPU 資源・偽の窓・自分のスレッドでのメッセージの汲み出し）。起票のときより「スレッドに縛られる資源」を使うテストの母集団が広い＝二分探索はこの 2 本を外した対照も採ること。
  - 前から GPU・COM を使うテストのファイル（`emo2_boot/{assets_tests, assets_shell_tests, frame_attach_tests, frame_visibility_integration_tests, switch_assets_tests}.rs`・`ghost_session_restart_tests.rs`・`shell_balloon_switch_session_lap_tests.rs`・`placement/{measure_tests.rs, placement_shared_test_support.rs}`・`input_events/balloon_pass_through_tests.rs`）は、どれもテスト同士を順番に並べる錠を持たない（`areka` の中に `Mutex<()>` の錠は 0。wintf には `TICK_WAKE_TEST_LOCK` がある）。`assets_tests.rs`・`assets_shell_tests.rs` は `CoInitializeEx(MULTITHREADED)` を自前で呼ぶ。
  - wintf `--test graphics` の一度だけの AV（覚え書き）は、負荷の高いときに 60 秒止まった後に落ちた形で、完了 `wintf-gpu-test-crash` と同じ。こちらは「スレッド 4 本で再現する」ので別の入口のまま。
- 触るファイル（見込み・原因しだい）: 上の GPU・COM を使うテストのファイルとその土台（`emo2_boot/ghost_switch_test_support.rs`・`mcp/dump_surface_gpu_test_support.rs`・`placement/placement_shared_test_support.rs`）。順番に並べる錠を足すなら新しい小さな共有ファイル 1 本。wintf の GPU 資源の作り方が原因なら `crates/wintf/src/ecs/graphics/` も。
- 議題: なし（直し方は原因が分かってから決まる）。
- 同時に走らせない: `ghost-session-test-load-flake`（同じ実行ファイル・負荷を互いに汚す）。

## 2026-10-10 `ghost-session-test-load-flake` からの申し送り（既定のスレッドの数でも落ちた）

- 負荷の再現（論理 CPU 22・負荷の子 44 本・`--test-threads` は渡さない＝既定の数）の下で、`cargo test -p areka --bin areka` が `0xc0000005`（`STATUS_ACCESS_VIOLATION`）で 3 度落ちた。panic も「60 秒を越えた」の行も無く、どのテストかは出ない。
  - `load-repro.md` の 4.7 の 4 回目（94 秒・ok 522 本・`frame::transition_branch_tests` の後）
  - 4.8 の 3 回目（18.7 秒・`frame::text_scale_tests` の後）
  - 4.9 の 5 回目（65.9 秒）
- 落ちる頃に走っていたのは、名前の順で `frame::visibility_integration_tests`（各スレッドで `CoInitializeEx(MULTITHREADED)` を呼び、`GraphicsCore::new` から MTA の Compositor を作る）の先頭の十数本。上の「`--test-threads=4` でだけ再現」の前提は崩れた＝既定のスレッドの数でも、負荷でスレッドの重なりが増えると起きる。
- 記録の置き場: `.kiro/specs/completed/areka-P0-ghost-session-test-load-flake/load-repro.md` の 4.7〜4.9（`target\load-flake\after-20261009-*\round-N.log`）。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

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
