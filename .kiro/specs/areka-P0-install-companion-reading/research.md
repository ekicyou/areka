# ギャップ分析: areka-P0-install-companion-reading

- 作成日: 2026-10-04（ブランチ `claude/areka-p0-install-companion-c1b07d`・起点のコミット `ab6365a2`）
- 対象: `requirements.md`（要件 1〜8）と、今のソースとの差
- 方法: ソースを読んで確かめた（テストの全体実行はしていない）。正典の引用は `requirements.md` の Introduction が引き直したものをそのまま使い、SSP の実測はしていない
- 位置づけ: 判断の材料と選択肢を並べる。最終の決定は要件の討議と設計で行う

## 1. 要約

- 変える中心は `crates/areka-nar/src/manifest.rs` の `collect_companions`（同梱の一覧を組む関数）と、`crates/areka-nar/src/plan.rs` の `in_folder`（エントリが取り出し元の配下かを答える関数）・`companion_placement`（同梱 1 件の配置を組む関数）の 3 か所。4 点の読み替えはどれもここに集まっている。
- 安全の検査（`crates/areka-nar/src/names.rs` の `is_valid_one_level_name`）と確定の手順（`crates/areka-nar/src/install.rs`）は、そのまま使える。足りないのは「読み替え」「番号を順に探す」「複数段の取り出し元」「読み替えの記録の種類」の 4 つ。
- `_` への置き換えを共有できる関数は、今のリポジトリに 1 つも無い（0 件）。置き場所の候補は 3 つある（5 節）。
- 取り出し元を `/` 区切りの正規化した文字列のまま `Companion.source_directory` に入れれば、`crates/areka/src/install/terms.rs` は本体を 1 行も変えずに階層付きで動く（テストだけ足す）。`procedure.rs`・`judge.rs` を変えずに済ませるには、**断る理由の種類（`RefuseReason`）を増やさない**ことが条件になる。
- 要件の文面とソースを突き合わせて、要件の討議で確かめたい点が 9 つ見つかった（7 節）。うち 2 つ（`../escape` の扱い・打ち切りの後ろの断片の記録の種類）は要件の文と実際の結果が食い違う、または 2 通りに読める。

## 2. 今の実装（調べた結果）

### 2.1 `install.txt` の読み手（`crates/areka-nar/src/manifest.rs`）

| 何の定義か | 今の動き |
|---|---|
| `parse_manifest`（`install.txt` の中身を解釈する関数） | 鍵を ASCII 小文字化した `BTreeMap` に写し、種別 → `name` → `directory` → 本体の再インストールの扱い → `collect_companions` の順に読む。本体の `directory` は `check_one_level` で検査する |
| `numbered`（接頭辞が「基の名前＋数字列」かを答える関数） | 基の名前の後ろに ASCII の数字が何桁続いても真。`balloon007`・`balloon01` も真 |
| `classify`（鍵を「単独の鍵／同梱の形／知らない鍵」に分ける関数） | 後半（`COMPANION_SUFFIXES`＝`directory`・`source.directory`・`refresh`・`refreshundeletemask`）を剥がし、接頭辞を `numbered` で判定する |
| `collect_companions`（同梱の一覧を組み、読み飛ばしを記録する関数） | ⑴ `<接頭辞>.directory` の行を接頭辞を鍵にした `BTreeMap` に全部集める（欠番で止まらない）。⑵ 鍵の名前順に読み飛ばしを記録する。⑶ `BTreeMap` の順（バイト順＝`balloon`・`balloon0`・`balloon10`・`balloon2`）に `Companion` を組む。`*.directory` と `*.source.directory` の両方に `check_one_level` を掛け、通らなければ `RefuseReason::InvalidDirectoryName` で書庫ごと断る |
| `check_one_level`（1 階層の名前かを確かめる関数） | `is_valid_one_level_name` が偽なら `InvalidDirectoryName { key, value }`。呼ばれるのは本体の `directory`・`<接頭辞>.directory`・`<接頭辞>.source.directory` の 3 か所 |
| `Companion`（同梱 1 件の公開の型） | 欄は `key`・`directory`・`source_directory`・`existing` の 4 つ。`source_directory` は `String` |
| `InstallManifest.companions` の説明 | 「接頭辞の名前順に並ぶ」と書いてある（本 spec で書き換える） |

`areka_parsers::kv::parse_kv`（`crates/areka-parsers/src/kv/parse.rs`）は `BTreeMap` を返し、行の順を保たない。したがって「無印 → 0 → 1 → 2…」の順は、鍵を順に**引いて**作るしかない（並べ替えではない）。

### 2.2 名前の検査（`crates/areka-nar/src/names.rs`）

- `is_valid_one_level_name`（木を掘らない 1 つの名前かを答える関数・`pub(crate)`）: 空でない・UTF-16 で 200 単位以内・`/` と `\` を含まない・`is_usable_windows_name` を満たす。
- `is_usable_windows_name`（Windows で作れる名前かを答える関数）: 禁止文字（`< > : " | ? *` と制御文字）・末尾のドットと空白・予約名（最初のドットより前の語の完全一致）。`..` と `.` は「末尾がドット」で、`C:` は禁止文字で落ちる。
- 上限は `crates/areka-nar/src/error.rs` の `MAX_ENTRY_PATH_UTF16`（200）。長さは同じファイルの `utf16_len` で数える。

読み替えた後の値にこの 2 つを掛ければ、要件 5 の 1〜4 項は新しい検査を書かずに満たせる。

### 2.3 配置の計画（`crates/areka-nar/src/plan.rs`）

| 何の定義か | 今の動き |
|---|---|
| `in_folder`（エントリが取り出し元そのもの、または配下かを答える関数） | エントリの**先頭の 1 要素だけ**を ASCII の大小を無視して比べる。本体から除く側と同梱へ取り込む側が同じ関数を使う |
| `companion_placement`（同梱 1 件の配置を組む関数） | `collect_tree` に「剥がす段数 1」を渡す。ファイルもフォルダも 0 件なら `RefuseReason::CompanionSourceMissing { key, source_directory }` |
| `collect_tree`（通したエントリを、先頭の何段かを剥がした木に写す関数） | 剥がす段数は引数。複数段を渡せばそのまま動く作り |
| `body_placement`（本体の配置を組む関数） | どれかの同梱の取り出し元の配下のエントリを本体から除く |
| `build_plan`（計画の全体を組む関数） | 本体 → `manifest.companions` の順。説明文に「接頭辞の名前順」とある（本 spec で書き換える） |

### 2.4 記録と断る理由（`crates/areka-nar/src/error.rs`・`lib.rs`）

- `ManifestWarning`（読み飛ばしの記録の種類）は 5 種: `UnsupportedCompanionKind`・`CompanionOnNonGhost`・`IgnoredKey`・`RefreshIgnoredForSupplement`・`InvalidMaskEntry`。「読み替えた」「探索で読まなかった」に当たる種類は無い（0 種）。
- 記録をログへ出すのは `crates/areka-nar/src/lib.rs` の `NarArchive::install`。成功したとき `outcome.warnings` を 1 件ずつ `warn!`（`[areka_nar] manifest entry skipped`）で出す。種類を足すだけで、この出口は変えずに済む（要件 6 の 8 項）。
- `RefuseReason`（断る理由）は `refuse_reasons!` マクロで 14 種に閉じている。`crates/areka/src/install/judge.rs` の `failure_word`（断る理由を正典の失敗の語へ写す関数）は 14 種を**漏れなく並べた** `match` で、ワイルドカードの腕を持たない。**種類を 1 つ足すと `judge.rs` がコンパイルできなくなる**。
- `crates/areka-nar/src/error_tests.rs` の `manifest_warnings_are_five_and_carry_their_key` は「5 種」を数で固定している。種類を足すとこのテストを書き換える。
- `crates/areka/` の中に `ManifestWarning` の種類の名前を書いている場所は 0 件（`ManifestWarning::` を検索して 0 件）。記録の種類を足しても `crates/areka/` は変わらない。

### 2.5 並びが知らせへ流れる道

`build_plan` の順 → `crates/areka-nar/src/install.rs` の `commit_all`（`InstallOutcome.installed` へ計画と同じ順に積む）→ `crates/areka/src/install/procedure.rs` の `installed_items`（`outcome.installed` を順に写す）→ `crates/areka/src/install/judge.rs` の `complete_ex_refs`（渡された順に繋ぐ）と `complete_legacy_refs`（2 件目の名前を Reference2 にする）。途中に並べ替えは無い。`manifest.companions` の順を変えれば、`procedure.rs`・`judge.rs` を変えずに知らせの並びが変わる（要件 1 の 8 項・Adjacent expectations のとおり）。

### 2.6 起動の側（本 spec は変えない）

- `crates/areka-ghost/src/catalog.rs` の `companion_balloon`（インストール済みのゴーストの `install.txt` から同梱バルーンの名前を引く関数）: `balloon.directory` の 1 鍵だけを読む。番号付きは読まない。値の置き換えはしない。空の値は「無し」（同じファイルの `lowercased` が空の値を落とす）。
- `crates/areka/src/boot_resolve.rs` の段 3（「ゴーストの同梱」の段）: その名前でバルーンの一覧を引き、無ければ `warn!`（`companion_balloon_not_found`）を出して次の候補へ進む。

## 3. 個別に確かめた 6 点（a〜f）

### a. 置き換えの後に 2 つの同梱が同じ宛先になったときの、今の動き

今は `_` への置き換えが無いので、同じ宛先になるのは「2 つの接頭辞が同じ `*.directory` の値を書いた」場合である。その動きをソースから追うと次のとおり。**この動きを固定しているテストは 0 件**（`crates/areka-nar/src/` を「同じ宛先」に当たる語で検索して該当なし）。

- 読み手（`collect_companions`）: 宛先の重なりを見る検査は無い。2 件の `Companion` がそのまま並ぶ。記録は 0 件。
- 計画（`build_plan`）: 同じ `destination` を持つ配置が 2 つできる。断らない。
- 組み上げ（`crates/areka-nar/src/lib.rs` の `place`）: 全ての配置を確定の**前に**組み上げる。`stage_placement` は組み上げの時点の宛先を見るので、2 件とも同じ「既存の別」になる。
- 確定（`install.rs` の `commit_all`・`commit_one`）:
  - 宛先が**元から在った**場合（2 件とも重ね置き／全消去）: 1 件目を確定した後、2 件目が「宛先（＝1 件目の結果）を退避 → 自分の木を宛先へ」を行う。2 件目の木は元の宛先を下敷きにして組んであるので、**1 件目が書庫から足したファイルは消え、2 件目だけが残る**。`InstallOutcome.installed` には同じ名前・同じ場所の要素が 2 つ並び、知らせにも 2 つ載る。
  - 宛先が**無かった**場合（2 件とも新規）: 2 件目は「作業フォルダ → 宛先」の 1 手だけを行うが、宛先は 1 件目が置いた後で既に在る。同じファイルの `Undo::Restore` の説明は「Windows の `rename` は既に在る宛先を上書きしない」ことを前提にしており、その前提どおりなら確定の段の I/O 失敗（`NarError::Io`・`IoPhase::Commit`）になって全体が巻き戻る。**ここはソースの読みだけで、実行して確かめてはいない**。
- つまり今は「元から在れば後勝ちで黙って 1 件目が消える／無ければ確定で失敗する見込み」で、どちらも記録に理由が残らない。本 spec の後は `a/b` と `a_b` のように**書いた値が違っても**同じ宛先になり得るので、この道に入る書庫が増える（要件は対象外としている＝7 節の 3）。

### b. `*.directory` の値が空のときの、今の動き

- `parse_kv` は値が空の行も鍵として残す。`collect_companions` はその鍵を「宛先が在る接頭辞」に数え、`check_one_level` が空の名前を通さないので、`RefuseReason::InvalidDirectoryName { key: "balloon.directory", value: "" }` で**書庫ごと断る**。
- 固定しているテスト: `crates/areka-nar/src/manifest_companion_tests.rs` の `refuses_a_companion_directory_that_is_not_a_one_level_name`（2 つ目の検査が `balloon.directory,` を断ることを確かめている）。
- 起動の側は逆で、空の値は「無し」になる（`crates/areka-ghost/src/catalog_tests.rs` の `empty_values_count_as_absent`）。インストールの側と起動の側で扱いが違うのは今のままである。
- 種別が `balloon`・`supplement` の書庫では値を検査しないので、空でも断らずに `CompanionOnNonGhost` を 1 件記録する。

### c. 種別が `ghost`・`shell` 以外の書庫の同梱を読まない場所

- `collect_companions` の先頭の `handles_companions`（`matches!(kind, InstallKind::Ghost | InstallKind::Shell)`）の 1 か所だけ。偽のとき:
  - 宛先を集める段を丸ごと飛ばす（同梱は 0 件）。
  - 同梱の形の鍵ごとに `ManifestWarning::CompanionOnNonGhost` を 1 件記録する（扱わない種別 `headline` などもここでは同じ記録になる）。
  - 値の検査はしない（壊れた値でも断らない）。
- 固定しているテスト: `manifest_companion_tests.rs` の `warns_and_skips_companions_written_on_a_balloon_or_a_supplement` と `a_broken_companion_directory_on_a_balloon_is_skipped_not_refused`。
- 計画の側（`plan.rs`）に種別の判定は無い（0 か所）。`manifest.companions` が空なので何も起きないだけである。
- 本 spec はここを変えない（要件 1 の 10 項）。変更 0。

### d. `*.directory`／`*.source.directory` を読む場所の全て

本番のコード（6 か所）:

| ファイル | 何の定義か | 読むもの | 本 spec で変わるか |
|---|---|---|---|
| `crates/areka-nar/src/manifest.rs` | `collect_companions`・`classify`・`numbered` | `install.txt` の 2 つの鍵そのもの | 変わる（中心） |
| `crates/areka-nar/src/plan.rs` | `body_placement`・`companion_placement`・`in_folder` | `Companion.source_directory`・`Companion.directory` | 変わる |
| `crates/areka-nar/src/error.rs` | `RefuseReason::CompanionSourceMissing`・`InvalidDirectoryName`・`ManifestWarning` | 理由と記録に載せる値 | 記録の種類を足す |
| `crates/areka/src/install/terms.rs` | `nested_terms`（同梱の中の利用条件のパスを集める関数） | `Companion.source_directory` を `format!("{}/{file}", …)` で繋ぎ、`NarArchive::entry_bytes` で引く | 取り出し元を `/` 区切りの正規化した文字列で持てば本体は変更 0 |
| `crates/sample-ghost-kit/examples/fold-samples.rs` | `installed_path`・`strip_folder`（検体の相対パスがインストール済みの形でどこへ行くかを求める関数） | `Companion.source_directory`・`Companion.directory` | 変わる（`strip_folder` が先頭の 1 要素だけを見ている） |
| `crates/areka-ghost/src/catalog.rs` | `companion_balloon` | インストール済みのゴーストの `install.txt` の `balloon.directory` | 変えない（`areka-P0-ghost-standard-balloon` の持ち分） |

間接に使う場所（値そのものは読まない）: `crates/areka/src/boot_resolve.rs` の段 3（`companion_balloon` の結果を使う）・`crates/areka/src/install/procedure.rs` の `installed_items`（`InstalledElement` の `kind`・`name`・`path` を使う）。どちらも変更 0。

テストと検体の道具（`install.txt` の同梱の行を書いている場所）:

- `crates/areka-nar/src/`: `manifest_companion_tests.rs`・`manifest_tests.rs`・`plan_tests.rs`・`install_tests.rs`・`lib_tests.rs`・`lib_entry_tests.rs`・`lib_vocabulary_tests.rs`・`error_tests.rs`
- `crates/areka/src/install/`: `terms_tests.rs`・`procedure_test_support.rs`（`ghost_with_balloon_descript`）・`judge_tests.rs`（`CompanionSourceMissing` の値を組むだけ）
- `crates/areka-ghost/src/`: `catalog_tests.rs`・`catalog_test_support.rs`
- `crates/areka/src/main_config_input_tests.rs`・`crates/areka/tests/smoke_boot_loop_exit.rs`（説明文だけ）
- `crates/sample-ghost-kit/src/lib_tests.rs`

文書: `doc/ukadoc-coverage/ledger/assets.toml` の 2 行（`…_2a.directory…`・`…_2a.source.directory…`）・`doc/COMPAT_ARCHITECTURE.md` §8。Lua・PowerShell のスクリプトに読む場所は無い（0 件）。

本 spec の後に**結果が変わる既存のテスト**（書き換えが要る）:

| テスト（どれも `manifest_companion_tests.rs`） | 今の期待 | 本 spec の後 |
|---|---|---|
| `reads_numbered_balloon_companions_in_key_order` | 並びは `balloon`・`balloon0`・`balloon10`・`balloon2` | `balloon`・`balloon0` だけ（`balloon1` が無いので打ち切り） |
| `balloon_followed_by_digits_is_a_companion` | `balloon9`・`balloon10`・`balloon007` を単独で書いても 1 件読む | 3 つとも 0 件（`balloon0` が無い／先頭の 0） |
| `refuses_a_companion_directory_that_is_not_a_one_level_name` | `balloon0.directory,../escape` を断る | 断らず `.._escape` として入る（7 節の 1） |
| `refuses_a_companion_source_directory_that_is_not_a_one_level_name` | `sub/kakukaku` を断る | 断らず 2 段の取り出し元として読む |

`crates/areka-nar/src/error_tests.rs` の `manifest_warnings_are_five_and_carry_their_key`（記録の種類は 5 種）も、種類を足すなら書き換える。

### e. 区切りを置き換える共有の規則が既に在るか

無い（0 件）。似た形は次の 2 つだが、どちらも使えない。

- `crates/areka/src/install/fetch_url.rs` の `file_name`（取ってきた書庫の一時ファイル名を作る関数・非公開）: ASCII の英数字と `.`・`_`・`-` 以外を全て `_` にする。規則が違い（日本語も `_` になる）、`areka` の実行ファイルのクレートの中に在るので `areka-ghost` からは呼べない。
- `crates/areka-emo-compose/src/method.rs` の正規化: `-` と `_` を取り除く別の用途。

依存の向き: `crates/areka-nar/Cargo.toml` と `crates/areka-ghost/Cargo.toml` はどちらも `areka-parsers` に依存し、互いには依存しない。`areka-parsers` の公開の子は `balloon`・`charset`・`kv`・`package`・`sakura`・`shell`（`crates/areka-parsers/src/lib.rs`）。置き場所の選択肢は 5 節。

### f. 要件 8 の場面ごとの、隣のテストと検体

書庫を組む道具は `crates/sample-ghost-kit/src/nar_writer.rs` の `NarBuilder`（`file`・`dir`・`done`・`bytes`）と `install_txt`（行を CRLF で繋ぐ関数）。一時フォルダは `sample_ghost_kit::WorkDir` と `temp_path_kit::TempPath` を既存のテストが使っている（置き場所が `target\` の下かは設計で確かめる）。場面ごとの隣は次のとおり。

| 要件 8 の場面 | 隣の既存テスト／助手 | 状態 |
|---|---|---|
| 欠番での打ち切り・無印が無い番号付きだけ・無印と `balloon0` の両方・`balloon2` と `balloon10` の並び・先頭に 0 を付けた番号 | `manifest_companion_tests.rs` の `reads_numbered_balloon_companions_in_key_order`・`balloon_followed_by_digits_is_a_companion`。助手は `manifest_tests.rs` の `parsed_ghost`・`refused_ghost` | 隣が在る（期待を書き換える・場面を足す） |
| `*.directory` の行が無い接頭辞の断片 | `manifest_companion_tests.rs` の `a_companion_key_without_its_directory_is_recorded_as_an_ignored_key` | 無印については今のテストがそのまま要件 6 の 5 項を満たす。番号付きの断片で打ち切る場面は無い（足す） |
| `\` と `/` の階層付きの取り出し元・兄弟のファイルが本体に残ること | `plan_tests.rs` の `a_companion_balloon_goes_to_the_balloon_folder_with_its_prefix_stripped`・`the_companion_leaves_no_copy_on_the_ghost_side`・`the_companion_source_name_may_differ_from_its_destination`・`the_companion_source_matches_the_archive_spelling_ignoring_case`。助手は `planned`・`plan_of`・`refusal_of`・`relative_files`・`ghost_archive` | 1 階層の隣が在る。複数段の場面は 0 件（足す） |
| `..` と空の段の取り除き・取り除いた後に段が残らない場合・`*.directory` の区切りの置き換え・`*.source.directory` を省略して `*.directory` に区切りがある場合・置き換えた結果が予約名／長すぎる名前 | `manifest_companion_tests.rs` の `refuses_a_companion_*`（2 本）・`defaults_the_companion_source_directory_to_the_directory`・`an_empty_companion_source_directory_falls_back_to_the_directory`。長さは `lib_tests.rs` の `a_directory_name_of_exactly_the_limit_opens`・`a_directory_name_one_over_the_limit_is_refused_with_a_bounded_value`（本体の `directory` 用） | 断る側の隣が在る。読み替える側の場面は 0 件（足す） |
| 本体の `directory` に区切りがある場合（今どおり断る） | `manifest_tests.rs` の `refuses_a_directory_that_is_not_a_one_level_name` | 既に在る（変更 0 で通り続けることを確かめるだけ） |
| 置かれた物が全て根の中の宛先の配下に在ること（要件 8 の 2 項） | `plan_tests.rs` の `every_path_of_every_plan_stays_under_the_root`（`every_accepted_plan` が並べた全ての計画の全ての道を歩く）と `escapes`・`is_inside` | 仕組みが在る。本 spec の場面の書庫を `every_accepted_plan` に相当する一覧へ足せば同じ歩き方で確かめられる |
| 読み替え・読み飛ばしの無い `install.txt` は記録 0 件（要件 5 の 8 項・6 の 9 項） | `manifest_tests.rs` の `every_real_sample_install_txt_is_accepted_verbatim_without_a_single_warning`（α の検体の `install.txt` を写したバイト列で確かめる） | 既に在る（そのまま通り続けることが回帰の番になる） |
| 知らせの並び（要件 8 の 3 項） | `crates/areka/src/install/procedure_tests.rs` の `ghost_with_balloon_lists_body_first_then_the_companion`。助手は `procedure_test_support.rs` の `ghost_nar`・`ghost_with_balloon_descript`・`FakePorts` | 同梱 1 件の隣が在る。10 件以上の場面は 0 件（置き場所は 7 節の 7） |
| 階層付きの取り出し元の直下の利用条件（要件 8 の 4 項） | `crates/areka/src/install/terms_tests.rs` の `top_level_terms_are_shown_and_nested_ones_are_listed` ほか。助手は `ghost`・`with`・`open` | 1 階層の隣が在る。同じ助手で階層付きの書庫を組める |
| 開発用の道具（要件 2 の 8 項） | `crates/sample-ghost-kit/examples/fold-samples.rs` は実行の例で、関数単位のテストは 0 件。道具自身が「入れた結果」と `installed_path` の写像を突き合わせて不一致を数える作り | 検体に階層付きの取り出し元を持つものは 0 体。確かめ方は 7 節の 8 |

行数の余裕: `plan_tests.rs` 916 行・`manifest_companion_tests.rs` 528 行・`manifest_tests.rs` 618 行・`procedure_tests.rs` 451 行・`terms_tests.rs` 200 行。`plan_tests.rs` には足せない（1 ファイル 1,000 行の上限）。新しい兄弟のテストのファイルは `manifest.rs`・`plan.rs` の末尾に `#[path]` の行を足して繋ぐ（今は各 1 本）。

## 4. 要件ごとの対応と不足

| 要件 | 今ある資産 | 不足・制約 |
|---|---|---|
| 1（探索の順・打ち切り・並び） | `collect_companions`・`classify`・`numbered`。知らせへの道（2.5）は順をそのまま運ぶ | **不足**: 鍵を無印 → 0 → 1… の順に引く探索。先頭の 0 を数えない判定。**制約**: `parse_kv` は順を保たない |
| 2（階層付きの取り出し元） | `collect_tree` は剥がす段数を引数で受ける。`NarArchive::entry_bytes` は `/` 区切りの全体のパスを ASCII の大小を無視して引く | **不足**: `in_folder` が先頭 1 要素だけを見る。`companion_placement` が段数 1 固定。`fold-samples.rs` の `strip_folder` が先頭 1 要素だけを見る |
| 3（`..` と空の段の取り除き） | なし | **不足**: 値を `\` と `/` で分けて `..` と空の段を落とす処理（0 件）。段が残らないときに断る理由（既存の `InvalidDirectoryName` が鍵と値を持つ） |
| 4（`_` への置き換え） | なし | **不足**: 置き換えの関数（0 件）と、起動の側も使える置き場所 |
| 5（安全の検査） | `is_valid_one_level_name`・`MAX_ENTRY_PATH_UTF16`・`utf16_len`・`bounded_value`。`plan_tests.rs` の「根の外へ出ない」歩き方 | 不足なし（読み替えた後の値に掛けるだけ）。**制約**: 断る理由の種類を足すと `judge.rs` が変わる（2.4） |
| 6（読み替えの記録） | `ManifestWarning`・`NarArchive::install` の出口 | **不足**: 「置き換えた」「取り除いた」「探索で読まなかった」の種類（0 種）。**制約**: `error_tests.rs` が 5 種を数で固定 |
| 7（文書） | `doc/COMPAT_ARCHITECTURE.md` §8 の表（項目・裁量・根拠・出典 spec の 4 列）。`assets.toml` の 2 行 | **制約**: 台帳の「実装済み」の行は、ソースの `// ukadoc: <URL>` の行を根拠にしている（`crates/ukadoc-survey/src/evidence/mod.rs` の説明）。今は 2 行とも `manifest.rs` の `COMPANION_SUFFIXES` の上の URL の行を指す。この行を消したり移したりするなら、台帳の「根拠の場所」も合わせる |
| 8（テスト） | 3 節の f の表 | 不足は表のとおり |

## 5. 実装の選択肢

### 5.1 読み替えの関数の置き場所（要件 4 の 4 項）

| 案 | 内容 | 良い点 | 悪い点 |
|---|---|---|---|
| A | `areka-parsers` の既存の子（`kv` か `package`）の下に小さなファイルを足し、その子の `mod.rs` から公開する | 新しい依存の辺が 0 本。`areka-ghost` の `catalog.rs` は既に `areka_parsers::kv::parse_kv` を使っている。`crates/areka-parsers/src/lib.rs` に触らずに済む（同じウェーブの約束） | `kv` は「分類も型付けもしない素朴な読み手」、`package` は「フォルダの木を歩く」役で、どちらの子も `install.txt` の鍵の意味を知らない。置く子の説明文に 1 行足すことになる |
| B | `areka-nar` に公開の関数として置き、`areka-ghost` が `areka-nar` に依存する | 規則と検査が 1 つのクレートに並ぶ | 新しい依存の辺が 1 本。起動の側が書庫の伸長の実装（`miniz_oxide`）まで引く |
| C | `areka-nar` に置き、起動の側は `areka` の実行ファイルのクレート（両方に依存済み）で置き換えを掛ける | 依存の辺が増えない | `install.txt` を読むのは `areka-ghost` の `companion_balloon` なので、「読む所」と「置き換える所」が 2 つのクレートに分かれる |

`..` と空の段の取り除き（要件 3）は、今わかっている使い手がインストールの側だけである。`areka-nar` の中に置く形と、`_` への置き換えと並べて案 A の場所に置く形のどちらも取れる。

### 5.2 取り出し元の持ち方

| 案 | 内容 | 良い点 | 悪い点 |
|---|---|---|---|
| A | `Companion.source_directory` を `String` のまま、`/` 区切りに正規化した相対パス（例 `extra/bal1`）を入れる | `terms.rs` の `nested_terms` は本体の変更 0 で階層付きに対応する（`entry_bytes` が `/` 区切りの全体を大小無視で比べるため）。公開の型の形が変わらない。`CompanionSourceMissing` の理由にもそのまま載る | `plan.rs` と `fold-samples.rs` が使うたびに `/` で分け直す |
| B | 段の列（`Vec<String>`）で持つ | `in_folder` が分け直さずに比べられる | 公開の型の欄の型が変わる。`terms.rs` と `fold-samples.rs` が繋ぎ直す |

### 5.3 探索と記録の組み方

| 案 | 内容 | 良い点 | 悪い点 |
|---|---|---|---|
| A（今の関数を広げる） | `collect_companions` の「宛先を集める段」を「無印を引く → `balloon0`・`balloon1`… を見つからなくなるまで引く」に替え、見つかった接頭辞の列（順つき）を作る。読み飛ばしの段は「見つかった列に無い `balloon*` の鍵」を、`*.directory` の行の有無で 2 種類の記録に振り分ける | 変更が 1 関数に収まる。`classify` はそのまま使える | `manifest.rs` は 431 行で、読み替えと記録を足すと 500 行台になる |
| B（同梱の読み手を兄弟のファイルへ分ける） | 同梱の探索・読み替え・記録を新しいファイルへ移し、`manifest.rs` は本体の鍵だけを読む | 役割が分かれ、新しいテストのファイルと 1 対 1 になる | 移す差分が大きく、既存の 2 本のテストのファイルの `use super::*` の繋ぎも動く |
| C（混ぜる） | 読み替えの純粋な関数（置き換え・取り除き）だけを 5.1 の場所へ出し、探索と記録は `collect_companions` に残す | 5.1 と自然に揃う。差分が小さい | 特になし |

### 5.4 断る理由と記録の種類

- 断る理由: 「取り除いた後に段が残らない」（要件 3 の 4 項）と「読み替えた後の値が検査を通らない」（要件 5 の 2・4 項）は、どれも鍵と値を載せる。既存の `RefuseReason::InvalidDirectoryName { key, value }` がその形で、これを使えば種類は 14 のまま、`judge.rs` は変更 0。種類を足す案は `judge.rs` の `failure_word` を必ず変える。
- 記録の種類: 要件 6 の 1・2・4 項に 1 種ずつ（3 種）足す案と、「読み替えた」（鍵・書かれていた値・読み替えた後の値）と「探索で読まなかった」（鍵）の 2 種にまとめる案がある。どちらでも `crates/areka/` は変更 0。

### 5.5 規模と危険の見立て

- 規模: **S〜M**（brief の見立て 7〜11 タスクのまま）。新しい仕組みは無く、既存の関数 3 つと記録の種類を広げ、テストの場面を足す仕事である。
- 危険: **中**。理由は 2 つ。⑴ 第三者の書庫を受ける口で、読み替えが「断る」を「入れる」に変える（7 節の 1・3・5）。⑵ `crates/areka/src/install/` で触ってよいのが `terms.rs`（とそのテスト）だけ、という並走の約束があり、知らせの並びのテストの置き場所に制約がある（7 節の 7）。

## 6. 設計へ持ち越す調べもの

- 新しいテストが使う一時フォルダ（`WorkDir`・`TempPath`）の実際の置き場所が、ワークツリーの `target\` の下であること（要件 8 の 5 項）。
- 台帳の検査（`crates/ukadoc-survey` の整合のテスト）が、`assets.toml` の 2 行の書き換えと「根拠の場所」の指し先に求める形。
- 3 節の a の「宛先が無かった場合」の実際の結果（確定の段で失敗して巻き戻るか）。本 spec の対象外だが、7 節の 3 の判断の材料になる。
- `areka-P0-install-live-target-hazards` の着手の状態と、`procedure_tests.rs`・`procedure_test_support.rs` を向こうが触るか（7 節の 7）。

## 7. 要件の討議で確かめたい点

1. **`../escape` のような `*.directory` は、断られずに `.._escape` として入る。** 要件 4 の 3 項は「`..` を含む値は Requirement 5 の検査で今どおり断られる」と書くが、区切りを `_` に置き換えた後の `.._escape` は今の検査（`is_valid_one_level_name`）を通る（末尾がドットでも予約名でもない）。断られるのは値が `..` そのものの場合だけである（要件 5 の 2 項の例はこちら）。宛先は `<根>/balloon/.._escape/` で根の外へは出ないので安全は保たれるが、要件 4 の 3 項の文と実際の結果が食い違う。文を直すか、「置き換える前の値に `..` の段が在れば断る」を足すかを決める。今のテスト `refuses_a_companion_directory_that_is_not_a_one_level_name` の 1 つ目の期待もこれで変わる。
2. **打ち切りの後ろに在る「`*.directory` の行が無い断片」は、どちらの記録にするか。** 例: `balloon0.directory` と `balloon5.refresh` だけが書かれている。要件 6 の 4 項（打ち切りの後ろの番号の鍵＝「探索で読まなかった」）と 5 項（`*.directory` の行が無い同梱の鍵＝今どおりの読み飛ばし）の両方に当たる。1 つの鍵から出す記録は最大 1 件（6 項）なので、どちらを優先するかを決める。
3. **置き換えで同じ宛先になる 2 つの同梱。** 要件は対象外（今の動きを変えない）とするが、今の動きは 3 節の a のとおり「元から在れば後勝ちで 1 件目が黙って消える／無ければ確定の段で失敗する見込み」で、理由が記録に残らない。本 spec の後は `a/b` と `a_b` のように書いた値が違っても重なる。対象外のままにするなら、起票先を決めておく。
4. **空の `*.directory` の行と探索。** 対象外（今の動きを変えない）とされているが、探索を書き直すので「空の値の行は、見つかった行か」を決める必要が出る。今の動きを保つなら「見つかったものとして扱い、今どおり書庫ごと断る」になる（3 節の b）。「見つからなかった」と読むと、無印なら黙って続き、番号付きなら打ち切りになり、今の「断る」から変わる。
5. **取り出し元が重なる 2 つの同梱。** 例: `balloon0.source.directory,extra` と `balloon1.source.directory,extra/bal1`。階層付きを許すと、片方がもう片方の配下になる組み合わせが書ける。`in_folder` を前方一致に広げるだけだと、`extra/bal1` の中身は両方の同梱へ入る。同じ取り出し元を 2 つの同梱が指すことは今も書けて、両方へ入る。要件はこの場合を書いていない。
6. **取り出し元の途中のフォルダが本体の側に空で残るか。** 例: 取り出し元が `extra/bal1` で、`extra/` の中身が `bal1/` だけの書庫。書庫が `extra/` のフォルダのエントリを持っていれば、それは取り出し元の配下ではないので本体の側に空の `extra/` が作られ、持っていなければ作られない。要件 2 の 4 項は「配下でないものは本体の側」とするので前者は要件どおりだが、今の 1 階層の実装は「中身の無い抜け殻を本体に生やさない」ために取り出し元のフォルダのエントリそのものを本体から除いている（`body_placement` の説明）。書庫の作り方で結果が変わってよいかを決める。
7. **知らせの並びのテスト（要件 8 の 3 項）の置き場所。** 知らせを組むのは `procedure.rs` で、そのテストは `procedure.rs` の末尾の `#[path]` で繋がる 3 本（`procedure_test_support.rs`・`procedure_tests.rs`・`procedure_branch_tests.rs`）である。新しい兄弟のテストのファイルを足すと `procedure.rs` に 1 行足すことになり、「`procedure.rs` は無改変」の約束に触れる。取れる形は、⑴ 既存の `procedure_tests.rs` に 1 本足す（並走の `areka-P0-install-live-target-hazards` と同じファイルを触る恐れ）、⑵ `areka-nar` の側で `InstallOutcome.installed` の並びを確かめ、知らせへの写しは既存の `ghost_with_balloon_lists_body_first_then_the_companion` と `judge_tests.rs` が順を保つことを確かめているのでそれに任せる、の 2 つ。⑵ は要件 8 の 3 項の文（知らせの並びを確かめる）を緩めて読むことになる。
8. **開発用の道具の追随（要件 2 の 8 項）の確かめ方。** `fold-samples.rs` は実行の例で関数単位のテストを持たず、階層付きの取り出し元を持つ検体も無い（0 体）。`strip_folder` を複数段へ広げても、今の検体では新しい枝を一度も通らない。確かめる手を足すか（道具の中の写像を、テストから呼べる場所へ出すなど）、読んで確かめるだけにするかを決める。なお、この道具の写像はインストールの結果と突き合わせる**独立した答え合わせ**なので、`areka-nar` の判定の関数をそのまま呼ぶ形にすると答え合わせにならない。
9. **起動の側との一時的なずれ。** 本 spec の後、`balloon.directory,extra\bal1` のゴーストは `balloon/extra_bal1/` に入るが、起動の側の `companion_balloon` は値を置き換えずに `extra\bal1` という名前でバルーンを探すので見つからず、`warn!`（`companion_balloon_not_found`）を出して次の候補へ進む。今はこの書庫は書庫ごと断られるので、悪くなる書庫は無い。`areka-P0-ghost-standard-balloon` が着地するまでこのずれが残ることを、`doc/COMPAT_ARCHITECTURE.md` §8 に書くかを決める。番号付きの同梱（`balloon0` だけを書いたゴースト）が起動で選ばれないのは今も同じである。

## 8. 設計への申し送り（選択肢のうち材料が揃っているもの）

- 断る理由は既存の `InvalidDirectoryName` を使い、種類を増やさない（増やすと `judge.rs` が変わる）。
- 取り出し元は `/` 区切りに正規化した文字列で持つ（`terms.rs` の本体が変更 0 になる）。
- `InstallManifest` に欄を足さない（`crates/areka/src/install/judge_tests.rs` が `InstallManifest` を欄を並べて組んでおり、欄を足すと並走の相手が触るファイルが変わる）。
- `manifest.rs` の `COMPANION_SUFFIXES` の上の `// ukadoc:` の 2 行は残す（台帳の 2 行の根拠）。
- `InstallManifest.companions` と `build_plan` の説明文の「接頭辞の名前順」を「探索の順」へ直す。

## 9. 要件の討議での扱い（2026-10-04）

### 9.1 開発者の決め: `*.directory` の区切りは置き換えず、今どおり断る

- 開発者の言葉: 「a/b なんてそもそも許容されるのか？ファイル名以外許可されないでしょう？」「バグを受け入れる必要はないと思う」。
- ukadoc の `*.directory` の定義「パス区切りは使えない（「_」に置換される）」のうち、「使えない」を採る。括弧の中は SSP の後始末で、areka は真似ない。
- brief の Desired Outcome ⑶ と、前の版の要件 4（`_` への置き換え・起動の側と共有する規則・置き換えの記録）を取り下げた。要件 4 は「今どおり断る」に書き換えた。
- この決めで、本文書の次の部分は要らなくなった。
  - 5.1 節（置き換えの関数の置き場所）。共有する関数は 0 個。`crates/areka-parsers/` に触る理由は無くなった（`..` と空の段の取り除きは `areka-nar` の中に置ける）。
  - 7 節の 1（`../escape` は今どおり断る）、3（書いた値が違うのに同じ宛先になる場面は生まれない）、9（起動の側とのずれは生まれない）。
  - 3 節の d の「結果が変わる既存のテスト」のうち `refuses_a_companion_directory_that_is_not_a_one_level_name` は変わらない（変わるのは 3 本）。
  - 5.4 節の記録の種類は「取り除いた」「探索で読まなかった」の 2 つで足りる。

### 9.2 要件へ直接書き足した（自明な直し）

- 7 節の 2: `*.directory` の行が無い断片は、打ち切りの後ろでも今どおりの読み飛ばしの記録にする（要件 6 の 3・4 項）。
- 7 節の 4: 空の値の `*.directory` の行は「見つかった」とし、今どおり書庫ごと断る（要件 1 の 2 項）。
- 7 節の 5: 取り出し元が重なる 2 つの同梱は、それぞれが自分の配下の全てを受け取る（要件 2 の 9 項）。
- 7 節の 6: 取り出し元の途中のフォルダは、本体の側へ置くものが配下に無ければ作らない（要件 2 の 4 項）。
- 7 節の 7: 並びは「何をインストールしたか」の列で確かめ、知らせへの写しは既存のテストに任せる（要件 8 の 3 項）。`procedure.rs`・`judge.rs` とそのテストは変更 0。

### 9.3 設計で決める

- 5.2〜5.4 節の選択肢（取り出し元の持ち方・探索と記録の組み方・記録の種類）と、6 節の調べもの。
- 7 節の 8: 開発用の道具の広げた枝をどう確かめるか。

### 9.4 本 spec の外に残るもの（完了の棚卸で起票する）

- `type` が `balloon`・`supplement` の書庫に書かれた同時インストールの指定。ukadoc は「アーカイブ自身のtypeを問わず機能する」と書くが、areka は読まない。担当の spec は今は無い（0 本）。
- 同じ `*.directory` の値を 2 つの同梱が書いた場合（3 節の a）。今の動きのままで、本 spec はこの場面を増やさない。