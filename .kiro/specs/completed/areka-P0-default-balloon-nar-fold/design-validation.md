# 設計レビュー: areka-P0-default-balloon-nar-fold

> 2026-09-23・本ブランチ（`05995f39`）で `design.md` の事実主張を実ファイルと突き合わせた。非対話（自動）レビュー。コードは「何の定義か」で指し、行番号では指さない。

## レビュー要約

設計は「既存の道具・登記表・窓口・`static LazyLock<SampleRoot>` の雛形を写すだけ」で、新しい仕組み・crate・依存・Rust ファイルはいずれも 0 と明示している。要件 1.1〜5.6 の 27 項目はすべて Requirements Traceability の表と本文の具体要素に対応し、開発者裁定（中身の sha256 を常設で判定しない）と要件 1.4 の一度きりの照合記録の両方を守っている。事実主張の照合で誤りは 0 件。残るのは、完了判定に使う検索の語形と期待ヒットが 1 か所だけ未確定という点で、設計ディスカッションで数行直せば足りる。

## 事実主張の照合（design.md → 実ファイル）

| design.md の主張 | 照合先（何の定義か） | 結果 |
|---|---|---|
| `SAMPLES` は 5 行 | `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES` 定義（`emo2`・`R_POST_and_KOMAINU`・`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`・`konnoyayame`） | 一致 |
| `known_sample_names()`・`manual_paths` は `SAMPLES` から導く | 同ファイルの `known_sample_names`（`SAMPLES.iter().map(...)`）・`manual_paths`（`registered(name)?` を通る） | 一致 |
| `staysee_root()` を直接呼ぶのは `assets.rs`（2）・`bake.rs`（1）・`faces.rs`（1）・`test_support.rs` 内 `resolve_faces`／`read_decoded` | `crates/areka-emo-text/tests/staysee_balloon_fixture/` を grep | 一致（他のテーマは経由呼びのみ） |
| テーマ別 8 ファイル・合計 2,868 行 | 同フォルダの `.rs` 9 本（`test_support.rs` 含む）＋入口＝10 本で `wc -l` 2,868 | 一致 |
| `Form::tokens` の `CheckedInTree` は末尾 `/` 付き | `crates/log-capture-kit/tests/sample_path_guard_test.rs` の `Form::tokens`（`format!("{VENDOR_PARENT}/{}/", s.name)`） | 一致 |
| 見張りの走査は `crates/**/*.rs` のみ・`vendors/` 除外・コメント除去 | `crates/log-capture-kit/tests/workspace_scan/mod.rs` の `walk_workspace_sources`・`scan_tokens` | 一致 |
| `body_placement` は最上位 `install.txt` を `Supplement` のときだけ除く | `crates/areka-nar/src/plan.rs` の `body_placement`（`skip_install_txt = manifest.kind == InstallKind::Supplement`） | 一致 |
| `lib_tests.rs` の「5」: doc 3 か所・assert 文言 2 か所・関数名 `five`・名前の一覧 2 か所・`BOOT_RECORD: [&str; 5]` は配列長 | `crates/sample-ghost-kit/src/lib_tests.rs`（module doc／landing テストの doc／未登録名テストの doc・`assert_eq!(SAMPLES.len(), 5, ...)` 2 か所・`unknown_sample_fails_with_all_five_known_names`・`BOOT_RECORD`） | 一致（検体数として 5 を語る箇所は列挙どおりで、他に無い） |
| `registry_records_kind_and_bundled_balloons` のバルーンのループは 2 名 | 同ファイルの `for name in ["emo2-kakukaku-offsetdpi", "emo2-kakukaku-wplimit"]` | 一致 |
| `STAYSEE_BALLOON_DIR` の値は `/` で終わらない | `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs` の定数（`"../../vendors/sample_ghost/StayseeBalloon"`） | 一致 |
| `EXPECTED_FILE_NAMES` は 29 本・`assets.rs` は `read_dir` でサブフォルダ 0 を判定 | `test_support.rs` の `EXPECTED_FILE_NAMES: [&str; 29]`・`assets.rs` の `stored_folder_holds_exactly_the_upstream_29_files` | 一致 |
| 雛形 `static EMO2: LazyLock<SampleRoot>` | `crates/areka-emo-compose/src/sample_test_support.rs` | 一致 |
| `sample-ghost-kit` は `areka-emo-text` の dev 依存に既存 | `crates/areka-emo-text/Cargo.toml` の `[dev-dependencies]` | 一致 |
| `git ls-files vendors/sample_ghost/StayseeBalloon` は 29 本 | 実行 | 29 |
| `.gitattributes` の `* -text` が `.nar` に効く | `vendors/sample_ghost/.gitattributes`・`git check-attr text -- vendors/sample_ghost/StayseeBalloon.nar` → `text: unset` | 一致 |
| `install.txt` は `type,balloon`／`directory,StayseeBalloon` | `vendors/sample_ghost/StayseeBalloon/install.txt` | 一致 |
| README「ここで畳んだ 4 本」・表の 4 列 | `vendors/sample_ghost/README.md`「⑷ 配布形の構造にする」・先頭の表 | 一致 |
| `structure.md`「バルーン 2 本」「展開フォルダのままで登記表に載っていない（…#42）」・`product.md`「`StayseeBalloon/` に無改変で保管」 | `.kiro/steering/structure.md` Sample Ghost Kit の「検体の顔ぶれ」・`product.md` 既定バルーンの節 | 一致 |
| brief 2 本の該当行 | `alpha-release-signoff/brief.md`「zip に入れるもの」・`baseware-root-layout/brief.md` の保管先の行 | 一致 |
| provenance §3.1 に 29 本の表（ファイル名・バイト数・sha256）・§1 に上流 URL | `.kiro/specs/completed/areka-P0-default-balloon-bundle/verification/provenance.md` | 一致 |
| `nar-sample-path` bin・`sha256sum`（Git Bash） | `crates/sample-ghost-kit/src/bin/nar-sample-path.rs`・`/usr/bin/sha256sum` | 実在 |
| `fold_tree` は名前順・日時 1980-01-01 固定 | `crates/sample-ghost-kit/src/nar_writer.rs` の module doc と `sort_by_key(file_name)` | 一致 |

平易語の確認: 「檻」「毒化」は design.md に 0 件。行番号による引用は 0 件。

## 判定基準ごとの確認

- **既存アーキテクチャとの整合**: 依存の向き（`areka-emo-text` テスト → `sample-ghost-kit` → `areka-nar`）は不変。窓口・展開器・見張りは 0 変更。
- **要件カバレッジ**: 1.1〜5.6 の 27 項目すべてが設計要素に対応（表「Requirements Traceability」）。暗黙のままの項目は無い。
- **常設 sha256 判定**: 追加していない（Non-Goals・Testing Strategy「常設しないもの」で明示）。要件 1.4 の一度きりの照合は「作業順 ②」と `verification/nar-roundtrip.md` に記録手順として在る。
- **規模 XS/S の逸脱**: 新規の仕組み・crate・依存 0。新規ファイルは `.nar`（要件 1.1）と照合記録（要件 1.4 が「検証記録に残す」と要求）の 2 本だけ。決定 A（既存ループに名前 1 語）と決定 B（README に箇条書き 1 行）は要件外だが、それぞれ要件 2.1 の直接判定と `tech.md`「出どころとライセンスを README に登記」の要請に根拠がある。
- **単一コミット**: 「作業順」が ①畳む→②登記・照合→③追随・付け替え→④追跡から外す→⑤判定→⑥文書 を 1 コミットに固定し、①はフォルダが在るうち・④は⑤の前という順序の要点を明記。要件 1.5 の途中状態は生まれない。
- **零の明示**: 新規 crate 0・依存 0・`Cargo.toml` 変更 0・テーマ別 0 行・新しいテスト 0 本・監視 0 が逐語で書かれている。

## 重大な問題（最大 3）

🔴 **重大な問題 1**: 完了判定に使う検索の語形と期待ヒットが未確定（要件 4.1 と 5.6 の判定が実装者の解釈に残る）
**懸念**: 作業順 ⑦ の検索は `vendors/sample_ghost/StayseeBalloon/`（末尾 `/` 付き）で組まれている。⒜ 要件 4.1 が 0 か所を求める綴りは末尾 `/` の無い `vendors/sample_ghost/StayseeBalloon` で、定数 `STAYSEE_BALLOON_DIR` の値も `/` で終わらない。付け替え後に定数の削除だけ漏れると、未使用の定数は警告どまりで結合テストは緑のまま、⑦ の検索にも見張り（`Form::tokens` は `/` 付き）にも掛からない。⒝ ⑦ の検索対象 `.kiro/steering` には `roadmap.md` が含まれ、台帳 #42 の行（「`vendors/sample_ghost/StayseeBalloon/` が展開フォルダのまま残っている」）が必ず当たる。設計は #37 の行を「経緯なので残してよい」と先に決めているが、#42 の行については「状態列を完了へ」としか書いておらず、当たった行をどう読むかが決まっていない。
**影響**: 「0 件」の判定が検索の生の結果と食い違い、実装レビューで「0 件ではない」と差し戻されるか、逆に定数の残骸を見逃す。
**提案**: ⑦ を 2 本にする——⒜ `grep -rn "vendors/sample_ghost/StayseeBalloon" crates/areka-emo-text/tests/`（`/` 無し・コメント込み）が 0 件＝要件 4.1、⒝ 現行の `/` 付き検索は「期待ヒットは `roadmap.md` の #37 行と #42 行の 2 件（経緯の記録）・それ以外 0 件」と期待値ごと書く。#42 の行は完了時に「残っていた」の過去形へ直すなら、その旨も Modified Files に足す。
**トレーサビリティ**: 要件 4.1・5.4・5.6
**根拠**: design.md「文書の追随」の検索の段落・「System Flows > 作業順」⑦・「Modified Files」の `roadmap.md` 行

## 設計の強み

1. **写すだけで新設 0**: 畳む道具・登記表・窓口・`static LazyLock<SampleRoot>` の雛形をすべて既存から採り、`staysee_root() -> PathBuf` の戻り型を保つことでテーマ別 8 ファイルを 0 行変更に抑えている。要件 4.3（プロセス中ずっと同じ位置）が `static` の型で成り立つ点まで根拠付きで書かれている。
2. **判定の根拠が正しく置かれている**: 「見張りは `/` 付きの形しか拾わない・付け替え漏れを赤にするのは展開フォルダが消えた後の結合テスト」という要件ディスカッションでの訂正を設計に持ち込み、作業順（④ を ⑤ の前）にまで落としている。

## 最終評価

**判定: GO**

**根拠**: 事実主張に誤りが 0 件、要件 27 項目すべてに設計要素があり、開発者裁定（常設 sha256 判定なし）と単一コミットの制約を守っている。重大な問題 1 は完了判定の検索の書き方の問題で、設計ディスカッションで数行の追記により解消できる。

**次のステップ**:
1. 設計ディスカッションで重大な問題 1 を解消（⑦ の検索を 2 本に・期待ヒットを明記・#42 行の扱いを決める）。
2. 併せて下記の軽微な点を反映するかを決める。
3. `/kiro-spec-tasks areka-P0-default-balloon-nar-fold` でタスク生成へ。

## 軽微な指摘（非ブロッキング）

- `baseware-root-layout/brief.md` には設計が挙げる保管先の行のほかに、冒頭の追記「既定バルーンを `.nar` へ畳む仕事は `areka-P0-default-balloon-nar-fold`（台帳 #42）が持つ。本文の『`.nar` へ畳むのは `areka-P0-nar-install` が引き受ける』は…偽になった」がある。完了後は「持つ」が過去形になるので、要件 5.3 の追随にこの 1 文も含めるかを決めておくとよい（要件 5.6 の検索には掛からない）。
- 照合記録 `verification/nar-roundtrip.md` は要件 2.4（`nar-sample-path` が `root=`／`folder=` を印字し終了コード 0）の確認を兼ねると書かれているので、記録に印字 2 行と終了コードをそのまま残すよう Output の項に 1 行足すと、2.4 の証跡が推移的でなく直接になる。
- `staysee_balloon_fixture_test.rs` の module doc には「本ファイルは検体パス定数と接続宣言だけを持つ入口である」の 1 文もある。設計の「module doc を書き直す」に含まれると読めるが、消える定数を語る箇所として Modified Files に名指ししておくと漏れにくい。
