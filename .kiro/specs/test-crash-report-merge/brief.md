# Brief: test-crash-report-merge

> 2026-11-20 棚卸㊵（`/kiro-next`）で起票。出どころ＝✅ `audit-emo2-boot-spine`（PR#476）の範囲の外で見つけたことの「エ」（`.kiro/specs/completed/audit-emo2-boot-spine/research.md` の 6 節と 873 行目＝「持ち主の spec が無い。完了のときに起票か扱いを決める」）。完了の記録（tasks.md の Implementation Notes と PR の本文）は起票 0 と書き、この件に触れずに閉じていた＝棚卸の残り物の仕分けで拾った。見直しの指摘は全部直す（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）ので、見直しの組（版の段の「v0.1.0 の後の隙間」）に置く。

## Problem

テストの実行ファイルが落ちたときに、落ちた場所とスレッドの一覧を書き残す「落ちの報告」の仕組みが、ほぼ同じ写しで 3 本ある（約 500 行ずつ・計 1,579 行）。違いは、許可の持ち手の表を持つかどうかと、先頭の説明の数行だけ。1 本を直すと残りの 2 本も同じように直す必要があり、直し忘れが黙って食い違う。windows-rs の移行（v0.2.0）でも同じ書き換えを 3 回することになる。

## Current State

- `crates/wintf/src/test_crash_report.rs`（494 行・`lib.rs` 7 行の `mod test_crash_report;`）
- `crates/areka-placement/src/test_crash_report.rs`（494 行・`lib.rs` 90 行の `mod test_crash_report;`）＝wintf の写しと先頭の説明 2 行だけが違う
- `crates/areka-runtime/src/crash_report_test_support.rs`（591 行・`lib.rs` 71 行の `pub mod`・許可の持ち手の表を持つ）。使い手＝`areka-runtime` の `spine_wait.rs`・`wire_tests.rs`・`assets*_tests.rs` と、本体の `crates/areka/src/runtime_tests/mod.rs` 12・17 行
- 3 本とも `windows` クレートの Win32 の関数（`CloseHandle`・`CoIncrementMTAUsage`・`LibraryLoader` ほか）を直に呼ぶ。
- 3 つの crate はどれも、ワークスペースの中の crate に依存しない leaf の `log-capture-kit` を開発用の依存に持つ（wintf は `features = ["env-filter"]`）。`log-capture-kit` の本番の依存は `tracing` だけ。

## Desired Outcome

落ちの報告の本体が 1 か所にだけ在り、3 つの crate はそれを呼ぶだけになる。許可の持ち手の表は、使う crate（`areka-runtime`）だけが持つ形で残る。テストの振る舞い（落ちたときに書く中身・書く先）は変わらない。

## Approach

- 置き場の候補は 2 つ（要件の段で決める）: ⑴ `log-capture-kit` に入れる（新しいファイル・任意の feature の後ろに `windows` の依存を置き、既定では引かない）。⑵ テスト専用の新しい leaf の crate を足す（`publish = false`・`windows` だけに依存）。どちらも wintf が上位の crate に依存しない向きを守る。推す＝⑴（新しい crate を増やさない）。
- 許可の持ち手の表は、共通の本体に「表を差し込む口」を足すか、`areka-runtime` の側に薄く残すかを設計の段で決める。
- 同じ見直しの「カ」の残り（`carrier_cue` の写しのうち、依存の向きで共有できなかった `crates/areka-ghost/src/prop_sink.rs`・`crates/areka-seriko/src/actor_test_support.rs`）を同じ置き場へ寄せられるかを要件の段で確かめる（寄せられなければ範囲の外のまま）。

## Scope

- **In**: 3 本の写しを 1 本にまとめる・3 つの crate の `mod` の宣言と使い手の `use` の付け替え・置き場の crate の `Cargo.toml`（と依存を足すなら `Cargo.lock`）・まとめたことを確かめるテスト。
- **Out**: 落ちの報告の中身や書く先を変えること・`LocalFree` を呼ばない件（`crash_report_test_support.rs` 436 行の意図的な先送り・`audit-emo2-boot-spine` の research 5 節 ⑵）・windows-rs の移行そのもの。

## Boundary Candidates

- 置き場の crate（`log-capture-kit` か新しい leaf）
- 使い手の 3 crate（wintf・areka-placement・areka-runtime）と本体の `runtime_tests/mod.rs`

## Out of Boundary

- 1,000 行の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表には触れない（まとめた本体は 600 行未満の見込み）。

## Upstream

- なし（✅ `audit-emo2-boot-spine`・✅ `audit-test-kits` は着地済み）。

## Downstream

- windows-rs の移行の本体（まだ起票していない）＝書き換えが 3 回から 1 回になる。

## Existing Spec Touchpoints

- `input-crate-split`（`log-capture-kit/src/event.rs` と `Cargo.lock` を触る）・`session-module-gather`（本体の `runtime_tests/` のパスを直す）・`audit-emo2-boot-frame`（`areka-runtime` の `frame*` と本体の `runtime_tests/frame*`）と触るファイルが近い＝同じウェーブに並べるときは突き合わせる。

## Constraints

- 依存を足すなら 1 ウェーブに 1 本だけの席（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md`）を使う。
- ワークスペースの依存の向き（wintf は上位の crate に依存しない）を壊さない。

## 2026-11-20 棚卸㊵の測り（main 67e0c70e）

- 触るファイル: 上の Current State の 3 本と各 `lib.rs`・`areka-runtime` の使い手 4 ファイル・本体の `runtime_tests/mod.rs`・置き場の crate の `Cargo.toml` と新しいファイル（⑴ なら `crates/log-capture-kit/src/` の新しいファイルと `lib.rs`）・`Cargo.lock`（依存を足すとき）。
- 規模: S（4〜8 タスクの見込み）。
- 先に要るもの: 働きの依存は無い。`Cargo.lock` の席が空くウェーブ（見込みの C23 は `input-crate-split` が使う＝C24 以降）。
- 優先度の区分: A（見直しの指摘の取り残し・開発者「指摘事項は全部直す」）。
- 要件定義のモデル: Opus（前例＝`log-capture-kit` が leaf として wintf と上位の crate に同じ仕組みを配っている形）。
- 議題: 置き場（⑴ か ⑵）・許可の持ち手の表の置き方・`carrier_cue` の残りを寄せるか。

## audit-emo2-boot-frame からの申し送り（2026-11-21）

`audit-emo2-boot-frame` の範囲の外（`areka-placement` は C23 のあいだ触れない）で見つけた、古い注と写しの件（`.kiro/specs/audit-emo2-boot-frame/research.md` の 6 節イ〜カと 12.5）。この spec が `areka-placement` に触るときにまとめて直す。

- **イ**: `crates/areka-placement/src/follow/window_move.rs` の 666 行付近の「見送りの 4 点」は今は 5 点。
- **ウ**: `crates/areka-placement/src/follow_window_move_hold_watch_tests.rs` の 35 行付近の `INVARIANT_WARNING` は手写し。在ることを確かめる側が無い。
- **エ**: `config.rs`・`follow/work_area.rs`・`follow_visibility_balloon_wiring_tests.rs` の古い道 `runtime::frame::…`・「本体の」は、今は `areka_runtime::frame::…`。
- **オ**: `crates/areka-placement/src/test_support.rs` の `code_only` は `source_text_test_support::code_only` の写し。
- **カ**: `crates/areka-placement/src/follow_window_move_diag_tests.rs` の `window_move_lines` は、`areka-runtime` の `frame_test_support.rs` と同じ働き（上流の crate の `test_support` へ移せば両方が 1 つを使える）。
- **移したテストの置き場の古い注**: `crates/areka-placement/src/chain_rule_tests.rs` の 5 行目が、移す前の置き場（本体の `frame_chain_rule_tests.rs`）を指している。今は `crates/areka-runtime/src/frame_chain_rule_tests.rs`。
- **武装の説明の古い注**: `crates/areka-placement/src/chain_realign.rs` の 168〜170 行（`arm_chain_realign` の説明）の「寸変化を伴って書込を起こしたときに呼ぶ。判断（寸が変わったか・書込が起きたか）は呼び手が持つ」は、今の呼び手（`crates/areka-runtime/src/frame/dpi.rs`）が窓の種類と窓の寸の前後比べだけを見る形と合わない（`audit-emo2-boot-frame` のかけ直しで `wrote &&` を消した・research の 13.5 と 13.7）。
