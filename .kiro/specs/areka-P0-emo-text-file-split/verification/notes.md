# areka-P0-emo-text-file-split — 検証の記録

## 1. 基準（分割前）

- 基準 SHA: `94e3ad78220141e1ee4ed4d61b6ae94d82dabb3e`（実装の最初のコミットの直前の HEAD・対象 7 本は main `1ce4c74e` と同一）
- 採取日時: 2026-10-03
- `tools/test-all.ps1` の終了コード: **0**（i686 導入・i686 成果物ビルド・fmt --check・x64 ワークスペース全テスト・i686 テストの全段 緑）
- 生ログ: `target\emo-text-file-split\before_*`（コミットしない）

| 証跡 | 行数・件数 |
|---|---:|
| `before_list.txt` | 9,371 行 |
| `before_results.txt` | 9,371 行（FAILED 0・ignored 103） |
| `before_warnings.txt` | `cargo build --workspace` 0 件・全体テストのコンパイル出力 0 件 |

### 採り方（after も同じ手順）

1. `pwsh -NoProfile -File tools/test-all.ps1 *>&1 | Tee-Object target\emo-text-file-split\<phase>_testall.log`
2. 一覧: `cargo test --workspace -- --list` と `cargo test -p shiori-host32-helper -p shiori-host32-ipc --target i686-pc-windows-msvc -- --list` の stdout を結合し、`: test`／`: bench` で終わる行だけを序数（`[StringComparer]::Ordinal`）で整列。重複行は残す。
3. 結果: 1 の生ログから `^test .+ \.\.\. (ok|FAILED|ignored.*)$` の行だけを抜き、序数で整列。
4. 警告: `cargo build --workspace` の出力と 1 の生ログの、`warning` で始まる行の件数。

## 2. 分割後の採取と突き合わせ（タスク 5.1）

- 採取した HEAD: `9ba1f380`（タスク 4 のコミット）・採取日 2026-10-03・手順は §1 と同じ
- `tools/test-all.ps1` の終了コード: **0**（全段 緑・1,000 行の番人 `file_length_guard_test` を含む）

| 突き合わせ | 結果 |
|---|---|
| 一覧 `before_list.txt` ↔ `after_list.txt` | 前例の `Compare-TestLists.ps1`（対応表なし）で **PASS**・9,371 行 ↔ 9,371 行（相異なる名前 9,303 ↔ 9,303） |
| 結果 `before_results.txt` ↔ `after_results.txt` | `diff` の差分 **空**（9,371 行・FAILED 0・ignored 103 が前後で同じ） |
| 警告 | `cargo build --workspace` 0 → 0・全体テストのコンパイル出力 0 → 0（増えていない） |
| 足したテスト・消したテスト | 0 本・0 本（一覧が完全一致） |

### 差分の範囲（基準 `94e3ad78` から）

- 触ったファイルは設計の対象 7 本・新しい子 12 本＋`region_tests.rs`・Modified Files（`lib.rs`・`layout_cursor_overflow_tests.rs`・`layout_styled_tests.rs`・`frame_attach_tests.rs`・`structure.md`）・本 spec の `tasks.md` と `verification/` だけ。
- 差分 0 行を確かめた場所: `Cargo.toml`／`Cargo.lock`（全 crate）・番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`・`crates/areka-emo-text/src/state.rs`・`crates/areka-parsers/src/balloon/`・同じウェーブの他の spec の持ち場（`emo2_boot/frame/`・`placement/`・`install/`・`main.rs`・`areka-sakura/`・`wintf/`・`areka-kanade/`・`user_break_cue.rs`・`balloon_visibility_phase.rs`・`update/`）・各 crate の `examples/` と `tests/`。
- 発生元の名前（`"areka_emo_text::actor"` などの対象モジュールの道筋）を完全一致で判定する箇所: **0 件**（grep）。
- 対象のファイルを字面や道筋で名指しする見張り: 設計の 5 つ（うち `draw_format_metrics_tests.rs` は 0 行）と、変更 0 の `balloon_visibility_phase.rs` を読む既存の 2 つ（`balloon_visibility_phase_zorder_chain_tests.rs`・`tick_gate_config_producers_tests.rs`）だけ。数え直しで追加は **0 件**。

### 移動以外の差分（`--color-moved` の確かめ）

各タスクのレビューで、基準の本文と移した先を改行コードを除いて機械で突き合わせた。移動以外の差分は設計の「許す差分」（`use`・`mod` 宣言・`pub(super)`・`impl X {`／`}`・子の `//!` 2 行・整形の折り返し）と `layout_inner` の割り出しだけだった。設計の一覧に無かった次の差分は、いずれも本文を変えずにコンパイル・警告 0 を保つためのもので、レビューで許すと判定した:

| ファイル | 差分 | 理由 |
|---|---|---|
| `actor_attach.rs` | `#[cfg(doc)] use` 2 行 | 動かした doc コメントの内部リンクを前と同じに解決させる（rustdoc の警告 36 → 36） |
| `layout_scan.rs`／`layout_scan_glyph.rs` | 可視の打ち切りの `break` → `ControlFlow::Break(())`・`tracing` の欄の明示（`inline_pos = self.inline_pos`）・構造体の省略形の明示・`mut` の除去・`fn finish(mut self)` | `layout_inner` の割り出し（要件 2.3 の例外 1 件）。欄名・文言・評価順は不変 |
| `viewbox_draw_render.rs` | `#[cfg(test)] use super::none_err;`・`decoration`／`plan` の関数を名前で引く `use` | テストの枝だけが使う名前・本文が素の名前で呼ぶため |
| `region.rs` | 接続に `#[rustfmt::skip]` | 字下げを戻すと rustfmt が `count_warns(|| { … })` の波括弧を外したがり、整形すると要件 7.3 に反するため |
| `input_events/balloon.rs` | 既存の `use std::rc::Rc;`・`use areka_sakura::ActorKey;` に `#[cfg(test)]` | 兄弟テストが `use super::*;` で受け取るだけになったため |
| `balloon_visibility_decision.rs` | `ContentDecisions` と欄 `shown`・`cleared` に `pub(super)` | 兄弟 `wait` の `decide_timeout` が引数で受けて欄を読むため（設計 §Error Handling） |

### 行数（すべて 700 行以下・700 行超の記録は無し）

| ファイル | 行数 | ファイル | 行数 |
|---|---:|---|---:|
| `actor.rs` | 471 | `viewbox_draw.rs` | 368 |
| `actor_attach.rs` | 237 | `viewbox_draw_render.rs` | 571 |
| `actor_present.rs` | 303 | `region.rs` | 499 |
| `layout.rs` | 489 | `region_tests.rs` | 479 |
| `layout_scan.rs` | 428 | `input_events/balloon.rs` | 431 |
| `layout_scan_glyph.rs` | 174 | `balloon_moved.rs`／`balloon_pressed.rs`／`balloon_exit.rs` | 176／186／192 |
| `viewbox.rs` | 460 | `emo2_boot/balloon_visibility.rs` | 450 |
| `viewbox_diff.rs` | 429 | `balloon_visibility_decision.rs`／`balloon_visibility_wait.rs` | 268／234 |
