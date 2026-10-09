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
