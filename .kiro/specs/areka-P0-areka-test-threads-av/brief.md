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
