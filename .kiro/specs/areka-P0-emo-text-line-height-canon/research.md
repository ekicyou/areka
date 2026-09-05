# Research & Gap Analysis: areka-P0-emo-text-line-height-canon

> 作成 2026-09-05（要件確定後のギャップ分析・`kiro-validate-gap`）。対象ブランチ HEAD `36d1c323`（`cursor-tag-canon` マージ済み）。本文書は「情報と選択肢」を提示するものであり、最終決定は要件ディスカッション／設計フェーズで行う。file:line はすべて本ブランチで実読して確認した値である。

## 0. 分析の要約

- **根因は 1 か所ではなく「行送りの源」が 3 系統に散っている**: ⑴ 係数 1.25（`state.rs:59-66`）⑵ `font.height` を DirectWrite の em サイズへ素通し（`draw.rs:308-353`）⑶ 実フォント比 `ascent+descent ÷ upem` による行ボックス丈（`draw.rs:489-537`）。行矩形の厚み（`layout.rs:780-803`）は `font_height` そのもの、選択肢の帯（`choice.rs:129`・`actor.rs:785-788`）は ⑵⑶ の折衷、と 4 つの寸法がそれぞれ別の式で決まっている。Requirement 3.5「同じ一つの源」は現状の構造に対する是正である。
- **SSP 実測なしには意味論を決められない**（ukadoc `font.height` は「高さ方向の大きさ（単位はピクセル）」のみ・`\_l` の `lh` は「1em＋行間」のみ・行間の既定値は正典沈黙・ukadoc MCP に `linespacing` の項目なし）。ただし候補ごとの**予測値**は今すぐ計算でき（§4）、実測 4 量（ピッチ・行ボックス丈・ベースライン・インク丈）で候補を一意に弁別できる見込みが高い。
- **既存の決定論テストは 30 ファイルが行送り値に数値依存**（§3.3 の一覧）。期待値の再導出は機械的だが量が多く、`draw.rs` は **980/1,000 行**（残 20 行）で、意味論の追随を `draw.rs` 内で完結させる余地がほぼ無い（§6 の境界判断）。
- **本体側バルーン `emo2` の行容量が 3 行→4 行へ変わり得る**（§4.4）。相方側の症状を直す式は本体側の行数も変えるので、Requirement 5.5 の「SSP と同じ行数」は本体側でも実測が要る。
- 「閉じる」右端欠け（Requirement 6）は**供給面の寸法を文字描画範囲ちょうどに固定している構造**（`actor.rs:663-665`・`canvas.rs:319-321`・`surface.rs:188-193`）に由来し、行送りとは独立に直せる。候補は 4 つ（§5）。

## 1. 現状調査（Current State Investigation）

### 1.1 行送り・文字寸法に関わる資産の所在（Requirement-to-Asset Map の基礎）

| 量 | 現在の式 | 定義点（file:line） | 消費点 |
|---|---|---|---|
| 行送りピッチ `line_pitch` | `ceil(font.height × 1.25)` | 係数の正本 `crates/areka-emo-text/src/state.rs:59-66`（`TextLayerConfig::line_pitch_factor`）・COM 層実装 `draw.rs:477-479`・純粋層の決定論用 `layout.rs:131-133`（`FixedMetrics`・`TextLayerConfig::default()` を読む） | 配置 `layout.rs:314`（自動折返し・改行 `×Σratio`・`\_l` の `lh` 係数 `layout.rs:553-558`→`cursor_tag.rs:120-125`）・帯の上限 `actor.rs:788` |
| 行ボックス丈 `line_box_height` | `font.height × (ascent+descent)/upem`（実フォント face metrics・**lineGap は含まない**） | `draw.rs:489-537`（`measure_line_box_ratio`・format が束縛したフォントを辿る）・純粋層仮想値 `layout.rs:120,135`（`FIXED_LINE_BOX_RATIO = 1.33`） | 帯 `actor.rs:787`／`choice.rs:129`（`clamp(box, font_height, max(font_height, pitch))`）・ダーティ矩形の拡張 `viewbox_draw.rs:225-250` |
| 行矩形の厚み（あふれ判定の入力） | `font_height`（em ボックス丈・実フォント非依存） | `layout.rs:780-803`（`finish_line`・横書き `bottom = block_pos + font_height`） | `visible_window` `layout.rs:634-680`（判定 `:653-654`・最小スキップ `:665`）・`canvas.rs:281-321`（住人の寸 `:307`） |
| DirectWrite フォントサイズ | `font.height` の値そのまま（em サイズ） | `draw.rs:308-353`（`create_text_format`→`try_create_format(…, font_size)`）・`ResolvedFont::resolve` `draw.rs:184-211`（欠落→12・0→警告＋12） | 行 TextLayout の箱 `draw.rs:601-620`（行送り軸の箱寸＝`font_height`・`max_height`） |
| 実測インクはみ出し | `GetOverhangMetrics`（行ボックス＝`font_height` 箱からのはみ出し） | `draw.rs:620-680`（`measure_line_overhang`） | `viewbox.rs:631-661`（`resident_rect`・ダーティ矩形） |
| 供給面の寸 | `ceil(validrect 寸 × k)`・offset＝validrect 原点 × k | `actor.rs:663-671`・`surface.rs:188-193`（`TextSurface::attach`） | `canvas.rs:319-321`（canvas 寸＝validrect 寸・validrect-local 空間）・`viewbox_draw.rs`（面全域の plan） |
| 折返し閾値 | `resolve(wordwrappoint.x)`（負値＝画像幅基準・**validrect へクランプしない**） | `region.rs:251-258,326-334`（`resolve_or`・未指定は `right`/`bottom` へ縮退・`debug!`） | `layout.rs:315`（`threshold`）・固定値 `tests/shipped_fixture_region_test.rs:147`（kero 254） |

派生する定数（`font.height,28`・Yu Gothic UI・実測 `draw_format_metrics_tests.rs:417-450`）: `upem 2048`・`ascent 2210`・`descent 514` → 行ボックス比 **1.3301**・28em の行ボックス丈 **37.24px**・ascent 比 0.8113。

### 1.2 データフロー（1 フレーム・`present_actor` `actor.rs:640-830`）

```
ResolvedBalloonText{font, region, mode, wrap}
  → DWriteMetrics::new(factory, font, mode, config)          draw.rs:393-417（format 生成→line_box_ratio 実測）
  → LayoutEngine::layout_with_cursor_warn(items, …, font.height, &metrics, wrap)
        pitch = metrics.line_pitch(font_height)              layout.rs:314
        行矩形の厚み = font_height                            layout.rs:780-803
        \_l の lh/em 係数 = (pitch, font_height)             layout.rs:553-558 → cursor_tag.rs:120-125
  → visible_window(lines, region, mode)                      layout.rs:634-680（式は本仕様で不変・R9.1）
  → band_extent = highlight_band_extent(font.height, box, pitch)   actor.rs:785-788 / choice.rs:129
  → ContentCanvas::from_layout(lines, region, mode)          canvas.rs:281-321（validrect-local・寸＝validrect 寸）
  → decorate_canvas(…, band_extent) → ViewboxExecutor::render（供給面＝validrect 寸・k は SetTransform 一点）
  → derive_hit_rows(lines, segments, mode, region, band_extent)
```

`\_l` の解決層（`cursor_tag.rs:120-125` `unit_coefficient`）は `line_pitch` を引数で受けるだけで係数の式を持たない——Requirement 4.3「係数の値だけが差し替わる」は現構造でそのまま成り立つ（`cursor-tag-canon` の層分けが効いている）。

### 1.3 症状の再現値（決定論・本ブランチで再計算）

- 相方側 `emo2-kakukaku`（`balloonk0s.txt:4-7`・画像 288×203）: 文字描画範囲 (24,40)-(240,133)・高さ 93・折返し閾値 254（`tests/shipped_fixture_region_test.rs:130-147` が固定）。`menu.pasta:15` は 3 行（`\n` 1 回＋`\_l[5em,2lh]`）。ピッチ 35 で 3 行目の行矩形 y110..138 → 138 > 133 → `visible_window = {1, −35}` → `draw.rs:773`（比較用オラクル）／viewbox 経路とも先頭行を描かない。
- 「閉じる」（`\_l[5em,…]`＝x 24+140=164 起点・3 文字 ≈ 84px → x164..248）: 閾値 254 までは折り返さない（`layout.rs:315`）が、供給面が 216px 幅（validrect 24..240）なので 240..248 の 8px が面の外。
- `menu.pasta:33`（4 項目・3 行目 `\_l[5em,2lh]` で「たまーに」と同じ行に「もどる」）・`:62`（2 項目・`\_l[5em,2lh]` で 2 行目が空・3 行目に「もどる」）も同じ 3 行目の行矩形 y110..138 であふれる。

### 1.4 慣習・規律（設計が従うべきもの）

- **層規律**（`lib.rs:10-26`）: 純粋層（`state`／`region`／`cursor_tag`／`layout`／`canvas`／`viewbox`）は `windows` 非依存。フォント実寸（ascent／descent／lineGap）は COM 層（`draw.rs`）でしか取れないため、「実フォント比で em を導く」候補は COM 層の責務になる。純粋層のテストは `FixedMetrics` の仮想値で回す。
- **`GlyphMetrics` trait**（`layout.rs:80-103`）: `advance`／`line_pitch`／`line_box_height` の 3 口が唯一の注入点（完了 spec design D8）。式の変更はこの trait の実装 2 つ（`DWriteMetrics`・`FixedMetrics`）へ閉じられる。
- **log-first**（`.kiro/steering/logging.md`・記憶 areka-log-first-no-silent-failure）: face metrics 取得失敗は現状 `warn!`＋係数へ縮退（`draw.rs:404-410`）。Requirement 3.9 はこの形を保ちつつ縮退値を「確定した式の既定値」へ差し替える要求。
- **テスト配置**: 本番ファイルの兄弟 `<stem>_<theme>_tests.rs`（`#[path]` include）＋ `tests/` の統合テスト。1 ファイル 1,000 行の見張り（`crates/log-capture-kit/tests/file_length_guard_test.rs`）。
- **ファイル残量**（`wc -l`）: `draw.rs` **980**・`layout.rs` 890・`actor.rs` 879・`region.rs` 863・`choice.rs` 550・`state.rs` 499。テスト側も `cursor_tag_resolve_tests.rs` 871・`layout_cursor_tests.rs` 842・`draw_oracle_tests.rs` 784・`layout_segmented_tests.rs` 779 が大きい。
- **正典表の所在**: 完了 spec `.kiro/specs/completed/areka-P0-emo-text-layer/design.md:725`（補足正準「行送りピッチ」）・`:736`（DPI/スケール契約表「フォントサイズの写像」）・`:513`（`LayoutParams` の doc に同じ式）・同 `research.md:200`（リスク登記）。`doc/COMPAT_ARCHITECTURE.md` §8（`:122` 以降）は 1 行 1 裁量の表で、実機確定の先例（`windowposition.x` 符号規約 `:146`・複数スコープ連鎖 `:153`）が「参照実装 SSP を受理オラクルとした」書式を持つ——本仕様の行はこの書式に揃えられる。

## 2. 外部依存の知見（設計フェーズで実測により裏を取る前提のメモ）

- **GDI `LOGFONT.lfHeight`**（SSP が GDI 描画である前提のとき）: 正値＝**セル丈**（`tmHeight`＝ascent＋descent）に一致させる／負値＝**文字丈**（`tmHeight − tmInternalLeading`＝em に相当）に一致させる／0＝既定。ukadoc の「高さ方向の大きさ（ピクセル）」だけではどちらか決まらない。SSP がどちらの符号で `CreateFont` を呼ぶかは実測（インク丈・ベースライン）から逆算する。
- **DirectWrite**: `DWRITE_FONT_METRICS` の `ascent`／`descent`／`lineGap`（design units）。既定の行送り（`DWRITE_LINE_SPACING_METHOD_DEFAULT`）は `(ascent + descent + lineGap) × em ÷ upem`、ベースラインは `ascent × em ÷ upem`。現行 `measure_line_box_ratio`（`draw.rs:499-537`）は **lineGap を含めていない**——SSP のピッチが `tmHeight + tmExternalLeading` 相当なら lineGap の扱いが設計判断になる。Yu Gothic UI の lineGap 値は未確認（Research Needed）。
- **GDI と DirectWrite のラスタライズ差**: GDI はヒンティング＋整数ピクセル寸、D2D は既定でグレースケール／ClearType のサブピクセル配置。同じ em でもインク丈が ±1px ずれ得るため、Requirement 3.3 の「k=1 で ±1px」はぎりぎりの許容幅。ベースライン（3.4）は整数丸めの方式（GDI は `tmAscent` 整数）で 1px 動き得る。
- **wintf の DirectWrite ラッパ**（`crates/wintf/src/com/dwrite.rs`）: `DWriteFactoryExt::create_text_format`／`DWriteTextLayoutExt::get_overhang_metrics` 等はあるが `SetLineSpacing` 相当は無い（候補 C を採る場合は追加が要る）。
- **既存の SSP 実測道具**: `.kiro/specs/completed/areka-P0-scope-chain-gap/tools/measure-ssp-rects.ps1`（Per-Monitor v2・窓矩形の読み取り専用）。**画素を読む道具は無い**（`PrintWindow`／`BitBlt` の利用実績はリポジトリに無い）。Requirement 1.1 の画素読み取りには新たな道具（スクリーンショット＋画素走査）が要る（Research Needed・§7）。

## 3. 要件実現性分析（Requirements Feasibility）

### 3.1 要件ごとの資産・ギャップ

| Req | 技術的に必要なもの | 既存資産 | ギャップ（Missing／Unknown／Constraint） |
|---|---|---|---|
| 1（SSP 実測） | SSP を既定設定で起動し、同バルーンで複数行を表示させ、k=1／k=2 で画素を読む手順と道具 | SSP 本体 `C:\wintools\ssp\ssp.exe`（実在確認済み）・窓矩形計測 `measure-ssp-rects.ps1`・過去の実機手順（`balloon-offset-dpi` research §13・`scope-chain-gap` `ssp-oracle-notes.md`） | **Missing**: 画素読み取り道具・「同じ文字列を複数行並べる」台本（emo2 実物 `menu.pasta` は 3 行で足りるが、参照グリフのインク丈を測るには単純な文字列が要る）。**Unknown**: SSP の既定「行間」設定が 0 か否か・DPI 2 水準の用意（DISPLAY1 192dpi／DISPLAY2 144dpi の環境が `balloon-offset-dpi` 当時に在った）。 |
| 2（正典表・裁量記録） | 完了 spec design／research・COMPAT §8・doc コメント・steering の改訂 | 改訂対象の行を特定済み（§1.4）。`1.25` の出現箇所を洗い出し済み（§3.4） | **Constraint**: 完了 spec のアーカイブは原則非改変だが、要件 2.1 が「注記つきで置き換え」を明示（先例: COMPAT §8 `:147,:153` は「アーカイブ済み spec は非改変とし上書きの事実を本表に記録」）——**要件 2.1 と先例の非改変方針が食い違う**（設計判断 #9）。 |
| 3（SSP と一致） | 意味論確定後の式を `GlyphMetrics` 2 実装・format 生成・行矩形・帯へ反映 | trait 注入点（`layout.rs:80-103`）・`TextLayerConfig`・`measure_line_box_ratio` | **Missing**: 「font.height からの em 導出」の口（現状は素通し）・lineGap の取得・縮退既定値。**Constraint**: `draw.rs` 残 20 行。3.10 の k 不変は `SetTransform` 一点適用で構造的に保たれる（変更不要）。 |
| 4（`\_l` 追随） | 係数値の差替えのみ | `cursor_tag.rs:120-125` が引数で受ける | ギャップなし。テスト側 `cursor_tag_test_support.rs:21`（`LINE_PITCH = 13` の doc「ceil(10×1.25)」）の注入値と doc の更新のみ（7.5）。 |
| 5（メニュー 3 台本） | 実 fixture × 実 parser→compile→state→region→layout の決定論テスト | `tests/emo2_fixture_e2e_test.rs`（本体側 `balloons0s.txt` × `menu.pasta:15`・headless readback）・`tests/shipped_fixture_region_test.rs`（kero 領域固定）・`crates/areka/src/emo2_boot/spine_conformance_script.rs:440-460`（3 台本の写し・ただし配置は見ない） | **Missing**: 相方側 `balloonk0s.txt` で `:15`／`:33`／`:62` の 3 台本を通し `first_visible_line == 0` を固定するテスト。5.5 の本体側行容量は **SSP 実測が要る**（§4.4）。5.6 の帯は `highlight_band_extent` の再導出。 |
| 6（右端欠け） | 供給面の寸法規則の裁定と実装・警告 1 回・residue 台帳登記 | `actor.rs:663-671`・`canvas.rs:319-321`・`surface.rs:188-193`・`region.rs:251-258` | **Unknown**: SSP が同条件で右端を欠かせるか（6.1 は実測で埋める）。**Missing**: 供給面を閾値まで広げる規則（§5 候補 3）・warn-once の置き場（`TextRegion::resolve` は純粋で複数回呼ばれる）・`balloon-canon-residue` brief への登記（同 brief は番号付き Problem 項目の列挙形式・現在 10 項目＋bvc 追加登記）。 |
| 7（既存テストの再導出） | 30 ファイルの期待値を式から計算し直す | §3.3 一覧 | **Constraint**: `assert_eq` を範囲判定にしない・本数を減らさない。**容量前提を持つテスト**（`viewbox_scroll_test.rs:60-80` のコンパイル時検査 `FILL_LINES`／`PITCH`・`viewbox_draw_live_diff_tests.rs:455,476` の `2P+F ≤ block ≤ 3P+F`・`examples/emo-text-layer/scenario.rs:9-21` の「横書き容量 3 行・縦書き 9 列」）は寸法の導き直しが要る。 |
| 8（新規テスト） | 3 台本×2 折返し方式・SSP 実測定数の固定・実フォント読み戻し（2 行のインク非重なり）・旧式へ戻すと赤になる対照 | `tests/draw_readback_test.rs`（`ink_min`／`opaque_count` の読み戻し補助 `draw_oracle_tests.rs:149-160`）・`emo2_fixture_e2e_test.rs` の実 fixture 読み込み経路 | **Missing**: 実 fixture の相方側読込み（`balloonk0s.txt` の 2 層マージは `shipped_fixture_region_test.rs` に実装済み・流用可）。8.7 の対照は係数を旧値へ差し替える口が要る（`TextLayerConfig` の field を残すか、テスト専用の注入か＝設計判断 #6）。 |
| 9（境界固定） | `visible_window` 非改変・fixture 非改変・1,000 行以下 | `layout_cursor_overflow_tests.rs:113-166`（追加登記 4 の現状値固定・値は行矩形の厚み変化で再導出） | **Constraint**: `draw.rs` 980 行——em 導出＋lineGap 取得を `draw.rs` に足すと 1,000 を超える公算が大きい（設計判断 #8）。 |
| 10（引き渡し） | e2e 記録 §13.2 の欄・roadmap W12 A′・decoration brief の相互参照 | 対象箇所を特定済み（`acceptance-record.md:681-684`・`roadmap.md:73,91`・`text-decoration-canon/brief.md:75`） | ギャップなし（文書作業）。 |

### 3.2 複雑さの信号

- アルゴリズム変更は小さい（式の差替え＋em 導出の 1 段）が、**「正しい式」が実測待ち**で、候補ごとに実装形が変わる（§4）。
- 外部統合（SSP 実測）は道具づくり込みで 1 日級。
- テスト再導出は量が多い（30 ファイル・golden は byte 等価でなくインク計数と矩形なので画像資産の差替えは不要＝`draw_oracle_tests.rs:149-160`・`tests/fixtures/` は `emo2-choice` のみ）。

### 3.3 行送り・行ボックス丈・フォントサイズに数値依存する既存テスト（洗い出し・R7.1）

`grep "1\.25|1\.33|37\.24|line_pitch"` で当たった 30 ファイル（DPI 拡大率 `k=1.25` としての出現を含む・R7.1 の列挙より広い）:

- 純粋層（`FixedMetrics` の 1.25／1.33 依存）: `layout_wrap_tests.rs`（26 箇所）・`layout_segmented_tests.rs`（10）・`layout_visible_window_tests.rs`（ピッチ 13 前提・`:10-80`）・`layout_cursor_tests.rs`・`layout_cursor_center_origin_tests.rs`・`layout_cursor_overflow_tests.rs`（`13.0`・`23.0` 等）・`layout_cursor_vertical_tests.rs`・`layout_cursor_vertical_canon_tests.rs`・`layout_cursor_wiring_tests.rs`・`cursor_tag_tests.rs`／`cursor_tag_resolve_tests.rs`／`cursor_tag_test_support.rs:21`（`LINE_PITCH = 13`）・`state_cue_apply_tests.rs`・`state_reveal_tests.rs`・`choice_tests.rs`（17・帯の clamp）・`viewbox_axis_tests.rs`・`viewbox_dirty_tests.rs`・`viewbox_plan_commit_tests.rs`・`actor_tests.rs`・`actor_choice_contract_tests.rs`・`actor_scale_refresh_tests.rs`
- COM 層／読み戻し: `draw_format_metrics_tests.rs`（`line_pitch(10)=13`・`line_box_height(28)=37.24`・係数 2.0 の非既定検査 `:400-410`）・`viewbox_draw_frame_render_tests.rs`・`viewbox_draw_live_diff_tests.rs`（`:455,476` の容量式）・`viewbox_draw_choice_hover_tests.rs`・`viewbox_draw_png_dump_tests.rs`（14）
- `tests/`: `draw_readback_test.rs:72`（pitch 15）・`viewbox_scroll_test.rs:32,66-80`（`PITCH = 15`・`const _: () = assert!` の容量検査）・`viewbox_blit_spike.rs:66-74`・`pipeline_test.rs`（7）・`scale_invariance_test.rs`（k=1.25 が主・行数不変の検査 `:325-365,462`）・`emo2_fixture_e2e_test.rs`・`choice_fixture_test.rs`
- example: `examples/emo-text-layer/scenario.rs:9-30`（「validrect 320×122・font 28・pitch 35」「横書き容量 3 行・縦書き 9 列」「`EXPOSURE_BAND_DRAW_BOUND = 3`」）・`drive.rs:371-375`（`line_pitch` を実行時に読む＝自動追随）
- 他クレート: `crates/areka/src/emo2_boot/spine_conformance_script.rs:452`（コメントのみ）・`crates/areka` の 8 テストファイルは `TextLayerConfig::default()` を渡すだけ（field 名変更時はコンパイルのみ影響）

### 3.4 「1.25」を行送り係数として述べる記述の所在（R2.4 の初期棚卸）

製品コード: `state.rs:51,59,66`・`layout.rs:86-87,109`・`draw.rs:369-371`（doc）。テスト・example: §3.3 の該当 doc コメント。spec／steering／doc: 完了 spec `design.md:513,725`・`research.md:200`・`roadmap.md:73,91`・e2e `acceptance-record.md:681`・`crates/areka/src/emo2_boot/spine_conformance_script.rs:452`。除外すべき DPI の 1.25: `region.rs:710-731`・`tests/scale_invariance_test.rs`・`crates/areka/src/placement/*`（`266×1.25` 等）。R2.4 の機械検査は「`1.25` かつ `line_pitch|行送り|係数` を同一行に含む」等の絞り込みが要る（設計判断 #10）。

## 4. 候補の比較（`font.height` の意味と行送りの式）

### 4.1 候補一覧と予測値（Yu Gothic UI・`font.height,28`・比 1.3301・ascent 比 0.8113）

| 候補 | `font.height` の意味 | DWrite へ渡す em | 行ボックス丈 | ピッチ（行間 0） | ベースライン（行上端から） | 3 行目下端（相方側・上端 40） |
|---|---|---|---|---|---|---|
| **α セル丈** | ascent＋descent＝28 | `28 ÷ 1.3301 = 21.05` | 28.0 | 28（＋行間） | `0.8113 × 28 = 22.7` | 40+56+28 = **124 ≤ 133** |
| **β em（現行）** | em＝28 | 28 | 37.24 | 現行 35 | 30.2 | 138 > 133 |
| **γ em＋ピッチ縮小** | em＝28・ピッチだけ実測へ | 28 | 37.24 | 実測（≈31？） | 30.2 | 40+62+37.24 = 139（インク重なり） |
| **δ 固定比定数** | セル丈だが比は定数 | `28 ÷ 定数` | 定数次第 | 28 | 定数次第 | 124 |

- α は Requirement 3.1／3.2／5.1〜5.4 を式から満たす（3 行 84px ≤ 93・行ボックス＝ピッチで敷き詰め・インク重なりなし）。文字の見た目は現行より小さくなる（em 28→21）。**SSP のインク丈が現行より小さければ α、現行と同じなら β／γ 系**——実測の「参照グリフのインク丈」1 量だけで α と β/γ を弁別できる。
- γ は Requirement 3.2（ボックス ≤ ピッチ）に反するため、SSP が本当に em 28・ピッチ 31 なら「SSP はインクが重なり得る描き方をしている」ことになり、Requirement 1.5 の裁定へ回す事案になる。
- δ は Yu Gothic UI 以外（ＭＳ ゴシックは比 1.0）で誤った縮小を起こすので単独では採れない。α の縮退既定値（face metrics 不取得時・R3.9）としてのみ意味がある。
- 開発者の「≈31px/行」は 93÷3 の逆算に見える（brief）。α（28）と 31 は 3px 違い、これが「行間」の既定（Yu Gothic UI の `lineGap`／`tmExternalLeading`）か、SSP の既定の行間設定かは実測で決まる。Yu Gothic UI の lineGap が 0 なら α（行間 0）＝28 が本命。

### 4.2 α を採る場合の実装形（候補 A1／A2）

- **A1: em を実フォント比から導く**（本命）: `ResolvedFont.height`（正典 px）は据え置き、COM 層で `em = height ÷ ratio(font)` を求めて `try_create_format` へ渡す。`ratio` は現行 `measure_line_box_ratio`（`draw.rs:499-537`）の値だが、**現状は format 生成後に format から測る**ため、format 生成の前に family を解決して測る順序へ組み替えが要る（既定フォント再試行 `draw.rs:308-331` との整合）。`line_box_height` は構成上 `= height`、`line_pitch = height + 行間既定（0 なら height）`、`FixedMetrics` も `pitch = font_height`・`FIXED_LINE_BOX_RATIO = 1.0`（または撤去）。`\_l` の `em` 係数は `font_height`（R4.2 の 5em=140 のまま）。
  - 副作用: `highlight_band_extent`（`choice.rs:129`）は `box == font_height == pitch` で恒等になり、「descent はみ出し」の防御が構造上不要になる（帯＝行矩形）。関連テスト（`choice_tests.rs`・`viewbox_draw_choice_hover_tests.rs`・`draw_format_metrics_tests.rs:417-450` の「box_28 > 28」）は**検証対象が退役**する扱いか、`FixedMetrics` に「比 ≠ 1」を残して式の頑健性を検証し続けるか（設計判断 #5）。
  - 「2 つの em」問題: ukadoc の `1em` は文字高さ＝`font.height`、DWrite の em は 21.05。名前を分ける（例 `font_height`／`dwrite_em_size`）必要がある（設計判断 #3）。
- **A2: DWrite の行送りを `SetLineSpacing(UNIFORM, 28, baseline)` で強制し em は導出値**: A1 と同じ em 導出に加え DWrite 側の行高を明示する。行 TextLayout は 1 行ずつ箱 `font_height` で作っている（`draw.rs:614-620`）ので、A1 だけで既に 1 行＝28 の箱に収まる。A2 は lineGap≠0 のフォントで DWrite の既定行高が 28 を超える場合の保険。wintf ラッパへの追加が要る。

### 4.3 β／γ を採る場合の実装形（候補 B）

em 素通しを維持し `line_pitch` の式だけを実測値へ（`font.height + 行間`）。行矩形の厚み（`finish_line`）は `line_box_height`（37.24）へ変える必要があり、あふれ判定の入力が実フォント依存になる（純粋層へ COM 由来の比が流れ込む・現行は `FixedMetrics` の 1.33 で代用可）。Requirement 3.2 を満たすには「ピッチ ≥ ボックス」が要り、ピッチ 31 では不成立。**要件 3.2 の裁定（応急処置却下）と両立しない**ので、実測が β を示した場合は Requirement 1.5 の手順で開発者裁定へ戻す。

### 4.4 本体側 `emo2`（`balloons0s.txt`・(36,46)-(356,168)・高さ 122）への影響（R5.5）

| ピッチ | 行 i の下端 `46 + i×pitch + 28` | 収まる行数 |
|---|---|---|
| 35（現行） | 74／109／144／179 | **3**（4 行目 179 > 168） |
| 31 | 74／105／136／167 | **4**（167 ≤ 168・1px 差） |
| 28（α・行間 0） | 74／102／130／158 | **4** |

→ 本体側は「3 行で 4 行目にあふれる」前提のテスト（`examples/emo-text-layer/scenario.rs:15-21`・`viewbox_draw_live_diff_tests.rs:455`・`tests/emo2_fixture_e2e_test.rs` の「短メニュー＝領域内」）が **4 行容量**へ変わる。R5.5「SSP と同じ行数」は本体側でも SSP 実測で 4 行を確認する必要がある（Requirement 1.1 の撮影に本体側の 4 行台本を含めるとよい）。e2e の走行期待（起動時の挨拶の行数）にも波及し得る（引き渡し文書 R10.2 に載せる）。

### 4.5 拡大率 k（R3.10・R1.1 の k=2）

areka 側は `SetTransform(scale(k))` 一点（`draw.rs:840-850`・viewbox 同様）で、レイアウトは image px。k=2 では em 21.05→物理 42.1px 相当・ピッチ 56。SSP の 192DPI 表示が「font.height×2 を GDI に渡す」のか「96DPI で描いて拡大」なのかで k=2 の実測ベースライン・インク丈が変わる（R3.3 の ±2px はそのための緩み）。実測で食い違えば R1.5 の手順。

## 5. Requirement 6（折返し閾値が文字描画範囲の外）の候補

前提: 供給面＝validrect 寸（`actor.rs:663-665`）・canvas 空間＝validrect-local（`canvas.rs:309-321`）・ヒット行も canvas-local。閾値 254 は画像幅 288 の内側。

| 案 | 内容 | 帰結 | 要件との関係 |
|---|---|---|---|
| 1 現状維持 | 8px 欠けたまま | 利用者に「閉じる」の右が欠けて見える | 6.2 に反する（選ばなかった案として 6.6 で記録） |
| 2 閾値の丸め込み | `wrap_threshold = min(254, validrect.right=240)` | 「閉じる」(164..248) が 240 を超え 4 行目へ折返し→再びあふれ→5.1 不成立 | 6.4 が明示的に禁止 |
| 3 供給面を閾値まで広げる（本命候補） | 行内軸の供給寸を `max(validrect 遠辺, wrap_threshold)`（上限＝画像辺）へ。`TextRegion` に「供給範囲」を持たせ（例 `supply_right()`）、`actor.rs:663-665`・`canvas.rs:319-321`・ヒット行の物理化が同じ源を読む。行送り軸は不変 | 相方側は幅 216→230（offset 24 は不変）。本体側は 351 ≤ 356 で **式の結果が validrect と一致＝1 画素も変わらない**（6.3 を構造で満たす）。縦書きは `wordwrappoint.y` と `bottom` で同形 | 6.2／6.3／6.4 を満たす。SSP が同条件で欠かせる場合でも「あるべき姿」として選べる（記憶: 物差しは SSP 服従でなくあるべき姿） |
| 4 供給面を画像全域へ | 288×203・offset 0 | 全 fixture の面寸が変わり、readback 系テストの座標・ダーティ矩形・`scale_invariance_test.rs:231-320` の物理寸期待がすべて動く | 6.3「1 画素も変えない」を本体側で崩す（面寸が違えば読み戻しの byte 列が変わる）。不採用寄り |
| 5 折返し閾値の外側をクリップして描かない（明示） | 現状 1 と同じ見え方を規約化 | 欠けが「仕様」になる | 6.2 に反する |

案 3 の設計上の注意点: ⑴ `ContentCanvas.size` と `TextSurface` の物理寸の同時変更（両者は同じ `region` から導くので 1 か所の関数で済む）⑵ ヒット行の窓物理 px 化（`actor.rs:800-830`）も同じ offset を使うため影響なし ⑶ 警告 1 回（6.5）の置き場——`TextRegion::resolve`（純粋・毎フレーム呼ばれ得る）ではなく、`ResolvedBalloonText` を組む結線層か `present_actor` の初回装着ブロック（`actor.rs:655-737`・「初回のみ」の分岐が既にある）が自然 ⑷ `balloon-canon-residue` brief へ「項目 N: `wordwrappoint` が validrect の外に解決される定義」を登記（同 brief は Problem 節に番号付き列挙・現在 1〜10＋bvc 追加登記）。

## 6. 実装アプローチの選択肢（Options A/B/C）

### Option A: 既存コンポーネントの拡張のみ

- 触る: `state.rs`（`TextLayerConfig` の係数→行間既定へ）・`layout.rs`（`FixedMetrics`・`GlyphMetrics` doc）・`draw.rs`（em 導出・ratio 実測の順序・`DWriteMetrics` の式）・`choice.rs`（帯の式・doc）・`actor.rs`／`canvas.rs`／`region.rs`（案 3 の供給範囲）。
- 利点: 新ファイル無し・trait 注入点をそのまま使える。
- 欠点: **`draw.rs` が 980 行**で、em 導出（family 解決→face metrics→format の順序組替え）と lineGap 取得を足すと 1,000 を超える公算が大きい。`draw.rs` 分割は `text-decoration-canon` の着手前提として登記されており（roadmap W13）、本仕様が分割を先取りすると所有が二重になる。

### Option B: 新規コンポーネント（metrics の切り出し）

- `draw.rs` から `DWriteMetrics`／`measure_line_box_ratio`／（新設）em 導出を **新モジュール（例 `metrics.rs`・COM 層）** へ移し、`draw.rs` は format 生成と行 TextLayout に痩せる。`GlyphMetrics` trait は `layout.rs` に残す（純粋層）。
- 利点: 意味論の定義点が 1 ファイルに集まり（Requirement 3.5「同じ一つの源」の構造的表現）、`draw.rs` の残量問題を解く。decoration の「`draw.rs` 分割が前提」とも整合（分割の一部を先に済ませる形）。
- 欠点: 新ファイル＋`#[path]` テストの付け替え（`draw_format_metrics_tests.rs` の一部が移る）。decoration 側の分割計画と継ぎ目を合わせる必要がある（設計判断 #8）。

### Option C: ハイブリッド（推奨候補）

- 相 1（意味論）: SSP 実測 → 正典表・COMPAT §8・steering の改訂（コードは触らない・同じコミット列の先頭）。
- 相 2（実装）: Option B の metrics 切り出し＋式の差替え＋`FixedMetrics` の追随＋案 3 の供給範囲。`visible_window` は非改変。
- 相 3（テスト）: 既存 30 ファイルの再導出→新規テスト（3 台本×2 方式・SSP 定数の読み戻し・2 行インク非重なり・旧式対照）→ワークスペース全体テスト（終了コードで判定）。
- 分割はしない（brief の Boundary Candidates どおり・正典表と実装がずれた中間コミットを残さない R9.6）。

## 7. 工数・リスク

| 項目 | 見積 | 根拠 |
|---|---|---|
| 工数 | **M（3〜7 日）** | 式の変更は小さいが、SSP 実測（道具づくり込み 1 日）＋30 ファイルの再導出（1〜2 日）＋新規テスト（1 日）＋文書改訂（0.5 日）。roadmap の M と一致 |
| リスク | **Medium〜High** | High 要因: ⑴ 実測が候補のどれとも合わない（R1.5 の裁定へ）⑵ GDI／DWrite のラスタライズ差で ±1px 許容を外す ⑶ `draw.rs` 残量。Medium 要因: 再導出は機械的・注入点が trait に閉じている |

## 8. 設計フェーズへの推奨事項と Research Needed

推奨: Option C。式の候補は α（セル丈・em＝`font.height ÷ 実フォント比`・ピッチ＝`font.height + 行間既定`）を第一候補として実測で確定し、Requirement 6 は案 3。

Research Needed（設計フェーズで消化）:

1. **SSP 実測の道具**: DPI aware な画素読み取り（`PrintWindow`／`BitBlt` または D3D デスクトップ複製）と、行ごとの基準点（ベースライン・インク上下端）を数値化する走査スクリプト。既存 `measure-ssp-rects.ps1` の骨組み（Per-Monitor v2 宣言・読み取り専用）を流用可。
2. **実測台本**: emo2 実物 `menu.pasta` 3 台本に加え、参照グリフ（例「あ」「漢」「H」「g」）を 4 行並べる単純台本（相方側・本体側の両バルーン）。本体側は 4 行が収まるか（§4.4）も同じ撮影で読む。
3. **Yu Gothic UI の `lineGap`／`tmExternalLeading`** と、DWrite `DWRITE_FONT_METRICS` の ascent/descent が OS/2 win metrics か typo metrics か（`USE_TYPO_METRICS` の有無）。GDI の `GetTextMetrics` と DWrite の値が一致するかを 1 度確認しておくと、「セル丈」の定義が GDI と DWrite で同じ数になるかが決まる。
4. **SSP の既定「行間」設定の所在**（設定 UI に行間項目があるか・profile 初期化で既定に戻るか）。ukadoc MCP には無い。
5. **k=2 の SSP 描画方式**（フォント高さ×2 で GDI へ渡すか、拡大か）。
6. **face metrics 不取得時の縮退既定値**（R3.9）: 比 1.0（em＝`font.height`・ＭＳ ゴシックと同じ）が最も無害だが、Yu Gothic UI で縮退が起きると行が 37px になり再びあふれる。警告の文言と値を決める。
7. **decoration の `\f[height,N]`／`N%`／`+N`**（`text-decoration-canon` brief `:20`）は本仕様の `font.height` 意味論を継承する（「15pixel で表示」の 15 がセル丈になる）——設計で明記して引き渡す。

## 9. 設計判断項目（要件ディスカッションへ回す番号付き一覧）

1. **`font.height` の意味と行間既定を「実測で確定する」手順の合意**: 撮影条件（SSP の版・profile 初期化・モニタ DPI 2 水準・台本）と、候補 α／β の弁別に使う量（インク丈が決め手）。実測が候補のどれとも合わない場合の R1.5 の進め方。
2. **行間既定が 0 でないとき（例 3px）のピッチの式**: `font.height + 行間` を整数のまま使うか、現行の `ceil` を残すか（`\n[half]` 等の比率で端数が出る）。`TextLayerConfig` は「係数」から「行間 px」へ意味を変えるか、撤去するか。
3. **「2 つの em」の命名**: ukadoc の `1em`（＝`font.height`・`\_l` の係数）と DirectWrite の em サイズ（導出値）を型・名前で分ける方針。
4. **em 導出の縮退既定値と警告**（R3.9）: face metrics が取れないときの比（1.0 案）と、`FixedMetrics` の仮想値をどう揃えるか。
5. **`highlight_band_extent` の帯が恒等になったあとの扱い**: descent はみ出しの防御式を残す（`FixedMetrics` に比 ≠ 1 を残して式を検証し続ける）か、退役として記録するか（R7.2 の「検証対象が仕様判断で退役した根拠を個別に記録」に該当）。
6. **R8.7「旧式へ戻すと赤になる」対照の作り方**: 旧係数を注入できる口（`TextLayerConfig` の field を残す／テスト専用の `GlyphMetrics` 実装）のどちらにするか。前者は製品コードに旧式を残すことになる。
7. **Requirement 6 の案 3（供給面を閾値まで広げる）の採否**と、SSP が同条件で右端を欠かせた場合でも案 3 を採るか（互換より「あるべき姿」を優先する裁定の明示）。縦書きへの適用（`wordwrappoint.y` と `bottom`）も同じ規則にする。
8. **`draw.rs` の残量（980/1,000）への対処**: metrics を新モジュールへ切り出す（Option B・decoration の分割前提と継ぎ目を合わせる）か、`draw.rs` 内に収めるか。切り出す場合の所有（本仕様が分割の一部を先取りする旨を decoration brief へ登記）。
9. **完了 spec `emo-text-layer` の design／research を直接書き換える**（R2.1／2.2）か、COMPAT §8 の先例（アーカイブ非改変・上書きの事実を本表と現行 spec に記録）に揃えるか。
10. **R2.4 の機械検査の式**: DPI 拡大率としての `1.25` を除外する grep 条件（同一行に `line_pitch|行送り|係数` を含む等）と、残してよい記述（履歴・「本仕様で改訂」注記）の扱い。
11. **本体側 `emo2` の行容量が 3→4 行へ変わる影響の引き受け**（R5.5・§4.4）: SSP 実測で 4 行を確認し、`scenario.rs`／`live_diff` の容量前提を導き直す。e2e の走行期待への波及を R10.2 の引き渡し文書に載せる。
12. **警告 1 回（R6.5）の置き場**: 純粋層 `TextRegion::resolve` ではなく結線層の初回装着ブロック（`actor.rs:655-737`）で出す案の確認と、`balloon-canon-residue` brief への登記番号。
13. **±1px 許容（R3.3／3.4）の測り方**: GDI と D2D のラスタライズ差を吸収する測定定義（インク丈＝不透明画素の上端〜下端・アンチエイリアス閾値）を実測前に決める。
