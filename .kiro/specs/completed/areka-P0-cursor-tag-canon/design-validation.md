# 設計レビュー報告: areka-P0-cursor-tag-canon

> 実施: 2026-09-02 ／ 対象: `design.md`（2026-09-02 版）／ 入力: `requirements.md`・`research.md`・`brief.md`・steering（`structure.md`／`tech.md`／`logging.md`／`roadmap.md`）
> 方式: 非対話。設計が引用する file:line はすべて当日のソースで再照合し、縦書きの期待値は fixture 値から手計算で再現した。

## レビュー要約

設計は「位置 = 基点 + 値 × 係数」の式 1 本と、原点を `TextRegion::start()` に置き換える 1 点の切替に本質を絞り込めており、引用された file:line は実質すべてコードと一致した。縦書き `vertical_rl` の期待値（`\_l[0,0]` → 列 `[390, 400]`・`\_l[@-1lh,0]` → `[377, 387]`）は `region.rs:208-226` と `layout.rs:307-311`・`:618-623` から再現でき、設計の主張は裏付けられた。残る懸念は 3 件で、いずれも実装の組み方を 1〜2 段補えば済むものであり、設計の骨格を変える必要はない。

## 検証結果（設計の引用 file:line と主張の再照合）

| 設計の主張 | 照合先 | 結果 |
|---|---|---|
| 原点 = `TextRegion::start()`（未宣言は書字開始角・`vertical_rl` は `(right, top)`） | `region.rs:222-226`（`:224` HorizontalTb/VerticalLr = `(left, top)`・`:225` VerticalRl = `(right, top)`）・`:227-240` | 一致 |
| 軸読み替え表（`vertical_rl` は `block_dir = −1`・`block_start = start.0`） | `layout.rs:305-311` | 一致 |
| 列矩形は `left: block_pos − font_height`・`right: block_pos` | `layout.rs:618-623` | 一致 |
| V1 `\_l[0,0]` → 列 `[390, 400]`（`vertical_rl`・validrect 未宣言） | `region.rs:208-211` で validrect 未宣言 → `right = 400`（`resolve_or` の fallback `:301-309`）。`start.0 = 400` → `block_pos = 400` → 列 `[390, 400]` | 再現 |
| V2/V3 `\_l[-13,0]`／`\_l[@-1lh,0]` → `[377, 387]` | `400 − 13 = 387`（pitch = `ceil(10 × 1.25) = 13`・`layout.rs:123-125`）→ 列 `[377, 387]`。自動列送り `block_pos += −1 × 13` と同値 | 再現 |
| V5 `vertical_lr` の鏡像 `[0, 10]`／`[13, 23]` | `start.0 = left = 0`・`block_dir = +1`・`layout.rs:624-629` | 再現 |
| 現行は原点を validrect の辺から取る | `layout.rs:453-454`（`region.left()`／`region.top()`） | 一致 |
| 現行は後の `\_l` が保留を丸ごと上書き | `layout.rs:470` | 一致 |
| 撤去対象 4 項目 `cursor_to_image_px`／`CursorDegrade`／`CursorWarnGuard`／`warn_cursor_degrade` | `layout.rs:634-748`（非負ゲート `:658`・`Percent => None` `:664`） | 一致 |
| 保留フラッシュ順序 (1)→(2)→(3) は非改変 | `layout.rs:349-372` | 一致 |
| `visible_window` 非接触 | `layout.rs:512-558` | 一致 |
| `draw.rs` 980 行・番人の例外表に `areka-emo-text` なし | `wc -l` = 980・`file_length_guard_test.rs` に `areka-emo-text` 0 件 | 一致 |
| `TextRegion::resolve` は既に `image_size` を受け取る | `region.rs:203` | 一致 |
| `CursorWarnGuard` はランタイム所有 | `actor.rs:31`（import）・`:240`（フィールド） | 一致 |
| 語彙 `state.rs:108-133`・`parse_cursor_coord :148-183`・状態適用 `:409-416` | 実測 `:108-133`・`:148-184`・`:408-417` | 一致（±1 行） |
| パーサ非接触 `decode.rs:212`・`:223-229`／`compile.rs:137-145` | 実測どおり | 一致 |
| ダーティ矩形クランプ `viewbox.rs:722-746` | `expand_guard_clamp` | 一致 |
| ファイル名規約（`layout_cursor.rs` を避ける理由） | `structure.md:155-174` 最長 stem 優先。`layout_cursor_vertical_tests.rs` は候補 stem が `layout` のみ・`cursor_tag_tests.rs` は `cursor_tag` のみで一意 | 整合 |
| 既存 `layout_cursor_tests.rs` は 13 本・すべて横書き | `#[test]` 13 件 | 一致 |
| Requirement 8 の出所 `choice-render` requirements `:47-56`／`:95-105`・design `:31`／`:123`／`:124`／`:607-625`／`:632` | 実測どおり（`\_l` 5 行は `:613-617`） | 一致 |
| 誤登記の所在 brief `:27`／`:44`／`:54`／`:75`／`:84`・`roadmap.md:89`・`:143` 追記(85)・bvc design `:628`／research `:283` | 実測どおり。`emo-text-layer` requirements に `\_l` 0 件・Requirement 6 は 4 項目 | 一致 |
| 家法の先例 COMPAT §8 `:147`／`:153`・`\_l` 行 `:183` | 実測どおり | 一致 |
| emo2 の `\_l[5em,2lh]` 3 箇所 | `menu.pasta:15,33,62` | 一致 |

一致しなかった引用: **なし**（軽微な差のみ＝`state.rs:148-183` は実際には `:148-184`・「呼び手 134 箇所」は `TextRegion::resolve(` の grep で 128 件。いずれも挙動の主張に影響しない）。

## 重要な指摘（最大 3 件）

### 指摘 1: 純粋層の構造テストに新ファイルを登録する計画が無い

- **懸念**: `lib.rs:170-190` の `pure_layer_modules_have_no_windows_imports` は純粋層ファイルを `include_str!` で**名前列挙**して `windows` 依存の混入を検査している。設計の File Structure Plan は `lib.rs` の変更を「`pub mod cursor_tag;` の 1 行」とだけ定めており、新設 `cursor_tag.rs` を列挙へ加える計画が無い。`structure.md:181` は兄弟テストファイルも走査対象に列挙するよう定めているため、`cursor_tag_tests.rs`・`layout_cursor_vertical_tests.rs` も同様である。
- **影響**: 新しい純粋層モジュールが層規律の検査から黙って外れ、被覆が縮む。設計が「純粋層に `windows` 系依存を持ち込まない」を steering との整合として掲げている以上、検査の側に登録が無いと主張が機械で守られない。
- **提案**: File Structure Plan の `lib.rs` 行を「`pub mod cursor_tag;`＋`PURE_SOURCES` へ `cursor_tag.rs`・`cursor_tag_tests.rs`・`layout_cursor_vertical_tests.rs` の 3 件を追加」に改める。ついでに既存の `layout_cursor_tests.rs`・`state_cursor_coord_parse_tests.rs` が未登録である事実を申し送り（本仕様で足すか、登記のみか）として記す。
- **要件**: 9.1（決定論テスト）・設計 "Architecture Pattern & Boundary Map" の「steering との整合」
- **根拠**: design.md「File Structure Plan」`lib.rs` 行／`crates/areka-emo-text/src/lib.rs:170-190`／`structure.md:181`

### 指摘 2: 範囲外判定（2.6）の境界の含み方が未定義で、縦書きの正典形が「範囲外」に当たる

- **懸念**: `note_out_of_range(axis, value, region)` は「解決値が validrect の当該軸範囲の外なら `debug!`」とだけ定め、境界を含むか否かを述べていない。`vertical_rl` の正典形 `\_l[0,0]` は `x = right = 400` ＝ validrect の右辺**そのもの**に解決される。判定が `value >= right` の形で書かれると、正典どおりの 1 列目指定が毎回 DEBUG を吐く。逆に `x = left` の列は矩形 `[left − font_height, left]` が全域外だが点としては範囲内になる。
- **影響**: 実装者の裁量で境界の扱いが決まり、テスト（Unit Tests 4）は「範囲内で 0 件・範囲外で 1 件」としか言わないため、境界値の誤りが検出されない。ログの意味がぶれ、後続の観測（`RUST_LOG` grep）が汚れる。
- **提案**: 解決表に「範囲は閉区間 `[min, max]`・判定は**点**（グリフ箱ではない）」と明記し、Unit Tests 4 に境界値 3 件（`value == min`・`value == max`＝0 件、`max + 0.5`＝1 件）と、V1 で DEBUG 0 件であることを加える。
- **要件**: 2.6・2.3・9.4
- **根拠**: design.md「Service Interface」`note_out_of_range`／「Testing Strategy」Unit Tests 4／`layout.rs:618-623`

### 指摘 3: `\_l` の後に `\n` が来る並びの結果が到着順に依らず、設計が触れていない

- **懸念**: 保留フラッシュは (2) 保留改行 → (3) 保留カーソルの固定順で、**到着順を持たない**（`layout.rs:360-371`）。DD-3 の「実効位置」は `\n` → `\_l` の順（H3）は正しく扱うが、逆順 `あ\_l[@10,]\nあ` では (2) で行内が先頭へ戻った後に (3) が `x = 20` を上書きし、結果は `(20, 13)` となる。SSP では `\_l` 後の `\n` が行内を先頭へ戻すため `(0, 13)`。同様に `\_l[,100]\nあ` は改行の送り 1 行ぶんが失われる（areka `y = 100`・SSP `y = 113`）。これは本仕様の前からある挙動だが、設計は「`\_l` の配線＝保留の合成規則」を本仕様の所有と宣言しており、9.1 は「全語彙 × 縮退経路」の固定を求める。
- **影響**: 選択肢メニューで `\_l[@..]` の直後に `\n` を置く一般的な書き方が、正典と異なる位置に着地する。テストが無いため、次に触る人がどちらの挙動を正とするか判断できない。
- **提案**: 設計ディスカッションで次のいずれかを決める。(a) 本仕様では固定順のまま保ち、`\_l`→`\n` の現行値を決定論テストで固定したうえで、正典との差を COMPAT §8 と「語彙登記と申し送り」に登記する（最小）。(b) 保留の合成を到着順つきにする——`\n` が到着したとき保留カーソルがあれば、その値を仮適用してから改行を積む（実効位置と同じ仮適用を `LineBreak` 腕にも置く）。(b) は変更が `LineBreak` 腕の数行に閉じ、`newline-defer` の「保留のみでは行を開かない」規律を壊さない。
- **要件**: 3.5・6.1・9.1・9.6
- **根拠**: design.md「System Flows」保留の合成／「Revalidation Triggers」フラッシュ順序／`layout.rs:349-372`・`:443-448`

## 設計の強み

1. **式 1 本と原点 1 点への圧縮が、コードで裏取りできる形になっている**。縦書きの期待値は fixture 値（画像 400×224・font 10・pitch 13）から `region.rs:208-226` と `layout.rs:307-311`・`:618-623` だけで手計算でき、設計が挙げた `[390, 400]`・`[377, 387]` と一致した。原点切替の影響が `vertical_rl` の X だけに閉じること（2.7）も `region.rs:224` の同一分岐から構造的に言える。
2. **完了仕様の扱いが家法どおりで、出所がすべて実在する**。`choice-render` の 7 箇所・誤登記 3 系統・先例 2 行・`\_l` 行の所在を全件照合でき、編集対象は生きている文書（brief・`roadmap.md`・COMPAT §8）に限られ、アーカイブ本体への編集計画は無い。

## 最終判定

**判定: GO**

**根拠**: 既存の層（転写 → 意味論 → 配線）と依存方向を保ったまま、意味論を純関数の兄弟ファイルへ出す構成は現行の構造と整合し、引用 file:line は全件一致した。3 件の指摘はいずれも「登録漏れ」「境界の明文化」「登記か数行の合成規則か」の範囲で、設計の骨格を変えずに設計ディスカッションで確定できる。

**次のステップ**:
1. 設計ディスカッションで指摘 1〜3 を裁定し、`design.md` の File Structure Plan（`lib.rs` 行）・解決表（範囲外の境界）・System Flows（`\_l`→`\n` の扱い）へ反映する。
2. 反映後に `/kiro-spec-tasks areka-P0-cursor-tag-canon` でタスク生成へ進む。Testing Strategy の順序（現行値の固定 → 原点切替 → 語彙追加 → 文書）は 10.1〜10.3 の一括着地と両立しており、そのままタスクの骨組みに使える。
