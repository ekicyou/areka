# Implementation Plan

> 2026-10-31・`design.md`（設計の討議の決め込み）から組んだ。行番号は設計の日の木のもの。着手時に引き直すこと。赤を先に立てる順（設計の Testing Strategy）に従い、検体 → 赤のテスト → 直し → 説明と文書 → 全体の確かめ、の順に並べる。

- [x] 1. 土台: 検体と共有のテストの手伝い
- [x] 1.1 パレットの検体 6 本を作る作り手と、コミットする検体を用意する
  - std だけで PNG（`IHDR`・`PLTE`・`tRNS`・無圧縮の zlib の `IDAT`・`IEND`）と GIF89a（色の表 4 色・透明の番号 1）を組む作り手を、emo-atlas の example として足す
  - 設計の Data Models の表どおりの 2×2 の検体 6 本（`idx8_mixed`・`idx4_mixed`・`idx8_plain`・`idx8_opaque_trns`・`gray8_trns`・`trans.gif`）を emo-atlas の `src/testdata/palette/` の 1 か所へだけ書く。どれも左上が不透明で、(1,0) が左上と同じ色
  - 同じフォルダに README（作り直しの手順・中身の表・第三者の著作物 0 件・wintf のテストもこの検体を読むこと）を置く
  - 作り手を 2 回走らせてもバイト列が同じで、6 本と README がコミットされている
  - _Requirements: 3.5, 5.4, 5.7, 5.8_
- [x] 1.2 emo-atlas の WIC の腕のテストが共有する COM の張り方を兄弟の手伝いへ移す
  - 今の動く絵のテストが持つ「COM を張って腕を組む」手伝いを、structure.md の決まりどおり共有の兄弟ファイルへ移し、読み手のファイルの末尾からテストの時だけつなぐ
  - 動く絵のテストの中身は変えず、移した手伝いを使う形にだけ直す
  - emo-atlas の今のテストが全部、前と同じ数で緑
  - _Requirements: 5.6_

- [x] 2. 赤のテストを先に立てる
- [x] 2.1 (P) emo-atlas でパレットの絵の判定と焼き込みの画素を確かめるテストを足し、直す前に赤を記録する
  - 6 本の検体を読み手で読み、α の有無が設計の表の「後」（真 4 本・偽 2 本）と一致することを確かめる
  - 透明度を持つ 4 本を `1`／`true` で正規化に通し、左上と同じ色の (1,0) が不透明のまま・透明の番号が全 0・半透明の番号が 128 のまま残ることを読み戻した画素で確かめる
  - 8 ビットと 4 ビットのパレットの絵を `full` で通し、透明の番号が透明のまま・半透明の番号が半透明のまま残ることを確かめる
  - 透明度を持たない絵と全部 255 の絵を `1`／`true` で通し、左上の色が今のまま抜かれることを確かめる（直す前も後も緑）
  - テストは `#[ignore]` を付けず、走る間にファイルを書かない
  - 読み手のファイルにはテストの時だけ新しい兄弟をつなぐ宣言 1 行だけを足し、判定には触らない（判定の直しは 3.1）
  - 直す前に走らせ、判定の表の真の 4 本・`1`／`true` の 4 本・`full` の 2 本が赤、透明度を持たない 2 本が緑であることを記録に残す
  - _Boundary: AtlasPaletteTests_
  - _Depends: 1.1, 1.2_
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 5.1, 5.2, 5.3, 5.4, 5.6, 5.8_
- [x] 2.2 (P) wintf の絵の読み込みの α の有無の表のテストを足し、直す前に赤を記録する
  - emo-atlas の検体 6 本を `CARGO_MANIFEST_DIR` からの相対パスで読み（crate の依存は足さない）、wintf の読み込みが返す α の有無が emo-atlas と同じ表と一致することを確かめる
  - テストの説明から作り手と README を名指しする
  - 直す前に走らせ、真の 4 本が赤、偽の 2 本が緑であることを記録に残す
  - _Boundary: WintfPaletteTests_
  - _Depends: 1.1_
  - _Requirements: 3.1, 3.2, 3.5, 5.5, 5.6_

- [x] 3. 2 つの読み手の判定を直す
- [x] 3.1 (P) emo-atlas の読み手で、番号つきの形式の α の有無をパレットの透明度で決める
  - 番号つきの形式（1・2・4・8 ビット）の集合を足し、その形式のときだけフレームのパレットを写して 255 未満の透明度が在るかを聞く。ほかの形式は今の 12 個の集合のまま
  - パレットの作成・写し・問い合わせの失敗は今の読みの失敗（パス付き）へそのままつなぎ、偽に読み替えない
  - 変換とその引数は触らない。モジュールと関数の説明を「形式とパレット由来の α 有無」に直す
  - 番号つきでない絵（emo2 の `online0.png`）のフレームでパレットを読む関数が `WINCODEC_ERR_PALETTEUNAVAILABLE` の失敗を返すことの檻を、2.1 のテストの兄弟に足す
  - 2.1 の赤が全部緑になり、emo-atlas の今のテスト（emo2 の焼き込みと golden を含む）が値を変えずに緑
  - _Boundary: WicArmJudgment, AtlasPaletteTests_
  - _Depends: 2.1_
  - _Requirements: 1.1, 1.5, 1.6, 2.1, 2.2, 2.3, 2.6, 4.1_
- [x] 3.2 (P) wintf の読み込みに同じ判定を写す
  - emo-atlas と同じ集合・同じ枝・同じ失敗の扱いを、どちらも非公開で足す。戻り値の型と変換器とその引数は変えない。関数の説明の Returns を直す
  - 番号つきでない絵（wintf の `tests/assets/test_8x8_rgba.png`）のフレームで非公開の関数が `WINCODEC_ERR_PALETTEUNAVAILABLE` の失敗を返すことの檻を、テストの時だけつなぐ兄弟のファイルに置く（公開の口は足さない）
  - 2.2 の赤が緑になり、今の `bitmap_source_integration_test.rs` の α の有無のテストがそのまま緑
  - _Boundary: WintfJudgment, WintfPaletteTests_
  - _Depends: 2.2_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.2_

- [ ] 4. 説明と文書を新しい前提に合わせる
- [x] 4.1 (P) emo2 の焼き込みのテストと golden の説明を事実に合わせる
  - `null.png` を「α を持たない・抜き色の腕を通る」と書いた説明を、「番号ごとの透明度を持つパレットの絵として α の道を通り、全画素が透明」に直す（設計の決め 8）
  - 抜き色の絵を名乗る定数名を `SHELL_NULL_PNG_KEY` に改める（同じファイルの中だけ・値と使い方は同じ）
  - 判定の値（失敗 0 件・原寸・配置・golden の 55 行）は 1 つも変えず、テストが緑
  - _Boundary: Emo2TestDocs_
  - _Depends: 3.1_
  - _Requirements: 2.6, 2.7_
- [x] 4.2 (P) 正規化のテストの説明 2 か所を直す
  - 抜き色の腕のテストと「`full` で α 無し」のテストの説明を、設計の決め 7 の文に直す。テストの名前・中身・期待値は変えない
  - 正規化のテストが前と同じ数で緑で、差分は説明の行だけ
  - _Boundary: NormalizeTestDocs_
  - _Requirements: 2.4, 2.5, 2.8_
- [ ] 4.3 (P) `doc/COMPAT_ARCHITECTURE.md` §8 の 2 行を書き直す
  - 273 行目を設計の「§8 の書き直し」どおりに同じ位置で置き換える（問い・決め・理由〔APNG の物差しの違いの記録を含む〕・持ち主・要件 4.8 の置き換え）
  - 274 行目の `tRNS` に触れた 2 か所だけを直し、決め（4 バイトの完全一致）は変えない
  - 差分が §8 のこの 2 行だけで、ほかの行と完了 spec の文書に差分が無い
  - _Boundary: CompatDocRows_
  - _Depends: 3.1, 3.2_
  - _Requirements: 6.1, 6.2, 6.3_

- [ ] 5. 全体の確かめ
- [ ] 5.1 2 つの crate のテストを通し、変えてはならない所の差分が無いことを確かめる
  - emo-atlas と wintf のテストを全部走らせ、緑であることを記録する（GPU・実機・人の操作・排他の机を要らない）
  - `WicDecoderArm` を使う `crates/areka` のテスト（配置の測りと golden を含む）も走らせ、値を変えずに緑であることを記録する
  - 差分に、変換器とその引数・12 個の集合・正規化の本番のコード・動く絵の読み手の行が現れないことを確かめる
  - 2.1・2.2 で記録した赤が、直した後に全部緑へ変わったことを記録に並べる
  - _Depends: 3.1, 3.2, 4.1, 4.2, 4.3_
  - _Requirements: 2.3, 2.4, 2.5, 2.6, 3.3, 3.4, 3.5, 5.6_

## Implementation Notes

- 2.1 直す前の赤（2026-10-31・`cargo test -p areka-emo-atlas palette_tests`）: 4 passed / 10 failed。赤＝has_alpha の真 4 本（idx8_mixed・idx4_mixed・gray8_trns・trans_gif が false）・`On` 4 本（(0,0)・(1,0) が 0,0,0,0 に抜かれる）・`Full` 2 本（(0,1) が 0,0,0,255・(1,1) が 128,0,0,255）。緑＝idx8_plain・idx8_opaque_trns の判定と `On` の 4 本。実際の値は設計の「直す前」の列と一致
- 2.2 直す前の赤（2026-10-31・`cargo test -p wintf --test com wic_test`）: 10 passed / 4 failed。赤＝palette_{idx8_mixed,idx4_mixed,gray8_trns,trans_gif}_has_alpha（has_alpha が false）。緑＝idx8_plain・idx8_opaque_trns と既存 8 本
- 範囲外（3.2 で見つけた・既存）: wintf の lib のテストで、MTA を張るテスト（`alpha_mask_resource`・`bitmap_source`）と ASTA の Compositor を作る `com::wuc::tests::begin_draw_roundtrip` が同時に走ると、プロセスが 0xc0000005 で落ちることがある（`cargo test -p wintf --lib -- begin_draw_roundtrip alpha_mask_resource bitmap_source` を 8 回で 2 回）。全体テストの揺れの元。本 spec の `wic_tests.rs` はテストのスレッドだけの STA で避けた。`/kiro-complete` の棚卸で起票する
