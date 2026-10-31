# 設計の検証: emo-atlas-palette-alpha

> 2026-10-31・`kiro-validate-design`（対話なし）。読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・`.kiro/steering/structure.md`（テストの置き方）。今のコードで確かめた所: `crates/areka-emo-atlas/src/decode/wic_arm.rs`（79 行目の判定・166〜169 行目の写しの説明・末尾の接続宣言）・`decode/wic_arm_tests.rs`（`with_arm`）・`decode/sniff.rs`（GIF は WIC の腕へ来る）・`normalize.rs`（79・82 行目の `has_alpha` の 2 か所）・`emo2_e2e.rs`（定数 `SHELL_KEY_COLOR_KEY` の 9 か所）・`normalize_key_color_tests.rs`・`normalize_rule_tests.rs`・`crates/wintf/src/com/wic.rs`（51 行目）・`crates/wintf/tests/com/wic_test.rs`（`decode_first_frame`）・`doc/COMPAT_ARCHITECTURE.md` §8 の 273・274 行目。

## 設計の検証の要約

判定の直しは 2 つの読み手のそれぞれ 1 行を「番号つきの形式ならパレットに聞く」枝に置き換えるだけで、正規化・抜き色・変換・動く絵の読み手に触らずに要件 1〜4 を満たす。行番号・関数名・定数名・§8 の行は今の木と一致し、WIC の振る舞い（番号つきの形式・`HasAlpha`・番号つきでない絵での `WINCODEC_ERR_PALETTEUNAVAILABLE`）は research.md §9.1 で実物から測ってある。直しの本体は最小で正しいが、テストの段取りに、もっと小さく済む所が 2 つある。

## 重要な指摘（2 件）

🔴 **指摘 1: 検体 6 本を 2 つの crate に二重に持ち、emo-atlas の作り手が wintf の木へ書く**
**懸念**: 同じ 6 本を `crates/areka-emo-atlas/src/testdata/palette/` と `crates/wintf/tests/assets/palette/` の両方にコミットし、emo-atlas の example が wintf のフォルダへも書く。2 本が同じであることは「作り手がいつも両方を書く」という約束だけで守られ、片方だけ手で直されても気付く檻が無い。README も片方にしか無い。
**影響**: 要件 3.5（同じ絵に同じ答え）の根拠が「同じバイト列」なのに、それを見張るものが無い。持ち物が 2 倍になり、作り手が別の crate の木を書くという前例の無い形が増える（`gen_animated_samples.rs` は自分の crate の 1 か所だけに書く）。
**提案**: 検体は 1 か所（作り手と README と同じ `crates/areka-emo-atlas/src/testdata/palette/`）にだけ置き、wintf のテストは `env!("CARGO_MANIFEST_DIR")` から `../areka-emo-atlas/src/testdata/palette/` を引く。別の crate のファイルを相対で読む前例は在る（`crates/areka/src/tick_gate_config_producers_tests.rs` が `../wintf/src/…` を読む）。「emo-atlas は wintf を参照しない」は crate の依存の決めで、検体のファイルを読むことは妨げない。こうすると 2 つのテストが文字どおり同じファイルを読むので要件 3.5 が強くなり、作り手は自分の crate にだけ書けばよい。逆向き（wintf の `tests/assets/palette/` に 1 か所）でもよい。
**対応する要件**: 3.5・5.7
**根拠の場所**: design.md「File Structure Plan」・「PaletteSamples」の Batch / Job Contract（出力・README）・「設計の決め」3・「Revalidation Triggers」2 つ目

🔴 **指摘 2: テストのためだけに wintf の `palette_has_alpha` を `pub` にする**
**懸念**: 要件 4.2 の失敗の道を `tests/com/wic_test.rs`（結合テスト）から呼ぶため、wintf の公開の関数を 1 つ増やす。本番の使い手は無い。
**影響**: テストの都合で wintf の公開の面が広がり、emo-atlas 側（非公開のまま子のテストのモジュールから呼ぶ）と形がそろわない。後から本番のコードがこの関数を使い始めても止める物が無い。
**提案**: 関数は非公開のままにし、失敗の道のテストだけを兄弟のファイル `crates/wintf/src/com/wic_tests.rs`（`wic.rs` の末尾に `#[cfg(test)] #[path = "wic_tests.rs"] mod tests;`）に置く。wintf には `src/` の兄弟のテストの前例が多数在る（`ecs/cursor/cur_file_tests.rs` など）。判定の表のテスト（`load_bitmap_source` の 2 つ目の戻り値）は公開の関数で書けるので、roadmap が名指しした `tests/com/wic_test.rs` に置いたままでよい。emo-atlas の失敗の道のテストと同じ形になる。
**対応する要件**: 4.2・5.5
**根拠の場所**: design.md「WintfJudgment」の責務と決まり（`pub` にする理由）・Service Interface・「設計の決め」4・「Integration Tests」の失敗の道

## 設計の良い所

- **直しの場所を判定の入口 1 か所に絞った**: `has_alpha` が正しくなれば `Normalizer::plan` の今の 2 行（`On if has_alpha`・`Full => !has_alpha`）がそのまま要件 1.2〜1.4・2.4・2.5 を満たすことを、今のコードと突き合わせて示している。番号つきの 4 個の集合で入口を決め、その枝でだけパレットを読み、失敗は `?` で今の `DecodeError::Decode { path }` へつなぐので、「読めない」を「無い」と推測する道も、記録の無い失敗の道も生まれない。
- **検体が 1 画素で違いを読める形**: どの検体も左上が不透明で同じ色の画素が (1,0) に在るので、抜き色の道と α の道の違いが 1 画素で決まる。出る画素の期待値は実物の WIC から測った値で、直す前に赤・後に緑になるテストの区分けも表になっている。テストは GPU・実機・排他の机を要らず、走る間にファイルを書かない。

## 小さな点（議題にしない）

- `wic_arm_test_support.rs` への `with_arm` の移しは、structure.md の「テーマ間で共有するヘルパは `<stem>_test_support.rs` へ集約する（1 件でも）」の決まりどおりで、要る。
- 定数名の `SHELL_KEY_COLOR_KEY` → `SHELL_NULL_PNG_KEY` は同じファイルの中の 9 か所だけで、本 spec の後は今の名前が事実と食い違うので、要件 2.7 の範囲として妥当。
- `wic_arm.rs` には昔からの `#[cfg(test)] mod tests { … }` が本番のファイルの中に在る（188 行目〜）。本 spec は新しい本体を足さないので、そのまま触らなくてよい。

## 最終の判断

**判断: GO**

**理由**: 本番のコードの直しは 2 か所の 1 行の置き換えと小さな関数 1 つずつで、要件のすべてに当たる部品と確かめ方が在り、今の木と食い違う記述も無い。2 つの指摘はどちらもテストの段取りを小さくする話で、直しの形は変わらない。

**次の段**: 設計の討議（`kiro-design-discussion`）で指摘 1・2 を議題にし、決まったら design.md を直してから `/kiro-spec-tasks emo-atlas-palette-alpha` へ進む。
