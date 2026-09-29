# Gap Analysis: areka-P0-balloon-color-emoji

> 実測: 2026-09-29・本ブランチ `claude/areka-p0-balloon-color-emoji-a984b3`（HEAD `57f74b3f`＝main `65e28545` の上に spec の初期化 1 コミット。ソースは main と同一）。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 本書は分析と選択肢を並べるもので、最終の決定はしない。決定は要件ディスカッション（本書末尾の「設計判断の項目」）と設計で行う。

## 1. 要約

- **色**は 2 か所（本番 `viewbox_draw.rs` の `ViewboxExecutor::render_styled` と照合用 `draw.rs` の `DrawExecutor::render`）で `D2D1_DRAW_TEXT_OPTIONS_NONE` を `ENABLE_COLOR_FONT` に替えるだけで着く。定数は `windows` 0.62.2 の `Win32_Graphics_Direct2D` に在り（`D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT = 4`）、wintf の `D2D1DeviceContextExt::draw_text_layout` は `options` をそのまま受け取るので **wintf に触らない**（要件 7.1）。
- **単位**は `char` が型・関数の署名に焼き込まれている: `TextItem::Glyph { ch: char }`（`state.rs`）・`PositionedGlyph { ch: char }`（`layout.rs`）・`GlyphMetrics::advance(ch: char, …)`／`advance_styled`（`layout.rs`）・`DWriteMetrics` のキャッシュ鍵 `(char, FontKey)`（`draw_metrics.rs`）。**どちらの型も `Copy` を導出しており、クラスタ（可変長の文字列）を持たせると `Copy` が外れる**。これが本仕様で最も広く波及する設計判断（§7 項目 1）。
- 「数える」側（出す間隔・再生時間・選択肢の範囲・装飾の番号列・分かち書きの塊）は**個数**でしか単位を見ておらず、`text.chars()` を「クラスタの列」に替えれば追随する。UTF-16 の範囲（`style_runs`・`segment_text_range`）は `g.ch.len_utf16()` を「クラスタの UTF-16 長」に替えれば、範囲の境界が自動でクラスタの境界に揃う（要件 4.6）。
- **クラスタの切り方の出どころ**は、純粋層（`state.rs`・`segment.rs`・`layout.rs`）が `windows` 系 crate を持てない層規律（`lib.rs` の `pure_layer_modules_have_no_windows_imports`）から、DirectWrite ではなく **`unicode-segmentation`（Unicode 17.0・UAX #29）** が自然（§7 項目 2）。`Cargo.lock` に 1.13.3 が既に在る（`convert_case` 経由）。
- 検証の足場は揃っている（`TextSurface::read_back`・`opaque_count`・live-diff のリグ・`DrawRig`）。欠けるのは (a) 色つきの画素の述語、(b) 代替フォントが無い機械での「理由を出して飛ばす」判定、(c) 「非絵文字の画素が変わらない」の対照（要件 6.4）の 3 つ。

## 2. 現状調査（要件 → 資産の対応表）

| 要件 | 既存の資産（定義名＋ファイル） | 状態 | ギャップの種別 |
|---|---|---|---|
| 1.1 色つきの字形 | `ViewboxExecutor::render_styled` の `draw_text_layout(..., D2D1_DRAW_TEXT_OPTIONS_NONE)`（`viewbox_draw.rs`）／`DrawExecutor::render` の同じ呼出（`draw.rs`・`#[cfg(test)]`） | フラグを渡すだけ | Missing（2 トークン） |
| 1.2 指定フォントが持つ字形は文字色 | `create_text_format`／`try_create_format`（`draw.rs`）はシステムのコレクションで書式を作る。`SetFontFallback` は 0 件＝OS の既定の代替 | 今日どおり | なし（構造で成立） |
| 1.3 `\f[color]`・ホバー色を重ねない | `apply_color_ranges`（`viewbox_draw_decoration.rs`）と選択肢の hover は `SetDrawingEffect(brush)`。色つきの層は Direct2D が字形の色で描く | 今日どおり | **Unknown**: COLR の層に「前景色を参照する層」（palette index 0xFFFF）があると、その層だけブラシの色に従う（§4 R-3） |
| 1.5 代替に色つきが無いときは単色 | Direct2D の既定（色つきの層が無い字形は通常の描画） | 今日どおり | なし |
| 1.6 縦書き 3 方式 | `DirectionRecipe::for_mode`（`draw.rs`）が書式に方向を焼く。描画の呼出は方向に依らず 1 か所 | 今日どおり | **Unknown**: 縦書きの色つき字形の向き（要件は判定しない・裁定 6） |
| 2.1〜2.3 クラスタで数える | `TextLayerState::apply_cue` の `Text`／`Choice` の腕（`state.rs`）: `text.chars().count()` と `text.chars().map(|ch| TextItem::Glyph { ch })` の 2 組 | `char` | **Missing**: クラスタの切り方の関数と `TextItem::Glyph` の型 |
| 2.4〜2.5 1 段ずつ・間隔 | `RevealSchedule::extend_chunk(glyph_count, …)`・`interval = duration / glyph_count`（`state.rs`） | 個数だけを見る | なし（2.1 に追随） |
| 2.6 再生時間 | `text_playback_duration`（`areka-sakura/src/duration.rs`）: `text.chars().count() as u64 * CHAR_NOMINAL_MS` | `char` | Missing（1 関数・`areka-sakura` に切り方が要る） |
| 2.7 選択肢の範囲 | `ChoiceSpan::glyph_range: start..(start + glyph_count)`（`state.rs`）・`annotate_lines`（`choice.rs`）は序数の交差 | 個数だけ | なし（2.1 に追随） |
| 3.1 幅＝クラスタ全体を描いた幅 | `DWriteMetrics::probe_advance(ch: char, …)`（`draw_metrics.rs`）: `HSTRING::from(ch.to_string())` の未折返しレイアウトから `get_cluster_metrics()` の `width` 合計 | 1 文字の文字列を測る＝**文字列なら今のままクラスタ全体を測れる** | Missing: 署名を `&str` へ |
| 3.2 計測と描いた行の一致 | `probe_advances_match_drawn_line_cluster_advances`（`draw_oracle_tests.rs`）は `widths.len() == text.chars().count()` を前提 | `char` 前提 | Missing: 前提をクラスタへ（要件 6.6）。**Unknown**: DirectWrite のクラスタ数と UAX #29 のクラスタ数が絵文字 5 形で一致するか（§4 R-1） |
| 3.3 キャッシュの鍵 | `DWriteMetrics::cache: RefCell<HashMap<(char, FontKey), f32>>`・`measure(ch: char, key, format)` | `char` | Missing: 鍵をクラスタ全体へ |
| 3.4 `\_l` | `TextItem::CursorMove`（`state.rs`）・`resolve_cursor_component`（`layout_line_ops.rs`）: 序数を消費せず、基点は `inline_pos` の合計 | 単位に依らない | なし（3.1 が満たされれば自動） |
| 3.5 縮退の固定幅 | `FixedMetrics::advance`（`layout.rs`）と `degraded_advance`（`draw_metrics.rs`）: `ch.is_ascii()` で半角／全角 | `char` | Missing: `str::is_ascii()` へ（複数スカラー値のクラスタは ASCII でないので全角 1 つ分になる） |
| 4.1 文字ごとの折り返し | `LayoutEngine::layout_inner` の `WrapPlan::CharByChar` の腕（`layout.rs`）: 1 `Glyph` アイテムを単位に判定 | アイテム単位 | なし（アイテム＝クラスタになれば途中で切れない） |
| 4.2 分かち書きの塊 | `segment_plan`（`segment.rs`）: `run_text.push(*ch)`・budouy のチャンクを `chunk.chars().count()` で `Segment { start, len }` へ | `char` | Missing。**Unknown**: budouy のチャンク境界がクラスタの途中に落ちうる（§4 R-2・§7 項目 4） |
| 4.3 1 クラスタが行幅を超える | `layout_inner` の「行頭の 1 グリフはどちらを超えても配置する」 | アイテム単位 | なし |
| 4.4 当たり範囲 | `annotate_lines` の `inline_range: (first.inline_pos, last.inline_pos + last.advance)`・`derive_hit_rows`（`choice.rs`） | 位置＋送り幅 | なし |
| 4.5 ホバー色 | `segment_text_range`（`viewbox_draw.rs`）: `g.ch.len_utf16()` の積み上げと文字の中心判定 | `char` | Missing: UTF-16 長をクラスタから取る |
| 4.6 装飾の範囲 | `style_runs`（`viewbox_draw_decoration.rs`）: `g.ch.len_utf16()` の積み上げで `DWRITE_TEXT_RANGE` | `char` | Missing: 同上 |
| 5.1〜5.3 非絵文字は不変 | 1 スカラー値の文字は 1 クラスタ＝アイテム列が同一。`draw_line_store.rs` の `CachedLineLayout.text` は `ch` の連結・`line_fingerprint`（`viewbox.rs`）も連結 | 連結は不変 | **Unknown**: `ENABLE_COLOR_FONT` が単色の字形の画素を 1 ビットも変えないか（§4 R-4・要件 6.4 の証拠） |
| 5.4 失敗経路を増やさない | `probe_advance` の失敗は `error!`＋縮退値・`measure` は失敗をキャッシュしない | 今日どおり | なし |
| 6.1 色つきの画素 | `TextSurface::read_back`（`surface.rs`・BGRA 密配列）・`opaque_count`／`block_axis_ink_span`（`viewbox_draw_test_support.rs`） | 画素の述語は α と位置だけ | Missing: 「文字色でも背景色でもない色」の述語 |
| 6.4 変更前の実物との対照 | `viewbox_draw_png_dump_tests.rs` は `AREKA_DIAG_OUT` へ出すだけで、リポジトリに golden の PNG は無い（`find -name "*.png"` 0 件） | golden なし | Missing: 対照の取り方（§7 項目 6） |
| 6.7 代替フォントが無い機械 | `FontCatalog::family_for(&[String]) -> Option<String>`（`draw_catalog.rs`）で実在を引ける。`yugothic_real_fixture_matches_oracle_byte_for_byte` は「フォント不在でも oracle↔viewbox は一致する」設計で飛ばさない | 引ける | Missing: 「理由を出して飛ばす」判定の型（既存に例が無い） |
| 7.1〜7.3 並走 | `sink.rs`・`crates/areka`・`crates/wintf` は触らない。`actor.rs` は `present_actor` が `WrapMode → WrapPlan` を選ぶ箇所を読むだけ | — | Constraint: **`layout.rs` は 973 行・`actor.rs` は 975 行**で 1,000 行の見張り（`log-capture-kit/tests/file_length_guard_test.rs`）の直下 |
| 8.1 記録を増やさない | `apply_cue` の `debug!` は cue 単位。`probe_advance` は失敗時だけ | 今日どおり | なし |
| 9.1 沈黙ルール対応表 | `doc/COMPAT_ARCHITECTURE.md` §8（4 列の表） | 1 行足す | なし |
| 9.2 先送りの引き受け | `completed/areka-P0-emo-text-layer/design.md`「書記素クラスタ結合は M2 検討事項として記録」・`research.md` D10・`completed/areka-P0-budoux-newline/research.md` | 本仕様の文書へ | なし |
| 10.8 影の申し送り | `.kiro/specs/areka-P0-text-align-shadow-canon/brief.md` 末尾「`balloon-color-emoji` からの申し送り（2026-09-29）」 | **既に書かれている**（3 行） | なし（要件確定時の作業は済んでいる） |

### 単位の変更が触る場所（本番コード）

| 場所 | 今 | 変更 |
|---|---|---|
| `state.rs` `TextItem::Glyph { ch: char }` | `Copy` | クラスタの表現（§7 項目 1） |
| `state.rs` `apply_cue` の `Text`／`Choice` の腕 | `text.chars()` ×2 組 | クラスタの列 |
| `layout.rs` `GlyphMetrics::advance`／`advance_styled` の署名・`FixedMetrics::advance`・`PositionedGlyph { ch }`・`layout_inner` の `match *item` | `char`・`Copy` | `&str`（または新しい型）・`match item` |
| `layout_styled.rs` `glyph_style_advance(ch: char, …)`・`layout_line_ops.rs` `segment_advance_sum` の `if let TextItem::Glyph { ch } = *item` | `char` | 同上 |
| `draw_metrics.rs` `measure`／`probe_advance`／`advance`／`advance_styled`／`degraded_advance`・`cache` の鍵 | `char` | `&str`・鍵を `(Box<str> または String, FontKey)` |
| `segment.rs` `segment_plan`（`run_text.push(*ch)`・`chunk.chars().count()`）と説明文「写像は無損失」 | `char` | クラスタ数＋境界の吸収規則（§7 項目 4） |
| `canvas.rs` `ContentCanvas::from_layout`（`ch: g.ch` の写し） | `Copy` | `clone` |
| `viewbox.rs` `line_fingerprint`・`viewbox_draw.rs` `render_styled` の `text: String = run.glyphs.iter().map(|g| g.ch).collect()` ×3・`draw.rs` `DrawExecutor::render` の同 1 か所 | `char` の連結 | クラスタの連結（`collect::<String>()` は `&str` の列でもそのまま通る） |
| `viewbox_draw.rs` `segment_text_range`・`viewbox_draw_decoration.rs` `style_runs` | `g.ch.len_utf16()` | クラスタの `encode_utf16().count()` |
| `areka-sakura/src/duration.rs` `text_playback_duration` | `chars().count()` | クラスタ数 |
| `viewbox_draw.rs`／`draw.rs` の `draw_text_layout` | `D2D1_DRAW_TEXT_OPTIONS_NONE` | `ENABLE_COLOR_FONT` |

**`GlyphMetrics` の実装は 3 つ**: `FixedMetrics`（`layout.rs`）・`DWriteMetrics`（`draw_metrics.rs`）・テストの `LegacyPitchMetrics`（`tests/kero_menu_capacity_test.rs`）。署名を変えると 3 つとも追随する。

**テストの構築点**: `TextItem::Glyph { ch: 'あ' }` の字面は本番 4 ファイル（`state.rs`・`layout.rs`・`layout_line_ops.rs`・`segment.rs`）のほかテスト 24 ファイルに**約 180 か所**、`PositionedGlyph { ch: … }` はテスト 6 ファイル。多くは支援ファイルの `glyph_items(s)`／`glyphs(s)`（`s.chars().map(|ch| TextItem::Glyph { ch })`）を経由するが、直書きも多い（`layout_cursor_tests.rs` 32・`layout_wrap_tests.rs` 27・`viewbox_dirty_tests.rs` 26）。表現の選び方（§7 項目 1）がこの書き換え量を決める。

### 上流（さくらスクリプトの字句解析）はクラスタを割らない

`areka-parsers/src/sakura/lexer.rs` の `lex` は `char_indices` で走査するが、テキストは `text.push(c)` で 1 つの `Token::Text` に**溜めてから** flush する（タグ・`%` の手前だけで区切る）。`chars().take(len)` を使うのはタグ名の綴り（`bare_tag_len`）だけで、台詞の本文をスカラー値で切ることは無い。`areka-sakura/src/compile.rs` も `Text` を 1 つの cue にする（`text_playback_duration` を 2 か所で呼ぶ）。よって**クラスタは cue の境界で割れず**、単位の変更は `areka-emo-text` と `duration.rs` に閉じる（brief の見立てどおり）。

### 手本（wintf のタイプライター）から写せるもの

`typewriter_layout.rs` の `convert_to_timeline` は `get_cluster_metrics()` で**クラスタの総数**を取り、進み方は `text.chars().count()` 回の 1 文字 1 歩（要件 Introduction の引き直し 1 のとおり）。文字とクラスタの対応表は持たない。写せるのは「`DWRITE_CLUSTER_METRICS.length`（UTF-16 単位の長さ）を積めば UTF-16 位置とクラスタの対応が取れる」ことまでで、純粋層の切り方の手本にはならない。

## 3. 実装の選択肢

### 3.1 全体の方針

| 案 | 内容 | 利点 | 難点 |
|---|---|---|---|
| **A: 既存の型と関数を広げる（推奨の土台）** | `TextItem::Glyph`・`PositionedGlyph`・`GlyphMetrics` の**単位の型だけ**を替え、関数の並びと層の分担は変えない。切り方は 1 つの関数に置き、`state.rs`・`segment.rs`・`duration.rs` がそれを呼ぶ | 変更が「型 1 つ＋呼出の追随」に収まり、既存の非回帰テストがそのまま効く。層規律（純粋層は `windows` 非依存）を保てる | `layout.rs`（973 行）へ足す余地がほぼ無い。テストの直書き約 180 か所の書き換え |
| **B: クラスタを別の層として新設する** | `TextItem::Glyph { ch }` は残し、クラスタの結合を配置の手前（`layout` の入力を作る段）で行う「結合の層」を新しいファイルに置く | 既存の型・テストが無傷 | 「同じ 1 つの切り方」（要件 2.1）が層ごとに複製される。演出（`RevealSchedule`）・再生時間（`duration.rs`）・選択肢の範囲（`glyph_range`）は `state.rs` の側で数えるので、結合の層だけでは間に合わない＝結局 `state.rs` も変える。1 フレーム遅らせる解には当たらないが、状態の持ち方を 2 重にする |
| **C: A を土台に、新しいものだけ別ファイル** | 型と署名は A のとおり替え、新しいコード（切り方の関数・クラスタの表現型・色つきの画素の述語・絵文字のテスト）は新しい兄弟ファイルへ置く | A の利点＋1,000 行の見張りを守れる。新設ファイルは `lib.rs` の `PURE_SOURCES`／`SOURCES_OUTSIDE_THE_PURE_SCAN` に載せれば済む | ファイルが 2〜4 本増える |

推奨は **C**（A の考え方＋ファイル配置の規律）。B は要件 2.1「次のすべてで同じ 1 つの切り方」を構造で満たせない。

### 3.2 クラスタの表現（`TextItem::Glyph` の中身）

| 案 | 型 | `Copy` | テストの書き換え | 備考 |
|---|---|---|---|---|
| ⒜ 文字列を持つ | `Glyph { text: Box<str> }`（または `String`） | 外れる | 直書き約 180 か所を `Glyph { text: "あ".into() }` 等へ（機械的） | 最も素直。`layout_inner` の `match *item` → `match item`、`from_layout` の写しを `clone()` に。`PositionedGlyph` も `Copy` を外す |
| ⒝ 上限つきの内蔵文字列 | `[u8; N]`＋長さ（例 N=32） | 保てる | ⒜ と同じ | 上限を超えるクラスタ（UAX #29 は結合文字の連なりに上限を置かない。RGI の最長の絵文字列は 10 スカラー値≒35 バイトで **32 を超える**）を割るか落とすかの縮退が要る＝正しさの穴と縮退の記録が増える |
| ⒞ 1 スカラー値は `char`・複数だけ文字列 | `enum Cluster { Single(char), Multi(Box<str>) }` | 外れる | 直書きは `'あ'.into()` で済むが結局全部触る | 2 形の分岐が消費側に漏れる（`as_str()` を通せば隠せる） |
| ⒟ 状態が文字列を 1 本持ち、アイテムは範囲 | `Glyph { start: u32, len: u16 }`＋`ActorTextState::text: String` | 保てる | 直書きはすべて変わる（範囲は手で書けない） | `layout`・`canvas`・`draw` が文字列本体を別途受け取る＝関数の署名が全部増える。`Clear` で文字列も消す。最も広く波及 |

`Copy` を外す影響は、実測では `layout.rs` の `match *item`・`layout_line_ops.rs` の `if let … = *item`・`canvas.rs` の `ch: g.ch` の 3 か所（本番）だけで、`.copied()` は `Segment`・`overhangs`・`routing` にしか使われていない。**⒜ が最小**。判断は §7 項目 1。

### 3.3 クラスタの切り方の出どころ

| 案 | 出どころ | 層 | 決定論 | 備考 |
|---|---|---|---|---|
| ⒜ `unicode-segmentation` | `UnicodeSegmentation::graphemes(true)`（拡張書記素クラスタ・Unicode 17.0） | 純粋層で使える（`windows` 非依存・`std` のみ・`rust-version 1.85`） | 完全（同じ入力→同じ切り方・OS の版に依らない） | `Cargo.lock` に 1.13.3 が在る（`convert_case` ← `derive_more` ← `bevy_ecs`）。直接依存を 1 行足す（要件 7.3 の範囲内）。`FixedMetrics` の縮退経路でも同じ切り方が使える |
| ⒝ DirectWrite `GetClusterMetrics` | 計測用レイアウトのクラスタ（`DWRITE_CLUSTER_METRICS.length`＝UTF-16 長） | COM 層のみ（`state.rs`・`segment.rs` は呼べない＝層規律に反する） | OS のフォントと DirectWrite の版に依る | 切り方を COM 層から純粋層へ渡す口（`GlyphMetrics` に「切り方」を足す等）が要り、`FixedMetrics` は別の切り方を持つことになる＝「同じ 1 つの切り方」が崩れる。**不採用が濃い** |
| ⒞ 自前の実装 | 絵文字 5 形の個別規則 | 純粋層 | 完全 | 裁定 4「個別に特別扱いしない」に反する。UAX #29 を書き直すことになる |

`unicode-segmentation` の `graphemes(true)` は `&str` の列を返すので、`state.rs` の `text.chars()` を差し替えるだけで済む。**⒜ を推奨**。設計で決めるのは「切り方の関数をどこに 1 つ置くか」（§7 項目 3）。

### 3.4 切り方の関数の置き場（1 つの定義点）

- **⒜ `areka-sakura` に置く**（例: `duration.rs` の隣か `contract.rs` に `pub fn clusters(text: &str) -> impl Iterator<Item = &str>`）。`areka-emo-text` は既に `areka-sakura` へ依存している（`Cargo.toml` の `areka-sakura = { path = … }`・`contract` を再輸出）ので、**依存の追加は `crates/areka-sakura/Cargo.toml` の 1 行だけ**で済み、`areka-emo-text/Cargo.toml` は触らない。要件 2.1 の「同じ 1 つの切り方」が定義点 1 つで成立し、再生時間（2.6）と演出（2.5）が同じ関数を通る。
- **⒝ 両 crate に依存を足し、それぞれが `graphemes(true)` を直接呼ぶ**。定義点は 2 つ（同じライブラリを呼ぶだけなので値は一致する）。要件 7.3 は「1 行まで」を crate ごとに許しているので範囲内。
- ⒜ は crate 間の依存方向（`areka-sakura → areka-emo-text`）と一致し、`areka-sakura` が「文字の単位」の正本になる（今日も `CHAR_NOMINAL_MS` の定義点は `duration.rs` だけ）。

### 3.5 幅の計測（`DWriteMetrics`）

- `probe_advance` は既に「文字列の未折返しレイアウトのクラスタ幅の合計」なので、引数を `&str` にすればクラスタ全体を 1 度で測る（要件 3.1）。結合した絵文字は DirectWrite が 1 クラスタに整形するので `width` の合計は 1 つ分になる。
- キャッシュの鍵 `(char, FontKey)` → `(Box<str>, FontKey)`。`HashMap` の引き方は `cache.get(&(text, key))` の所有権の都合で `(String, FontKey)` を作るか、鍵を `(Box<str>, FontKey)` にして `raw_entry` 相当の二段の表（`HashMap<FontKey, HashMap<Box<str>, f32>>`）にするかの小さな設計。1 クラスタ 1 回の割り当てで足りる（計測は成功値だけを覚え、毎フレームは引くだけ）。
- **色つきの字形の送り幅は単色と同じ**（COLR の層は基の字形の送り幅を共有する）ので、色の変更が計測に影響する見込みは無い。ただし §4 R-1 で実測する。

### 3.6 分かち書き（`segment.rs`）

budouy の `parse` はスカラー値の境界で切るので、チャンクの境界がクラスタの途中（結合文字の手前・ZWJ の前後・肌色修飾の前）に落ちうる。写像の規則を設計で 1 つ決める:

- **⒜ 境界を後ろへ寄せる**: クラスタの列を走り、クラスタの先頭バイトが属するチャンクへそのクラスタを割り当てる。チャンクの末尾がクラスタの途中なら、そのクラスタは前のチャンクに入り、次のチャンクは 1 クラスタ短くなる（空になれば塊を生まない）。塊の長さの合計＝run のクラスタ数（無損失）。
- ⒝ 境界を前へ寄せる（クラスタの末尾バイトが属するチャンクへ）。
- どちらでも要件 4.2 は満たす。⒜ は「budouy が切った位置より前で切る」ことが無いので、既存の分かち書きの結果（クラスタの途中に境界が無い日本語）は 1 ビットも変わらない。判断は §7 項目 4。

### 3.7 テストの足場

| 必要な検査 | 使える資産 | 足すもの |
|---|---|---|
| 色つきの画素が出る（6.1） | `read_back`（BGRA 密配列・premultiplied）・`opaque_count` | 「α≠0 かつ (B,G,R) が文字色の premultiplied 値でも背景（透明）でもない」画素を数える述語 1 つ。文字色を黒（既定）にすれば「R・G・B のどれかが α より明るい画素」で判定できる（黒は premultiplied でも 0,0,0） |
| 1 段・1 クラスタ幅・割れない（6.2） | `TextLayerState`＋`RevealSchedule::visible`（純粋）・`LayoutEngine::layout`＋`FixedMetrics`（純粋）・`DWriteMetrics`（COM） | 絵文字 5 形の入力を足すだけ。純粋層は GPU 無しで回る |
| 選択肢の絵文字（6.3） | `annotate_lines`・`derive_hit_rows`（純粋）・`viewbox_draw_choice_hover_tests.rs`（COM・hover の読み戻し） | hover 中の絵文字の画素が非 hover と同じ色である判定 |
| 非絵文字が変わらない（6.4） | live-diff（`LiveDiffRig`＝`DrawExecutor` と `ViewboxExecutor` を同じ `Rig` で回す） | 対照の取り方は §7 項目 6。同じテストの中で「色つきの設定あり／なし」を描き比べるなら、`ViewboxExecutor` に描画オプションを差す口（`#[cfg(test)]` のフィールドか構築関数）が要る |
| バイト等価の組に絵文字（6.5） | `run_live_diff_scenario_on`・`viewbox_draw_scroll_retain_tests.rs` | 台詞に絵文字を含む組を 1 つ足す |
| 既存 3 件の改名（6.6） | `glyph_unit_is_rust_char`・`counts_chars_not_bytes_or_utf16_units`・`probe_advances_match_drawn_line_cluster_advances` | 名前と前提の書き換え（判定の数値は据え置き） |
| 代替フォントが無い機械（6.7） | `FontCatalog::family_for(&["Segoe UI Emoji".into()])` | 「無ければ `eprintln!` で理由を出して `return`」の型。既存のテストに同じ型は無い（`yugothic_real_fixture_…` はフォント不在でも通る設計）ので、ここが初例になる |
| 3 方式（6.8） | `draw_oracle_tests.rs`・live-diff は 3 方式を回している | 同じ並びで絵文字の組を足す |

## 4. 設計で調べること（Research Needed）

- **R-1 DirectWrite のクラスタ数と UAX #29 のクラスタ数の一致**（絵文字 5 形＋結合文字）。`probe_advances_match_drawn_line_cluster_advances` の前提「幅の数＝クラスタ数」がそのまま成り立つかを、設計の spike で 3 方式について実測する。一致しない形があれば、その形の幅は `probe_advance` の合計（今の式）で正しく出るので判定だけを緩める。
- **R-2 budouy がクラスタの途中で切る実例**。設計の spike で「👨‍👩‍👧」「か゚」「1️⃣」を含む和文を `parse` にかけ、境界の位置を確かめて §3.6 の規則を決める。
- **R-3 COLR の「前景色を参照する層」**。Segoe UI Emoji の字形に前景色（palette index 0xFFFF）の層があると、その層だけ `\f[color]`／ホバー色に従う。要件 1.3／4.5 の判定に使う絵文字は、全層が固有色のもの（🇯🇵・😀 など）を選ぶ。実測で決める。
- **R-4 `ENABLE_COLOR_FONT` が単色の字形の画素を変えないこと**（要件 5.1／6.4 の前提）。Direct2D の仕様では色つきの層を持たない字形は通常の経路で描かれるが、実測で「同じ台詞を NONE と ENABLE_COLOR_FONT で描いた読み戻しがバイト等価」を確かめる。等価でなければ要件 5.1 の証拠の取り方（§7 項目 6）を変える。
- **R-5 字形の版（COLR v0 か v1 か）**。Windows 11 の Segoe UI Emoji は COLR v1 と v0 の両方を持ち、`ID2D1DeviceContext4` 以降の `DrawTextLayout` は COLR v1 の描画を外部から止められない（Microsoft Learn「Color font support」）。areka の `dc` は `ID2D1DeviceContext`（`create_device_context`）で作るが、実体の版は OS に依るので、どちらの字形が出るかは実測（裁定 3 のとおり、どちらでも要件 1.1／6.1 は満たす）。Windows 10 は COLR v0 のみ。
- **R-6 縦書きでの色つき字形**。`DirectionRecipe` の縦書き 2 方式で `ENABLE_COLOR_FONT` が効くこと（色つきの画素が出ること）だけを実測する。向きは判定しない（裁定 6）。
- **R-7 テスト機の代替フォント**。Windows 10/11 の同梱フォント「Segoe UI Emoji」の実在を `FontCatalog::family_for` で引けること（家族名の綴り）。
- **R-8 異体字セレクタと指定フォントの基字**（要件 1.7）。游ゴシック（バルーンの既定）が基の字形を持つ `❤️`（U+2764 U+FE0F）・`☺️` などで、DirectWrite の代替が U+FE0F を見て Segoe UI Emoji へ回すか、基字のまま単色で描くかを実測し、design.md に事実として書く。どちらでも要件は満たす（独自の代替の規則は持たない）。

参考: [Color font support - Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/directwrite/color-fonts)・[Using color fonts for beautiful text and icons - Windows Developer Blog](https://blogs.windows.com/windowsdeveloper/2017/06/06/using-color-fonts-beautiful-text-icons/)・[unicode-segmentation (crates.io)](https://crates.io/crates/unicode-segmentation)・[UAX #29](https://www.unicode.org/reports/tr29/)。

## 5. 規模とリスク

- **規模: M**（要件の裁定 1 と一致）。本番の変更は型 2 つ・trait の署名 2 つ・関数 10 前後・定数の差し替え 2 か所・`Cargo.toml` 1〜2 行。テストは既存の直書き約 180 か所の追随（機械的）＋新規の決定論テスト（純粋層 5 形 × 3 方式、COM 層の色・hover・バイト等価）。
- **リスク: Medium**。技術は既知（Direct2D のフラグ・UAX #29 のライブラリ）だが、(1) `Copy` を外す波及、(2) budouy との境界の規則、(3) 「非絵文字が変わらない」証拠の取り方、(4) DirectWrite のクラスタ数の一致、の 4 点は設計の spike で実測してから決める。
- **1,000 行の見張り**: `layout.rs` 973・`actor.rs` 975・`viewbox_draw.rs` 882。`layout.rs` は署名の変更（`char` → `&str`）と `match *item` → `match item` の書き換えだけなら行数は増えない。新しい関数は兄弟ファイルへ。

## 6. 設計フェーズへの推奨

1. 方針 C（既存の型・署名を替え、新しいものは兄弟ファイルへ）。
2. クラスタの表現は ⒜ `Box<str>`（`Copy` を外す）。`PositionedGlyph` も同じ。
3. 切り方は `unicode-segmentation` の `graphemes(true)` を、`areka-sakura` の 1 関数に包んで両 crate が呼ぶ（依存の追加は `areka-sakura/Cargo.toml` の 1 行）。
4. 分かち書きは「境界を後ろへ寄せる」規則で写す（既存の分かち書きの結果を動かさない）。
5. 「非絵文字が変わらない」の証拠は、同じテストの中で `NONE` と `ENABLE_COLOR_FONT` を描き比べる形（`ViewboxExecutor` に描画オプションの試験用の口を 1 つ）。
6. 設計の冒頭に spike（R-1〜R-5）を 1 タスクで置き、結果を design.md に実測として書く。

## 7. 設計判断の項目（要件ディスカッションへ）

1. **クラスタの表現と `Copy`**: `TextItem::Glyph`／`PositionedGlyph` に文字列（`Box<str>`）を持たせて `Copy` を外すか（⒜・最小）、上限つきの内蔵文字列で `Copy` を保つか（⒝・上限超えの縮退が要る）。推奨 ⒜。
2. **切り方の出どころ**: `unicode-segmentation`（純粋層・決定論・Unicode 17.0）か DirectWrite のクラスタ計測（COM 層・純粋層から呼べない）か。推奨 `unicode-segmentation`（裁定 4 の「出どころは設計で決める」への答え）。
3. **切り方の定義点と依存の置き場**: `areka-sakura` に 1 関数を置いて両 crate が呼ぶ（依存 1 行・定義点 1 つ）か、両 crate が直接 `graphemes(true)` を呼ぶ（依存 2 行・定義点 2 つ）か。推奨 `areka-sakura` の 1 関数。
4. **分かち書きの境界がクラスタの途中に落ちたとき**: クラスタを前のチャンクへ入れる（境界を後ろへ寄せる）か、次のチャンクへ入れるか。推奨 前のチャンク（既存の結果が不変）。
5. **`probe_advances_match_drawn_line_cluster_advances` の前提**: DirectWrite のクラスタ数と UAX #29 のクラスタ数が一致しない形が見つかったとき、幅の一致だけを判定して数の一致は外すか、その形を組から外すか。spike（R-1）の結果を見て決める。
6. **「非絵文字の画素が変わらない」の対照**（要件 6.4）: 同じテストの中で `NONE`／`ENABLE_COLOR_FONT` を描き比べる（`ViewboxExecutor` に `#[cfg(test)]` の描画オプションの口を足す）か、変更前の `main` から取った読み戻しを固定値として持つか（golden の PNG は今リポジトリに無い＝新設になる）。推奨 描き比べ（R-4 が等価なら証拠として十分・golden の保守が要らない）。
7. **代替フォントの実在の判定**（要件 6.7・要件ディスカッションで「飛ばす」→「失敗にする」へ改めた）: `FontCatalog::family_for` で「Segoe UI Emoji」を引けなければ理由を添えて `panic!`／`assert!` する前提の関数を、テストの支援ファイルに 1 つ置く形。
8. **要件 1.3／4.5 の判定に使う絵文字の選び方**: 前景色を参照する層を持たない絵文字（🇯🇵・😀 など）に限る（R-3）。判定の期待値に「自分の色のまま」を書くには、hover 中と非 hover の画素の比較で足りるか。
9. **COLR の版の記録**（裁定 3）: spike で出た字形の版（v0／v1）を design.md に実測として書くだけで、要件は変えない。Windows 10 では v0 になる旨も併記する。
10. **異体字セレクタの代替の実測の記録**（要件 1.7・R-8）: 游ゴシックが基字を持つ U+FE0F つきの絵文字がどちらに出るかを design.md に事実として書く。独自の代替の規則は足さない。

---

# 設計フェーズの調査記録（2026-09-29・`/kiro-spec-design`）

## Summary

- **Feature**: `areka-P0-balloon-color-emoji`
- **Discovery Scope**: Extension（既存の `areka-emo-text`・`areka-sakura` の拡張。軽量の調査＋設計判断のための実測 1 回）
- **Key Findings**:
  - DirectWrite のクラスタ数は UAX #29 と **国旗以外**で一致する。国旗は Windows に字形が無く、地域表示記号 2 つが単色の 2 クラスタになる（色も出ない）。
  - `ENABLE_COLOR_FONT` は単色の字形の画素を **1 ビットも変えない**（2 書体 × 3 方式 × 2 色の塗りでバイト等価）。
  - 色つきの字形は塗りの色に従う層を持たない（黒／赤の塗りでバイト等価）。字形はこの機械ではグラデーション（COLR v1）。
  - budouy は結合文字の手前で塊の境界を落とす（「か゚き゚く゚の話」→ `か`｜`゚き゚く`｜`゚の`｜`話`）。写像規則が要る。
  - `TextLayerState` は `tests/pipeline_test.rs` がスレッドの `join` 越しに値で返すので `Send` を保つ必要がある（クラスタの型の選択に効く）。

## Research Log

### 実測の方法（一時的な検証プログラム）

- **Context**: §4 の R-1〜R-8 は設計で実測すると決めていた。
- **Sources Consulted**: `crates/areka-emo-text/tests/` に一時ファイル `zz_color_emoji_spike.rs` を置き、`GraphicsCore::new()` → `ID2D1Device::CreateDeviceContext` → `CreateBitmap`（`TARGET`）へ `DrawTextLayout` で描き、`CPU_READ` の bitmap へ `CopyFromBitmap` → `Map` で読み戻した。書式は `create_text_format` と同じ引数（`ja-JP`・NORMAL）で `DirectionRecipe::for_mode` を焼き、家族名はＭＳ ゴシック（既定）と Yu Gothic UI の 2 つ、方式は 3 つ、大きさは計測 24px・描画 48px。`cargo test -p areka-emo-text --test zz_color_emoji_spike -- --nocapture` で 1 回走らせ（0.73 秒）、ファイルは削除した（コミットしない）。
- **Findings**: 下の各項目。
- **Implications**: 設計は実測に基づく事実として design.md「設計の調査で確かめた事実」へ転記した。

### R-7 代替フォントの実在

- **Findings**: `FontCatalog::family_for(&["Segoe UI Emoji"])` → `Some("Segoe UI Emoji")`。`Yu Gothic UI`・`游ゴシック` も `Some`。
- **Implications**: 6.7 の判定はこの綴りで引く。

### R-1 DirectWrite のクラスタ数と幅（`GetClusterMetrics`・24px・折り返し無し）

| 形 | UAX #29 | DirectWrite（横） | 幅（ＭＳ ゴシック・横） | 幅（縦 2 方式） |
|---|---|---|---|---|
| a あ 🦆 | 3 | 3 | 12.0 / 24.0 / 32.95 | 12 / 24 / 22.52 |
| 😀 | 1 | 1 | 32.95 | 22.52 |
| 👨‍👩‍👧 | 1 | 1（UTF-16 長 8） | 30.07 | 22.52 |
| 🇯🇵 | 1 | **2**（各 UTF-16 長 2） | 7.15 ＋ 11.09 | 22.52 ＋ 22.52 |
| 👍🏻 | 1 | 1（長 4） | 32.95 | 22.52 |
| ❤️ | 1 | 1（長 2） | 24.0（Yu Gothic UI は 32.95） | 24.0（同 22.52） |
| 1️⃣ | 1 | 1（長 3） | 24.0 | 24.0 |
| か゚ | 1 | 1（長 2） | 24.0 | 24.0 |
| a あ 👨‍👩‍👧 b 🇯🇵 | 5 | 6 | 12 / 24 / 30.07 / 12 / 7.15 / 11.09 | — |

- **Findings**: 国旗だけ 2 クラスタ。行の中（最後の行）でも同じ 2 つの幅なので、`probe_advance` の合計式は行の中で占める幅と一致する。Yu Gothic UI でも数は同じ。
- **Implications**: 単位は UAX #29。照合用の判定（`probe_advances_match_…`）は DirectWrite のクラスタを UTF-16 長でクラスタごとに束ねて合計を比べる形へ改める（§7 項目 5 の答え）。国旗の色は 1.5 の縮退。

### R-4 非絵文字の画素（`NONE` と `ENABLE_COLOR_FONT`）

- **Findings**: 「a あ 漢字 Hello ♥ 。 i W」（24px・256×96）を黒／赤の塗りで描いた読み戻しは、ＭＳ ゴシック・Yu Gothic UI × 3 方式のすべてで **バイト等価**（不透明画素 1,540〜1,671）。
- **Implications**: 5.1 は成り立つ。6.4 の証拠は同じテストの中の描き比べで足りる（golden の PNG は作らない）。

### R-3／R-5／R-6／R-8 色つきの字形（48px・128×128・`ENABLE_COLOR_FONT`）

| 形 | 書体 | `NONE` の色つき画素 | `COLOR` の色つき画素 | 不透明画素の色の種類 | 黒／赤の塗りでバイト等価 |
|---|---|---|---|---|---|
| 😀 | 両方 | 0 | 1,889（縦 1,805） | 1,109 | **等価** |
| 👨‍👩‍👧 | 両方 | 0 | 1,971（縦 1,898） | 1,232 | **等価** |
| 👍🏻 | 両方 | 0 | 1,258（縦 1,226） | 844 | **等価** |
| 🇯🇵 | 両方 | 0 | **0** | 1 | 不等価（単色＝塗りに従う） |
| 1️⃣ | 両方 | 0 | **0** | 1 | 不等価（単色） |
| か゚ | 両方 | 0 | 0 | 1 | 不等価（単色） |
| ❤️ | ＭＳ ゴシック | 0 | **0** | 1 | 不等価（単色） |
| ❤️ | Yu Gothic UI | 0 | 1,437 | 1,125 | **等価** |
| ☺️ | ＭＳ ゴシック | 0 | 0 | 1 | 不等価（単色） |
| ☺️ | Yu Gothic UI | 0 | 1,893 | 1,515 | **等価** |
| ♥ U+2665 | 両方 | 0 | 0 | 1 | 不等価（単色） |

- **Findings**: 色つきで出る形は黒／赤の塗りで読み戻しが等価＝塗りに従う層が無い（R-3）。不透明画素の色が 1,000 通りを超えるのでグラデーションの字形＝COLR v1（R-5。DC は `ID2D1DeviceContext4`／`5` へ QI 可）。縦書き 2 方式でも色つきの画素が出る（R-6）。異体字セレクタつきは基字を持つＭＳ ゴシックでは単色・持たない Yu Gothic UI では色つき（R-8）。国旗とキーキャップは色つきにならない（国旗は Windows に字形が無い・キーキャップは `1` を基字体が持ち U+20E3 が単色の囲みで代替される）。♥ U+2665 は単色（1.2）。
- **Implications**: 1.3／4.5 の判定は 😀・👨‍👩‍👧・👍🏻 で行う。裁定 3（版は OS 任せ）はそのまま。1.7 は事実として design.md に記録。国旗の色は Non-Goals に明記。

### R-2 budouy の塊の境界とクラスタ

- **Findings**: 「今日は家族👨‍👩‍👧で出かけた」→ 3 塊（ZWJ 列は塊の中）・「日本🇯🇵に行きたい」→ 2 塊（国旗は塊の中）・「いいね👍🏻と思う」→ 2 塊・「番号は1️⃣です」→ 2 塊（キーキャップは塊の中）・「好き❤️だよ」→ 1 塊・絵文字 3 つだけ → 1 塊。**「か゚き゚く゚の話」→ `か`｜`゚き゚く`｜`゚の`｜`話`（結合文字 U+309A の手前で切れる）**。
- **Implications**: 境界がクラスタの途中に落ちる実例がある。規則「クラスタは先頭バイトが属する塊へ」で `[1, 2, 1, 1]`（合計 5＝クラスタ数）。純粋関数に切り出して budouy 無しで全分岐を判定し、この例文は回帰の固定に使う。

### `TextLayerState` の `Send`

- **Context**: クラスタの型を `Rc<str>` にすると毎フレームの写しが割り当て無しになるが、`Send` を失う。
- **Sources Consulted**: `crates/areka-emo-text/tests/pipeline_test.rs` の `run_channel_pipeline`——`std::thread::spawn(move || { … state.borrow().clone() }).join()` で `TextLayerState` を値で返す（`std::thread::spawn` は戻り値に `Send` を要求する）。`actor.rs` の `TextLayerRuntime` は `!Send`（`Rc` 共有）だが、状態の型そのものは今日 `Send`。
- **Findings**: `Rc<str>` は既存テストのコンパイルを壊す。`Box<str>` は `Send` だが毎フレームの写しで割り当てる。`Arc<str>` は両方を満たす（単一スレッドでは参照数の増減だけ）。
- **Implications**: `TextItem::Glyph { text: Arc<str> }`・`PositionedGlyph { text: Arc<str> }`（設計決定 D1）。

## Architecture Pattern Evaluation

§3.1〜3.4 の表のとおり。採用は方針 C（既存の型と署名を替え、新しいものは兄弟ファイルへ）＋ ⒜ 文字列を持つ表現 ＋ `unicode-segmentation` ＋ `areka-sakura` の 1 定義点 ＋ 境界を後ろへ寄せる写像。

## Design Decisions

### D1: クラスタの表現は `Arc<str>`・`Copy` を外す
- **Context**: §7 項目 1。
- **Alternatives Considered**: ⒜ `Box<str>`（毎フレームの写しで割り当て）／`Rc<str>`（`Send` を失い `pipeline_test.rs` が壊れる）／⒝ 上限つき内蔵文字列（上限超えの縮退が要る）／⒞ `enum { Single(char), Multi }`（2 形の分岐が漏れる）／⒟ 範囲＋状態の文字列（署名が全部増える）。
- **Selected Approach**: `Arc<str>`。構築は `TextItem::glyph(&str)`。
- **Rationale**: 割り当て無しの写し＋`Send` 保持＋`From<&str>`・`Borrow<str>` で記憶の鍵にも使える。
- **Trade-offs**: 参照数の増減が原子的操作になる（単一スレッドでは無視できる）。
- **Follow-up**: 直書き約 180 か所の置換を機械的に行う（`TextItem::Glyph { ch: 'X' }` → `TextItem::glyph("X")`）。

### D2: 切り方は `unicode-segmentation`・定義点は `areka-sakura/src/cluster.rs`
- **Context**: §7 項目 2・3。
- **Alternatives Considered**: DirectWrite の `GetClusterMetrics`（純粋層から呼べない・国旗で数が違う）／両 crate で直接 `graphemes(true)`（定義点 2 つ）／自前の規則（裁定 4 に反する）。
- **Selected Approach**: `areka-sakura` に `clusters()`／`cluster_count()` を置き、`duration.rs` と `areka-emo-text/src/state.rs` が呼ぶ。依存の追加は `areka-sakura/Cargo.toml` の 1 行。
- **Rationale**: 依存の向き（`areka-sakura → areka-emo-text`）に沿い、「文字の単位」の正本が `CHAR_NOMINAL_MS` と同じ crate に揃う。
- **Trade-offs**: `areka-sakura` に小さなモジュールが 1 つ増える。

### D3: 分かち書きの写像は「先頭バイトが属する塊へ」
- **Context**: §7 項目 4・R-2。
- **Selected Approach**: 純粋関数 `assign_items_to_chunks(chunk_byte_lens, item_byte_lens, run_start)`。空になった塊は生まない。前提が崩れたときのはみ出しは最後の塊へ（記録なし）。
- **Rationale**: budouy が切った位置より前で切らないので既存の結果が不変。budouy 無しで全分岐を判定できる。

### D4: 照合用の判定は DirectWrite のクラスタを束ねる
- **Context**: §7 項目 5・R-1（国旗）。
- **Selected Approach**: 行の `GetClusterMetrics` を `length`（UTF-16）で累積し、各クラスタの UTF-16 長に達したところで束ねて `width` を合計。束の境界がクラスタの境界と一致しなければ理由つきで失敗。
- **Rationale**: 国旗を組から外さずに 6 形すべてを判定できる。

### D5: 6.4 は同じテストの中の描き比べ
- **Context**: §7 項目 6・R-4。
- **Selected Approach**: `ViewboxExecutor` に `#[cfg(test)]` の欄 `text_draw_options` と `set_text_draw_options_for_test`（`fail_next_render` と同型）。本番は定数 `TEXT_DRAW_OPTIONS`（`draw.rs`）。対照は executor と面を別々に作り、口は最初の描画の前に 1 度だけ呼ぶ（設計検証 2026-09-29 の指摘 1: 描画オプションは `line_fingerprint` に入らないので、同じ executor の描き直しは何も描かず空振りする）。
- **Rationale**: golden の PNG（新設・保守）が要らない。R-4 の等価が実測で確かめられた。

### D6: 代替フォントの実在は理由つきの失敗
- **Context**: §7 項目 7・R-7。
- **Selected Approach**: `require_segoe_ui_emoji(&FontCatalog)`（テスト支援・`panic!`）。

### D7〜D10: 判定に使う絵文字・字形の版・異体字セレクタ・その他の記録
- 😀・👨‍👩‍👧・👍🏻 を色の判定に使う（国旗・キーキャップ・ＭＳ ゴシックの ❤️ は単色）。字形の版は OS 任せ（この機械は COLR v1・Windows 10 は v0）。異体字セレクタは基字の有無で OS が決める（記録のみ）。国旗の色は Non-Goals。タグで割ったクラスタは繋がない（Non-Goals）。

## Risks & Mitigations

- `Copy` の除去がテストの直書きへ広く波及する — 機械的な 1:1 置換（`TextItem::glyph`・`text: "…".into()`）で行数を変えずに追随する。
- `unicode-segmentation` の版更新で切り方が変わる — 1 スカラー値の文字は影響を受けない。6 形のテストが赤になれば版の差として扱う。
- budouy の版更新で塊が変わる — 既存と同じ性質。「か゚」の固定は budouy の実物に依存するので、版を上げたら見直す。
- 1,000 行の見張り — `layout.rs`・`actor.rs` に関数を足さない。新しいコードは兄弟ファイル。見込みは design.md の表。
- Windows 10 の機械では字形が平面（COLR v0） — 判定は「色つきの画素が出る」だけなので同じ結果。

## References

- [Color font support - Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/directwrite/color-fonts)
- [UAX #29: Unicode Text Segmentation](https://www.unicode.org/reports/tr29/)
- [unicode-segmentation (crates.io)](https://crates.io/crates/unicode-segmentation) — 1.13.3・MIT/Apache-2.0
- `crates/areka-emo-text/src/lib.rs` 層規律・`crates/log-capture-kit/tests/file_length_guard_test.rs` 1,000 行の見張り
