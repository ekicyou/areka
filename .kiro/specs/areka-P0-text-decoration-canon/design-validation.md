# 設計レビュー報告: areka-P0-text-decoration-canon

> 実施: 2026-09-12（非対話・設計フェーズ）。入力は `requirements.md`（16 要件＋付録 A/B）・`design.md`（1,025 行）・`research.md` §0〜§9・`brief.md`（末尾 2026-09-11 追記が範囲の正本）・steering（`structure.md`・`tech.md`・`logging.md`・`roadmap.md`・`workflow.md`・`focus.md`）。
> 開発者裁定（DirectWrite 標準機能のみ／`sub`・`sup`・`outline` は語彙のみ／フォントファイル不読／縦書きの線の側は DirectWrite 既定／無効表示の色は背景原点画素＋shell の式／`\f` は `CueCommand::Custom` に乗せる／装飾は並行ベクタ／`\c` は戻さず台本先頭は戻す／`draw.rs` 分割先行・例外表不変／`\f[height]` は `line_pitch` 1 点／`\f` 無し台本は同一）は再審議せず、設計がそれを守っているかだけを見た。
> 設計の主張のうちコードで確かめられるものは、すべて実ファイルを読んで照合した（下の「照合した事実」）。

---

## 1. 要約

設計は、開発者裁定 10 点をすべて守り、既存の構造（純粋層／COM 層／結線層の 3 層・`\!` の汎用キャリア・`#[path]` 兄弟ファイル・log-first）の延長として組まれている。要件 16 件は追跡表で全項目が部品と検証に対応づけられ、依存方向と 1,000 行の見張りも守れる見込みである。
ただし、コードと照合して**設計の主張がそのままでは成り立たない点が 2 つ**見つかった。どちらも構造の変更ではなく、分割の割り方と 1 か所の契約文の訂正で済むが、タスク生成の前に `design.md` を直しておかないと、最初の作業（分割）で既存テストが赤になり、解読テストの述語も赤になる。

**判定: GO（条件付き）**——下の重要な問題 2 件を `design.md` に反映してからタスク生成へ進む。

---

## 2. 照合した事実（設計の主張 × コード）

| 設計の主張 | 照合結果 | 所在 |
|---|---|---|
| `draw_format_metrics_tests.rs` が `include_str!("draw.rs")` で `try_create_format` 出現 3・`create_text_format` 本文の呼出 2・`for_mode` 出現 2 を固定 | **一致**。`try_create_format` は定義 1（:340）＋呼出 2（:313・:321）、`for_mode` は定義（:259）＋呼出（:335）で、いずれも D15 が `draw.rs` に残す部分にある | `crates/areka-emo-text/src/draw_format_metrics_tests.rs` :507・:594〜:698 |
| 同ファイルの字面検査は上の 1 本だけ | **不一致**——もう 1 本ある。`at_prefixed_font_name_generation_is_absent_from_production_source` は「`draw.rs` に `@` が 1 個以上ある」ことを空振り防止の対照として要求する。`draw.rs` の `@` は `DrawExecutor::render` の束縛パターン `seam @ (ResidentContent::Image(_) \| …)` の **1 か所だけ**（:788）で、D15 はこれを `draw_oracle.rs` へ出す | 同 :548〜:560・`draw.rs` :788 → **重要な問題 1** |
| `layout_cursor_overflow_tests.rs` が `layout.rs` 内の `finish_line(` 4・`finish_pending_line(` 3・`fn finish_pending_line(` を固定 | **一致**。出現は :485・:656・:792・:842 と :393・:527・:780 で、D16 が出す `segment_advance_sum`（:751）・`resolve_cursor_component`（:897）の中には無い | `crates/areka-emo-text/src/layout_cursor_overflow_tests.rs` :422〜:437 |
| `lib.rs` の `pure_layer_modules_have_no_windows_imports` は対象ファイルを配列で列挙する | **一致**（25 ファイル・`use windows`／`windows::`／`windows_core`／`windows_numerics`／`extern crate windows` を禁止）。新設 5 ファイルの追記が要る点も設計どおり | `crates/areka-emo-text/src/lib.rs` :174〜 |
| `ResolvedBalloonText::resolve` の既存署名は変えない（呼び手が多い） | **一致**。呼出は 19 ファイル・**52 か所**（`research.md` D19 の「26 か所」は少なく数えているが、署名を変えないので影響なし） | `grep "ResolvedBalloonText::resolve("` |
| lexer は角括弧内の `\` を `\]` のときだけ特別扱いする（`\![\f,…]` が同名になり得る） | **一致** | `crates/areka-parsers/src/sakura/lexer.rs` `fn scan_bracket_args` :279 |
| `\f[]` は lexer が `[""]` を返すので `args == [""]` | **不一致**。lexer は `[]` を「真の空・0 引数」として `args == []` を返す（既存テスト `empty_bracket_yields_no_args` が `\s[]` → `args: vec![]` を固定） | `lexer.rs` :263〜:271・`lexer_tests.rs` :79〜:87 → **重要な問題 2** |
| `BalloonScopeAssets { scope, emo_world, atlas, model }`・`connect_balloon_text(runtime, view, actor, model)` | **一致**。`attach.rs` :311 の分解束縛は `..` 付きなのでフィールド追加で壊れない。テストでの構築は `frame_test_support.rs` :72 と `input_events/balloon_test_support.rs` :65 の **2 か所**（設計が挙げる `assets_tests.rs` には構築が無い） | `crates/areka/src/emo2_boot/assets.rs` :112・:377／`frame/attach.rs` :408 |
| アトラスの面は `resolve(SetId(0), rel_path)` → `entry(id).placement`（`page`・`uv_rect`・`trim_offset`）→ `page(index).bytes`（premultiplied BGRA・`stride` 明示） | **一致** | `crates/areka-emo-atlas/src/table.rs` :61〜:90・:141〜:181 |
| `windows` 0.62.2 に `SetFontFamilyName`／`SetFontSize`／`SetFontWeight`／`SetFontStyle`／`SetUnderline`／`SetStrikethrough`／`SetDrawingEffect`／`GetSystemFontCollection`／`FindFamilyName` がある | **一致**（レジストリの `windows-0.62.2/src/Windows/Win32/Graphics/DirectWrite/mod.rs` で各 1 件以上） | `Cargo.toml` `windows-core = "0.62.2"` |
| `CueCommand::as_command_carrier()` は `Option<(&str, Vec<&str>)>` | **一致**（`apply_font_args(actor, &tokens)` の型と合う） | `crates/dola/src/cue/command.rs` :213 |
| `FONT_TAG_CARRIER` を `areka_sakura::contract` に置き、emo-text が参照する | **成立**。`areka-emo-text` は `areka-sakura` に依存済み | `crates/areka-emo-text/Cargo.toml` |
| `Clear`／`ClearAll` は `request_clear` で行レイアウトの店（`LineLayoutStore`）を空にする（装飾番号の振り直しと再利用鍵の前提） | **一致**。`actor.rs` :485・:499 → `ViewboxExecutor::request_clear` → `line_store.clear()`（`viewbox_draw.rs` :185） | — |
| `ActorTextState` の中身は `items`・`reveal`・`choices`（`clear_content()` の対象） | **一致**（3 フィールドのみ。`Clear` の腕は :406 で構造体ごと `default()`） | `crates/areka-emo-text/src/state.rs` :325 |
| `cached_probe_count`・`line_layout_creations` は `#[cfg(test)]` の私有メソッド（分割で `pub(super)` にする） | **一致**（:443・:908）。子モジュールへ出すと親配下のテストから見えなくなるので、設計どおり可視性の変更が要る | `draw.rs` |
| `viewbox_draw_frame_render_tests.rs` が `super::plan_inconsistency` を参照 | **一致**（:458）。ファサードに素の `use plan::{…}` を残せば届く | — |
| `model.cursor()` は `&BalloonCursor`・`ResolvedChoiceStyle::resolve(Option<&BalloonCursor>, color)`・`paint(color)` は `Option<(fill, text)>` | **一致** | `model.rs` :169・`choice.rs` :503・:543 |
| `LineLayoutStore::line_layout` は行送り軸の箱寸に `font.height` を渡す／`finish_line` の行矩形も `font_height` 丈 | **一致**（`draw.rs` :614〜:617・`layout.rs` :862〜:877）。既定だけの行では `block_extent(run.size)` と `font.height` が同じ値になる前提は成り立つ（下の細かい所見 2 を参照） | — |

---

## 3. 重要な問題（最大 3 件）

### 🔴 重要な問題 1: `draw.rs` の分割（D15）が、もう 1 本の字面検査を赤にする

- **問題**: 設計は「`draw.rs` を字面で読む検査は `font_family_reaches_directwrite_only_as_author_name_or_default_retry` の 1 本で、検査対象の定義を動かさなければ分割は緑」と述べる。しかし同じテストファイルの `at_prefixed_font_name_generation_is_absent_from_production_source` は、空振り防止のために **`draw.rs` に `@` が 1 個以上あること**を要求している。`draw.rs` の `@` は `DrawExecutor::render` の束縛パターン（:788）の **1 か所しか無く**、D15 はその `DrawExecutor` を `draw_oracle.rs` へ出す。分割した瞬間に `draw.rs` から `@` が消え、この検査が赤になる。
- **影響**: 要件 1.5「分割後に既存テストが変更なしで緑」が最初の作業で破れる。実装順 1（分割）の割り方が変わる。
- **提案**: `DrawExecutor`・`FormatKey`・`create_target_bitmap`・`none_err`（いずれも `#[cfg(test)]`）は **`draw.rs` に残し**、出すのは `DWriteMetrics`＋`measure_line_box_ratio`（→ `draw_metrics.rs`）と `CachedLineLayout`＋`LineLayoutStore`＋`measure_line_overhang`（→ `draw_line_store.rs`）の 2 束にする。残る `draw.rs` は約 650 行＋本仕様の追加約 45 行 ≈ 700 行で、上限 1,000 から十分に遠い。`draw_oracle.rs` は新設しない（File Structure Plan・Modified Files・§D の steering 反映からも外す）。分割の段階で当該テストを触る案は要件 1.5 に反するので採らない。
- **追跡**: 要件 1.1・1.5・1.6。
- **根拠**: `design.md` 「Existing Architecture Analysis」の「字面で守られている 2 ファイル」・「draw ファサードの分割（段階 1）」・`research.md` §9.1 表 1 行目と D15。コード: `crates/areka-emo-text/src/draw_format_metrics_tests.rs` の `at_prefixed_font_name_generation_is_absent_from_production_source`（`assert!(src.contains('@'))`）・`crates/areka-emo-text/src/draw.rs` の `seam @ (ResidentContent::Image(_) | ResidentContent::Surface(_))`。

### 🔴 重要な問題 2: `\f[]` の引数は `[""]` ではなく `[]`（lexer の実挙動と食い違う契約文）

- **問題**: 設計の `Instruction::Font` の契約（doc コメント・要件 2.2 の追跡行・`decode_font_tests.rs` の述語「`\f[]`＝`[""]`」）は「`\f[]` は lexer が `[""]` を返す」と書く。lexer の `scan_bracket_args` は `[]` を「真の空・0 引数」として `args == []` を返し、既存テスト `empty_bracket_yields_no_args`（`\s[]` → `args: vec![]`）がそれを固定している。`[""]` になるのは `\f[""]`（クォートした空）だけである。
- **影響**: 設計どおりに書いた解読テストの述語が赤になる。`FontTagIssue::NoKey` の判定は「`args` が空、または `args[0]` が空文字列」の両方を見る必要がある（設計の状態機械は `args[0]` をキーとするので、`[]` を index せずに扱う書き方が要る）。
- **提案**: 契約文を「`\f[]` と裸の `\f` はどちらも `args == []`・`\f[""]` は `[""]`・`\f[bold,]` は `["bold",""]`」に改め、`apply_font_tag` の前提を「`args.first()` が `None` または空文字列なら `NoKey`」とする。`decode_font_tests.rs` の述語も同じ 3 形で固定する（`research.md` §1.1 の「`\f[]` は `[""]` で届く」の記述が誤りの源なので、§9 に訂正を 1 行足す）。
- **追跡**: 要件 2.2・2.6・15.1。
- **根拠**: `design.md` 「Parsers 層 › `Instruction::Font` と解読の腕」の Responsibilities 1 点目と Contracts の doc コメント・Requirements Traceability 2.2 行・Testing Strategy Unit Tests 1。コード: `crates/areka-parsers/src/sakura/lexer.rs` `fn scan_bracket_args` の `']'` の腕（「`[]`（真の空）は引数 0 個」）・`crates/areka-parsers/src/sakura/lexer_tests.rs` `empty_bracket_yields_no_args`。

（3 件目に相当する問題は見つからなかった。）

---

## 4. 設計の強み

1. **「番号 0＝既定の見た目」（D14）が、`\f` 無し台本の同一性を構造で保証する**。既定だけの行は装飾番号がすべて 0 なので run が 1 つになり、範囲指定・色のリセット・ブラシ生成のいずれも呼ばれない。要件 14 の「1 画素も変えない」を、比較テストの緑に頼らず読み取れる形にしている。装着前に届いた命令が表示時の既定で描ける点も同じ仕組みから出ている。
2. **戻す操作を「`TextLook` の丸ごと置換」にしたことで、項目の列挙を持たない**。後続の寄せ・影・選択肢マーカーが `TextLook` にフィールドを足すだけで自動的に戻しに含まれ、要件 10.4 の「更新漏れが起きない構造」を満たす。`\c`（内容だけ消す）と台本の先頭（`ClearAll`＝装飾も戻す）の切り分けも、既存の `request_clear` → `line_store.clear()` の経路と噛み合っており、装飾番号を振り直しても古いレイアウトが再利用されない前提をコードで確認できた。

---

## 5. 細かい所見（最大 5 件・設計討議で触れれば足りる）

1. **消費者台帳への 1 行**: `crates/areka/src/emo2_boot/consumer_ledger.rs` は「コマンド名→担当」の宣言表で、モジュール doc が「以後のコマンド追加は消費者＋本表 1 行」と定める。`"\\f"` → 文字レンダリング層の行（と `CommandConsumer` の variant 1 つ・同ファイルの正準表テスト）を Modified Files に足すとよい。実行時の選別には使われないので、漏れても動作は変わらない。
2. **既定だけの行の箱寸は `font.height` をそのまま渡す**: 設計は行の箱の行送り軸寸を `block_extent(run.size, mode)` から取るが、`run.size` は行矩形の引き算（`rect.bottom - rect.top`）で、f32 では `font_height` と厳密に一致しないことがあり得る（`\_l` の小数座標など）。既定だけの行は従来どおり `font.height` を渡す分岐を残すか、`PositionedLine` に「行の em」を値として持たせて引き算を経ずに渡すと、要件 14.1 の同一性が f32 の丸めに依存しなくなる。
3. **行末の空行の高さ（要件 7.9）**: `GlyphStyles` は「グリフ序数→見た目」しか表せないため、台本の末尾に文字の無い行が続く場合（例: `\f[height,30]\n\n` で終わる）は「次に置く文字」が無く、設計の規則では既定の大きさに落ちる。要件は「そのとき効いている大きさ」を求めているので、`GlyphStyles`（または `layout_styled` の引数）に「末尾の現在の見た目」を 1 つ足しておくと規則が閉じる。
4. **実体化の段階のテスト改訂の範囲**: D24 ⑴ は `decoration_and_disable_seams_are_type_only` の述語だけを挙げるが、同ファイルの `use super::{…, FontDisableSeam, RESERVED_KEY_DISABLE_FONT_PREFIX, …}` の import 行と、`:158`（`RESERVED_KEY_DISABLE_FONT_PREFIX`）・`:164`（`resolved.disable == FontDisableSeam::default()`）の断言も同時に変わる。`surface.rs` :747 の doc コメントも `FontDisableSeam` を名指ししているので追随対象に入れる。
5. **数の記述の訂正**: `BalloonScopeAssets` のテスト構築は 2 か所（`assets_tests.rs` には無い）。`ResolvedBalloonText::resolve` の呼出は 52 か所（`research.md` D19 の 26 は過少）。いずれも作業は変わらないが、要件 16.7「主張は実測で裏取り」に沿って直しておく。

---

## 6. 判定

**GO（条件付き）**。

- **理由**: 既存の層構造・依存方向・命名規約・ログの規律との整合が取れており、開発者裁定はすべて守られている。要件 16 件はすべて部品と検証に対応づけられ、各要件に「過去の壊れ方を再現すると赤」の較正が置かれている。見つかった 2 件はいずれも構造の再設計を要さず、分割の割り方（`DrawExecutor` を `draw.rs` に残す）と契約文 1 か所（`\f[]` は `[]`）の訂正で閉じる。
- **タスク生成の前に行うこと**: 重要な問題 1・2 を `design.md`（File Structure Plan・Modified Files・「draw ファサードの分割」・`Instruction::Font` の契約・Traceability 2.2・Testing Strategy）と `research.md` §9（D15 の訂正・§1.1 の `[""]` の訂正）に反映する。細かい所見 1〜5 は設計討議で採否を決めればよい。
- **次の段階**: 反映後に `/kiro-spec-tasks areka-P0-text-decoration-canon`。
