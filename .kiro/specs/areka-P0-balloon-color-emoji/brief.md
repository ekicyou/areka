# Brief: areka-P0-balloon-color-emoji

> 起票: 2026-09-29（`/kiro-discovery`「areka のバルーンはカラー絵文字に対応しているか？対応が無いなら対応を入れたい」）。
> **段＝前倒し**（開発者 2026-09-29「α 後の早い時期に。並行開発できる項目なら α 前に前倒しましょう。いま並行開発要素が少ない」）。表現力の spec なので本来は α 後。α の残り 4 本と接触ファイルを照合し、並走できると判断したので B6（`file-drop`）と並べる。

## Problem

ゴーストの台詞に絵文字（😀・👍🏻・🇯🇵・👨‍👩‍👧）を書いても、バルーンでは次のように崩れる。

1. **色が出ない。** 文字色一色の輪郭で塗られる。
2. **複数の符号でできた絵文字が壊れる。** 1 文字ずつ出す演出の途中で部品が見える（👨 → 👨‍👩 → …／🇯 の箱だけが出る）。幅は部品の合計（家族なら約 3 文字）で見積もられるのに、描かれるのは 1 文字分。そのため後ろの折り返しが早まり、選択肢の当たり範囲や `\_l` の位置がずれる。行の途中で割れることもある。

## Current State（2026-09-29 実測・main `c3876110`）

- **描画**: 本番のバルーン文字は `crates/areka-emo-text/src/viewbox_draw.rs` の `ViewboxExecutor::render` で `draw_text_layout(..., D2D1_DRAW_TEXT_OPTIONS_NONE)` を呼ぶ 1 か所だけ。DC は `ID2D1DeviceContext`（v1・`create_device_context` で作る）、描き先は B8G8R8A8 の premultiplied。`draw.rs` の `DrawExecutor` は `#[cfg(test)]` の照合用で、こちらも `NONE` を渡している。
- **フォント**: `CreateTextFormat` はシステムのフォントコレクションを使う（`draw.rs` の `create_text_format`）。`SetFontFallback` は呼んでおらず、独自のコレクションも無い。つまり OS の代替フォントで Segoe UI Emoji までは届く（字形はあるが単色）。
- **文字の単位は Rust の `char`**（書記素クラスタではない）。
  - `state.rs`: `TextItem::Glyph { ch: char }`、出す間隔は `duration / chars().count()`。
  - `areka-sakura/src/duration.rs`: 再生時間は `chars().count() * 50ms`。
  - `layout.rs`: `GlyphMetrics::advance(ch: char, …)`。
  - `draw_metrics.rs`: 1 文字ずつの計測で、キャッシュの鍵は `(char, FontKey)`。
  - `segment.rs`: budoux の塊を `chars().count()` で数える。
  - 折り返し: `CharByChar` はどの `char` の前でも切る。
  - 過去の設計に「D10 グリフ単位は `char`… 書記素クラスタは M2 検討事項」と先送りの記録がある（`completed/areka-P0-emo-text-layer/research.md`・`design.md`、`completed/areka-P0-budoux-newline/research.md`）。
- **UTF-16 の範囲計算は正しい**（`len_utf16()` の積み上げ・`viewbox_draw_decoration.rs`）。サロゲートペアの単独絵文字（😀）は今でも 1 段・1 文字幅で出る。
- **色の効果**: `\f[color]` と選択肢のホバー色は、範囲ごとの `SetDrawingEffect(brush)` で塗る（`viewbox_draw.rs`・`viewbox_draw_decoration.rs` の `apply_color_ranges`）。
- **テスト**: 絵文字を描くテストは 0 件。ZWJ・国旗・肌色・異体字セレクタを扱うテストも 0 件。`draw_oracle_tests.rs` の `probe_advances_match_drawn_line_cluster_advances` が「1 文字＝1 cluster」を前提にしている。
- **参考**: wintf のタイプライター（`crates/wintf/src/ecs/widget/text/typewriter_layout.rs`）は DirectWrite のクラスタ単位で動いている（`completed/wintf-P0-typewriter/requirements.md`）。ただし areka のバルーンはこれを使っていない。
- **依存**: `unicode-segmentation` 1.13.3 は `convert_case` 経由で `Cargo.lock` に既に入っている（新しい外部クレートにはならない）。

## Desired Outcome

- バルーンで、代替フォントの絵文字が**カラーで**描かれる。
- 複数の符号でできた絵文字（ZWJ・国旗・肌色・異体字セレクタ・キーキャップ）が、**1 文字として**出る・測られる・折り返される。
  - 1 文字ずつ出す演出で部品が見えない。
  - 幅と描画が一致する。
  - クラスタの途中では行を割らない。
  - 選択肢の範囲や再生時間も、クラスタ単位で数える。
- これまでの文字（日本語・英字・♥ などのフォント自身が持つ記号）の見た目と挙動は変わらない。

## Approach

1. **色**: 本番の描画と照合用の描画の両方に `D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT` を渡す（`ID2D1DeviceContext` v1 で Windows 8.1 以上）。
   - Win11 の Segoe UI Emoji は COLR v1 も持っている。この経路で出るのは平面の COLR v0 の絵になる見込みだが、どちらの絵になるかは要件で実測して決める。COLR v1 を出すには `IDWriteFactory8::TranslateColorGlyphRun` と自前の描き手が要るので、要らなければ取らない。
2. **単位**: `char` を書記素クラスタに置き換える。対象は `TextItem::Glyph`、出す間隔、再生時間、計測（キャッシュの鍵を含む）、budoux の塊の数え方、`CharByChar` の切れ目、選択肢の範囲、`\_l` と当たり矩形。
   - クラスタの出どころは設計で決める。候補は `unicode-segmentation`（既にロックにある）と DirectWrite の `GetClusterMetrics`（wintf のタイプライターの手本）。
3. **テスト**: 読み戻しで「色つきの画素が出る」ことを確かめる。家族・国旗・肌色・異体字セレクタが、それぞれ 1 段・1 文字幅で出て、折り返しで割れないことも確かめる。スクロールの blit と live-diff のバイト等価の組にも絵文字を入れる。

## Scope

- **In**
  - バルーン文字の描画（`viewbox_draw.rs` と照合用の `draw.rs`）のカラーフォント化。
  - `areka-emo-text` の文字単位を書記素クラスタにすること（演出・計測・折り返し・範囲）。
  - `areka-sakura` の再生時間の数え方。
  - 上記のテスト。
- **Out**
  - wintf のウィジェット（`draw_labels.rs`・`typewriter_draw.rs`）: areka のバルーンではなく、example（`mock-shell`）だけが使う。
  - Win32 標準メニュー・`MessageBoxW`: OS が描く。
  - `\_u[0x…]` の解釈（現状は未確認・別件）。
  - Shift_JIS のゴースト: 応答に絵文字を載せられない。charset の既存仕様どおり。

## Boundary Candidates

- **描画層（色）と配置層（単位）。** 色の変更は 1 か所で済むが、テストの足場（読み戻し）は単位の側と共有する。1 本にまとめる理由は、色だけ先に出しても国旗や肌色つきのような今よく使われる絵文字が崩れたままになるから。
- 要件段階で規模が L に届いたら、「色＋単独の絵文字」と「クラスタ化」の 2 本に分けることを検討する（分けたら、それ以上は削らない）。

## Out of Boundary

- **絵文字を `\f[color]`／ホバー色で染めること。**
  - カラー絵文字は自前の色で描き、文字色に従わない。切り替えのスイッチも作らない。
  - SSP（GDI）は絵文字も文字色の単色で描き、里々 Wiki にも「絵文字の扱いは文字と等価」とある。それでも、よく使われる `\f[name,游ゴシック]\f[color,255,0,0]\_u[0x2665]` の赤い ♥ は変わらない。指定したフォント自身が ♥ を持っていて、代替の Segoe UI Emoji に回らないからである。色がつくのは、指定したフォントに無い字形の絵文字だけ（ブラウザと同じ振る舞い）。
- **文字の影**（`text-align-shadow-canon` の所有）。影の複製をカラーフォントのまま描くと、影まで多色になる。「影の複製はカラーフォントを使わずに（単色のブラシで）描く」ことを同 spec へ申し送る。
- 縦書きでの絵文字の向き（DirectWrite の既定に任せる。崩れが見つかったら別件にする）。

## Upstream / Downstream

- **Upstream**: 完了 `emo-text-layer`・`emo-text-viewbox`・`budoux-newline`・`text-decoration-canon`（色の範囲・フォントの範囲）。
- **Downstream**
  - `text-align-shadow-canon`（影の複製の描き方）。
  - `choice-marker-styling`・`anchor-tag-canon`（範囲がクラスタ単位になった後で着地する）。
  - `emo-text-canon-residue`（同じ crate）。

## Existing Spec Touchpoints

- **Extends**: なし。完了 spec の「書記素クラスタは M2 検討事項」の先送りを、ここで引き受ける。
- **Adjacent**
  - `shell-balloon-switch`（**09-29 に B8 へ繰り下げ**・`areka-emo-text` の `TextMsg`＝`sink.rs` に触る）。本 spec は `sink.rs` に触らない。`actor.rs` は折り返しの方式を選ぶ箇所を読むだけにして、書き換えが要るなら先に着地した側へ合わせる。
  - `network-update`（**09-29 に B7 へ繰り上げ**）とは共有 0（`crates/areka`・`crates/areka-update`・kanade）。`Cargo.lock` は別パッケージの節。
  - `file-drop`（B6・着手中）とは共有ファイル 0（2026-09-29 に `git diff main...claude/areka-p0-file-drop-9fdc80 --stat` で確認。`areka-emo-text`・`areka-sakura`・`areka-parsers`・`Cargo` は 0 件）。

## Constraints

- **並走の条件**: `crates/areka-emo-text/src/sink.rs`・`crates/areka/`・`crates/wintf/` には触らない。`Cargo.toml` を触るのは `crates/areka-emo-text/Cargo.toml`（と必要なら `crates/areka-sakura/Cargo.toml`）の依存 1 行まで。`Cargo.lock` の変化は、その crate の依存の並びだけに収める。
- 既存のバイト等価テスト（blit・live-diff・PNG）は、絵文字を含まない組で**不変**であること。
- 1 フレーム遅らせる解は取らない。
- Windows 10/11（tech.md）。カラーフォントは `ID2D1DeviceContext` v1 で足りる。
