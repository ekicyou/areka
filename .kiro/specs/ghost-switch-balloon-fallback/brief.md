# Brief: ghost-switch-balloon-fallback

> 2026-10-30 `startup-install-option` の完了時の棚卸で `/kiro-discovery` で起票（対話なし・実機の試験を作るときに見つけた件の引き渡し）。起票の条件: 直し方が開発者の裁定に要る（SSP の振る舞いは測っていない）。区分 **B**。

## Problem

- **利用者**: バルーンを同梱していないゴーストへ切り替えると、置き場にもバルーンが無いとき、今使っているバルーンのまま続けずに切り替えが失敗する。既定のゴーストも置き場に無いと、areka はそのまま終わる。
- SSP は既定のバルーンで続けると見込んでいる（測っていない）。

## Current State

- 切り替え（`crates/areka/src/emo2_boot/ghost_switch.rs` 624 行目の `switch_to`）の段 ③ で、切替先のバルーンを `resolve_balloon_for_ghost` で解く（`ghost_switch.rs` 828 行目）。決まらなければ `failed("balloon", …)` で `ghost_switch_boot_failed stage="balloon"` の `error!` を残して失敗する（835 行目）。
- 失敗すると既定のゴーストへ戻す（`switch_to_default`・723 行目）。既定のゴーストがカタログに無いと `ghost_switch_fatal` で終わる（`fatal`・892 行目）。
- `resolve_balloon_for_ghost`（`crates/areka/src/boot_config.rs` 456 行目）は、そのゴーストの最後のバルーンの記憶 → descript.txt の `default.balloon.path` → `balloon` → 同梱の最初の 1 個 → 唯一 → 既定 → 無作為の順に解く。候補の一覧（ゴーストの中の `balloon\` と根の和）が空なら `NoBalloon` を返す（`crates/areka/src/boot_resolve.rs` 319 行目）。**今使っているバルーン（起動の行で渡したもの・切り替えの前に使っていたもの）は候補に入らない。**
- 起動の行でバルーンを渡したときは、起動ではそのバルーンが使える。切り替えでだけ落ちる。
- `startup-install-option` の実機の試験 `crates/areka/tests/startup_install_real_run.rs` は、根の `balloon\` に既定のバルーン（検体 `areka-balloon`）を写して、この件を避けている（配布物と同じ形）。2026-10-30 までは emo2 の同梱バルーン `emo2-kakukaku` を写しており、切替先のゴーストが emo2 のバルーンのままになっていた（段 5「唯一」で選ばれるため）。
- `default-balloon-selfmade`（2026-10-30 着地）で既定のバルーンは `areka-balloon` になった（`boot_resolve.rs` 34 行目の `DEFAULT_BALLOON_FOLDER`）。配布物には `balloon\areka-balloon` が入る（`tools/package.ps1`・`dist/README.txt`）。ただしこれは置き場にフォルダを置くだけで、解き方の鎖は変わらない。置き場が空のとき（試験の根・利用者が消したとき）は今も `NoBalloon` になる。既定のバルーンが「候補の一覧に 2 つ以上あるとき」の段 6 でしか参照されないのも変わらない。

## Desired Outcome

- 切替先のゴーストにも置き場にもバルーンが無いとき、切り替えは今使っているバルーン（または決めた代わり）で続く。areka が終わらない。
- 決まらなかったときと代わりを使ったときは、記録に 1 件残る。
- 決め方は ukadoc と SSP に合わせる。

## Approach

- 要件の段の最初に、ukadoc でバルーンの決め方（切り替えのときの決め方・バルーンが無いとき）を読む。読めない点は SSP の MCP で測る（打つ前に許し）。
- 第一の案は、切り替えのときだけ「今使っているバルーン」を鎖の最後（`NoBalloon` の前）の段として足す。起動の鎖も同じにするかは要件の段で決める。
- 直す前に赤になる決定論テストを置く（空の置き場・バルーンを持たないゴーストへの切り替え）。直した後、`startup_install_real_run.rs` の写し（根の `areka-balloon`）を外しても切り替えが通るかを確かめる。

## Scope

- **In**: 切り替えのバルーンの解き方（`resolve_balloon_for_ghost` と鎖 `boot_resolve::resolve_balloon`、または `ghost_switch.rs` の段 ③）・記録・決定論のテスト・実機の試験の回避を外すこと。
- **Out**: 既定のバルーンの中身・配布物の作り方・シェルに紐づくバルーン（`shell-companion-balloon`）。

## Boundary Candidates

- バルーンの解き方の鎖（`boot_resolve.rs`・`boot_config.rs`）
- 切り替えの段 ③（`emo2_boot/ghost_switch.rs`）

## Out of Boundary

- 切り替えの失敗から既定のゴーストへ戻す道そのもの（`switch_to_default`・`fatal`）

## Upstream / Downstream

- **Upstream**: 完了 spec `startup-install-option`（`.kiro/specs/completed/startup-install-option/`・実機の試験で見つけた）。完了の `default-balloon-selfmade`（既定のバルーン）・`ghost-inner-balloon`（ゴーストの中のバルーン）。
- **Downstream**: 空の根から回す実機の試験（`install-before-boot`）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `shell-companion-balloon`（同じ `resolve_balloon_for_ghost` を「シェルを先に決めてからバルーン」に並べ替える＝触る順を決める）・`audit-emo2-boot-switch`（`ghost_switch.rs` の見直し）・`runtime-module-untangle`／`session-module-gather`（`ghost_switch` と `boot_config` を `session/` へ移す＝同じウェーブに置かない・パスは roadmap の「本体の crate 分け」の対応表で読み替える）・`install-before-boot`（空の根から起こす）。

## Constraints

- 失敗はメッセージボックスで告げない（既定のゴーストの台詞）。
- 失敗の道は必ず記録（`error!`＋`Err`）。
- テストは兄弟のファイル `*_tests.rs`。
- 実機の根・検体は作業木の `target\` の下だけ。

- **優先度の区分**: B（バグ。切り替えが落ち、既定のゴーストが無いと areka が終わる）。
- **版**: v0.2.0（致命的でない新しいバグ＝次の版。起こるのは置き場にバルーンが無いときだけで、配布物には既定のバルーンが入る）。
- **要件定義のモデル**: Opus。今ある鎖に段を 1 つ足すかどうかで、新しい境界・並行の設計には当たらない。
- **規模**: S（3〜6 タスクの見込み・見積もりは要件の段）。
- **分割の案**: なし。
- **要件の段の議題**（答えで作業が変わるもの）:
  1. 代わりに使うバルーン（今使っているもの・既定のバルーン・起動の行で渡したもの）。ukadoc を読み、読めなければ SSP で測る。
  2. 起動の鎖も同じにするか。

## 2026-11-04 棚卸㉞の再測定（main `932406f9`）

- **前提の変化**: 前の棚卸の後に着地した `areka-test-binary-stability` と v0.1.0 の公開は、この spec の触るファイルを動かしていない。行の番号は起票の後にずれている（下の「古くなった記述」）。
- **触るファイル**: 切り替えの段 ③（`crates/areka/src/emo2_boot/ghost_switch.rs` 934 行）・バルーンの解き方の鎖（`crates/areka/src/boot_config.rs` 658 行・`crates/areka/src/boot_resolve.rs` 769 行）とその兄弟のテスト（`ghost_switch_balloon_tests.rs`・`boot_resolve_balloon_tests.rs` ほか）・実機の試験 `crates/areka/tests/startup_install_real_run.rs`。新しいファイルは要らない見込み。どのファイルも配置（`placement`）を引いていない。
- **規模**: S（3〜6 タスク）のまま。
- **先に要るもの（見直しと crate 分け）**: 起票が 10-22 の見直しの突き合わせより後なので、今回はじめて突き合わせた。`ghost_switch.rs` は見直し `audit-emo2-boot-switch`、`boot_config.rs`・`boot_resolve.rs`・`tests/` は見直し `audit-areka-app` の持ち物。crate 分けでは `runtime-module-untangle` が `ghost_switch.rs` を、`session-module-gather` が `boot_config.rs`・`boot_resolve.rs` を `src/session/` へ移す。見直しは作り直しの前に済ませる決まりなので、この spec は見直し 2 本の着地を待つ。`audit-areka-app` は crate 分けの最後の `session-crate-split` の後なので、実際には crate 分けの 6 本が全部着地するまで待つ。roadmap の行の「同じウェーブに置かない」は「着地を待つ」へ強める。
- **ほかの未完了の spec と分け合うファイル**: `shell-companion-balloon`（`boot_config.rs` の `resolve_balloon_for_ghost`）・`sylphya-sqlite-kvs`（`boot_resolve.rs`）・`instant-to-interrupt-ticks`（`ghost_switch.rs`）・`dev-folder-alias`・`multi-ghost-runtime`（両方）。
- **優先度の区分**: B（切り替えが落ち、既定のゴーストが無いと areka が終わるバグ）。
- **要件定義のモデル**: Opus（今ある鎖に段を 1 つ足すだけ）。
- **分割の案**: なし。
- **古くなった記述**: 行の番号＝`switch_to` は 613 行目（起票は 624）・`resolve_balloon_for_ghost` の呼び出しは 824（828）・`failed("balloon", …)` は 831（835）・`switch_to_default` は 718（723）・`fatal` は 888（892）・`boot_config.rs` の `resolve_balloon_for_ghost` は 448（456）・`boot_resolve.rs` の `NoBalloon` を返す所は 335（319）。
