# Design Validation: areka-P0-emo-text-line-height-canon

> 実施 2026-09-05・対象 `design.md`（コミット `32143efa`・HEAD＝`cursor-tag-canon` マージ済み）。非対話で実施し、設計の主張はすべて本ブランチのコードで file:line を実読して突合した。実測環境（SSP・モニタ DPI・fixture）も本機で確認した。

## レビュー要約

設計は、行送りの源が 4 系統に散っている現状（係数 1.25／em 素通し／実フォント比の行ボックス丈／帯の折衷）を `TextLayerConfig::line_pitch` と `font.height` の 2 値へ畳む方針が明快で、`visible_window`・`\_l` の解決層・fixture を触らない境界もコードと一致している。`draw.rs` の計測部を `metrics.rs` へ切り出す案は行数・依存方向ともに成立し、W13 `text-decoration-canon` の「`draw.rs` 分割が着手前提」とも継ぎ目が合う。一方で、(1) 実測の決定手順が `font.height` 1 水準だけでは「行間＝定数」と「行間＝フォント比例」を弁別できないこと、(2) 期待値の再導出台帳が `DWriteMetrics` のピッチに依存する 2 ファイル（うち 1 つは Requirement 7.4 が名指し）を落としていることの 2 点は、実装前に設計へ書き足す必要がある。いずれも設計の骨格を変えずに直せる。

## 検証した論点と証跡

### (a) `metrics.rs` 切り出しで `draw.rs` が 1,000 行を下回るか・W13 との衝突

- 移設対象は実在する: `create_text_format` `draw.rs:308-337`・`try_create_format` `:340-354`・`DWriteMetrics` `:356-492`・`measure_line_box_ratio` `:494-538`。合計約 220 行で、980 − 220 ＋ 再輸出／doc ≈ 770〜800 行（設計の見込み ≈ 800 と整合）。
- 兄弟テストの `#[path]` 取り込みは `draw.rs:970-980` にあり、`draw_format_metrics_tests.rs` の `DWriteMetrics` 系を `metrics_tests.rs` へ移す設計は取り込み口の付け替えで済む。
- `impl GlyphMetrics` は crate 内の 2 か所だけ（`draw.rs:441`・`layout.rs:122`）。`line_pitch_factor` の外部参照は `examples/emo-text-typewriter-demo.rs:227`（注記のみ）と `draw_format_metrics_tests.rs:404`・`state_cue_apply_tests.rs:596` で、Revalidation Triggers の列挙と一致する。他クレート（`crates/areka`）に `DWriteMetrics`／`line_pitch(` の参照は無い。
- `text-decoration-canon` brief（`:58`・`:87`・`:94`）は「`draw.rs` 分割が着手前提」「基盤相＝`draw.rs` 分割＋decode 腕＋per-run 配管」と述べるだけで分割の形は未定。計測部だけを先取りし、`derive_dwrite_em` を純関数として置く設計は、decoration の `\f[height,N]` が同じ導出を per-run で使える形になっており衝突しない。相互登記（design §11.3）も妥当。

### (b) `highlight_band_extent`／`line_pitch_factor` の撤去と `GlyphMetrics` 2 口化

- `line_box_height` の製品コード消費点は `actor.rs:787` の 1 か所のみ（grep で確認）。`expand_overhang_for_band`（`viewbox_draw.rs:737-750`）は `band_extent − font_height` の超過分を足すだけで、帯＝`font_height` なら恒等——設計 DD-6 の主張どおり。
- 完了 spec `emo-text-layer` の `design.md:511-515` の trait 定義は `advance`／`line_pitch` の **2 口**である。`line_box_height` は後続 spec が足した口なので、2 口へ戻す設計は元の正典表と矛盾しない。
- `cursor-tag-canon` の解決層 `cursor_tag.rs:120-127`（`unit_coefficient`）は `line_pitch` と `font_height` を引数で受けるだけで式を持たず、配線 `layout.rs:553-559`（`CursorBasis`）も同様。係数の値だけが差し替わる（R4.3・R9.2）はコードで裏が取れる。
- R3.5（同じ一つの源）・R5.6（帯とヒット帯が同じ源・descent が切れない）・R7.2（退役は個別記録）・R8.7（旧式へ戻すと赤）・R9 は設計内で対応づけられている。ただし帯＝セル丈は「グリフのインクがフォントの descent の内側にある」ことに依拠する。`menu.pasta` の文字列（かな・漢字）では成り立つ見込みだが、これは R5.6 の実フォント読み戻しテストで固定される前提として、退役台帳 D に一言添えるべき（軽微）。

### (c) 二段折返し（soft＝`wordwrappoint`／hard＝`validrect`）の完全性

- 現状のゲート③（`layout.rs:386-434`）は `threshold` 1 値のみ。CharByChar 腕（`:393`）・塊先頭の `cap_rem`／`cap_full`（`:410-411`）・長大塊と非被覆の腕（`:426`・`:431`）の 4 か所に soft 判定があり、設計の「hard は配置直前に必ず通す」は塊内（`:398-401`・追加判定なし）にも効く形で正しい。塊内で hard が発火すると `seg_remaining` を保ったまま行送りする設計は「塊は途中分割されない」不変条件（`layout.rs:340-342`）の例外だが、soft > hard の粗いバルーン定義でしか起きず `debug!` を残す方針は妥当。
- 縦書き: `TextRegion::resolve`（`region.rs:250-258`）が `wordwrappoint.y`／`bottom` を軸解決済みで、`layout` には mode 分岐が無い（`:316-321` の軸読み替えのみ）。`inline_limit` を `right`／`bottom` で持たせれば 3 方向とも同じ式で回る。
- `visible_window`（`layout.rs:634-680`）は行矩形の列だけを入力にし、折返し規則を参照しない。二段折返しは行数を増やし得るが判定式には触れない（R9.1）。相方側では α の文字送り（≈ 21.05 × 3 ＝ 63.2 → x 164..227.2 ≤ 240）で「閉じる」は折り返されず行数も変わらない。本体側は soft 351 ≤ hard 356 で出力不変（R6.4）。

### (d) SSP 実測手順の実行可能性と決定手順の弁別力

- 本機で確認: `C:\wintools\ssp\ssp.exe` FileVersion 2.8.83.3000／ghost `emo`（`shiori,emo.dll`・`dic/menu.pasta` 不在）／balloon `emo2-kakukaku` あり／SSP 側 `descript.txt` と repo fixture の差は設計どおり 2 点（SSP 側のみ `origin.x,0`・`origin.y,0`、repo 側のみ `budoux_newline,1`）／モニタ DPI は DPI 対応プロセスから 192・144 の 2 面／先例道具 `completed/areka-P0-scope-chain-gap/tools/measure-ssp-rects.ps1` 実在。
- 決定手順 1（α／β の弁別）は、インク丈の差が約 25%（em 21.05 対 28・k 1.5 で 9px 前後）に対し許容幅 ±1〜2px なので確実に弁別できる。
- 決定手順 2（行間の源）は **1 水準の `font.height`（28）だけで測る**ため、実測ピッチが 28 でないとき「定数 c（α2）」と「フォント高さに比例する量（α1 以外の比例、例 `ceil(28 × 1.1) = 31`）」を区別できない（Critical Issue 1）。
- 台本を SSTP で送る案（DD-14）は成立するが、`\![change,balloon,…]` を SSTP 経由で受け付けるかは SSP 側の設定に依る。受け付けない場合は SSP の右クリックメニューで手動切替すればよく、手順書に代替を 1 行足すだけでよい（軽微）。

### (e) `FixedMetrics` 仮想行間 3 で純粋層の期待値が変わらないか

- 純粋層テストの共通前提は font 10（`layout_test_support.rs:42`・`viewbox_test_support.rs:13`・`cursor_tag_test_support.rs:20-22`・`layout_cursor_*_tests.rs` の各 doc・`viewbox_plan_commit_tests.rs:357` の `PITCH 13`・`tests/pipeline_test.rs:196` の pitch 13）と font 12（`actor_choice_contract_tests.rs:154` の `ceil(12 × 1.25) = 15`）で、`10 + 3 = 13`・`12 + 3 = 15` の同値が成り立つ。`layout_visible_window_tests.rs:10-80`（下端 10/23/36/49・`block_offset −13`）・`layout_cursor_overflow_tests.rs:113-166`（行矩形の厚み＝`font_height` は不変）も同値。
- 例外は `tests/scale_invariance_test.rs`（`FixedMetrics`・font 40・`:363-391`）で、pitch 50 → 43・`block_offset −50 → −43` に変わる。3 行目の下端は 46 + 2×43 + 40 = 172 > 168 で依然あふれるため「行数不変」の検査は保たれる——設計の台帳 B の記述どおり。
- `fixed_metrics_line_pitch_ceils_fractional_values`（`layout_wrap_tests.rs:24`）は `ceil` の検証が退役するため名前と本文の差し替えが要る（台帳 A に記載済み）。R7.2「名前を減らさない」との整合は退役台帳への記録で担保される。

### (f) 要件トレーサビリティと再導出台帳

- 要件 10 本・受入基準 64 項目（1.1〜1.7／2.1〜2.5／3.1〜3.10／4.1〜4.4／5.1〜5.6／6.1〜6.9／7.1〜7.6／8.1〜8.7／9.1〜9.6／10.1〜10.4）は design の Requirements Traceability 表にすべて行がある。
- 再導出台帳 A〜D は research §3.3 の一覧（純粋層 21・COM 5・`tests/` 7・example 1）を被覆している。しかし §3.3 自体が **`DWriteMetrics` のピッチに数値依存する 2 ファイルを落としている**: `src/draw_oracle_tests.rs:430`（「font 10 → pitch 13・行下端 10/23/36/49——validrect.bottom 40 で 4 行目があふれる」・実 `DWriteMetrics`・ＭＳ ゴシック）と `src/viewbox_draw_oracle_regression_tests.rs:11,:112`（「font 28px・pitch 35px」「行1セル 0..28・行間 28..35・行2セル 35..63」・実 `DWriteMetrics`）。後者は Requirement 7.4 が名指しする画素等価比較であり、design は 7.4 を「台帳 B（live_diff）」にしか対応づけていない（Critical Issue 2）。
- `region.rs` の in-file テスト（`:439-794`・kero fixture の 2 層マージ `:536` を含む）と `tests/shipped_fixture_region_test.rs` は `TextRegion::resolve` を直接呼ぶ。設計が `resolve` の末尾に `warn!` を足すため、これらがログ件数を固定していれば赤になる。台帳の対象外（行送り非依存）だが「警告 1 件の追加」による再導出として台帳に 1 行加えるべき（軽微）。

## Critical Issues（最大 3 件）

🔴 **Critical Issue 1**: 行間の既定値を「定数」と「フォント比例」で弁別できない実測設計
**Concern**: design §4.2 決定手順 2 は `gap(k) = pitch_ssp(k) ÷ k − 28` を `font.height,28` の 1 水準でしか求めない。実測ピッチが 28 なら問題ないが、開発者の観測（≈31px/行）どおり 3px 前後の差が出た場合、「行間＝定数 3（α2）」と「行間＝フォント高さに比例（例 `28 × 0.1`・α1 以外の比例）」は 28 では同じ 31 になり区別できない。ukadoc の `1lh = 1em + 行間` は形を定めるだけで、「行間」が定数か比例かは沈黙している。
**Impact**: 弁別できないまま `TextLayerConfig { line_gap: 定数 }` を採ると、`font.height` が 28 以外のバルーン（既定 12・`number.font.height,12` 等）と `\_l[N lh]` の着地で SSP と食い違う。R1.3「行間の既定値を実測から確定」と R1.5「推測で埋めない」の両方に抵触する。
**Suggestion**: §5.3 の台本に **第 2 の `font.height` 水準**を 1 本足す。SSP はさくらスクリプトの `\f[height,N]` を受けるので、S8: `\1\f[height,14]あ漢Hg\nあ漢Hg\f[height,56]\nあ漢Hg\nあ漢Hg\e` のように 14／56 の行を並べ、行ごとの `gap = pitch − height` が一定なら定数、`height` に比例するなら比例、と決定手順 2 に 1 行足す（`\f[height]` が使えない場合は複製バルーンをもう 1 つ `font.height,14` で置く）。§4.2 の候補表に「α3: 行間 ＝ `font.height × 定率`」を加え、`TextLayerConfig` の型（`line_gap` 定数か `line_gap_ratio` か）は実測後に確定する旨を DD-2 に明記する。
**Traceability**: R1.3・R1.5・R3.5・R4.1
**Evidence**: design §4.2「行間の源」表・決定手順 2、§5.3 手順 4（台本 S1〜S7）、DD-2

🔴 **Critical Issue 2**: 再導出台帳が `DWriteMetrics` ピッチ依存の 2 ファイルを落としている（うち 1 つは R7.4 が名指し）
**Concern**: `src/draw_oracle_tests.rs:430`（ＭＳ ゴシック 10・pitch 13 → 新式では 10。下端が 10/20/30/40 になり `validrect.bottom 40` で **4 行目があふれなくなる**＝テストの前提が崩れる）と `src/viewbox_draw_oracle_regression_tests.rs:11,:112`（font 28・pitch 35 → 28。「行間 28..35」の領域そのものが消え、行境界の欠け診断の意味が変わる）が、research §3.3 の 30 ファイルにも design の台帳 A〜D にも無い。design は「着手時に 30 ファイルすべてを `rg` で再確認」としており、30 の外側は見ない手順になっている。
**Impact**: R7.3「前提が崩れて緑のまま意味を失うことを防ぐ」と R7.4「`viewbox_draw_oracle_regression_tests.rs` が両側とも同じ寸法で動くことを確認」が、台帳の外で起きる。実装中に赤になったテストを場当たりで直す圧力が最も強い箇所でもある。
**Suggestion**: 台帳の入口を「§3.3 の 30 ファイル」から「`rg "1\.25|1\.33|37\.24|line_pitch|line_box|band|pitch" crates/areka-emo-text/{src,tests,examples}` の全ヒット」へ改め、上記 2 ファイルを台帳 C（容量前提）に加える。`draw_oracle_tests.rs:430` は `validrect.bottom` を 35 等へ導き直して「4 行目があふれる」前提を保ち、`viewbox_draw_oracle_regression_tests.rs` は行間 0 でも診断が意味を持つ寸法（行 1 セル 0..28・行 2 セル 28..56）へ前提コメントごと書き直す。Requirements Traceability の 7.4 の行に同ファイルを明記する。
**Traceability**: R7.1・R7.3・R7.4
**Evidence**: design「Testing Strategy › 再導出台帳」表・同末尾の再確認手順・Requirements Traceability 7.4 の行

（3 件目に相当する重大な問題は見つからなかった。）

## 軽微な所見（設計ディスカッションでの「自明な修正」候補）

1. `TextRegion::resolve` へ `warn!` を足すと、`region.rs` の in-file テスト（`:439-794`・kero 2 層マージ `:536`）と `tests/shipped_fixture_region_test.rs`・`src/cursor_tag_test_support.rs:95` のうちログ件数を固定しているものが赤になり得る。台帳に「警告 1 件の追加による再導出」として 1 行加える。
2. 帯＝セル丈（DD-6）は「グリフのインクがフォントの descent の内側にある」ことに依拠する。退役台帳 D の根拠欄に、この前提を R5.6 の実フォント読み戻し（「閉じる」「もどる」）で固定する旨を添える。
3. §5.3 手順 2 の `\![change,balloon,…]` は SSP の SSTP 設定で拒否され得る。手動切替（右クリックメニュー）の代替を 1 行足す。
4. Revalidation Triggers の「`GlyphMetrics` trait を実装・消費する `examples/emo-text-layer/`・`tests/viewbox_blit_spike.rs`」は、実装ではなく `DWriteMetrics` の消費（`drive.rs:223,:371-376`）。文言を「消費」に直す。
5. R2.4 の機械検査 `rg -v "旧式|本仕様で改訂|履歴"` は、現行式を述べる行に偶然「履歴」等の語が含まれると素通りする。除外語を行頭マーカー（例 `> 旧式:`）に限定するか、除外後の残り行を目視で 1 度確認する手順を添える。
6. 台帳 B の「既定フォント（比 1.0）ではグリフ描画が不変なのでピッチ由来の差だけが出る」は Yu Gothic UI を使う `tests/emo2_fixture_e2e_test.rs`・`tests/choice_fixture_test.rs`・`draw_format_metrics_tests.rs:417-450` には当てはまらない（文字送りも 28 → 21.05 に変わる）。当該 3 ファイルを「文字送りも変わる」側として台帳に区別して書く。
7. `FontBinding` は actor ごと単一フォント前提。W13 で `\f[name]`／`\f[height]` が per-run になったとき `cell_ratio` を family ごとに引く必要がある——`derive_dwrite_em`・`measure_cell_metrics` を family 単位の純関数として切っておく現設計はその布石になるので、decoration brief への登記文にその旨を 1 句足す。

## 設計の強み

1. **「同じ一つの源」を構造で表す**: 行送り・行ボックス・行矩形・帯・`lh` 係数の 5 寸法を `TextLayerConfig::line_pitch` と `font.height` の 2 値から導き、恒等になった口（`line_box_height`・`highlight_band_extent`・`expand_overhang_for_band`・`FIXED_LINE_BOX_RATIO`・`line_pitch_factor`・`create_text_format`）を 6 点撤去する。撤去対象の消費点はコードで実在を確認できた（`actor.rs:787` の 1 点など）。「2 つの em」を `FontBinding { font_height, dwrite_em }` で型分離する DD-3 も取り違えを塞ぐ良い判断である。
2. **境界の固定が実装に落ちている**: `visible_window` の判定式・`\_l` の解決層・fixture を触らない Out of Boundary が、`cursor_tag.rs:120-127`／`layout.rs:553-559`／`layout.rs:634-680` の現状構造でそのまま成り立つことを設計が file:line で示し、R8.7 の対照（旧式で赤）を製品コードに口を残さず実 fixture 経路のテスト専用実装で組む方針（DD-11）も規律に沿う。

## 最終判定

**GO**（条件付き）。

**根拠**: 既存アーキテクチャ（純粋層／COM 層／結線層の一方向依存・`GlyphMetrics` 注入点・`SetTransform` 一点適用）との整合が取れ、要件 64 項目がすべて設計要素へ対応づけられ、実測環境（SSP 2.8.83・ghost・balloon・192／144 DPI）が本機に揃っている。Critical Issue 1・2 はいずれも設計の骨格を変えず、§4.2／§5.3 への台本 1 本の追加と台帳の入口の書き換えで直せる。

**次のステップ**:
1. 設計ディスカッションで Critical Issue 1（第 2 の `font.height` 水準の台本と候補 α3・DD-2 の追記）と Critical Issue 2（台帳の入口を crate 全域の `rg` へ・2 ファイルを台帳 C へ・7.4 のトレース修正）を design.md に反映する。
2. 軽微な所見 1〜7 のうち自明なもの（1・4・6）は同時に反映し、残りは判断を記録する。
3. その後 `/kiro-spec-tasks areka-P0-emo-text-line-height-canon` へ進む。実装の第 1 タスクは SSP 実測（意味論の確定）であり、実測が α 以外を示したときは R1.5 の手順で本書 §4 を改訂してから実装へ進む。
