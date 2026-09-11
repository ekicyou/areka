# ギャップ分析: areka-P0-text-decoration-canon

> 作成: 2026-09-11（要件確定後・設計前）。対象は `requirements.md`（16 要件＋付録 A/B）と `brief.md`（末尾の 2026-09-11 棚卸⑬の追記が範囲の正本）。
> 引用は「何の定義行か」で指し、行番号は当日の実測値を括弧で添える（動く前提）。本書は分析と選択肢の提示であり、決定は設計フェーズと開発者の裁定に委ねる。要件の付録・裁定済みの点（縦書きの下線の側・`font.height`＝em・行送りの式など）は再審議しない。

---

## 0. 要約

- **土台が丸ごと無い、というより「置き場所はあるが中身が空」**である。解読（`decode_tag`）に `"f"` の腕が無く、台本の組み立て（`compile`）の catch-all が捨てる。文字レンダリング層は 3 層（追記・配置・行）とも文字以外の属性を持たず、フォントは 1 本の `IDWriteTextFormat`、太さ・斜体は固定、`TextEffects`／`FontDisableSeam` は空の型である。**一方で、置き換え先の型・関数・テスト基盤はすべて実在し、どこへ何を足すかは一意に近い**。
- **最重要の設計判断は「文字ごとの装飾を DirectWrite でどう描くか」**である。太さ・斜体・大きさ・フォント名・下線・打ち消し線は `IDWriteTextLayout` の範囲指定（`DWRITE_TEXT_RANGE`）で 1 行 1 レイアウトのまま表せる（既存の hover 文字色が同じ経路）。しかし **上下付き（基線のずらし）と白抜き（輪郭だけ描く）は範囲指定では表せない**。自前の描画器（`IDWriteTextRenderer` の COM 実装）を持つか、run ごとにレイアウトを分けるかの選択が、実装量と縦書きの制御性を決める。
- **変更の波及を抑える鍵は「型に足すか、横に持つか」**。`TextItem::Glyph { ch }` の構築箇所は 182 か所・25 ファイル（大半がテスト）なので、この variant にフィールドを足すと機械的な書き換えが大量に出る。配置済みの 1 文字 `PositionedGlyph`（7 か所）・行 `GlyphRunContent`（6 か所）は安い。装飾の運搬も、既存の汎用キャリア `CueCommand::Custom` に乗せれば dola と 20 余りの網羅 match（他 crate を含む）に触れずに済む。
- **既定の見た目と無効表示の見た目の 2 層は `ResolvedFont` の置き換えで立つ**が、無効表示の色を「バルーンの背景色の側へ寄せる」ための**背景色の源が文字レンダリング層に届いていない**（届くのは寸法と倍率だけ）。フォントファイルの探索先（バルーンのフォルダ・`ghost/master`）も同様に届いていない。どちらも上流（`crates/areka/src/emo2_boot`）に値はあり、渡す口を 1 つ足せばよい。
- **着手条件の分割**（`draw.rs` 988 行）は、テーマ 3 つ（フォント解決・計測・行レイアウトの記憶）を兄弟ファイルへ出せば本体が約 350 行に落ち、公開の入口は `draw.rs` からの再輸出で不変にできる。`layout.rs`（955）は本仕様の変更で 20〜40 行増える見込みなので、先に自己完結した塊を 1 つ兄弟ファイルへ出しておくのが安全である。
- 規模の見立て: **L**（要件どおり）。リスク: **中**（DirectWrite の自前描画器と縦書きでの装飾位置の実測が未知数・それ以外は既存の型と経路の延長）。

---

## 1. 現状の資産（2026-09-11 実測）

### 1.1 解読と台本の組み立て（`crates/areka-parsers/src/sakura/`・`crates/areka-sakura/src/`）

| 場所 | 現状 | 本仕様との関係 |
|---|---|---|
| `decode.rs` の `fn decode_tag`（:201）の `match word.as_str()` | `_w`・`n`・`p`・`s`・`b`・`_l`・`q`・`!` の腕だけ。末尾 `_ => decode_passthrough_tag(word, args)` が `Instruction::Raw(reconstruct_tag(..))` へ落とす（:321） | `"f"` の腕を足す場所。各腕に `// ukadoc:` の URL コメントを添える書式が既にある（R2.7 と同形） |
| `lexer.rs` の `fn scan_tag`（:132） | 語は `[`／`\`／`%` の手前まで読む。`\f[bold,1]` は `Tag { word: "f", args: ["bold","1"] }` に、引数なしの `\f` は bare（`decode_bare` → `Raw`）になる | 本仕様は触らない。並走 `sakura-tag-word-boundary` の対象。`\f[]` は `args` が `[""]`（空 1 個）で届く点に注意（R2.2 の「引数が空」） |
| `model.rs` の `pub enum Instruction`（:25） | `#[non_exhaustive]`・`Clone/Debug/PartialEq` のみ。`GenericCommand { name, raw_args }` と `Raw(String)` が寛容経路 | variant 追加は後方互換。`\f` 専用 variant（例 `Font { args: Vec<String> }`）を足す |
| `compile.rs` の `fn compile` の catch-all `other => tracing::debug!(.., "M-boot 外タグを無視")`（:202-204） | `Raw` と未知 variant を捨てる。`GenericCommand` は `CueCommand::command_carrier(name, raw_args)` で瞬時（duration 0）の cue になる（:187-195） | `\f` の腕を `GenericCommand` の腕と同じ形（`emit(scope, offset, 0.0, ..)`）で足す。並び順は `emit` が `offset` を転写するので保たれる（R2.3） |
| `compile.rs` の冒頭 `ClearAll` 前置（:225-233） | 内容 cue を持つ台本の先頭に `ClearAll`（`start_time=0.0`）を 1 件挿入する | **「台詞の開始」の観測点がここにある**（R3.8）。文字レンダリング層は `ClearAll` を「新しい台本の先頭」として受け取れる |
| 既存テスト `decode_tests.rs`（624 行）・`compile_arm_tests.rs`（917 行） | `\foo[a,b]` を `Raw` に固定する `unknown_tag_absorbed_as_raw`（:422）は word が `foo` なので `\f` の腕と衝突しない。`catch_all_ignored_set_is_raw_only`（:126）は「Raw だけが捨てられる」ことを固定する | `\f` を `Raw` に固定する既存テストは**無い**（R2.8 の「既存テスト変更なし」は成立する見込み）。`compile_arm_tests.rs` は 917 行なので、新しいテストは `compile_font_tests.rs` のような兄弟ファイルに置く |

### 1.2 命令の運搬（`crates/dola/src/cue/`）

| 場所 | 現状 | 本仕様との関係 |
|---|---|---|
| `command.rs` の `pub enum CueCommand`（:130） | `#[non_exhaustive]` **ではない**・`Serialize/Deserialize` 付き。`Custom { command, params: DynamicValue }` が `\!` の汎用キャリアで、正準形の構築は `command_carrier(name, tokens)`・抽出は `as_command_carrier()` の 2 点に集約 | 新 variant を足すと**網羅 match の全箇所が壊れる**（`CueCommand::Wait` を含む match は 22 ファイル。`state.rs`・`actor.rs`・`dola/cue/sink.rs`・`areka-ghost/src/sink.rs`・`areka-seriko/src/actor.rs` の本番 5 か所＋テスト 17 ファイル） |
| `sink.rs` の `fn cue_target_of`（:54） | `Custom` は `None`（型では担当を決めず、消費側が名前で自己選別する規約） | `Custom` に乗せるなら、文字レンダリング層が `command == "f"` で自己選別する |
| `crates/areka-emo-text/src/sink.rs` の `impl CueSink for EmoTextSink` | 全 cue を broadcast で受け、担当外も純粋状態機械へ届く（duration は honor する） | `Custom` でも `Balloon` 向けの新 variant でも、sink は変更不要 |

### 1.3 文字レンダリング層（`crates/areka-emo-text/src/`）

**純粋層（Windows 非依存・決定論テストの対象）**

| 場所 | 現状 | ギャップ |
|---|---|---|
| `state.rs` の `pub enum TextItem`（:104）の `Glyph { ch }` | `Copy`。構築箇所 **182 か所・25 ファイル**（本番は `state.rs` の `Text`／`Choice` 腕の 2 か所、他はテスト） | 文字ごとの装飾を**この variant に足す**と全構築箇所の書き換えが要る（§3 D2） |
| `state.rs` の `pub struct ActorTextState`（:319）＝`items`・`reveal`・`choices` | スコープ（`ActorKey`）ごとに独立。`TextLayerState::apply_cue`（:381）は `Text`／`NewLine`／`Clear`／`ClearAll`／`Choice`／`Cursor` を消費し、`Custom` は「消費しない cue」として `debug!` で無視（:481-487） | 「現在の装飾状態」（R3.1）の置き場所はここ。`Custom`（または新 variant）の腕を足す。**`Clear` の腕は `ActorTextState::default()` で構造体ごと置き換える**（:403-406）ため、装飾状態を同じ構造体に置くと `\c` で装飾が消えてしまう（R3.7 違反）——状態は別フィールドに置くか、`Clear` の腕で装飾だけ引き継ぐ |
| `state.rs` の `parse_cursor_coord`（:197）と `CursorCoord` 語彙 | 「不透明文字列 → 語彙 enum」の全域関数（パニックしない・`Result` も返さない）と、その兄弟テスト `state_cursor_coord_parse_tests.rs` | `\f` の値（6 値・`+N`／`N%`・色の各書式）の解析も同じ作法で組める先例 |
| `state.rs` の `TextLayerConfig::line_pitch(font_height)`（:78） | 行送りの唯一の式 | R7.8 のとおり、行の高さが変わっても**ここ 1 点を通す**。文字ごとの高さは `font_height` 引数の値として入る |
| `layout.rs` の `pub trait GlyphMetrics`（:96）の `advance(&self, ch, font_height)` | 文字と高さだけで送り幅を返す。実装は `FixedMetrics`（純粋）・`DWriteMetrics`（COM）・テスト用 `LegacyPitchMetrics`（`tests/kero_menu_capacity_test.rs`）の 3 つ | 装飾込みの計測（R11.1）にはフォント名・太さ・斜体・大きさを渡す口が要る。既存の署名を変えると 3 実装＋呼び手が動く |
| `layout.rs` の `pub struct PositionedGlyph { ch, inline_pos, advance }`（:171） | `Copy`。構築箇所 **7 か所・5 ファイル**（本番は `layout.rs` の配置と `canvas.rs::from_layout` の 2 か所） | 装飾（または装飾の番号）を足す場所として安い |
| `layout.rs` の `fn layout_inner`（:325）と `fn finish_line`（:842） | 1 回の配置で `font_height` は**1 つ**。行矩形の丈も `pitch` も同じ値から出る。`segment_advance_sum`（:751）も同じ | R7.9「行の高さ＝行内の最大 em」には、行ごとの最大値を追跡して `finish_line` と `block_pos += pitch` に渡す変更が要る（見積 +20〜40 行・**955 行の上限まで残り 45 行**） |
| `canvas.rs` の `pub struct TextEffects {}`（:143）・`RESERVED_EFFECT_*`（:45-51）・`Resident.effects`（:271）・`GlyphRunContent { glyphs, size }`（:150） | すべて空の予約。`Resident` は行ごとに `effects: TextEffects::default()` を持つ | `outline` を実体化し（R5.9）、`multicolor`／`rotation` は予約のまま残す（R16.4）。`Resident.effects` の扱いは設計で決める（行単位の装飾は本仕様に無い→撤去または未使用のまま） |
| `viewbox.rs` の `fn line_fingerprint`（:660） | 行の再描画要否を「文字列・ブロック軸位置・寸・hover 印」で判定 | 装飾だけ変わった行を古いまま再利用しないため（R11.4）、指紋に**装飾の要約**を足す必要がある（`CommittedLine` の 1 フィールド追加） |
| `choice.rs` の `decorate_canvas`（:386）・`ResolvedChoiceStyle`（:466） | hover 行の塗りと文字色を `HighlightPaint` に焼く。`cursor.font.color` は `ResolvedChoiceStyle::SquareFill.text` として既に解決済み | R8.7（`default.cursor`）の色源はここから取れる。R3.5「hover が優先」は `SetDrawingEffect` の順序（全範囲 reset → 装飾色 → hover 色）で自然に成立する |

**COM 層（DirectWrite／D2D）**

| 場所 | 現状 | ギャップ |
|---|---|---|
| `draw.rs` の `pub struct ResolvedFont { name, fallback_chain, height, color, effects, disable }`（:158）と `ResolvedFont::resolve`（:184） | バルーン定義の 5 キー＋ukadoc 既定から 1 束を作る。`fallback_chain` は保持のみで未消費 | 「既定の見た目」の構築点（R4.1）。`effects`／`disable` は置き換え対象（R4.4・R5.9）。`name` の候補列を「最初に見つかったもの」で解決する規則（R9.8）はここへ足す |
| `draw.rs` の `pub struct FontDisableSeam {}`（:150）・`RESERVED_KEY_DISABLE_FONT_PREFIX`（:136） | 空の予約 | 「無効表示の見た目」の実体へ置き換える（R4.4） |
| `draw.rs` の `fn try_create_format`（:340） | `DWRITE_FONT_WEIGHT_NORMAL`／`DWRITE_FONT_STYLE_NORMAL` を固定で渡す | 太さ・斜体を引数化する（フォント名・大きさ・太さ・斜体の 4 つ組で format を作る） |
| `draw.rs` の `pub struct DWriteMetrics { .., format, font_height, cache: RefCell<HashMap<char, f32>> }`（:371）と `advance`（:456） | 文字 1 つ→送り幅 1 つの記憶。**`font_height` が束縛値と違うと `warn!` を出して束縛 format の値を返す**（:457-463） | 装飾込みの鍵（R11.1）へ改める。そのままだと `\f[height]` の行で毎文字 `warn!` が出て、計測も合わない |
| `draw.rs` の `fn measure_line_box_ratio`（:507） | `GetSystemFontCollection` → `FindFamilyName` → `GetFirstMatchingFont` → `GetMetrics` の一連が**既にある** | R9.2／9.8「インストール済みか」の判定にそのまま再利用できる |
| `draw.rs` の `LineLayoutStore::line_layout(index, text, format, font_height, mode)`（:594）と `CachedLineLayout { text, layout, overhang }` | 行 index をキーに、文字列が同じなら再利用。箱の行送り軸寸は `font_height` | 再利用の判定に装飾を含める（R11.4）。行の丈は行内最大 em を渡す。生成後に範囲指定の属性を焼く工程が要る |
| `viewbox_draw.rs` の `ViewboxExecutor::render`（:209）の描画資源確定区間（:280-420） | **`DWRITE_TEXT_RANGE` で `SetDrawingEffect` を「全範囲 `None` にリセット → hover 範囲へ文字色ブラシ」の順に適用する既存経路**（:352-402）。`segment_text_range`（:807）がグリフ列から UTF-16 の文字範囲を導く | 文字ごとの装飾の範囲適用は、この「リセット正準列」と `segment_text_range` の考え方をそのまま拡張できる（run ごとに `startPosition`／`length` を積む） |
| `viewbox_draw.rs` の `type FormatKey = (String, u32, WritingMode)`（:70）・`ensure_format`（:530） | フォント名・高さ・方向が変わると format と行キャッシュを組み直し、全域ダーティへ縮退する | 装飾は run 単位なので、この「actor の既定 format」の鍵は既定の見た目の分だけで足りる（run の属性は行レイアウトの範囲指定に載せる） |
| `crates/wintf/src/ecs/widget/text/typewriter_draw.rs`（:245-264） | 透明ブラシを `SetDrawingEffect` で範囲適用して未表示部分を隠す（仕様外の既存資産） | 「同じレイアウトに効果を焼いて戻す」作法の 2 つ目の先例 |
| `#[implement(...)]` の COM 実装 | `crates/areka/src/shiori_host.rs`（`#[implement(IShioriHost)]`・windows-core 0.62 の `*_Impl` 面へ pub vtable メソッドを書く注記 :210）ほか shiori 系に先例あり。**DirectWrite の `IDWriteTextRenderer` を実装した例は repo に無い** | 白抜き・上下付きを自前描画器で描くなら、この作法を DirectWrite へ持ち込む（§3 D3） |
| `windows` crate（workspace 0.62・`Win32_Graphics_DirectWrite` 有効） | `SetFontWeight`／`SetFontStyle`／`SetUnderline`／`SetStrikethrough`／`SetFontSize`／`SetFontFamilyName`／`SetFontCollection`（いずれも `DWRITE_TEXT_RANGE` 付き）・`CreateFontSetBuilder`／`AddFontFile`／`CreateFontCollectionFromFontSet`／`CreateFontFileReference`／`GetGlyphRunOutline` が **crate ソースに存在することを 2026-09-11 に確認**（`c:\rust\cargo\registry\src\...\windows-0.62.*\src\Windows\Win32\Graphics\DirectWrite\mod.rs`） | 新しい依存は不要。feature 追加も不要 |

**テスト基盤**

| 場所 | 現状 | 本仕様での使い方 |
|---|---|---|
| `tests/draw_readback_test.rs`・`tests/line_pitch_readback_test.rs` | `GraphicsCore::new()`（WARP 可）→ `present_frame` → `TextSurface::read_back()` で画素を読み、インクの位置・行送りを述語にする通し経路が**既にある**。`line_pitch_readback_test.rs` には `ink_runs`／`ink_extent`／`ink_count`／`is_ink` の補助がある | R12.4／R15.4「下線・打ち消し線の側と位置」「装飾あり／なしで画素が違う」はこの経路で組む。縦書きは `tests/vertical_fixture_test.rs` の fixture を使える |
| `draw_test_support.rs`・`viewbox_draw_test_support.rs`・`state_test_support.rs`・`layout_test_support.rs` | 各テーマの共有ヘルパ | 新テーマ（装飾）のヘルパは `<stem>_test_support.rs` へ集約する規約 |
| `draw_format_metrics_tests.rs` の `decoration_and_disable_seams_are_type_only`（:157） | **`size_of::<TextEffects>() == 0`・`size_of::<FontDisableSeam>() == 0` を固定**している | 実体化すると必ず赤になる。R16.4 のとおり、この述語は「実体化した」ことを述べる形へ改訂する（分割の段階では触らず、実体化の段階で改訂） |
| `crates/log-capture-kit/tests/file_length_guard_test.rs` | `OVER_LIMIT_ALLOWED` 11 件・`OVER_LIMIT_ALLOWED_COUNT = 11`・`areka-emo-text` は不在 | 触らない（R1.4）。`draw.rs` 988／`layout.rs` 955／`actor.rs` 952／`region.rs` 951 は要件どおり |

### 1.4 バルーン定義側（触らないが読む）

- `crates/areka-parsers/src/balloon/parse.rs` の `Font::new(font.name, font.height, FontColor)`（:106-117）と `model.rs` の `impl Font { name(), height(), color() }`（:379）。`cursor.font.color.*` は `BalloonCursor::font_color()`（:482）。**`font.bold` 等の残り 8 キーは読めない**（`balloon-font-descript-keys` の所有）。
- **バルーンのフォルダ・ゴーストのフォルダ・バルーン画像の画素は、文字レンダリング層に届いていない**。`TextSlotBinding`（`actor.rs` :50）が運ぶのは `slot`・`window`・`scale`・`surface_size`・`image_size` だけ。`ResolvedBalloonText::resolve(model, image_size)`（:155）も同様。一方、上流 `crates/areka/src/emo2_boot/mod.rs` は `ghost_root`／`balloon_root`（:189・:283・:304）を持つ。

### 1.5 文書・台帳

- `doc/COMPAT_ARCHITECTURE.md` §8 の「`\f[align]`／`\f[valign]`／下線の縦書き写像」の行（:181）は「areka は align／valign を全書字方向でまだ実装していない…本登記は現在の表示結果を変えない」と書く（R16.2 の改訂対象）。「`font.height` の意味・行送りの式・行間の既定」の行（:214）は継承元。
- `doc/ukadoc-coverage/ledger/sakura-script.toml` の本仕様 12 項目は `owner = "areka-P0-text-decoration-canon"`・`status = "absent"` 11 件＋`underline` が `vocabulary-only`（:5064）。`assets.toml` の `disable.font.(フォント定義),(指定)`（:1514）は「`FontDisableSeam` が名前だけ持つ」と書く。
- `crates/areka-emo-text/src/canvas.rs` のモジュール doc（:38）と `draw.rs` の `FontDisableSeam` doc（:141-147）が「M1 では実挙動を一切実装しない」「追跡先は `areka-P0-text-decoration-canon`」と書く（R16.4）。

---

## 2. 要件→資産の対応表

凡例: **Missing**＝無い・新設 ／ **Unknown**＝設計で調べる ／ **Constraint**＝既存の構造・規約による制約 ／ ✅＝そのまま使える資産あり

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| R1 分割と見張り | `draw.rs` は「フォント解決」「計測」「行レイアウトの記憶」「比較用の描画器（`#[cfg(test)]`）」の 4 テーマが同居。テストは既に `#[path]` で兄弟ファイルに出ている | **Constraint**: 公開の入口 6 つ（`DrawExecutor`・`DWriteMetrics`・`LineLayoutStore`・`create_text_format`・`ResolvedFont`・`DirectionRecipe`）を `crate::draw::` のまま保つ→再輸出で解決（§5）。`layout.rs` は残り 45 行しか無い |
| R2 解読と転写 | `decode_tag` の腕の書式・`GenericCommand`→`command_carrier` の転写経路 ✅ | **Missing**: `"f"` の腕・`Instruction` の新 variant・`compile` の腕。運搬形の選択（§3 D1） |
| R3 装飾状態と文字への適用 | `ActorTextState`（スコープ独立）・`apply_cue` の腕構造・`PositionedGlyph`／`GlyphRunContent` ✅ | **Missing**: 装飾状態の型・追記時の付与・3 層への配管・run 分割。**Constraint**: `Clear` の腕が状態を丸ごと `default()` にする／`TextItem::Glyph` の構築 182 か所（§3 D2） |
| R4 2 層の見た目 | `ResolvedFont::resolve`・`DEFAULT_FONT_NAME`／`DEFAULT_FONT_HEIGHT` ✅ | **Missing**: 10 項目を持つ「見た目」の型・無効表示層・「与える口」。**Unknown**: 無効表示の色の混ぜ先＝バルーンの背景色の源（§3 D8） |
| R5 真偽 5 項目 | `SetFontWeight`／`SetFontStyle`／`SetUnderline`／`SetStrikethrough` は範囲指定で可 ✅ | **Missing**: `outline`（範囲指定に無い→自前描画）。**Unknown**: 縦書きで DirectWrite が既定で引く下線・打ち消し線の側（列の右側の裁定と一致するか実測が要る・§3 D3） |
| R6 上下付き | `SetFontSize` は範囲指定で可 | **Missing**: 基線のずらし（範囲指定に無い→自前描画器か run 別レイアウト）。比率・ずらし量の定数 1 か所 |
| R7 大きさ | `line_pitch` の 1 点 ✅・`SetFontSize` ✅ | **Constraint**: `layout_inner` は行内で `font_height` が 1 つ（行内最大 em の追跡が要る）・`LineLayoutStore` の箱寸・`DWriteMetrics.advance` の高さ不一致 `warn!` |
| R8 色 | `SetDrawingEffect`＋ブラシで範囲適用 ✅・`cursor.font.color` の解決済み値 ✅ | **Missing**: 色の書式解析（10 進・%・#RGB/#RRGGBB・色名 147 語・`default.*`）を 1 か所に置く純粋関数。色名表は自前（依存追加なし） |
| R9 フォント名 | `measure_line_box_ratio` の「インストール済みか」の一連 ✅・`fallback_chain` の保持 ✅ | **Missing**: 候補列の解決規則・探索結果の記憶（台詞中の再利用）・フォントファイル読み込み（`CreateFontSetBuilder` 系）。**Unknown**: 探索フォルダ（バルーン／`ghost/master`）を文字レンダリング層へ渡す口（§3 D9） |
| R10 戻す操作 | `ClearAll` が台本の先頭に必ず立つ ✅ | **Missing**: 戻す操作そのもの（スコープ単位・全スコープ）。**Constraint**: `\x` は未実装（後続が呼ぶ）。「項目を足すだけで戻しに含まれる」構造は「見た目の型ごと既定に置き換える」設計で満たせる（列挙を持たない） |
| R11 送り幅と折返し | `GlyphMetrics` の注入点・二段構え（soft／hard）✅ | **Missing**: 装飾込みの鍵と format の複数保持。**Constraint**: `advance(ch, font_height)` の署名（3 実装）。`line_fingerprint` に装飾を含める |
| R12 縦書き | `DirectionRecipe`（3 方向）✅・`vertical_fixture_test.rs` ✅ | **Unknown**: DirectWrite の縦書きでの下線／打ち消し線の既定位置。自前描画器なら側を自分で決められる |
| R13 失敗時の記録 | log-first の書式（`device_err`・`warn!`）✅・`CursorWarnGuard`（スコープごと 1 回の抑止）✅ | **Missing**: 「1 台詞につき同じ値ごとに 1 度」の抑止（`CursorWarnGuard` と同型で組める） |
| R14 表示結果の不変 | オラクル比較（`draw_oracle_tests.rs`・`viewbox_draw_live_diff_tests.rs`）・PNG 比較 ✅ | **Constraint**: 装飾が無い行は**従来と同じ呼び出し**（`DrawTextLayout`）で描く分岐を残すと構造的に満たせる（§3 D4） |
| R15 決定論テスト | 純粋層の兄弟テスト・読み戻しテストの通し経路 ✅ | **Missing**: 3 方向 × 10 項目の読み戻し述語・較正（過去の壊れ方を再現して赤にする） |
| R16 文書・台帳 | §8 の表・台帳の `status`／`owner` 欄・予約名の doc ✅ | 追随のみ |

---

## 3. 論点ごとの選択肢

各論点に「答えで作業が変わるか」を付す。変わらないもの（勝者が明白）は設計で決めて結果だけ報告すればよく、開発者の議題にするのは「変わる」ものだけである。

### D1. `\f` を運ぶ命令の形（R2.4）

| 案 | 内容 | 利点 | 欠点 |
|---|---|---|---|
| **a. 既存の汎用キャリアに乗せる** | `Instruction::Font { args }`（parsers）→ `compile` で `CueCommand::command_carrier("f", args)`（`Custom`）→ 文字レンダリング層が `as_command_carrier()` で `command == "f"` を自己選別 | dola を触らない。`CueCommand` の網羅 match（本番 5 か所＋テスト 17 ファイル）に触れない。「消費側が名前で自己選別する」既存規約そのもの | `\![f,...]` と綴った台本が装飾と衝突する（ukadoc に `\![f]` は無いが、`\!` の名前空間は任意文字列）。`cue_target_of(Custom) = None` のままなので「Balloon 向け」であることは型に現れない |
| **b. dola に typed variant を新設** | `CueCommand::Font { key, args }`・`cue_target_of` → `Balloon` | 型で担当が決まる。`\!` の名前空間と混ざらない | `CueCommand` は `#[non_exhaustive]` でないため、他 crate（`areka-ghost`・`areka-seriko`・`dola` のテスト群）の網羅 match が全部壊れる＝共有ファイル 0 の W13 編成と衝突する（`areka-ghost/src/sink.rs`・`areka-seriko/src/actor.rs` を触る） |

**推奨**: **a**。衝突は「`Custom` の名前を `\f` 由来と分かる予約名にする」（例: コマンド名を `f` でなく `\f` の綴りそのものにする——`\!` の引数に `\` は現れない）で構造的に避けられる。**答えで作業が変わる**（b は他 crate に波及）が、a に明白な優位があるので設計で a を採り、結果だけ報告する。

### D2. 文字ごとの装飾を「型に足す」か「横に持つ」か（R3.3）

| 案 | 内容 | 利点 | 欠点 |
|---|---|---|---|
| **a. `TextItem::Glyph { ch, style }`** | variant にフィールドを足す | 配管が素直 | 構築 182 か所（25 ファイル）の書き換え。`TextItem: Copy` を保つには `style` を番号（`StyleId`）にする必要がある |
| **b. `ActorTextState` に並行ベクタ `glyph_styles: Vec<StyleId>`** | グリフの通し番号（`layout_inner` の `placed`・`segment_advance_sum` の `serial`・`ChoiceSpan.glyph_range` と同じ序数空間）で引く | `TextItem` と 182 か所に触れない。`Clear` の腕の置き換えと無関係に「現在の装飾状態」を別フィールドに置ける | 序数の整合を 2 つの列で保つ（追記は `Text`／`Choice` 腕の 2 か所だけなので危険は小さい）。layout に `&[StyleId]`（と番号→見た目の表）を渡す引数が 1 つ増える |
| **c. `Rc<TextLook>` を各グリフに持たせる** | 参照共有 | 表が不要 | `Copy` が消え、`PositionedGlyph: Copy` に依存する箇所が壊れる |

共通: 見た目の実体（フォント名は `String`）はスコープごとの**表**（`Vec<TextLook>`・同じ内容は同じ番号に畳む）に置き、グリフは番号（`u16`／`u32`）だけ持つ。`PositionedGlyph { ch, inline_pos, advance, style: StyleId }`（構築 7 か所）と `GlyphRunContent` には番号を足し、描画時に表を引く。行の指紋（`line_fingerprint`）は番号列の要約でなく**表の中身**（番号は `Clear` で振り直される）を含めるか、`Clear` で指紋も破棄される事実（`FramePlan::FullClear`）に依拠する。

**推奨**: **b**（変更範囲が最小）。答えで作業量は変わるが、勝者は明白。設計で決める。

### D3. DirectWrite での run の描き方（R3.6・R5〜R7・R12）——**最重要**

前提の事実:
- 範囲指定（`DWRITE_TEXT_RANGE`）で表せるもの: 太さ・斜体・大きさ・フォント名・フォント集合・下線・打ち消し線・色（`SetDrawingEffect` にブラシ）。
- 範囲指定で**表せない**もの: **上下付きの基線のずらし**（`IDWriteTextLayout` に範囲の基線オフセットは無い）・**白抜き**（塗らずに輪郭だけ描く指定は無い）。
- 縦書きで DirectWrite が下線・打ち消し線をどの側に引くかは**未実測**（`SetUnderline` は縦書きでも使えるが、側は DirectWrite が決める）。

| 案 | 内容 | 利点 | 欠点 |
|---|---|---|---|
| **A. 1 行 1 レイアウト＋範囲指定のみ** | 既存 `LineLayoutStore` に属性を焼く | 変更最小。計測（`GetClusterMetrics`）と描画が同じレイアウト | 上下付き・白抜きが描けない→要件を満たせない。縦書きの下線の側を制御できない |
| **B. 1 行 1 レイアウト＋範囲指定＋自前描画器（`IDWriteTextRenderer` の COM 実装）** | 太さ・斜体・大きさ・フォント名・下線・打ち消し線の**有無**は範囲指定で焼き（字形と送り幅は DirectWrite が正しく出す）、描画は `layout.Draw(renderer)` で自前描画器へ。`DrawGlyphRun` で run ごとに色・基線のずらし（上下付き）・白抜き（`GetGlyphRunOutline` → `ID2D1PathGeometry` → `DrawGeometry` で輪郭だけ描く）を処理し、`DrawUnderline`／`DrawStrikethrough` で**線の側と位置を自分で決める**（縦書きで列の右側／中央を保証できる） | 10 項目すべてを 1 つの経路で満たす。縦書きの側が裁定どおりになることを構造で保証。run の切り分けは `clientDrawingEffect`（`SetDrawingEffect` に渡す自前の小さな COM 物）で受け取れる | `#[implement(IDWriteTextRenderer)]`（`IDWritePixelSnapping` の 3 メソッド＋`DrawGlyphRun`／`DrawUnderline`／`DrawStrikethrough`／`DrawInlineObject`）を新規に書く（見積 250〜350 行・新ファイル 1 本）。`DrawTextLayout` と画素が同一になる保証は無い→**装飾の無い行は従来の `DrawTextLayout` のまま**にする分岐が要る（D4） |
| **C. run ごとに別レイアウト** | 行を run に割り、run ごとに `IDWriteTextFormat`（名前・大きさ・太さ・斜体）でレイアウトを作り、原点を行内位置でずらして描く。上下付きは原点のブロック軸ずらしで済む | 自前描画器が不要。`LineLayoutStore` を「run ストア」に読み替えるだけ | 白抜きは結局 `GetGlyphRunOutline` が要る（＝自前描画器か、透明ブラシで中を抜く小細工）。行内で run が変わるたびにカーニング・シェーピングが切れる（和文では実害小）。下線・打ち消し線は DirectWrite 任せ（縦書きの側は未実測のまま）。行の `overhang` 計測・指紋・ダーティ矩形が run 単位に分裂し、`viewbox` 側の変更が広がる |

**推奨**: **B**。理由: ⑴ 白抜きと上下付きは A では不可能、C でも白抜きに描画器相当が要る ⑵ 縦書きの下線の側は裁定済みで再審議しないため、DirectWrite の既定に委ねる A/C は「実測して合わなければ結局 B」になる ⑶ 装飾の無い行を従来経路に残せば R14 が構造で守れる。**答えで作業が大きく変わる**論点だが、要件（白抜き・上下付き・縦書きの側）が事実上 B を指定している。開発者へは「B を採る・DirectWrite 描画器の自前実装が本仕様の最大の新規部品になる」ことを報告し、規模の合意だけ取る。

補足（B の中の小さな選択）: run の属性は `SetDrawingEffect` に渡す小さな COM 物（`#[implement(IUnknown)]` に `StyleId` を持たせる）で描画器に届ける。色ブラシを直接渡す既存 hover 経路と同居できる（hover 範囲は run の効果の上に重ねて後勝ち）。

### D4. 装飾の無い行の描画経路（R14.2）

- **a. 装飾の有無で分岐**: 行に既定以外の run が 1 つも無ければ従来どおり `DrawTextLayout`、あれば `layout.Draw(自前描画器)`。→ 既定だけの行は 1 バイトも変わらない（オラクル比較・PNG 比較が変更なしで緑）。
- b. 常に自前描画器: 経路が 1 本になるが、既存の画素同一性テストを描画器の同一性で再証明する必要がある。

**推奨**: **a**（勝者明白）。

### D5. 送り幅の計測の鍵と format の複数保持（R11.1・R11.3）

- `GlyphMetrics` に **既定実装付きの新メソッド**（例 `advance_styled(&self, ch, look: &TextLook) -> f32` の既定＝`advance(ch, look.height)`）を足す→ `FixedMetrics`・`LegacyPitchMetrics` と既存呼び手は無変更。`DWriteMetrics` だけが上書きし、鍵 `(char, フォント名, 大きさ, 太さ, 斜体, 上下付きの縮小)` で記憶し、format を `HashMap<StyleKey, IDWriteTextFormat>` で持つ。
- 計測と描画で同じ format 生成関数（`create_text_format` の引数化版）を通せば R11.3 の「同じ経路」が保てる。上下付きは縮小後の大きさで計測する（R6.6）。

**推奨**: 上記 1 案（勝者明白）。

### D6. 行の高さ（R7.9）と `layout.rs` の余裕

- `layout_inner` で行内最大 em を追跡し、`finish_line` の丈と `block_pos += block_dir * pitch(line_max)` に使う。文字の無い行は「そのとき効いている大きさ」。
- **Constraint**: `layout.rs` は残り 45 行。変更前に自己完結した塊（候補: `visible_window`（約 60 行）または `segment_advance_sum`＋`resolve_cursor_component`＋3 つの `apply_pending_*`）を兄弟ファイルへ出す。`region.rs`（951）は変更が無い見込み。`actor.rs`（952）は `present_actor` に「番号→見た目の表」を渡す数行だけ。

### D7. 見た目の型と 2 層の置き場所（R4）

- 新しい純粋ファイル（例 `crates/areka-emo-text/src/look.rs`）に `TextLook { name: Vec<String>（候補列）, height, color, bold, italic, underline, strike, outline, script: None|Sub|Sup }` と、`\f` の値を解釈して `TextLook` を更新する状態機械（`apply_font_tag(&mut TextLook, key, args, default: &TextLook, disable: &TextLook) -> Result<(), 記録用の理由>`）を置く。`ResolvedFont` は `default_look`／`disable_look` の 2 つを持ち、`effects`／`disable` の空フィールドを撤去する（`ResolvedFont` は `#[non_exhaustive]` なので外部の構築は無い。構築は `draw.rs` と兄弟テストの 6 か所）。
- 「バルーン定義から与える口」＝`TextLook` の各フィールドを `Option` で上書きする `with_balloon_keys(&mut self, ...)` を 1 つ用意し、`balloon-font-descript-keys` 着地時にそこへ値を差し込むだけにする（R4.3／4.7）。
- 「戻す操作」（R10）は `current = default.clone()`（スコープ）／全スコープの繰り返し。**項目の列挙を持たない**ので、後続が `TextLook` にフィールドを足せば自動的に戻しに含まれる（R10.4）。

### D8. 無効表示の色（R4.6）——**背景色の源が無い**

要件は「既定の文字色をバルーンの背景色の側へ寄せた色」で、混ぜ方は shell の式 `(background 画像の 0,0 の色 + 文字色 × 2) / 3` を輸入する。**文字レンダリング層はバルーン画像の画素を持っていない**（届くのは `image_size`・`scale`・entity だけ）。

| 案 | 内容 | 利点 | 欠点 |
|---|---|---|---|
| **a. 背景色を「与える口」だけ用意し、当面は定数（白）で混ぜる** | `ResolvedBalloonText` に `background_color: (u8,u8,u8)`（既定 255,255,255）を足し、`register_actor` で上流が渡せるようにする | 本仕様の範囲で閉じる。バルーン画像の (0,0) を読む配線は上流（`emo2_boot`／atlas）が後で差し込める | 背景が暗いバルーンでは「薄い」でなく「明るい」に見える（白と混ぜるため）。利用者から見える差: 黒文字なら (170,170,170) の灰色になる |
| **b. バルーン画像の (0,0) を上流で読んで渡す** | `emo2_boot` はバルーン画像を読んでいるので、装着時に 1 画素を取り出して `TextSlotBinding` か `register_actor` の引数に足す | ukadoc の式どおり | `areka-emo-present`／`areka` 側の配線が要る（本仕様の編集集合が広がる。W13 の他 spec との共有ファイルは `emo2_boot/mod.rs` に及ぶ可能性→要確認） |

**答えで作業が変わる**（b は上流 crate を触る）。推奨は **a**（口を用意し既定は白）、b は「口に値を入れる」だけの後続作業として brief に登記。**開発者の議題候補**（利用者から見える差は「暗いバルーンで無効表示が薄くならず明るくなる」の 1 点）。

### D9. フォントファイルの読み込み（R9.3・R9.9）

- API は `IDWriteFactory3::CreateFontSetBuilder` → `AddFontFile(path)` → `CreateFontSet` → `CreateFontCollectionFromFontSet` → format 生成時に集合を渡す（範囲指定なら `SetFontCollection`）。集合内の family 名は `IDWriteFontSet::GetPropertyValues` で引く必要がある（ファイル名→family 名の対応は自明でない。**Research Needed**）。
- **探索フォルダが文字レンダリング層に無い**（D8 と同じ構造）。`register_actor` に `font_dirs: Vec<PathBuf>`（バルーンのフォルダ・`ghost/master`）を渡す口が要り、上流 `emo2_boot` が `balloon_root`／`ghost_root` を持っているので配線は短い。
- R9.9 が「最後のひとまとまり・後送り可」と定めているので、設計は **口＋縮退（候補を `warn` で読み飛ばす）を先に、読み込み本体を最後のタスク**にする。答えで作業は変わらないが規模には効く。

### D10. 色の解析の置き場所（R8.10）

- 純粋関数 `parse_color(&str) -> Result<Color, 理由>` を新ファイル（例 `crates/areka-emo-text/src/color.rs`）に置く。色名 147 語は静的表（依存追加なし。repo に色名を引く crate は無い）。`default.*` 系は色でなく「戻し先の種類」なので、解析結果を `enum ColorSpec { Rgb, Default, DefaultPlain, Disable, DefaultCursor, DefaultCursorNotSelect, DefaultAnchor* }` の語彙にして、値の解釈（どの色を引くか）は状態機械側に置く（`parse_cursor_coord` と同じ「語彙と解決の分離」）。勝者明白。

### D11. 「台詞の開始」の観測点（R3.8・R10.3）

- `compile` が内容のある台本の先頭に必ず `ClearAll` を置く。文字レンダリング層は `ClearAll` を「新しい台本の先頭」として全スコープの装飾を既定に戻せば R3.8 を満たす。`\c`（`Clear`）は戻さない（R3.7）。
- 後続の `\x` は同じ「戻す操作」を呼ぶ（本仕様は関数を公開するだけ）。
- 注意: `Clear` の腕（`state.rs` :403）は `ActorTextState` を丸ごと `default()` に置き換えるため、装飾状態を `ActorTextState` に置くなら `Clear` の腕で装飾だけ引き継ぐ、または別フィールドに置く（D2-b と同じ判断）。勝者明白。

### D12. `draw.rs` の分割の形（R1）——§5 参照

- **a. `draw.rs` の中に `#[path = "draw_font.rs"] mod font;` と `pub use font::*;`**（テストの `#[path]` 接続と同じ作法・`lib.rs` 無変更・外部パス `crate::draw::X` 不変）。
- b. `lib.rs` に平置きモジュールを足し `draw.rs` から `pub use`。
- どちらも R1.2 を満たす。a は「兄弟ファイル」の命名規則にそのまま乗る。勝者明白。

### D13. スタイルシートの大きさキーワード・`default.anchor*`・所有外キー（R7.7・R8.8・R2.5）

- 受理して表示を変えず記録だけ残す。「1 度だけ」の抑止は `CursorWarnGuard`（`cursor_tag.rs` :223）と同型の小さな集合で足りる。勝者明白。

---

## 4. 実装方針の総合

- **A（既存の拡張のみ）**: 不可。`draw.rs` に足せず、上下付き・白抜きが範囲指定で表せない。
- **B（新設のみ）**: 不可。解読の腕・`compile` の腕・`state.rs` の腕・`layout_inner` の行高さ・`viewbox_draw.rs` の効果適用は既存関数の中の変更である。
- **C（併用・推奨）**:
  1. `draw.rs` をテーマ別に兄弟ファイルへ分割（既存関数は移動のみ・テスト無変更）。
  2. 新設: `look.rs`（見た目の型・2 層・`\f` の値の状態機械・戻す操作）、`color.rs`（色の解析）、`draw_font.rs`／`draw_metrics.rs`／`draw_line_store.rs`（分割先）、`draw_renderer.rs`（`IDWriteTextRenderer` の実装）、`draw_font_files.rs`（フォントファイル・最後のタスク）、各兄弟テスト。
  3. 既存の腕の追加: `decode_tag`・`compile`・`state.rs::apply_cue`・`actor.rs::apply_cue`（no-catch-all 規律で `Custom` の腕は既にある）・`viewbox_draw.rs` の資源確定区間（run の効果適用）・`layout_inner`（行高さ）・`line_fingerprint`（装飾の要約）。
  4. 段階: ⑴ 分割 → ⑵ 解読・転写・状態機械（純粋層・描画なしで決定論テストが緑に）→ ⑶ 計測の鍵と行高さ → ⑷ 範囲指定で描ける 6 項目（太さ・斜体・大きさ・フォント名・下線・打ち消し線＝横書き）→ ⑸ 自前描画器（色・上下付き・白抜き・縦書きの線の側）→ ⑹ 2 層と戻し・記録 → ⑺ フォントファイル → ⑻ 文書・台帳。

---

## 5. `draw.rs` の分割案（988 行→本体約 350 行）

| 新ファイル | 移すもの（定義名） | 概算 |
|---|---|---|
| `draw_font.rs` | `DEFAULT_FONT_NAME`・`DEFAULT_FONT_HEIGHT`・`LOCALE_JA_JP`・`RESERVED_KEY_DISABLE_FONT_PREFIX`・`FontDisableSeam`・`ResolvedFont`＋`impl`・`DirectionRecipe`＋`impl`・`create_text_format`・`try_create_format` | 約 250 行 |
| `draw_metrics.rs` | `PROBE_MAX_EXTENT`・`DWriteMetrics`＋`impl`・`impl GlyphMetrics for DWriteMetrics`・`measure_line_box_ratio` | 約 190 行 |
| `draw_line_store.rs` | `CachedLineLayout`・`LineLayoutStore`＋`impl`・`measure_line_overhang` | 約 130 行 |
| `draw.rs`（残す） | モジュール doc・`FormatKey`（test）・`DrawExecutor`（test・比較用）・`create_d2d_target_bitmap`・`create_target_bitmap`・`none_err`・`device_err`・`#[path]` 接続と `pub use` | 約 350 行 |

- 接続は `#[path = "draw_font.rs"] mod font; pub use font::{...};`（`pub(crate)` の `LineLayoutStore` は `pub(crate) use`）。`device_err` は 3 ファイルが使うので `draw.rs` に残し `super::device_err` で辿る（構造 steering の「サブモジュールから見た `super` はファサード自身」の注記どおり）。
- 兄弟テストの命名規則との衝突確認: 既存 `draw_format_metrics_tests.rs`・`draw_oracle_tests.rs`・`draw_test_support.rs` は最長 stem の規則で `draw` に解決され、新 stem `draw_font`／`draw_metrics`／`draw_line_store` とは衝突しない（`draw_metrics_` で始まる既存テスト名は無い）。
- 分割後に本仕様が足す描画器（`draw_renderer.rs`）と装飾の効果適用は新ファイルに置く。`viewbox_draw.rs`（852）への追加は run 範囲の焼き込み呼び出し（数十行）に留め、範囲の算出は新ファイルへ。

---

## 6. 規模とリスク

| 区分 | 見立て | 根拠 |
|---|---|---|
| 規模 | **L** | 新規部品: 見た目の型と状態機械（純粋・約 300 行＋テスト）・色の解析（約 250 行・色名表込み）・自前描画器（約 300 行）・分割（移動のみ）・計測の鍵（約 100 行）・読み戻しテスト（3 方向 × 10 項目）。フォントファイル読み込みは後送り可 |
| リスク | **中** | 未知数は 2 つ: ⑴ `#[implement(IDWriteTextRenderer)]` の実装（repo に DirectWrite の COM 実装の先例が無い・shiori 系の `#[implement]` 作法は流用可）⑵ 縦書きでの線の位置と上下付きの側の実測。既存の型・経路の延長部分（解読・転写・状態機械・範囲指定）はリスク低 |
| 並走との干渉 | 低 | W13 の共有ファイルは 0（`sakura-tag-word-boundary` は `lexer.rs`・本仕様は `decode.rs`／`compile.rs`）。D1-a を採れば dola と他 crate に触れない。D8-b／D9 の上流配線を本仕様で行う場合のみ `crates/areka/src/emo2_boot/mod.rs` に触れる可能性がある（W13 の他 spec との共有を再確認） |

---

## 7. 設計フェーズへ持ち越す調査項目（Research Needed）

1. **縦書きでの DirectWrite の既定の下線・打ち消し線の側**（`SetUnderline` を `DWRITE_READING_DIRECTION_TOP_TO_BOTTOM` で使ったときの位置）。自前描画器（D3-B）で側を決めるなら参考値、範囲指定に委ねる案なら決定的。読み戻しテストの通し経路で 30 分程度で測れる。
2. **`#[implement(IDWriteTextRenderer)]` の最小実装**の確認（`IDWritePixelSnapping` の 3 メソッドの戻り値・`DrawGlyphRun` から `ID2D1DeviceContext::DrawGlyphRun` を呼ぶ形・`clientDrawingEffect` の取り出し方）。windows-core 0.62 の `*_Impl` 面の作法は `crates/areka/src/shiori_host.rs` の注記が先例。
3. **白抜きの描き方**: `IDWriteFontFace::GetGlyphRunOutline` → `ID2D1PathGeometry`（`ID2D1Factory::CreatePathGeometry`＋`Open` の sink）→ `DrawGeometry`（線幅 1 image px 相当）。emo-text は `ID2D1Factory` を直接持っていないので `GraphicsCore` から取り出す口を確認する。
4. **フォント集合の family 名の引き方**（`IDWriteFontSet::GetPropertyValues(DWRITE_FONT_PROPERTY_ID_FAMILY_NAME)`）と、`.ttc` の複数 face の扱い。R9.9 で後送り可。
5. **`Custom` に載せるコマンド名の予約**（D1-a）: `\!` の引数に現れ得ない綴りを選ぶ根拠（`lexer.rs` が `\` で語を切る事実）を設計に書く。
6. **`line_fingerprint` に装飾を含める粒度**: 番号列か、表の中身の要約（ハッシュ）か。番号が `Clear` で振り直されても `FullClear` が指紋を捨てる事実で足りるかを確認する。
7. **上下付きの縮小比率とずらし量の定数**（R6.4・areka 裁量）: 候補は CSS の慣例（縮小 ≈ 0.58〜0.7・ずらし ≈ ±0.3 em）。値は設計で決めて §8 に登記する。
8. **`DWriteMetrics.advance` の高さ不一致 `warn!`**（:457）を、装飾込みの鍵へ改めたあとどう扱うか（撤去か、鍵に高さを含めて不一致自体を消すか）。

---

## 8. 要件と現状の食い違い・注意点（設計で必ず拾うもの）

- **`\c` で装飾が消える構造**: `state.rs::apply_cue` の `Clear` 腕は `ActorTextState` を丸ごと `default()` にする。装飾状態を同じ構造体に置くなら、`Clear` の腕で装飾だけを引き継ぐ実装が要る（R3.7）。`Choice` 腕・`Text` 腕の追記点は 2 か所だけなので、装飾の付与漏れは起きにくい。
- **`DWriteMetrics::advance` は高さが違うと `warn!` を毎回出す**: `\f[height]` の行で文字ごとに警告があふれる。鍵を装飾込みへ改める作業（R11.1）と同時に消す。
- **`decoration_and_disable_seams_are_type_only`（`draw_format_metrics_tests.rs` :157）は実体化で必ず赤**: R1.5「分割後に既存テストが変更なしで緑」は分割の段階の話で、実体化の段階では R16.4 に従って述語を改訂する。分割と実体化を**別タスクに分ける**理由がここにある。
- **`Resident.effects: TextEffects`（`canvas.rs` :271）と `RESERVED_EFFECT_*` 4 定数**: `outline` は run 単位の属性になるので行単位の `effects` には載らない。`multicolor`／`rotation` の予約名は残し、`TextEffects` を「行単位の予約」として残すか撤去するかを設計で決める（R16.4 は「フィールド 0 個の予約型を残さない」と `outline` について定める）。
- **`\f[underline]` の台帳は `vocabulary-only`、他 11 件は `absent`**（`sakura-script.toml` :5064）: 実装後は 12 件とも `implemented`（スタイルシートのキーワードのみ注記付き）へ。`assets.toml` の `disable.font.*` の行も同時に。
- **`GenericCommand` の「空トークンを保持する」規約**は `\f[]`・`\f[bold,]` にも当てはまる（R2.2）。`args` の空文字列を潰さない。
- **`\f` bare（引数なし）**は lexer で bare タグになり `decode_bare` → `Raw` へ落ちる。R2.6 は「`\f` も `warn` で表示不変」と言うので、`decode_bare` にも `"f"` の腕（`Font { args: vec![] }`）を足すか、`Raw` のまま捨てる（記録は `compile` の catch-all の `debug!`）かを設計で決める。後者は R2.6 の `warn` と食い違う。
- **上流の配線が無い 2 点**（背景色・フォントの探索フォルダ）は本仕様の brief の「編集集合」に無い crate（`areka`／`areka-emo-present`）に触れる可能性がある。口だけ用意して値の配線を後続へ渡すなら、その追跡先を brief に相互登記する（R16.6）。
