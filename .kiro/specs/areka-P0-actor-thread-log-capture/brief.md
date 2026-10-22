# Brief: areka-P0-actor-thread-log-capture

> 起票: 2026-10-10（`areka-P0-ghost-session-test-load-flake` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票。`completed/areka-P0-ghost-session-test-load-flake/tasks.md` の Implementation Notes の 5.6 と「範囲外の申し送り」⑵）。コードの読みで確かめた。

## Problem

テストの記録の捕捉 `log_capture_kit::capture` は、呼んだスレッドで出た記録しか集めない。`areka-actor` の `spawn_actor` で起こしたスレッド（インストールの背景の `install`・更新の `update`・ゴーストの `kanade`・SHIORI の `shiori` など）が出す記録は、そのスレッドを動かしているテストの捕捉に入らない。そのため、テストが赤になったとき、背景のスレッドが書いた失敗の詳しい記録（どのパスで、OS のどのエラーか）を赤の文言に添えられず、原因の読みが遠回りになる。

## Current State

- `log_capture_kit::capture`（`crates/log-capture-kit/src/capture.rs`）は `tracing::subscriber::with_default` で、呼んだスレッドの既定の受け手だけを窓の間差し替える。crate の説明（`lib.rs`）も「呼出スレッド局所」と書き、別のスレッドの記録は入らない。
- 全スレッドの捕捉 `install_global_capture_all`（`global.rs`）は、プロセスの既定を 1 度だけ据え付けて取り消せず、以後そのテストの実行ファイルの全スレッドで `tracing::enabled!` を真にする。areka のテストの実行ファイルでは使わない決まりにしている（`session_end_deadline_tests.rs` の冒頭の説明）。
- `spawn_actor`（`crates/areka-actor/src/spawn.rs`）は `std::thread::Builder` で素のスレッドを起こし、中で `info_span!("actor")` に入って body を呼ぶだけ。呼んだスレッドの既定の受け手を子へ渡す手は無い。
- 走り始めのフック `install_thread_start_hook` はある。ただし型 `ThreadStartHook` は `fn(&str)`（アクター名だけを受ける・状態を持てない）で、プロセスに 1 度だけ・最初の導入が勝つ。中身は実行体 `areka` の `thread_roles`（スレッド名簿へ役割名を載せる）で、本番は `main` が導入する。areka のテストの実行ファイルでも `thread_roles_tests.rs` が同じ関数を導入するので、テストの捕捉に使える枠は空いていない。テストの捕捉には今は使われていない。
- 起きた所（`ghost-session-test-load-flake`）:
  - タスク 5.6: 上書きの失敗の赤を読むとき、`areka_nar` の確定の失敗（パスと os error 5）は背景のスレッド `install` で出るので、テストの手がかり（`desk_overwrite_tests.rs` の `clues`）に載せられず、`OnInstallFailure` の Reference の語で代わりにした（`clues` の説明に同じことが書いてある）。
  - タスク 3.3: kanade のスレッドで出る締切の error（`change_deadline_exceeded`）が捕捉に映らず、問い合わせの往復で「締切が切れていない」を見る形で代わりに判定した。
  - `session_end_deadline_tests.rs` も、見張りのスレッドの `warn!` を捕まえられないので、解く手の回数で代わりに判定している。

## Desired Outcome

- テストが捕捉の窓の中で動かしたアクターのスレッドの記録が、そのテストの捕捉に入る。赤の文言に背景のスレッドの失敗の記録（パス・OS のエラー）を添えられる。
- 本番の記録（出る行・水準・出先）は変わらない。
- 並べて回るほかのテストの記録は混ざらない。

## Approach

候補を要件で決める。

- `spawn_actor` が、起こす時点の呼んだスレッドの既定の受け手（`tracing` の `dispatcher::get_default`）を取り、子のスレッドの中で既定にする。本番は既定の受け手がプロセスの全体の設定なので出る記録は同じで、テストでは窓の受け手へ届く。確かめること: 窓が閉じた後に子が出した記録の扱い・テストより長く生きるアクター・窓の外で起こされたスレッドは捕まらないこと（`install` のスレッドは窓口が最初の依頼で 1 度だけ起こす＝窓の中で依頼を出すテストでないと引き継げない）・log-capture-kit が説明する発行点の interest の焼き付きと番兵の扱い。
- フックを状態を持てる形へ広げる案は、本番の `thread_roles` と同じ枠を分け合うので、取るなら両方が並べる形にする。
- 引き継ぎをテストだけに効かせるか、本番も同じ道を通すかは要件で決める（本番の出る記録が変わらないことはテストで固定する）。

## Scope

- **In**: 子のスレッドへの受け手の引き継ぎ（または同じ働きの仕組み）・それを固める檻（窓の中で起こしたアクターの記録が入る・窓の外の記録は入らない・本番の形では出る記録が変わらない）・効いていることの対照として `desk_overwrite_tests.rs` の手がかりに背景のスレッドの warn 以上を足す。
- **Out**: 既存のテストの代わりの判定の書き換え（残してよい）・全スレッドの捕捉の窓口の変更・本番のログの出先と文言。

## Boundary Candidates

- `areka-actor` の `spawn_actor`（とフックの形）
- log-capture-kit 側に要る口（あれば）

## Out of Boundary

- 記録の文言・水準の変更
- `thread_roles` の役割名の宣言

## Upstream / Downstream

- **Upstream**: なし（`areka-actor`・`log-capture-kit` は在る）。
- **Downstream**: 背景のスレッドの記録を見たいテストすべて（install・update・kanade・SHIORI の結線のテスト）。`areka-test-binary-stability` の件 2 の赤の読み分けも楽になる。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-test-cage-determinism`（log-capture-kit）・完了 `areka-P0-draw-load-parity`（走り始めのフック）。
- **Adjacent**: `areka-actor` に触る spec とは同時に走らせない（2026-10-10 の時点で、brief に `areka-actor` を挙げる起票済みの spec は無い）。`areka-test-binary-stability` の件 2（同じテストを読む）。

## Constraints

- 本番の振る舞いと出る記録を変えない。
- 1 ファイル 1,000 行の番人を守る。
- テストの実行ファイルの全スレッドで `tracing::enabled!` を真にする形（全体の設定）は取らない。
- 規模の見立て: S（3〜6）。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 起票（10-10）から main は動いていない。`spawn_actor`（`crates/areka-actor/src/spawn.rs`・360 行）を呼ぶ本番のファイルは 11 本（`areka` の `install/worker.rs`・`update/worker.rs`、kanade の `actor.rs`・`shiori/real.rs`、`areka-ghost` の `dispatcher.rs`・`relay.rs`・`ticker.rs`、`areka-sakura` の `drive.rs`、`areka-seriko` の `actor.rs`、`areka-sylphya` の `actor.rs`、`areka-mcp` の `server.rs`）。走り始めのフックを入れるのは今も `crates/areka/src/thread_roles.rs` の 1 か所だけ。
- **触るファイル**: `crates/areka-actor/src/spawn.rs` と兄弟のテスト（今ある `spawn_hook_tests.rs` 60 行か、新しい 1 本）・口が要れば `crates/log-capture-kit/src/capture.rs`（107 行）・対照として `crates/areka/src/install/desk_overwrite_tests.rs`（686 行）の手がかりを集める関数。
- **規模**: S（4〜6）。
- **先に要るもの**: なし。ファイルの重なり: `desk_overwrite_tests.rs` は `install-live-target-hazards` が挙げる。`areka-actor` を触ると書く未完了の brief はほかに無い。
- **測定の仕事ではない**（重い回 0。静かな机の全体テスト 1 回で確かめる）。
- **優先度の区分**: C（テストの道具の改善・本番の記録は変えない）。
- **要件定義のモデル**: Fable（スレッドをまたぐ受け手の引き継ぎ・窓が閉じた後に子が出す記録の扱い＝並行）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 背景のスレッドは `spawn_actor` だけで起きていない。素のスレッドで起こす所が本番に 7 か所ある（`crates/areka/src/install/desk.rs`・`install/fetch_url.rs`・`emo2_boot/switch_assets.rs`・`mcp/dump_surface.rs`・`readme/opener.rs`・`shiori_host.rs`、kanade の `shiori/probe.rs`）。`spawn_actor` だけを直しても、これらの記録はテストの捕捉に入らない＝範囲に入れるか（同じ引き継ぎを通す小さな共通の起こし方を作るか）を要件で決める。

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）

- 前提の変化: 記録の捕捉の部品（`crates/log-capture-kit/`）とアクターの起こし方（`crates/areka-actor/src/spawn.rs`・360 行）は、棚卸㉓から 1 行も動いていない。走り始めの差し込み口は今も「アクター名だけを受ける関数・プロセスに 1 度だけ」で、本番の役割名の登録（`crates/areka/src/thread_roles.rs`）が使っている。捕捉は今も呼んだスレッドの中だけで受け手を差し替える形（`capture.rs` の 66 行目）で、窓の中で書き込みの注目を作り直す（69 行目）。
- インストールの側は `baseware-root-list` で大きく動いた（`install/` の 13 ファイル・置き場を決める `destination.rs` を新しく足した）。対照に挙げた `install/desk_overwrite_tests.rs`（686 行）は動いていない。
- 素のスレッドで起こす本番の所が 1 つ増えた: 置き場の並びの係（`crates/areka/src/roots/desk.rs` の 101 行目・`baseware-root-list` が作った）。棚卸㉓の 7 か所と合わせて 8 か所。ほかにゴーストの中継（`areka-ghost` の `relay.rs`）と sylphya の写し（`mirror.rs`）にも素のスレッドが在る。
- 触るファイル（アクターだけに絞る形）: `crates/areka-actor/src/spawn.rs` と、兄弟の新しいテストのファイル 1 本（今ある `spawn_hook_tests.rs` 60 行へ足してもよい）。要るときだけ `crates/log-capture-kit/src/capture.rs`（107 行）。
- 規模: アクターだけに絞れば 3〜5 タスク。素のスレッドまで広げると 8〜12 で、`install/desk.rs`・`readme/opener.rs`・`roots/desk.rs`・`shiori_host.rs` などに触る。新しい依存は要らない（`areka-actor` は既に記録の部品を使っている）。人の手の確かめ・静かな机・前後の全体テストの比べは要らない（最後の全体テスト 1 回）。
- 先に要るもの: なし（先頭）。
- 今の版の 2 本との重なり: アクターだけに絞り、効いていることの対照を `areka-actor` の中の新しいテストで示すなら 0。brief の対照（`install/desk_overwrite_tests.rs` に手がかりを足す）を取ると、`user-data-root` が触る `install/desk.rs`（展開器へ作業の根を渡す所）の兄弟のテストに触る＝同じウェーブに置けない。＝今の版の席に入れるなら「アクターだけ・対照は自分のクレートの中」の形に限る。その形の上限 5 は `user-data-root` の下限 12 と `write-failure-notice` の下限 6 のどちらより小さい。
- 気をつけること: アクターのスレッドが捕捉の窓の受け手を引き継ぐと、今まで窓に入らなかった背景の記録が入るようになる。「記録が無いこと」を確かめている既存のテストが赤に変わりうる＝最後の全体テストで洗う。
- 優先度の区分: C（テストの道具の改善・本番の記録は変えない）。
- 要件定義のモデル: Opus に改める（棚卸㉓は Fable）。呼んだスレッドの受け手を取って子のスレッドで既定にするのは記録の部品の決まった使い方で、新しい並行や順の保証を設計するのではない。窓が閉じた後の記録の扱いも、受け手の寿命の決めで足りる。
- 分割の案: なし。素のスレッドまで広げる部分は、要るなら今の版の後に別の 1 本で足す。
- 見つけた穴・古くなった記述: 棚卸㉓の「素のスレッドは 7 か所」は 8 か所になった（上の `roots/desk.rs`）。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: アクターの起こし方 `crates/areka-actor/src/spawn.rs`（360）・兄弟の `spawn_hook_tests.rs`（60）・捕捉の `crates/log-capture-kit/src/capture.rs`（107）は 10-16 から変わらない。log-capture-kit で動いたのはテストの番人 2 本だけ（`tests/sample_path_guard_test.rs`＝`real-run-mcp-kit`・`tests/file_length_guard_test.rs`＝`mcp-reload`）で、本 spec の触る所ではない。`spawn_actor` の本番の呼び手は 11 ファイルのまま。素のスレッドで起こす本番の所は 8→11 か所（`crates/areka/src/emo2_boot/switch_copy.rs`・`emo2_boot/spine_wait.rs`・`perf_thread_report.rs` が増えた・ほかに sylphya の `mirror.rs`）。対照の `crates/areka/src/install/desk_overwrite_tests.rs` は 688 行。
- 触るファイル（アクターだけに絞る形）: `crates/areka-actor/src/spawn.rs` と兄弟の新しいテスト 1 本（`spawn_hook_tests.rs` へ足してもよい）・要るときだけ `capture.rs`。
- 規模: アクターだけ 3〜5／素のスレッドまで 8〜12（棚卸㉖のまま）。
- 先に要るもの: なし（先頭）。v0.1.0 との重なり: アクターだけ・対照を自分のクレートの中で示す形なら 0（`areka-actor` と `log-capture-kit/src/` を挙げる v0.1.0 の brief は無い。`pasta-in-windows-verification`・`dot-sakura-specimen` は log-capture-kit の `tests/sample_path_guard_test.rs` を触るが別のファイル）。依存は足さない＝1 本だけの席は取らない。静かな机・人の待ち・前後の全体テストの比べは要らない（最後の全体テスト 1 回で、「記録が無いこと」を見る既存のテストが赤に変わらないかを洗う）。版に縛らない列として隙間に入れるなら、この形（上限 5）に限る。
- 優先度の区分: C のまま（テストの道具の改善・本番の記録は変えない）。
- 要件定義のモデル: Opus のまま（呼んだスレッドの受け手を子のスレッドで既定にするのは記録の部品の決まった使い方＝新しい並行や順の保証を設計するのではない）。
- 分割の案: なし（素のスレッドまで広げる分は、要るなら後で別の 1 本）。
- 見つけた穴・古くなった記述: 棚卸㉖の「素のスレッドは 8 か所」は 11 か所。
