# Brief: helper-first-load-refusal

> 2026-10-24 `shiori2-protocol` の完了時の棚卸で `/kiro-discovery` で起票（対話なし・測った記録からの引き渡し）。起票の条件: 直したことを確かめるのに、実機の走行を何度も回す必要がある（原因不明・再現待ち）。

## Problem

32 ビットの補助プログラム（i686 helper）が、ゴーストの SHIORI（pasta）の読み込み（`load`）を時々断られる。断られると areka はそのゴーストを話させられず、実物の pasta で回す試験は挨拶を待ったまま時間切れで赤になる。たまにしか起きないので、開発の試験の結果が信用できなくなる。

## Current State

- 2026-10-23 の `shiori2-protocol` のタスク 4.3 で、実物の pasta で回す条件つきの試験 `crates/areka-ghost/tests/real_pasta_online.rs` を排他テストの机で 1 回目に回したとき、helper の LOAD が `LoadReturnedFalse { last_error: 203 }` で断られた。203 は `ERROR_ENVVAR_NOT_FOUND`。記録が空のまま挨拶を 120 秒待ち、`real_pasta_online.rs:192` で panic した（5 通過・1 失敗・121.59 秒）。
- 同じ命令の 2 回目は緑だった（6 通過・79.16 秒）。違いは、1 回目は `TMP`・`TEMP` を `target` の下へ向け替えたまま online の試験を回していたこと（2 回目は online では向け替えない）。
- 完了 spec `areka-P0-emo2-real-run-stability` の件 2（新しい写しからの最初の LOAD が時々断られる）と同じ形。あの spec は 32 回くり返しても再現せず原因不明で閉じ、手がかりとして `last_error` を仲介に足した。今回が `last_error` の初めての観測。
- 失敗した回の `pasta.log` は、試験がゴーストの写しを消すので残っていない。
- 確かめの往復（`shiori2-protocol`）は LOAD の ack の後にしか走らないので、この失敗には届かない。
- 証跡: `.kiro/specs/completed/shiori2-protocol/verification.md` の 4.3 節、`target\s2p-4-3\online.log`（`shiori2-protocol` の作業木の下・消えうる）。

## Desired Outcome

- LOAD が断られる条件が分かり、直っている。または、再現しなければ条件と結果を記録して閉じる。
- 断られたときに、原因を辿れるもの（`pasta.log` の中身・`last_error`・環境変数の様子）が消えずに残る。

## Approach

まず再現する。`TMP`・`TEMP` を向け替えた形と向け替えない形で `real_pasta_online` を何度か回し、203 が出る条件を絞る（`last_error` が本当の原因か、前の呼び出しの残りの値かを見分ける）。失敗した回の写しと `pasta.log` を消さずに残す仕掛けを、試験の側に先に入れる。

## Scope

- **In**: 再現の手順・原因の特定・helper か試験の土台の直し・失敗した回の記録を残す仕掛け。
- **Out**: SHIORI の版の確かめ（`shiori2-protocol`）・helper を `shiori4_host` へ乗り換えること（`shiori4-api`）・pasta の側の直し（pasta のリポジトリの持ち物＝見つかれば申し送る）。

## Boundary Candidates

- 試験の土台（`real_pasta_online.rs` の写しの作り方と後始末）
- helper の LOAD の道（`crates/shiori-host32-helper/` の `shiori_proxy.rs` の `load`／`loadu` の呼び方と `last_error` の取り方）

## Out of Boundary

- `last_error` 以外の新しい診断の口を areka 本体に足すこと（要るなら要件の段で決める）。

## Upstream / Downstream

- **Upstream**: 完了 spec `areka-P0-emo2-real-run-stability`（件 2 と `last_error` の手がかり）。
- **Downstream**: `shiori4-api`（helper を乗り換えるときに同じ癖を持ち込まないように）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の件 2 の続き）。
- **Adjacent**: `areka-test-binary-stability`（areka の試験の実行ファイルの落ち・別の場所）・`shiori4-api`（helper の作り）。

## Constraints

- 実物の pasta で回す試験は排他テストの机が要る。原因が分かるまで何度も回すので、静かな机の時間にまとめて回す。
- 試験の根・一時フォルダは作業木の `target\` の下だけ。

- **優先度の区分**: B（バグ）。
- **版**: 版に縛らない（測ることが本体）。

## 2026-10-24 `descript-window-keys` の完了の棚卸で足した観測

- `cargo test -p areka`（クレートの試験・机なし）の中で `script_security_real_run::real_boot_path_carries_request_origin_and_mcp_script_enters_no_user_break` が、i686 の helper の初回の LOAD が `LoadReturnedFalse` で断られて 2 回続けて赤になった。そのテストだけを単独で回すと緑（47 秒）。同じ検体を使う `emo2_real_run`・`mcp_get_log_real_run`・`smoke_boot_loop_exit` は同じ走行で緑。変えたのは既定の面の値を渡す道だけで、本番の値は組み込みの値と同じだった（`descript-window-keys` の task 3.1 の実装ノート）。初回の LOAD の断りが real-run のテストにも出る例として、原因を調べるときの材料にする。
- 起票でなくここへ足した訳: 触る場所（helper の初回の LOAD）が同じで、確かめるのに実物の走行を何度も回す必要がある（kiro-complete 冒頭ステップの手順 3 の 4 つ目）。
