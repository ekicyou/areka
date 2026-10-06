# 設計の検証: areka-P0-self-alpha-declaration

- 作成: 2026-10-05（`/kiro-validate-design`・対話なし）
- 対象: `design.md`（`requirements.md`・`research.md`・steering と突き合わせ）
- 調べ方: ソースを読んだだけ（ビルド・実行はしていない）。設計が既存コードについて言っていることのうち、設計を支えているものは自分で読み直した
- 2026-10-05 の開発者の裁定（明示の値は正典どおり・明示の `0` は α を切って残りの色で抜く・宣言なしは α を使う・`.pna` は無視・descript.txt の無いシェルは範囲外）は決まったこととして扱い、蒸し返していない

## 判定

**GO（条件つき）**。組み方は「今ある部品を広げるだけ」で、足すより消すほうが多い。作り過ぎは見つからなかった。下の 3 件はどれも設計の骨組みを変えずに直せるので、設計ディスカッションで扱えば足りる。

## まとめ

透過の決定を「1 枚目で決める扱い（`AlphaRule`）」1 つにまとめ、静止画にも動く絵の全部のコマにも同じ関数で当てる形は、今のコードの形（`normalize.rs` の `Normalizer::normalize`・`animated.rs` の `PendingFrames::new`）に無理なく乗る。`1` の経路が今と 1 バイトも変わらないこと、消す型の使い手が外に居ないこと、画像だけのシェルが入口 1 か所で届くことは、コードで確かめられた。足りないのは、⑴ 宣言なしの新しい決まりで赤くなる既存テストの数え漏れ、⑵ シェルの descript.txt の文字コードの扱いが書かれていないこと、⑶ 宣言なしの判定に読み手の `has_alpha` を挟んでいることで静止画と動く絵の扱いがずれうること、の 3 件である。

## コードで確かめたこと

| 設計の言い分 | 結果 | 見た場所 |
|---|---|---|
| `NormalizeError`・`AlphaSource`・`BakeError::Normalize`・`Normalizer::key_color` の使い手は `areka-emo-atlas` の外に 0 件 | 正しい | `crates/` 全体を検索。外のクレートに在るのは `balloon.rs` 冒頭の `probe_pna` の説明文だけ |
| `BakeResult` を組み立てる・分解する場所は `areka-emo-atlas` の中だけ | 正しい | `lib.rs` の `bake_with_limits` の末尾と、同ファイルのテスト 1 か所 |
| `ShellTarget` を構造体の形で組むのは `build_shell_target_with_boxes` だけ（欄を足しても外は壊れない） | 正しい | `shell_target.rs` |
| `1` の 2 つの枝は今と同じ処理 | 正しい | 今の `Normalizer::normalize` は「`has_alpha` が真ならバッファをそのまま渡す／偽なら左上の 4 バイトを `clear_key_color`」。設計の表の `On` の 2 行と同じ。動く絵の 2 枚目以降も、今は「1 枚目の抜き色を消すだけ」で、`AlphaRule { opaque: false, key }` と同じ |
| 絵は乗算済みで届く | 正しい | `decode/wic_arm.rs` の `WicDecoderArm::decode_inner`（`32bppPBGRA` へ変換）と `decode/image_arm.rs` の `to_bgra`（自前で乗算） |
| α を 255 に書いてから左上の 4 バイトで抜く、が動く絵にも通る | 通る | 2 枚目以降のコマは `PendingFrames::new` の中で 1 コマずつ `clear_key_color` を通っている。同じ場所で `apply` に替えれば足りる。コマは `image` クレートが重ね済みで返すので、コマごとに α を 255 にしても前のコマとの重ねは壊れない |
| `surfaces.txt` を本番で読むのは `load_shell_target` だけ | 正しい | `crates/` の本番コードで `surfaces.txt` を綴るのは `shell_target.rs` の定数 1 つ（`areka-seriko` の `resolve.rs` はテストの中） |
| 切り替えは `build_shell_assets`／`build_balloon_assets` を呼び直す。バルーンはスコープのループで焼く | 正しい | `emo2_boot/assets.rs` の `build_balloon_assets`、`placement/measure.rs` の `measure_native_scope_sizes` → `measure_balloon_surface0` |
| 当たり判定は α が 128 以上を当たりにする | 正しい | `wintf` の `alpha_mask.rs` の `pack_pbgra32_alpha`（`alpha >= ALPHA_THRESHOLD`・コメント「閾値128」） |
| `parse_kv` は後の行が勝つ・前後の空白を除く | 正しい | `areka-parsers` の `kv/parse.rs` の `parse_kv`（キーの大小は区別する。値の大小を区別しないのは設計の `parse` の仕事） |
| 「全画素が不透明か」の走査が見るもの | 設計の読みどおり。ただし 3 件目の論点あり | WIC が α なしと答える絵は `has_alpha` が偽で届くので、設計の表では走査の前に抜き色の枝へ落ちる |

## 重要な論点（3 件）

### 1. 宣言なしの新しい決まりで赤くなる既存テストが数えられていない

- **何が起きるか**: 設計は、シェルの入口 `load_shell_target` が自分で descript.txt を読み、バルーンの包み `build_balloon_target` が中で `load_balloon_use_self_alpha` を呼ぶ、としている。descript.txt を置かない一時フォルダを使う既存のテストは、これで「宣言なし」になる。それらのテストの絵は、ほとんどが「`has_alpha` が真・全画素不透明・1 色」である。宣言なしの表ではこれは抜き色の枝に当たり、左上と同じ色＝絵の全部が透明になる。
- **確かめた例**: `areka-emo-present` の `balloon_target_tests.rs` の `build_balloon_target_end_to_end_frames_only`。絵は `opaque_1x1()`＝`(1, 1, 4, [10, 20, 30, 255], true)`、フォルダに descript.txt は無く、「不透明ゆえ placement を持つ」を表明している。設計どおりに実装すると絵が全部抜かれて placement が無くなり、赤になる。同じ `opaque_1x1()` は `shell_target_load_tests.rs` にも在り、`load_shell_target` を一時フォルダで呼ぶテスト（`directories_are_not_taken_as_surface_images`・`every_record_is_emitted_once_per_load` など）が通る。`shell_target_boxes_tests.rs`・`shell_target_nesting_tests.rs` も一時フォルダとメモリ上の復号器を使っている。
- **設計の今の書き方**: 「書き換え・削除する既存のテスト」と申し送り 7 は、「`warn!` が 1 行増える」ことしか挙げていない。「約 40 のテストファイルは無変更」は `AlphaParams` を直に組むテストについては正しいが、入口を通るテストはこの数に入っていない。
- **直し方（どれも小さい）**: タスクに「入口を通る一時フォルダのテストの棚卸し」を 1 つ足し、直し方を先に決めておく。候補は、⑴ それらのテストのフォルダに `seriko.use_self_alpha,1`／`use_self_alpha,1` と書いた descript.txt を置く（今と同じ `1` の経路を見続ける。申し送り 7 の `warn!` 1 行も同時に消える）、⑵ 絵の左上だけ別の色にする、のどちらか。⑴ のほうが「このテストは `1` の経路を見ている」ことが読み取れてよい。
- **補足**: `areka` クレートの側の一時フォルダの検体は、同梱の えも？？ の PNG（透明な画素を持つ）を写して使っているので、宣言なしでも α の枝に当たり、変わらない見込み（`placement_shared_test_support.rs` を見た範囲）。

### 2. シェルの descript.txt をどの文字コードで読むかが書かれていない

- **何が起きるか**: 設計は「入口は呼ばれるたびに `shell_dir/descript.txt` を読む」「読めないときは `warn!` を出して宣言なしで続ける」とだけ書いている。バルーンの側は、文字コードつきで読む既存の `read_descript_layer` を通すと明記してあるが、シェルの側には読み方の指定が無い。実装が `std::fs::read_to_string` を選ぶと、Shift_JIS で書かれた descript.txt（昔からのシェルでは普通）は UTF-8 として読めずに失敗し、`seriko.use_self_alpha,0` や `full` と書いてあっても宣言なしとして描かれる。本仕様がいちばん助けたい「昔からの資産」で宣言が届かなくなる。
- **今のコードの前例**: `areka` の `emo2_boot/assets.rs` の `build_shell_assets` は、同じファイルを `decode(&bytes, DefaultEncoding::Ansi)` → `parse_kv` で読んでいる。`shell_target.rs` も `surfaces.txt` を同じ `decode` で読んでおり、`areka_parsers::charset::{DefaultEncoding, decode}` は既に取り込み済みである。
- **直し方**: 設計の「ShellTarget 入口」に 1 行足す — 「`std::fs::read` で読み、`areka_parsers::charset::decode(&bytes, DefaultEncoding::Ansi)` で文字列にする（`surfaces.txt` と同じ）」。テスト（`shell_target_image_only_tests.rs`）に、Shift_JIS の日本語を含む descript.txt で宣言が読めることを 1 本足す。

### 3. 宣言なしの判定に `has_alpha` を挟むと、同じ種類の絵が静止画と動く絵で別の扱いになりうる

- **何が起きるか**: 設計の表は、宣言なしのとき「`has_alpha` が真 **かつ** α<255 の画素が 1 つ以上」なら α、ほかは抜き色、としている。`has_alpha` の出どころは読み手ごとに違う。静止画は WIC の変換前の画素形式の一覧（`wic_arm.rs` の `pixel_format_has_alpha`・索引つきの形式は載っていない）、動く絵は `image` クレートの色の形式（`image_arm.rs` の `open` の `png.color_type().has_alpha()`）である。パレットの PNG に透明の情報が付いた絵は、静止画では `has_alpha` が偽（設計も「見込み」と書いている）、動く絵では `image` が RGBA に広げて返すので真になる見込みで、同じ作り方の絵が静止画なら「透明な所はそのまま＋左上の色も抜く」、動く絵なら「α のまま」に分かれる。静止画の側は、要件 9 の 1（透明な画素を含む絵は α をそのまま使う）の文面からも外れる。
- **直し方（条件が 1 つ減る）**: 宣言なしの 2 行から `has_alpha` を外し、「α<255 の画素が 1 つ以上あれば α、無ければ抜き色」だけにする。α を持たない絵は全画素 α=255 で届く（WIC は「α 無し画像も 100% 不透明として埋める」、動く絵は RGBA の器に入る）ので、要件 9 の 3 と要件 8 の 2（宣言なし × α なし ＝ `1` × α なし）はそのまま成り立つ。読み手の答えに依らなくなるので、research 9.5 の「実物で確かめていない」という残りのリスクも宣言なしについては消える。開発者の裁定「透明な画素を持つ絵は α、持たない絵は抜き色」にも、こちらのほうが字面どおりに沿う。
- **変えない所**: `1` の 2 行は今のまま `has_alpha` で分ける（今と 1 バイトも変えないため）。読み手も変えない。
- **採らない場合**: 今の表のままでも手持ちの検体には影響しない（宣言なしの 2 体は透明の情報を持たない）。その場合は、静止画と動く絵で分かれうることを設計のリスクの欄に書いておく。

## 良い点

1. **足すより消すほうが多い**。`AlphaRule`（2 欄）と `plan`／`apply` にまとめたことで、`AlphaSource`・`NormalizeError`・`select_source`・`key_color`・`BakeError::Normalize` と「正規化で落ちた絵」の枝が無くなる。動く絵の 2 枚目以降に別の引数を足さずに済み、決める場所が 1 つになる。新しい型・層・外部クレートは無い。開発者の方針（今ある部品を広げる・作り過ぎない）に合っている。
2. **読み手の答えに依らない形にしてある**。`full` × α なしと `0` は「α を 255 に書く」ので、パレットの PNG の届き方を実物で確かめていなくても要件 4 の 2・5 の 6 が成り立つ。確かめられなかったことを設計の側で吸収しており、3 件目の直しを入れると宣言なしも同じ性質になる。
3. **シェルの入口の署名を変えない**。`load_shell_target` が自分で宣言を読むので、起動・切り替え・採寸・examples の呼び手が無変更で、起動と採寸が別々の値を持つ余地が無い。

## 小さい気付き（論点にはしない）

- 「Modified Files」は核の呼び出しの追随先に `shell_target_base_image_tests.rs`・`shell_target_boxes_tests.rs` を挙げているが、`shell_target_load_tests.rs` にも `build_shell_target` を直に呼ぶ所が 1 つ在る。
- `load_balloon_use_self_alpha` は名前に反して `.pna` の数え上げと記録も受け持つ。関数の説明に書いておけば足りる。
- `.pna` の数え方がシェル（焼いた絵に添えてあった数）とバルーン（フォルダ直下のファイルの数）で違う点は、設計の申し送り 2 に在るとおり。記録 1 行のための仕組みが 2 つになるが、要件 5 の 7 が「枚数とともに 1 度だけ」を求めているので妥当な大きさである。
- 要件の文面に裁定の前の言い回し（「既定の `0`」）が残っている点は、設計の申し送り 6 が既に挙げている。設計の中身は裁定の後の文面に沿っている。

## 次の一歩

- 設計ディスカッション（`/kiro-design-discussion areka-P0-self-alpha-declaration`）で上の 3 件を扱う。1 はタスクへの追加、2 は設計への 1 行の追記、3 は表の 2 行の書き換え（または採らずにリスクとして記す）で済む。
- その後 `/kiro-spec-tasks areka-P0-self-alpha-declaration` へ進める。
