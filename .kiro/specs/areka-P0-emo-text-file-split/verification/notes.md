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
| `balloon_visibility.rs` | 型 `ContentDecisions` を子 `decision` でなく親に置いた（`pub(super)` なし・基準の字面のまま） | 兄弟 `wait` の `decide_timeout` が引数で受けて欄を読むため。当初は子 `decision` に置いて `pub(super)` を 3 つ付けたが、子同士が引き合う形になったので完了時に親へ戻した（§4） |

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

## 3. 実機の確かめ（タスク 5.2・要件 8）

- ビルド: 分割後の HEAD `66d52b2c` の `cargo build -p areka`（debug）。検体は `nar-sample-path emo2` の `folder=`／`balloon.emo2-kakukaku=` の絶対パス（どちらも `target\nar-samples\manual\emo2\` の下）を引数で渡した（引数起動＝記憶と起動中の印に触れない）。
- 置き場: ログ・作業フォルダ・`TEMP`／`TMP` はすべて `target\emo-text-file-split\real\` の下（要件 8.3）。生ログはコミットしない。
- 環境: `RUST_LOG=info,areka_emo_text::actor=debug,areka::input_events::balloon=debug,areka::emo2_boot::balloon_visibility=debug`（2 回目は `areka::input_events::user_break=debug` も）・`NO_COLOR=1`・`AREKA_APP_SMOKE_EXIT_MS=600000`（有界の自動終了 10 分）・`AREKA_BALLOON_TIMEOUT_MS=8000`（時間切れを見やすくするため 8 秒）。
- 操作: 選択肢のホバーとクリック・ダブルクリックは開発者が手で行った（画面操作の道具の許可は下りなかった）。

| 走行 | pid | 結果 |
|---|---|---|
| run0 | 8444 | 実行体の隣の helper が 64bit 版（ワークスペースのビルドが上書き）で `pasta.dll` の LOAD が `0x800700C1`。分割とは無関係の既知の準備漏れ（`transition_judge_offset_signoff_tests.rs` §2.2）。i686 の helper を隣へ写して起こし直した |
| run1 | 2908 | 10 分で自動終了・ERROR 0 件。文字の表示・ホバー・時間切れを確認。ダブルクリックは体の上（`OnMouseDoubleClick`＝メニュー）とバルーンの上（話の無いときの `user_break` で閉じる）だった |
| run2 | 14404 | 10 分で自動終了（`event="app_exit" origin=Smoke`・`ghost shutdown sequence completed`）・ERROR 0 件。4 つの振る舞いすべてを確認 |

### 4 つの振る舞い（要件 8.1）——いずれも分割前と同じ

| 振る舞い | 開発者の目視 | ログの抜粋（run2・時刻は UTC 04 時台の分:秒） |
|---|---|---|
| 会話の文字の表示 | 表示された | `49:19.86 areka_emo_text::actor::present: テキスト供給面を予約スロットへ装着した … actor=0`／`49:22.37 … actor=1` |
| 選択肢のホバーとクリック | 「大丈夫」 | `49:30.13 areka::input_events::balloon::moved: hover 遷移を上流 runtime へ注入 event="choice_hover_inject" scope=1 ordinal=Some(1)`／`49:30.65 areka::input_events::balloon::pressed: 選択確定: ChoiceSelection を発行 event="choice_selected" scope=1 id=Onおしゃべり頻度メニュー`（以降メニューを 4 段たどった） |
| ダブルクリックでの中断 | 「消える」 | `50:34.39 kanade: 利用者の中断を受け入れた——現行のトークを止める event="balloon_break_accepted" scope=0 talk_id=10`／同時刻 `balloon_visibility::phase: … trigger="user_break" visible=false`（scope 0・1） |
| バルーンの表示と非表示（時間切れを含む） | 消えた | `49:19.18 areka::emo2_boot::balloon_visibility::wait: バルーン非表示までの待ち時間を確定 env="AREKA_BALLOON_TIMEOUT_MS" timeout_secs=8.0`／`50:01.12 … trigger="timeout" visible=false`／`51:48.71 … 抑止が成立しているためタイムアウトによる非表示を見送った hover=true choice=true` |

### 子の道筋のログ（要件 2.4）

前置きの絞り込み 3 本で、子の道筋の行が拾えている（run2 の件数）: `areka_emo_text::actor::attach` 57,581（毎フレームの再追従の debug・文言は分割前と同じ）・`::actor::present` 2・`::actor::decoration` 2・`areka::input_events::balloon::moved` 30・`::pressed` 13・`areka::emo2_boot::balloon_visibility::wait` 1・`::phase` 146（run1 では `::balloon::exit` 1 も）。

### 持ち越しの議題（承認フローで扱う・本 spec の範囲外）

- **選択を待っているメニューを、なでなで（`OnMouseMove`）のような軽いイベントの返事で上書きしてよいのか**（2026-10-03 開発者の指示で記録）。
  - 観測（run2・`50:15.54`〜`50:16.07`）: 体のダブルクリックでメニューの話（`talk_id=8 origin="OnMouseDoubleClick"`）が始まった 0.5 秒後、体の上のマウスの動きで `OnMouseMove` の返事が届き、`event="steady_talk_replace" talk_id=9 origin="OnMouseMove"` でメニューが置き換わった。開発者には「メニューを出した後に後続のトークが止まってないときがある」と見えた。
  - 同じ形: `51:38.20` に始まった `OnSecondChange` の話を、`51:38.36` の体のダブルクリック（メニュー）が置き換えた。
  - 分割との関係: 無い。決めているのは運行（kanade）の「話の最中に新しい返事が来たら差し替える」（単一 slot 置換）で、`crates/areka-kanade/`・`OnMouseMove` を送る側（`input_events/mod.rs`・`input_events/throttle.rs`・`emo2_boot/hit_region.rs`）・`crates/areka-ghost/` の基準 `94e3ad78` からの差分は 0 行。
  - 次の一歩: 正典（ukadoc／SSP の話の最中・選択待ちの最中のイベントの扱い）と照らして、バグとして `/kiro-discovery` で起票するかを承認フローで決める。

## 4. 機能全体の検証（`/kiro-validate-impl`・2026-10-03）

- 判定: **GO**（Critical 0・Warning 0）。`9ba1f380` 以降はコードの差分 0 行（spec 文書だけ）なので、§2 の全体テストの緑は HEAD にそのまま当てはまる。実機は §3。
- 横断の確認: `lib.rs` の 2 つの一覧（`PURE_SOURCES` 65 件・`SOURCES_OUTSIDE_THE_PURE_SCAN` 36 件）の和が `src/*.rs` の実ファイル 102 本と一致・重複 0。構造テスト 3 つの最終形は設計 §構造テストの追随 と一致。設計と違う箇所はすべて §2 の表と `tasks.md` の Implementation Notes に記録がある。

### Info（完了を止めない・承認フローで扱う）

- **`decision` と `wait` の相互参照**（完了時にその場で解決）: 当初は `balloon_visibility_wait.rs` が `use super::decision::ContentDecisions;` で兄弟を引いていた。型 `ContentDecisions` を親 `balloon_visibility.rs` へ戻し（基準の字面のまま・`pub(super)` 3 つを撤去）、子 2 本は `use super::{…, ContentDecisions, …}` で親から引く形にした（「定義は元・処理は子」）。`cargo test -p areka balloon_visibility` 88 passed・警告 0。
- **`region_tests.rs` に `//!` が無い**: 要件 7.3（字下げ以外 1 文字も変えない）を守るための意図した例外（設計の region の表に「足す予定の spec」の列が無く、タスク 4 も `region_tests` を除く）。要件 1.4 の例外であることをここに記す。
- **`lib.rs` の doc の訂正**: 層規律の段落の「`#[cfg(test)] mod layer_discipline` 内」を実体の `mod tests` に直した（基準からの誤記・タスク 4 のコミット `9ba1f380`）。
