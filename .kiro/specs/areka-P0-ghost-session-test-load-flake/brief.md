# Brief: areka-P0-ghost-session-test-load-flake

## Problem
機械が重いとき（他のセッションの cargo・rustc・areka が約 15 本並走）に `cargo test -p areka --bin areka` を回すと、ゴーストの切り替えまわりのテストが**毎回違う組で 5〜10 件**赤になる。開発者と実装の手順（`/kiro-impl` のレビュー・`/kiro-complete` の全体テスト）は複数のワークツリーを並べて走らせるのが常なので、本物の欠陥でない赤がレビューの判定を濁し、回し直しの手間と「関係ない赤」を見逃す癖を生む。

## Current State
- 2026-10-04、`areka-P0-mcp-log-history` の 2.3 のレビューで観測した。赤になった族: `ghost_session_switch_tests.rs`・`ghost_session_switch_fallback_tests.rs`・`shell_balloon_switch_session_update_tests.rs` ほか `crates/areka/src/ghost_session_*_tests.rs`・`shell_balloon_switch_session_*_tests.rs`。1 回目の全体で約 10 件、`ghost_session::` だけの回し直しで 5 件、組は 2 回とも違った。
- 単独で回すと緑（`switch_to_self_takes_down_and_reboots_a`・`switch_translate_tests` など）。静かな時の `cargo test -p areka --bin areka ghost_session` は 41 件緑（104 秒）。`tools/test-all.ps1` の全体も緑。
- 変更の差分（log の出口の付け替え）は単体テストが通らない `fn main()` だけで、原因ではない。
- 疑い: 落ち着くのを待つテストが壁時計の締切（例: `ghost_session_restart_tests.rs` の 10 秒の deadline、`run_bounded(.., 20 秒)`）で待っており、負荷で締切を越える。**全体テストでの赤も 1 回記録がある**: 10-05、`host32-testdll-marker-race` の完了時の `tools/test-all.ps1` で `ghost_session::switch_tests::fallback_tests` の 2 本（`target_connect_fail_boots_default_with_halt_and_no_alert`・`sync_target_boot_failure_leaves_only_default_windows`）が「切替先の失敗で既定ゴーストへ戻らない」で赤（他のセッションの cargo と並走して x64 段が約 2 倍の 1,497 秒・直前に `target\nar-samples\work` の残骸の退避が os error 5・単独では 9 本すべて緑＝roadmap の覚え書き「一度だけ落ちた試験」）。os error 5 との関係も調べる。

## Desired Outcome
- 負荷をかけた条件で赤を再現する手順が 1 つあり、どのテストがどの待ちで落ちるかが分かっている。
- 落ちる理由が「締切が短すぎる」なら、締切を延ばすのでなく**待つ条件を観測（チャネル・状態の到達）へ置き換える**か、締切を越えたときの失敗の文言が「負荷で遅い」と区別できる形にする（sleep を足して直さない）。
- 本物の競合（順序の取り違え）が見つかったら、それを直すテストを 1 本添えて直す。
- 再現しなければ、その事実（何を試して赤 0 件か）を記録して据え置きの行へ移す。

## Approach
まず再現: `cargo test -p areka --bin areka ghost_session` を、CPU を食う並走（`cargo build` の並列や `stress` 相当の子プロセス）を付けて数回回し、落ちたテスト名と失敗の文言を集める。落ちたテストの待ちを読み、壁時計の締切に頼る待ちを観測の待ちへ置き換える。

## Scope
- **In**: 上のテストの族の待ち方の見直し・再現の手順の記録・本物の競合があればその修正。
- **Out**: 本番のゴーストの切り替えの振る舞いの変更（競合が見つかった場合を除く）・他の族（`zorder-chain-residue` の族）。

## Boundary Candidates
- テストの待ちの部品（共有の待ち合わせの手伝い）
- 本番側の競合（見つかった場合だけ）

## Out of Boundary
- 全体テストのスクリプト（`tools/test-all.ps1`）の並列度の変更
- `zorder-chain-residue` の間欠赤の族

## Upstream / Downstream
- **Upstream**: なし
- **Downstream**: 並走する全ての spec のレビューと完了の手順（赤の雑音が減る）

## Existing Spec Touchpoints
- **Extends**: なし（完了 spec の切り替え系のテストを触る）
- **Adjacent**: `zorder-chain-residue`（同じ「間欠赤」の型・別の族）

## Constraints
- sleep を足して直さない・1 フレーム遅らせる解を取らない（状態の持ち方で解く）。
- 実機の一時フォルダはワークツリーの `target\` の下だけ。
