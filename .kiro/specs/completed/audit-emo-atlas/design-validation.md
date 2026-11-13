# 設計の検証: audit-emo-atlas（絵の焼き込みとアトラスの見直し）

- 検証した日: 2026-11-13
- 対象: `design.md`（確定版）・`requirements.md`・`research.md`・`spec.json`（`language: ja`・`phase: design-generated`）
- 照らした steering: `product.md`・`tech.md`・`structure.md`・`roadmap.md` の C19 の行
- 進め方: `kiro-validate-design` の手順（分析 → 重い問題 → よい所 → GO／NO-GO）。対話なしで判定まで出す。

## 1. まとめ

設計は、確定した 22 件の全部に直し方と、壊れたら落ちるテスト（またはテストを置かない理由）を付けている。C19 の約束（`crates/areka/`・`crates/areka-emo-text/`・`crates/areka-placement/`・`vendors/` に触らない、どの `Cargo.toml` と `Cargo.lock` も変えない、公開の形は指摘 22 のほかは減らさない）も、実際のコードを grep して守れることを確かめた。実装へ進めてよい。重い問題は 2 件で、どちらも設計の小さな書き足しで片づく。

## 2. 分析（コードと突き合わせた結果）

### (a) 22 件の直し方とテスト

| 指摘 | 直し方 | 壊れたら落ちるテスト・置かない理由 | 判定 |
|---|---|---|---|
| 1 | `resolve_indirect`・`by_id`・`visited` を消す | 5 万面の参照の鎖を 256 KiB のスタックで `derive` | 可。`manifest.rs:53-128` を読み、参照先はどれも同じセットの `set.surfaces` の面で、その絵は直に数える回し方で入っていることを確かめた（消しても結果は同じ） |
| 2 | `MAX_SIDE`＝16,384・`pixel_len`・`check_image`・WIC の `GetSize` の直後に確かめ・`trim.rs`／`bake.rs` の掛け算を `usize` に | WIC の 16,385×1 と 16,384×1・`pixel_len` の純な関数・動く絵の見出し・`MemoryDecoder` の静止画 | 可。`normalize.rs` はもう `usize` で掛けている |
| 3 | 頁に入らない絵をその絵だけの頁に・`PackOutput.pages: Vec<Size>`・`bake_pages` から `page_size` を消す | 1×2,100 の絵で頁が 3×2,102・`Packer` の単体 | 可。下流（`areka-emo-compose` の `blit.rs:134-190`・本体の `balloon_background.rs:50-56`）は `AtlasPage` の `width`・`height`・`stride` で引いており、正方でない頁でも壊れない。GPU へ頁をそのまま上げる所は無い（CPU の写しだけ） |
| 4・10 | `src/test_support.rs` に 1 つ | 部品の直し（理由あり） | 可。`test_support.rs` の名は `areka-placement` ほか 4 クレートに前例がある |
| 5 | `cli.rs` へ移し、ずれの見張りが `#[path]` で読む | 移した 4 本が走る | 可。4 本が `Mode`・`parse_args`・`first_difference`・`write_replacing`・`io_text` を全部使うので、見張りのターゲットで使われない項目は出ない |
| 6 | ⑴〜⑸ の 5 本 | テストそのものが直し | 可。⑷ は WIC の答えが走らせないと分からない点を、表から項を外して赤を確かめる手順で押さえている |
| 7 | `bake` と `bake_with_limits(.., from_env())` の比べ | 環境に依らない 1 本 | 可 |
| 8 | 弱いテストの直しと `with_frames` の `frames[0]` の確かめ | `frames[0]` が親でない並びで panic | 可。範囲の外の `with_frames` の呼び手は `film_tests.rs:231` の 1 か所だけ |
| 9 | 後ろ（重い側）を消す | 消す側だけ（理由あり） | 可 |
| 11 | `new` を `with_frames` の 1 行に | 同じ鍵の `resolve` の比べ | 可。範囲の外の `AtlasTable::new` の呼び手 18 か所は、空の表か鍵の重ならない表だけ。`new` の panic の文を `expected` で比べるテストは 0 |
| 12 | 面積の下限から頁の数を試す | `first_bins` の 1 本・今のテストと golden がそのまま緑 | 可。下限より少ない頁では必ず入らず、`pack_rects` は試すたびに新しい頁で組むので、最初に入る頁の数と配置は今と同じ |
| 13 | `areka-parsers` の `shell::pattern_word` へ | 今のテストが守る（理由あり） | 条件付きで可（下の重い問題 1） |
| 14・15・16・17・19・20・21 | 説明・小さなコード・`Arc`・道具の写し・兄弟のファイル・名の渡し方・文に依る比べ | どれも理由あり（17 は出力のバイト列の差分 0 を走らせて確かめる） | 可 |
| 18 | `check_image` を `load` の 3 か所（1 枚読み・縮めた 1 枚・全コマ）に | 長さの足りない 2×2 の絵で panic しない | 可。全部の読みが `animated::load` を通ってから正規化へ渡るので、正規化と切り詰めの手前で止まる |
| 22 | 9 つの型を `pub(crate)` へ | 全体のビルドで確かめる（理由あり） | 条件付きで可（下の重い問題 2） |

### (b) C19 の約束と範囲の外の呼び手

- **9 つの型の使い手**: ツリー全体を grep した。範囲の外の当たりは `areka-emo-present` の説明の文 2 か所（`shell_target.rs:531`・`shell_target_base_image_tests.rs:73`）だけで、コードの使い手は 0。`areka_emo_atlas::pack::` ほかモジュールのパスで引く所も 0。残る公開の関数の引数と戻り値に、下げる型は出てこない（下げた型どうしの中で閉じる）。
- **`bake_pages`・`PackOutput`・`page_count`**: 範囲の外の当たりは 0。
- **`ElementDecoder` の実装**: 範囲の外は 3 つ（本体 `measure_bake_count_tests.rs:71` の `CountingDecoder`・`areka-placement` の `FirstFrameDecoder` と `Counting`）。どれも中の読み手へそのまま渡す包みで、長さの約束を変えない。
- **`MemoryDecoder::insert` の約束（`stride >= width * 4`・`bgra.len() == stride * height`）**: 設計は「指摘 18 のタスクで grep」と後へ回しているが、この検証で先に確かめた。本体 `crates/areka/` の 3 ファイル（`balloon_background_tests.rs:95,257`・`balloon_face_tests.rs:48-49`・`measure_bake_log_tests.rs:145-186`）は全部が約束の内。`areka-emo-present`・`areka-emo-compose`・`areka-emo-text`・`areka-placement` も、数字で書いた呼び出しに約束を破るものは 0、行の詰め物つきの `balloon_marker_load_tests.rs:198`（幅 2・stride 12・24 バイト）も約束の内。本体に触らずに済む。
- **`Cargo.toml`・`Cargo.lock`**: 指摘 5 を関数の移しにし、指摘 13 は parsers に元から依存しているクレートだけが使うので、変更は 0。`areka-emo-atlas/Cargo.toml:13` と `areka-emo-compose/Cargo.toml:13` に `areka-parsers` があることを確かめた。
- **C19 のほかの 7 本と触るファイル**: roadmap の C19 の行の触るファイルに `areka-parsers` と `areka-emo-compose` は無い。

### (c) 指摘 13 の公開のパス

`areka-emo-compose/src/lib.rs:51` の `pub use pattern_word::{PatternWord, UnreadableTarget, call_targets, pattern_word};` は変えず、`pattern_word.rs` の中を parsers の再公開にするので、`areka_emo_compose::PatternWord`・`pattern_word` と `areka_emo_compose::pattern_word::…` のどちらのパスも残る。使い手 `areka-seriko/src/table_calls.rs:14` はそのままビルドできる。compose の中の `canonical_method_name` は今 `pub(crate)`（`method.rs:139`）で、parsers で `pub` になるのは公開の形を足すだけ。

### (d) 1 ファイル 1,000 行の番人

`lib.rs`（873 行）は中のテスト約 500 行を移して約 370 行に減る。`animated_tests.rs`（838 行）には足さず減るだけ。新しいテストは `bake_guard_tests.rs` ほか兄弟のファイルへ置く。ほかの触るファイルはどれも 650 行以下（`manifest.rs` 622・`pack.rs` 473・`decode.rs` 380）で、足す行は数十行。番人の例外表に足すファイルは無い。

### (e) 失敗の記録（`error!`＋`Err`）

新しい失敗（一辺の上限を越える絵・約束を破る読み手）は今の `DecodeError::Decode` と同じ形で `BakeResult::errors` に入る。本番の焼きの呼び手は `areka-emo-present` の 2 か所（`shell_target.rs:622`・`balloon.rs:807`）だけで、どちらも `errors` を `error!` で出す（`shell_target.rs:477`・`balloon.rs:811`）。記録の無い失敗の道は増えない。消す `error!`（`pack.rs:103-112`）は、正しい入力として自頁に載せるようになって起きなくなる道なので、消してよい。

### (f) タスクの見込みと要件 8

見込みは 22 タスクで 20 を越えるが、要件 8.1 の討議で「切らずに 1 本・見込み 20〜23」と確定しており、設計の見込みはその範囲の中。並べる順（22 を 3 の前・2 を 3 の前・部品を先に）も筋が通る。

## 3. 重い問題（2 件）

### 重い問題 1: 指摘 13 の後も、描画メソッドの綴りの比べ方が 2 つのクレートに分かれて残る

- **問題**: 設計は「綴りの比べ方も parsers の 1 つにする…写しが残らない」と書くが、`areka-emo-compose/src/method.rs:199-205` の `same_canonical`（確保しないで同じ規則を当てる関数・`is_implemented_name` が使う）は「compose の中の仕組みなので触らない」として残す。直した後は、規則の正本が parsers の `canonical_method_name`、その写しが compose の `same_canonical` になり、両者が同じ答えを返すことを確かめるテストは今も無い（compose の `_tests.rs` に当たり 0）。
- **影響**: 議題 3 の結論は「写しのまま説明だけ足す案は、食い違いの種を残すので取らない」。同じ種が、今度はクレートをまたいで残る。parsers の規則を直したとき、compose の「描かれるか」の判定だけが古い綴りのまま動く。アトラスの範囲の外なので、直した後の audit のかけ直しでも見つからない。
- **推す直し**: `method.rs` は指摘 13 で開くので、`same_canonical` を parsers の `pattern_word.rs` へ `canonical_method_name` と並べて移し（6 行・公開の形に足すだけ）、compose はそれを取り込む。移さないなら、少なくとも compose に「いくつかの綴りで `same_canonical(n, c) == (canonical_method_name(n) == c)`」の 1 本を置き、設計の「写しが残らない」の文を事実に合わせる。
- **要件**: 2.1・5.4（範囲の外は直しに要る箇所だけ）・討議の議題 3
- **根拠の場所**: design.md「指摘 13: 8 語の表を `areka-parsers` へまとめる」の 1 つ目と 3 つ目の箇条・File Structure Plan の「範囲の外」の `method.rs` の行

### 重い問題 2: 指摘 22 の場所に挙がっている `pub mod` の 8 つを、設計は「指摘に挙がっていない」として残す

- **問題**: `research.md` 3 節の指摘 22 は、場所を `lib.rs:24-32,321,330,333,336` とし、「`pub mod` の 8 つと再公開のうち…」と書く。設計は型 9 つだけを下げ、「`pub mod` の 8 つのモジュールはそのまま（指摘の範囲の外・…指摘に挙がっていない）」とする。モジュールのパスで引く範囲の外の使い手は 0（この検証で grep した）なので、残すと `areka_emo_atlas::pack::PackConfig` のような 2 本目のパスと、`pub mod` の中の中身の無い入口が残る。
- **影響**: 直した後の audit のかけ直し（要件 4.1・4.2）が同じ場所をもう一度指す見込みが高い。そのときは公開の形を減らす直しになり、要件 6.3 の議題（`runtime-module-untangle` と申し合わせるか C20 へ回すか）が実装の終わり際に出て、着地が遅れる。
- **推す直し**: 設計の段で決めておく。推すのは、指摘 22 のタスクで、中の公開の項目が全部ルートで再公開されているモジュール（`pack`・`bake`・`trim`・`manifest` ほか）を `pub mod` から `mod` へ下げること（要件 6.8 と同じく、直す前に grep で使い手 0 を確かめ直し、下げるモジュールの名も `runtime-module-untangle` へ知らせる）。下げると、ルートで再公開していない `pub` の項目が中でも使われていなければ `dead_code` の警告が出るので、そのタスクで警告 0 を確かめる。残すと決めるなら、残す理由（例: モジュールのパスも公開の形として保つ）を設計に 1 行書き、かけ直しの audit でこの点を新しい指摘に数えない根拠にする。
- **要件**: 4.1・4.2・6.2・6.3・6.8
- **根拠の場所**: design.md「指摘 22: 使い手の無い公開の型を `pub(crate)` へ」の 2 つ目の箇条・research.md 3 節の指摘 22

## 4. よい所

1. **範囲の外への波及を、名前と行で押さえている**: 下げる型・変わる欄・新しい panic・正方でない頁・`MemoryDecoder` の約束のそれぞれに、範囲の外の呼び手を名指しして壊れない理由を書いている（Revalidation Triggers）。この検証で grep し直しても、書かれたとおりだった（`with_frames` の外の呼び手 1 か所・`AtlasTable::new` の 18 か所・`AtlasPage` の欄で引く下流 2 か所）。
2. **「壊れたら落ちる」を、直す前に赤になる形で 1 件ずつ書いている**: Testing Strategy の表が、指摘 1・2・3・6⑶・8・11・18 のそれぞれに「直す前の結果」を書き、WIC の振る舞いが走らせないと分からない所（指摘 2 の巨大な見出し・6⑷ の 1・2 ビット）は、それに依らない大きさの検体や、表から項を外して赤を確かめる手順で押さえている。普通の大きさの絵の焼き上がりが 1 バイトも変わらないこと（golden・`multi_page_split`・`bake_quiet` と `bake` の一致）も確かめの道に入っている。

## 5. 判定

- **判定: GO**
- **理由**: 22 件の全部に直し方とテスト（または理由）があり、C19 の約束と範囲の外の呼び手は実際のコードで守れることを確かめた。重い問題の 2 件は、どちらも設計の書き足し（指摘 13 の綴りの比べ方の置き場・指摘 22 の `pub mod` の扱い）で片づき、作りの向きを変えるものではない。
- **残る危うさ（判定を変えないもの）**: 自頁の絵は焼いた後も頁として持ち続けるので、一辺が上限近い絵が何枚もあるシェルでは、その枚数ぶん（1 枚約 1 GiB）メモリを持つ。討議の議題 1・2 で「画素の合計には上限をかけない・一辺は上限まで」と決まった範囲の中の重さで、この検証では議題に戻さない。
- **次の段**: 設計の討議（`/kiro-design-discussion audit-emo-atlas`）で重い問題 1・2 を議題にし、決めた形を `design.md` に書き足してから、`/kiro-spec-tasks audit-emo-atlas` でタスクを作る。
