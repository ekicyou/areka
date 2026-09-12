# 境界の不変を機械で確認した記録（タスク 6.1）

対象仕様: `areka-P0-charset-canon`
対象要件: 9.8 / 12.1 / 12.2 / 12.3 / 12.4 / 12.5
実施日: 2026-09-12
実施した版: `f21d284c`（タスク 5.2 まで着地済み）＋本タスクで見つかった後退の是正 1 件
（`crates/areka/src/emo2_boot/assets_tests.rs`・6 節）を作業ツリーに載せた状態
比較の基点: `1712561b`（本仕様の tasks.md 生成時点＝実装が 1 行も入っていない版）
実行環境: Windows 11 Pro 10.0.26200 / x86_64-pc-windows-msvc / ワークツリー
`C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d`

この記録は「変えない約束」を検査で確かめたものである。書いてある数はすべて実際に走らせて
得たもので、見込みや引き写しではない。

---

## 0. 結論（先に）

- 形の変更（型と引数）で壊れたテストは無い。2 クレートのコンパイル確認は緑。
- 境界の差分 0 は 5 種すべて成立。**「差分なし」を書く前に、そのパスが本当に追跡されている
  ことを毎回確かめた**（確かめずに書いていたら、固定物の確認は嘘になっていた。後述 4 節）。
- 既存の公開ログ行は 1 行も変わっていない。本仕様のログは追加のみ。
- 触った `.rs` はすべて 1,000 行未満（最大 983）。行数番人の例外表は不変。
- **本タスクは本仕様が作り込んだ後退を 1 件見つけた**——タスク 4.1 が一時パスの共通窓口を
  迂回し、別クレートに住む番人が 3 本赤になっていた。**是正済み**（6 節）。
- 是正後のワークスペースの赤は **1 件だけ**で、それは本仕様の着手前から落ちている既存の赤
  （`areka --test smoke_boot_loop_exit`）である。
- **この後退がタスク別レビューを全部通り抜けた理由そのものが、本タスクの最大の収穫である。**
  独立した節（7 節）に書いた。完了ゲートと今後の spec はここを読むこと。

---

## 1. 形の変更でテストが壊れていないか（コンパイル確認 2 本）

実行に 32bit（i686）成果物を要するテストは x64 の通常走行では動かないが、コンパイルは常に
走る。`Shiori3Client::new` の引数・`build_request` の戻り型・`parse_response` の第 2 引数を
変えたので、ここが壊れていないことはこの 2 本でしか見えない。

| コマンド（逐語） | 終了コード | 結果 |
|---|---|---|
| `cargo test -p shiori-host32-host --no-run` | 0 | 緑 |
| `cargo test -p areka-ghost --no-run` | 0 | 緑 |

生成されたテスト実行体（本数が減っていたら、あるテストファイルが黙ってコンパイルされなく
なったことを意味する）:

- `shiori-host32-host`: **6 本**＝`unittests src\lib.rs` 1 本＋`tests/*.rs` **5 本**
  （`error_paths.rs` / `lifecycle_cyclic_e2e.rs` / `lifecycle_kill_e2e.rs` /
  `shiori_load_e2e.rs` / `shiori_request_e2e.rs`）。期待どおり 5 本。
- `areka-ghost`: **2 本**＝`unittests src\lib.rs` 1 本＋`tests/ghost.rs` 1 本
  （`snapshot_capture_test.rs` はこの `ghost.rs` に束ねられている）。

警告は両方とも `patch pasta_core ... was not used in the crate graph` の 1 件のみ。これは
本仕様と無関係の既存警告である。

### 予告されていた警告 3 件が消えていること

タスク 3.2 が `LOG_TARGET` / `default_charset` / `initial_charset` に未使用の警告 3 件を
出すと予告し、3.4 が本番の起動経路から呼んで消すことになっていた。増分ビルドだと警告が
再表示されないので、2 ファイルの更新時刻だけを進めて（中身は変えていない。確認後の
`git status` は空）強制的に再コンパイルさせた。

```
touch crates/areka-ghost/src/shiori_wiring.rs crates/shiori-host32-host/src/charset.rs
cargo check -p areka-ghost -p shiori-host32-host --all-targets
```

終了コード 0。`dead_code` / `never used` / `never read` を含む行は **0 件**。予告どおり
3.4 で消えている。

---

## 2. 書式と全ターゲットの型検査

| コマンド（逐語） | 終了コード | 結果 |
|---|---|---|
| `cargo fmt --check` | 0 | 緑（差分なし） |
| `cargo check --workspace --all-targets` | 0 | 緑。警告は前述の `pasta_core` 1 件のみ |

---

## 3. ワークスペースのテスト

### 3.1 まず分かったこと: 既定の `cargo test --workspace` は途中で止まる

```
cargo test --workspace
```

終了コードは **101**。しかし `test result:` 行は **3 本**しか出ていない。既存の赤
（`areka --test smoke_boot_loop_exit`）で cargo がそこで打ち切るためである。つまり
**既定のまま走らせると、その後ろのクレートは 1 つも走っていないのに「走った」ように見える。**
以後の確認はすべて `--no-fail-fast` を付けた走行で行った。これが 6 節の後退を隠していた
仕組みそのもので、経緯と手当ては 7 節に独立して書いた。

### 3.2 全数の走行（是正前）

```
cargo test --workspace --no-fail-fast
```

終了コード **101**。

| 項目 | 数 |
|---|---|
| テストターゲット数（`test result:` 行） | 103 |
| 成功 | 7,224 |
| 失敗 | **4** |
| 無視 | 40 |

失敗 4 件の内訳は 6 節。うち 3 件が本仕様由来で、是正した。

### 3.3 全数の走行（是正後・これが今日の確定値）

同じコマンドを、6 節の是正を作業ツリーに載せた状態で走らせ直した。

```
cargo test --workspace --no-fail-fast
```

終了コード **101**（残る既存の赤 1 件のため。緑ではないが、緑であってはいけない——
既存の赤を隠すために `--no-fail-fast` の終了コードを握り潰さない）。

| 項目 | 是正前 | 是正後 |
|---|---:|---:|
| テストターゲット数（`test result:` 行） | 103 | **103** |
| 成功 | 7,224 | **7,227** |
| 失敗 | 4 | **1** |
| 無視 | 40 | **40** |

残る失敗は `areka --test smoke_boot_loop_exit` の 1 本だけで、これは本仕様の着手前から
落ちている既存の赤である（6.2 項）。緑のターゲットは 103 中 **102**。

あわせて走らせ直したもの:

| コマンド（逐語） | 終了コード | 結果 |
|---|---|---|
| `cargo fmt --check` | 0 | 緑 |
| `cargo check --workspace --all-targets` | 0 | 緑（警告は既存の `pasta_core` 1 件のみ） |

`log-capture-kit` の全 7 ターゲット（是正が直接効く先）:

| ターゲット | 件数 | 結果 |
|---|---:|---|
| `unittests src\lib.rs` | 40 | 緑 |
| `tests\temp_path_guard_test.rs` | **16** | **緑**（是正前は 13 緑 / 3 赤） |
| `tests\file_length_guard_test.rs` | 6 | 緑 |
| `tests\with_default_guard_test.rs` | 24 | 緑 |
| `tests\workspace_scan_test.rs` | 18 | 緑 |
| `tests\capture_calibration_test.rs` | 1（無視 2） | 緑 |
| Doc-tests | 5 | 緑 |

**この表の出どころはワークスペース走行である**（11 節の `cargo test -p log-capture-kit` 単体
走行では `unittests` が **30**・Doc-tests が **4** になる）。矛盾ではなく feature の効き方の
違いで、`log-capture-kit` の `env-filter` feature（既定 off・有効化するのは `wintf` のみ）が
ワークスペース走行では cargo の feature 統合で on になり、`filter` モジュールのテスト
10 本と doc-test 1 本が増えるためである。番人 2 本（`temp_path_guard_test` 16・
`file_length_guard_test` 6）と残り 3 ターゲットはどちらの走り方でも同数で、7 ターゲット
すべて緑という結論も同じ。

### 3.4 本仕様が「適用前と同一であること」を確かめる対象の件数

いずれも期待どおりで、適用前から動いていない。

| 対象（コマンドは上記 1 本の走行から抜粋） | 件数 | 結果 |
|---|---|---|
| `shiori-host32-host` `unittests src\lib.rs` | 126 | 緑 |
| `shiori-host32-host` `tests/*.rs` 5 本（`error_paths` 2 / `lifecycle_cyclic_e2e` 2 / `lifecycle_kill_e2e` 1 / `shiori_load_e2e` 2 / `shiori_request_e2e` 2） | 計 9 | 緑 |
| `areka-ghost` `unittests src\lib.rs` | 117 | 緑 |
| `areka-ghost` `tests\ghost.rs` | 40 | 緑 |
| `areka-kanade` `unittests src\lib.rs` | 286 | 緑 |
| `areka-kanade` `tests\kanade.rs` | 50 | 緑 |
| `areka-parsers` `unittests src\lib.rs` | 424 | 緑 |
| `areka` `unittests src\main.rs`（本体・UTF-8 ゴーストの決定論群を含む） | 1,573（無視 2） | 緑 |
| `areka` `tests\emo2_fixture_e2e_test.rs` | 3 | 緑 |
| `areka` `tests\emo2_real_run.rs` | 1 | 緑 |
| `ukadoc-survey` `unittests src\lib.rs`（網羅台帳の常設検査） | 548 | 緑 |
| `ukadoc-survey` `tests\consistency.rs` | 38 | 緑 |
| `ukadoc-survey` `tests\cli_streams.rs` | 5 | 緑 |
| `ukadoc-survey` Doc-tests | 5 | 緑 |
| `log-capture-kit` `tests\file_length_guard_test.rs`（行数番人） | 6 | 緑 |
| `shiori-host32-ipc` `unittests src\lib.rs` | 20 | 緑 |
| `temp-path-kit` `unittests src\lib.rs` | 8 | 緑 |

### 3.5 どのテストがどの要件を担っているか

要件 12.1 の「UTF-8 ゴーストの要求バイト列が変わらない」は、本仕様が新しく導き直したので
はなく、タスク 2.3 と 4.1 が固定した検査が受け持っている。6.1 はそれが実在して緑であること
を確かめた。

| 要件 | 受け持つ検査（file:line ではなく何の検査かで指す） | 結果 |
|---|---|---|
| 12.1（要求バイト列） | `shiori3_charset_tests.rs` の「適用前のバイト列と逐語で等しい」テスト（`utf8_request_equals_the_pre_spec_byte_stream`）。定数は `4db516fe` の組立順から手で書き下したもので、今日の符号化器を 1 度も通していない | 緑（`shiori-host32-host` 126 本の内側） |
| 12.1（surfaces.txt の解析結果） | `decode_charset_tests.rs` の 4 種の固定物が同じ解析結果を産むテスト群（`four_charset_fixtures_produce_the_same_parse_result` ほか計 4 本）。空同士の等値で緑にならないよう「日本語が 3 か所へ実際に届く」ことを別に主張している | 緑（`areka-parsers` 424 本の内側） |
| 12.1（本番読取点が復号を通ること） | `measure_tests.rs` と `assets_tests.rs` の兄弟テスト各 1 本（退化させると `stream did not contain valid UTF-8` で赤になることを 4.1 が実走確認済み） | 緑（`areka` 1,573 本の内側） |
| 5.3（片道イベントの応答を採用の根拠にしない） | `client.rs` の in-file テスト。`pub fn notify(` 以降の本体を切り出し、禁止語 `parse_and_note` / `note_response` / `parse_response` が 1 つも無いことを見る（＝採用も解析もしない）。対になる `adoption_in_an_answered_event_reaches_the_next_one_way_request` が「応答待ちで採用した文字コードは次の片道要求に載る」側を固定する | 緑（`shiori-host32-host` 126 本の内側） |
| 5.1（交渉状態が接続に 1 つだけ・呼出ごとに作り直さない） | `areka-kanade` の `the_connection_owns_one_negotiator_and_never_mints_one_per_call`（`real.rs` の本文に `CharsetNegotiator::new` が無く、`&mut self.negotiator` がちょうど 2 回＝応答待ちと片道の 2 入口） | 緑（`areka-kanade` 286 本の内側） |
| ステータス扱い（204 / 400 / 500 でもヘッダを採用する） | `charset_tests.rs` の `note_response` 表 7 行を 1 本ずつ固定したテスト群（21 本） | 緑（`shiori-host32-host` 126 本の内側） |
| 9.8（x86 ビルドへの依存 0） | 上記の仕様テストはすべて `src/` の兄弟ファイルに置かれ、`tests/*.rs` に無い。x64 の通常走行で全数が走っている（3.4 の件数がその証拠） | 成立 |

---

## 4. 境界の差分 0

**書く前に、そのパスが本当に追跡されていることを確かめた。** 存在しないパスへの
`git diff` は空で終了コードも 0 になるので、確かめずに書いた「差分なし」は何も言って
いないのと同じである。

実際にこの罠を踏みかけた。要件 12.1 が emo2 の固定物を `fixtures/emo2/ghost/master/descript.txt`
と書いているのでそのまま `git diff -- fixtures` を掛けたところ、変更 0 件と出た。だが
`git ls-files -- fixtures` は **0 件**で、そんなパスは存在しない。本当の場所は
`crates/pilot/examples/shiori-host-32/fixtures/emo2/`（110 ファイル）である。

| 対象 | 実在の確認（コマンドと結果） | 差分の確認（コマンド） | 変更ファイル数 |
|---|---|---|---|
| 12.2 32bit helper | `git ls-files -- crates/shiori-host32-helper` → 8 件 | `git diff --stat 1712561b..HEAD -- crates/shiori-host32-helper` | **1**（下記の例外 1 行のみ） |
| 12.2 IPC | `git ls-files -- crates/shiori-host32-ipc` → 2 件 | `git diff --name-only 1712561b..HEAD -- crates/shiori-host32-ipc` | **0** |
| テスト専用の surfaces.txt 読取（design 名指しの 2 件） | `git ls-files -- crates/areka-emo-compose/src/world.rs crates/areka-seriko/src/resolve.rs` → 2 件 | 同上 | **0** |
| 12.3 完了仕様の文書（`completed/` 全体） | `ls -d .kiro/specs/completed/*/` → 174 件 | `git diff --name-only 1712561b..HEAD -- .kiro/specs/completed` | **0** |
| 12.3 名指しの完了仕様 8 件（host32-request / shiori-protocol / parser-foundation / ukadoc-survey-\* 5 本） | `git ls-files` で 8 ディレクトリすべて追跡を確認 | 同上（8 パスを列挙） | **0** |
| 12.4 行数番人の例外表 | `git ls-files -- crates/log-capture-kit/tests/file_length_guard_test.rs` → 1 件 | 同上 | **0** |
| 12.1 emo2 の固定物 | `git ls-files --error-unmatch crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/descript.txt` → 成功 / 配下 110 件 | `git diff --name-only 1712561b..HEAD -- crates/pilot/examples/shiori-host-32/fixtures/emo2` | **0** |
| 12.1 ワークスペース全体の fixture と testdata | `git ls-files -- '*fixtures/*'` → 259 件 | `git diff --name-only 1712561b..HEAD -- '*fixtures/*'` および `-- '*testdata/*'` | **0** / **0** |

上の表は当初 `1712561b..HEAD`（コミット済みの範囲）で取った。6 節の是正を作業ツリーに
載せたあと、**基点に `..HEAD` を付けない `git diff 1712561b -- <path>` で全項目を取り直し、
すべて同じ結果であることを確認した**（是正が境界の外へはみ出していないことの確認でもある）。
そのとき合わせて、一時パス番人の例外表
`crates/log-capture-kit/tests/temp_path_guard_test.rs` も変更 **0 件**であることを確かめた
（＝迂回を例外表で正当化していない。6 節）。

### 12.2 の唯一の例外（説明コメント 1 行）

要件 12.2 が明示的に認めた 1 行だけが変わっている。

```
git diff 1712561b..HEAD -- crates/shiori-host32-helper/src/shiori_proxy.rs
```

差分は `//!` の 1 行の差し替えのみ（`request` は UTF-8（下流・本仕様非呼出） →
`request` は**任意の文字コードのバイト列**（意味は x64 側＝`areka-P0-charset-canon` の
交渉結果。本モジュールは解釈しない））。

コメント以外の差分が無いことを機械で確かめた:

```
git diff -U0 1712561b..HEAD -- crates/shiori-host32-helper/src/shiori_proxy.rs \
  | grep -E '^[+-]' | grep -v '^[+-][+-][+-]' | grep -cvE '^[+-]\s*(//!|///|//)'
```

結果 **0**（全差分行は ± 合わせて 2 行、いずれもコメント）。挙動とバイト列に関わる変更は 0。

### テスト専用の surfaces.txt 読取の実数（design の名指し 2 件は少ない）

design はテスト専用読取を 2 件（`areka-emo-compose/src/world.rs`・`areka-seriko/src/resolve.rs`）
としか名指していないが、実数はもっと多い。零は実数で書く。

調べ方: `crates/` 配下の `.rs` から `std::fs::read(` / `std::fs::read_to_string(` の
呼出行を全数挙げ、その前後 4 行に `surfaces.txt` または `SURFACES_TXT` が現れるものを
読取点とした（`vendors/` は対象外）。

- 読取点の総数: **16 か所 / 14 ファイル**
  - 本番: **2 か所**（`areka/src/placement/measure.rs:337`・`areka/src/emo2_boot/assets.rs:281`）
    — どちらも本仕様が復号に載せ替えた
  - 本仕様の兄弟テストが新たに足したもの: **2 か所**（`measure_tests.rs:973` と `:974`。
    後者は「適用前の読み方」を比較のために故意に残している）
  - それ以外のテスト・サンプル: **12 か所 / 11 ファイル**
- 読取点を持つ 14 ファイルのうち本仕様が触ったのは **3 ファイル**（`measure.rs`・
  `assets.rs`・`measure_tests.rs`）だけで、**残る 11 ファイルは差分 0**。design が名指した
  2 件（`areka-emo-compose/src/world.rs`・`areka-seriko/src/resolve.rs`）はその 11 に含まれる。

（実装ノート 4.1 は「本番 2・テスト/サンプル 15 の計 17 か所」と書いているが、上の実測
（呼出点 16 / ファイル 14）とどちらの単位でも一致しない。統合担当は数え方をそろえて
書き直すこと。零の成立そのものには影響しない。）

---

## 5. 行数（12.4）

行数番人が見るのは `crates/**/*.rs` で、上限は 1,000 行（`> 1000` で赤）。例外表
`OVER_LIMIT_ALLOWED` は 11 件で、その件数は `OVER_LIMIT_ALLOWED_COUNT = 11` に逐語で
持たれている。**この表も件数も本仕様は触っていない（4 節の差分 0）。**

本仕様が触った 38 ファイルすべての行数を測った（`git diff --name-only 1712561b` の
各ファイルに `wc -l`。基点に `..HEAD` を付けないので 6 節の是正を載せた作業ツリーの姿を
測っている）。番人の対象である `.rs` の上位:

| 行数 | ファイル |
|---:|---|
| 983 | `crates/areka/src/placement/measure_tests.rs` |
| 963 | `crates/areka-parsers/src/package/resolve.rs` |
| 919 | `crates/areka/src/emo2_boot/assets_tests.rs`（是正前 941。共通窓口へ寄せて 22 行減） |
| 775 | `crates/shiori-host32-host/src/client.rs` |
| 710 | `crates/shiori-host32-host/src/shiori3.rs` |
| 702 | `crates/areka-kanade/src/shiori/real_tests.rs` |
| 678 | `crates/areka-ghost/src/runtime.rs` |
| 590 | `crates/areka-ghost/src/shiori_inproc.rs` |
| 584 | `crates/shiori-host32-helper/src/shiori_proxy.rs` |
| 565 | `crates/areka-parsers/src/shell/decode.rs` |

**1,000 行以上の `.rs` は 0 件**（最大 983）。`cargo test -p log-capture-kit --test
file_length_guard_test` も 6 本すべて緑。

1,000 行を超えているファイルは 2 つあるが、いずれも番人の対象外（`.rs` ではない）で、
本仕様が行を足したものでもない: `doc/ukadoc-coverage/ledger/shiori.toml`（16,160 行）と
`ledger/assets.toml`（8,312 行）。

### 余裕が少ない 2 本（申し送り）

- `crates/areka/src/placement/measure_tests.rs` — **983 行（残り 17 行）**
- `crates/areka/src/emo2_boot/assets_tests.rs` — **919 行（残り 81 行）**。是正前は 941 行
  （残り 59）だったので、共通窓口へ寄せたことで余裕が 22 行増えた。それでも次に何かを足す
  人が当たりうる位置であることは変わらない。

次にこの 2 本へ何かを足す人は上限に当たる。同じクレートに分割の先例
（`placement_prepare_tests.rs` / `placement_shared_test_support.rs`）があるので、引受先を
先に決めておくこと。

---

## 6. 赤の帰属と、見つかった後退 1 件の顛末

是正前の全数走行で赤は 4 件だった。内訳は「本仕様が作った 3 件」と「本仕様の着手前から
落ちている 1 件」である。前者は是正し、後者は残っている。

### 6.1 本仕様が作った後退 1 件（タスク 4.1 が作り、6.1 が見つけ、是正済み）

**何が起きたか。** タスク 4.1 が `assets_tests.rs` に検査を足すとき、一時ディレクトリを
自前の `TempDir` 構造体で組み、`std::env::temp_dir()` を直に叩いた。この迂回を見張る番人が
別のクレート（`log-capture-kit`）に住んでおり、3 本が赤になっていた。

```
cargo test -p log-capture-kit --test temp_path_guard_test
```

是正前は 16 本中 **3 本が赤**
（`no_temp_dir_entry_point_lives_outside_the_gateway_and_the_allow_table`
／`the_measurement_is_not_vacuous_and_matches_the_allow_table`
／`dropping_a_known_exception_turns_the_guard_red`）。

赤の中身（番人の言葉そのまま）:

> テスト用の一時パスは共通窓口（temp-path-kit の TempPath）から取ること。入口を直接叩くと
> プロセス間で名前が衝突し、同じテストを複数プロセスで同時に走らせたときに互いの一時
> ファイルを奪い合って落ちる
>   `crates/areka/src/emo2_boot/assets_tests.rs:848` （語: `env::temp_dir(`）

**新しい迂回であることの裏取り**（これが「本仕様が作った」ことの証拠であり、是正後も
残すべき記録である）:

- 適用前（`1712561b`）の `assets_tests.rs` に `temp_dir` は **0 件**
  （`git show 1712561b:crates/areka/src/emo2_boot/assets_tests.rs | grep -n temp_dir` が空）。
- `git log -S "env::temp_dir" -- crates/areka/src/emo2_boot/assets_tests.rs` は
  **`899a0e33`（タスク 4.1「本番の surfaces.txt 読取 2 経路をファイル層の復号に載せる」）**
  ただ 1 件を返す。

同じタスク 4.1 がもう一方に足した `measure_tests.rs` は元から例外表に載っているので赤に
ならず、`assets_tests.rs` だけが新しい迂回になっていた。

**どう直したか（採った道と、退けた道）。** タスク 6.1 はコードを書かないので、実装は
統合担当（コントローラ）が行った。取りうる道は 2 つあった。

1. **採った道**——`assets_tests.rs` の自前 `TempDir` を共通窓口へ寄せる。
   `temp_path_kit::TempPath` を import し、`TempPath::new("areka-boot-assets-charset")` と
   `root.path()` で一時パスを取る形にした。自前の構造体とその `Drop` は削除。
   `temp-path-kit` は `areka` の dev-dependency に既にあったので依存の追加は無い。
2. **退けた道**——例外表 `ALLOWED_ENTRY_POINT_USES` へ種別と理由付きで足す
   （表・`ALLOWED_COUNT`・種別ごとの件数の 3 か所の編集）。**退けた理由**: 番人自身の
   失敗メッセージが窓口へ寄せる方を薦めており、例外を足すと迂回そのものを「前例」として
   正当化してしまう。番人は「やむを得ない場合」に限って例外を認めているが、ここは
   やむを得ない場面ではない——同じことが共通窓口で普通に書ける。

**是正の副作用**（いずれも望ましい向き）: `assets_tests.rs` は 941 行 → **919 行**
（22 行減・上限までの余裕が 59 → 81）。例外表 `ALLOWED_ENTRY_POINT_USES` は**触っていない**
（`git diff --name-only 1712561b -- crates/log-capture-kit/tests/temp_path_guard_test.rs`
が 0 件。迂回を正当化していないことがこれで見える）。

**是正後の確認**（3.3 項に全数を載せた）:

| コマンド（逐語） | 結果 |
|---|---|
| `cargo test -p log-capture-kit --test temp_path_guard_test` | **16 passed / 0 failed** |
| `cargo test -p log-capture-kit`（全 7 ターゲット） | すべて緑 |
| `cargo test -p areka --bins` | 1,573 passed / 2 ignored（是正前と同数） |
| `cargo fmt --check` | 終了コード 0 |
| `cargo check --workspace --all-targets` | 終了コード 0 |
| `cargo test --workspace --no-fail-fast` | 終了コード **101**・103 ターゲット中 **102 緑・赤は既存の 1 件のみ** |

**注記（改行コード）**: 是正の編集で `assets_tests.rs` が LF で書かれ、git が
`LF will be replaced by CRLF the next time Git touches it` と警告する。行数の数え方にも
番人にも影響しない（どちらも行を数えるだけで、改行の綴りを針にしていない）が、この
リポジトリの作業ツリーは CRLF なので、コミット時に正規化されることを承知しておくこと。

### 6.2 本仕様と無関係な既存の赤 1 件（**09-12 は残っていた・09-13 に再現せず＝間欠**）

```
cargo test -p areka --test smoke_boot_loop_exit
```

`skeleton_boots_with_real_ghost_windows_and_exits_zero` が `bevy_ecs` の
`Entity despawned ... ID 21v0` で panic（`1 passed; 1 failed`・終了コード 101）。

本仕様の着手前から落ちている。親セッションが `1712561b`（実装が 1 行も入っていない版）に
一時ワークツリーを立て、同じテストを走らせて同じ落ち方（`1 passed; 1 failed`・exit 101）を
確認済み。実装ノート 3.4 も同じ結論を記録している。**本仕様に帰属させない。**

**追記（2026-09-13・完了ゲートの実測）**: この赤は**間欠**だった。最終検証で
`cargo test -p areka --test smoke_boot_loop_exit` を 3 回続けて走らせ、**3 回とも
`2 passed; 0 failed`・終了コード 0**。同日のワークスペース走行
（`cargo test --workspace --no-fail-fast`）も **103 ターゲット・7,228 passed・失敗 0・
終了コード 0** で、この 1 件を含めて赤は 1 つも出ていない。

したがって**下の 7 節が定めた完了ゲートの判定文は使えない**。「103 ターゲット・
失敗 1 件・その 1 件が着手前から赤」を条件にすると、実在しない赤を探し続けることになる。
**正しい判定文は「103 ターゲット・失敗 0・終了コード 0」**である。落ちる版が在ることは
事実なので（09-12 の本節の実測と、`1712561b` の一時ワークツリーでの再現）、赤が出たときは
まず `smoke_boot_loop_exit` の 1 件かどうかを見て、そうであれば本仕様に帰属させない——
という読み方に変える。間欠の原因は特定していない（この repo には「Defender の再スキャンが
協調テストループを飢餓させる」既知の罠がある）。

---

## 7. 手順の欠陥: 既存の赤 1 件が、その後ろの全ターゲットを隠す

**完了ゲートと今後の spec はこの節を読むこと。** 6.1 の後退が一度も赤として見えないまま
タスク別レビューを全部通り抜けたのは、実装者やレビュアーの不注意ではなく、検証の手順に
穴があったからである。

### 何が起きていたか

`cargo test --workspace` は既定で fail-fast である。あるテストターゲットが落ちると、cargo は
そこで打ち切り、**以降のクレートのテストは 1 つも走らない**。このリポジトリには本仕様と
無関係な既存の赤（`areka --test smoke_boot_loop_exit`）があり、`areka` はターゲットの並びで
かなり前に来る。結果として:

```
cargo test --workspace          → 終了コード 101・test result: 行はわずか 3 本
cargo test --workspace --no-fail-fast → 終了コード 101・test result: 行は 103 本
```

**同じワークスペースで、片方は 3 ターゲット、もう片方は 103 ターゲット。** 差の 100
ターゲットの中に `log-capture-kit` の番人 7 本が入っている。つまり、既定のまま走らせる限り
番人は**一度も走っていない**のに、走らせた側には「テストを流した」という感触だけが残る。

### なぜタスク別のレビューでも見えなかったか

タスク 4.1 が触ったのは `areka` と `areka-parsers` である。触ったクレートを走らせる、という
まっとうな検証（`cargo test -p areka` 等）をしても、**番人は別のクレート
（`log-capture-kit`）に住んでいる**ので視界に入らない。ワークスペース全体を走らせれば
届くはずだったが、それは上の fail-fast で潰れていた。

**「触ったものだけを検証する」と「番人は触られる側ではなく見張る側に住む」が噛み合わない。**
横断的な番人（行数・一時パス・ログ語彙・台帳整合など）は、その性質上いつも他人のクレートに
いる。

### 何をすれば早く見つかったか

次のいずれか 1 つで捕まえられた。

1. **`cargo test --workspace --no-fail-fast` を使う。** 既存の赤があるリポジトリでは
   `--no-fail-fast` の無いワークスペース走行は検証として成立しない。終了コードは既存の赤の
   ぶん 101 のままなので、**終了コードだけでなく `test result:` 行の本数と失敗テスト名を
   見て判定する**こと。
2. **番人の住むクレートのターゲット一式を明示的に走らせる**——
   `cargo test -p log-capture-kit`（1 本を名指しした `--test file_length_guard_test` では
   隣の `temp_path_guard_test` に届かない。本仕様のタスク検証は実際に行数番人だけを
   名指ししており、一時パス番人が抜けていた）。

### 副次の罠: パイプが終了コードを隠す

本タスクでも一度踏みかけた。走行を別プロセスに投げた側の報告が「exit code 0」だったのに、
cargo 自身の終了コードは **101** だった。`cargo test ... | tee` や `| tail` のようにパイプへ
繋ぐと、シェルが返すのは最後のコマンドの終了コードだけになる。本記録では毎回
`cargo test ... > out.txt 2>&1; echo "EXIT=$?" > exit.txt` の形で cargo 自身の終了コードを
別立てで捕まえている。この罠はこのリポジトリで過去にも 2 度起きている。

### 記録として残す判定基準

「ワークスペースのテストは緑だった」という報告は、次の 3 つが揃って初めて意味を持つ。

- `--no-fail-fast` が付いている
- `test result:` 行の本数が期待どおり（今日の値は **103**）
- 失敗したテスト名が列挙され、その 1 本 1 本について帰属（本仕様か既存か）が言えている

---

## 8. 既存の公開ログ行の語彙が変わっていないこと（12.5）

「変わっていない」は言い切りでは意味がないので、調べ方を書く。

### 方法 1: 仕様の全期間の差分をログ行で濾す

```
git diff -U0 1712561b..HEAD -- '*.rs' \
  | grep -E '^[+-]' | grep -v '^[+-][+-][+-]' \
  | grep -E '(trace!|debug!|info!|warn!|error!|event!|warn_then_debug!)'
```

ヒット **11 行**。内訳は **追加 10 行・削除 1 行**。

削除された 1 行は `crates/areka/src/placement/measure.rs` の `error!(` で、同じ塊の中で
そのまま足し直されている。`std::fs::read_to_string` を `std::fs::read(..).map(decode)` に
替えたのでクロージャの入れ子が 1 段深くなり、字下げが動いただけである。メッセージ
`"measure: shell surfaces.txt の読取に失敗"` もフィールド（`path` / `error`）も同一。

### 方法 2: ファイルごとのログ行数を基点と今日で突き合わせる

| ファイル | 基点 | 今日 |
|---|---:|---:|
| `areka-ghost/src/runtime.rs` | 18 | 18 |
| `areka-ghost/src/shiori_inproc.rs` | 15 | 15 |
| `areka-kanade/src/shiori/real.rs` | 6 | 6 |
| `areka/src/placement/measure.rs` | 9 | 9 |
| `areka/src/emo2_boot/assets.rs` | 2 | 2 |
| `areka-ghost/src/shiori_wiring.rs` | 0 | 2（**追加のみ**） |
| `shiori-host32-host/src/charset.rs` | 0 | 7（**追加のみ**） |

既にログを持っていたファイルは 1 つ残らず同数。増えたのは元が 0 の 2 ファイルだけ。

### 方法 3: 削除行のうち文字列リテラルを含むものを全数見る

`crates/*/src/*.rs` の削除行で `"` を含むものは 17 行。うちログのメッセージ文字列は
上記の `"measure: shell surfaces.txt の読取に失敗"` **1 行だけ**で、それは足し直されて
いる。残りはすべて doc コメントと in-file テストの `expect()` 文字列。

### 本仕様が足したログ（追加のみ・design §Monitoring の表と逐語一致）

target 2 つ・event 7 つ。実機確認（6.2）と後続 spec がこの綴りを grep する。

| target | event |
|---|---|
| `ghost-boot` | `charset_initial` / `charset_label_unresolved` |
| `shiori-charset` | `charset_switched` / `charset_label_unresolved` / `charset_forced_ignores_header` / `charset_unmappable_replaced` / `charset_invalid_bytes_replaced` |

design §Monitoring の表 7 行と 1 対 1 で一致している（`grep -oE 'LOG_TARGET: &str = "[^"]*"|event = "[^"]*"'` で全数を挙げて突合）。

**6.2 への注意**: debug 水準の行を点けるときの `RUST_LOG` は **target 名**で指定すること
（`RUST_LOG=info,shiori-charset=debug,ghost-boot=debug`）。モジュールパス
（`shiori_host32_host=debug`）では `charset_switched` が点かず、0 行が沈黙と区別できなくなる。

---

## 9. 要件が求める「零」の裏づけ

零は書かなければ満たしたことにならないので、調べ方と理由を添えて残す。

### 要件 1.3 — OS のロケール設定を読む箇所 0

調べ方: `crates/` 配下の `.rs` を `GetACP` / `GetOEMCP` / `GetUserDefaultLocale` /
`GetSystemDefaultLocale` / `GetLocaleInfo` / `GetUserDefaultLCID` / `SetThreadLocale` /
`LOCALE_` / `CP_ACP` / `CP_THREAD_ACP` で走査（`vendors/` は対象外）。

ヒット **33 行**。すべて文字コードの決定とは別の経路であり、`DefaultEncoding` の生成に
関わるものは **0 件**:

- `areka-emo-text/src/draw.rs` の `LOCALE_JA_JP = "ja-JP"` — DirectWrite に渡す
  ロケール文字列（字形選択）であって文字コードではない。
- `shiori-host32-helper/src/shiori_proxy.rs` と `pilot/examples/shiori-host-32/shiori_proxy.rs`
  の `CP_ACP` — SHIORI DLL の `load` に渡す**ディレクトリのパス**を ANSI で符号化する
  ための API。SHIORI の通信本文の文字コードとは別物。

本番で `DefaultEncoding` を生む場所は 2 つだけで、どちらもリテラルである:
`crates/areka/src/boot_config.rs:130`（`DefaultEncoding::Ansi`）と
`crates/areka/src/main.rs:672`（同）。ここで零が担保される。

### 要件 2.5 — descript の `charset` キーを SHIORI 通信の初期値に使う箇所 0

調べ方: `crates/` 配下の `.rs` で `"charset"` を全数走査。ヒット 13 行。本番の読み手は
2 つだけ:

- `areka-parsers/src/charset/prescan.rs:58` — ファイル層の復号のための先読み。消費点は
  `charset/decode.rs` の 1 つのみで、SHIORI 通信へは流れない。
- `shiori-host32-host/src/shiori3.rs:189` — SHIORI 応答の `Charset:` ヘッダ走査。これは
  wire のヘッダであって descript のキーではない。

`areka/src/placement/config.rs:586` のヒットは `#[cfg(test)]` の内側。残りはすべて
テストファイル。挙動の側でも `descript_charset_key_does_not_feed_shiori_encoding`
（`areka-parsers` 424 本の内側・緑）が零を固定している。

---

## 10. 統合担当への申し送り（完了ゲートと 6.2 への引き継ぎ）

`tasks.md` の `## Implementation Notes` に散っている ⚠ 印の申し送りを、6.1 の責任で 1 か所に
集めた。

**突合（数を主張する以上、数え直した結果を書く）**: `grep -c '⚠' tasks.md` は **9 件**。
下の ⑴⑵⑶⑹ がその 9 件を覆う（⑷⑸ は ⚠ 無しの別勘定）。対応は次のとおりで、取りこぼしは 0 件。

| `tasks.md` の行 | 出どころ | 本節のどこ |
|---|---|---|
| 175 | 2.2 SHIORI/4 直接呼出経路に記録が出ない | ⑵ の前半 |
| 197 | 3.3 解析エラー時に `note_response` へ届かない | ⑵ の後半 |
| 186 | 3.1 design の末尾空白の固定物が実現不能 | ⑶ |
| 190 | 3.2 `default_charset` の網羅 `match` が実現不能 | ⑶ |
| 210 | 4.1 テスト専用読取の名指し 2 件 vs 実数 | ⑶ |
| 216 | 4.2 固定物「3 種」vs「4 種」・変更行数見込み | ⑶ |
| 200 | 3.4 要件 2.7／12.1 の数え違い | ⑴ |
| 218 | 5.1 同上（要件・設計・実機手順の同時訂正） | ⑴ |
| **226** | **5.2 ⚠ 6.1 への申し送り・登記簿の要求側の備考** | **⑹** |

⚠ 印を持たないが引受先が明示されている申し送りも 2 件ある。どちらも本節に入れた——
台帳の陳腐化を見張る検査の不在（⑸）と、**本記録が名指しの引受先である採取フィクスチャの
3 つの既定**（⑺。`tasks.md:205` が「6.1 の境界不変の記録に残すこと」と書いている）。
さらに本タスク自身が見つけた検証手順の欠陥（⑼）を足してある。

**⑴ と ⑵ は裁定が要る。⑹ は登記の追記が要る。⑺ は将来の改変への警告。⑼ は引受先が要る。**

### ⑴ 要件本文の数え違い（裁定が要る・6.2 の期待値に直結）

要件 2.7 と 12.1 の除外「**最初の要求**の `Charset` ヘッダ値」（単数）は、要件 5.3
（片道イベントの応答は採用の根拠に用いない）と突き合わせると数が合わない。宣言を持たない
UTF-8 のゴーストが既定の Shift_JIS で送る要求は **2 本**（`OnInitialize` の片道イベント →
username 照会の応答待ちイベント）で、採用は 2 本目の応答で起き、UTF-8 になるのは
3 本目からである。

挙動は正しく要件 5.3 を守っている。誤っているのは要件・design の本文のほう。タスク 5.1 が
正典文書の登記には**事実（2 本）**を書いており、登記簿は後続 spec が典拠として引くので、
要件を鏡写しにすると偽が正典化する。レビュアーが boot 系列と起動スモークの期待値で独立に
数え直して 2 本を確認済み。

6.1 の実測でも前提は裏づけられた: emo2 の
`crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/descript.txt` は
`charset,UTF-8` のみで `shiori.encoding` を宣言していない（1 行目 `charset,UTF-8`・
7 行目 `shiori,pasta.dll`）。

**訂正が要るのは 3 か所同時**: 要件 2.7／12.1 の本文・design の該当行・**6.2 の実機手順の
期待値**（「最初の要求だけが違う」→「最初の 2 本」・採用 1 回の前に Shift_JIS の要求が 2 本）。

### ⑵ 記録の出ない後退が 2 経路残っている（要件 4.7／7.1 に対する設計由来の穴）

- **SHIORI/4 の直接呼出経路**（`areka-ghost/src/shiori_inproc.rs`）は交渉状態を持たない
  ので `decode_had_errors` の消費点が無く、不正なバイト並びの代替文字置換がこの経路だけ
  無記録で通る。従来は解析エラーだった経路が「無言で化けた文字を返す」に変わっている。
- **`parse_response` が解析エラーを返すと `note_response` へ到達しない**ので、ステータス行
  ごと壊れた不正バイト応答も記録が出ない。

タスク 5.1 は正典文書の登記でこの 2 経路を明示して書いており、普遍的な記録を主張する行には
していない。要件 7.1 の「ログなしの後退 0」を満たすかどうかは裁定が要る。

### ⑶ design 文書の訂正（実害なし・実装は正しい側に従っている）

- design §Testing Strategy の `resolve_tests.rs` 行が名指しした固定物
  「`shiori.forceencoding, Shift_JIS ` の空白を含む生の値」と tasks.md 3.1 の
  「末尾空白を含む形」は**実現不能**。`areka-parsers/src/kv/parse.rs` の値挿入行が無条件に
  前後空白を落とすため、行末の空白は転記層に到達し得ない。両文書を「`parse_kv` が返した値と
  1 バイト違わないこと」へ直すこと。要件 2.5／8.5 の充足には影響しない。
- design §shiori_wiring の `default_charset` を「2 腕の網羅 `match`」とする記述は文字どおり
  には**実現不能**。`DefaultEncoding` が `#[non_exhaustive]` なのでクレート外からの網羅
  match は E0004 になる。実装は 2 変種を名指ししたうえでワイルドカード腕を足した形。
- design のテスト専用読取の名指しは 2 件だが、実数は 4 節のとおり（呼出点 16・ファイル 14）。
  実装ノート 4.1 の「15」とも数え方が違う。**数え方をそろえて書き直すこと。**
- design の新規ファイル欄は surfaces.txt の固定物を「3 種」と書くが、同じ design の
  Testing Strategy と tasks.md 4.2 は「4 種」。実装は 4 種（正しい側）。design の変更行数
  見込み「+2」に対し実測 +5。いずれも実害なし。

### ⑷ 行数の余裕（5 節の再掲）

`measure_tests.rs` 983 行（残り 17）・`assets_tests.rs` 919 行（残り 81。6 節の是正で
941 行から 22 行減った）。次に足す人が上限に当たる。引受先を先に決めておくこと。

### ⑸ 台帳の陳腐化を見張る検査が存在しない

タスク 5.2 で、担当の 5 行の外にも本仕様の着地が偽にした記述が同じ台帳に **55 行**あった
（⑴ 6 行がシェル定義ファイルの読み方を旧状態で書き、状態を「三手／3 枚」と数えていた
＝正しくは二手・4 枚、⑵ 1 行が「SHIORI との受け渡しは UTF-8 固定」と書いていた、
⑶ 48 行が共有する定型文のキー列挙に本仕様が新たに読む 2 キーが欠けていた）。いずれも
是正済みだが、**これを見つけたのは人間の読み直しであって検査ではない**。台帳の記述が
実装の着地で偽になったことを自動で赤にする仕組みは無い。

### ⑹ 登記簿の要求側の備考が SHIORI/4 の直接呼出経路に触れていない（`tasks.md:226`・**6.1 宛て**）

網羅台帳（`doc/ukadoc-coverage/ledger/shiori.toml`）の**要求側の文字コードヘッダ**の備考は、
交渉経路（32bit helper 越しの SHIORI/3.0）だけを述べており、**SHIORI/4 の直接呼出経路が
このヘッダを UTF-8 固定で書く**ことに触れていない。

実害は小さい——応答側の行ともう 1 行がその経路を明示しているので、台帳を通読すれば
直接呼出経路の存在自体は分かる。だが**台帳は後続 spec が典拠として引く登記簿**であり、
要求側の行だけを引いた読み手は「要求のヘッダは常に交渉結果で決まる」と読み違える。
統合担当は要求側の備考にこの経路を書き足すこと。

9 件の ⚠ のうち**唯一 6.1 を名指しで引受先にしている項目**であり、初版の本節から落ちて
いた。⑵ とは**別の穴**である点に注意——⑵ は同じ経路の「不正バイト吸収が無記録で通る」
話で、こちらは「登記の記述が経路を 1 つ書き落としている」話。経路が同じだけで、直す
対象（コード／設計 vs 台帳の本文）も直し方も違う。

### ⑺ 採取フィクスチャは既定を 3 つ持つ（`tasks.md:205`・**引受先は本記録**）

`crates/areka-ghost/tests/ghost/snapshot_capture_test.rs`（実 backend へ Recorder を合成する
採取フィクスチャ）は、文字コードの既定を**独立に 3 つ**持っている。実ファイルで確かめた:

| # | 場所 | 値 | 意味 |
|---|---|---|---|
| 1 | `package::resolve(&root, DefaultEncoding::Utf8)`（:330） | **UTF-8** | ファイル層の既定（descript 等の読取） |
| 2 | `real_connect(helper_exe, mount.shiori, DefaultEncoding::Ansi)`（:339） | **Shift_JIS** | SHIORI 通信の既定。**本番と一致** |
| 3 | `GhostBootOptions { default_encoding: DefaultEncoding::Utf8 }`（:355） | **UTF-8** | 起動オプションの既定 |

**3 つ目が無害なのは、このフィクスチャが結線を差し替えているからにすぎない。**
フィクスチャは `ShioriWiring::Custom(Recorder(real_connect(..)))` を使い、2 の
`DefaultEncoding::Ansi` を自分で書いて渡している。一方**本番の結線**
（`crates/areka-ghost/src/runtime.rs:580` の `ShioriWiring::Helper` 腕）は
`real_connect(helper_exe, mount.shiori.clone(), options.default_encoding)` と書かれており、
**起動オプションの値をそのまま SHIORI の既定に使う**。

つまり、このフィクスチャを `Custom` から本番の `Helper` へ替えると、3 の UTF-8 が 2 の
Shift_JIS を置き換え、**SHIORI の既定が警告も出さずに UTF-8 になる**。ログは
`charset_initial charset=UTF-8 source=default` と正しく記録するので、記録を見ても
「間違っている」とは見えない——変わったこと自体は見えるが、意図した変更と区別が付かない。

境界の不変としてここに残す理由: 本仕様は「宣言が無ければ SHIORI の既定は Shift_JIS」を
正典としている。その正典が 1 行の結線の書き換えで静かに破れる場所がここに 1 つあり、
**検査は守っていない**（フィクスチャが `Custom` であること自体を見る検査は無い）。将来この
フィクスチャを本番結線へ寄せる人は、2 と 3 の値をそろえてから替えること。

### ⑻ 構造検査の限界（今日は零が成立しているが将来素通りする）

- 片道イベントの検査は「採用経路の関数名が現れない」ことを見る形なので、将来別名の採用
  入口が増えると素通りする。今日は採用状態を変える手続きが `note_response` 1 つだけなので
  零は成立している。
- `shiori_wiring.rs` の組立点の検査も綴り違い（`<CharsetNegotiator>::new`・別名 import・
  `Default`／`from` 系）は素通りし、正当な 2 つ目の組立点が増えると誤検知で赤になる。

### ⑼ 検証手順そのものの欠陥（7 節・引受先が要る）

既存の赤 1 件が `cargo test --workspace` の fail-fast でその後ろの 100 ターゲットを隠し、
本仕様の後退が最後の確認まで見えなかった。判定基準と手当ては 7 節に書いた。**これは
本仕様だけの話ではなくこのリポジトリ全体の検証手順の問題なので、完了ゲートで引受先を
決めること**（少なくとも、ワークスペース走行を `--no-fail-fast` 前提に改めることと、
横断的な番人の住むクレートをタスク検証の対象に含めること）。

### ⑽ 解消済み（記録のため）

- 実装ノート 2.1 が心配した `shiori3.rs` の裸の課題番号は解消されている。「タスク 2.2」の
  区画見出しは無くなり、`要件 1.6` は `完了仕様 areka-P0-host32-request 要件 1.6` と
  spec 名で修飾されている。
- 本仕様が触った `.rs` に `TODO` / `FIXME` / `TBD` は **0 件**。

---

## 11. 実行したコマンドの一覧（再現用・逐語）

```
git log --oneline -1 1712561b
git diff --name-only 1712561b..HEAD

cargo fmt --check                                   # EXIT=0
cargo test -p shiori-host32-host --no-run           # EXIT=0
cargo test -p areka-ghost --no-run                  # EXIT=0
cargo check --workspace --all-targets               # EXIT=0
touch crates/areka-ghost/src/shiori_wiring.rs crates/shiori-host32-host/src/charset.rs
cargo check -p areka-ghost -p shiori-host32-host --all-targets   # EXIT=0 / dead_code 0 件
cargo test --workspace                              # EXIT=101・結果行 3 本で打ち切り
cargo test --workspace --no-fail-fast               # EXIT=101・103 ターゲット 7,224 緑 4 赤（是正前）

# --- 6 節の是正を作業ツリーに載せたあと、結果が動きうる確認を全部取り直した ---
cargo fmt --check                                   # EXIT=0
cargo check --workspace --all-targets               # EXIT=0
cargo test -p log-capture-kit --test temp_path_guard_test   # 16 passed / 0 failed
cargo test -p log-capture-kit                       # EXIT=0・全 7 ターゲット緑（単体走行は lib 30 / doc 4。3.3 項の註）
cargo test -p areka --bins                          # 1573 passed / 2 ignored
cargo test --workspace --no-fail-fast               # EXIT=101・103 ターゲット 7,227 緑 1 赤（確定値）
git diff --name-only 1712561b                       # 38 ファイル（作業ツリー込み）
git diff --name-only 1712561b -- crates/log-capture-kit/tests/temp_path_guard_test.rs  # 0

git ls-files -- crates/shiori-host32-helper
git diff --stat 1712561b..HEAD -- crates/shiori-host32-helper
git ls-files -- crates/shiori-host32-ipc
git diff --name-only 1712561b..HEAD -- crates/shiori-host32-ipc
git ls-files -- crates/areka-emo-compose/src/world.rs crates/areka-seriko/src/resolve.rs
git diff --name-only 1712561b..HEAD -- crates/areka-emo-compose/src/world.rs crates/areka-seriko/src/resolve.rs
git diff --name-only 1712561b..HEAD -- .kiro/specs/completed
git ls-files -- crates/log-capture-kit/tests/file_length_guard_test.rs
git diff --name-only 1712561b..HEAD -- crates/log-capture-kit/tests/file_length_guard_test.rs
git ls-files --error-unmatch crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/descript.txt
git diff --name-only 1712561b..HEAD -- crates/pilot/examples/shiori-host-32/fixtures/emo2
git diff --name-only 1712561b..HEAD -- '*fixtures/*'
git diff --name-only 1712561b..HEAD -- '*testdata/*'

git diff -U0 1712561b..HEAD -- crates/shiori-host32-helper/src/shiori_proxy.rs \
  | grep -E '^[+-]' | grep -v '^[+-][+-][+-]' | grep -cvE '^[+-]\s*(//!|///|//)'   # 0

git diff -U0 1712561b..HEAD -- '*.rs' | grep -E '^[+-]' | grep -v '^[+-][+-][+-]' \
  | grep -E '(trace!|debug!|info!|warn!|error!|event!|warn_then_debug!)'           # 11 行

git show 1712561b:crates/areka/src/emo2_boot/assets_tests.rs | grep -n temp_dir     # 0 件
git log --oneline -S "env::temp_dir" -- crates/areka/src/emo2_boot/assets_tests.rs  # 899a0e33
```
