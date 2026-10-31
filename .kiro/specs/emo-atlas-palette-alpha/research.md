# Research & Gap Analysis: emo-atlas-palette-alpha

> 2026-10-31・本ブランチ（main `da020c10` の上）で、確定した `requirements.md` と今のコードを突き合わせた。行番号はこの日の木のもの。設計・実装の着手時に引き直すこと。

## 1. 分析の要約

- **判定の持ち場は 2 か所だけで、どちらも同じ形の写し**: `crates/areka-emo-atlas/src/decode/wic_arm.rs` の `pixel_format_has_alpha`（170 行目・79 行目で引く）と `crates/wintf/src/com/wic.rs` の `pixel_format_has_alpha`（71 行目・51 行目で引く）。どちらも変換前の形式の GUID が 12 個の α つきの形式の集合に入るかだけを見る。`has_alpha` を使うのは `crates/areka-emo-atlas/src/normalize.rs` の `Normalizer::plan`（79・82 行目）だけで、wintf の `has_alpha` は本番では誰も使っていない（`ecs/widget/bitmap_source/bitmap_source.rs:111` が `_has_alpha` で捨てる）。
- **WIC の振る舞いを実測した（WPF の WIC 経由・作業木の `target\palette-alpha-check\` だけに検体を置いた）**: `tRNS` つきのパレットの PNG は変換前が `Indexed8`／`Indexed4`／`Indexed2`、フレームのパレットの α に `tRNS` が入り（`tRNS` が短ければ残りは 255）、PBGRA への変換の出力にも α が乗算済みで入る。emo2 の `null.png` は `Indexed8`・パレットの α `[255,0]`・全画素 `0.0.0.0`。`IWICPalette::HasAlpha` 相当の答え（パレットに 255 未満の α が在るか）は、要件の物差し「番号の透明度が 1 つでも 255 未満か」とそのまま一致する。
- **足りないものは「番号つきの形式ならパレットを写して `HasAlpha` を聞く」数行と、検体と赤のテスト**。`windows` 0.62.2 の今の機能（`Win32_Graphics_Imaging_D2D` が `Win32_Graphics_Imaging` を含む）に `IWICImagingFactory::CreatePalette`・`IWICBitmapSource::CopyPalette`・`IWICPalette::HasAlpha`・`GUID_WICPixelFormat{1,2,4,8}bppIndexed` が全部在る＝依存も機能も足さない。
- **要件の境界に食い違う事実が 1 つ見つかった**: WIC はグレーの PNG（色の型 0・8 ビット以下）の `tRNS` も `Indexed8`／`Indexed4` ＋ α つきのパレットとして出す。「番号つきの形式ならパレットの `HasAlpha`」で判定すると、境界で「変えない」とした色の型 0 の `tRNS` も α を持つ絵へ移る。同じく、透明色を持つ GIF も `Indexed8` ＋ α 0 の番号として出るので α を持つ絵へ移る（要件 1.6 の「形式によらず」には沿う）。8 ビットの BMP はパレットの α が全部 255 で変わらない。
- **推す形は案 A（今の 2 つのファイルをそれぞれ広げる）**。規模 S・危険度 低〜中（中の理由はグレーの `tRNS` と GIF の扱いを決める必要があること）。同梱の検体 11 本のうち、パレット＋`tRNS` の絵は emo2 の `null.png` 1 枚だけで、結果は今と同じ（全画素が透明）。

## 2. 今のコードの調べ

### 2.1 判定と、その使い手

| 場所 | 中身 | 本 spec との関係 |
|---|---|---|
| `crates/areka-emo-atlas/src/decode/wic_arm.rs`（306 行） | `decode_inner`（60〜111 行目）: `CreateDecoderFromFilename → GetFrame(0) → GetPixelFormat → pixel_format_has_alpha → FormatConverter(PBGRA) → CopyPixels`。判定は 170〜186 行目 | 直す所。`frame` が手元に在るので、`frame.CopyPalette` を足すだけで済む |
| `crates/wintf/src/com/wic.rs`（217 行） | `load_bitmap_source`（28〜65 行目）: 同じ手順。判定は 71〜90 行目。`pub fn load_bitmap_source` は `(IWICBitmapSource, bool)` を返す | 直す所（写し）。戻り値の形は変えなくてよい |
| `crates/areka-emo-atlas/src/normalize.rs`（188 行） | `plan`: `On if has_alpha → α`・`On → 左上を抜く`・`Full → opaque = !has_alpha`・`Off`・`Undeclared` は `has_alpha` を見ない | 変えない。`has_alpha` が真になれば要件 1.2〜1.4 は今の腕のまま満たされる |
| `crates/wintf/src/ecs/widget/bitmap_source/bitmap_source.rs:111` | `Ok((source, _has_alpha))` で捨てる | wintf 側の直しは本番の見た目を変えない（要件 3.4 は自明に満たされる） |
| `crates/areka-emo-atlas/src/decode/image_arm.rs` | 動く絵の `has_alpha` は `image` の色の型から取る | 範囲外（変更 0） |

`has_alpha` を読む本番のコードは `normalize.rs` の 2 行だけで、`crates/areka` 側（`placement/measure.rs` など `WicDecoderArm` を使う所）は `has_alpha` を直接読まない（`grep` で 0 件）。

### 2.2 WIC の実測（2026-10-31・WPF の `BitmapDecoder`／`FormatConvertedBitmap` 経由）

検体は Python で手書きしたバイト列を `target\palette-alpha-check\` に書いた（`C:\` 直下・OS の一時フォルダは使っていない）。WPF の画像の読みは WIC の上に乗っており、パレットは WIC のフレームの `CopyPalette` から、変換は WIC の形式の変換器から来る。`windows` クレートからの直の呼び出しでは確かめていない（下の「要調査」1）。

| 検体 | 変換前の形式 | パレットの α | PBGRA の出力（先頭の画素） | 今の `has_alpha` | パレットの `HasAlpha` で判定した後 |
|---|---|---|---|---|---|
| 8 ビット・番号 0 不透明・1 透明・2 半透明（`tRNS` 3 個＝番号 3 は 255） | `Indexed8` | `255,0,128,255` | `0.0.255.255`・`0.0.0.0`・`128.0.0.128`・… | 偽 | **真** |
| 同じ絵の 4 ビット | `Indexed4` | `255,0,128,255` | 同上 | 偽 | **真** |
| 同じ絵の 2 ビット | `Indexed2` | `255,0,128,255` | 同上 | 偽 | **真** |
| 8 ビット・`tRNS` 無し | `Indexed8` | 全部 255 | 全部 α 255 | 偽 | 偽（変わらない） |
| 8 ビット・`tRNS` 全部 255 | `Indexed8` | 全部 255 | 全部 α 255 | 偽 | 偽（変わらない） |
| emo2 `purple/a/null.png`（382×547） | `Indexed8` | `255,0` | 全画素 `0.0.0.0` | 偽 | **真**（結果は同じ＝全画素が透明） |
| GIF89a・透明色 1 | `Indexed8` | `255,0,255,255` | 透明色の画素は `0.0.0.0` | 偽 | **真**（下の決めごと 2） |
| 8 ビットの BMP（予約の 1 バイトは 0） | `Indexed8` | 全部 255 | 全部 α 255 | 偽 | 偽（変わらない） |
| グレー 8 ビット＋`tRNS` | `Indexed8`（256 色のグレーのパレット） | 該当の 1 つだけ 0 | 該当の画素は `0.0.0.0` | 偽 | **真**（下の決めごと 1） |
| グレー 4 ビット＋`tRNS` | `Indexed4` | 該当の 1 つだけ 0 | 同上 | 偽 | **真**（同上） |
| グレー 8 ビット・`tRNS` 無し | `Gray8` | パレット無し | 全部 α 255 | 偽 | 偽（番号つきでないので見ない） |
| グレー 16 ビット＋`tRNS` | `Rgba64` | パレット無し | 該当の画素は透明 | **真**（今も） | 真（変わらない） |
| RGB 8 ビット＋`tRNS` | `Bgra32` | パレット無し | 該当の画素は透明 | **真**（今も） | 真（変わらない） |
| RGB 8 ビット・`tRNS` 無し | `Bgr24` | パレット無し | 全部 α 255 | 偽 | 偽（変わらない） |

分かったこと:

- 要件 1.5（番号の桁によらない）は、WIC が 2・4・8 ビットを全部同じ形（番号つきの形式＋α つきのパレット）で出すので、判定 1 つで満たせる。1 ビットは `GUID_WICPixelFormat1bppIndexed` として同じ集合に入れればよい。
- 要件 2.6（emo2 の結果を変えない）: `null.png` の左上は `0.0.0.0` で全画素も `0.0.0.0` なので、抜き色の道（左上と同じ 4 バイトを透明に）でも α の道（素通し）でも出力は 1 バイトも同じ。golden の `0\tpurple/a/null.png\tEMPTY orig=382x547` は変わらない見込み。
- 要件の境界の「色の型 0・2 の `tRNS` は今の読み方を変えない」は、色の型 2（RGB）と 16 ビットのグレーについては今も WIC が α つきの形式で出しているので「今も α を持つ絵」であり、本 spec の判定では変わらない。**8 ビット以下のグレー＋`tRNS` だけが、判定の形しだいで変わる**（決めごと 1）。

### 2.3 同梱の検体の中身（`vendors/sample_ghost/*.nar` の PNG を全部読んだ）

| 検体 | PNG の色の型の内訳 |
|---|---|
| `emo2` | 色の型 2: 2・色の型 3（`tRNS` 無し）: 4・**色の型 3＋`tRNS`: 1（`shell/master/purple/a/null.png`）**・色の型 6: 66 |
| `konnoyayame` | 色の型 3（`tRNS` 無し）: 23 |
| `dot_sakura` | 色の型 2: 6・色の型 3（`tRNS` 無し）: 127 |
| `emily4` | 色の型 0（`tRNS` 無し）: 1・色の型 2: 72・色の型 3（`tRNS` 無し）: 45・色の型 6: 14 |
| `emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit` | 色の型 2: 2・色の型 3（`tRNS` 無し）: 4・色の型 6: 9 |
| `R_POST_and_KOMAINU`・`StayseeBalloon`・`areka-balloon`・`claudia` | パレットの絵なし |

- パレット＋`tRNS` の絵は全検体で `null.png` の 1 枚だけ（要件の記述どおり）。その `tRNS` は `ff 00`、パレット 2 色、全画素が番号 1（左上の番号も 1・透明度 0）。
- 色の型 0＋`tRNS`・GIF・BMP は検体に 0 件（GIF・BMP のファイル自体が無い）。決めごと 1・2 をどちらに決めても、同梱の検体の出方は変わらない。
- `crates/areka/shell/x1.png`〜`x5.png` は要件のとおり赤の検体にならない（本調べでは読み直していない）。

### 2.4 テストの置き場と流儀

- emo-atlas: 本番ファイルにテストの本体を書かず、兄弟のファイルへ置く（`structure.md`「Unit Tests」）。`wic_arm.rs` は末尾で `#[path = "wic_arm_tests.rs"] mod animation_tests;` を持つ（`wic_arm_tests.rs` は動く絵の 3 メソッドの檻・155 行）。`wic_arm.rs` の中にも古い `mod tests`（188〜302 行目・実 emo2 の `online0.png` など）が在る。COM は `CoInitializeEx(MTA)` を張って `WicDecoderArm::new()` する（`with_arm`）。
- emo-atlas の検体の前例: `src/testdata/animated/` に 12 個のファイルをコミットし、作り手 `examples/gen_animated_samples.rs`（`image` と std だけ・CRC は自前の数行・48 行目 `fn crc32`）で作り直せるようにしている。
- emo-atlas の dev-dependencies に `sample-ghost-kit` が在り、`sample_ghost_kit::WorkDir::new()` が作業木の `target` の下に空のフォルダを借りて、落とすときに消す（`areka-nar` などが使う「OS の一時フォルダを使わないための唯一の窓」）。
- wintf: 統合テスト `tests/com/wic_test.rs`（190 行・`com::wic` の各トレイトを直に検証）と `tests/widget/bitmap_source_integration_test.rs`（`has_alpha` の判定のテスト `test_has_alpha_true_for_rgba_png`・`test_has_alpha_false_for_rgb_bmp` が在る）。検体は `tests/assets/` にコミットしたファイル（`test_8x8_rgb.bmp` など）。wintf の dev-dependencies に `sample-ghost-kit` は**無い**が、統合テストには cargo の `CARGO_TARGET_TMPDIR`（`target\tmp`）が在る（`areka-emo-text/tests/` が前例）。
- `pixel_format_has_alpha` は wintf でも emo-atlas でも非公開。wintf のテストは `pub fn load_bitmap_source` を通して `has_alpha` を見る形になる（今の 2 本と同じ）。
- `image` 0.25 の PNG の書き手は番号つきの形式を書けない。パレットの PNG の検体はバイト列を手で組む（`IHDR`・`PLTE`・`tRNS`・`IDAT`・`IEND`。CRC32 と zlib の包み）ことになる。`png` クレートは直の依存に無い（足さない方針）。

### 2.5 書き直す文書の今の位置

- `doc/COMPAT_ARCHITECTURE.md` §8（124 行目から）の **273 行目**＝「パレット形式の `tRNS`（色ごとの透明度）を持つ絵を抜き色で扱うときに、`tRNS` の透明度を生かすか」（決め「生かす」・持ち主 `areka-P0-shell-implicit-surface`（要件 4.8）・理由 ⑴ ⑵ の中で `normalize_key_color_tests` の `tRNS` 相当の入力を挙げる）。
- **274 行目**＝「抜き色の色を比べる値の空間」。`tRNS` に触れる言い回しは 2 か所: 決めの欄の「α も比較に入るため、`tRNS` で α が異なる画素は左上と色が同じでも別の画素として残る」と、理由の欄の「α を比較に入れるのは上の `tRNS` の裁量（透明度を生かす）と同じ帰結で、…」。
- §8 の 451 行目（動く絵の「透明度を持つかの見分け」）は「APNG は色の形式が α つきか `tRNS` を持つこと」で、`tRNS` が全部 255 でも α を持つ絵になる。静止画の要件 2.2（全部 255 なら α を持たない）と物差しが違うが、動く絵は範囲外（変更 0）。食い違いの記録だけ残す（決めごと 9）。
- 要件 2.7 が直すテストの説明は `crates/areka-emo-atlas/src/emo2_e2e.rs` の 80〜84・92〜93・168〜169・281〜282 行目と、`crates/areka-emo-atlas/src/emo2_golden.rs` の 172〜173・200〜203 行目（「α チャンネルを持たない」「α 無し」「抜き色の腕を通って」）。判定の値（失敗 0 件・原寸 382×547・`placement == None`・golden の行）は変えない。
- 要件の範囲に名前の出ていない、同じ前提の説明: `normalize_key_color_tests.rs` 144〜148 行目（「`tRNS` 相当」「パレット＋`tRNS` の絵は、抜き色の腕へ α の掛かった画素として届く」）と `normalize_rule_tests.rs` 119〜120 行目（「α の印が偽なのに α<255 の画素を持つ絵（パレットの PNG の透明の情報など）」）。テストの中身（正規化器の単体の振る舞い）は本 spec の後も正しいが、説明の前提が古くなる（決めごと 8）。

## 3. 要件と今の資産の対応

| 要件 | 今の資産 | 差 |
|---|---|---|
| 1.1 透明度を持つパレットの絵を α を持つ絵に | `wic_arm.rs::pixel_format_has_alpha` | **Missing**: 番号つきの形式でパレットを見る分岐 |
| 1.2〜1.4 `1`／`true`・`full` での焼き込みの結果 | `normalize.rs::plan`／`apply` | 無し（`has_alpha` が真になれば今の腕で満たす） |
| 1.5 桁によらない | WIC が 1・2・4・8 ビットを番号つきの形式で出す（実測） | **Missing**: 4 つの番号つきの GUID の集合 |
| 1.6 形式によらない | WIC がフレームのパレットを出す | **Constraint**: GIF の透明色・グレーの `tRNS` も同じ物差しに入る（決めごと 1・2） |
| 2.1・2.2 `tRNS` 無し・全部 255 は今のまま | `HasAlpha` が偽を返す（実測） | 無し |
| 2.3 パレットでない形式は変えない | 12 個の集合 | 無し（集合はそのまま残す） |
| 2.4・2.5 `0`・宣言なし | `plan` の `Off`・`Undeclared` は `has_alpha` を見ない | 無し |
| 2.6 emo2 の結果が同じ | `emo2_e2e.rs`・`emo2_golden.rs`・`emo2_null_png_logs_debug_not_warn` | 無し（実測で出力が同じ）。golden の通過は実装で確かめる |
| 2.7 emo2 のテストの説明 | 2.5 の行 | **Missing**: 説明の書き直し（値は変えない） |
| 3.1〜3.3 wintf の答え | `wic.rs::pixel_format_has_alpha` | **Missing**: 同じ分岐の写し |
| 3.4 wintf の画素を変えない | 変換器は触らない | 無し |
| 3.5 2 つの読み手が同じ答え | 2 つの写し | **Constraint**: 両方を見る crate のテストは無い。同じ検体・同じ期待値で両方を檻に入れる形になる（決めごと 6） |
| 4.1・4.2 パレットが読めないときは失敗 | `decode_inner` の `?` → `DecodeError::Decode { path }`、wintf は `Result` | **Missing**: `CreatePalette`／`CopyPalette`／`HasAlpha` を `?` でつなぐこと。**Unknown**: 本物の WIC で `CopyPalette` だけを決定論で失敗させる検体が作れるか（要調査 3） |
| 5.1〜5.5 赤のテスト | `wic_arm_tests.rs`・`tests/com/wic_test.rs`・`bitmap_source_integration_test.rs` | **Missing**: パレットの PNG の検体の組み手と、読み戻しのテスト |
| 5.6 決定論・GPU 不要 | WIC は CPU だけ（今のテストも同じ） | 無し |
| 5.7 検体の置き場 | `sample_ghost_kit::WorkDir`（emo-atlas）・`CARGO_TARGET_TMPDIR`（wintf の統合テスト） | **Constraint**: コミットしたファイルを検体にする前例（`testdata/animated/`・`tests/assets/`）が 5.7 の 2 つの形のどちらかに当たるか（決めごと 3） |
| 6.1〜6.3 §8 の書き直し | 273・274 行目 | **Missing**: 文の書き直し（実装の着地の後） |

## 4. 実装の案

### 案 A: 今の 2 つのファイルをそれぞれ広げる（写しを保つ）

- `wic_arm.rs` の `decode_inner` で、`pre_format` が番号つきの 4 つの GUID のどれかなら `factory.CreatePalette()` → `frame.CopyPalette(&palette)` → `palette.HasAlpha()` を `?` でつなぎ、`has_alpha` をその答えにする。番号つきでなければ今の 12 個の集合で決める。
- `wic.rs` の `load_bitmap_source` にも同じ数行（`factory` と `frame` は手元に在る）。
- テストは emo-atlas の兄弟のファイル（`wic_arm_tests.rs` に足すか、新しい兄弟 `wic_arm_palette_tests.rs`）と、wintf の統合テスト（`tests/com/wic_test.rs` か `tests/widget/bitmap_source_integration_test.rs`）。
- 利点: 足す行が少ない（判定は各 10 行前後）。今の「emo-atlas は wintf を参照しない・同じ判定を写す」構造のまま。依存も公開の型も増えない。
- 欠点: 判定を 2 か所に持つことは今と同じ（写しの食い違いはテストで見張る）。

### 案 B: 判定を 1 か所の新しい部品に置く

- 例: `windows` だけに依存する小さな葉の crate（または既存の葉）に「フレームと factory から α の有無を答える関数」を置き、emo-atlas と wintf の両方から呼ぶ。
- 利点: 判定の持ち主が 1 つになり、要件 3.5 が構造で保たれる。
- 欠点: 新しい crate（または既存の葉への責務の追加）と依存の線が増える。判定は 10 行前後で、葉を足す釣り合いが取れない。今の「写す」という決め（`wic_arm.rs` 166〜169 行目の説明）を覆す。

### 案 C: 判定を変換前の形式から外し、PBGRA の出力の画素で決める

- 例: 番号つきの形式のときは、変換した後の画素に α<255 が在るかで決める（`Undeclared` の決め方と同じ物差し）。
- 利点: パレットを読む呼び出しが要らない。
- 欠点: 要件の物差し「番号の透明度が 1 つでも 255 未満か」と違う（透明度 255 未満の番号を 1 画素も使っていない絵で答えが割れる）。wintf の `load_bitmap_source` は画素を写さずに `IWICBitmapSource` を返すので、wintf 側だけ全画素を読む手間が増える。要件 2.2 の「全部 255 は α を持たない」とは合うが、要件 1.1 の判定とは合わない。

### 比べ

| | 案 A | 案 B | 案 C |
|---|---|---|---|
| 要件の物差しとの一致 | 一致（`HasAlpha` の定義＝255 未満の番号が在るか） | 一致 | 一部違う |
| 足すもの | 各ファイルに数行＋テスト | 部品・依存の線＋テスト | 画素の走査（wintf は写しも） |
| 今の構造との合い方 | そのまま | 写しの決めを覆す | 判定の場所は同じ |
| 規模 | S | S〜M | S |

## 5. 規模と危険度

- **規模: S**（1〜3 日）。判定は 2 か所に各 10 行前後、検体の組み手（CRC32 と zlib の包みで 40 行前後）とテスト、文書の 2 行。brief の見立て（3〜5 タスク）と合う。
- **危険度: 低〜中**。WIC の振る舞いは実測で要件の前提どおり。中にする理由は、グレーの `tRNS` と GIF の透明色の扱い（決めごと 1・2）が要件の境界の書き方と食い違っていて、要件の討議で決める必要があること。同梱の検体ではどちらに決めても出方が変わらない。

## 6. 要調査（設計へ持ち越す）

1. **`windows` クレートからの直の確かめ**: 実測は WPF（WIC の上）経由。`IWICPalette::HasAlpha` を `windows` 0.62.2 から呼んで同じ答えになることは、実装の赤のテストがそのまま確かめる（別の調べは要らない見込み）。
2. **パレットを持つ絵を出すほかの WIC の読み手**: TIFF・ICO など。element の画像は実質 PNG だが、要件 1.6 の「形式によらず」の範囲をどこまで檻に入れるか（GIF だけでよいか）。
3. **要件 4 の失敗の道の檻**: 本物の WIC で `CopyPalette`／`HasAlpha` だけを決定論で失敗させる検体が作れるか（壊れた `PLTE` は `GetFrame`・`CopyPixels` の段で先に失敗する見込み）。作れなければ、失敗の道は `?` で今の `DecodeError::Decode { path }`／wintf の `Err` へつながる構造だけで保つ形になる。
4. **壊れた `tRNS`**（パレットより長い・`PLTE` の前に在る）の WIC の扱い。拒むのか無視するのか。要件の物差しには直接関わらないが、赤の検体を組むときに避ける形を知っておく。

## 7. 設計の段への申し送り（要件の討議で決めること）

> **2026-10-31 要件の討議（開発者が推奨の案で一括承認）**: 1 と 2 は案 ⓐ（グレーの `tRNS`・GIF の透明色も同じ物差しに入れる・検体を要件 5.8 に足す）。全部 255 の `tRNS` は α を持たない絵のまま（要件 2.2）で、動く絵との物差しの違いは §8 の理由の欄に記録だけ残す（要件 6.1・9 の後半）。8 は要件 2.8 に入れた。3 は要件 5.7 をコミットした検体も許す形に広げ、どちらにするかは設計で決める。残り（3〜7・9 の言い換え・10）は設計の段で決める。

1. **グレー（色の型 0・8 ビット以下）＋`tRNS` の扱い**: WIC は番号つきの形式＋α つきのパレットとして出すので、「番号つきなら `HasAlpha`」の判定では α を持つ絵へ移る。要件の境界の「色の型 0・2 の `tRNS` は本 spec では調べず、今の読み方を変えない」と食い違う。
   - 案 ⓐ: 移るのを受け入れ、境界の文を「WIC が番号つきの形式で出す絵は全部同じ物差し」に改める。RGB＋`tRNS`・16 ビットのグレー＋`tRNS` は今も α を持つ絵なので、`tRNS` の扱いが色の型によらずそろう。足すコードは無い。
   - 案 ⓑ: 判定を PNG の色の型 3 に限る。PNG の `IHDR` を読む必要がある（emo-atlas は `sniff.rs` で見出しを読む前例があるが、wintf には無い）。2 つの読み手に見出しの読みを足すことになり、要件 1.6（形式によらず）とも合わせにくい。
2. **GIF の透明色**: 同じく α を持つ絵へ移る（今は `1`／`true` で左上を抜く）。要件 1.6 の「PNG とそれ以外で静止画の読み手が読むパレットの絵」に入るものとして受け入れるか。同梱の検体に GIF は 0 件。§8 の書き直しで GIF に触れるか。
3. **検体の置き場（要件 5.7）**: ⓐ テストの中でバイト列を組み、emo-atlas は `sample_ghost_kit::WorkDir`、wintf は `CARGO_TARGET_TMPDIR` の下へ書く（どちらも `target` の下・依存は足さない）。ⓑ 前例（`testdata/animated/`＋作り手の example・wintf の `tests/assets/`）にならってファイルをコミットする。ⓑ が 5.7 の「バイト列から組む」に当たるかの解釈。
4. **パレットの PNG の組み手を何か所に持つか**: emo-atlas と wintf は互いに参照できない。ⓐ 2 か所に同じ数十行を写す ⓑ テスト専用の葉（`sample-ghost-kit` など）に置き、wintf に dev-dependency を 1 行足す ⓒ 決めごと 3 の ⓑ（ファイルをコミット）なら組み手は example 1 か所。
5. **判定の形**: 番号つきの 4 つの GUID の集合で入口を決め、そこでだけ `CopyPalette` を呼ぶ形（推す見込み）か、どの形式でも `CopyPalette` を試し、`WINCODEC_ERR_PALETTEUNAVAILABLE` を「パレット無し」と読む形か。後者は「読めなかった」と「無い」を失敗の種類で見分けることになり、要件 4（推測で決めない）と相性が悪い。
6. **要件 3.5（2 つの読み手が同じ答え）の確かめ方**: 両方を参照する crate のテストは無い。同じ検体の形・同じ期待値で、2 つの crate がそれぞれ檻を持つ形でよいか（`areka` crate は両方に依存するので、ここに突き合わせのテストを置く手もあるが、触るファイルが広がる）。
7. **emo-atlas のテストのファイル**: 動く絵の檻の `wic_arm_tests.rs`（155 行）に足すか、新しい兄弟（例 `wic_arm_palette_tests.rs`）にするか。焼き込みの結果（要件 5.1・5.2）を `WicDecoderArm`＋`Normalizer` で見るテストを読み手のファイルの兄弟に置くか、`normalize` 側の兄弟に置くか。wintf は `tests/com/wic_test.rs`（roadmap が名指し）か `tests/widget/bitmap_source_integration_test.rs`（今の `has_alpha` のテストが在る）か。
8. **要件の範囲に名前の無い、古くなる説明**: `normalize_key_color_tests.rs` 144〜148 行目と `normalize_rule_tests.rs` 119〜120 行目は「パレット＋`tRNS` の絵が α の印の偽で届く」ことを前提に書いている。テストは正規化器の単体の備えとして残してよいが、説明を直すか（要件 2.7 は emo2 の焼き込みのテストと golden の説明だけを名指しする）。決めごと 1 が ⓐ なら、WIC の読み手から「α の印が偽で α<255 の画素」が届く道は見当たらなくなる。
9. **§8 の 274 行目の言い換えの中身（要件 6.2）**: 「α を比較に入れるのは上の `tRNS` の裁量と同じ帰結」の根拠が無くなる。決めごと 1 の答えしだいで、抜き色の腕へ α<255 の画素が届く道が残るか（ⓑ ならグレーの `tRNS` が残る）が変わり、言い換えの文が変わる。あわせて、動く絵の行（451 行目・`tRNS` が在れば全部 255 でも α を持つ絵）と静止画の物差しが違うことを記録だけするか（要件 6.3 で他の行は変えない）。
10. **wintf の `has_alpha` に本番の使い手が無いこと**: 直しは答えをそろえるだけで見た目は変わらない。要件 3 の檻だけで足りるか（使い手を足すのは範囲外）。

## 8. 設計の段のおすすめ

- 案 A を軸に、決めごと 1・2 の答えを先にもらう。ⓐ なら判定は「番号つきの形式ならパレットの `HasAlpha`、それ以外は今の集合」の 1 本で、2 つの読み手で同じ数行になる。
- 赤のテストは、検体（8 ビットと 4 ビット・左上は不透明の番号・透明度 0 と 128 の番号を持つ）を組んで、`WicDecoderArm::decode` の `has_alpha` と、`Normalizer` に `On`／`Full` で通した画素を読み戻す。wintf は `load_bitmap_source` の 2 つ目の戻り値を見る。直す前は `has_alpha == false` で赤、`On` では左上と同じ色の画素が透明になって赤、`Full` では透明度 0 の画素が不透明になって赤になる。
- 着地の後、emo2 の 3 本（`emo2_shell_all_elements_baked`・`emo2_null_png_logs_debug_not_warn`・`emo2_shell_matches_golden`）と結合のテストが値を変えずに通ることを確かめ、説明だけを直す。

## 9. 設計の段の調べ（2026-10-31・`/kiro-spec-design -y`）

- **調べの種類**: 軽い調べ（今の仕組みを広げる）。新しい依存は無いので外の調べはしていない。使ったスキルは `kiro-spec-design`（規則 `design-principles`・`design-discovery-light`・`design-synthesis`・`design-review-gate`）だけ。

### 9.1 `windows` 0.62.2 からの直の確かめ（§6 の要調査 1・3 の答え）

- 作業木の `target\wicprobe\` の下だけに小さな使い捨てのプログラムと検体を置いて走らせ、終わった後に消した（`C:\` 直下・OS の一時フォルダには書いていない。プログラムの元は会話の作業用フォルダ）。検体は手で組んだバイト列（zlib は無圧縮の 1 ブロック）。
- 結果（変換前の形式・`CopyPalette`・`HasAlpha`・PBGRA の 4 画素）:

| 検体（2×2） | 変換前 | `CopyPalette` | `HasAlpha` | PBGRA |
|---|---|---|---|---|
| 8 ビットのパレット・`tRNS` `ff 00 80`・番号 0,0,1,2 | `8bppIndexed` | 成功 | 真 | `0,0,255,255`・`0,0,255,255`・`0,0,0,0`・`128,0,0,128` |
| 同じ絵の 4 ビット | `4bppIndexed` | 成功 | 真 | 同上 |
| 8 ビットのパレット・`tRNS` 無し・番号 0,0,1,1 | `8bppIndexed` | 成功 | 偽 | `0,0,255,255`×2・`0,255,0,255`×2 |
| 同じ・`tRNS` `ff ff ff` | `8bppIndexed` | 成功 | 偽 | 同上 |
| 8 ビットのグレー・`tRNS` で値 0・値 255,255,0,128 | `8bppIndexed` | 成功 | 真 | `255,255,255,255`×2・`0,0,0,0`・`128,128,128,255` |
| GIF89a・色の表 4 色・透明の番号 1・番号 0,0,1,2 | `8bppIndexed` | 成功 | 真 | `0,0,255,255`×2・`0,0,0,0`・`255,0,0,255` |
| RGBA の PNG（1×1） | 番号つきでない | **失敗 `0x88982F45`（`WINCODEC_ERR_PALETTEUNAVAILABLE`）** | — | `3,2,1,255` |

- 分かったこと: WPF 経由の実測（§2.2）と同じ答え。番号つきでない絵のフレームに `CopyPalette` を呼ぶと決まって失敗するので、パレットを読む関数が失敗を返すことを決定論で檻に入れられる（§6 の 3 の答え）。GIF の LZW は、符号ごとに消去の符号を挟むと 3 ビットのままで書ける。

### 9.2 §7 の持ち越しの答え（設計で決めた・詳しい理由は design.md「設計の決め」）

| §7 | 決め |
|---|---|
| 3 検体の置き場 | コミットする。作り手は emo-atlas の example `gen_palette_samples.rs`（std だけ）1 つで、emo-atlas の `src/testdata/palette/` と wintf の `tests/assets/palette/` へ同じバイト列を書く。テストは走る間に書かない |
| 4 組み手を何か所に持つか | 作り手 1 か所（決め 3 の ⓑ）。テストの中で組む形は 2 つの crate に写すことになるので採らない |
| 5 判定の形 | 番号つきの 4 個の集合で入口を決め、その枝でだけパレットを読む。`PALETTEUNAVAILABLE` を「無い」と読む形は採らない |
| 6 要件 3.5 の確かめ方 | 同じバイト列を 2 つの crate のテストが同じ期待値の表で読む。`crates/areka` に突き合わせを置く形は採らない |
| 7 テストのファイル | emo-atlas は新しい兄弟 `wic_arm_palette_tests.rs`（共有の `with_arm` は `wic_arm_test_support.rs` へ移す）・焼き込みの画素は `WicDecoderArm`＋`Normalizer`。wintf は `tests/com/wic_test.rs` |
| 要件 4 の檻（§6 の 3） | 番号つきでない絵のフレームでパレットを読む関数が失敗を返すことを檻に入れる。wintf は関数を `pub` にして結合テストから呼ぶ |
| 9 の言い換え | 274 行目は決めを変えず、`tRNS` に触れた 2 か所を「α を持たないとされた絵に α の異なる画素が混じって届いた場合の備え」に直す。文は design.md「§8 の書き直し」 |
| 10 wintf の使い手 | 足さない。答えをそろえて檻に入れるだけ |

### 9.3 まとめ方の見直し（design-synthesis）

- **一般化**: 要件 1.5・1.6 の「桁・形式によらない」は、WIC がどれも「番号つきの形式＋パレット」で出すので、判定 1 本で済む。形式ごとの枝は作らない。
- **作るか使うか**: 判定は WIC の `IWICPalette::HasAlpha`（定義が要件の物差しと同じ）を使い、パレットの色を自分で歩かない。検体の PNG は `image`（番号つきの形式を書けない）も `png`（依存に無い）も使わず、std で組む。
- **削ったもの**: 2 つの読み手の判定を 1 か所へ寄せる葉の crate（案 B）・テスト専用の葉への組み手・`crates/areka` の突き合わせのテスト・wintf の新しい Ext トレイト・wintf の検体のフォルダの README・1 ビットの検体。どれも要件を満たすのに要らない。

### 9.4 危険と手当て

- **2 つの写しの食い違い**: 同じ検体・同じ期待値の表を 2 つの crate のテストが持つことで見張る。作り手はいつも両方のフォルダを書く。
- **集合に無い α つきの形式**（例 `16bppBGRA5551`・`64bppRGBAHalf`）は今も `has_alpha` が偽で、抜き色の腕へ α<255 の画素が届きうる。要件 2.3 で今のまま（範囲外）。§8 の 274 行目の言い換えは、この形の絵が来ても成り立つ書き方にした。
- **§8 の行の位置**: 同じウェーブのほかの spec が §8 に行を足しても、本 spec は既存の 2 行をその場で書き直すだけなので競合しない。行番号は着地のときに引き直す。
