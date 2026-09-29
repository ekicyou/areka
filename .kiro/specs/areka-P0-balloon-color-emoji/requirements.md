# Requirements Document

> 本文の実測は **2026-09-29・本ブランチ**（main `65e28545`＝本 spec の起票のコミット。ソースは brief の実測 `c3876110` から不変）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 要件 10 の裁定は、brief が要件へ委ねた点と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: ゴーストの作者と、そのゴーストを使う利用者。台詞に絵文字（😀・👍🏻・🇯🇵・👨‍👩‍👧）を書いても、areka のバルーンでは色が出ず（文字色一色の輪郭）、複数の符号でできた絵文字は 1 文字ずつ出す演出の途中で部品が見え、幅の見積もりが描かれる幅と合わずに後ろの折り返し・選択肢の当たり範囲がずれる。

**今の状態**: バルーン文字の描画は色つきフォントを使わない設定で 1 か所から呼ばれている。文字の単位は Rust の `char`（Unicode スカラー値）で、出す間隔・再生時間・幅の計測・分かち書きの塊の数え方・折り返しの切れ目・選択肢の範囲がすべてその単位で数えられている。完了 `areka-P0-emo-text-layer` の設計は「書記素クラスタ結合は M2 検討事項」と先送りを記録している。絵文字を描くテストは 0 件。

**何を変えるか**: バルーンで、指定フォントに無い絵文字が代替フォントの**色つき**の字形で描かれるようにする。文字の単位を**書記素クラスタ**（人が 1 文字と見る単位）へ替え、演出・再生時間・計測・折り返し・範囲のすべてがクラスタで数えるようにする。これまでの文字（日本語・英字・指定フォント自身が持つ ♥ などの記号）の見た目と挙動は変えない。

> 起票: 2026-09-29 `/kiro-discovery`。段は「前倒し」（α の機能ではないが、α の残りの枝と接触するファイルが 0 なので B6 で `file-drop` と並べる）。

## Introduction

### 誰が困っているか

- **ゴーストの作者**: 辞書に絵文字を書くと、SSP では単色ながら 1 文字として出るのに、areka では部品がばらけて出たり、後ろの行が早く折り返されたりする。
- **利用者**: 家族・国旗・肌色つきの絵文字が、台詞の途中で崩れて見える。
- **areka の開発者**: 完了 spec が先送りした「書記素クラスタ」の宿題が、絵文字の普及で表に出た。

### いま何が起きているか（2026-09-29 実測）

- **描画は色つきフォントを使わない。** 本番の描画は `crates/areka-emo-text/src/viewbox_draw.rs` の `ViewboxExecutor`（描画の相 2）が `draw_text_layout` を `D2D1_DRAW_TEXT_OPTIONS_NONE` で呼ぶ 1 か所だけ。照合用の `crates/areka-emo-text/src/draw.rs` の `DrawExecutor`（`#[cfg(test)]` 限定）も同じ `NONE`。どちらも描き先は `ID2D1DeviceContext`（`create_device_context` で作る）で、`D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT` の綴りは `crates/` に **0 件**。
- **フォントの代替は OS 任せ。** `draw.rs` の `try_create_format` と `draw_metrics.rs` の `DWriteMetrics::probe_format_for` はシステムのフォントコレクションでフォーマットを作り、`SetFontFallback`・独自のコレクションは **0 件**。だから絵文字の字形自体は代替フォント（Segoe UI Emoji）から届いており、単色で描かれている。
- **文字の単位は Rust の `char`。**
  - `state.rs`: `TextItem::Glyph { ch: char }`。`TextLayerState::apply_cue` の `Text` の腕は `text.chars().count()` を文字数とし、出す間隔を `duration / 文字数` で割り、`RevealSchedule` に 1 文字 1 時刻を積む。`Choice` の腕も同じで、`ChoiceSpan::glyph_range` は文字の通し番号。`state_decoration.rs` の `push_current_style` は 1 文字 1 `StyleId`。
  - `crates/areka-sakura/src/duration.rs`: `text_playback_duration` は `chars().count() × CHAR_NOMINAL_MS`（50 ms）。
  - `layout.rs`: `GlyphMetrics::advance(ch: char, …)`／`advance_styled`、`PositionedGlyph { ch: char, … }`。`FixedMetrics::advance` は ASCII を半角・それ以外を全角に見る。
  - `draw_metrics.rs`: `DWriteMetrics` のキャッシュの鍵は `(char, FontKey)`、`probe_advance` は 1 文字の文字列で計測用レイアウトを作りクラスタ幅を足す。
  - `segment.rs`: `segment_plan` は budoux の塊を `chunk.chars().count()` で `Segment { start, len }` へ写す（説明文に「M1 は書記素クラスタ結合なし ゆえ写像は無損失」）。
  - 折り返し: `layout.rs` の `WrapPlan::CharByChar` はどの文字の前でも切る。分かち書き（`Segmented`）の塊が長すぎるときも文字の規則へ戻る。方式は `actor.rs` が `WrapMode` から `WrapPlan` へ写して選ぶ（読むだけ）。
  - 範囲: `viewbox_draw_decoration.rs` の `style_runs` は `g.ch.len_utf16()` の積み上げで `DWRITE_TEXT_RANGE` を組み、`apply_color_ranges`／`apply_font_ranges` が使う。`viewbox_draw.rs` の `segment_text_range` は選択肢のホバーの範囲を同じく `len_utf16` と文字の中心で決める。`choice.rs` の `annotate_lines` は `glyph_range` と行の文字数の交差で当たり範囲を出す。
  - `\_l`: `TextItem::CursorMove` は文字の通し番号を消費せず、単位は px／em／lh／%（`CursorUnit`）。**文字の単位に直接は依らない**（送り幅の合計を通してだけ影響を受ける）。
- **サロゲートペア 1 つの絵文字（😀）は今でも 1 段・1 文字幅で出る。** UTF-16 の範囲計算は正しい。崩れるのは複数のスカラー値でできた絵文字（ZWJ 列・国旗・肌色・異体字セレクタ・キーキャップ）だけ。
- **テスト。** 絵文字を描くテストは **0 件**。ZWJ・U+FE0F・国旗の符号を含むテストは `areka-emo-text`・`areka-sakura`・`areka-parsers` に **0 件**。非 BMP を含むのは純粋層の 3 件だけ（`state_cue_apply_tests.rs` の `glyph_unit_is_rust_char`＝「aあ🦆」を 3 グリフと判定、`viewbox_draw_decoration_tests.rs` の `style_runs_group_consecutive_ids_and_count_utf16_units`、`duration.rs` の `counts_chars_not_bytes_or_utf16_units`）。`draw_oracle_tests.rs` の `probe_advances_match_drawn_line_cluster_advances` は「幅の数＝`chars().count()`」を前提に、1 文字の計測値と行を描いたときのクラスタ幅の一致を判定する。
- **読み戻しの足場は在る。** `surface.rs` の `TextSurface::read_back`（BGRA の密な配列）と、`viewbox_draw_test_support.rs` の画素の述語（`opaque_count`・`ink_min`・`block_axis_ink_span`）、live-diff のバイト等価（`viewbox_draw_live_diff_tests.rs`）、スクロールの blit の等価（`viewbox_draw_scroll_retain_tests.rs`）が既にある。
- **依存。** `unicode-segmentation` 1.13.3 は `Cargo.lock` に在るが、`convert_case` ← `derive_more` ← `bevy_ecs` の推移依存だけで、ワークスペースの `Cargo.toml` はどれも直接は引いていない。`crates/areka-emo-text/Cargo.toml`・`crates/areka-sakura/Cargo.toml` にも無い。
- **縦書きは 3 方式とも本番。** `writing.rs` の `WritingMode`（横・縦右→左・縦左→右）を `draw.rs` の `DirectionRecipe` が読み方向と流し方向へ写す。`SetVerticalGlyphOrientation` は使っていない。

### 正典の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [`\_u[0x0000]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_u_5b0x0000_5d:1) | 「UCS-2 コード埋め込み。」 | 絵文字を UTF-8 で直接書く経路だけを扱う。`\_u` は範囲外（下の「brief に無い事実」1）。 |
| 里々 Wiki「絵文字（Unicode文字）を表示する」概要と注意点 | 「絵文字の扱いは文字と等価です。具体的には使用しているフォントによって変わるほか、文字サイズ、文字色の変更の影響を受けます。」 | SSP は絵文字も文字色の単色で描く。areka は指定フォントに無い絵文字だけ色つきで描く＝**正典の沈黙（ukadoc はカラー絵文字に触れない）に対する areka の裁量**として記録する（要件 9）。指定フォントが持つ記号（游ゴシックの ♥）は今日どおり文字色に従う。 |
| 里々 Wiki 同ページ「\_l[x,y]」 | 「表示したい記号がそれまでの文字との位置で微調整したい時は、\_l[x,y] のさくらスクリプトを使う」 | 絵文字の後ろの `\_l` は、送り幅が描画と一致すれば正しい位置へ着く。 |

ukadoc はカラー絵文字・書記素クラスタに沈黙している。SSP の挙動を測って合わせることはしない（SSP 実測主義は取らない）。

### brief の記述を実物で引き直して改めた点

1. brief の「wintf のタイプライターは DirectWrite のクラスタ単位で動いている」は、正確には**クラスタの総数**を `GetClusterMetrics` から取るだけで、進み方は `chars().count()` 回の 1 文字 1 歩（`typewriter_layout.rs` の `convert_to_timeline`）。文字とクラスタの対応表は持っていない。手本として写せるのは「クラスタの数を DirectWrite から取る」ことまで。範囲外であることは変わらない。
2. brief の Approach 2 は `\_l` の位置を「置き換える対象」に挙げるが、`\_l` は文字の通し番号を消費せず単位も px／em／lh／% なので、単位の置き換え対象では**ない**。送り幅が描画と一致すれば自動で正しい位置へ着く（要件 3.4 で確かめる）。
3. brief の Out「`\_u[0x…]` の解釈（現状は未確認）」は実測で確定した: `areka-parsers/src/sakura/decode.rs` の `decode_tag` に `_u` の腕は無く `Instruction::Raw` へ素通りし、`areka-sakura/src/compile.rs` の受け皿が `debug!` で捨てる。**今日の areka では `\_u[…]` は何も描かない**。範囲外のまま（担当 spec は未定・棚卸で決める）。

### 要件を書く途中で見つかった、brief に無い事実

1. **選択肢の絵文字はホバーで色が変わらない。** ホバー色は範囲ごとの塗りの差し替え（`SetDrawingEffect`）で付け、色つきの字形はその塗りに従わない（brief の Out of Boundary の帰結）。選択肢の中の絵文字は、ホバー中も自分の色のまま出る。要件 4.5 で明示する。
2. **「非絵文字の画素が変わらない」の証拠は、照合用の描画との等価では立たない。** 本番と照合用の両方に同じ色つきフォントの設定を入れるので、live-diff は「同じ変更同士」の比較になる。変わっていないことの証拠は、変更前（git の `main`）の実物から取った対照、または同じテストの中で「色つきフォントの設定あり／なし」を描き比べる形で立てる（要件 6.4）。
3. **分かち書きの塊の写像は「無損失」の注釈ごと変わる。** `segment.rs` の説明文が `char` 前提を明記しているので、単位を替えたら注釈も改める。
4. **既存の 3 件の純粋テストの前提が変わる。** `glyph_unit_is_rust_char`・`counts_chars_not_bytes_or_utf16_units`・`probe_advances_match_drawn_line_cluster_advances` は、判定の数値は同じまま（「aあ🦆」は 3 クラスタ）名前と前提が古びる。陳腐化したテストは名前と前提をクラスタへ改めて残す（要件 7.6）。

### 何を変えるか

描画に色つきフォントの設定を入れ、文字の単位を `char` から書記素クラスタへ替える。クラスタは「1 文字ずつ出す」「幅を測る」「折り返す」「範囲を数える」「再生時間を数える」の全部で同じ 1 つの切り方を使う。日本語・英字・指定フォントが持つ記号は、切り方が変わっても 1 クラスタ＝1 スカラー値のままなので、見た目も挙動も変わらない。

## Boundary Context

- **In scope**:
  - バルーン文字の描画（本番の描画と、テストの照合用の描画の両方）で、指定フォントに無い絵文字を色つきの字形で描くこと。
  - `areka-emo-text` の文字の単位を書記素クラスタにすること: 出す演出（1 段ずつ出す間隔）・幅の計測（キャッシュの鍵を含む）・分かち書きの塊の数え方・折り返しの切れ目（文字ごと／分かち書きの両方式）・選択肢の当たり範囲とホバーの範囲・色とフォントの範囲。
  - `areka-sakura` の再生時間の数え方（1 クラスタ 50 ms）。
  - 上記の決定論テスト（読み戻しで色つきの画素・1 段・1 文字幅・折り返しで割れない・バイト等価の組に絵文字を足す）と、非絵文字の組の不変の証拠。
  - areka の裁量（絵文字は文字色に従わない）の記録 1 項目。
- **Out of scope**:
  - wintf のウィジェット（`draw_labels.rs`・`typewriter_draw.rs`・`typewriter_layout.rs`）: areka のバルーンは使わず、example（`mock-shell`）だけが使う。
  - Win32 標準メニュー・`MessageBoxW` の文字（OS が描く）。
  - `\_u[0x…]` の解釈（今日は何も描かない・別件）。
  - Shift_JIS のゴースト（応答に絵文字を載せられない・charset の既存仕様どおり）。
  - 絵文字を `\f[color]`／ホバー色で染めること、染めるか否かのスイッチ（要件 9・裁定 2）。
  - COLR v1（グラデーション等）の字形を出すこと（標準の設定で出る字形を受け入れる・裁定 3）。
  - 文字の影の描き方（`text-align-shadow-canon` の所有。下の申し送り）。
  - 縦書きでの絵文字の向き（DirectWrite の既定に任せる・崩れが見つかったら別件）。
  - `\_b[…,inline]`（画像を 1 文字として貼る）。
- **Adjacent expectations**:
  - 完了 `emo-text-layer`・`emo-text-viewbox`・`budoux-newline`・`text-decoration-canon` の配置・描画・範囲の仕組みに依る。完了 spec の文書は書き換えない（先送りの記録は本仕様が引き受けたことを本仕様の側に書く）。
  - **`text-align-shadow-canon` へ申し送る**: 影の複製を色つきフォントの設定のまま描くと影まで多色になる。「影の複製は色つきフォントを使わず単色の塗りで描く」こと。申し送りは同 spec の brief へ本仕様の要件確定時に 1 行足す。
  - `choice-marker-styling`・`anchor-tag-canon`・`emo-text-canon-residue` は、範囲がクラスタ単位になった後に着地する。
  - 並走の条件（brief の Constraints）: `crates/areka-emo-text/src/sink.rs`・`crates/areka/`・`crates/wintf/` には触らない。`actor.rs` は折り返しの方式を選ぶ箇所を読むだけ。`Cargo.toml` を触るのは `crates/areka-emo-text/Cargo.toml`（と必要なら `crates/areka-sakura/Cargo.toml`）の依存 1 行まで。`Cargo.lock` の変化はその crate の依存の並びだけ。`file-drop`（B6）・`network-update`（B7）と共有するソースは 0。`shell-balloon-switch`（B8）とは `sink.rs` を避ければ共有 0。

## Requirements

### Requirement 1: 指定フォントに無い絵文字は色つきで描かれる

**Objective:** As a ゴーストの作者, I want 台詞に書いた絵文字がバルーンで色つきに出ること, so that 意図した表情や記号が利用者に伝わる

#### Acceptance Criteria

1. When 台詞の文字が指定フォント（`\f[name,…]` またはバルーンの既定フォント）に無く、OS の代替フォントが色つきの字形を持つ, the areka のバルーン shall その文字を色つきの字形で描く（文字色一色の輪郭にしない）。
2. When 台詞の文字が指定フォント自身の字形を持つ（例: 游ゴシックの ♥ U+2665）, the areka のバルーン shall 今日どおりその字形を文字色で描く（色つきの代替へ回さない）。
3. The areka のバルーン shall 色つきの字形を描くとき、`\f[color]`・選択肢のホバー色・無効表示の混色を字形の色に重ねない（色つきの字形は自分の色で出る）。
4. The areka のバルーン shall 色つきの字形の見た目（平面か、グラデーションか）を OS の標準の色つきフォントの描画に任せ、自前の描き手を持たない。
5. If OS の代替フォントに色つきの字形が無い（単色の字形しか無い・または字形自体が無い）, then the areka のバルーン shall 今日どおり単色の字形（または代替の記号）を描き、失敗にしない・記録も増やさない（新しい失敗経路 0）。
6. The areka のバルーン shall 色つきの字形を、横書き・縦書き（右→左・左→右）の 3 方式のどれでも描く（縦書きでの向きは OS の既定に任せ、本仕様では判定しない）。

### Requirement 2: 文字の単位は書記素クラスタ

**Objective:** As a 利用者, I want 複数の符号でできた絵文字が 1 文字として扱われること, so that 台詞の途中で部品がばらけて見えない

#### Acceptance Criteria

1. The areka のバルーン shall 台詞の文字を Unicode の拡張書記素クラスタ（人が 1 文字と見る単位）で数え、次のすべてで同じ 1 つの切り方を使う: 1 段ずつ出す演出・幅の計測・分かち書きの塊の数え方・折り返しの切れ目・選択肢の当たり範囲・ホバーの範囲・色とフォントの範囲・再生時間。
2. The areka のバルーン shall 次の形をそれぞれ 1 クラスタとして扱う: ZWJ で結んだ列（👨‍👩‍👧）・地域表示記号の対（🇯🇵）・肌色の修飾つき（👍🏻）・異体字セレクタつき（❤️＝U+2764 U+FE0F）・キーキャップ（1️⃣）・結合文字つき（か゚＝U+304B U+309A）。
3. The areka のバルーン shall 1 スカラー値の文字（日本語・英字・記号・😀 のようなサロゲートペア 1 つの絵文字）を今日どおり 1 クラスタとして扱う（数も順序も変わらない）。
4. When 台詞を 1 段ずつ出す, the areka のバルーン shall クラスタの部品（👨 だけ・🇯 だけ・ZWJ の途中）を途中の段で見せず、クラスタ全体を 1 つの段で出す。
5. When 台詞を 1 段ずつ出す, the areka のバルーン shall 出す間隔を「台本の時間 ÷ クラスタの数」で決める（部品の数で割らない）。
6. When 台本の文字列の再生時間を決める, the areka（さくらスクリプトの再生）shall クラスタ 1 つを 50 ms（今日の 1 文字の名目時間）と数える（部品の数で数えない）。
7. The areka のバルーン shall 選択肢（`\q`）の文字もクラスタで数え、選択肢の範囲の始点と長さをクラスタの通し番号で持つ。

### Requirement 3: 幅の見積もりと描画が一致する

**Objective:** As a 利用者, I want 絵文字の後ろの文字が正しい位置に出ること, so that 折り返しや `\_l` の位置がずれない

#### Acceptance Criteria

1. The areka のバルーン shall クラスタ 1 つの幅を、そのクラスタ全体を描いたときの幅と同じ値で見積もる（部品の幅の合計にしない）。
2. The areka のバルーン shall 見積もった幅と、行を描いたときのクラスタごとの幅とが、横書き・縦書き 2 方式の計 3 方式すべてで一致する（今日の「計測値と描いた行のクラスタ幅の一致」の判定を、クラスタの数を前提に絵文字へ広げる）。
3. When 同じクラスタを同じフォントで 2 度以上測る, the areka のバルーン shall 2 度目以降を 1 度目と同じ値で返す（キャッシュの鍵はクラスタ全体で、部品の 1 つに潰さない）。
4. When 絵文字の後ろに `\_l` で位置を指定する, the areka のバルーン shall 指定の基準（今の位置）を絵文字 1 つ分だけ進んだ位置に置く（部品の合計分だけ進めない）。
5. The areka のバルーン shall 描画層のフォントの計測が使えない縮退（固定幅の見積もり）でも、クラスタを 1 つの幅として見積もる（クラスタの中の部品の数で幅を増やさない）。

### Requirement 4: 折り返しと範囲はクラスタの途中で割らない

**Objective:** As a 利用者, I want 絵文字が行の途中で割れないこと, so that 家族や国旗が 2 行にまたがって崩れない

#### Acceptance Criteria

1. When 文字ごとの折り返し（`CharByChar`）で行を閉じる位置を決める, the areka のバルーン shall クラスタの境界でだけ切る（クラスタの途中に切れ目を置かない）。
2. When 分かち書きの折り返し（budoux）で行を閉じる位置を決める, the areka のバルーン shall 分かち書きの塊をクラスタの数で数え、塊が長すぎて文字の規則へ戻るときもクラスタの途中では切らない。
3. When クラスタ 1 つの幅が行の幅（折り返しの基準・絶対上限のどちらでも）を超える, the areka のバルーン shall そのクラスタを割らずに 1 行へ置く（今日の「1 文字が幅を超えるときの扱い」と同じ）。
4. The areka のバルーン shall 選択肢の当たり範囲（クリックの矩形）を、選択肢の中のクラスタの先頭の位置から末尾のクラスタの右端（縦書きは下端）までにする（部品の数で範囲を伸縮しない）。
5. While 選択肢の上にポインタが在る, the areka のバルーン shall 選択肢の文字のうち色つきの字形でないものをホバー色で描き、色つきの字形は自分の色のまま描く（ホバーの範囲は選択肢全体のクラスタを覆う）。
6. When `\f[color]`・`\f[name]`・`\f[height]` などの装飾の範囲がクラスタを含む, the areka のバルーン shall 範囲の境界をクラスタの境界に置き、クラスタの途中で装飾が切り替わらない。
7. The areka のバルーン shall 縦書き 2 方式でも 4.1〜4.6 と同じ振る舞いをする。

### Requirement 5: 既存の文字の見た目と挙動は変わらない

**Objective:** As a ゴーストの作者, I want 絵文字対応で日本語や英字の台詞が変わらないこと, so that 既存のゴーストの見た目がそのまま保たれる

#### Acceptance Criteria

1. The areka のバルーン shall 絵文字を含まない台詞の画素を、本仕様の前後で同じにする（日本語・英字・記号・指定フォントが持つ ♥）。
2. The areka のバルーン shall 絵文字を含まない台詞の、出す段の数・間隔・再生時間・折り返しの位置・選択肢の範囲・`\_l` の位置を、本仕様の前後で同じにする。
3. The areka のバルーン shall 既存のバイト等価の判定（スクロールの blit・live-diff・照合用の描画との等価）を、絵文字を含まない組で不変のまま通す（判定の期待値を書き換えない）。
4. The areka のバルーン shall 描画の失敗の記録と戻り値を今日のまま保つ（新しい失敗経路 0・`error!` の追加 0）。

### Requirement 6: 決定論テストで確かめる

**Objective:** As a areka の開発者, I want 色・単位・折り返しの 3 つが実機無しで再現できること, so that 後退をフルテストで検知できる

#### Acceptance Criteria

1. The テスト shall 読み戻しで、絵文字（指定フォントに無いもの）を描いた領域に「文字色でも背景色でもない色の画素」が出ることを判定する（色つきの字形が実際に描かれた証拠）。
2. The テスト shall 家族（ZWJ）・国旗・肌色・異体字セレクタ・キーキャップの 5 形を、それぞれ 1 段で出る（途中の段に部品が見えない）・1 クラスタ分の幅で置かれる・行の途中で割れないことを判定する。
3. The テスト shall 選択肢の中の絵文字について、当たり範囲がクラスタ 1 つ分で、ホバー中も色つきの字形の色が変わらないことを判定する。
4. The テスト shall 「絵文字を含まない台詞の画素が変わらない」ことを、変更前の実物（git の `main`）から取った対照、または同じテストの中で色つきフォントの設定の有無を描き比べる形で判定する（本番と照合用の描画に同じ設定を入れた後の live-diff だけでは証拠にしない）。
5. The テスト shall スクロールの blit と live-diff のバイト等価の組に、絵文字を含む台詞を 1 組以上足す。
6. The テスト shall 前提が `char` だった既存の 3 件（`glyph_unit_is_rust_char`・`counts_chars_not_bytes_or_utf16_units`・`probe_advances_match_drawn_line_cluster_advances`）を、名前と前提をクラスタへ改めて残す（消さない・判定の数値は据え置く）。
7. The テスト shall 絵文字が代替フォントで描かれることを確かめるとき、機械のフォント環境に依らない判定にする（Windows 10/11 に同梱の代替フォントが無い機械では理由を出して飛ばす。黙って緑にしない）。
8. The テスト shall 横書き・縦書き 2 方式の計 3 方式すべてで 6.2 を判定する。

### Requirement 7: 並走の条件と接触の範囲

**Objective:** As a areka の開発者, I want 並走する `file-drop`・`network-update`・`shell-balloon-switch` と衝突しないこと, so that α の枝を止めずに前倒しできる

#### Acceptance Criteria

1. The 本仕様 shall `crates/areka-emo-text/src/sink.rs`・`crates/areka/`・`crates/wintf/` に手を入れない（変更 0 ファイル）。
2. The 本仕様 shall `crates/areka-emo-text/src/actor.rs` を読むだけにし、折り返しの方式を選ぶ箇所の書き換えが要るときは先に着地した側へ合わせる。
3. The 本仕様 shall `Cargo.toml` の変更を `crates/areka-emo-text/Cargo.toml`（と必要なら `crates/areka-sakura/Cargo.toml`）の依存 1 行までにし、`Cargo.lock` の差分をその crate の依存の並びだけに収める（新しい外部クレートの追加は、既に `Cargo.lock` に在るものに限る）。
4. The 本仕様 shall 1 フレーム遅らせる解を取らない（状態の持ち方を変えて 0 フレームで解く）。

### Requirement 8: 記録

**Objective:** As a areka の開発者, I want 絵文字の扱いが記録から分かること, so that 実機で崩れたときに原因を辿れる

#### Acceptance Criteria

1. The areka のバルーン shall 絵文字を色つきで描くこと・クラスタで数えることのために、毎フレーム・毎文字の記録を増やさない（記録の量は今日のまま）。
2. If 幅の計測が失敗して固定幅の縮退へ倒れる, then the areka のバルーン shall 今日どおり `warn!` を残す（絵文字のために新しい記録の種類を作らない）。

### Requirement 9: areka の裁量の記録

**Objective:** As a areka の開発者, I want SSP と違う振る舞いが正典沈黙の記録に載ること, so that 後の spec が同じ判断を繰り返さない

#### Acceptance Criteria

1. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8「沈黙ルール対応表」へ 1 項目を足す: 指定フォントに無い絵文字は色つきの字形で描き、文字色・ホバー色に従わない（SSP は文字色の単色）。指定フォントが持つ記号は文字色に従う。退けた案＝⒜ 絵文字も単色（SSP と同じ・色つきの字形を捨てる）⒝ 切り替えのスイッチ（設定の口が増える）。
2. The 本仕様 shall 完了 `emo-text-layer` の設計が先送りした「書記素クラスタ結合は M2 検討事項」を本仕様が引き受けたことを、本仕様の文書（要件・設計）に書き、完了 spec の文書は書き換えない。

### Requirement 10: 裁定（推奨案による暫定の確定・要件ディスカッションで覆せる）

**Objective:** As a areka の開発者, I want brief が要件へ委ねた点の答えが 1 か所に並ぶこと, so that 討議で覆すべき点だけを選べる

#### Acceptance Criteria

1. **裁定 1（規模・分割）**: The 本仕様 shall 1 本のまま進める。色の変更は 2 か所（本番と照合用）、単位の変更は `areka-emo-text` の中と `areka-sakura` の 1 関数に収まり、テストの足場（読み戻し・バイト等価）を共有するので、規模は M と見立てる。分けない（分けたら削らないの規律より前に、分ける理由が無い）。
2. **裁定 2（色に従わない）**: The areka のバルーン shall 色つきの字形を `\f[color]`・ホバー色・無効表示の混色で染めず、染めるか否かのスイッチも持たない（brief の Out of Boundary をそのまま採る）。理由: ブラウザと同じ振る舞いで、指定フォントが持つ ♥ の赤は変わらない。
3. **裁定 3（字形の版）**: The areka のバルーン shall OS の標準の色つきフォントの描画で出る字形をそのまま受け入れ（平面の字形になる見込み）、グラデーション等の版を出すための自前の描き手は持たない。実測は設計の調査で行い、どちらの字形でも要件 1.1・6.1（色つきの画素が出る）を満たす。
4. **裁定 4（クラスタの規則）**: The areka のバルーン shall クラスタの切り方を Unicode の拡張書記素クラスタの規則（UAX #29）に置き、絵文字の形（ZWJ・国旗・肌色・異体字セレクタ・キーキャップ）を個別に特別扱いしない。切り方の出どころ（既に `Cargo.lock` に在る `unicode-segmentation` か、DirectWrite のクラスタ計測か）は設計で決める。
5. **裁定 5（再生時間）**: The areka（さくらスクリプトの再生）shall 1 クラスタを 50 ms と数える（今日の `CHAR_NOMINAL_MS` の単位だけを替え、値は変えない）。
6. **裁定 6（縦書きの向き）**: The 本仕様 shall 縦書きでの絵文字の向きを判定しない（DirectWrite の既定に任せる）。縦書きで確かめるのは「色つき・1 段・割れない」の 3 点だけ。
7. **裁定 7（`\_u`）**: The 本仕様 shall `\_u[0x…]` を扱わない。今日の areka では何も描かないことが実測で分かったので、担当 spec の決定は棚卸へ持ち込む（本仕様の要件・設計に `\_u` の経路を足さない）。
8. **裁定 8（影の申し送り）**: The 本仕様 shall `text-align-shadow-canon` の brief へ「影の複製は色つきフォントを使わず単色の塗りで描く」を 1 行足す（要件確定時）。本仕様は影を描かない。
9. **裁定 9（既存テストの扱い）**: The 本仕様 shall 前提が `char` だった既存テストを「陳腐化」と見て名前と前提を改めて残し、「壊れた」と見て消さない（判定の数値は据え置き）。
