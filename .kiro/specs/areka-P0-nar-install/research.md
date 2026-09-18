# ギャップ分析: areka-P0-nar-install

> 2026-09-18 実施（要件生成と同日・同ブランチ `claude/areka-p0-nar-install-3882a2`）。対象は確定済みの `requirements.md`（10 要件）と現行コード。本文の数値は全て本日この作業ツリーで数え直した実測値で、数え方は末尾「付録 A 再現コマンド」に置く。外部依存の調査は docs.rs / rustsec.org / ukadoc を当たった（節 2.4・2.8）。
>
> 本書は「情報と選択肢」を出すもので、最終決定はしない。設計判断が要る項目は節 5 に番号付きで並べ、要件ディスカッションへ渡す。

## 1. 分析の概要

- **既存の土台は「文字コード」「`key,value` 読み」「一時ファイル→`rename` の確定」の 3 つだけで、アーカイブ読取・`install.txt` 解釈・展開・開発用の根の管理は 0 から作る。** `areka-parsers` の `charset`（`prescan.rs:58` がキー `charset` を大小無視で探し、`model.rs:37` が ANSI→Shift_JIS 固定）と `kv/parse.rs`（最初のカンマ・trim・後勝ち・**キーの大小は保持**）はそのまま使える。`areka-sylphya/src/persist/io.rs:77-103` の temp→fsync→rename はファイル 1 個の確定であり、フォルダ単位の原子的確定の前例は無い。
- **参照 38 ファイルは再現できた**（`shiori-host-32/fixtures` を実行行で綴る `.rs`＝src 21・`tests/` 8・`examples/` 9。`fn emo2_root` 12 定義・`const FIXTURE_DIR` 1・`emo2-kakukaku` を綴る `.rs` 45）。ただし「コメントだけの 11 ファイル」は別の検索語（`fixtures/emo2`）でしか 11 にならず、38 と同じ語では 8 である。`tools/perf` 6 本・`doc/` 3 本の数は再現できない（実測は 3 本・2 本。節 6）。
- **`zip` クレート（8.6.0）の公開 API では「名前は UTF-8」の印（汎用目的ビット 11）を読めない。** `ZipFileData.is_utf8` はクレート内私有で、`name()` は印が無いと CP437 で復号する。要件 2.2/2.3 を「印で決める」形で満たすには、印の回復方法か自前の中央ディレクトリ読みが要る（節 5 項目 8）。`ZipArchive::extract()` はシンボリックリンクを**作る**実装であり、RUSTSEC-2025-0168（1.3.0〜2.2.x・HIGH）が住んでいた API＝呼ばない方針は正しい。
- **本番の依存グラフに `flate2`・`miniz_oxide`・`crc32fast`・`typed-path` は 1 つも無い**（`flate2` は wintf の dev 依存 `image→png` 経由のみ）。`zip` を足すとこれらが本番側に初めて入り、`THIRD-PARTY-NOTICES.md`（`ignore-dev-dependencies = true`）に新しい謝辞が複数増える。brief の「新規は `typed-path` 1 本だけ」は Cargo.lock 全体の話で、配布物の謝辞の話ではない。
- **台帳と番人は「触ると赤くなる」形で待ち構えている。** `assets.toml` の `descript_install` 15 項目に `owner` を書くと、`roadmap-draft.md` に `[[spec]]` 行（`owner_count = 11`・`bundle`・`[briefs].count` 27→28）が無い限り `cargo test -p ukadoc-survey` の判定 ⑸ 腕 c/f が赤になる。番人 3 本の走査は `crates/` 配下だけで `target`・`vendors` を除く（`workspace_scan/mod.rs:41`）ので、展開物や `.nar` は拾われない。要件 1.8 の新しい常設検査はこの走査部品の隣（`log-capture-kit/tests/`）に置くのが構造どおり。

## 2. 現状調査（資産地図）

### 2.1 検体の保管と参照

| 対象 | 実測 | 根拠 |
|---|---|---|
| `crates/pilot/examples/shiori-host-32/fixtures/` 追跡ファイル | **150**（`emo2/` 110＝本体 90＋`emo2-kakukaku/` 20・`emo2-kakukaku-offsetdpi/` 20・`emo2-kakukaku-wplimit/` 20） | `git ls-files` |
| `vendors/sample_ghost/R_POST_and_KOMAINU/` | **43**（`ghost/master/dic09_Test.txt` を含む・追跡済み） | `git ls-files` |
| `vendors/sample_ghost/StayseeBalloon/` | **存在しない**（`default-balloon-bundle` 未着手） | `ls` |
| リポジトリ内の `.nar` | **0**（サブモジュール `vendors/pasta/crates/pasta_sample_ghost/hello-pasta.nar` のみ・対象外） | `git ls-files --recurse-submodules` |
| 検体パスを実行行で綴る `.rs` | **38**（src 21・`tests/` 8・`examples/` 9） | 付録 A-1 |
| コメント行だけで綴る `.rs` | 検索語 `shiori-host-32/fixtures` なら **8**・`fixtures/emo2` なら **11**・`shiori-host-32` なら 11・`fixtures` なら 12 | 付録 A-1 |
| `fn emo2_root()` の定義 | **12**（`spine.rs:491`・`mod.rs:572`・`assets_tests.rs:25`・`frame_attach_tests.rs:226`・`frame_visibility_integration_tests.rs:39`・`placement_shared_test_support.rs:23`・`source_tests.rs:15`・`emo2_real_run.rs:69`・`smoke_boot_loop_exit.rs:40`・`collision-probe/fixture.rs:45`・`window-placement.rs:128`・`areka-parsers/src/package/validation_tests.rs:25`） | 付録 A-2 |
| `const FIXTURE_DIR` | **1**（`areka-parsers/src/balloon/validation_tests.rs:36`） | 同上 |
| `emo2-kakukaku` を綴る `.rs` | **45**（うち実行行で綴るのは 27） | 同上 |
| 本番コードからの参照 | **0**。src 側の 5 本（`areka-parsers/src/package/resolve.rs:604,936`・`areka-seriko/src/resolve.rs:221`・`areka-emo-compose/src/world.rs:228`・`areka-emo-atlas/src/decode/wic_arm.rs:178`・`areka/src/emo2_boot/mod.rs:572`）はいずれも `#[cfg(test)] mod` の内側（それぞれ `:398/:649`・`:122`・`:172`・`:156`・`:35` の宣言の下） | `grep -n cfg(test)` |
| `.rs` 以外 | `tools/perf/` **3 本**（`invoke-followup-checks.ps1:129`＝バックスラッシュ綴り・`judge-perf.py`・`perf-loop.measure.ps1`）・`doc/` **2 本**（`emo2-conformance-scope.md`・`ukadoc-coverage/briefing-assets.md`）・example README **1 本**（`README.md:26`）。`doc/COMPAT_ARCHITECTURE.md` は **0 件** | 付録 A-3 |

綴りは 2 種で、一括 `join("../pilot/examples/shiori-host-32/fixtures/emo2")` と分割 `join("..").join("pilot")...join("shiori-host-32")`（`package/resolve.rs:604`）がある。実機走行の 2 本（`emo2_real_run.rs:142-155`・`smoke_boot_loop_exit.rs:51`）は `areka.exe <ghost_root> <balloon_root>` の 2 引数と `AREKA_APP_SMOKE_EXIT_MS` で起動し、バルーンは `ghost_root.join("emo2-kakukaku")` で指している（`boot_config.rs:46-60` が `args[1]`＝ghost root・`args[2]`＝balloon root）。

### 2.2 汚染と初期化の現況

- 永続化の書き手: `areka-ghost/src/runtime.rs:523-524` が `ghost: Some(profile_areka_root(&mount.shiori.dir))` と `shell: Some(profile_areka_root(&mount.shell.dir))` の両方を設定する（`sylphya_wiring.rs:88`）。
- その場しのぎの初期化: `crates/areka/src/emo2_boot/spine.rs:491-501` の `emo2_root()` だけが `remove_dir_all(<root>/ghost/master/profile/areka)` を行う。他の 11 定義は行わない。
- 無視規則: `crates/pilot/examples/shiori-host-32/.gitignore` は `fixtures/emo2/ghost/master/profile/` の 1 行のみ。`git check-ignore` の実測: ゴースト側 `…/ghost/master/profile/areka/sylphya.toml` は無視される、シェル側 `…/shell/master/profile/areka/x.toml` は**無視されない**（要件 1 節「潜在」の裏付け）。本日時点で `fixtures/emo2/ghost/master/profile/` が実在する（追跡外）。
- バイト保存: `vendors/sample_ghost/.gitattributes`（`* -text`）と `.gitignore`（`!*_test.txt` `!*_dump.txt`）が在り、`git check-attr text` は `R_POST…/install.txt: unset`、`fixtures/emo2/install.txt: unspecified`。`core.autocrlf=true` はシステム設定（`C:/Program Files/Git/etc/gitconfig`）由来。`dic09_Test.txt` は `git check-ignore` で無視されない（打ち消しが効いている）。

### 2.3 `install.txt` の解釈に使える既存層

| 層 | 場所 | 使える点 | 足りない点 |
|---|---|---|---|
| 文字コード決定 | `areka-parsers/src/charset/prescan.rs:29,58`・`decode.rs:24`・`model.rs:18-37` | BOM 除去→冒頭 ASCII 走査→`charset` 行（キー trim・**大小無視**）→`encoding_rs` 復号。宣言無しは `DefaultEncoding::Ansi`＝`SHIFT_JIS`。本番の起動設定も `boot_config.rs:130` で `Ansi` を渡す | 無し（`install.txt` は同じ行形式） |
| `key,value` | `areka-parsers/src/kv/parse.rs:20-46` | `split_once(',')`・trim・後勝ち・カンマ無し行スキップ | **キーの大小をそのまま保持**（`map.insert(key.to_owned(), …)`）。emo2 の `Charset,UTF-8` は復号には効く（prescan が大小無視）が、`parse_kv` の結果では `Charset` キーになる。`type`/`directory` 等を大小無視で引くかは決めていない（節 5 項目 3） |
| パッケージ解決 | `areka-parsers/src/package/resolve.rs:1-8` | 展開済みの根から `ghost/master/descript.txt` を起点に解決 | 前置きで「`install.txt` / balloon 系 / NAR には触れない」と宣言。`package/validation_tests.rs:113,123` が `install.txt`・`emo2-kakukaku/`・`delete.txt` が結果に影響しないことを固定 |
| 依存方針 | `areka-parsers/Cargo.toml`（`encoding_rs`・`tracing` のみ・「外部パーサ非依存」`structure.md:280`） | — | アーカイブ読取の依存をこのクレートへ入れる余地は無い |

検体 5 つの `install.txt` の実測（`cat -A`・全て CRLF）:

| 検体 | 行 |
|---|---|
| emo2 | `Charset,UTF-8`・`type,ghost`・`name,えもツー`（UTF-8）・`directory,emo2`・`balloon.directory,emo2-kakukaku`・`balloon.source.directory,emo2-kakukaku` |
| emo2 同梱 `emo2-kakukaku/install.txt` | `charset,UTF-8`・`type,balloon`・`name,kakukaku for emo2`・`directory,emo2-kakukaku`（要件 5.3 の「あっても無視」の実例） |
| offsetdpi / wplimit | `charset,UTF-8`・`type,balloon`・`name,…`・`directory,emo2-kakukaku-…` |
| R_POST_and_KOMAINU | `charset, Shift_JIS`（値の前に空白・trim で吸収）・`type,ghost`・`name,`（Shift_JIS）・`directory,R_POST_and_KOMAINU` |

最上位のその他のファイル: emo2 は `delete.txt`・`updates.txt`・`readme.txt`、R_POST は `developer_options.txt`・`readme.txt`・`thumbnail.png`。要件 5.2 の「その他のファイル」としてそのまま置かれる。どの検体にも `refresh`・`accept` 行は無い。

### 2.4 アーカイブ読取の既存資産と依存グラフ

- **ワークスペースに `zip` は無い**（全 `Cargo.toml` を検索・0 件）。`flate2 1.1.10` は `image 0.25.10 → png 0.18.1 → flate2` の **dev 依存**経由のみ（`cargo tree -e all -i flate2`）で、`-e normal` では 0。
- 本番グラフ（`cargo tree -e normal`）に在る／無い: `indexmap` 在・`memchr` 在／`flate2`・`miniz_oxide`・`crc32fast`・`typed-path`・`adler2`・`simd-adler32` **無**。ハッシュ系（`md-5`・`sha2`・`sha1`・`blake3`・`crc`）も **無**。
- `deny.toml`: 許可リスト方式（MIT・Apache-2.0・…・BSD-3-Clause・Unicode-3.0）・`all-features = true`・git ソース禁止。`about.toml`: `ignore-dev-dependencies = true`・同じ許可リスト。道具は導入済み（`cargo-deny 0.20.2`・`cargo-about 0.9.2`）。`THIRD-PARTY-NOTICES.md` は自動生成（103,855 バイト・MIT 209 crate ほか）。
- 依存の書き場所の前例: `crates/ukadoc-survey/Cargo.toml` が `serde_json = "1"` を**クレート側**に直書きし「ルートの `Cargo.toml` を触らないため版はクレート側に書く（前例 `crates/dola/Cargo.toml`）」と注記。`zip` も同じ形が取れる。
- `tech.md` の登記書式: `encoding_rs (0.8): …（意図的依存追加＝2026-07-02 承認済）`。`roadmap.md:36,132` は `zip` を「承認待ち」と明記。

**`zip` クレートの公開 API（docs.rs `zip 8.6.0`・2026-07-10 公開）**

| 事実 | 出典 | 本仕様への意味 |
|---|---|---|
| 既定機能は `aes-crypto`・`bzip2`・`deflate`・`deflate64`・`lzma`・`ppmd`・`time`・`xz`・`zstd`。`deflate-flate2` は「flate2 のバックエンド機能と組み合わせて品質 1..=9 の deflate」、`deflate-zopfli` は書き込み側（品質 10 以上） | docs.rs トップ | `default-features = false, features = ["deflate-flate2"]` の brief の選択は妥当。`flate2` 側のバックエンド機能（`rust_backend`）を明示する必要があるかは設計で確認（Research Needed） |
| `ZipFile::name_raw(&self) -> &[u8]`「エンコードは未定義」 | `struct.ZipFile.html` | 生バイトは取れる |
| `name()` は `is_utf8` が偽なら `from_cp437()`、真なら `from_utf8_lossy` | `read.rs` ソース | Shift_JIS 名は `name()` で必ず化ける＝自前復号が必須 |
| **`is_utf8` を読む公開メソッドは無い**（`ZipFileData` のクレート内私有フィールド） | `read.rs` ソース | 要件 2.2/2.3 を「印で決める」ためには印の回復か自前読みが要る（節 5 項目 8） |
| `is_symlink()`・`is_dir()`・`unix_mode()`・`compression()`・`encrypted()`・`size()`・`crc32()`・`enclosed_name()`・`mangled_name()` | `struct.ZipFile.html` | 拒否判定の材料は揃う。`enclosed_name()` は `name()`（CP437 経路）由来なので使わない |
| `ZipArchive::by_index`・`by_index_raw`（展開せず取得）・`by_index_seek`・`len`・`file_names`・`decompressed_size` | `struct.ZipArchive.html` | 番号で回す経路はある |
| `extract()`・`extract_unwrapped_root_dir()` は `safe_prepare_path` を通し、`is_symlink()` なら `make_symlink` で**リンクを作る** | `read.rs` ソース | 要件 2.7（リンクを決して作らない）と衝突＝呼ばない |
| RUSTSEC-2025-0168: 影響 1.3.0〜2.2.x・修正 2.3.0・API は `ZipArchive::extract` と `ZipStreamReader::extract`・CVSS 7.3・リンクの正規化パス検証漏れによる任意書き込み | rustsec.org | 8.6.0 は修正後だが、経路ごと避ける方針は据え置きでよい |

### 2.5 原子的確定・一時パス・`target/` の見つけ方の前例

- **ファイル単位の原子的確定**: `areka-sylphya/src/persist/io.rs:11-13,50-61,77-103`（同一ディレクトリの `.tmp` へ全書込→fsync→`rename`・Windows でも既存宛先を置換）。**フォルダ単位**（一時フォルダへ展開→フォルダごと `rename`）の前例は無い。Windows の `rename` は宛先フォルダが既に在ると失敗するので「空の宛先へ移す」か「旧を退避→新を置く→旧を消す」の 2 段が要る（設計事項）。
- **一時パスの窓口 `temp-path-kit`** は `std::env::temp_dir()` の下に `areka-{札}-{pid}-{連番}` を組む（`lib.rs:91-93`）。要件 7.10（OS の一時フォルダを使わない）により**この窓口は本仕様の展開先には使えない**。番人 `temp_path_guard_test.rs:72,81` は `env::temp_dir(`／`temp_dir(` の綴りを実行行で探すので、`target/` 配下だけを使う限り触れない。
- **`target/` の見つけ方の前例**: `CARGO_MANIFEST_DIR` 相対（38 ファイル全部がこの形）／`std::env::current_exe()` の親（`boot_config.rs:78,98`・`runtime_tests.rs:552`＝テストバイナリは `target/<profile>/deps/` に居る）／`frame_attach_tests.rs:743` は走査で `target` という名前のフォルダを飛ばすだけ。`CARGO_TARGET_DIR` を読む箇所は 0。どれを採るかは節 5 項目 9。
- **ハッシュ**: 本番グラフにハッシュ crate は無い。`ukadoc-survey/src/hash.rs:1-12` は FNV-1a 64 を自前で持ち、「標準ライブラリの既定ハッシュは版をまたいだ同値保証が無い」と理由を書く。`zip` を入れると `crc32fast` が同時に入るので、`.nar` 全体の CRC32 か、中央ディレクトリの各エントリ `crc32()` の列が無料で使える。

### 2.6 常設の検査と台帳（触ると赤くなるもの）

| 検査 | 場所 | 本仕様との関係 |
|---|---|---|
| 1 ファイル 1,000 行の番人 | `log-capture-kit/tests/file_length_guard_test.rs`（例外表 `OVER_LIMIT_ALLOWED` 10 件・`:61,105`） | 新設ファイルは 1,000 行未満（要件 10.8）。例外表は触らない |
| 一時パスの窓口の迂回検知 | 同 `temp_path_guard_test.rs:72,81` | `temp_dir(` を書かなければ無関係 |
| 共有機構の迂回検知 | 同 `with_default_guard_test.rs:9,282-289` | `log-capture-kit` が `[dependencies]` に現れたら赤（要件 10.3） |
| 走査部品 | 同 `workspace_scan/mod.rs:41,82`（`crates/` 起点・`target`・`vendors`・`.git` を除外） | 要件 1.8 の新しい常設検査の置き場候補。`vendors/sample_ghost/*.nar` は走査外 |
| 台帳の整合（判定 ⑸） | `ukadoc-survey/tests/consistency/spec_checks.rs:1-40,177-193,279-305` | 腕 a: `[briefs].count`＝`[[spec]]` 行数（今 27・`roadmap-draft.md:89-91`）／腕 c: `owner_count`＝台帳の数え直し／腕 d: `bundle` が `linkage.md` に在る（「インストール」は `linkage.md:106,131` に実在）／腕 f: 台帳の非空 `owner` は `[[spec]]` か `[[owner_completed]]` の名前。**`descript_install` に `owner` を 11 件書くなら `[[spec]]` 行の追加が同時に必須** |
| 台帳の現況 | `doc/ukadoc-coverage/ledger/assets.toml:3842-4019` | `descript_install` **15 項目**（`install.txt` のキー 11・`bootghost`・「相対パス」系 3）が全て `status = "absent"`・`owner = ""`・`priority = "B1"`。`manual_install`・`dev_nar`・`manual_directory` の 3 ページは `pages` に在るが項目は 0 |
| `roadmap-draft.md` の散文 | `:67`「置き場にあるが表に無いもの 2 本: `areka-P0-nar-install`…」・`:83`「意図しない重なり 1 行: 段階 B『インストール』の `areka-P0-nar-install`」・`:351` 段階 B 順位 1 の行 | `[[spec]]` 行を足すときにこの 3 か所の記述も同時に直さないと文書内で矛盾する |

### 2.7 隣接仕様の brief が本仕様に置いている期待

| 仕様 | 記述 | 整合 |
|---|---|---|
| `ghost-install`（`brief.md:16`） | `areka-nar` が `install.txt` の `refresh`／`refreshundeletemask`／`*.directory`／`*.source.directory` を**読み**、パス安全性・原子的確定を持つ。`:39` `install(root, path) -> InstallOutcome` は「展開＋`accept` 判定＋結果の型」 | 「読む」とは書くが、`refresh` の**実行**（既存フォルダを消す）をどちらが持つかは書いていない |
| `network-update`（`brief.md:71`） | 「確定と削除（一時フォルダ → `rename`・`delete.txt`）」を自分で持つ | `refresh` はネットワーク更新の経路には出てこない（ファイル差分更新のため） |
| `default-balloon-bundle`（`brief.md:109-110`） | 保管は展開フォルダ `vendors/sample_ghost/StayseeBalloon/`、`.nar` 化は本仕様が引き受け。検体パスの参照は「新規テストの定数 1 か所」に留める | 本仕様の段 ③ がその展開フォルダを消すと、並走側の定数が宙に浮く（節 5 項目 7） |
| `shell-implicit-surface`（`brief.md:150`） | 検体は本仕様の窓口経由で受ける | 窓口の関数名・戻り値の形が先に決まる必要がある |
| `baseware-root-layout`（`brief.md:22,57`） | 展開先の形＝`<根>/ghost/<directory>/`・`<根>/balloon/<balloon.directory>/`・窓口の根を `BasewareRoot` として受ける | 要件 1.2・5 と一致 |
| `popup-menu-minimal`（`brief.md:108`） | 接触面に検体参照 0 | 共有 0 |

### 2.8 実在する `.nar` の観測（`hello-pasta.nar`・pasta 側資産・参考のみ）

Python の `zipfile` で読んだ: 54 エントリ・**全エントリでビット 11 は未設定**（名前は全て ASCII）・圧縮方式は deflate（8）のみ・フォルダのエントリ 0・最上位に `install.txt`・`updates.txt`・`updates2.dau`・`ghost/`・`shell/`。`install.txt` は `type,ghost`／`name,hello-pasta`／`directory,hello-pasta`／**`accept,`（値が空）**・`charset` 行無し。示唆: ⑴ 印が無い＝Shift_JIS 既定でも ASCII 名は無害、⑵ `accept` の値が空の行は「無い」と同義に扱う必要がある（要件 3.11 の戻り値の形に影響）、⑶ フォルダのエントリが無くてもフォルダを作る経路が要る（要件 4.9 は「あれば作る」しか言っていない）。また `pasta_sample_ghost` には `build.rs` が**在る**（`vendors/pasta/crates/pasta_sample_ghost/build.rs`・再実行トリガーと `ghosts/hello-pasta/` 不在の警告だけで生成はしない）。brief の「build script を意図的に拒否」は「生成を build.rs でやらない」の意味であり、`structure.md:183` の「`crates/*` に `build.rs` 0 件（`vendors/` は対象外）」とは矛盾しない。

## 3. 要件 → 資産の対応表

凡例: **無** = 作る／**制約** = 既存の形に合わせる／**要調査** = 設計で確定。

| 要件 | 既存資産 | 差分 | 印 |
|---|---|---|---|
| 1.1–1.5 窓口（名前→フォルダ／根／同梱バルーン・失敗理由・2 手で追加） | 12 の私家版 `emo2_root()`・`FIXTURE_DIR`・直書き 25 | 共有の窓口は 0。置き場（クレート・依存種別）が未決 | **無**・要調査（項目 10） |
| 1.6–1.7 38 ファイルの収束・本番 0 | 38 ファイル特定済み（2.1）・本番 0 を確認 | 機械的書き換え。`emo2-kakukaku` を実行行で綴る 27 ファイルも同時に窓口へ | 制約 |
| 1.8 常設検査 | `workspace_scan`（`crates/` 起点・コメント除去あり） | 新しい検査 1 本＋較正 | **無**（置き場は制約） |
| 1.9 1 コマンドで絶対パス | `cargo run --example`／`cargo run -p` の慣行 | 出力形式・どのクレートの bin/example にするか | **無** |
| 2.1 `.nar`/`.zip` 同一手順 | 無し | `zip` か自前読み | **無**（項目 8・節 4） |
| 2.2–2.3 ビット 11 で UTF-8／Shift_JIS | `encoding_rs`（既存）・`name_raw()` | **印を読む公開 API が無い** | 要調査（項目 8） |
| 2.4 復号不能は拒否＋生バイト 16 進 | `encoding_rs` の `decode_without_bom_handling` は置換文字で通す | 「損失なく」の判定は自前（`had_errors` を見る） | **無** |
| 2.5–2.7 暗号化・対応外方式・破損・リンクの拒否 | `encrypted()`・`compression()`・`is_symlink()`・`crc32()` | 判定と語彙 | **無** |
| 3.1–3.2 最上位 `install.txt`・包み 1 段は拒否 | 無し | 最上位の定義（`/` を含まない名前）と理由一覧 | **無**（項目 4） |
| 3.3 文字コード（キー大小無視・ANSI 既定） | `charset::decode`（`prescan.rs:58` が大小無視・`Ansi→SHIFT_JIS`） | そのまま使える | 制約 |
| 3.4 `key,value` 規則 | `kv::parse_kv` | キー大小は保持される | 要調査（項目 3） |
| 3.5–3.16 `type`・必須キー・`*.directory`・警告・無視キー | 無し | 解釈器 1 本 | **無** |
| 4.1–4.9 パス安全性 | 無し（`enclosed_name()` は CP437 経路で不採用） | 自前の名前検証（Windows 予約名・大小衝突・`\`・NUL・`..`・絶対・UNC） | **無** |
| 5.1–5.11 配布形→インストール済み形・原子性 | `FsPersistIo::commit`（ファイル単位の手本） | フォルダ単位の確定・失敗時の無傷保証 | **無** |
| 6.1–6.5 `refresh` の実行 | 無し。正典: `refresh,1` で同じディレクトリを全消去してから展開・1 以外は無効／`refreshundeletemask` はコロン区切り・パス不可・**全てのディレクトリの同名ファイルを除外**（ukadoc 実引用で確認） | 実行の所有が未決 | 要調査（項目 1） |
| 7.1 `target/` 配下の専用名前空間 | `current_exe()`・`CARGO_MANIFEST_DIR` の前例 | `target/` の見つけ方 | 要調査（項目 9） |
| 7.2–7.3 無ければ展開・変わったら展開し直す | 無し | 刻印（ハッシュ）の方式・鮮度判定 | **無**（項目 2） |
| 7.4–7.6 取得のたびに新品・並走・多重プロセス | 無し | 「利用者ごとの複製」か「共有木を差し替えない」か | 要調査（項目 2） |
| 7.7–7.9 自己修復・在り続けない・使用量上限 | 無し | 回収方針 | **無** |
| 7.10 OS 一時フォルダを使わない | `temp-path-kit` は `temp_dir()` 直下＝使えない | 自前で `target/` に組む | 制約 |
| 7.11 追跡外ファイル 0 | シェル側 `profile/` は無視されない（実測） | 展開物が `target/` に在れば構造的に解決 | 制約 |
| 8.1–8.9 `.nar` 化・バイト保存・`spine.rs`・`.gitignore` | `vendors/sample_ghost/.gitattributes`・`.gitignore` | `.nar` 5（4）本の作成手順・突合の記録 | **無**（項目 5・6） |
| 9.1–9.7 失敗の可視性・決定論テスト・実機一周 | `emo2_real_run.rs`・`smoke_boot_loop_exit.rs` の起動形 | 固定 `.nar` の作り方（テスト内で組む vs 追跡） | 要調査 |
| 10.1–10.4 依存登記・機能絞り・dev 依存の規律 | `tech.md` 書式・`deny.toml`・`about.toml`・`with_default_guard_test.rs` | 謝辞に増える crate 数の見積り | 制約 |
| 10.5 スクリプト・文書の追随 | 実測 3＋2＋1 本 | 要件の 6＋3＋1 と食い違う（節 6） | 制約 |
| 10.6–10.7 台帳 `owner`・`roadmap-draft.md` | 腕 a/c/d/f・散文 3 か所 | 同時編集の一式 | 制約（項目 11） |
| 10.9 `roadmap.md` の定石 1 行 | `roadmap.md:180`「実機運転の定石」 | 追記先は在る | 制約 |
| 10.10 畳む手順の README | `vendors/sample_ghost/` に README は**無い** | 新規 | **無** |

## 4. 実装方針の選択肢

### 4.1 コンテナ読取（要件 2）

| 案 | 内容 | 利点 | 難点 |
|---|---|---|---|
| **A. `zip` 8.6 を使い、名前の印は「比較で回復」** | `by_index` で回し、`name_raw()` を取る。印の有無は `name()` と `from_utf8(name_raw)` の一致で判定する（印あり→`name()` は UTF-8 復号と一致／印なし→CP437 復号は 1 バイト 1 文字なので非 ASCII を含む限り UTF-8 復号と一致しない／全 ASCII はどちらでも同じ） | 依存 1 本・展開器の CRC 検査と多方式の拒否判定を借りられる | 印の回復が「クレートの内部挙動（CP437 と lossy の使い分け）への依存」になる。zip 側の変更で黙って壊れないよう較正テストが要る |
| **B. 自前の中央ディレクトリ読み＋伸長だけ外部（`miniz_oxide` または `flate2`）** | EOCD 探索→中央ディレクトリ→汎用目的フラグ・方式・CRC・名前の生バイトを自分で読む（zip64 非対応で可・ゴーストは 4 GB 未満）。deflate（8）と無圧縮（0）だけ受理 | 印を**直接**読める。`extract()` も CP437 経路も `typed-path` も持ち込まない。本番グラフに増えるのは伸長 crate 1〜2 本 | 読み手 200〜300 行を自前で持つ。データ記述子・zip64・暗号化ビット等の「拒否すべき形」を自分で網羅する |
| **C. `zip` を使い、印は `name_raw` の UTF-8 妥当性で近似** | 生バイトが妥当な UTF-8 なら UTF-8、そうでなければ Shift_JIS | 最も短い | 要件 2.2/2.3（印で決める）を満たさない。印なし＋偶然 UTF-8 妥当な Shift_JIS 列を取り違える余地がある（稀だが零ではない） |

証拠が示す向き: 要件の文言（印で決める）を守るなら A か B。**依存を最小にし「拒否すべき形」を自分の語彙で持つ**という要件 9.2 の閉じた語彙とは B の相性がよい。A は展開器の成熟した検査を借りられるが、印の回復が間接的になる。C は要件を弱める。設計で A/B を比較する（Research Needed: `miniz_oxide` の `inflate` API と `flate2` のどちらを伸長に使うか・`zip` 8.6 の `deflate-flate2` が `flate2` のバックエンド機能の明示を要るか）。

### 4.2 クレートの置き場（要件 3〜6・10.3）

| 案 | 内容 | 判断 |
|---|---|---|
| A. `areka-parsers` を拡張 | `package/` に `install.txt` 解釈を足す | 解釈だけなら方針に合う（純パーサ・`encoding_rs` のみ）。**展開器は同居できない**（外部パーサ非依存・I/O は `resolve.rs` だけ） |
| **B. `areka-nar` を新設** | 読取・解釈・展開・`refresh` を 1 クレートに | brief・隣接 brief 3 本の前提。`areka-parsers::{charset,kv}` を使う（依存方向は下流→上流で問題なし） |
| C. 解釈は parsers・容器と展開は `areka-nar` | brief「Boundary Candidates」が示した割れ方 | 2 クレート跨ぎで `install.txt` の型が parsers 側に住む。`ghost-install` は `areka-nar` だけ呼べばよいので、外から見た利点は小さい |

証拠が示す向き: **B**。`install.txt` 解釈の型（`InstallManifest` 相当）を `areka-nar` に置き、文字コードと `key,value` は `areka-parsers` の既存関数を呼ぶ。

### 4.3 検体の窓口の置き場（要件 1・7）

| 案 | 内容 | 判断 |
|---|---|---|
| A. `areka-nar` 内の `dev` モジュール | 本番クレートに開発専用の関数を同居 | 要件 1.7（本番は窓口を知らない）を「呼ばない約束」でしか保証できない。`cargo` の feature 切りは `deny` の `all-features = true` で常に有効側が検査される |
| **B. 新設のテスト専用 leaf（例: `sample-ghost-kit`・`[dev-dependencies]` のみ）** | `temp-path-kit` と同じ配置（依存最小・`publish = false`）。`areka-nar` を `[dependencies]` に持ち、`.nar` の場所（`vendors/sample_ghost/`）と `target/` の名前空間と登記表を持つ | 本番グラフに窓口が入らないことを**依存の向きで**保証できる。`with_default_guard_test.rs` と同じ形の見張り（本番 `[dependencies]` に現れたら赤）が 1 本で書ける |
| C. 各クレートの `*_test_support.rs` に薄い転送 | 窓口本体は B、各クレートは 1 行の `use` | B の上に乗る形。38 ファイルの差分を「パスの得方の行」に限る（要件 1.6）のに役立つ |

証拠が示す向き: **B（＋C）**。`temp-path-kit`／`log-capture-kit` の前例（`structure.md:346-361`）がそのまま型になる。`examples/` から使う場合は `[dev-dependencies]` で届く（example は dev 依存を見る）。

### 4.4 全体の段取り

brief の 3 段（① 収束・② エンジン・③ 切替）は妥当。証拠から付け足す点: ① の時点で窓口の**戻り値の形**（ゴーストのフォルダ／根／同梱バルーンのフォルダの 3 種）を最終形で決めておかないと、③ で `emo2-kakukaku` の位置が `<emo2>/emo2-kakukaku` から `<根>/balloon/emo2-kakukaku` へ動くときに 27 ファイルをもう一度触ることになる（要件 1.3 の「二度書き換えない」の根拠）。

## 5. 設計判断項目（要件ディスカッションへ）

各項目は「影響する要件 / 何が問題か / 選択肢 / 証拠が示す向き」の順。

### 項目 1. `refresh`／`refreshundeletemask` の**実行**をどちらが持つか

- 影響: 要件 3.15・6.1〜6.5・9.3（再インストール 2 種の固定入力）。
- 問題: 本仕様の brief の In 列挙に「実行」は無く、`ghost-install` の brief（`:16`）は `areka-nar` が「読む」とだけ書く。`network-update` は自分の経路（ファイル差分・`delete.txt`）を持ち `refresh` を使わない。実行の主体を誰も明記していない。
- 選択肢:
  - (a) **本仕様が実行まで持つ**（要件 6 のまま）。`ghost-install` は呼ぶだけ。
  - (b) 本仕様は値を読んで返すまで（3.15）とし、既存フォルダの消去・上書きは `ghost-install` が実装。要件 6 は削除し、9.3 の再インストール 2 種も落とす。
  - (c) 本仕様は「宛先が既に在るときの振る舞い」を引数（`OnExisting::{Overlay, RefreshExcept(mask)}` 相当）で受ける純粋な展開器にし、`install.txt` の値から引数を組むのは呼び出し側。実行は本仕様、判断は呼び出し側。
- **2026-09-18 開発者裁定: (a)**（本仕様が実行まで持つ。設計で (c) の「引数で受ける」形に寄せるのは可）。あわせて展開の結果は「何をインストールしたか」の要素の列（種別・名前・置き場・宛先ゴースト・既存宛先の扱い）で返す＝要件 5.10 を改訂。
- 証拠が示す向き（裁定前の記述）: **(a) か (c)**。理由 ⑴ 展開の原子性（5.11）と「消してから展開」は同じ書き込みの経路にあり、別クレートに割ると「消した後に展開が失敗した」時の記録（6.4）を二重に持つことになる。⑵ 開発用の根は再インストールを通らない（6.5）ので、本仕様の実機走行では検証されず、決定論テストだけが根拠になる＝どちらに置いても検証の形は同じ。(c) は「値の解釈」と「実行」を分けるので、`ghost-install` が `accept` 照合の後で実行を決められる利点がある。

### 項目 2. 「取得のたびに新品」の実現形＝複製か共有か・ハッシュの持ち方

- 影響: 要件 7.2〜7.9・9.5。
- 問題: 7.4（毎回起動記録が無い）と 7.5（利用中の木を差し替えない）を同時に満たす形が 2 通りあり、`target/` の使用量（7.9）と鮮度判定（7.3）の方式が変わる。
- 選択肢:
  - (a) **利用者ごとの複製**: `target/<名前空間>/cache/<検体>-<刻印>/`（読み専用の原本・`.nar` のハッシュで鮮度判定）を 1 度だけ作り、取得のたびに `target/<名前空間>/runs/<検体>-<pid>-<連番>/` へ**コピー**して返す。返した木は呼び出し側の `Drop` で消す（`TempPath` と同じ型）。原本は差し替えても利用中の複製に影響しない。
  - (b) 共有の木＋起動前に `profile/` だけ削除: 展開は 1 本、取得時に `ghost/*/master/profile/`・`shell/*/profile/` を消す。複製は無いが、並走する 2 利用者の片方が消した瞬間にもう片方の起動記録が消える（7.5 違反の余地）。
  - (c) 毎回 `.nar` から展開し直す（複製ではなく再展開）: 6.6 MB の deflate 伸長を取得のたびに行う。原本の管理が要らない代わりに、38 ファイル分のテストが毎回伸長する。
  - 刻印: (i) `.nar` 全体の CRC32／FNV-1a 64（`ukadoc-survey/hash.rs` の前例・crate 不要）、(ii) `.nar` の長さ＋更新時刻（速いが `git checkout` で更新時刻が動く）、(iii) 中央ディレクトリの各エントリ `crc32()` の列（アーカイブを読む途中で無料）。
- 証拠が示す向き: **(a)＋(i) または (iii)**。7.5・7.6・7.7 を「原本は読み専用・複製は自分だけの物」で構造的に満たせる。使用量は `runs/` を `Drop` で消し、落ちたプロセスの残りは次回起動時に「自分の pid 以外で生存していない pid のフォルダ」を回収する（Windows で pid の生存は `OpenProcess` 相当が要る＝簡略に「起動時に `runs/` を全消し」でも 7.9 は満たせるが、同時実行中の別プロセスの木を消す恐れがあるので、複製先の名前に pid を入れ、**同じ pid の古い連番だけ**を消す形が安全）。`emo2` 110 ファイル 6.6 MB のコピーは SSD で 100 ms 程度。

### 項目 3. `install.txt` のキーの大小

- 影響: 要件 3.3・3.4・3.12。
- 問題: 既存 `kv::parse_kv` はキーの大小を保持する（`parse.rs:44`）。大小無視は `charset` の prescan だけ（`prescan.rs:58`）。emo2 は `Charset,UTF-8` と綴る。`type`／`directory` を大文字で書く検体は手元に無いが、他人の `.nar` が来る `ghost-install` は同じ解釈器を使う。正典 `descript_install` はキーの大小について沈黙している。
- 選択肢:
  - (a) **全キーを ASCII 大小無視で引く**（読み込み後にキーを小文字化した写しを作る。`kv` 層は触らない）。
  - (b) 正典の綴り（小文字）だけ受理。`Charset` は prescan が拾うので復号は正しく、`parse_kv` の結果に `Charset` が残るだけで害は無い。他のキーが大文字なら「必須キーが無い」で拒否。
  - (c) `charset` のみ大小無視（現状の層の挙動と同じ）、他は (b)。
- 証拠が示す向き: **(a)**。SSP が大小を区別する証拠は無く、区別しないほうが受理範囲が広い（拒否は利用者に見える失敗）。`kv` 層は `descript.txt` 等の他の読み手と共有なので触らず、`install.txt` 解釈器の側で小文字化する。決めたら要件 3.4 の文言（「既存の `key,value` 層と同じ規則」）に「キーは ASCII 大小無視で引く」を足す。

### 項目 4. 正典が沈黙している 4 つの決定を要件に固定するか

- 影響: 要件 5.3（同梱バルーンをゴースト側に残さない）・5.8（supplement の最上位 `install.txt` を重ねない）・3.2（包み 1 段を剥がさず拒否）・2.3（印なしは Shift_JIS）。
- 問題: いずれも要件本文が「本仕様の決定」と明示しているが、SSP の実挙動と違えば `ghost-install` 経由で利用者に見える。
- 選択肢と証拠:
  - 5.3 複製を残さない: (a) 残さない（現要件）／(b) 残す（SSP は残す可能性がある＝未確認）。証拠: ukadoc「全体の構成」はバルーンをバルーン格納フォルダに置くと定めるだけ。`package/validation_tests.rs:123` は「`emo2-kakukaku/` が結果に影響しない」を固定しており、残しても起動は壊れない。残さないほうが `git`・容量・列挙（`baseware-root-layout`）で二重に数えない。**(a) を推す**。
  - 5.8 `install.txt` を重ねない: (a) 重ねない（現要件）／(b) 重ねる。証拠: `hello-pasta.nar` の `install.txt` は `type,ghost` を持ち、重ねると素性が変わる。**(a) を推す**。ただし `readme.txt` 等の他の最上位ファイルは重ねる（要件どおり）。
  - 3.2 包みフォルダ: (a) 拒否（現要件）／(b) 1 段だけ剥がす（`zip` の `extract_unwrapped_root_dir` が同じ発想を持つ＝一定の需要はある）。証拠: 正典 `manual_install` は作る側に「一番上に `install.txt`」を求める。拒否の理由に最上位のエントリ一覧を含める（3.2）ので、利用者は原因が分かる。**(a) を推す**が、`ghost-install` の要件段階で「剥がす」を足すのは後からでも可能（拒否語彙に `wrapped-root` を 1 つ持っておけば互換のまま緩められる）。
  - 2.3 Shift_JIS 既定: (a) Shift_JIS（現要件）／(b) OS の ANSI コードページ。証拠: `charset` 未宣言時の既定を `Ansi→SHIFT_JIS` に固定した前例（`charset/model.rs:19-20`・「areka では CP932 へ固定写像」）と同じ判断。**(a) を推す**。

### 項目 5. 「38 ファイル」の数え方の固定と、常設検査の語彙

- 影響: 要件 1.6・1.8・Introduction の数値。
- 問題: 38 は `shiori-host-32/fixtures`（または分割 `join("shiori-host-32")`）を**実行行**で綴る `.rs` として再現した（付録 A-1）。一方「コメントだけの 11」は検索語 `fixtures/emo2` で数えた値で、同じ語だと 8。要件 1.8 の検査は「窓口の定義ファイルの外の `.rs` に現れない綴り」を列挙するが、段 ③ 後は旧置き場が消えるので `shiori-host-32/fixtures` を綴るコードは自然に壊れる（検査より先にコンパイルが落ちるわけではない＝文字列なので落ちない。検査は要る）。
- 選択肢:
  - (a) **検査語は 3 つ**: `shiori-host-32/fixtures`・`vendors/sample_ghost/<各検体名>/`（`.nar` 名は許す）・展開先の名前空間フォルダ名。実行行のみ。較正は「窓口ファイル以外に 1 件足すと赤」。
  - (b) 上に加えて `emo2-kakukaku` の直書きも禁じる（同梱バルーンは窓口 1.3 で取るため）。27 ファイルが対象。
  - (c) 検査を `.rs` に限らず `tools/`・`doc/` にも広げる（走査部品は `crates/` 起点なので別の列挙が要る）。
- 証拠が示す向き: **(a)＋(b)**。(b) は要件 1.3「段 ③ で二度書き換えない」を機械で守る唯一の手段。(c) は文書が `.nar` の置き場を名指しすることを禁じない（1.8）ので、文書側は「旧置き場の綴りが 0 件」を最終検証で 1 度数えれば足りる。要件本文の「11」は付録 A-1 の検索語を注記して残すか、8 に直す。

### 項目 6. brief と現物の食い違い 4 件の扱い

- 影響: 要件 8.5・10.1・10.7・Introduction。
- 事実:
  - `pasta_sample_ghost` に `build.rs` は**在る**（生成はしない）。`structure.md:183` の「`crates/*` は 0 件」は `vendors/` を除くので矛盾しない。→ 本仕様に影響なし。記述を「生成を build.rs でやらない前例」と読み替える。
  - `Cargo.lock` は `.gitignore:2` で無視され追跡されない。→ `THIRD-PARTY-NOTICES.md` の再生成は各自の解決に依存する（記憶: 謝辞が `Cargo.lock` より新しい解決を写した前例）。再生成は `cargo update` を伴わずに行い、生成物の差分に `zip` 系以外が混ざったら止まる、を完了条件に書くか。
  - `StayseeBalloon/` は無い。→ 要件 8.5 の「存在するとき」の条件分岐は正しい。段 ③ の時点で在れば畳む。
  - `roadmap-draft.md` に本仕様の行が無い（`:67,:83` が明記）。→ 項目 11。
- 選択肢: (a) 要件は変えず設計・検証報告で吸収／(b) Introduction に 2 行（`build.rs`・`Cargo.lock`）を追記。証拠が示す向き: **(a)**（要件の内容は変わらない）。

### 項目 7. 段 ① と段 ③ の間の「同梱バルーンの位置」と、並走仕様との窓

- 影響: 要件 1.3・1.6・8.5・Adjacent（`default-balloon-bundle`）。
- 問題: 段 ① で窓口は `<emo2>/emo2-kakukaku`（現物）を返し、段 ③ で `<根>/balloon/emo2-kakukaku` を返す。窓口の**関数**が同じなら呼び出し側は変わらないが、`emo2_real_run.rs:143` のように「ゴーストの根に `emo2-kakukaku` を継ぎ足す」呼び方は段 ① で先に窓口 1.3 へ替えないと段 ③ で壊れる。並走側 `default-balloon-bundle` は `vendors/sample_ghost/StayseeBalloon/` を「新規テストの定数 1 か所」で指す約束（`brief.md:110`）だが、段 ③ がその展開フォルダを追跡から外すと、その定数が指す先が消える。
- 選択肢:
  - (a) **段 ① の窓口を最終形の 3 関数で公開**（ゴースト／根／同梱バルーン）し、段 ① で 27 ファイルの `emo2-kakukaku` 継ぎ足しも窓口 1.3 へ替える。段 ③ は窓口の中身だけ替える。
  - (b) 段 ① は「根」1 関数だけ、同梱バルーンは段 ③ で一括。→ 27 ファイルを二度触る（要件 1.3 違反）。
  - (c) 並走側との窓: 段 ③ で `StayseeBalloon` を畳むのは「`default-balloon-bundle` が `main` に着地している場合のみ」。着地前なら畳まず、8.5 の後段どおり報告に書く。着地後に本仕様が先にマージされると、並走側の定数（展開フォルダ直指し）が壊れる。順序は**本仕様が先にマージ**（並走側は窓口を使って書き直す）か、**並走側が先**（本仕様が段 ③ で畳む）かを決める。
- 証拠が示す向き: **(a)＋(c) は「並走側が先なら畳む・本仕様が先なら畳まない」**。どちらの順でも壊れないのは「本仕様の窓口に `StayseeBalloon` の登記を 1 行足し、`.nar` を置く」作業を**後から着地する側**が行う形。要件 8.5 の文言は既にその形なので、変更は不要。窓口の段 ① の形 (a) は設計で固定する。

### 項目 8. 「名前は UTF-8」の印（ビット 11）の読み方

- 影響: 要件 2.2・2.3・2.4・9.4。
- 問題: `zip` 8.6 の公開 API に印を読む手段が無い（節 2.4）。
- 選択肢: 節 4.1 の A（比較で回復）／B（自前の中央ディレクトリ読み）／C（UTF-8 妥当性で近似・要件を弱める）。
- 証拠が示す向き: A か B。B は依存を最小にし要件を文字どおり満たす。A は `zip` の検査（CRC・方式）を借りられる。**設計で両者の行数と依存の差を並べて選ぶ**（Research Needed: `miniz_oxide::inflate` の API・`flate2` の `rust_backend` 既定）。C は採らない。

### 項目 9. `target/` の見つけ方

- 影響: 要件 7.1・7.10・1.9。
- 問題: 38 ファイルは全て `CARGO_MANIFEST_DIR` 相対で組む（ソースツリーからの相対）。`target/` はソースツリー直下とは限らない（`CARGO_TARGET_DIR`・`--target-dir`）。`build.rs` を持たないので `OUT_DIR` は無い。
- 選択肢:
  - (a) `CARGO_MANIFEST_DIR/../../target/<名前空間>/`（最短・`CARGO_TARGET_DIR` を無視）。
  - (b) `std::env::current_exe()` から `target/` を遡る（テストバイナリは `target/<profile>/deps/`・example は `target/<profile>/examples/`・bin は `target/<profile>/`）。`boot_config.rs:78,98` の前例と同じ入口。`CARGO_TARGET_DIR` にも追随する。
  - (c) 環境変数 `CARGO_TARGET_DIR` があればそれ、無ければ (a)。
- 証拠が示す向き: **(b)**。既存の前例があり、`cargo` の配置規則に従う限り `target` の名前のフォルダに必ず当たる（見つからなければ理由付きで失敗＝1.4 と同じ形）。1.9 のコマンドも同じ関数を呼べば同じ根を出す。

### 項目 10. 検体の窓口の置き場と依存の種別

- 影響: 要件 1.5・1.7・10.3・10.4。
- 選択肢: 節 4.3 の A（`areka-nar` 内 `dev` モジュール）／B（テスト専用 leaf を新設・`[dev-dependencies]` のみ）／C（B＋各クレートの `*_test_support.rs` から 1 行で転送）。
- 証拠が示す向き: **B＋C**。`temp-path-kit` の前例（依存 0・`publish = false`・見張りは `log-capture-kit/tests/`）に倣い、「本番 `[dependencies]` に現れたら赤」の見張りを `with_default_guard_test.rs` と同じ部品で 1 本足す。`structure.md` のクレート一覧に 2 クレート（`areka-nar`・窓口）を登記する（要件 10.4）。

### 項目 11. 台帳の `owner` 登記に伴う `roadmap-draft.md` の同時編集

- 影響: 要件 10.6・10.7。
- 問題: `owner` を 11 件書いた瞬間に判定 ⑸ の腕 c（`owner_count`）と腕 f（宛先の行き先）が赤になる。`[[spec]]` 行の追加には `bundle`（`linkage.md` に在る「インストール」）と `wave`、`[briefs].count` 27→28、それに散文 3 か所（`:67`「表に無い 2 本」・`:83`「意図しない重なり」・`:351` 段階 B の表）の書き換えが要る。`roadmap-draft.md` は全仕様の合流点（`spec_checks.rs:17-22` が「共有ファイルにしない」設計を説明）。
- 選択肢:
  - (a) **`[[spec]]` 行を足し `bundle = "インストール"`・`owner_count = 11`・`wave = "A0"` とし、散文 3 か所を直す**（要件 10.7 のとおり）。
  - (b) `owner` を書かない（台帳は着地後の `status` 更新だけ）。腕 c/f は緑のまま。要件 10.6 を落とす。
  - (c) `owner` を書き、`[[spec]]` 行の代わりに `[[owner_completed]]` を完了時に足す（腕 f は `[[owner_completed]]` も認める）。ただし 11 件が全て実装済みでないと腕 e が赤。
- 証拠が示す向き: **(a)**。`wave` の語彙が「W13〜W17」前提なら `A0` が受理されるかを `documents/parse.rs` で確認する（Research Needed・小）。`bootghost` と「相対パス」系 3 件の `owner` は空のまま `note` に理由（要件 10.6 のとおり）。

### 項目 12. 決定論テスト用の固定 `.nar` の持ち方

- 影響: 要件 9.3・9.4・`ghost-install` の brief `:95`（「固定 `.nar` 4 種は `nar-install` の fixture を再利用」）。
- 問題: 本仕様は `.nar` の**書き込み側を持たない**（Out）。しかし 20 種超の固定入力（Shift_JIS 名・シンボリックリンク・`\` 区切り・NUL・大小衝突…）は市販の zip ツールでは作りにくいものを含む。
- 選択肢:
  - (a) **テスト内で最小の zip 書き手（無圧縮・ローカルヘッダ＋中央ディレクトリ＋EOCD・100 行程度）を `#[cfg(test)]` に持ち、入力をコードで組む**。壊れた形（不正 CRC・`..`・ビット 11 の有無）を自由に作れる。追跡するバイナリは 0。
  - (b) 固定 `.nar` をバイナリで追跡（`tests/fixtures/*.nar`）。作り方の再現性が落ち、`.gitattributes` の手当てが要る。
  - (c) `zip` クレートの書き込み側を `dev-dependencies` で有効化して組む。`zip` を採らない場合（項目 8 の B）は使えず、`deflate-zopfli` 等の機能を dev 側で引く。
- 証拠が示す向き: **(a)**。`ghost-install` が同じ入力を使う約束なので、書き手は `areka-nar` の `*_test_support.rs` に置き（`structure.md:176`）、`ghost-install` から `[dev-dependencies]` で届く形にする。

## 6. requirements.md の事実誤り・要確認（修正はしない）

> **2026-09-18 要件ディスカッションでの処置**: 下表の「誤り」3 行（`tools/perf` 6→3・`doc/` 3→2・コメントだけの 11→8＋検索語の注記）と「曖昧」1 行（3.4 にキーの ASCII 大小無視を明記＝節 5 項目 3 の (a) を採用）は `requirements.md` を是正済み。節 2.8 の示唆 2 件（`accept,` の空値・フォルダのエントリ無し）は 3.11・4.9 に反映。10.1 に謝辞再生成の条件、10.7 に `roadmap-draft.md` の同時編集（項目 11 の (a)）を追記。項目 4 の 4 決定は現要件のまま維持（正典が沈黙・「あるべき姿」で決める方針）。項目 6 は (a)＝要件は変えない。**項目 1（`refresh` 実行の所有）は開発者と議論**、項目 2・5・7・8・9・10・12 は設計フェーズへ。

| 箇所 | 記述 | 実測 | 種別 |
|---|---|---|---|
| Introduction `:17`・Boundary `:34`・要件 10.5 `:212` | `tools/perf/` のスクリプト **6 本** | 検体パスを綴るのは **3 本**（`invoke-followup-checks.ps1:129`・`judge-perf.py`・`perf-loop.measure.ps1`）。`emo2` という語を含む `.ps1`/`.py` なら 7 本 | 誤り |
| 同上 | `doc/` の文書 **3 本**（`COMPAT_ARCHITECTURE.md` を含む） | **2 本**（`emo2-conformance-scope.md`・`ukadoc-coverage/briefing-assets.md`）。`COMPAT_ARCHITECTURE.md` は `fixtures`・`shiori-host-32` とも **0 件** | 誤り |
| Introduction `:17`・要件 1.8 `:66` | コメントだけで言及する **11 ファイル** | 38 と同じ検索語（`shiori-host-32/fixtures`）では **8**。11 になるのは検索語 `fixtures/emo2` のとき（付録 A-1） | 数え方の不一致（要注記） |
| 要件 2.2〜2.3 `:76-77` | 「印（汎用目的ビット 11）が立っている／いない」で決める | `zip` 8.6 の公開 API は印を返さない。実現方法は項目 8 | 実現性の注記が要る |
| 要件 3.4 `:92` | 「既存の `key,value` 層と同じ規則」 | 既存層はキーの大小を保持する。3.3 の「キーの ASCII 大小は区別しない」は `charset` 行の prescan にだけ当てはまる | 曖昧（項目 3） |
| 要件 7.10 `:169` | 「テスト用一時パスの窓口の迂回検知に触れない」 | 正しいが、`temp-path-kit` そのものも `temp_dir()` 直下に組むので**窓口を再利用できない**ことを設計に書く必要がある | 補足 |
| 要件 10.7 `:214` | `roadmap-draft.md` の表に本仕様の行を足す | 行の追加には `[briefs].count`・`bundle`・散文 3 か所の同時編集が要る（項目 11） | 補足 |
| brief `:Approach` | 「新規に graph へ入るのは `typed-path` 1 本だけ」 | 本番グラフ（`-e normal`）には `flate2`・`miniz_oxide`・`crc32fast`・`adler2`・`typed-path` が全て新規。Cargo.lock 全体では `typed-path` のみ新規、という意味なら正しい。謝辞（`ignore-dev-dependencies = true`）には複数増える | 要件 10.1 の見積りに影響 |
| brief `:Current State` | 「`pasta_sample_ghost` は build script を意図的に拒否」 | `build.rs` は在る（生成をしないだけ） | 記述の精度 |
| Introduction `:13` `:18` `:19` `:20` `:21` | 110/90/20・43・`.nar` 0・15 項目 absent/B1・4 通りの綴り・`.gitattributes` の有無・`target` 除外 | 全て一致 | 正しい |

## 7. 工数とリスク

| 段 | 工数 | リスク | 理由 |
|---|---|---|---|
| ① 収束（38＋27 ファイル・窓口 leaf・見張り 1 本） | **M** | 低 | 機械的だが 9 クレートに跨る。挙動不変はテスト名の集合と合否の同一で判定できる |
| ② エンジン（容器読取・解釈・安全性・展開・`refresh`・語彙・固定入力 25 種） | **L** | 中 | 外部依存の採否（項目 8）と印の読み方が未決。パス安全性の網羅は Windows 固有の規則が多い |
| ③ 切替（`.nar` 5 本・原本と複製の管理・削除・文書・台帳・謝辞・実機一周） | **M** | 中 | `.nar` の一致検証（8.4）は機械で確かめられる。合流点（steering・台帳・謝辞・roadmap）は完了時に `main` を取り込み数え直す前例どおり |
| 合計 | **L〜XL** | 中 | brief の見立て（M）より重い。理由: 要件 6（再インストール）・7.5〜7.9（並走・回収）・9.3（25 種の固定入力）が brief 起票後に加わった |

## 8. 設計フェーズへの推奨と Research Needed

推奨する骨格（決定ではない・節 4 の「証拠が示す向き」の集約）:

1. クレート 2 つ: 本番 `areka-nar`（読取・解釈・安全性・展開・`refresh` 実行または引数受け）と、テスト専用 leaf の検体の窓口（`[dev-dependencies]` のみ・`temp-path-kit` の型）。
2. 窓口は段 ① で最終形の 3 関数を公開し、同梱バルーンの継ぎ足し 27 ファイルも段 ① で寄せる。
3. 展開先は `current_exe()` から遡った `target/<名前空間>/` に「読み専用の原本（`.nar` の刻印付き）」と「利用者ごとの複製（pid＋連番・`Drop` で削除）」の 2 層。
4. 容器読取は `zip`（印を比較で回復）か自前読み（印を直接読む）の 2 案を、行数と本番グラフに増える crate 数で比較して決める。
5. 台帳 `owner` 11 件と `roadmap-draft.md` の `[[spec]]` 行＋散文 3 か所は同じコミットで行う。

Research Needed（設計で確定）:

- `zip` 8.6 の `deflate-flate2` が `flate2` のバックエンド機能（`rust_backend`）の明示を要るか／`miniz_oxide` 単体で inflate する API の形と行数。
- 印の「比較で回復」が全 ASCII・非 ASCII UTF-8・Shift_JIS・不正列の 4 象限で正しいことの証明（表にして較正テストへ）。
- Windows でフォルダ単位の原子的差し替え（宛先が既に在るときの `rename` の失敗・退避→置換→削除の 2 段）と、失敗時に「以前の内容が無傷」を保つ手順。
- `roadmap-draft.md` の `wave` 欄の受理語彙（`documents/parse.rs`）に `A0` が通るか。
- `cargo about` の再生成で増える crate の一覧と、その全てが `about.toml` の許可リストに入ること（`cargo deny check licenses` の実走・brief は 09-12 に緑を確認済みだが `zip` 8.6.0 は 07-10 公開で依存木が動いている可能性）。
- 決定論テスト用の最小 zip 書き手の置き場（`areka-nar/src/*_test_support.rs`）と、`ghost-install` から届く形。

## 付録 A. 再現コマンド（Git Bash・作業ツリー直下で実行・`target/` 除外）

A-1. 検体パスを綴る `.rs` の数（38＝実行行・8／11＝コメント行のみ）

```bash
# 全言及（46）
grep -rl --include=*.rs -e "shiori-host-32/fixtures" -e 'join("shiori-host-32")' crates | grep -v /target/ | sort > /tmp/all.txt
# 実行行（コメント行 ^\s*// を除いた行）で綴る（38）
for f in $(cat /tmp/all.txt); do grep -v '^\s*//' "$f" | grep -q -e "shiori-host-32/fixtures" -e 'join("shiori-host-32")' && echo "$f"; done | sort > /tmp/code.txt
wc -l < /tmp/code.txt                                   # 38
grep -c "/src/" /tmp/code.txt; grep -c "/tests/" /tmp/code.txt; grep -c "/examples/" /tmp/code.txt   # 21 / 8 / 9
comm -23 /tmp/all.txt /tmp/code.txt | wc -l             # 8（コメントだけ）
# 要件の「11」になる検索語
grep -rl --include=*.rs "fixtures/emo2" crates | grep -v /target/ | sort | comm -23 - /tmp/code.txt | wc -l   # 11
```

A-2. 私家版の定義と同梱バルーンの綴り

```bash
grep -rn --include=*.rs "fn emo2_root" crates | grep -v /target/ | wc -l          # 12
grep -rn --include=*.rs "const FIXTURE_DIR" crates | grep -v /target/             # 1
grep -rl --include=*.rs "emo2-kakukaku" crates | grep -v /target/ | wc -l         # 45
for f in $(grep -rl --include=*.rs "emo2-kakukaku" crates | grep -v /target/); do grep -v '^\s*//' "$f" | grep -q "emo2-kakukaku" && echo "$f"; done | wc -l   # 27
```

A-3. `.rs` 以外（区切り文字 `/`・`\` の両方）

```bash
grep -rlE 'shiori-host-32[\\/]fixtures|fixtures[\\/]emo2|sample_ghost[\\/]R_POST' tools | sort    # 3
grep -rlE 'shiori-host-32[\\/]fixtures|fixtures[\\/]emo2|sample_ghost[\\/]R_POST' doc | sort      # 2
grep -n -E "fixtures" crates/pilot/examples/shiori-host-32/README.md                                 # 1 行（:26）
```

A-4. 追跡数・無視規則・属性・依存グラフ

```bash
git ls-files crates/pilot/examples/shiori-host-32/fixtures | wc -l          # 150
git ls-files crates/pilot/examples/shiori-host-32/fixtures/emo2 | wc -l     # 110
git ls-files crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku | wc -l   # 20
git ls-files vendors/sample_ghost/R_POST_and_KOMAINU | wc -l                # 43
git check-ignore -v crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/profile/areka/x.toml   # 何も出ない＝無視されない
git check-attr text -- crates/pilot/examples/shiori-host-32/fixtures/emo2/install.txt vendors/sample_ghost/R_POST_and_KOMAINU/install.txt
cargo tree --workspace -e normal --prefix none 2>/dev/null | awk '{print $1}' | sort -u | grep -cx -e flate2 -e miniz_oxide -e crc32fast -e typed-path   # 0
cargo tree --workspace -e all -i flate2 2>/dev/null | head -8                # image → png → flate2（dev のみ）
```

A-5. `hello-pasta.nar` の観測（Python 3.13・`zipfile`）: `flag_bits & 0x800` が立つエントリ 0／54・`compress_type` は 8 のみ・`is_dir()` 0・`install.txt` は `accept,`（空値）。

---

# 設計フェーズの調査記録（2026-09-18・`design.md` の根拠）

> ここから下は設計生成時に足した。節 1〜8 と付録 A は要件ディスカッション時点の記録のまま残す。節 5 の未決 7 項目（2・5・7・8・9・10・12）の決定は「設計判断」に番号を引いて書く。

## 9. 外部クレートの再確認（docs.rs・GitHub のソース・crates.io を当たり直した）

### 9.1 `zip` 8.6.0

- **「名前は UTF-8」の印を返す文書化された公開 API は無い。** `ZipFile` は `HasZipMetadata` を実装し（`lib.rs` で `pub use crate::read::HasZipMetadata`）、`get_metadata(&self) -> &ZipFileData` を返す。`ZipFileData` は `pub struct` で `pub flags: u16`・`pub is_utf8: bool` を持つが、置き場の `mod types;` が非公開なので docs.rs に頁が無い（`zip/8.6.0/zip/types/struct.ZipFileData.html` は 404）。フィールド参照はコンパイルが通るが、文書に無い漏れ出し経路。節 4.1 の案 A「比較で回復」は不要になったが、代わりに「文書に無い経路に依存する」形になる。
- `name()` は読込時に 1 回だけ判定し、印なしは `from_cp437()?`（8.6 では失敗しうる）・印ありは `from_utf8_lossy`（`read.rs:547-556`）。`name_raw() -> &[u8]`「エンコードは未定義」。
- **`default-features = false, features = ["deflate-flate2"]` は単体ではコンパイルできない。** `zip` は `flate2` を `default-features = false` で引くため伸長のバックエンドが立たず、`flate2` 側の `compile_error!("You need to choose a zlib backend")` が出る。`flate2 = { version = "1.1", features = ["rust_backend"] }` を自分の直接依存に足すか、`deflate-flate2-zlib-rs` を選ぶ必要がある。brief の指定はこの罠を踏んでいた。
- 全 feature を切ったときの非任意依存は `crc32fast`・`indexmap`・`memchr`・`typed-path` の 4 本。`extract()` は `make_symlink` を呼ぶ（`read.rs:122-145,448`）。エントリ単位の `crc32()`・`header_start()`・`data_start()` は公開。8.x 最新は 8.6.0（2026-04-25・MIT）。RUSTSEC-2025-0168 は `>=1.3.0, <2.3.0` で 8.6.0 は無関係。

### 9.2 `miniz_oxide` 0.9.1（最新は 0.8 系ではなく 0.9 系・`flate2` 1.1.10 も 0.9 を引く）

- `pub fn decompress_to_vec(input: &[u8]) -> Result<Vec<u8>, DecompressError>`（zlib ヘッダ無しの生 deflate）・`pub fn decompress_to_vec_with_limit(input: &[u8], max_size: usize) -> Result<Vec<u8>, DecompressError>`（上限超過は `TINFLStatus::HasMoreOutput`）。ストリーミング `inflate::stream::inflate(&mut InflateState, input, output, MZFlush)` もあるが、エントリ単位に宣言サイズが分かる本用途では上限付きの一括版で足りる。
- 既定 feature は `with-alloc`。非任意依存は `adler2` のみ。純 Rust・`unsafe` FFI 無し。`MIT OR Zlib OR Apache-2.0`（`deny.toml` の許可リストは MIT を含むので OR 式は通る）。

### 9.3 `flate2` 1.1.10

- `default = ["rust_backend", "runtime_detection"]`・`rust_backend = ["miniz_oxide", "any_impl"]`。既定で `miniz_oxide`（`features = ["simd"]`＝`simd-adler32` も入る）と `crc32fast` を引く。`read::DeflateDecoder` は生 deflate 用。読み手だけの本用途には `miniz_oxide` を直接引くほうが 3 本少ない。

### 9.4 本番グラフに増える crate の数

| 選択肢 | 増える crate | 本数 |
|---|---|---|
| `zip 8.6`（`deflate-flate2`）＋ `flate2/rust_backend` | zip・crc32fast・cfg-if・indexmap・equivalent・hashbrown・memchr・typed-path・flate2・miniz_oxide・adler2・simd-adler32 | 12 |
| `flate2` 既定 | flate2・miniz_oxide・adler2・simd-adler32・crc32fast | 5 |
| **`miniz_oxide` 既定** | **miniz_oxide・adler2** | **2** |

## 10. コード側の再確認（設計が依拠する事実）

- `areka_parsers::charset::decode(bytes: &[u8], default: DefaultEncoding) -> String`（`decode.rs:24`）は `prescan_charset` でキーを `eq_ignore_ascii_case("charset")` で探し（`prescan.rs:58`）、`DefaultEncoding::Ansi → SHIFT_JIS`（`model.rs:37`）。`kv::parse_kv(text: &str) -> BTreeMap<String, String>`（`parse.rs:20`）。
- `workspace_scan/mod.rs`: `walk_workspace_sources()`（`crates/**/*.rs`・`target`／`vendors`／`.git` 除外・src／tests／examples を全て含む・`mod.rs:79`）・`read_source`・`scan_tokens`・`strip_comments`。`with_default_guard_test.rs:283-318` に `is_production_dependency_section`／`production_kit_dependencies` があり、本番依存の見張りはこの部品で書ける。
- `ukadoc-survey/src/documents/parse.rs:673` は `wave` を `string_field` で読む（自由文字列）。`parse_tests.rs` は `"W13"`・`"保留"` を使う。**`"A0"` は通る。** `roadmap-draft.md` の `[[spec]]` 行は `name`・`stage`・`bundle`・`owner_count`・`wave` の 5 欄（`:118-151`）。
- 38 ファイルの内訳（付録 A-1 を再実行・一致）: `areka` 13・`areka-emo-text` 7・`pilot` 4（`shiori-host-32/{helper,helper_window,main,shiori_proxy}.rs`＝i686 の helper を含む）・`areka-emo-atlas` 3・`areka-emo-compose` 3・`areka-parsers` 3・`areka-seriko` 3・`areka-emo-present` 1・`areka-ghost` 1。`emo2-kakukaku` を実行行で綴る 27 のうち 38 に無いのは 4 で、パスを組むのは `areka/examples/emo-present/setup.rs:147`（`emo2("emo2-kakukaku")`）だけ。残り 3（`balloon_model_tests.rs:10`・`balloon_target_tests.rs:434`・`state_cue_apply_tests.rs:608`）は説明文の中の綴り。
- `.rs` 以外: `tools/perf/invoke-followup-checks.ps1:129`・`perf-loop.measure.ps1:64`（既定の根を変数で持つ）・`judge-perf.py:197`（コメント）・`doc/emo2-conformance-scope.md:25`・`doc/ukadoc-coverage/briefing-assets.md:362,377,439`・`shiori-host-32/README.md:26`。
- 検体の追跡ファイル名に非 ASCII は 0（`git ls-files | grep -P '[^\x00-\x7F]'` が空）。`.nar` に畳んだときエントリ名は全て ASCII で、ビット 11 の有無は展開結果に影響しない。名前の文字コードの決定論テストは `nar_writer` で組む固定入力だけが担う。
- `vendors/sample_ghost/.gitattributes`（`* -text`）・`.gitignore`（`!*_test.txt` `!*_dump.txt`）は既に在り、`.nar` にもそのまま効く。ルートに `.gitattributes` は無い。
- `current_exe()` の前例: `boot_config.rs` の `default_helper_exe_path`・`default_app_profile_dir`（親を取るだけ）。`CARGO_TARGET_DIR` を読む箇所は 0。

## 11. 設計判断（節 5 の未決項目の決定）

### 決定 D1（項目 8）: コンテナ読取は自前・伸長だけ `miniz_oxide`

- **選択肢**: A `zip`（印は `get_metadata().is_utf8`）／B 自前の中央ディレクトリ読み＋`miniz_oxide`／C UTF-8 妥当性で近似。
- **決定**: B。
- **理由**: 要件 2.2/2.3 を文字どおり満たせる唯一の文書化された形。本番グラフの増分が 2（A は 12）。`extract()`・CP437 経路・`typed-path` を持ち込まない。拒否すべき形（9.2 の 13 語彙）を自分の語で持てる。A は `deflate-flate2` の罠と文書に無いフィールドへの依存を抱える。C は要件を弱める。
- **代償**: 読み手 350 行前後を自前で持つ（EOCD・中央ディレクトリ・ローカルヘッダの 3 構造・zip64 とマルチディスクは印を見て拒否）。CRC-32 も 30 行の表引きで自前に持つ（`crc32fast` を足すと cfg-if と 2 本増える・較正値 `0xCBF43926` を固定）。
- **追跡**: 開発者の承認（`tech.md` 登記）は `miniz_oxide` に対して求める。承認が `zip` に傾いた場合は `container.rs` だけを差し替える。

### 決定 D2（項目 2）: 原本＋取得ごとの複製・刻印は長さ＋CRC-32・札ファイルで回収

- **選択肢**: (a) 利用者ごとの複製／(b) 共有の木＋`profile/` 削除／(c) 毎回再展開。刻印は (i) 全体ハッシュ／(ii) 長さ＋更新時刻／(iii) エントリ CRC の列。
- **決定**: (a)＋(i)（長さ＋自前 CRC-32・`.nar` は展開時に丸ごと読むので追加 I/O 無し）。
- **回収の形**: pid の生存判定（`OpenProcess`＝`windows` crate が要る）ではなく、**共有モード `FILE_SHARE_READ` だけで開いたままの札ファイル**で生存を表す。他プロセスの `remove_file` は共有違反で失敗し、持ち主が死ねば成功する。`std::os::windows::fs::OpenOptionsExt::share_mode` だけで書けて依存 0。
- **原本の完全性**: 「名前が合う原本は完全」を不変条件にする。作るときは作業フォルダで組んでから `rename` で入れ、消すときは `rename` で作業フォルダへ出してから消す。半端な木が `cache/` の名前の下に見えることが無い。
- **多重プロセスの初回**: 両者が作業フォルダで組み、`rename` の負け側（宛先フォルダが既に在ると Windows の `rename` は失敗する）は自分の作業を消して勝者を使う。
- **代償**: 取得のたびに emo2 で 6.6 MB・110 ファイルの複写（SSD で 100〜200 ms）。ワークスペース全体で 100 回前後＝数十秒だが並列に散る。共有の木 (b) は同一プロセスの並走で 7.5 を破る余地があるので採らない。
- **手動用**: bin が配る木はプロセス終了後も要るので `manual/<名>/` に置き、呼ぶたびに作り直す（回収の対象外）。

### 決定 D3（項目 9）: `target/` は `CARGO_TARGET_DIR` → `current_exe()` の祖先で名前が `target` のフォルダ

- 38 ファイルの `CARGO_MANIFEST_DIR` 相対はソースツリーからの相対で `--target-dir` に追随しない。`current_exe()` はテストバイナリ（`target/<profile>/deps/`）・example（`target/<profile>/examples/`）・bin（`target/<profile>/`）のどれでも `target` に当たる。見つからなければ `TargetDirNotFound { started_from }`。`CARGO_TARGET_DIR` は 2 行で読めるので先に見る。

### 決定 D4（項目 10）: 窓口は `sample-ghost-kit`（テスト専用 leaf）＋各消費クレートは `[dev-dependencies]` 1 行

- 本番から見えないことを依存の向きで保証し、`with_default_guard_test.rs` と同じ部品で「本番 `[dependencies]` に現れたら赤」を 1 本置く。各クレートの `*_test_support.rs` からの転送は、`SampleRoot` の読み口が借用を返す型なので `pub(super) fn emo2_root() -> PathBuf` の形には写せない。呼び手は `let sample = SampleRoot::acquire("emo2")?;` を 1 行置き、以降 `sample.folder()` を使う（差分はパスを得る行に限る）。
- `areka-parsers` の dev 依存が `sample-ghost-kit` → `areka-nar` → `areka-parsers` と自分へ戻るのは dev 依存の循環で cargo が許す形（`tokio` ⇄ `tokio-test`）。テストバイナリに `areka-parsers` が 2 度リンクされるが、窓口はパスしか渡さない。
- i686 の `pilot` helper も同じ窓口を使う。`areka-nar`・`miniz_oxide`・`sample-ghost-kit` は純 Rust なので i686 でも組める。

### 決定 D5（項目 7）: 段 ① で読み口 3 つの**形**を公開し、`root()` の**提供**は段 ③

- `folder()`・`balloon()` は段 ① で最終形の署名で公開し、中身だけ段 ③ で切り替える。段 ① で 27 ファイルの `emo2-kakukaku` 継ぎ足しも `balloon()` へ寄せる（1.3）。`root()` は段 ① の呼び手に使う者がおらず、追跡済みの展開形には「根」に相当するフォルダが無い（`fixtures/` の直下に `ghost/` は無い）ので、嘘の値を返すより段 ③ で足す。A1 の 3 仕様は段 ③ の後に着手する。
- 並走側 `default-balloon-bundle` との窓: 要件 8.5 のとおり「後から着地する側が `.nar` 1 本と登記 1 行を行う」。順序をどちらにも固定しない。

### 決定 D6（項目 5）: 見張りの語彙は 4 種＋本番依存

- ⑴ `shiori-host-32/fixtures`・`join("shiori-host-32")` ⑵ `vendors/sample_ghost/<登記名>/` ⑶ `nar-samples` ⑷ 同梱バルーンの名前でパスを組む形だけ＝`join("emo2-kakukaku")`・`emo2-kakukaku/`・`/emo2-kakukaku"`。設計レビューの指摘で ⑷ を「引用符込みの丸ごと」から「パスを組む形」に狭めた——窓口の読み口 `balloon("emo2-kakukaku")` の引数は 1.3 の正規の使い方で、丸ごとの綴りを禁じると呼び手 27 ファイル全部が赤になる。説明文の中の綴り（実在 3 件）も当たらない。走査語は `concat!` で 2 片に割り、除外は `crates/sample-ghost-kit/src/` だけ。段 ① から置く。
- 本番依存の見張りは `with_default_guard_test.rs` の私有関数（`manifest_lines`／`production_kit_dependencies`・`log-capture-kit` の名前を固定で持つ）を `workspace_scan/mod.rs` へ移してクレート名を引数化し、両方の見張りが呼ぶ。

### 設計レビュー（2026-09-18・Fable・1 回で通過）で直した点

- **`open` で全エントリを伸長・CRC 突合し、伸長済みバイト列を保持する**（当初は `install` の書出し中に伸長していたため、CRC 破損の `.nar` が `root/.nar-work/` へ書いた後に拒否され 2.6／4.1 に反していた）。
- **10.2 の字義からの逸脱を明示**: `miniz_oxide` は伸長と圧縮を feature で分けられない。代替は「`areka-nar/src/` が `miniz_oxide::deflate` を綴らない」字面の見張り。`zip`＋`flate2` 経路でも同じ。
- **段ごとの `Cargo.toml`**: 段 ① の kit は `thiserror` だけ・段 ② で `areka-nar`・`miniz_oxide`（`nar_writer` の deflate 側）を足し `devroot` の `target/` 発見と `WorkDir` を段 ② へ前倒し（`areka-nar` のテストが根に使う）・段 ③ で原本・複製・手動用。
- **9.2 の判定は 1 本のテスト**が 13 変種の固定入力を全部組んで `kind()` の集合と `ALL_KINDS` の完全一致を見る（兄弟テスト間で状態を集める形は成立しない）。
- **`supplement` で `refresh` を読まない**ことを 6.2 の狭めとして traceability と Open Questions に登記。
- **Windows の `rename`** は宛先が**空でない**フォルダのときに失敗する（空フォルダは std 1.98 が置き換える）。原本・宛先は常に空でないので設計の不変条件は保たれる。

### 決定 D7（項目 12）: `.nar` 書き手は `sample-ghost-kit::nar_writer`

- `areka-nar/src/*_test_support.rs`（`#[cfg(test)]`）に置くと `ghost-install` の `[dev-dependencies]` から届かない。feature で公開モジュールにすると本番クレートにテスト専用の書き手が同居する。`sample-ghost-kit` に置けば `areka-nar` 自身のテストも後続も `[dev-dependencies]` 1 行で同じ入力を得る。書き手は無圧縮を既定にし、deflate は `miniz_oxide::deflate`（dev 側だけ）で任意に掛ける。壊れ方（CRC・長さ・データ・暗号化ビット・方式・S_IFLNK・印の有無）を口として持つ。

### 決定 D8: 確定は「作業フォルダで組み上げてから入れ替える」

- `Overlay`（refresh≠1）は既存の木を作業フォルダへ複写してからアーカイブを上書き、`Replace { keep }`（refresh,1）は mask に合うファイルだけ複写してからアーカイブを書く。確定は `dest → old`・`stage → dest`・`old` 削除の 3 手で、2 手目の失敗は `old → dest` で戻す。要件 6.2 の「消してから展開」と結果は同じで、「消した後に展開が失敗する」経路が構造的に無くなる（6.4 の記録は `old` の削除失敗＝`leftovers` と巻き戻し失敗＝`committed` の 2 つに縮む）。作業フォルダは `root/.nar-work/<pid>-<連番>/`（同一ボリューム＝`rename` 可・`ghost/`／`balloon/` の列挙に混ざらない）。
- 同じ根への同時インストールは想定しない（製品は単一インスタンス・開発用の根は取得ごとに別）。

### 決定 D9: 正典が沈黙する小さな決定（設計で追加したもの）

- `install.txt` の名前は ASCII 大小無視で探す（Windows のファイルシステムの意味論・`Install.txt` を拒否しない）。
- 拒否は最初の 1 件で全体を止める（製品側が写す理由は 1 つ・4.1 は「書き込み前に止まる」ことが要点）。
- `supplement` では `refresh` を読まない（重ね置き先はゴースト本体で、全消去は危険。書かれていれば警告）。
- 同梱バルーンの取り出し元フォルダ内の `install.txt` は普通のファイルとして置く（解釈はしない。8.4 の 20 ファイル一致に必要）。
- 大小衝突の比較は Unicode の小文字化（Windows の照合表とは厳密には違うが、安全側に倒れる）。
- 宣言サイズの総和は 1 GiB で打ち切る（利用者の `.nar` を受ける経路の信頼境界）。

## 12. 統合（一般化・採用か自作か・簡素化）

- **一般化**: 「宛先ごとの配置」を `Placement` 1 つの形にしたので、ghost／balloon／shell／supplement／同梱バルーンの 5 経路が同じ確定コードを通る。`ExistingPolicy` を本体と同梱バルーンで共有し、6.3 は追加コード無しで満たす。
- **採用か自作か**: 伸長は採用（`miniz_oxide`）。zip 構造の読み・CRC-32・パス安全性・Windows 名規則は自作（採用候補 `zip` は印の公開 API が無く、`enclosed_name()` は CP437 経路。`crc32fast` は 2 本増える）。一時パスの窓口 `temp-path-kit` は OS の一時フォルダに組むので採れない（7.10）。
- **簡素化**: 引数で `refresh` を上書きする口は置かない（呼び手が `accept` で入れるか入れないかを決めるだけで、入れ方を変える者はいない）。刻印ファイルは置かず原本のフォルダ名に刻印を入れる。pid の生存判定は札ファイルで代替し `windows` crate を窓口に入れない。stage ① の `root()` は作らない。

## 13. リスクと対策（設計時点）

- 依存の承認が `zip` に傾く → `container.rs` だけ差し替え・他の契約は不変。
- 複写の費用がテスト時間を押し上げる → 計時を `debug!` に出し、実測で問題なら「読み専用で使う呼び手向けの共有複製」を**別の読み口**として足す（既定は変えない）。
- 走行中に `.nar` を差し替えると古い原本の回収が別プロセスの複写と競合しうる → 開発者の操作でしか起きない。赤になった走行をやり直す。
- dev 依存の循環でビルド時間が延びる → `areka-parsers` は小さい。実測で問題なら `areka-nar` の `kv`／`charset` 呼び出しを最小の複製に置き換える判断を設計ディスカッションへ。
- `StayseeBalloon` の有無で段 ③ の成果物が変わる → 8.5 の条件分岐のまま。報告に「どちらだったか」を書く。

## 14. 参照

- docs.rs `zip` 8.6.0 `struct.ZipFile.html`／`trait.HasZipMetadata.html`、GitHub `zip-rs/zip2` `v8.6.0` の `src/read.rs`・`src/types.rs`・`src/lib.rs`・`Cargo.toml`、crates.io `zip/8.6.0/dependencies`。
- docs.rs `miniz_oxide` `inflate::decompress_to_vec_with_limit`・`inflate::stream::inflate`、GitHub `Frommi/miniz_oxide` `Cargo.toml`。
- GitHub `rust-lang/flate2-rs` `Cargo.toml`・`src/lib.rs`（`compile_error!`）。
- rustsec.org `RUSTSEC-2025-0168`。
- ukadoc `manual_install`・`manual_directory`・`descript_install`（`refresh`・`refreshundeletemask`・`*.directory`・`*.source.directory`・`accept`）。
- 本リポジトリ: `crates/areka-parsers/src/charset/{decode,prescan,model}.rs`・`kv/parse.rs`・`crates/log-capture-kit/tests/{with_default_guard_test,temp_path_guard_test}.rs`・`workspace_scan/mod.rs`・`crates/ukadoc-survey/src/documents/parse.rs`・`tests/consistency/spec_checks.rs`・`crates/areka/src/boot_config.rs`・`crates/temp-path-kit/src/lib.rs`。
