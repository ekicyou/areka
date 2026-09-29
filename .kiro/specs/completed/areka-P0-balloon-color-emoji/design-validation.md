# Design Validation: areka-P0-balloon-color-emoji

> 実施: 2026-09-29・ブランチ `claude/areka-p0-balloon-color-emoji-a984b3`（HEAD `40064159`・ソースは main `65e28545` と同一）。
> 手順: `.claude/skills/kiro-validate-design/rules/design-review.md`（分析 → 重要な問題 ≤3 → 強み → GO/NO-GO）。
> 設計の主張のうちコードで確かめられるものは Grep/Read で実物と突き合わせた（下の「実物との照合」）。コードは「何の定義か」で指し、行番号では指さない。

## Design Review Summary

設計は、要件が委ねた 10 項目すべてに実測（R-1〜R-8）で答えを出し、変更を「型の単位を替える＋新しいものは兄弟ファイルへ」に収めている。切り方の定義点を `areka-sakura/src/cluster.rs` の 1 つに置く構造は依存の向きと層規律に沿い、`Copy` を外す波及が本番 3 か所であることも実物で確認できた。実装へ進める水準にあるが、テストの組み方に 2 点、**判定が空振りする（何も検査していないのに緑になる）**形が含まれているので、設計ディスカッションで直してから着手する。

## Critical Issues

### 🔴 Critical Issue 1: 同じ executor で「描き直す」比較は再描画が起きず、判定が空振りする

- **Concern**: 設計は 6.1／6.4 の対照を「同じ canvas を `set_text_draw_options_for_test(NONE)` で描き直すだけで比較できる（行の TextLayout は再利用され、描画のオプションだけが違う）」としている。しかし `ViewboxExecutor::render_styled` は `ScrollPlanner::plan_with_overhangs` が前回確定行（`prev_lines`）との差分からダーティ領域を導き、変化の無いフレームは文字を描き直さない（`FramePlan` の blit／無変化の経路）。描画オプションは行の指紋（`line_fingerprint`＝文字列と装飾）に含まれないので、2 度目の描画は前フレームの面をそのまま残す。
- **Impact**: 6.4「非絵文字の画素が `NONE` と `ENABLE_COLOR_FONT` でバイト等価」は、2 度目に何も描かれないため**必ず等価**になり、証拠にならない（要件 5.1 の根拠が消える）。6.1 の「`NONE` に戻すと `colored_count == 0`」は逆に必ず赤になる。
- **Suggestion**: 対照は **executor と面を別々に作って**取る（既存の `viewbox_draw_choice_hover_tests.rs` が `exec_glyph`／`exec_choice` の 2 台で比べている形と同じ）。`set_text_draw_options_for_test` は**最初の描画の前**に 1 度だけ呼ぶ口として説明を改める。同じ executor で続ける案を残すなら、`request_clear()` を挟んで FullClear → 全域再描画の 2 フレームを回し、`stats` で再描画が起きたことを判定に含める。いずれも Testing Strategy と `ViewboxExecutor` の節の書き換えだけで済み、構造は変わらない。
- **Traceability**: 要件 5.1・6.1・6.4
- **Evidence**: design.md「`ViewboxExecutor`／`segment_text_range`」の末尾の段落、「Testing Strategy › Integration Tests › 1」

### 🔴 Critical Issue 2: ホバー判定「絵文字の帯に白の画素が 0」は、白を含む絵文字で成り立たない

- **Concern**: 4.5／6.3 の判定を「絵文字の帯に切替文字色（白）の画素が 0・`colored_count > 0`」としているが、判定に使うと決めた 😀 は字形そのものに白（歯・目の光）を持つ。ホバーの有無に関わらず白の画素が出るので、この判定はホバー無しでも赤になる。裁定 8 の「非ホバーと同数」だけが正しい形で、「0」は余計な条件。
- **Impact**: 実装時に赤の原因を追う手戻りが出るか、赤を避けるために閾値を緩めて（白を少し許す等）判定の意味が薄れる。
- **Suggestion**: 判定を「hover=None と hover=Some で、絵文字の帯の**白の画素数が同じ**・`colored_count` が同じ」に改める（0 は要求しない）。「あ」「い」の帯は白の画素が hover=Some でだけ増えることで、ホバーの範囲が選択肢全体を覆い絵文字の前後で途切れないことを示す。ハイライトの矩形塗り（`HighlightPaint`）を持つ選択肢の見た目を使うなら、帯の背景色も変わるので、数える色を「切替文字色」と「色つきの画素」の 2 つに限る旨を書く。
- **Traceability**: 要件 1.3・4.5・6.3
- **Evidence**: design.md「Testing Strategy › Integration Tests › 1 › ホバー中の絵文字」、「裁定と設計判断 › 8」

（3 つ目に挙げるほどの問題は無い。`TextItem::glyph(&str)` が「非空・1 クラスタ」の前提を検査しない点は、`debug_assert!(cluster_count(text) == 1)` を 1 行置けば済む軽微な事項として設計ディスカッションで触れる。）

## Design Strengths

1. **切り方の定義点が 1 つで、依存の向きに沿う**。`areka-emo-text/Cargo.toml` は既に `areka-sakura` へ依存しており、`unicode-segmentation` 1.13.3 は `Cargo.lock` に在る（実物で確認）。再生時間（`duration.rs`）とアイテムの追記（`state.rs`）が同じ関数を通るので、要件 2.1「同じ 1 つの切り方」が構造で成り立つ。
2. **実測に基づく決定と、波及の見積もりが実物と一致する**。`Copy` を外す本番の影響は `layout.rs` の `layout_inner` の `match *item`・`layout_line_ops.rs` の `segment_advance_sum` の `= *item`・`canvas.rs` の `from_layout` の `ch: g.ch` の 3 か所だけで、`.copied()` は `Segment`・`routing`・`overhangs` にしか無い（Grep で確認）。国旗が DirectWrite で 2 クラスタになる事実を、単位（UAX #29）と幅（`probe_advance` の合計）と照合用の判定（束ねる）の 3 つで矛盾なく吸収している。

## 実物との照合（設計の主張 → 確認結果）

| 主張 | 確認 |
|---|---|
| `TextItem::Glyph { ch: char }`・`derive(Copy)`・`apply_cue` の `Text`／`Choice` の腕が `text.chars()` を 2 組使う（`state.rs`） | 一致 |
| `GlyphMetrics::advance(ch: char, …)`／`advance_styled`・`FixedMetrics::advance` の `ch.is_ascii()`・`PositionedGlyph { ch: char }`＋`Copy`・`layout_inner` の `match *item`（`layout.rs`） | 一致 |
| `DWriteMetrics` の `cache: HashMap<(char, FontKey), f32>`・`measure`／`probe_advance`（`HSTRING::from(ch.to_string())`）／`degraded_advance`・`cached_probe_count` は `#[cfg(test)]`（`draw_metrics.rs`） | 一致 |
| `segment_plan` の `run_text.push(*ch)`・`chunk.chars().count()`・説明文「M1 は書記素クラスタ結合なし ゆえ写像は無損失」（`segment.rs`） | 一致。「か゚き゚く゚の話」の写像規則を手で追うと `[1, 2, 1, 1]`（合計 5）になり設計と一致 |
| 本番の描画が `D2D1_DRAW_TEXT_OPTIONS_NONE` を渡すのは `viewbox_draw.rs` の `render_styled` と `draw.rs` の `DrawExecutor::render`（`#[cfg(test)]`）の 2 か所・`ENABLE_COLOR_FONT` は `crates/` に 0 件 | 一致（他は wintf のウィジェットと `viewbox_blit_spike.rs` で範囲外） |
| `fail_next_render` が `ViewboxExecutor` の `#[cfg(test)]` の欄として在る（差し替えの口の手本） | 一致 |
| `style_runs`・`segment_text_range` が `g.ch.len_utf16()` を積む | 一致 |
| `line_fingerprint`（`viewbox.rs`）・`from_layout`（`canvas.rs`）・`render_styled` ×3・`DrawExecutor::render` が `g.ch` を連結 | 一致 |
| 1,000 行の見張り: `layout.rs` 973・`actor.rs` 975・`viewbox_draw.rs` 882・`viewbox.rs` 866 | 一致（`wc -l`）。見込みの表で 1,000 を超えるファイルは無い |
| `lib.rs` の `PURE_SOURCES.len() == 56`・`SOURCES_OUTSIDE_THE_PURE_SCAN`・`pure_layer_modules_have_no_windows_imports` | 一致 |
| `DRAW_FACADE_SOURCES` の実ファイル照合は `draw*` かつ `_tests.rs`／`_test_support.rs` 以外 | 一致（新設 3 ファイルは対象外） |
| `unicode-segmentation` 1.13.3 が `Cargo.lock` に在る・`areka-sakura/Cargo.toml` に無い・`areka-emo-text` は `areka-sakura` へ依存済み | 一致 |
| `FontCatalog::family_for(&[String]) -> Option<String>`（`draw_catalog.rs`） | 一致 |
| `LegacyPitchMetrics::advance(ch: char, …)`（`tests/kero_menu_capacity_test.rs`） | 一致 |
| `run_channel_pipeline` が `std::thread::spawn(…).join()` で `TextLayerState` を値で返す（`Send` が要る根拠） | 一致 |
| 既存 3 テスト `glyph_unit_is_rust_char`・`counts_chars_not_bytes_or_utf16_units`・`probe_advances_match_drawn_line_cluster_advances`（`widths.len() == text.chars().count()` 前提）・`style_runs_group_consecutive_ids_and_count_utf16_units`・`representative_japanese_boundaries_match_budouy_chunks` | すべて実在 |
| `doc/COMPAT_ARCHITECTURE.md` §8「沈黙ルール対応表」・`text-align-shadow-canon/brief.md` 末尾「`balloon-color-emoji` からの申し送り（2026-09-29）」 | 実在 |
| `TextItem::Glyph { ch` の直書きの規模 | 28 ファイル・199 か所（設計の「本番 4＋テスト 24＋約 180 か所」と整合） |

## Final Assessment

- **Decision: GO**（条件つき）
- **Rationale**: 層規律・依存の向き・1,000 行の見張り・log-first・並走の接触範囲（`sink.rs`／`crates/areka`／`crates/wintf` 変更 0）のいずれにも反しておらず、要件 1〜10 の追跡表は実物の定義名と一致する。2 つの問題はどちらも Testing Strategy の書き方の誤りで、構造や型の判断には触れない。設計ディスカッションで該当の段落を改めれば、タスク生成へ進める。
- **Next Steps**:
  1. 設計ディスカッションで Issue 1（対照は executor と面を別々に・口は最初の描画の前に 1 度）と Issue 2（白の画素は「0」でなく「非ホバーと同数」）を design.md に反映する。
  2. `TextItem::glyph` に `debug_assert!(cluster_count(text) == 1)` を置くかを決める（軽微）。
  3. `/kiro-spec-tasks areka-P0-balloon-color-emoji` へ進む。
