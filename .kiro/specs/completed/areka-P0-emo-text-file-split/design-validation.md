# 設計レビュー報告: areka-P0-emo-text-file-split

> 実施 2026-10-03／対象 `design.md`（基準 HEAD `71be5003`・ブランチ `claude/areka-p0-emo-text-split-dc4e04`）
> 進め方: `kiro-validate-design` の手順（分析 → 重大な問題 ≤3 → 強み → GO/NO-GO）。設計の主張はすべて実ソースを Read/Grep で突き合わせた（下の「確かめた主張」）。非対話で実施し、開発者への質問はしていない。

## レビューの要約

設計は「定義は元に残し、処理（`impl` の塊と関数）を子へ出す」型で 7 本を分け、唯一の例外 `layout_inner` を走査の状態 `Scan` と分岐ごとの関数へ割る。構造テスト 5 つの読む範囲・母数・登記の差し替え、`layout_inner` の 19 分岐に通る既存テスト、名前の衝突、触ってはいけない場所——いずれも実ソースで裏が取れ、設計の記述は正確である。見つかった問題は 1 件の可視性の書き漏れ（そのままではコンパイルできない）と、束ね直しの名前の一覧の不足で、どちらも設計の型を変えずに数行で直せる。

## 重大な問題（最大 3 件）

### 🔴 問題 1: `Scan::glyph` の可視性が足りない（設計の形のままでは E0624）

- **懸念**: §layout_inner の割り方 の形では、`layout_scan_glyph.rs`（`scan` の子 `mod glyph`）の `impl Scan { fn glyph(…) }` が私有のままで、呼び手の `layout_inner` は親 `scan` にある。Rust のメソッドの可視性は「`impl` を書いたモジュールとその子孫」なので、親からは私有の `glyph` を呼べない（`finish_line` などの逆向き——親の私有関数を子が呼ぶ——は問題ない）。§束ね直しの規則の末尾に挙げた `pub(super)` を付ける 9 項目にも `glyph` が無い。
- **影響**: 実装者が設計どおりに書くとコンパイルで止まる。Error Handling の「可視性が足りない → `pub(super)`」で直せるが、設計の正本に書いておかないとタスク生成とレビューが「設計どおり」を判定できない。
- **提案**: `fn glyph` を `pub(super) fn glyph` に改め、`pub(super)` を付ける項目の一覧へ `glyph` を足す（計 10 項目）。あわせて、親に残る `none_err`／`device_err` の `pub(super)` は不要（子孫は親の私有の項目をそのまま見える・`viewbox_draw` の親＝crate ルートへ見える範囲を広げるだけ）なので、一覧から外して私有のままにするか、「付けない」と明記する。
- **トレーサビリティ**: 要件 2.3（割り方を設計に記録）・4.5（コンパイル不能は束ね直しか `use` で解く）・3.1（可視性を上げない）
- **根拠**: design.md §layout_inner の割り方「形」のコード、§束ね直しの規則の末尾の段落。実ソース: `crates/areka-emo-text/src/layout.rs` の `fn layout_inner`（走査の駆動が親に残る）。

### 🟡 問題 2: 親で束ね直す名前の一覧が、`#[cfg(test)] use` と `pub(super)` 以外は規則だけで名指しされていない

- **懸念**: `present_frame` は `pub fn` の自由関数で、`crates/areka/src/emo2_boot/frame/wiring.rs`・`frame/scale_text.rs`・`examples/`・`tests/` が `areka_emo_text::actor::present_frame` で呼ぶ（Grep で確認）。子 `actor_present.rs` へ出すなら親に `pub use present::present_frame;` が要る。§束ね直しの規則の表は「元の外の本番コードが使う名前は `pub use`」と規則を書いているが、名指しの一覧には `pub(crate) use decision::decide;`・`pub(crate) use wait::configured_timeout_secs;`・ハンドラ 3 本の素の `use` はあって、`present_frame` が無い。
- **影響**: 要件 3.3（呼び出し側 0 行）の成否が「規則を読んで気付くか」に掛かる。タスク生成とレビューが機械で突き合わせられる一覧が無い。
- **提案**: §束ね直しの規則の末尾に「親ファイルごとの束ね直しの全行」を 1 つの表にする（`actor.rs`: `pub use present::present_frame;`／`balloon.rs`: `use moved::…; use pressed::…; use exit::…; use super::user_break;`／`viewbox.rs`: `use diff::…`（`is_backward_shrink` はメソッドなので不要）＋`#[cfg(test)] use diff::line_fingerprint;`／`balloon_visibility.rs`: `pub(crate) use decision::decide; pub(crate) use wait::configured_timeout_secs; #[cfg(test)] use wait::{resolve_timeout_secs, parse_timeout_ms};`／`layout.rs`: なし（`layout_inner` はメソッド）／`viewbox_draw.rs`: なし（`render_styled` はメソッド））。
- **トレーサビリティ**: 要件 3.1・3.3・2.7
- **根拠**: design.md §束ね直しの規則、§File Structure Plan の `actor.rs` の表（`present_frame` を子へ）。実ソース: `crates/areka-emo-text/src/actor.rs` の `pub fn present_frame`。

（3 件目に当たる問題は無い。）

## 設計の強み

1. **構造テストの追随が数の単位で裏取りされている。** `layout_cursor_overflow_tests.rs` の `finish_line(` 4／`finish_pending_line(` 3 は、分割後の `glyph`（呼び出し 1＋1）・`line_break`（1）・`finish`（1）・`finish_pending_line` の定義と中の呼び出しで実際に 4／3 になる。`lib.rs` の母数 59 → 65（純粋層へ 6 件）＋読まない一覧へ 1 件は、新しいファイル 7 本と一致する。`frame_attach_tests.rs` の `ACTOR_SCAN_SITES` は、`state.actors()` の走査が `present_frame` にしか無いことを実ソースで確認でき、差し替え後の並び順（`crates/areka-emo-text/…` が先）も `sort()` の結果と一致する。
2. **「定義は元・処理は子」の型で可視性の付与を最小にしている。** `Scan` が自己参照にならないこと（`LineHeights` は参照を持たない・`WrapPlan` は `Copy`）、`TimeoutSource::as_str`・`TIMEOUT_ENV_KEY`・`TextLayerRuntime` の私有の欄を子孫がそのまま読めることまで、Rust の可視性の決まりに沿って組まれている。19 分岐すべてに名指しされた既存テストが実在する（20 本を Grep で確認）ので「足すテスト 0 本」も根拠がある。

## 最終判定

**判定: GO**

**理由**: 設計の主張はコードと一致しており、残る問題 1 は 1 語（`pub(super)`）の追記、問題 2 は一覧の補完で、設計の型・ファイルの切れ目・構造テストの追随・証跡の採り方のどれも変わらない。設計ディスカッションでこの 2 点を design.md に反映すれば、タスク生成へ進める。

**次の段**:
1. 設計ディスカッションで問題 1・2 を design.md に反映する（§layout_inner の割り方・§束ね直しの規則）。
2. `/kiro-spec-tasks areka-P0-emo-text-file-split` でタスクを生成する。

## 確かめた主張（実ソースとの突き合わせの記録）

| 設計の主張 | 確かめ方 | 結果 |
|---|---|---|
| 7 本の行数 975／977／871／914／930／923／977 | `Get-Content` の行数 | 一致 |
| `layout_inner` は引数 9・`match` の腕 3（文字・改行・`\_l`）＋最終行の確定 | `layout.rs` の `fn layout_inner` を全文読み | 一致。`break` は文字の腕の先頭 1 か所のみ |
| `finish_line(` 4・`finish_pending_line(` 3・`fn finish_pending_line(` を含む | `layout.rs` を Grep | 定義 1＋呼び出し 3／定義 1＋呼び出し 2。分割後は `layout_scan.rs`＋`layout_scan_glyph.rs` の連結で同じ数 |
| `concat!(include_str!, include_str!)` が 1 つの定数になる | research §10.1（設計時に `rustc` で実測） | 記録あり |
| `lib.rs` の `PURE_SOURCES` が 59・`SOURCES_OUTSIDE_THE_PURE_SCAN` に `viewbox_draw.rs` 系 | `lib.rs` の `assert_eq!(PURE_SOURCES.len(), 59, …)` と 2 つの一覧 | 一致。新しい 7 本の内訳（純粋層 6＋読まない 1）も一致 |
| `actor.rs`・`layout.rs`・`viewbox.rs`・`region.rs` に `windows` の参照 0 | Grep | 一致（`actor.rs` は `wintf` のみ） |
| `layout_styled_tests.rs` の `SOURCES` が 3 件・`metrics.line_pitch(` 1 回・`line_gap` 0 回 | 当該テストを読み | 一致。`layout_inner` は `line_pitch_of` 経由のみ |
| `frame_attach_tests.rs` の `ACTOR_SCAN_SITES` 3 件・`state.actors()` の走査は `present_frame` だけ | 当該テストと `actor.rs` を読み | 一致（`actor.rs` の `.actors()` は `present_frame` の 1 か所＋`present_actor` の注釈 1 行。どちらも子へ移る） |
| `draw_format_metrics_tests.rs` は `draw` で始まる名前だけに反応 | `name.starts_with("draw")` | 一致 |
| 19 分岐それぞれに既存テスト | §Supporting References の代表名 20 本を Grep | 20 本すべて実在 |
| `Scan` が自己参照にならない | `LineHeights` の定義（`layout_styled.rs`・寿命パラメータ無し）・`WrapPlan` の `derive(Clone, Copy)` | 問題なし |
| `parse_timeout_ms` の本番の消費者は `resolve_timeout_secs` だけ | `crates/` を Grep | 一致。テストは `balloon_visibility_timeout_config_tests.rs` のみ |
| `line_fingerprint` はテスト 2 本が `use super::{…, line_fingerprint}` で引く | Grep | 一致（`viewbox_choice_marker_tests.rs`・`viewbox_style_fingerprint_tests.rs`） |
| `observe_suppression` の呼び手は `decide_timeout` | Grep | 一致（同じ子 `wait` の中に収まる） |
| ハンドラ 3 本の消費者は `balloon.rs` の木の中だけ | Grep | 一致（`user_break.rs` は名前の言及のみ） |
| `present_frame` の消費者 | Grep | `frame/wiring.rs`・`frame/scale_text.rs`・examples・`tests/` → 問題 2 |
| 新しい stem と既存ファイルの頭の重なり | 3 ディレクトリの一覧 | 重なりなし。`viewbox_draw_frame_render_tests.rs` は `viewbox_draw_render` を頭に持たない |
| 触ってはいけない場所（`frame/`・`placement/`・`install/`・`main.rs`・`areka-sakura`・`wintf`・`areka-kanade`・`user_break_cue.rs`・`balloon_visibility_phase.rs`・`update/`）が File Structure Plan と Modified Files に無い | 両節を読み | なし（`frame_attach_tests.rs` は `emo2_boot/` 直下） |
| 番人の例外の表に 7 本は無く、設計も触らない | `file_length_guard_test.rs` を Grep | 7 本の名前は無し・設計の変更一覧にも無し |
| `region.rs` の内蔵テストは 489 行目から | `region.rs` を読み | `#[cfg(test)] mod tests {` が 489〜490 行目 |
| 要件 1.1〜8.3 の 40 件が設計の要素へ対応 | §Requirements Traceability | すべて具体的な節か機構を指す（1.2・7.4 は「該当なし」で明示） |
