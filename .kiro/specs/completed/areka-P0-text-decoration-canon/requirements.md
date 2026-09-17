# Requirements Document

## Project Description (Input)
さくらスクリプトの文字装飾タグ `\f[...]` のうち、**フォント系 10 項目**（`name`・`height`・`color`・`bold`・`italic`・`underline`・`strike`・`sub`・`sup`・`outline`）のうち **DirectWrite の範囲指定で素直に表せる 7 項目**（`name`・`height`・`color`・`bold`・`italic`・`underline`・`strike`）と**一括の戻し 2 項目**（`\f[default]`・`\f[disable]`）を、3 つの書字方向（`horizontal_tb`／`vertical_rl`／`vertical_lr`）すべてで ukadoc の定めどおりに効かせる。範囲指定で表せない 3 項目（`sub`・`sup`・`outline`）は語彙として受理し表示は変えない（開発者裁定 2026-09-11: DirectWrite で普通に書ける範疇に限り、自前の描画器など逸脱する手段は採らない）。あわせて、その土台となる**基盤**（解読の腕・装飾を運ぶ命令・文字ごとの属性の配管・既定の見た目と無効表示の見た目の 2 層・`draw.rs` の分割）を建てる。

- **困っている人**: 既存の伺かゴースト作者およびその利用者。SSP 向けに `\f[bold,1]` や `\f[color,red]` を使って書かれた台詞が、areka では素の文字で表示される。
- **現状**: `\f[...]` は 43 項目のうち **1 項目も解読されていない**。字句解析は通るが、解読の分岐に `"f"` の腕が無いため生のまま保持され（`Instruction::Raw`）、台本を組み立てる段階で「対象外のタグ」として記録だけ残して捨てられる。文字ごとの属性を持つ場所も無く（グリフは文字 1 つだけを持つ）、フォントはバルーン全体で 1 つ、太さと斜体は常に標準に固定されている。
- **変わるべきこと**: 上記 7 項目＋戻し 2 項目が 3 書字方向で正典どおりに効き、`sub`・`sup`・`outline` は語彙として受理され、`\f[default]`／`\f[disable]` が「何を戻すか」を一か所で定め、それを後続の仕様（`\x` のクリック待ち・選択肢マーカーの装飾）がそのまま使える。残る 31 項目（寄せ 2・影 3・選択肢マーカー 10・アンカー 16）は本仕様が建てる基盤の上に、それぞれの所有仕様が後から乗る。

---

## Introduction

`\f[...]` は、バルーンの文字の見た目（フォント・大きさ・色・太字・斜体・下線・打ち消し線・上下付き・白抜き・寄せ・影）を台詞の途中で切り替えるタグ族である。ukadoc は 43 項目を定めるが、areka では 2026-09-11 現在、**どの項目も画面に影響しない**。`\f[bold,1]` は字句解析を通ったあと、解読関数（`crates/areka-parsers/src/sakura/decode.rs` の `decode_tag`）に `"f"` の腕が無いため `Instruction::Raw` へ落ち、台本の組み立て（`crates/areka-sakura/src/compile.rs` の「M-boot 外タグを無視」の腕）で記録だけ残して捨てられる。

不足しているのは語彙よりも**土台**である。文字レンダリング層（`crates/areka-emo-text`）は、追記される 1 文字（`TextItem::Glyph { ch }`）・配置済みの 1 文字（`PositionedGlyph`）・行のグリフ列（`GlyphRunContent`）のいずれも文字以外の属性を持たず、フォントは balloon 定義から 1 つだけ解決され（`ResolvedFont`）、DirectWrite の書式も 1 本で、太さ・斜体は標準固定である。文字装飾の予約型 `TextEffects` はフィールドが 0 個の空の型、`disable.font.*` の予約 `FontDisableSeam` も同様で、「格納はされるが読まれない」。

本仕様は、2026-09-11 の棚卸しで 3 分割された `\f` 装飾仕様群の**先頭**（基盤＋フォント系 10 項目＋一括の戻し 2 項目）である。**描画の手段は DirectWrite の標準機能（`IDWriteTextLayout` の範囲指定と `DrawTextLayout`）に限る**（開発者裁定 2026-09-11）。範囲指定で表せない上下付き（基線のずらし）と白抜き（輪郭だけの描画）は本仕様では表示を変えず、語彙として受理するにとどめる。寄せ 2 項目と影 3 項目は `areka-P0-text-align-shadow-canon`、バルーン定義 `font.*` 13 キーの読み取りは `areka-P0-balloon-font-descript-keys` が持つ。本仕様は `crates/areka-parsers/src/balloon/parse.rs` に**触れない**——既定の見た目は、いま読めている 5 キー（`font.name`・`font.height`・`font.color.r/g/b`）だけから組む。

意味論は ukadoc から輸入する（SSP の実測は行わない・開発者方針 2026-09-05）。縦書きでの写像（下線は列の右側、など）は完了仕様 `areka-P0-balloon-vertical-canon` の裁定を継承し、再審議しない——ただし線を引く位置は DirectWrite の既定に委ねるので、裁定は「期待値」として実測で照合し、食い違えば §8 に登記する（areka 側で線の位置を自前で描き分けることはしない）。`font.height` は em の大きさとして DirectWrite へそのまま渡し、行送りの式は `crates/areka-emo-text/src/state.rs` の `TextLayerConfig::line_pitch` の 1 点にある（完了仕様 `areka-P0-emo-text-line-height-canon` の裁定）——`\f[height]` もこの 1 点を通る。

**着手の前提**: 1 ファイル 1,000 行の見張り（`crates/log-capture-kit/tests/file_length_guard_test.rs`・例外表 11 件・例外表は増やさない）に対し、`crates/areka-emo-text/src/draw.rs` は **988 行**（残り 12 行）、`layout.rs` 955・`actor.rs` 952・`region.rs` 951 も射程にある。本仕様の最初の作業は `draw.rs` の分割であり、既存ファイルへ足す変更は新しいファイルへ置く。

---

## Boundary Context

- **In scope（利用者から見える範囲）**
  - `\f[...]` 全 43 形の**受理と転写**（解読の腕・装飾を運ぶ命令）。本仕様が意味を与えるのは下の 12 項目で、残りは「語彙として受理するが表示は変えない」状態に置き、所有仕様が後から消費者を足せる形にする。
  - フォント系 10 項目のうち表示に効く 7 項目: `name`・`height`・`color`・`bold`・`italic`・`underline`・`strike`。残る 3 項目（`sub`・`sup`・`outline`）は語彙として受理し表示は変えない（受理・6 値の解釈・戻しへの参加まで）。
  - 一括の戻し 2 項目: `\f[default]`・`\f[disable]`。および「`\f` の状態の何が・いつ戻るか」の権威定義（後続仕様が消費する）。
  - 既定の見た目（バルーン定義から読めている 5 キー）と、無効表示の見た目（`disable.font.*` に相当する第 2 の既定層）の 2 層。
  - 文字ごとの属性の配管（追記→配置→行のグリフ列→描画）と、属性に応じた送り幅の計測・折返し。
  - 3 書字方向での写像（`areka-P0-balloon-vertical-canon` の裁定を継承）。
  - `draw.rs` の分割と、1,000 行の見張りを緑に保ったままの実装。
  - 3 書字方向 × 7 項目 × 戻しの経路の決定論テスト（語彙のみの 3 項目は状態の遷移だけを固定）。
  - 正典文書（`doc/COMPAT_ARCHITECTURE.md` §8）・網羅調査の台帳・コード内の予約名の注記の追随。

- **Out of scope（本仕様が持たない範囲）**
  - `\f[align]`／`\f[valign]`（寄せ 2）・`\f[shadowcolor]`／`\f[shadowcolor,none]`／`\f[shadowstyle]`（影 3）・`\_l` からの追加登記 4 件（`\_l` 直後の左寄せ戻し・中央揃えのインデント・疑義 SC8 の裁定・行送り方向へ後戻りした行のあふれ判定の前提）——すべて **`areka-P0-text-align-shadow-canon`** が持つ。本仕様は寄せ・影の値を受理して保持するだけで、行の置き場所や影の描画は変えない。
  - バルーン定義 `font.*` 13 キーの読み取り拡張（`font.bold`／`font.italic`／`font.underline`／`font.strike`／`font.outline`／`font.shadowcolor.*`／`font.shadowstyle`・`disable.font.*`）——**`areka-P0-balloon-font-descript-keys`** が持つ。本仕様は `crates/areka-parsers/src/balloon/parse.rs`・`model.rs` に触れない。
  - `\f[cursor*]` 10 項目（`areka-P0-choice-marker-styling`）・`\_a` とアンカー装飾 16 項目（`areka-P0-anchor-tag-canon`）。
  - `\x`／`\x[noclear]` の実装（`areka-P0-balloon-lifecycle-events` 項目 9）。本仕様は「戻す操作」を供給するだけで、クリック待ちそのものは作らない。
  - `TextEffects` の予約名のうち `multicolor`・`rotation`（M2 の予約のまま残す）。`shadow` は影 3 項目の所有仕様が実体化する。
  - 行末禁則のぶら下がり・`writing_mode` の警告文言・縦書き字形の観測（`areka-P0-emo-text-canon-residue`）。
  - `\f[name]` のフォントファイル（`.ttf`／`.otf`／`.ttc`）の読み込み（開発者裁定 2026-09-11・実装しない。ファイル名の候補は読み飛ばして次の候補へ進む）。
  - `sub`・`sup`・`outline` の表示（開発者裁定 2026-09-11・DirectWrite の範囲指定で表せず、自前の描画器を要するため実装しない。語彙として受理し `TextEffects` の予約と同列に M2 予約へ置く）。
  - DirectWrite の標準機能から逸脱する描画手段（`IDWriteTextRenderer`／`IDWriteInlineObject` の自前実装・グリフ輪郭の取り出し・run ごとの別レイアウト）。
  - `sstpmessage.font.*`／`number.font.*`／`communicatebox.font.*` 等の接頭辞付き font 族（各機能の仕様が解禁時に本基盤へ乗る）。

- **Adjacent expectations（隣接する仕様・運用への期待）**
  - `areka-P0-text-align-shadow-canon`（W15）は本仕様の文字ごとの属性と「戻す操作」に自分の項目（align・valign・影 3）を**登記して**乗る。本仕様は寄せ・影の値を捨てずに保持し、同仕様が消費者を足すだけで済む形にする。
  - `areka-P0-balloon-font-descript-keys`（W13・並走）が残り 8 キーをバルーン定義から読めるようにする。**後に着地した方が配線する**（同仕様の brief に相互登記済み）——本仕様が先なら既定層は 5 キーで立ち、残り 8 キーの配線は同仕様の最後の作業になる。本仕様が後なら本仕様が既定層へ 13 キーを繋ぐ。
  - `areka-P0-balloon-lifecycle-events` 項目 9（`\x`／`\x[noclear]`）と `areka-P0-choice-marker-styling` は、本仕様が定める「戻す操作」（Requirement 10）を**そのまま呼ぶ**。`\x` は戻す操作を呼び、`\x[noclear]` は呼ばない。
  - `areka-P0-sakura-tag-word-boundary`（W13・並走）は同じ `crates/areka-parsers/src/sakura/` の `lexer.rs` を触る（本仕様は `decode.rs`・`compile.rs`）。共有ファイルは 0 で、後に着地した方が rebase する。
  - 適合対象ゴースト emo2 は `\f` を 1 つも使わない。本仕様は `\f` を含まない台本の表示結果を変えず、M1 の適合検証（`areka-P0-emo2-conformance-e2e`）を妨げない。
  - 1,000 行の見張りの例外表は誰も触らない運用が続く。
  - 網羅調査の台帳（`doc/ukadoc-coverage/ledger/sakura-script.toml`）は本仕様の 12 項目を `owner = "areka-P0-text-decoration-canon"` で載せている（12 件とも 2026-09-11 実測）。`assets.toml` の `font.*` 13 キーの `owner` 欄はいま本仕様を指しているが、読み取りの所有は `areka-P0-balloon-font-descript-keys` へ移ったため、同欄の書き換えは同仕様が行う。

---

## Requirements

### Requirement 1: 着手の前提——`draw.rs` の分割と 1,000 行の見張り

**Objective:** As a リポジトリの保守者, I want 本仕様の変更が 1 ファイル 1,000 行の見張りを赤にしないこと, so that 例外表を増やさずに文字装飾の基盤を足せる

#### Acceptance Criteria
1. The system shall `crates/areka-emo-text/src/draw.rs`（2026-09-11 実測 988 行）を、文字装飾の変更を足す**前に**テーマ単位の複数ファイルへ分割する（本仕様の最初の作業）。
2. The system shall 分割後も `DrawExecutor`・`DWriteMetrics`・`LineLayoutStore`・`create_text_format`・`ResolvedFont`・`DirectionRecipe` の公開の入口（名前と呼び出し方）を crate 内から見て変えない（呼び手 `actor.rs`・`viewbox_draw.rs` の変更は `use` の付け替えに限る）。
3. When 本仕様が `crates/areka-emo-text/src/` の `layout.rs`（955 行）・`actor.rs`（952 行）・`region.rs`（951 行）・`viewbox_draw.rs`（852 行）・`viewbox.rs`（846 行・行の再利用判定を持つ）・`canvas.rs`（738 行）・`state.rs`（528 行）へ振る舞いを足すとき, the system shall 追加分を新しい兄弟ファイルへ置き、既存ファイルの行数を上限へ近づけない。
4. The system shall `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表（`OVER_LIMIT_ALLOWED`・11 件）を 1 件も増減させない。
5. When 分割が完了したとき, the system shall 既存の決定論テスト（`draw_oracle_tests.rs`・`draw_format_metrics_tests.rs`・`viewbox_draw_*_tests.rs` ほか `areka-emo-text` の全テスト）が変更なしで緑のまま通る。
6. The system shall 分割で生じた新ファイルの接続と命名を steering `structure.md` の「1 ファイル 1,000 行以下の目安」と兄弟テストの命名規則に従わせる。

### Requirement 2: `\f[...]` の解読と転写

**Objective:** As a 既存ゴーストのスクリプト作者, I want `\f[...]` が捨てられずに文字レンダリング層まで届くこと, so that 装飾の指定が台詞のとおりに効く

#### Acceptance Criteria
1. When 台本に `\f[キー,値...]` の形のタグが現れたとき, the system shall 解読関数（`crates/areka-parsers/src/sakura/decode.rs` の `decode_tag`）で `"f"` の腕として受理し、`Instruction::Raw` へ落とさない。
2. The system shall 受理した `\f` の**キー**と**引数列**を、意味を解釈せず記述順のまま失わずに保持する（43 形すべて・引数が空の `\f[]`・引数の途中が空の形を含む）。
3. When 台本を組み立てるとき（`crates/areka-sakura/src/compile.rs`）, the system shall `\f` を再生時間 0 の命令として、前後の文字の並び順を保ったまま台本へ載せ、「M-boot 外タグを無視」の腕へ落とさない。
4. The system shall `\f` を運ぶ命令を、キーごとに型を新設せず **1 本の運搬形**で表現する（`\!` コマンドの汎用キャリアと同じ考え方）。消費側（文字レンダリング層）がキーで自己選別する。
5. When 文字レンダリング層が本仕様の所有外のキー（`align`・`valign`・`shadowcolor`・`shadowstyle`・`cursor*`・`anchor*`・`anchor.font.color`）を受け取ったとき, the system shall 値を捨てずに保持し、表示は変えず、`debug` の記録を残す（後続の所有仕様が消費者を足すまでの「語彙のみ」の状態）。
6. If `\f` のキーが 43 形のいずれでもない、または引数が無い（`\f[]`・`\f`）とき, the system shall 表示を変えず `warn` の記録を残し、解析も再生も中断しない。引数なしの `\f` は字句解析で bare 形になり `decode_tag` でなく `decode_bare` を通るため、同関数にも `"f"` の腕を置いて同じ扱いにする（`Raw` のまま `compile` の catch-all で `debug` に落とす経路を残さない）。
7. The system shall 解読の腕の各項目に、対応する ukadoc の URL を `// ukadoc:` の 1 行コメントで添える（既存の腕と同じ書式）。
8. The system shall `\f` を含まない台本の解読結果・台本の内容・再生時間を 1 バイトも変えない（既存の `decode_tests.rs`・`compile_*_tests.rs` が変更なしで緑）。

### Requirement 3: 装飾状態の保持と文字への適用

**Objective:** As a 利用者, I want 装飾の指定がそれ以降の文字にだけ効き、台詞の途中で切り替わること, so that 「ここだけ太字」のような表現が台詞のとおりに見える

#### Acceptance Criteria
1. The system shall スコープ（`\0`・`\1`・`\p[n]`）ごとに独立した「現在の装飾状態」を持つ（本体側と相方側の指定は互いに干渉しない）。
2. When `\f` の命令が届いたとき, the system shall 現在の装飾状態を更新し、**その後に追記される文字**へ新しい状態を与える。それ以前に追記済みの文字の見た目は変えない（フォント系 10 項目は遡って効かない）。
3. The system shall 追記される 1 文字（`TextItem::Glyph`）・配置済みの 1 文字（`PositionedGlyph`）・行のグリフ列（`GlyphRunContent`）のそれぞれが、その文字に効く装飾を保持して下流へ渡す（文字ごとの属性の配管）。
4. While 文字が 1 文字ずつ現れる途中（リビール中）でも, when `\f` の命令が届いたとき, the system shall それ以降に現れる文字へ即時に適用し、文字の現れる時刻・間隔を変えない。
5. When 選択肢（`\q`）の表示文字列が追記されるとき, the system shall そのときの装飾状態を選択肢の文字にも与える。ただし選択肢の hover 表示（文字色の差し替えと帯の塗り）は既存の規則が優先し、装飾は hover の判定と当たり判定を変えない。
6. The system shall 1 行の中で装飾の異なる文字が混在しても、各文字を自分の装飾で描く（行を装飾ごとの連続区間＝run に分けて描き分ける）。
7. When `\n`・`\n[...]`・`\_l`・`\c` が現れたとき, the system shall フォント系の装飾状態を**保持**する（これらは装飾を戻さない。`\_l` が戻すのは寄せだけ——所有は `areka-P0-text-align-shadow-canon`）。
8. When 台詞の再生が始まるとき（新しい台本の先頭）, the system shall 全スコープの装飾状態を既定の見た目へ戻す（装飾は台詞をまたいで残らない。根拠は ukadoc `\x` の項「`\e` で解除されるさくらスクリプトは継続」＝`\f` の効果は台詞の終わりで消える側に属する）。
9. The system shall 装飾状態の更新と文字への適用を、GPU や COM に依存しない純粋な層（`state.rs`・`layout.rs` の兄弟）で行い、決定論テストが描画なしで観測できる形にする。

### Requirement 4: 既定の見た目と無効表示の見た目（2 層）

**Objective:** As a バルーン作者, I want バルーン定義の `font.*` が「戻す先」として効き、`disable` が薄い文字になること, so that `\f[...,default]` と `\f[...,disable]` が ukadoc の定めどおりに振る舞う

#### Acceptance Criteria
1. The system shall 「既定の見た目」を、バルーン定義から現在読めている 5 キー（`font.name`・`font.height`・`font.color.r/g/b`）と ukadoc の既定値（フォント名 ＭＳ ゴシック・高さ 12・色 黒・太字/斜体/下線/打ち消し線/白抜き/上下付き すべて無効）から組む。
2. The system shall 既定の見た目の構築を `crates/areka-parsers/src/balloon/parse.rs`・`model.rs` に手を入れずに行う（残り 8 キー `font.bold`／`font.italic`／`font.underline`／`font.strike`／`font.outline`／`font.shadowcolor.*`／`font.shadowstyle` の読み取りは `areka-P0-balloon-font-descript-keys` の所有。後に着地した方が配線する）。
3. The system shall 既定の見た目の各項目に「バルーン定義から与える口」を用意し、残り 8 キーが読めるようになった時点で値を差し込むだけで効く形にする（口が無いために配線側がコードを組み替える事態を作らない）。
4. The system shall 「無効表示の見た目」を第 2 の既定層として持ち、`\f[disable]`・`\f[キー,disable]` の戻し先にする。既存の予約 `FontDisableSeam`（`crates/areka-emo-text/src/draw.rs`）はこの層の実体へ置き換え、フィールド 0 個の予約型を残さない。
5. The system shall 無効表示の見た目を、ukadoc の定め「`disable.font.color` のみバルーンの画像色とミックスした色、ほかは `font.` 定義群と同じ」（descript_balloon の `disable.font.(フォント定義),(指定)`）に従って組む——色以外の項目は既定の見た目と同じ値。
6. While バルーン定義に `disable.font.color` が無い（現時点では読めない）とき, the system shall 無効表示の色を「既定の文字色をバルーンの背景色の側へ寄せた色」として導く。混ぜ方は 1 か所の定数で定め（areka の裁量として §8 に登記・ukadoc の shell 側 `menu.disable.font.color` の式「(background 画像の 0,0 の色 + 文字色 × 2) / 3」を輸入する）、利用者には「通常より薄い同系色」として見える。
7. The system shall 無効表示の見た目にも「バルーン定義から与える口」を用意する（`disable.font.*` が読めるようになったとき、キーごとに上書きできる）。
8. The system shall バルーン定義の値が既定の見た目・無効表示の見た目に反映されるかを、バルーン定義の有無 × 各項目で決定論テストに固定する。

### Requirement 5: 真偽値で切り替える 4 項目——`bold`・`italic`・`underline`・`strike`

**Objective:** As a 既存ゴーストのスクリプト作者, I want `\f[bold,1]` など 4 項目の on/off が ukadoc の 6 値で効くこと, so that SSP 向けの台詞がそのまま太字・斜体・下線・打ち消し線になる

#### Acceptance Criteria
1. When `\f[bold,値]`・`\f[italic,値]`・`\f[underline,値]`・`\f[strike,値]` の値が `true` または `1` のとき, the system shall 以降の文字に当該の装飾を DirectWrite の範囲指定（`SetFontWeight`／`SetFontStyle`／`SetUnderline`／`SetStrikethrough`）で付ける（ukadoc: 「パラメータに true または 1 を指定すると太字」「…イタリック(斜体)」「…下線を引きます」「…打ち消し線」）。
2. When 値が `false` または `0` のとき, the system shall 以降の文字から当該の装飾を外す（ukadoc: 「パラメータに false または 0 を指定すると無効」）。
3. When 値が `default` のとき, the system shall 当該の項目だけを既定の見た目（Requirement 4）へ戻す（ukadoc: 「default を指定するとバルーン設定の標準に戻る」）。他の項目は変えない。
4. When 値が `disable` のとき, the system shall 当該の項目だけを無効表示の見た目（Requirement 4）へ合わせる（ukadoc: 「disable を指定すると無効表示と同じ設定になる」）。他の項目は変えない。
5. If 値が上記 6 値（`true`／`1`／`false`／`0`／`default`／`disable`）のいずれでもない、または値が無いとき, the system shall 当該の項目を変えず `warn` の記録（キーと値を含む）を残す。
6. The system shall 太字・斜体を「フォントに太字・斜体の書体があればそれを使い、無ければ文字描画基盤の合成に委ねる」形で描き、書体が無いことを致命扱いしない（ukadoc: 「フォントが対応していない場合、太字にならない」「…斜体にならない」）。
7. The system shall 下線と打ち消し線の位置（横書きでは文字の下／中央、縦書きではどちらの側か）を DirectWrite の既定に委ね、areka 側で線を描き分けない。縦書きの期待値は完了仕様 `areka-P0-balloon-vertical-canon` の裁定（下線は**列の右側**）で、実測の結果を §8 に登記する（一致すれば「DirectWrite の既定で裁定を満たす」、食い違えば「DirectWrite の既定を採り裁定は期待値として残す」の 1 行）。裁定そのものは再審議しない。
8. The system shall 4 項目を組み合わせて同時に付けられる（例: 太字かつ下線）。
9. When `\f[outline,値]` が届いたとき, the system shall 値を 6 値の規則（5.1〜5.5）で解釈して装飾状態に保持し、表示は変えず、`warn` の記録を 1 台詞につき 1 度残す（白抜きは DirectWrite の範囲指定で表せず、自前の描画器を要するため本仕様では実装しない・開発者裁定 2026-09-11）。既存の予約名 `outline`（`canvas.rs` の `RESERVED_EFFECT_OUTLINE`）は M2 予約のまま残す。

### Requirement 6: 上付き・下付き——`sub`・`sup`（語彙のみ）

**Objective:** As a 後続の実装者, I want `\f[sub]`／`\f[sup]` が捨てられずに状態として届いていること, so that 将来 DirectWrite の標準機能の範囲で表せる手段が見つかったときに受理と解釈を作り直さずに済む

#### Acceptance Criteria
1. When `\f[sub,値]`／`\f[sup,値]` が届いたとき, the system shall 値を Requirement 5 と同じ 6 値の規則で解釈して装飾状態に保持し、表示（大きさ・位置・送り幅）は変えない（ukadoc: 「下付き文字」「上付き文字」。基線のずらしは DirectWrite の範囲指定で表せず、自前の描画器か `IDWriteInlineObject` の自前実装を要するため本仕様では実装しない・開発者裁定 2026-09-11）。
2. When `sub` と `sup` の両方が有効になったとき, the system shall 後から指定した方を採り、先の方を自動的に外す（状態としての排他だけを定める）。
3. The system shall `sub`・`sup` が有効な間に文字が追記されたとき、`warn` の記録を 1 台詞につき 1 度残す（利用者が「効かない」原因をログから追える）。
4. The system shall `sub`・`sup` の状態を「戻す操作」（Requirement 10）の対象に含める。

### Requirement 7: 文字の大きさ——`height`

**Objective:** As a 既存ゴーストのスクリプト作者, I want `\f[height,数値]` の絶対・相対・百分率の指定が効くこと, so that 強調や小声の表現が台詞のとおりに見える

#### Acceptance Criteria
1. When `\f[height,N]`（`N` は正の数値）のとき, the system shall 以降の文字の em の大きさを `N` image px にする（ukadoc: 「テキストを指定した文字サイズに変更する」・記述例「`\f[height,15]` 15pixel で表示」）。単位空間はバルーン定義 `font.height` と同じ image px で、表示倍率（DPI）への追従は既存の拡大縮小経路に委ねる。
2. When `\f[height,+N]`／`\f[height,-N]` のとき, the system shall **そのとき効いている大きさ**に `N` を加減した大きさにする（ukadoc: 「+や-による相対的な変更が可能」・記述例「`\f[height,+3]` 3pixel 大きくする」）。相対指定は重ねて効く。
3. When `\f[height,N%]` のとき, the system shall **既定の見た目の大きさ**の `N`% にする（ukadoc 記述例: 「`\f[height,200%]` デフォルトサイズの 200% で表示」）。
4. When `\f[height,default]` のとき, the system shall 大きさだけを既定の見た目へ戻す（ukadoc: 「default を指定するとバルーン設定の標準に戻る」）。
5. When `\f[height,disable]` のとき, the system shall 大きさだけを無効表示の見た目へ合わせる（`disable.font.height` が定義可能である以上、`disable` を受ける。areka の裁量として §8 に登記）。
6. If 結果の大きさが正の有限値にならない（0 以下・非数・相対指定で 0 以下へ落ちる）とき, the system shall 大きさを変えず `warn` の記録を残す。
7. When 値がスタイルシートの大きさキーワード（`xx-small`〜`xx-large`・`larger`・`smaller` 等）のとき, the system shall 値を語彙として受理しつつ表示を変えず、`warn` の記録を 1 度だけ残す（ukadoc「スタイルシートのサイズ指定も可能」は語彙のみ。実導出は areka の裁量で見送り §8 に登記）。
8. The system shall 行送りを `crates/areka-emo-text/src/state.rs` の `TextLayerConfig::line_pitch` の **1 点**を通して求め、`\f[height]` のために別の式・別の係数を持ち込まない。
9. The system shall 1 行の行送りに用いる文字の高さを「その行に置かれた文字のうち最も大きい em の大きさ」とし、文字の無い行（改行だけの行）はそのとき効いている大きさを用いる（areka の裁量として §8 に登記）。
10. The system shall 大きさの変わった文字の送り幅を、その大きさで計測して折返しと当たり判定に反映する。

### Requirement 8: 文字色——`color`

**Objective:** As a 既存ゴーストのスクリプト作者, I want `\f[color,色指定]` の各書式が効くこと, so that 台詞の一部を色分けできる

#### Acceptance Criteria
1. When `\f[color,R,G,B]`（各 0〜255 の 10 進数）のとき, the system shall 以降の文字色をその色にする（ukadoc「色指定」: 「赤・緑・青の三原色の明度をそれぞれ 0〜255 の 10 進数値で表現し、カンマで区切る」）。
2. When `\f[color,R%,G%,B%]`（各 0〜100%）のとき, the system shall 百分率を 0〜255 へ写して文字色にする（ukadoc: 「…0〜100% の百分率で表現し、カンマで区切る」）。
3. When `\f[color,#RRGGBB]` または `\f[color,#RGB]` のとき, the system shall 16 進表記を文字色にする（ukadoc: 「『#』に続けて 16 進数で表現し、つなげる」・3 桁形と 6 桁形）。
4. When `\f[color,色名]`（`red`・`white`・`black` のような小文字の色名）のとき, the system shall HTML/CSS の色名表から色を引いて文字色にする（ukadoc: 「色名を表すキーワード(全て小文字)で指定可能」「基本は HTML・CSS の色の表現と似ています」）。
5. When `\f[color,default]` または `\f[color,default.plain]` のとき, the system shall 文字色だけを既定の見た目へ戻す（ukadoc: 「指定した文字をその文字のデフォルト色に戻す」）。
6. When `\f[color,disable]` のとき, the system shall 文字色だけを無効表示の色にする（ukadoc: 「無効表示用の色に設定できる」）。
7. When `\f[color,default.cursor]`／`\f[color,default.cursornotselect]` のとき, the system shall 文字色を、バルーン定義から既に読めている選択肢の文字色（`cursor.font.color.*`・完了仕様 `areka-P0-choice-render` が解決済み）にする（ukadoc: 選択肢のデフォルト文字色へ変更）。
8. When `\f[color,default.anchor]`／`default.anchornotselect`／`default.anchorvisited` のとき, the system shall アンカーの色定義がまだ存在しないため `default` と同じ扱いにし、`warn` の記録を 1 度だけ残す（消費者は `areka-P0-anchor-tag-canon` が足す・相互登記）。
9. If 色指定が上記のいずれの書式でもない、成分が範囲外、または成分数が足りないとき, the system shall 文字色を変えず `warn` の記録（キーと値を含む）を残す。
10. The system shall 色の解析を `\f[color]` 専用にせず、同じ「色指定」の書式を使う後続の項目（影の色・選択肢マーカーの色・アンカーの色）が再利用できる 1 か所に置く。

### Requirement 9: フォント名——`name`

**Objective:** As a 既存ゴーストのスクリプト作者, I want `\f[name,フォント名,...]` で優先順に書いたフォントのうち使えるものが選ばれること, so that 環境ごとにフォントの有無が違っても意図に近い見た目になる

#### Acceptance Criteria
1. When `\f[name,名前]` のとき, the system shall 以降の文字を、その名前でインストール済みのフォントで描く（ukadoc: 「テキストフォントを指定したフォント名に変更する」）。
2. When `\f[name,名前1,名前2,...]` のとき, the system shall 書いた順を優先度として、最初に見つかったインストール済みフォントを採る（ukadoc: 「カンマ区切りでフォント名を複数指定可で、書いた順＝優先度順で自動的にインストールされている中から選択する」）。
3. When 候補にフォントファイル名（`.ttf`／`.otf`／`.ttc`）が含まれるとき, the system shall その候補を読み込まずに `warn` の記録（1 台詞につき 1 度）を残して読み飛ばし、次の候補へ進む。フォントファイルの読み込み（ukadoc: 「[実行したゴーストのフォルダ]/ghost/master 以下や現在のバルーンのフォルダに置いたフォントファイルも指定可能」・記述例「`\f[name,メイリオ,meiryo.ttf]`」）は**実装しない**（開発者裁定 2026-09-11・過剰。インストール済みフォントの指定だけで足りる）。台帳では `name` を「縮退（フォントファイルは読み飛ばし）」の注記付きで扱い、§8 に areka の裁量として登記する。
4. When 候補がすべて見つからないとき, the system shall 既定の見た目のフォントへ戻し、`warn` の記録（試した候補を含む）を 1 度だけ残す（ukadoc: 「指定したフォント候補が全てない場合は、バルーン設定の標準に戻る」）。
5. When `\f[name,default]` のとき, the system shall フォントだけを既定の見た目へ戻す。
6. When `\f[name,disable]` のとき, the system shall フォントだけを無効表示の見た目に合わせる（ukadoc: 「disable を指定した場合、無効表示と同じフォントに設定される」）。
7. The system shall フォントの探索結果を台詞の再生中は再利用し、同じ候補列を文字ごとに探索し直さない。
8. The system shall バルーン定義 `font.name` のカンマ区切り候補列（現在は先頭だけを採り、残りを `ResolvedFont::fallback_chain` に保持して未消費）にも同じ「優先順に最初のインストール済みを採る」規則を適用する（既定の見た目の側でも候補列が効く）。

### Requirement 10: 一括の戻し——`\f[default]`・`\f[disable]` と「戻す操作」の契約

**Objective:** As a 後続の仕様（`\x`・選択肢マーカー装飾）の実装者, I want 「`\f` の状態の何が・いつ戻るか」が 1 か所で定まっていること, so that クリック待ちや選択肢の装飾がそれをそのまま呼べる

#### Acceptance Criteria
1. When `\f[default]` のとき, the system shall 現在のスコープの装飾状態の**全項目**を既定の見た目へ戻す（ukadoc: 「全てのバルーン属性をデフォルトに戻す」）。
2. When `\f[disable]` のとき, the system shall 現在のスコープの装飾状態の**全項目**を無効表示の見た目に合わせる（ukadoc: 「全てのバルーン属性を無効なテキスト表示に戻す」）。
3. The system shall 「戻す操作」（全項目を既定へ戻す）を、`\f[default]`・台詞の開始（Requirement 3.8）・後続の `\x` が**同じ 1 つの操作**として呼べる形で提供する（それぞれが別々の戻し方を持たない）。
4. The system shall 「戻す操作」の対象を「装飾状態の全項目」と定め、本仕様の 10 項目に加えて、後続仕様が登記する項目（寄せ 2・影 3・選択肢マーカーの実行時上書き）も同じ操作で戻る形にする（項目を足すだけで戻しに含まれる。列挙の更新漏れが起きない構造）。
5. The system shall 「戻す操作」がスコープ単位であることを定める（`\x` が「スコープが `\0` へ戻る」と同時に呼ぶ場合、全スコープを戻す呼び方も 1 回で行える）。
6. The system shall 戻す操作の対象と時期（`\f[default]`・台詞の開始・`\x` は戻す／`\c`・`\n`・`\_l`・`\x[noclear]` は戻さない）を `doc/COMPAT_ARCHITECTURE.md` §8 に 1 行で登記し、`areka-P0-balloon-lifecycle-events` 項目 9 と `areka-P0-choice-marker-styling` の brief からたどれる形にする。
7. When 戻す操作が呼ばれたとき, the system shall 既に表示済みの文字の見た目を変えない（戻しは以降の文字にだけ効く）。

### Requirement 11: 送り幅の計測と折返しの整合

**Objective:** As a 利用者, I want 装飾で文字の幅が変わっても折返し位置と当たり判定がずれないこと, so that 太字や大きい文字を含む行が欠けたり重なったりしない

#### Acceptance Criteria
1. The system shall 各文字の送り幅を、その文字に効く装飾（フォント名・大きさ・太字・斜体）で計測する（現行の「文字 1 つ→送り幅 1 つ」の記憶は装飾込みの鍵へ改める）。
2. The system shall 折返し（折返し基準 soft／描画範囲 hard の二段構え・完了仕様 `areka-P0-emo-text-line-height-canon` で確定）を装飾込みの送り幅で判定し、二段構えの意味論そのものは変えない。
3. The system shall 計測に使う書式と描画に使う書式を同じ経路で作り、装飾を含めても「計測した送り幅＝描画した送り幅」の一致が崩れない（既存の一致テストを装飾ありの行へ拡張する）。
4. The system shall 行の描画結果の再利用（行ごとの記憶）を、行の文字列だけでなく行内の装飾が同じときに限る（装飾だけが変わった行を古い見た目のまま再利用しない）。
5. The system shall 選択肢の当たり判定の範囲を、装飾込みの送り幅から導く（太字や大きい文字の選択肢がクリックできない・隣とずれる事態を作らない）。

### Requirement 12: 縦書きでの写像

**Objective:** As a 縦書きバルーンの利用者, I want 装飾が縦書きでも意味を保つこと, so that 横書きと同じ台詞が縦書きでも装飾のとおりに見える

#### Acceptance Criteria
1. The system shall `name`・`height`・`color`・`bold`・`italic` を書字方向によらず同じ意味で効かせる。
2. The system shall 縦書きの下線・打ち消し線の位置を DirectWrite の既定に委ね（Requirement 5.7）、実測した位置と完了仕様 `areka-P0-balloon-vertical-canon` の裁定（下線は列の右側）との照合結果を §8 に登記する。
3. The system shall `vertical_rl` と `vertical_lr` で装飾の見た目を変えない（列送りの向きが違うだけで、列の中での装飾の位置は同じ）。
4. The system shall 縦書きの装飾の位置（下線・打ち消し線の側と位置）を、オフスクリーンの描画結果の読み戻しで観測する決定論テストに固定する（述語は「DirectWrite の既定で今日引かれた側」＝値が変われば赤になる形。裁定との一致を要求する述語にはしない）。

### Requirement 13: 失敗時の扱いと記録

**Objective:** As a 運用者, I want 装飾の不正な指定やフォントの不在が黙って通らず記録に残ること, so that 「装飾が効かない」原因をログから追える

#### Acceptance Criteria
1. If `\f` の値が解釈できないとき, the system shall 表示を変えず `warn` の記録（キー・値・スコープを含む）を残し、台詞の再生を中断しない。
2. If フォントの候補が 1 つも見つからないとき, the system shall 既定へ戻したうえで `warn` の記録（試した候補を含む）を残す。
3. If 描画の基盤（DirectWrite）で装飾の適用に失敗したとき, the system shall `error` の記録と `Err` で扱い、panic しない（既存の log-first の規律を保つ）。
4. The system shall 同じ不正な値が台詞の中で繰り返されても、記録を 1 台詞につき同じ値ごとに 1 度に留める（記録があふれない）。
5. The system shall 失敗を記録なしで飲み込む経路を持たない（記録なしの失敗経路の禁止）。

### Requirement 14: 既存の表示結果の不変

**Objective:** As a M1 適合の保守者, I want `\f` を使わない台本の表示が 1 画素も変わらないこと, so that emo2 の適合検証が本仕様で崩れない

#### Acceptance Criteria
1. The system shall `\f` を含まない台本について、文字の位置・送り幅・行送り・折返し・描画結果を本仕様の前後で同一に保つ（既存のオラクル比較テスト・PNG 比較テストが変更なしで緑）。唯一の例外は Requirement 9.8 による既定フォントの候補列の解決で、バルーン定義 `font.name` がカンマ区切りの複数候補を持ち、かつ先頭の候補がインストールされていない環境では、採るフォントが変わる（正典どおりの変化であり退行ではない）。適合対象 emo2 のバルーン定義は単一名 `Yu Gothic UI`（2026-09-11 実測）なので、この例外は emo2 の適合検証に及ばない。
2. The system shall 既定の見た目だけが効いている行の描画を、装飾の配管を通しても従来と同じ描画経路・同じ結果にする（装飾の run が 1 つだけの行は従来の 1 行描画と同一）。
3. The system shall emo2 の適合検証（`areka-P0-emo2-conformance-e2e` の 20 項目・完了済み）の観測対象を変えない。
4. The system shall 台本の再生時間（各命令の時刻と長さ）を `\f` の有無で変えない（`\f` は再生時間 0）。

### Requirement 15: 決定論テスト

**Objective:** As a 開発者, I want 3 書字方向 × 12 項目 × 戻しの経路が機械で検証されること, so that 装飾の退行が全緑のまま通り抜けない

#### Acceptance Criteria
1. The system shall 解読（`\f` 43 形の受理・引数の保持・不正形の扱い）を `decode.rs` の兄弟テストで固定する。
2. The system shall 台本の組み立て（`\f` が再生時間 0 で並び順を保つこと・`\f` なし台本の不変）を `compile.rs` の兄弟テストで固定する。
3. The system shall 装飾状態の更新（12 項目 × 6 値〔語彙のみの 3 項目を含む〕・`+N`/`-N`/`N%`・色の各書式・スコープ独立・戻す操作・台詞開始の戻し・`\c`/`\n`/`\_l` で戻らないこと）を、描画なしの純粋な層の決定論テストで固定する。
4. The system shall 3 書字方向 × 7 項目の描画結果を、オフスクリーンの描画結果の読み戻しで観測するテストで固定する（太字・斜体・下線・打ち消し線・色・大きさ・フォント名の切り替えが、それぞれ「効いていない場合と画素が違う」ことと、下線・打ち消し線の側と位置が今日の DirectWrite の既定と同じであることを述語にする）。語彙のみの 3 項目（`sub`・`sup`・`outline`）は「有効にしても画素が変わらない」ことを述語にする。
5. The system shall 装飾込みの送り幅の計測と描画の一致・折返し位置・当たり判定の範囲を決定論テストで固定する。
6. The system shall 既定の見た目・無効表示の見た目が「バルーン定義の有無 × 各項目」で正しく組まれることを決定論テストで固定する。
7. The system shall テストの各述語が「過去に壊れていた形」を再現すると赤になることを、少なくとも各要件 1 件の較正で確かめる（何が起きても緑になるテストを作らない）。
8. The system shall すべてのテストを `cargo test` の常時実行に含め、x86 や実機の起動を要求しない。

### Requirement 16: 文書・台帳・予約名の追随

**Objective:** As a 後続仕様の担当者, I want 本仕様の裁定と着地が正典文書と台帳に写っていること, so that 次の仕様が古い記述を前提にしない

#### Acceptance Criteria
1. The system shall areka の裁量で決めた各点（相対指定の基準・百分率の基準・スタイルシートのキーワードの見送り・フォントファイルの読み込みの見送り・`height,disable`・`sub`／`sup`／`outline` の見送り（語彙のみ）・縦書きの下線・打ち消し線の位置を DirectWrite の既定に委ねる裁定と実測結果・無効表示の色の混ぜ方・行の高さの決め方・戻す操作の対象と時期・台詞開始の戻し）を `doc/COMPAT_ARCHITECTURE.md` §8 に、項目・裁量・根拠（ukadoc の逐語引用と URL）・出典 spec の 4 欄で登記する。
2. The system shall `doc/COMPAT_ARCHITECTURE.md` §8 の「`\f[align]`／`\f[valign]`／下線の縦書き写像」の行にある「areka は align／valign を全書字方向でまだ実装していない…本登記は現在の表示結果を変えない」の注記を、下線が実装済みであること・align／valign の追跡先が `areka-P0-text-align-shadow-canon` であることに合わせて改訂する。同じ表で本仕様を「`\f` 核 17 項目の所有者」として追跡先に挙げている箇所（同行の追跡先欄と、「`\_l` の縦書き座標系の正典写像」の行にある疑義 SC8 の追跡先）も、2026-09-11 の 3 分割後の所有（本仕様＝基盤＋フォント系 10＋一括の戻し 2・SC8 と寄せは `areka-P0-text-align-shadow-canon`）に合わせて改訂する。
3. The system shall `doc/ukadoc-coverage/ledger/sakura-script.toml` の本仕様 12 項目の `status` を実装後の状態（7 項目＋戻し 2 項目は `implemented`〔`name` はフォントファイル読み飛ばし・`height` はスタイルシートのキーワードのみ注記付き〕・`sub`／`sup`／`outline` は `vocabulary-only`）へ更新し、`doc/ukadoc-coverage/ledger/assets.toml` の `disable.font.(フォント定義),(指定)` の行を無効表示の層が実体化したことに合わせて更新する。
4. The system shall `crates/areka-emo-text/src/canvas.rs`・分割後の `draw.rs` 群の予約名の注記（`TextEffects`・`FontDisableSeam`・`RESERVED_EFFECT_*`・「M1 では実挙動を一切実装しない」）を、`disable` が実体化したこと・`outline`（および `sub`／`sup`）は本仕様が語彙のみで見送り M2 予約に加えたこと・`shadow` は `areka-P0-text-align-shadow-canon`・`multicolor`／`rotation` は M2 予約のままであることに合わせて改訂する。あわせて、予約型が空であることを固定している既存テスト（`draw_format_metrics_tests.rs` の `decoration_and_disable_seams_are_type_only`＝`size_of` が 0 であることの検査）を、実体化した内容を述べる述語へ改訂する。この改訂は実体化の段階で行い、Requirement 1.5 の「分割の段階では既存テストを変更しない」とは段階が異なる（分割と実体化を別の作業に分ける理由）。
5. The system shall 分割後の新ファイルと接続を steering `structure.md` の該当節へ反映する。
6. The system shall 本仕様が消費者を足さなかった語彙（寄せ 2・影 3・`cursor*`・`anchor*`・`default.anchor*`・スタイルシートのキーワード）の追跡先を、それぞれの所有仕様の brief に 1 行ずつ相互登記する。`sub`・`sup`・`outline` は所有仕様が無いため、steering `roadmap.md` の M2 予約（`TextEffects` 予約名 `rotation`／`multicolor` の並び）に「DirectWrite の標準機能で表せる手段が見つかるまで語彙のみ」と 1 行で登記し、台帳の `status` は `vocabulary-only` にする。
7. The system shall 文書の主張（file:line・行数・件数）を書く前に実測で裏取りし、行番号でなく「何の定義行か」で指す。

---

## 付録 A: 本仕様が輸入した ukadoc の逐語（2026-09-11 取得・ukadoc MCP スナップショット＋ライブページ）

| 項目 | 逐語 | URL |
|---|---|---|
| `\f[name,フォント名]` | 「テキストフォントを指定したフォント名に変更する。default を指定した場合、または指定したフォント候補が全てない場合は、バルーン設定の標準(SSP 設定でバルーンフォント上書きしている場合はそちら)に戻る。disable を指定した場合、無効表示と同じフォントに設定される。フォント名には、[実行したゴーストのフォルダ]/ghost/master 以下や現在のバルーンのフォルダに置いたフォントファイルも指定可能。カンマ区切りでフォント名を複数指定可で、書いた順＝優先度順で自動的にインストールされている中から選択する(SSP のみ)。」 | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bname_2c_30d5_30a9_30f3_30c8_540d_5d:1 |
| `\f[height,数値]` | 「これテキストを指定した文字サイズに変更する。+や-による相対的な変更が可能。スタイルシートのサイズ指定も可能。default を指定するとバルーン設定の標準に戻る。」記述例「`\f[height,15]` 15pixel で表示。`\f[height,+3]` 3pixel 大きくする。`\f[height,200%]` デフォルトサイズの 200% で表示。`\f[height,default]` 元のサイズに戻す。」 | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bheight_2c_6570_5024_5d:1 |
| `\f[color,色指定]` | 「文字色を変更。指定方法については※下記参照。」記述例「`\f[color,red]` 赤色で表示。`\f[color,100,150,200]` 色指定で表示。`\f[color,default]` バルーンのデフォルト色に戻す。」 | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bcolor_2c_8272_6307_5b9a_5d:1 |
| 色指定（※） | 「基本は HTML・CSS の色の表現と似ています。」／「赤・緑・青の三原色の明度をそれぞれ 0〜255 の 10 進数値で表現し、カンマで区切る。」／「…0〜100% の百分率で表現し、カンマで区切る。」／「『#』に続けて 16 進数で表現し、つなげる」（3 桁形と 6 桁形）／「red、white、black というような色名を表すキーワード(全て小文字)で指定可能」／`default`「指定した文字をその文字のデフォルト色に戻す」／`disable`「無効表示用の色に設定できる」／`default.anchor`・`default.anchornotselect`・`default.anchorvisited`・`default.cursor`・`default.cursornotselect`・`default.plain` | 同ページ `\f[color]` 直下の注（ライブページ 2026-09-11） |
| `\f[bold,パラメータ]` | 「パラメータに true または 1 を指定すると太字。パラメータに false または 0 を指定すると無効。default を指定するとバルーン設定の標準に戻る。disable を指定すると無効表示と同じ設定になる。フォントが対応していない場合、太字にならない。」 | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1 |
| `\f[italic,パラメータ]` | 「…イタリック(斜体)…フォントが対応していない場合、斜体にならない。」（他は bold と同文） | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bitalic_2c_30d1_30e9_30e1_30fc_30bf_5d:1 |
| `\f[underline,パラメータ]` | 「パラメータに true または 1 を指定すると下線を引きます。」（他は bold と同文） | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bunderline_2c_30d1_30e9_30e1_30fc_30bf_5d:1 |
| `\f[strike,パラメータ]` | 「…打ち消し線…」（他は bold と同文） | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bstrike_2c_30d1_30e9_30e1_30fc_30bf_5d:1 |
| `\f[sub,パラメータ]`／`\f[sup,パラメータ]` | 「…下付き文字…」「…上付き文字…」（他は bold と同文） | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bsub_2c_30d1_30e9_30e1_30fc_30bf_5d:1 ／ …#_5cf_5bsup_2c_30d1_30e9_30e1_30fc_30bf_5d:1 |
| `\f[outline,パラメータ]` | 「パラメータに true または 1 を指定すると白抜き。」（他は bold と同文）記述例「`\f[height,30]\f[outline,1]` 白抜きにする。」 | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5boutline_2c_30d1_30e9_30e1_30fc_30bf_5d:1 |
| `\f[default]` | 「全てのバルーン属性をデフォルトに戻す。」記述例「`\f[shadowcolor,#6699cc]` 色々な `\f[bold,1]` 装飾を `\f[underline,1]` した `\f[height,20]` 文字を `\f[default]` 全てバルーンのデフォルト設定に戻す。」 | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bdefault_5d:1 |
| `\f[disable]` | 「全てのバルーン属性を無効なテキスト表示に戻す。」（2.5.51） | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bdisable_5d:1 |
| `disable.font.(フォント定義),(指定)` | 「操作無効を意味する薄い色の文字を定義する。`\f[disable]`(または `\f[color,disable]` など)で活用する。『フォント定義』には、font. ではじまる各定義をそのまま定義できる。定義方法も同じ。例：disable.font.color。disable.font.color のみバルーンの画像色とミックスした色、ほかは font. 定義群と同じ。」（2.5.51） | https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#disable.font._28_30d5_30a9_30f3_30c8_5b9a_7fa9_29_2c_28_6307_5b9a_29:1 |
| `\x` | 「バルーンをクリック待ちにする。クリック後、スコープはリセットされ `\0` となり、`\f`(文字装飾)系のさくらスクリプトの効果も解除される。`\e` で解除されるさくらスクリプトは継続。」 | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cx:1 |
| `\x[noclear]` | 「バルーンを一時クリック待ちにする。クリック後、バルーンの内容とスコープは保持される。`\x` と違い、`\f`(文字装飾)系のさくらスクリプトの効果は残る。」 | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cx_5bnoclear_5d:1 |
| `menu.disable.font.color.r`（shell・無効表示色の式の輸入元） | 「選択不可文字色赤(0〜255)　(background 画像の 0,0 の色 + menu.background.font.color * 2 ) / 3」 | https://ssp.shillest.net/ukadoc/manual/descript_shell.html#menu.disable.font.color.r:1 |

## 付録 B: 2026-09-11 に実測した現状の所在（行番号は当日値・「何の定義行か」を正とする）

- 解読の腕が無い場所: `crates/areka-parsers/src/sakura/decode.rs` の `fn decode_tag`（:201）——`match word.as_str()` の末尾 `_ => decode_passthrough_tag(word, args)`（→ `Instruction::Raw`・:321）。
- 捨てられる場所: `crates/areka-sakura/src/compile.rs` の catch-all `other => tracing::debug!(instruction = ?other, "M-boot 外タグを無視")`（:202-204）。
- 命令の型: `crates/areka-parsers/src/sakura/model.rs` の `pub enum Instruction`（:25）／`crates/dola/src/cue/command.rs` の `pub enum CueCommand`（:130・`Custom { command, params }` が `\!` の汎用キャリア）。
- 文字ごとの属性が無い 3 層: `crates/areka-emo-text/src/state.rs` の `TextItem::Glyph { ch }`（:104-106）／`layout.rs` の `pub struct PositionedGlyph { ch, inline_pos, advance }`（:171）／`canvas.rs` の `pub struct GlyphRunContent { glyphs, size }`（:150）。
- 空の予約: `canvas.rs` の `#[non_exhaustive] pub struct TextEffects {}`（:143）と `RESERVED_EFFECT_OUTLINE`／`MULTICOLOR`／`SHADOW`／`ROTATION`（:45/:47/:49/:51）・モジュール doc「M1 では実挙動を一切実装しない」（:38）／`draw.rs` の `pub struct FontDisableSeam {}`（:150）・`RESERVED_KEY_DISABLE_FONT_PREFIX`（:136）・`ResolvedFont { name, fallback_chain, height, color, effects, disable }`（:158-172）。
- 太さ・斜体の固定: `draw.rs` の `fn try_create_format` が `DWRITE_FONT_WEIGHT_NORMAL`／`DWRITE_FONT_STYLE_NORMAL` を渡す行（:348-349）。
- 文字単位の計測の記憶: `draw.rs` の `DWriteMetrics { cache: RefCell<HashMap<char, f32>> }`（:385）。行の描画結果の記憶: `LineLayoutStore`（key＝行 index・内容文字列で不変判定・:582）。
- 行送りの 1 点: `state.rs` の `TextLayerConfig::line_pitch(font_height) = font_height + line_gap`（:78）。
- 既定の見た目の 5 キー: `crates/areka-parsers/src/balloon/parse.rs` の `font.color.r/g/b`・`font.name`・`font.height` の引き（:106-117）／`model.rs` の `impl Font { name(), height(), color() }`（:379-389）・`cursor.font.color.*`（:482）。
- 文字範囲への効果適用の先例: `crates/areka-emo-text/src/viewbox_draw.rs` の `SetDrawingEffect`（reset :358・hover 適用 :388 付近）。repo 内で `SetUnderline`／`SetStrikethrough`／`SetFontWeight`／`SetFontStyle`／`SetFontSize` を呼ぶ箇所は areka 系 crate に 0 件（wintf の `typewriter_draw.rs` は仕様外の既存資産）。
- 1,000 行の見張り: `crates/log-capture-kit/tests/file_length_guard_test.rs`（`OVER_LIMIT_ALLOWED` 11 件・`OVER_LIMIT_ALLOWED_COUNT = 11`）。`crates/areka-emo-text/src/` の実測: `draw.rs` 988・`layout.rs` 955・`actor.rs` 952・`region.rs` 951・`viewbox.rs` 846・`viewbox_draw.rs` 852・`canvas.rs` 738・`state.rs` 528。
- 台帳: `doc/ukadoc-coverage/ledger/sakura-script.toml` の本仕様 12 項目（`owner = "areka-P0-text-decoration-canon"`・`status` は `absent` 11 件＋`underline` が `vocabulary-only`）／`assets.toml` の `disable.font.(フォント定義),(指定)`（`vocabulary-only`・owner 本仕様）。
- 正典文書: `doc/COMPAT_ARCHITECTURE.md` §8 の「`\f[align]`／`\f[valign]`／下線の縦書き写像」の行と「`font.height` の意味・行送りの式・行間の既定」の行。
