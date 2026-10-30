# Research & Gap Analysis: emo-text-dwrite-fidelity

> 2026-10-29 `kiro-validate-gap`（要件の段のギャップ分析）。読んだのはワークツリーの先頭 `b181b054`（main `932d2c30` の上に spec の初期化 1 コミット）。コードは静的に読んだだけで、GPU の読み戻しや実機での測りはしていない。「見込み」と書いたものは、設計の段で測って確かめる。

> **2026-10-29 設計の段（`kiro-spec-design`）の追記**: 件 1 の原因・件 2 の引き当て方・件 3 の直し方を、使い捨ての測り（DirectWrite だけ・GPU と画面は使わない・測った後に消した）で決めた。結果は §10。§1〜§9 は要件の段の記録のまま残す（§4.3 の H1〜H5 は §10.1 で決着）。

## 1. 分析の要約

- **件 2（GDI の名前）は、今ある `FontCatalog` を広げれば足りる**。足りないのは 2 点。⑴ 家族名で見つからない候補を、GDI の名前として引き直す口。⑵ 引き当てた**太さ・斜体を書式まで運ぶ道**。今は `family_for` が `Option<String>`（家族名だけ）を返し、書式を作る 3 か所（`draw.rs::try_create_format`・`draw_metrics.rs::probe_format_for`・`viewbox_draw_decoration.rs::apply_font_ranges`）がどれも太さを NORMAL か BOLD の 2 値で決め打ちしている。
- **件 1（縦書きの開き括弧）は原因がまだ分からない**。areka の縦書きの設定は、方向レシピ 4 点と `ja-JP` だけ（`IDWriteTextFormat1`・`IDWriteTypography`・`SetVerticalGlyphOrientation` は使っていない）。最初の仕事は、どこで食い違うかを測り分けること。DirectWrite が実際に出した字形番号・横倒しの印・字形のずらしを、自前の `IDWriteTextRenderer` で抜き出して HarfBuzz の縦組みと並べる（§4.3）。
- **件 3（列の左右のずれ約 6px）は、既に測ってある em の箱のずれと数が合う**。`vertical-choice-hover-band` が測った游ゴシックの比 −0.3013 に 20px を掛けると −6.0px で、起票の実測 −6.5px とほぼ同じ。同じ現象の見込みが高い。直すなら行の矩形の側（`layout.rs`、ウェーブの約束で触れない）か、DirectWrite の行の置き方（行間の設定、`draw.rs`／`draw_line_store.rs`）になる。
- **太字を描く経路の穴（見込み）**: バルーン定義の `font.bold,1` は既定の見た目の `bold` を真にする。ところが束縛書式は NORMAL のままで、既定の区間は `apply_font_ranges` が飛ばすので、**太く描く経路が見当たらない**。要件 4.2 が前提にしている「太字の指定」のうち、バルーン定義の側がそもそも効いていない見込みがある。
- **`number`／`sstpmessage`／`communicatebox` の `font.name` は、どこからも読まれていない**（要件 4.7 の条件は今は成り立たない）。

## 2. 今あるもの（Current State）

### 2.1 フォント名の解決（`draw_catalog.rs`・208 行）

- `FontCatalog::family_for(&[String]) -> Option<String>`: 候補を書いた順に見る。フォントファイル名なら飛ばして 1 度だけ記録する。`has_family`（`IDWriteFontCollection::FindFamilyName`）で最初に見つかったものを返す。全滅なら 1 度だけ記録する。結果は候補列ごとに `memo` に覚える（要件 3.11 は今のまま満たしている）。
- `pick(&ResolvedFont) -> ResolvedFont`: 解決した家族名を `name` に入れた複製を返す。全滅なら `DEFAULT_FONT_NAME`。
- 問い合わせ先は `FamilySource::System(IDWriteFontCollection)` と、`#[cfg(test)]` の `Fake(BTreeSet<String>)`（小文字にした名前の集合）。偽の集合は**家族名しか表せない**ので、GDI の名前→家族・太さ・斜体の対応を表せるよう広げる必要がある（要件 6.2）。
- `FindFamilyName` は、書体が持つどの言語の家族名にも大小を問わず当たる。そのため家族名の側では要件 3.5（日本語名・英語名・大小無視）を今の時点で満たしている。

### 2.2 書式の生成（太さが決め打ちの所）

| 場所 | 今の太さ・斜体 | 用途 |
|---|---|---|
| `draw.rs:428` `try_create_format` | `WEIGHT_NORMAL`／`STYLE_NORMAL` 決め打ち | 描画と計測で共用する束縛書式（`create_text_format` から呼ぶ） |
| `draw_metrics.rs:226` `probe_format_for` | `key.bold ? BOLD : NORMAL`・`key.italic ? ITALIC : NORMAL` | 既定でない見た目の、計測鍵ごとの試験用書式 |
| `viewbox_draw_decoration.rs:166` `apply_font_ranges` | `look.bold != default.bold` のときだけ `SetFontWeight(BOLD or NORMAL)`。斜体も同じ形 | `\f[...]` の区間の範囲指定（行の TextLayout を作るとき 1 度だけ） |
| `viewbox_draw_render.rs:521/526` | `create_text_format(&self.dwrite, &self.fonts.pick(font), mode)` | 本番の束縛書式。触らない約束のファイルだが、呼ぶだけの行 |

`draw_metrics.rs::measure_line_box_ratio` は書式の `GetFontWeight/Stretch/Style` から字面を辿る。そのため太さが書式に入れば、行の箱の比も中太の字面から測られるようになる（自動で追いつく）。

### 2.3 問い (b)：`apply_font_ranges` は太さを Regular に戻すか

**戻す（条件つき）**。

- `look.bold != default.bold` のときだけ、`DWRITE_FONT_WEIGHT_BOLD` か `DWRITE_FONT_WEIGHT_NORMAL` を範囲に当てる。既定の太字が真の区間で `\f[bold,0]` を使うと、**名前の太さ（500）ではなく 400** になる（要件 4.3 に反する）。
- `look.name != default.name` で家族名を `SetFontFamilyName` で差し替えるとき、太さには触らない。そのため区間の太さは束縛書式の太さを受け継ぐ。区間の名前が GDI の名前（例 `\f[name,Yu Gothic Medium]`）でも、既定の太さのまま描かれる（要件 4.6 に反する）。逆に、既定の名前が中太で、区間の名前が Regular の家族のときは、中太を受け継いでしまう。
- 計測の側（`probe_format_for`）も 2 値の決め打ちなので、描画を直すなら同じ規則で直さないと要件 4.1 がずれる。

### 2.4 問い (a)：`number`／`sstpmessage`／`communicatebox` の `font.name` の通り道

- **どこからも読まれていない**。`areka-parsers` の `balloon::map_merged` はこれらのキーを引かない。`parse_tests.rs:159/662` と `validation_tests.rs:190` は、これらのキーが基底の `font.*` に「漏れない」ことを確かめる紛らわし役（distractor）として出てくるだけ。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` でも 3 本とも `status = "absent"`（担当なし。number は A13、sstpmessage と communicatebox は D2）。
- `FontCatalog` を使うのは emo-text の本文（`actor_decoration.rs:59` で台帳を 1 つ作り、`DWriteMetrics::new_shared`／`ViewboxExecutor::new_shared` に渡す）とテストだけ。
- ⇒ 要件 4.7 は「Where」の条件が今は成り立たない。本書で手を入れる所は無い。後で表示を作る spec が同じ `FontCatalog` を通せば、自動で効く。

### 2.5 縦書きの設定（件 1 の場）

- `DirectionRecipe::for_mode`: 縦書きは `READING_TOP_TO_BOTTOM` と、`FLOW_RIGHT_TO_LEFT` か `FLOW_LEFT_TO_RIGHT`。寄せは `LEADING`／`NEAR`。`LOCALE_JA_JP`。どちらも wintf の `typewriter_layout.rs` から写したもの。
- 縦書きに関わる DirectWrite の設定は**ほかに 1 つも無い**。ワークスペース全体を grep しても、`IDWriteTextFormat1`・`SetVerticalGlyphOrientation`・`IDWriteTypography`・`SetTypography`・`IDWriteTextAnalyzer1`・`IDWriteFactory3` の使用は 0 件。
- 行は `draw_line_store.rs` で、1 行に 1 つの `CreateTextLayout` で作る（縦書きの箱は幅 `block_extent`＝既定の行なら `font_height`、高さ `PROBE_MAX_EXTENT`）。1 字の計測は `draw_metrics.rs::probe_advance`（同じ書式の 1 字の TextLayout の cluster width）。em の箱の比は `measure_vertical_em_box_left_ratio`（「あ」1 字の `GetMetrics().left + baseline`）で測る。
- 描画は `viewbox_draw_render.rs` の `draw_text_layout`（D2D 既定の描き方）。**自前の `IDWriteTextRenderer` は無い**。そのため「字ごとに位置をずらす」最後の手段（要件 2.3）を DrawGlyphRun の段で行うと、触らない約束の `viewbox_draw_render.rs` に及ぶ（§5 の制約 C2）。

### 2.6 今ある檻（直しで動く見込みのもの）

| 檻 | 動く理由 |
|---|---|
| `draw_format_metrics_tests.rs:741` `font_family_reaches_directwrite_only_as_author_name_or_default_retry` | `try_create_format(factory, &font.name, font.height)`・`try_create_format(factory, DEFAULT_FONT_NAME, font.height)` の**字面**と、`probe_format_for` の `let family = self.fonts.family_for(&key.name).unwrap_or_else(...)` の**字面**を照らしている。太さ・斜体を引数に足せば赤になる。922 行のファイルで、ウェーブの約束は「足さない」。字面を**直す**のは約束の外か（§6 議題 4） |
| 同 `:814` `writing_mode_selects_only_the_direction_recipe_not_the_font_name` | `create_text_format` の本文で `mode` を含む行が 2 行であることを数えている。件 1 の直しで縦書きだけの設定をここに足すと赤になる。`DirectionRecipe` の側に足せば当たらない |
| `tests/decoration_readback/vertical.rs` の `COLUMN_RL (274,301)`・`COLUMN_LR (2,30)`・下線と打ち消し線の帯の定数 | Yu Gothic UI・28px・本文「あア亜Agほ」。件 1 の直しが括弧のほかの字（かな・漢字・英字の横倒し）を動かすか、件 3 を直せば動く（要件 6.5） |
| `viewbox_draw_vertical_band_readback_tests.rs`・`viewbox_draw_vertical_band_scroll_tests.rs` | em の箱の比に合わせて帯を置くことを固定している（要件 5.4）。束縛書式・行の置き方を変えると、帯の比を測る「あ」の試験用の TextLayout と、描く行の TextLayout が同じ設定を受けているかが鍵になる |
| `draw_oracle_tests.rs` の `probe_advances_match_drawn_line_advances_per_cluster` ほか | 1 字の計測と行の送り幅が等しいことを見張っている（要件 2.4・4.1） |
| `draw_catalog_tests.rs`（318 行・17 本） | 偽の集合の形が変わればテストの土台（`catalog()`）も直す |

### 2.7 見込みの穴：バルーン定義の `font.bold` が描かれない

- `ResolvedFont::resolve_with_background` は `balloon_overrides::overrides` を通して、`font.bold,1` を `looks.default.bold = true` にする（完了 `areka-P0-balloon-font-descript-keys` の要件 5.5「その装いを既定の見た目として描く」）。
- しかし束縛書式（`try_create_format`）は NORMAL で、既定の区間（`StyleId::DEFAULT`）は `apply_font_ranges` が `continue` で飛ばす。既定だけの行は `line_layout` を素のまま通る。**既定の太字・斜体を書式か範囲に焼く経路が見当たらない**。
- 計測も同じで、`default_key` の計測は束縛書式（NORMAL）で行う。描画と計測は揃っている（どちらも細い）ので、折り返しはずれていない。
- 読み戻しの檻は `\f[bold,1]` しか見ていない（`decoration_readback_test.rs`）。バルーン定義の `font.bold` を描いた画素を確かめる檻は見当たらない。
- ⇒ 要件 4.2（太字の指定が名前の太さより勝つ）・4.4 を満たすには、束縛書式の太さを「`default.bold` なら BOLD、でなければ名前の太さ」で決める必要がある。ついでにこの穴も塞がる。本書の範囲として扱うか、別の起票にするかは議題（§6 議題 2）。**設計の段で実際に描いて確かめる**こと（静的に読んだだけの見立て）。

## 3. 要件と今あるものの対応（Requirement-to-Asset Map）

| 要件 | 今あるもの | 区分 |
|---|---|---|
| 1.1・1.2・1.5 開き括弧を下半分、閉じ括弧は今のまま | 縦書きの設定は方向レシピだけ | **Unknown**（原因が分からない・§4） |
| 1.3 列の左右の真ん中 | 起票の実測で ±1px 内 | 今のまま（壊さない） |
| 1.4・1.6 手本との見比べ・動いた字の記録 | 無し（測りの仮のテストは消してある） | **Missing**（測りの道具を作り直す） |
| 1.7 横書きは不変 | 横書きの読み戻しの檻 | Constraint |
| 2.1・2.3 原因の特定・最後の手段 | — | **Unknown**・最後の手段は C2 に当たる |
| 2.4 送り幅が変わるときの計測と行の一致 | `probe_advance` と行の TextLayout は同じ書式。`IDWriteTypography` は TextLayout にしか付かない | **Constraint**（行と試験用の TextLayout 3 種〔1 字・「あ」・行〕に同じ設定が要る） |
| 2.5 日本語のまま | `LOCALE_JA_JP` | 今のまま |
| 3.1・3.2・3.4・3.5 GDI の名前での引き直し | `has_family`（家族名だけ） | **Missing** |
| 3.3・3.6・3.7・3.11 家族名で当たる・全滅・ファイル名・記憶 | `family_for`・`memo`・`WarnKey` | 今あるもの（形は広げる） |
| 3.8・3.9 設定の項目も新しい書き方も作らない | — | Constraint |
| 3.10 ぱすたの descript で中太 | — | **Missing**（3.1＋4.x の結果） |
| 4.1 描画と計測で同じ太さ・斜体 | `try_create_format`・`probe_format_for`（2 値） | **Missing**（太さを運ぶ道） |
| 4.2〜4.5 太字・斜体の指定との重なり | 2 値の決め打ち・既定の太字を描く経路が無い見込み（§2.7） | **Missing** |
| 4.6 `\f[name]` の区間 | `apply_font_ranges`（家族名だけ差し替え・太さは差分のときだけ） | **Missing** |
| 4.7 number／sstpmessage／communicatebox | 読む経路が無い（§2.4） | 条件が成り立たない（今は何もしない） |
| 5.1〜5.3・5.5 件 3 の直し | em の箱の比（`vertical_em_box_left`）が既にある | **Missing**（2026-10-29 議題 1 で本書で直すと決まった・§4.4） |
| 5.4 帯を壊さない | 帯の読み戻しの檻 2 本 | Constraint |
| 6.1 縦書き 2 方向の括弧の読み戻し | `tests/decoration_readback/` の型（`run`・`ink_col_range` など）。行で数える道具は横書きの側にある | **Missing**（括弧の行の範囲を測る述語） |
| 6.2 偽の集合での引き直し | `FamilySource::Fake(BTreeSet)` | **Missing**（名前→家族・太さ・斜体の表に広げる） |
| 6.3 実際の書体の集まりで 500 | `assert_real_font_present` の型（`tests/decoration_readback/mod.rs:130`） | **Missing** |
| 6.6〜6.8・7.1 SSP との見比べ・実機・台帳 | `doc/pasta-real-run.md`・`assets.toml:1623` | 記録の仕事 |

## 4. 外の調べと原因の候補

### 4.1 GDI の名前の引き当て方（件 2）

| 案 | 中身 | 長所 | 短所・要確認 |
|---|---|---|---|
| **G1 `IDWriteGdiInterop::CreateFontFromLOGFONT`** | `IDWriteFactory::GetGdiInterop` → `LOGFONTW { lfFaceName=候補, lfWeight=FW_NORMAL, lfItalic=0, lfCharSet=DEFAULT_CHARSET }` → `IDWriteFont`。そこから `GetFontFamily().GetFamilyNames()`（家族名）・`GetWeight()`・`GetStyle()` を取る | GDI と同じ名前の規則（SSP と同じ流儀）。数十行。依存の追加なし（`Win32_Graphics_DirectWrite`・`Win32_Graphics_Gdi` はワークスペースで有効）。Windows 7 から使える | **Research Needed**: ⑴ 見つからない名前で `DWRITE_E_NOFONT` を返すか、既定の書体へ置き換えて成功するか（公式の文書に書いていない。Chromium の `platform_font_win.cc` の注記は失敗する側を示している）。⑵ 日本語の GDI 名「游ゴシック Medium」にも当たるか。⑶ 家族名「游ゴシック」にも当たるので、**家族名 → GDI の順を守る**。⑷ 戻ってくる家族名をどの言語で取るか（`FindFamilyName` に渡し直せる名前を選ぶ）。⑸ システムの書体の集まりしか引かない（下流の `balloon-font-file` がファイルの書体を足すとき、別の道が要る） |
| **G2 フォントセットの性質で絞る** | `IDWriteFactory3::GetSystemFontSet` → `GetFilteredFonts`／`GetMatchingFonts` に `DWRITE_FONT_PROPERTY_ID_WIN32_FAMILY_NAME`（と `FULL_NAME`）を渡し、当たった字面の参照から太さ・斜体・家族名を取る | 自前のフォントセット（下流のファイルの書体）でも同じ問い合わせができる。名前の種類（Win32 の家族名・フルネーム）を選べる | Windows 10 から（`IDWriteFactory3`）。コードが G1 より長い。`FULL_NAME` と `WIN32_FAMILY_NAME` のどちらで当てるかの決めが要る。大小と言語の扱いを確かめる |
| G3 名前の末尾を自前で読む | 「… Medium／Light／Bold」などの語を切って、家族名と太さに分ける | 依存なし・決定論 | 書体ごとの命名に依存して脆い（「ＭＳ Ｐゴシック」のような名前は表せない）。正典の「SSP と同じ」に合わない。偽の集合の形には使える |

偽の集合（要件 6.2）は、どの案でも「名前（小文字）→（家族名・太さ・斜体）の表」に広げれば表せる。`FamilySource` に `Gdi` の表を足すか、`Fake` を 2 つの表に分ける。

### 4.2 太さ・斜体を運ぶ形（件 2）

- `family_for` の戻り値を `Option<String>` から「家族名＋太さ＋斜体」の小さな値に広げる。呼び手は `pick`・`probe_format_for`・`apply_font_ranges`、それにテスト（`viewbox_draw_test_support.rs:184`・`tests/decoration_readback/mod.rs:131`・`draw_catalog_tests.rs`・`draw_metrics_styled_tests.rs`・`viewbox_draw_vertical_band_readback_tests.rs`・`viewbox_draw_choice_hover_tests.rs` など）。テストのファイルでの出現は約 40 か所。
  - 呼び手を増やさない案として、`family_for` は今の形で残し、太さを返す `face_for` を足す道もある。ただし同じ候補列を 2 度解決しない（要件 3.11）ため、記憶は 1 つにまとめる。
- 束縛書式: `ResolvedFont`（`#[non_exhaustive]`）に解決した太さ・斜体を足すか、`pick` の戻り値を対にする。`try_create_format` に太さ・斜体の引数を足す。どちらも §2.6 の字面の檻に当たる。
- 太さの決め方（4.2〜4.5）は 1 か所にまとめるのが安全。例「`bold` なら BOLD、でなければ名前の太さ」「`italic` なら ITALIC、でなければ名前の斜体」。これを束縛書式・試験用書式・範囲指定の 3 か所が同じ規則で使う。今は 3 か所に 2 値の決め打ちが写されていて、`probe_format_for` の doc にも「`try_create_format` 側の設定を変えても、ここは自動では追随しない」と書いてある。
- 範囲指定（`apply_font_ranges`）は「既定と違うときだけ焼く」形。そのため**名前が違う区間では、太さも名前の太さで必ず焼く**よう規則を改める必要がある（§2.3）。

### 4.3 問い (c)：開き括弧が em の箱の上端に来る原因の候補

起票の実測（brief）から分かっている手がかり:

1. 形は縦書き用（﹁）。ただし**横の「「」を時計回りに 90° 倒しても同じ形になる**ので、「`vert` で縦書き用の字形に替わった」とは言い切れない。
2. 閉じ括弧「」」は正しい位置（上端寄り）。かな・漢字は送り幅 20px で列の真ん中。
3. `IDWriteTypography` で `vchw`＋`halt` を当てると、墨が下端（上から 12〜17px）へ約 10px＝半 em 動き、送り幅は 20 のまま。`vhal` は括弧を動かさず、範囲の外の「っ」を動かす。`vpal` は何も変えない。**`halt` は横組みの機能**で、これが縦書きの字に効いたのなら、その字が**横組みで整形されて横倒しで描かれている**ことを示す（`vchw` だけでどう動くかは分けて測っていない）。
4. Chromium（並べは HarfBuzz、ラスタライズは DirectWrite）では下端に寄る。

原因の候補（設計の段で、この順に測り分けるのが安い）:

| # | 候補 | 根拠・見立て | 決め手になる測り |
|---|---|---|---|
| H1 | **横倒し（sideways）で描かれている**。U+300C は Unicode の Vertical_Orientation が `Tr`（縦書き用の字形があれば立てて使い、無ければ倒す）。TextLayout の分析が `vert` の字形を見つけられず（文字種 Common が前後の文字種に溶けたあと、`vert` の言語系に当たらないなど）、横の字形を倒している | 手がかり 3（`halt` が効く）と合う。ただし横の字形が em の右半分にあるなら、時計回りに倒すと下半分になるはずで、上端に来ることの説明はまだつかない（倒すときの基準＝横の基線を列の中央の基線に合わせる置き方が絡む見込み） | 自前の `IDWriteTextRenderer` を `IDWriteTextLayout::Draw` に渡し、`DrawGlyphRun` の `DWRITE_GLYPH_RUN.isSideways`・`glyphIndices`・`glyphOffsets`・`glyphAdvances`・`bidiLevel` を「…「繋」で抜き出す。`IDWriteFontFace1::GetVerticalGlyphVariants` で引いた縦書き用の字形番号と比べる |
| H2 | **`vert` の字形には替わっているが、縦の原点の取り方が手本と違う** | DirectWrite は `DWRITE_GLYPH_METRICS.verticalOriginY`（VORG か、vmtx の topSideBearing＋字形の箱）で置く。手本の並べ（Blink の `OpenTypeVerticalData`／HarfBuzz）が別の原点を使っていれば食い違う。游ゴシックの vmtx の、その字形の topSideBearing の値次第 | 同じ字形番号で `IDWriteFontFace1::GetDesignGlyphMetrics(isSideways=FALSE)` の `topSideBearing`・`verticalOriginY` を読み、`hb-shape --direction=ttb --font-funcs=ot`（と `--features=vert`）の y_offset と比べる。書体の `vmtx`／`VORG` を表として読む |
| H3 | **言語系と機能の当たり方**（`vert` と `vrt2`・`jp04`／`jp90` など、ja-JP のときの既定の機能の組） | `vhal` を区間に当てて括弧が動かないのは、括弧に GPOS の縦の機能が当たっていない兆し。H1 とつながる | H1 の抜き出しで、`vert` だけ／`vrt2` だけを `IDWriteTypography` で明示して字形番号と置き場が変わるか |
| H4 | **`IDWriteTextFormat1::SetVerticalGlyphOrientation`** の既定（`DEFAULT`）と `STACKED` の違い | 文書上は欧文などの倒す・積むの違いで、括弧には効かない見込み。ただし 1 行で試せる | 書式を `IDWriteTextFormat1` に cast して `STACKED` にし、括弧の墨の位置を測る |
| H5 | 書体の代わりの書体（fallback）で別の書体が描いている | 游ゴシックには「「」があるので見込みは薄い | H1 の抜き出しの `fontFace` の家族名 |

**Research Needed**: どれかに決めるには、上の抜き出し（自前の renderer）が要る。この道具はテストの中の仮の測りとして作り、測ったら消すのが起票時と同じやり方。原因が H1 か H3 なら「設定（機能の明示・向きの設定）で直す」道があり、H2 なら「設定では直らない」（要件 2.3 の最後の手段）に寄る。

### 4.4 件 3（列の左右のずれ）

- 起票の実測は、墨の中心 414.5、行の矩形の中心 421、差 −6.5px。完了 `vertical-choice-hover-band` が測った游ゴシックの em の箱の左の比は −0.3013（右から左）で、20px なら −6.03px。**差が 0.5px 以内で一致する**。墨の左右の偏りを除けば同じ量と読める。
- 原因は、DirectWrite が縦書きの列を「書体の ascent＋descent の行の箱」の中で中央の基線に置くのに対し、areka の行の矩形（`layout.rs`）が em の箱を列の箱の真ん中にある前提で置くこと（`column_centred_em_box_left` の既定）と見込まれる。帯の側は既に em の箱の比で手当てしてある。
- 直し方の候補:
  - (i) 行の矩形の側（`layout.rs`）＝約束で触れない。
  - (ii) 描く原点（`viewbox_draw_render.rs`）＝約束で触れない。
  - (iii) DirectWrite の行の置き方を変える。例えば縦書きだけ `SetLineSpacing(UNIFORM, h, baseline)` で em の箱を列の真ん中へ寄せる。`DirectionRecipe::apply`（`draw.rs`）か `draw_line_store.rs` に置け、約束の内に収まる。ただし帯の比（「あ」の試験用の TextLayout）・はみ出しの実測・下線と打ち消し線の帯の定数（`vertical.rs`）が一緒に動く。横書きを動かさないことと、帯の檻が緑のまま残ることを測ってから決めることになる。
  - (iv) 直さずに閉じる（理由と値を残す）。
- 2026-10-29 要件の討議 議題 1「承認」で、本書で直すと決まった（要件 5）。(iii) から試し、(i)(ii) に及ぶときは約束を理由に避けず、触る前にほかのセッションへ知らせる。

## 5. 制約（Constraint）

- **C1 ウェーブ C12 の約束**: emo-text に新しいソースファイルを足さない。`layout.rs`・`viewbox_draw_render.rs` に触らない。922 行の `draw_format_metrics_tests.rs` に足さない。ほかに触るファイルとして、`viewbox_draw_decoration.rs`（要件の申し送り 3）と `tests/decoration_readback/vertical.rs`（`src` の外）が要る。同じウェーブの ④ `real-run-log-noise` は `region.rs`・`actor_attach.rs`・`actor_decoration.rs`・`choice.rs` に触る。本書が `actor_decoration.rs` に触らなければ重なりは 0。
- **C2 最後の手段（要件 2.3）の置き場**: D2D の `DrawTextLayout` の後で字ごとにずらすには、自前の `IDWriteTextRenderer` を本番の描画に差し込むことになり、`viewbox_draw_render.rs` に及ぶ。TextLayout の段でできる手（機能の明示・範囲ごとの設定）は `draw_line_store.rs` で済む。
- **C3 設定を置く所**: `IDWriteTypography` は TextLayout にだけ付き、書式（`IDWriteTextFormat`）には付かない。そのため機能で直すなら、行（`draw_line_store.rs`）・1 字の計測（`draw_metrics.rs::probe_advance`）・em の箱の比の「あ」（`measure_vertical_em_box_left_ratio`）の **3 つの TextLayout** に同じ設定を当てる（要件 2.4）。書式に付く設定（`SetVerticalGlyphOrientation`・`SetLineSpacing`）なら、`create_text_format` を通る 1 か所で全部に効く。ただし `probe_format_for` は自前で書式を作るので、`DirectionRecipe::apply` に置けば両方に効く。
- **C4 字面の檻**: §2.6 の 2 本は字面を照らしている。引数を足すなら照らす字面を改める（足すのではなく直す）。
- **C5 横書き不変（1.7・6.4）**: 縦書きだけに効く設定にする。`DirectionRecipe` は向きから一意に決まるので、縦書きだけの値を持たせるならここが自然。
- **C6 GPU の檻は実 GPU**。游ゴシックの有無を確かめてから回す（無い机では理由を言って飛ばす。今の `assert_real_font_present` は無ければ panic する型なので、どちらにするかは設計で決める）。

## 6. 実装の選択肢

### 案 A：今ある部品を広げる（推奨の見込み）

- 件 2: `draw_catalog.rs` に GDI の名前の引き直し（G1 か G2）と「家族名＋太さ＋斜体」の戻り値を足す。`draw.rs::try_create_format`・`draw_metrics.rs::probe_format_for`・`viewbox_draw_decoration.rs::apply_font_ranges` を、同じ太さ・斜体の決め方で揃える。テストは `draw_catalog_tests.rs`・`draw_metrics_styled_tests.rs`、それに装飾の兄弟のテストに足す。
- 件 1: §4.3 の測り分けのあと、設定の直しを `DirectionRecipe`（書式に付く設定）か `draw_line_store.rs`＋`draw_metrics.rs`（TextLayout に付く設定）に置く。檻は `tests/decoration_readback/vertical.rs`（274 行で余裕あり）に足す。
- 長所: 新しいファイル 0 で、約束に収まる。今ある記憶・記録の 1 度化・偽の集合の型をそのまま使える。
- 短所: `draw_catalog.rs` が 208 行から 300 行前後に増える見込み（上限の 1,000 行には遠い）。§2.6 の字面の檻を直す要がある。

### 案 B：GDI の名前の引き当てを別の部品にする

- 例 `draw_gdi_names.rs` を足して `FontCatalog` から呼ぶ。
- 長所: 責務が分かれる。下流の `balloon-font-file` の差し込み口が見やすい。
- 短所: **C1 の約束（新しいソースファイルを足さない）に反する**。足すなら同じウェーブの他と調整が要り、1 本だけの席を取ることになる。今の規模なら利点が薄い。

### 案 C：混成（件 1 の最後の手段だけ別の道）

- 件 2 と、件 1 の設定の直しは案 A のまま。設定で直らないと分かったとき（H2）だけ、字ごとのずらしを TextLayout の段（例 `draw_line_store.rs` で、その字の範囲に `SetTypography` で GPOS の機能を明示する、あるいは書体の vmtx の差を足す）で行い、描く段（`viewbox_draw_render.rs`）は使わない。
- 描く段のずらしが避けられないなら、C1 の約束を外す調整（ほかのセッションとの相談）が要る。

## 7. 規模と危うさ

- **規模: M（3〜7 日）**。件 2 は既にある形を広げるだけで S〜M。件 1 は原因の測り分け（自前の renderer・HarfBuzz との比較・書体の表を読む）に幅があり、M の上の端まで行きうる。brief の見立て 9〜13 タスクと合う。
- **危うさ: 中〜高**。件 2 は低い（DirectWrite の GDI 互換の口は文書のある API。確かめるのは失敗時の振る舞いと日本語名だけ）。件 1 は高い（原因が分からない。設定で直らなければ最後の手段の置き場が約束に当たる）。件 3 の (iii) は帯・下線・はみ出しを一緒に動かすので中。

## 8. 設計の段への申し送り

### 推奨の見立て（決めではない）

- 案 A。件 2 は G1（`CreateFontFromLOGFONT`）を第一候補とし、失敗時の振る舞いを最初に確かめる。下流の `balloon-font-file` のために G2 へ替えやすい形（引き直しの口を 1 つの関数に閉じる）にしておく。
- 太さ・斜体の決め方は 1 つの関数にまとめ、束縛書式・試験用書式・範囲指定の 3 か所から使う。
- 件 1 は §4.3 の H1 → H3 → H4 → H2 の順に測り分ける。

### Research Needed（設計の段で調べる）

1. `CreateFontFromLOGFONT` が見つからない名前で失敗するか、置き換えて成功するか。日本語の GDI 名に当たるか。戻る家族名の言語。
2. 自前の `IDWriteTextRenderer` で「…「繋」の括弧の `isSideways`・字形番号・ずらしを抜き出し、`GetVerticalGlyphVariants`／`GetDesignGlyphMetrics(vertical)`／`hb-shape --direction=ttb` と並べる（H1〜H3 の決め手）。
3. 游ゴシック（Windows 11 の版）の `vmtx`／`VORG`／GSUB `vert`・`vrt2`／GPOS `vhal`・`vchw` の、「「」の縦書き用の字形の値。
4. `vchw` だけ・`halt` だけを当てたときの、括弧の墨と送り幅（起票は 2 つを同時に当てた結果だけ）。
5. 件 3 の (iii)（縦書きだけの行間の設定）で、墨の中心が行の矩形の中心に来るか。帯・下線・打ち消し線の定数がどう動くか。
6. バルーン定義の `font.bold,1` が今、本当に細く描かれているか（§2.7 の見立ての確かめ）。

## 9. 要件の討議に出す議題（答えで作業が変わるもの）

1. **件 3 の描く側の直し**（要件の申し送り 1 を具体化したもの）: 件 3 は em の箱の比（−0.30×h）と同じ量と分かった。`layout.rs`／`viewbox_draw_render.rs` に触らずに、縦書きだけの行の置き方の設定（`DirectionRecipe`／`draw_line_store.rs`）で直せる見込みがある。これを本書で試すか、測って残すだけにして閉じるか。
2. **バルーン定義の `font.bold`／`font.italic` が描かれていない見込み（§2.7）**: 要件 4.2〜4.5 を満たす直し（束縛書式の太さを「太字なら BOLD、でなければ名前の太さ」にする）で一緒に塞がる。これを本書の範囲として記録に残すか、別の起票（B のバグ）に分けるか。分けると、本書の要件 4.2 の「`font.bold`」の側が檻にできない。
3. **要件 4.7 の扱い**: number／sstpmessage／communicatebox の `font.name` はどこからも読まれていない（台帳も `absent`）。要件 4.7 は条件が成り立たないので、何もせず記録に残すだけでよいか。要件の文を「今は経路が無い」と書き添える形に改めるか。
4. **字面の檻の扱い（ウェーブの約束との関係）**: `draw_format_metrics_tests.rs`（922 行）の 2 本は、`try_create_format` の呼び方と `probe_format_for` の家族名の導き方の字面を照らしている。太さ・斜体を足すと赤になる。約束は「足さない」なので、照らす字面を**直す**だけなら約束の内と読んでよいか。
5. **GDI の名前の引き当ての方式**: G1（`CreateFontFromLOGFONT`・Windows 7 から・短い）と G2（フォントセットの性質・Windows 10 から・自前の集まりにも効く）のどちらを第一にするか。下流の `balloon-font-file` との継ぎ目を重く見るなら G2。
6. **件 1 の最後の手段の置き場**: 設定で直らなかったとき（H2）、字ごとのずらしを TextLayout の段（`draw_line_store.rs`）でやれない場合がある。そのとき約束（`viewbox_draw_render.rs` に触らない）を外す調整をするか、本書では直さず理由を残して別の起票にするか。
7. **游ゴシックが無い机での檻**（要件 6.1・6.3）: 今の `assert_real_font_present` のように panic で止めるか、理由を記録して飛ばすか。

### 9.1 要件の討議での振り分け（2026-10-29）

- 議題 1（件 3）・議題 2（バルーン定義の `font.bold`）＝開発者が「承認」（2026-10-29）＝どちらも本書で直す（要件 5・要件 4.8／4.9）。
- 議題 3（要件 4.7）＝自明の直し。Boundary Context に「今は経路が無い」と書き足した。
- 次は設計の段（`/kiro-design`）で決める:
  - 議題 4（字面の檻）: 照らす字面を**直す**だけなら、922 行のファイルに「足さない」約束の内と読む（約束の狙いは行数の上限）。行が増えるときは、照らす役ごと兄弟のファイルへ移す。
  - 議題 5（G1／G2）: 設計で決める。G1 が見つからない名前で失敗するかを最初に測る。
  - 議題 6（最後の手段の置き場）: 約束を理由に回避の形へ逃げず、本質の置き場で直す（開発者の方針・2026-10-05）。C12 のほかの 7 本は `viewbox_draw_render.rs`・`layout.rs` を触らないので、触るなら兄弟のセッションへ知らせるだけで済む。
  - 議題 7（游ゴシックが無い机）: 今の `assert_real_font_present` の型に倣う。
  - 開き括弧類の檻の字の選び（要件 1.1）: 「「」を主に、「『」「（」を添える見込み。
  - 「中太の名前＋太字の指定」の SSP の振る舞い（要件 4.2）: GDI の流儀（太字の指定が勝つ）で書いてある。SSP では中太を太く合成する見込みで、areka は本物の Bold（700）になりうる。ぱすたは太字を使わない見込みなので、要件 6.6 の見比べのついでに見るだけにする。

## 10. 設計の段の調べ（2026-10-29・`kiro-spec-design`）

### 10.0 やり方と使った手引き

- **発見の型**: 軽い発見（今ある仕組みの拡張）。ただし件 1 の原因が未特定だったので、DirectWrite だけを呼ぶ使い捨ての統合試験（`crates/areka-emo-text/tests/` に仮に置いた 1 ファイル・自前の `IDWriteTextRenderer1` で `DrawGlyphRun` を抜き出す・`IDWriteGdiInterop::CreateFontFromLOGFONT` を呼ぶ）を回して測った。GPU・画面・SSP・areka の MCP には触れていない。測りのファイルは消した（`git status` は spec の文書だけ）。
- **使った手引き**: `kiro-spec-design` の `rules/`（design-principles・design-discovery-light・design-synthesis・design-review-gate）と、テンプレート `.kiro/settings/templates/specs/{design,research}.md`。steering は `product.md`・`tech.md`・`structure.md` と `roadmap.md` の C12 の行（ウェーブの約束）。ほかの技能（skill）は、この仕事の領域に当たる物が無いので使っていない。
- 測った机: 開発者の机（Windows 11・游ゴシック／Yu Gothic UI／メイリオ／BIZ UDPゴシック／ＭＳ ゴシック／Segoe UI が入っている）。

### 10.1 件 1: 開き括弧が上端に来る原因（要件 2.1）

**原因**: DirectWrite の TextLayout は、`IDWriteTypography` を付けないとき「既定の機能の組」で字形を並べる。游ゴシックの縦書きでは、この既定の組が **`vpal`（縦書きのプロポーショナルの寸法）と全く同じ字形の位置の調整**を当てる。ただし送り幅は変えない（20px のまま）。そのため開き括弧は位置だけが送りの逆向きへ半 em（20px で −10px）動き、続く字との間が空く。

測った値（游ゴシック Regular・20px・縦書き右から左・本文「…「繋がってる」」）:

| 場合 | 「「」の字形 | 「「」の位置のずらし（送りの向き） | かなのずらし（が・っ・て・る） | 送り幅 |
|---|---|---|---|---|
| 既定（typography 無し） | 7913（`vert` の縦書き用） | **−10.00px** | −0.34・−2.82・−0.80・−0.12 | 全部 20.00 |
| `vpal`＝1 だけを付けた typography | 7913 | **−10.00px** | −0.34・−2.82・−0.80・−0.12（既定と 6 値とも一致） | 20.00 |
| 空の typography（機能 0 個） | 7913 | 0 | 0・0・0・0 | 20.00 |
| `vkrn`・`kern`・`palt`・`vchw`・`liga`・`calt`・`ccmp`・`locl`・`chws`・`vert` のどれか 1 つ（＝1 でも 0 でも） | 7913 | 0 | 0（`palt`＝1 は列の幅の向きにだけ ±0.2px） | 20.00 |
| `vhal`＝1 だけ | 7913 | −10.00 | 0 | 20.00 |

- 縦書き用の字形 7913 の墨は、字の枠の上から 13.0〜18.9px（`GetDesignGlyphMetrics(isSideways=TRUE)` の上の余白 219・下の余白 221・upem 2048 から）＝**下端に寄った正しい置き場**。−10px ずらすと 3.0〜8.9px になり、起票の実測「上から 2〜8px」と合う。
- 横倒し（H1）ではない: 「「」の並び（run）は `isSideways=TRUE`（縦書きで立てて描く側）で、字形は `vert` の縦書き用（7913）。
- 縦の原点（H2）ではない: 字形の縦の寸法どおりなら下端に来る。ずれは字形の寸法ではなく、並べたときのずらし（`glyphOffsets.advanceOffset`）だけ。
- 機能の当たり方（H3）が原因の側。ただし `vert`／`vrt2` ではなく、既定の組の中の `vpal` 相当の位置の調整。
- `SetVerticalGlyphOrientation(STACKED)`（H4）は何も変えない。代わりの書体（H5）でもない（並びの書体は游ゴシック）。
- 書式の言語（ja-JP・en-US・zh-CN・空）を替えても既定の −10px は変わらない＝言語の設定の抜けではない（要件 2.5 の ja-JP はそのまま）。
- 太さ 500（中太）でも同じ（−10px）。
- ほかの書体（Yu Gothic UI・メイリオ・BIZ UDPゴシック・ＭＳ ゴシック）は既定でもずらし 0＝今も正しい。游ゴシックの GPOS の `vpal` のデータに当たる書体だけで起きる。
- 手本（Chromium＝HarfBuzz で並べる）は縦書きで既定に `vert` だけを当て、`vpal` を当てない（HarfBuzz の `hb-ot-shape.cc` の縦書きの既定の組）。ゆえに手本ではずらし 0＝下端。areka との違いは、この既定の組の差だけ。
- 起票で見えた「`vchw`＋`halt` で下端へ動く・`vhal` が括弧を動かさない」は、どちらも「typography を付けた時点で既定の組が外れる」ことで説明がつく（`vchw`／`halt` 自体は縦書きの括弧に効いていない）。

**直し（要件 2.2）**: 縦書きの TextLayout に、`vpal` を含まない機能の組を明示した typography を付ける。組は次の 10 個: `ccmp`・`locl`・`mark`・`mkmk`・`rlig`・`calt`・`clig`・`liga`・`kern`・`rclt`。

| 確かめ | 既定 | 明示の 10 個 |
|---|---|---|
| 游ゴシックの「「」のずらし | −10.00 | 0 |
| 游ゴシックの開き括弧類 9 種（「『（〔［【〈《｛） | 全部 −10.00 | 全部 0 |
| 游ゴシックの閉じ括弧「」」 | 0 | 0（墨は上から 1.1〜7.0px のまま） |
| 中太（500）の開き括弧類 9 種 | — | 全部 0（墨は上から 11.3〜13.5px より下） |
| 横倒しの英字「AVAfiWAVE12」（游ゴシック） | AV の詰め・fi の合字 | 同じ（字形番号・送り幅が全部一致） |
| Segoe UI の横倒しの英字 | — | 既定と一致 |
| Yu Gothic UI・メイリオ・BIZ UDPゴシック・ＭＳ ゴシック の「…「繋がってる」AVAfi」 | — | 既定と全部一致（字形・送り幅・ずらし） |
| 送り幅 | 20.00 | 20.00（変わらない） |

- 空の typography（機能 0 個）でもずらしは消えるが、横倒しの英字の詰め（`kern`）と合字（`liga`）も消える（AV の送り 12.18→12.98・fi が 2 字に分かれる）。ゆえに組は明示する。
- `vert` は組に入れない。`vert`＝1 を明示すると、横倒しの並び（「…」）にも縦書き用の字形が当たり、倒した向きで描かれてしまう（字形 670→7899）。`vert` は DirectWrite が立てる並びにだけ自分で当てる（明示しなくても 7913 になる）。
- `vkrn` は組に入れない（手本 HarfBuzz の縦書きの既定に無い。入れても今回の書体では結果が同じ）。
- 直しで動く字（要件 1.6 の記録）: 游ゴシックの縦書きで、開き括弧類（−10→0）・濁点「゛」（−10→0）・長音「ー」（−0.40→0）・かな（−0.1〜−2.8→0）・「あ」（−0.14→0）。どれも手本（ずらし 0）の側へ動く。ほかの書体は動かない。実装の段で画素でも測り直して記録する。
- **要件 2.3（字を個別にずらす最後の手段）は使わない**。設定（機能の組の明示）で直ることを測りで確かめた。

### 10.2 件 2: GDI の名前の引き当て（G1 に決めた）

`IDWriteGdiInterop::CreateFontFromLOGFONT`（`LOGFONTW` の `lfFaceName`＝候補・`lfWeight`＝400・`lfCharSet`＝`DEFAULT_CHARSET`）で測った:

| 候補 | 家族名で当たるか | GDI の名前として | 太さ・幅・斜体 | 家族名（en-us／ja-jp） |
|---|---|---|---|---|
| 游ゴシック Medium | 当たらない | 当たる | 500・5・0 | Yu Gothic／游ゴシック |
| Yu Gothic Medium・yu gothic medium・YU GOTHIC MEDIUM | 当たらない | 当たる | 500・5・0 | 同上 |
| 游ゴシック Light・Yu Gothic Light | 当たらない | 当たる | 300 | 同上 |
| 游ゴシック・Yu Gothic | 当たる | 当たる | 400 | 同上 |
| Yu Gothic UI Semibold | 当たらない | 当たる | 600 | Yu Gothic UI |
| Arial Black | 当たらない | 当たる | 900 | Arial |
| Arial Narrow | 当たらない | 当たる | 400・**幅 3**（condensed） | Arial |
| Segoe UI Light Italic | 当たらない | 当たる | 300・5・**斜体 2** | Segoe UI |
| NoSuchFontXYZ・`pasta.ttf`・空 | 当たらない | **`DWRITE_E_NOFONT`（0x88985002）で失敗** | — | — |

- **見つからない名前は失敗する**（既定の書体へ黙って置き換えない）＝§8 の問い ⑴ は「失敗する」。日本語の GDI の名前にも当たる（⑵）。大小を問わない（要件 3.5）。家族名にも当たるので、家族名 → GDI の名前の順を守る（⑶）。戻る家族名はどの言語のものでも `CreateTextFormat`／`SetFontFamilyName` に渡し直せる（⑷・家族名の照合はどの言語の名前にも当たる）。
- 名前が指すのは太さ・斜体だけでなく**幅**（Arial Narrow）もある。名前どおりの書体で描くため、幅も運ぶ（幅を落とすと Arial Narrow が並幅で描かれる）。
- `lfFaceName` は 32 単位（終端込み）。31 単位を超える候補は GDI の名前になりえないので引かない。
- ⑸ ファイルの書体（下流の `balloon-font-file`）: Windows 10 からの `IDWriteGdiInterop1::CreateFontFromLOGFONT(logfont, fontCollection)` が書体の集まりを受け取る（`windows` 0.62 に在る）。下流はここへ自前の集まりを渡せばよい＝G2 に替える必要は無い。本書は呼ばない（下流の持ち物）。
- 太字の指定と名前の太さの重ね方（要件 4.2）: 「中太の名前＋太字」は 700。名前の太さが 700 より重い（Arial Black の 900）ときは名前の太さのまま（GDI も同じ名前の中の顔を選ぶ）＝`max(700, 名前の太さ)`。

### 10.3 件 3: 列の左右のずれ（要件 5.1）

`SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, h, h/2)`（行の箱の幅＝h・基線＝その真ん中）を縦書きの TextLayout に付けて測った（20px・本文「あ亜Agほ」と「あ」）:

| 書体 | 向き | 既定の em の箱の左の比 | 揃えた後 | 立てる並びの基線の x（行の箱の左から） |
|---|---|---|---|---|
| 游ゴシック | 右から左 | −0.3013（「あ」）／−0.3398（本文） | 0 | 3.98 → **10.00** |
| 游ゴシック | 左から右 | ＋0.3008 | 0 | 16.80 → 10.00 |
| Yu Gothic UI | 右から左／左から右 | −0.1650／＋0.1992 | 0／0 | 6.02／13.30 → 10.00 |
| メイリオ | 右から左／左から右 | −0.2500／＋0.2500 | 0／0 | 5.00／15.00 → 10.00 |
| ＭＳ ゴシック・BIZ UDPゴシック | 両方 | 0 | 0（変わらない） | 10.00 |

- 列の基線（立てる字の em の箱の真ん中）が、どの書体・どちらの向きでも行の箱の真ん中（x＝10）に来る＝em の箱の左右の中心が行の矩形の中心に重なる（要件 5.1）。起票の −6.5px（游ゴシック・20px）が消える見込み（画素の確かめは実装の檻）。
- 原因: DirectWrite の既定の行間は書体の `ascent＋descent＋lineGap`（20px で游ゴシック 32.8px・Yu Gothic UI 27.3px・メイリオ 30.0px・ＭＳ ゴシック 20.0px）で列の箱を組み、その箱を行の矩形の書き始めの辺に揃える。areka の行の矩形は em の幅（`layout.rs`）。行間を em に揃えると、DirectWrite の列が areka の行の矩形と同じ形になる。
- 横倒しの英字の基線は 2.40px（ＭＳ ゴシックは 2.81）へ動く＝立てる字と同じ量だけ真ん中の側へ寄る。
- 装飾で大きさの違う字が混じる行（行の箱の幅＝行内最大 em）は、同じ設定を行の箱の幅で付ける（`SetLineSpacing` は TextLayout にも在る）。
- `layout.rs`・`viewbox_draw_render.rs` には触れずに済む（設定は TextLayout を作る所に置く）。

### 10.4 バルーン定義の `font.bold`・`font.italic`（要件 4.8・4.9）

静的に読み直して確かめた: 本番で `looks.default.bold`／`italic` を読む所は `draw_metrics.rs::probe_format_for`（既定でない計測鍵だけ）と `viewbox_draw_decoration.rs::apply_font_ranges`（既定でない区間だけ）の 2 か所しかない。束縛書式（`try_create_format`）は太さ・斜体を NORMAL に決め打ち。ゆえに `font.bold,1` だけのバルーンの既定の字は細く描かれ、計測も細い（両方そろって細いので折り返しはずれない）。§2.7 の見立てどおり。直しは「束縛書式に既定の見た目の太字・斜体を焼く」。檻は実 GPU の読み戻しで太く・斜めに描かれることを固定する（要件 4 の注記の「設計の段で確かめる」は、この読みと実装の檻で満たす）。

### 10.5 合成（design-synthesis）の結果

- **一般化**: 「名前 → 顔（家族・太さ・幅・斜体）」の解決を 1 つにし、太字・斜体の重ね方を 1 つの規則（`FontFace::weight_with`／`style_with`）にした。書式（束縛・計測鍵ごと）と範囲指定の 3 か所が同じ規則を通る。縦書きの TextLayout の設定（機能の組・行間）も、TextLayout を作る 3 か所（行・1 字の計測・em の箱の測り）が通る 1 つの関数にまとめた。
- **作るか使うか**: GDI の名前は OS の `IDWriteGdiInterop`（SSP と同じ GDI の規則）を使う。名前の末尾を自前で読む G3 は採らない。機能の組は DirectWrite の `IDWriteTypography` を使う。新しい依存は 0。
- **削る**: 計測鍵ごとの試験用書式が `try_create_format` の設定を字面で写していた重複をやめ、同じ関数を呼ぶ形にする（「`try_create_format` を変えても試験用書式は追随しない」という既知の弱点が消える）。新しいソースファイルは 0。`layout.rs` の既定の実装（列の箱の真ん中）は純粋層の偽の計測器の模型として残す（本番は測った比を使い、測れないときの縮退だけを 0 にする）。
- **見送った物**: ファイルの書体の集まりを GDI の引き当てに渡す口（下流 `balloon-font-file` の持ち物）・`vhal`／`vchw` などの約物の詰め（下流 `text-typesetting`）。

### 10.6 見つけた既存の食い違い（2026-10-29 設計の討議 議題 1「承認」＝本書で直す・要件 4.10）

- `\f[name,…]` の候補が全部見つからないとき、**計測**（`probe_format_for`）は既定の書体（ＭＳ ゴシック）で測り、**描画**（`apply_font_ranges`）は束縛の書体のまま描く。完了 spec の 2 つの決め（計測＝R11.3・描画＝要件 9.8）の間の既存の食い違いで、送り幅がずれうる。本書は計測と描画の 2 つの関数を書き直すので、残すと食い違いを新しいコードに書き直すことになり、太さ・斜体まで広がる（束縛が中太なら描画 500・計測 400）。設計の確かめ（design-validation.md 指摘 2）を受けて、設計の討議で「全滅は描画も計測も束縛の顔」に揃えると決めた（計測の R11.3 を描画の 9.8 に揃えて上書き）。

## 11. 直した後の実測（2026-10-30・実装の段 タスク 5.2・要件 1.4・1.6・2.1・5.2）

### 11.0 測り方

- 本番の描き方（`present_frame` → 実 GPU の読み戻し・倍率 1.0）で、`crates/areka-emo-text/tests/decoration_readback/vertical.rs` の檻 G1〜G5 と、同じファイルの縦書きの下線・打ち消し線の帯の檻（タスク 4.3 で固定値を改めたもの）を、排他テストの机を取って回した値。直す前の値は起票の実測（`brief.md`）と §10.1・§4.4 の値。
- 書体は、括弧と列の檻（G1〜G4）が**ぱすたの縦書きのバルーンと同じ候補の列・20px**（右から左・左から右）、帯の檻が Yu Gothic UI・28px。字の枠は上端が y＝0、半 em が y＝10（20px のとき）。

### 11.1 件 1: 開き括弧の位置（要件 1.4・1.6・2.1）

| 測り | 直す前 | 直した後 | 出どころ（直した後） |
|---|---|---|---|
| 開き括弧「「」「『」「（」の墨の行（字の枠の上から） | 3.0〜8.9px（§10.1 の字形の寸法から）・起票の画素の実測は 2〜8px | 上端 11〜13・下端 18／19（3 字とも半 em の y＝10 より下＝**下半分**） | G1 `opening_brackets_sit_in_the_lower_half_and_closing_in_the_upper_half_of_the_glyph_frame`（右から左・左から右 × Regular・中太・太字の 6 通り全部） |
| 閉じ括弧「」」の墨の行 | 0〜6px（起票） | 0〜6（**動かない**） | G1 |
| 「「」の墨の下端 →「繋」の墨の上端の空き | 約 11px（起票） | **2px**（「「」の最後の墨の行 18 →「繋」の最初の墨の行 20）。設計 G3 の期待値（書体の寸法から約 2.1px）と ±1px で一致 | G3 `gap_between_opening_bracket_and_next_kanji_follows_the_font_metrics` |
| 「「」と「繋」の墨の左右の中心の差 | ±1px（起票） | **0.5px**（右から左 「「」(292,309)・繋 (291,309)／左から右 「「」(2,19)・繋 (1,19)） | G2 `opening_bracket_is_centred_in_the_column_like_the_next_kanji` |

- **動いた字と量（要件 1.6）**: 画素で動きを測ったのは開き括弧類で、墨が送りの向きへ**約 10px（半 em）下がった**（3.0〜8.9 → 11〜13／18〜19。上端の行が 11〜13 と字ごとに違うのは字形の墨の高さの違い）。DirectWrite の字のずらし（`glyphOffsets.advanceOffset`）の値では、§10.1 のとおり 游ゴシックの開き括弧類と濁点「゛」が −10.00→0、長音「ー」が −0.40→0、かなが −0.1〜−2.8→0（例: が −0.34・っ −2.82・て −0.80・る −0.12）、「あ」が −0.14→0。どれも手本（Chromium＝HarfBuzz・ずらし 0）の側へ動いた。閉じ括弧類・漢字の位置は動かない（閉じ括弧の墨の行 0〜6 は直す前と同じ）。かなの 1px 未満のずらしは、字ごとの画素の読み戻しでは分けて測っていない（値は §10.1 の字形の並びの抜き出しが根拠）。ほかの書体（Yu Gothic UI・メイリオ・BIZ UDPゴシック・ＭＳ ゴシック）は §10.1 のとおり字のずらしが元から 0 で、機能の組の明示では動かない。
- **原因と根拠（要件 2.1）**: 原因は §10.1 のとおり、縦書きの TextLayout が `IDWriteTypography` を付けないときの**既定の機能の組が、游ゴシックに `vpal` と全く同じ字の位置の調整を当てる**こと（送り幅は 20px のまま、位置だけが送りの逆向きへ半 em ずれる）。根拠は §10.1 の表（既定と `vpal`＝1 だけの typography で 6 値とも一致・空の typography で 0）。直し（`vpal` を含まない 10 個の機能の組を明示）の後、画素でも開き括弧類が半 em 下がり（上の表）、空きが約 11px → 2px になったことで、原因の特定が画素の上でも裏づけられた。
- 太さ（要件 1.5）: G1 は Regular・中太・太字の 3 通りで同じく下半分（§10.1 の中太の 9 種 全部 0 と合う）。

### 11.2 件 3: 列の左右のずれ（要件 5.2）

本文「あ亜繋国」（かなと漢字だけの列）の墨の左右の範囲と、最初の列の行の矩形の中心との差（G4 `kana_kanji_column_ink_is_centred_on_the_first_line_rect`）:

| 書体 | 向き | 直す前の差 | 直した後の墨の列（x） | 直した後の差 |
|---|---|---|---|---|
| ぱすたの候補の列（游ゴシック）20px | 右から左 | **−6.5px**（起票: 墨の中心 414.5・行の矩形の中心 421） | (291,309) | **＋0.5px** |
| 同上 | 左から右 | — | (1,19) | ＋0.5px |
| ＭＳ ゴシック 20px（較正・直す前から真ん中） | 右から左 | 0（§10.3 の比 0） | (291,308) | 0px |
| 同上 | 左から右 | 0 | (1,18) | 0px |

- **原因と根拠**: §10.3 のとおり、DirectWrite の既定の行間（書体の ascent＋descent＋lineGap。20px で游ゴシック 32.8px）で組んだ列の箱を行の矩形の書き始めの辺に揃えるため、em の箱が列の真ん中から外れていた（游ゴシックの em の箱の左の比 −0.3013＝20px で −6.03px が、起票の −6.5px と 0.5px 以内で一致）。直し（縦書きの TextLayout に `SetLineSpacing(UNIFORM, 行の箱の幅, その半分)`）の後、差は ±1px の内（＋0.5px）。＋0.5px は墨の左右の偏り（字の墨が em の箱の真ん中から半画素ずれる分）で、ＭＳ ゴシックの較正が 0px で合うことから、式の側のずれではない。
- 帯の檻（`underline_lands_on_the_same_side_of_the_column_in_both_vertical_modes`・`strike_lands_on_the_same_side_of_the_column_in_both_vertical_modes`）の読み戻し（Yu Gothic UI・28px・本文「あア亜Agほ」・面の幅 310）でも列が行の矩形の真ん中の側へ動いた:

| 測り | 直す前 | 直した後 |
|---|---|---|
| 右から左の列の墨（x） | (274,301)・中心 287.5 | (279,307)・中心 293（行の矩形の中心 296） |
| 左から右の列の墨（x） | (2,30) | (0,25)（左端は validrect で切れる。切らなければ −3〜25） |
| 下線の帯 右から左／左から右 | (275,276)／(3,4) | (280,281)／(0,1)（今ある「帯は内へ寄せる」規則で内側へ） |
| 打ち消し線の帯 右から左／左から右 | (289,291)／(18,19) | (295,296)／(13,14) |

  - 動いた量（右から左 約 ＋5.5px・左から右 約 −5px）は、§10.3 の Yu Gothic UI の立てる並びの基線の動き（20px で 6.02→10・13.30→10）を 28px に伸ばした量（＋5.57・−4.62）と画素の丸めの内で合う。Yu Gothic UI には `vpal` 相当の調整が無い（§10.1）ので、この動きは行間の直しだけによる。

### 11.3 ついでに測った値（要件 4.8・4.9 の檻 G5）

`font.bold,1`／`font.italic,1` のバルーンの既定の字の墨の画素の数（無し／太字／斜体）: 横書き 1161／1536／1192・縦書き右から左 1158／1583／1232・左から右 1129／1550／1203。太字は多く、斜体は違う（G5 `balloon_bold_and_italic_reach_the_default_text`）。

## 出典

- [Vertical Text（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/win32/directwrite/vertical-text)
- [DWRITE_VERTICAL_GLYPH_ORIENTATION（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/win32/api/Dwrite_1/ne-dwrite_1-dwrite_vertical_glyph_orientation)
- [IDWriteGdiInterop::CreateFontFromLOGFONT（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritegdiinterop-createfontfromlogfont)
- [Chromium `ui/gfx/platform_font_win.cc`（GDI の名前と DirectWrite の照合・見つからないときの振る舞いの注記）](https://chromium.googlesource.com/chromium/src/+/0b71f14/ui/gfx/platform_font_win.cc)
- [IDWriteGdiInterop1::CreateFontFromLOGFONT（Microsoft Learn・書体の集まりを渡せる版）](https://learn.microsoft.com/en-us/windows/win32/api/dwrite_3/nf-dwrite_3-idwritegdiinterop1-createfontfromlogfont)
- [IDWriteTypography（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nn-dwrite-idwritetypography)
- [IDWriteTextFormat::SetLineSpacing（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextformat-setlinespacing)
- [HarfBuzz `src/hb-ot-shape.cc`（縦書きの既定の機能は `vert` だけ）](https://github.com/harfbuzz/harfbuzz/blob/main/src/hb-ot-shape.cc)
