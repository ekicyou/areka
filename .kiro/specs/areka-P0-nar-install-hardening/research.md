# Gap Analysis: areka-P0-nar-install-hardening

> 2026-09-23・本ブランチ HEAD `92f5f448`（`a86a8dfa` で spec 初期化済み）。要件 `requirements.md` は確定済みで、本書は**要件を変えず**、既存コードとの差と実装の選択肢を並べる。引用は行番号でなく「何の定義か」で指す。実測は `crates/areka-nar/src/` と `crates/sample-ghost-kit/src/`・`crates/log-capture-kit/src/` を読んで取った。

## 1. 要約（3〜5 行）

- **新しい仕組みは要らない。** 3 つの要件はいずれも既存の関数の並びに 1 手を足す形で収まる（長さの検査＝`names.rs` の `validate_one` と `is_valid_one_level_name`／在りか＝`NarError::Io` に欄 1 つ／片付けの例外＝`prepare_shelf` に条件 1 つ／テスト＝既存の助手 `hold`・`capture`・`tree` の組み合わせ）。
- **確定の段の失敗を公開の入口で起こす手は、既に本 crate の中で実証済み。** `install_commit_tests.rs` の場合 ⑴ が「宛先のファイルを読みだけ共有して開いたまま → 1 手目の `rename(dest → old-k)` が os error 5 で失敗 → `phase: Commit`・`rolled_back: true`」を毎回踏んでいる。要件 3 はその同じ仕掛けを `NarArchive::install` 越しに置き直し、記録の `work` の欄を読むだけで済む。
- **「本当に戻せなかった巻き戻し」で元の木が生き残る形は、`commit_all` を通しては作れない**（テストが先に開いたハンドルは、確定の `rename` も巻き戻しの `rename`/削除も同じように塞ぐので、確定だけ通して巻き戻しだけ塞ぐことができない）。要件 2.6 が許す「確定の部品の単位」＝`roll_back` を直接呼ぶ形で、**新しい木の側**を掴んで `remove_tree(dest)` を失敗させれば、`old-k` が元のバイト列のまま残る決定論の形になる（既存の「退避先が消えている」形は `old` が無いので 2.2 の「中身で判定」に使えない）。
- **決めるべきことは 6 件**（§7）。大きいのは⑴上限の値と数え方（要件の裁定候補 1）、⑵片付けの例外の有無（裁定候補 2）、⑶失敗の値に足す欄が「作業フォルダ」か「`old-<k>`」か（要件 2.1 と 2.4 の読み方）、⑷長さの拒否を新しい語にするか `UnsafePath` の下位理由にするか（要件 1.8）。
- 規模 **S**・リスク **低**。触るファイルは本番 4（`names.rs`・`error.rs`・`lib.rs`・`install.rs`）＋テスト 5＋文書 1。呼び手の追随は 0。

## 2. いまの資産の地図（要件 → 既存の部品・欠けているもの）

| 要件 | 既存の部品（何の定義か） | 状態 |
|---|---|---|
| 1.1〜1.4・1.6〜1.9 エントリの長さ | `names.rs` `validate_one`（復号 → NUL → `\` → 絶対 → `..` → 空要素 → Windows 名 → シンボリックリンク）。`EntryName.path` が「`/` 区切り・末尾 `/` 無し」の正規形 | **Missing**: 長さの項が無い。並びの中の位置は未定（§3.1） |
| 1.4 有界の一部だけを載せる理由 | `RefuseReason::UnsafePath { index, name, why }` は `name` に全体を載せる。`NameUndecodable` は `raw_hex` に生バイト全体の 16 進を載せる | **Missing**: 名前を切り詰めて持つ理由の形が無い。**Constraint**: 既存の理由の形は変えない（付録 A） |
| 1.5 `install.txt` のフォルダ名 | `names.rs` `is_valid_one_level_name`（空・`/`・`\`＋`is_usable_windows_name`）。呼び手は `manifest.rs` `check_one_level`（拒否 `InvalidDirectoryName`）・`parse_mask`（警告 `InvalidMaskEntry`）・`plan.rs` `existing_target_ghost`（拒否 `TargetGhostMissing`） | **Missing**: 長さの項。足せば 3 つの呼び手に同時に効く（`target_ghost` にも効く＝要件に無い副作用・§3.1 ⑸） |
| 1.6 定数 1 か所 | `container.rs` `MAX_TOTAL_DECLARED_SIZE`（総量の上限）が先例。`pub(crate) const` | 型は先例どおり。置き場は `names.rs` が自然（§3.1） |
| 1.10 検体 5 本の受理 | `sample-ghost-kit/src/lib_tests.rs` `every_registered_sample_lands_where_its_registry_row_says` | 変更 0 で緑のはず（最長 53 ＜ 候補 200） |
| 2.1〜2.3 失敗の値に在りかの欄 | `error.rs` `NarError::Io { archive, phase, path, source, committed, rolled_back }`。組み立て箇所は `lib.rs` に 3（`read` の Read・`io()` の Stage・`place` の Commit/Rollback）＋`error_tests.rs` に 2。crate の外は 0 | **Missing**: 欄。5 か所の追随で済む |
| 2.4・2.5 記録の `work` の欄 | `lib.rs` `log_failure(failure, work)`。`place` が `WorkArea::create` 直後に `*work = Some(area.path())` を置く（巻き戻せたかに関わらず） | 実装は 2.5 の意味。**注釈だけ**が「`rolled_back` が真なら空」と言う → 注釈を直す |
| 2.6 欄を外すと赤になるテスト | `install_commit_tests.rs` `a_rollback_that_cannot_finish_reports_the_first_stuck_destination`（`roll_back` を直接呼ぶ・退避先が無い形） | **Constraint**: この形は `old` が実在しないので 2.2 の「中身で判定」に流用できない（§3.3） |
| 2.7 片付けの例外 | `install.rs` `prepare_shelf`（棚の全項目を消す・自分の番地が消せなければ失敗・他は `residue`）。`residue` は成功時だけ `InstallOutcome.leftovers` に合流し `lib.rs` の `install` が warn で 1 件ずつ出す | **Missing**: 「`old-` を含む番地は消さない」条件。**Constraint**: 失敗の経路では `residue` がどこにも出ない（§3.4） |
| 3.1〜3.3 公開の入口で確定を失敗させる | `install_commit_tests.rs` `hold`（`share_mode(1)`）＋場合 ⑴。`lib_vocabulary_tests.rs` の `capture`＋`at(records, ERROR)`＋`field("work")` | 部品は全部ある。**Missing**: 公開の入口を通す 1 本 |
| 3.4 欄ごとに比べる | `log-capture-kit` `CapturedEvent::field`（Debug 表現）／`field_str`（`record_str` 経路の生値のみ） | **Constraint**: `work = %path.display()` は `record_debug` 経路なので `field_str` は `None`。`field` で読む（§3.3） |
| 3.5 13 の固定入力を弱めない | `lib_vocabulary_tests.rs` `cases()`・`every_refusal_kind_has_a_fixture_and_is_recorded_once`（集合の完全一致＋`observed.len() == 13`） | 手書きの 13 は 4 か所（§3.2） |
| 3.7 1,000 行 | 実測: `lib.rs` 253・`lib_tests.rs` 390・`lib_vocabulary_tests.rs` 351・`install.rs` 469・`install_tests.rs` 710・`install_commit_tests.rs` 689・`names.rs` 218・`names_tests.rs` 685・`manifest_tests.rs`（未計測・既存） | 各 +30〜80 行で収まる。番人 `log-capture-kit/tests/file_length_guard_test.rs` の例外表に `areka-nar` は 0 件 |
| 4.1 §8 の 1 行 | `doc/COMPAT_ARCHITECTURE.md` §8 の表（4 列: 項目／裁量／根拠／出典 spec）。末尾は `\x` の行 | 追記だけ。`shiori-loadu` と末尾で競合し得る（要件どおり後着が取り込む） |
| 4.3 `sample-ghost-kit` 変更 0 | `nar_writer.rs` `NarBuilder::file(name: impl Into<Vec<u8>>, …)`。名前の長さは `u16::try_from(name_raw.len())` で 64 KiB まで | 長い名前の固定入力は既存の口で組める |

### 2.1 前提の訂正（brief の「30 万文字」）

zip の中央ディレクトリの名前の長さ欄は **16 bit**（`container.rs` `read_central_header` が `u16_at(bytes, at + 28)` で読む）。よって 1 エントリの名前は生バイトで最大 **65,535**、UTF-16 の単位ではそれ以下になる（UTF-8 も Shift_JIS も 1 単位あたり 1 バイト以上）。「30 万文字」は zip の形式上は作れない。ただし 65 k の名前でも作業フォルダの段で OS が落ちる（要素 255 超）・記録に 65 k の名前が載る、という問題はそのままなので、**上限を置く必要は変わらない**。要件の文面には触れないが、設計と §8 の根拠の行では「65,535 バイト」を使う。

### 2.2 見落としやすい既存の制約

- **`names.rs` は字面の見張りを持つ**（`names_tests.rs` `names_source_contains_no_filesystem_mutation`）。本体に `fs::`・`path::`・`process::` の綴りが 1 つでも入ると赤。長さの定数の注釈に「`std::path::`」と書くだけで赤になるので、注釈は「パス」と書く。`encode_utf16` は綴りに当たらない。
- **記録を出す本番ソースは `lib.rs` だけ**（`lib_tests.rs` `only_the_public_surface_writes_records`・`tracing::error!` はちょうど 1 つ）。片付けの例外（2.7）を `install.rs` で「記録に残す」形にしてはならない。残す物は `residue` に載せ、出すのは既存の `lib.rs` の warn。
- **`lib_tests.rs` の `before.len() >= 14`** は「固定入力 13＋受理 1」の手書きの数。語を足す形なら 15 に改める（`>=` なので赤にはならないが、数が嘘になる）。
- **`error_tests.rs` `samples_in_declaration_order`** は変種を足すと `all_kinds_matches_every_variant_in_declaration_order` が赤になる仕掛け（意図どおり）。語を足す形なら見本を 1 つ足す。
- `NarError` は `std::io::Error` を持つので `PartialEq` が無い。欄を足しても「欄を外しても緑」の罠は `NarError` では起きにくいが、`CommitError` にも足すなら同じ注意（brief の制約）。
- Rust の `std::fs` は Windows で長い絶対パスに `\\?\` を自動で付ける（`sys/windows` の `maybe_verbatim`）。つまり **260 を超えても areka 自身の書き込みは通り得る**（NTFS の要素 255・全体 32,767 が本当の壁）。上限の意味は「areka が落ちないため」ではなく「後で読む側（`LoadLibrary` で `shiori.dll` を読む等・長いパス非対応の利用側）を守る」「異常な名前を入口で見つける」。§8 の根拠に書く価値がある（Research Needed: このツールチェーンでの実挙動の 1 回の確認）。

## 3. 要件ごとの分析

### 3.1 要件 1: 長さの上限

**数え方（1.2）**: `s.encode_utf16().count()`（std・1 行）。検体 6 体では文字数・UTF-8 バイト数と同じ値になるので、較正のテストは **BMP 外の文字**（例: `𠮷` U+20BB7 ＝ 2 単位）を 1 つ混ぜ、「文字数では 200 なのに単位では 201 で拒否される」を判定する。

**検査の位置（1.3・付録 A の「復号の後」）**: `validate_one` の中の候補は 3 つ。

| 位置 | 内容 | 利点 | 難点 |
|---|---|---|---|
| P1: 復号の直後・NUL の前 | `name.strip_suffix('/')` の長さを最初に見る | 長い名前が後段の `UnsafePath { name: 全体 }` へ**一度も写らない**（1.4 の趣旨に最も近い）。他の検査に長い文字列を通さない | 順序のテスト `the_checks_run_in_a_fixed_order_and_the_first_hit_wins` に「長い＋`..`」の 1 行を足す。モジュール注釈の順序の列挙を直す |
| P2: 空要素の後・Windows 名の前 | 正規化済み `trimmed`／`components` で数える | 1.1 の文面（正規化を終えた形）と字義が一致 | NUL・`\`・`..` を含む長い名前は既存の理由で拒否され、その理由が 65 k の名前を丸ごと載せる（既存の振る舞い・1.4 の趣旨に反しはしないが趣旨からは遠い） |
| P3: シンボリックリンクの後・最後 | 同上 | 既存の順序に手を触れない | P2 と同じ難点＋「長さは最後」という順序を新たに説明する必要 |

推す形は **P1**。「長さ」は他の検査より先に効く条件（長すぎる名前はどの検査でも意味を成さない）として説明でき、後段へ長い文字列を渡さない。

**拒否の形（1.4・1.8）**:

| 形 | 内容 | 追随する箇所 |
|---|---|---|
| V1: 新しい語 `PathTooLong { index, length, limit, head }` | `refuse_reasons!` に 1 変種。`head` は先頭 N 単位（N は定数・例 32）。表示は「エントリ {index} の名前が長すぎる（{length} 単位・上限 {limit}・先頭 {head}…）」 | `error.rs` 注釈の「13 変種」・`error_tests.rs` の `all_kinds_has_thirteen_entries`（名前と数）と `samples_in_declaration_order`・`lib_vocabulary_tests.rs` の注釈 3 か所と `observed.len() == 13`・`cases()` に 1 件・`lib_tests.rs` の `>= 14`。**`kind()` は「PathTooLong」という独立の語**になり、`ghost-install` が `OnInstallFailure` の理由へそのまま写せる |
| V2: 下位の理由 `UnsafeWhy::TooLong { length, limit }` | `UnsafePath { index, name, why }` を流用。ただし `name` に全体を載せられないので、この変種のときだけ `name` を先頭 N 単位に切り詰める | 手書きの数は 0 か所。`cases()` は既存の `UnsafePath` で足りる（1.8 は「下位の理由なら固定入力を足すだけ」と読める）。**難点**: `name` の意味が「全体」と「先頭だけ」の 2 通りになり、注釈で断らなければ読み違える。`kind()` は「UnsafePath」のままで、利用側が長さ超過を区別するには `why` まで見る必要がある |

どちらも差分は小さい。**V1 を推す**——語彙が「閉じている」ことの意味は「短い語 1 つで理由が分かる」であり、名前の切り詰めを `UnsafePath` に混ぜると `name` の契約が割れる。

**`install.txt` のフォルダ名（1.5）**: `is_valid_one_level_name` に `&& name.encode_utf16().count() <= LIMIT` を足す 1 手。効く先は 3 つ——`check_one_level`（拒否 `InvalidDirectoryName`・1.5 のとおり）・`parse_mask`（警告・1.5 のとおり）・**`plan.rs` `existing_target_ghost`（呼び手が渡す宛先ゴーストの名前・要件に記載なし）**。3 つ目は「200 単位を超える名前のゴーストは宛先として選べない」という副作用で、実害は無い（そもそも 1.5 の上限を通らないと作れない名前）が、設計で明記する。

**注意**: `InvalidDirectoryName { value }` は値の全体を載せる。`directory` の値が 65 k 文字なら理由の 1 行が 65 k 文字になる。1.4 は「上限超えの拒否の理由」全般に読めるので、ここが 1.5 の「既存の理由で拒否」と衝突し得る（§7 の決定 5）。

**上限の値（1.6・裁定候補 1）**: 定数は `names.rs` に `pub(crate) const MAX_ENTRY_PATH_UTF16: usize = 200;` の形（名前は設計で決める）。§8 の行と拒否の表示は必ずこの定数を綴る（表示は `{limit}` 欄で自動・§8 は文書なので手書き）。値の候補 200／255 は要件の付録 B のとおり。

**境界のテスト（1.9）**: `open` 経由で「ちょうど 200 が通る」「201 が拒否」の 2 本＋`install.txt` の `directory` で同じ 2 本＋UTF-16 の較正 1 本。固定入力は `with_manifest().file(format!("g/{}", "a".repeat(198)), b"x")` の形（`g/`＋198 ＝ 200）。`NarBuilder::file` は任意のバイト列を受けるので新しい口は要らない（4.3）。置き場は `names_tests.rs`（`validate_entry_names` の単位）と `lib_tests.rs`（`open` の単位）のどちらか。要件は「公開の入口 `open` を通して」と言うので `lib_tests.rs`。

### 3.2 要件 1 と 3.5 が触る手書きの数（一覧）

語を足す形（V1）で改める数は次の 4 ファイル 7 か所。下位の理由（V2）なら 0。

- `error.rs`: マクロ内の注釈「13 変種で閉じる」
- `error_tests.rs`: `all_kinds_has_thirteen_entries`（関数名の thirteen と `13`）・注釈「13 変種すべてに」
- `lib_vocabulary_tests.rs`: 注釈 3 か所（先頭・`cases()`・テスト）・`observed.len() == 13`
- `lib_tests.rs`: `before.len() >= 14`

完了 spec の文書（`completed/areka-P0-nar-install/*`）にも「13」が在るが、完了 spec は書き換えない（本 spec の付録 A の方針）。

### 3.3 要件 2・3: 在りかの欄と確定の段のテスト

**確定の段の失敗は決定論で起こせる（3.1・3.2）**。根拠は既存の `install_commit_tests.rs` 場合 ⑴（`a_destination_in_use_fails_the_commit_and_leaves_it_byte_identical`）。宛先の中のファイルを `share_mode(FILE_SHARE_READ)` で開いたままにすると、**1 手目** `rename(dest → old-k)` が os error 5 で失敗し、`commit_one` は `undo` に何も積まないまま返る → `roll_back` は `unwind(空)` で `None` → `CommitError { phase: Commit, path: dest, rolled_back: true, committed: [] }`。宛先は 1 バイトも動かない。この形を `NarArchive::install` に置き直すと `NarError::Io { phase: Commit, path: dest, rolled_back: true, committed: [] }` になり、`log_failure` が `work = <根>/.nar-work/<pid>-<連番>` を載せて error を 1 件出す。**新しい部品は 0**——`hold` を `lib_tests.rs`（または新設の兄弟ファイル）へ写す（`std::os::windows::fs::OpenOptionsExt`）。

`open` の後に `install` を呼ぶ前に宛先を作って掴む必要がある（`open` は宛先を読まない。`install` の計画の段で `destination.is_dir()` を見て `Overlaid` になる）。

**記録の欄の読み方（3.3・3.4）**: `log_failure` は `work = %work.unwrap_or_default().display()` と書く。`%` は `tracing::field::display` → `record_debug(format_args!("{}", …))` の経路なので、`CapturedEvent::field("work")` の Debug 表現は**引用符無しの表示文字列**（既存テストの `trim_matches('"')` は実は無くても通る）。`field_str("work")` は `None`。よって新しいテストは `field("work")` を取り、`Path::new(…)` にして `is_dir()` と `starts_with(root.join(".nar-work"))` を判定する。欄ごとに比べる（3.4）は自然に満たす。

**失敗の経路では作業フォルダが残る**（`WorkArea` に `Drop` 無し・`place` の失敗経路に片付け無し）ので、`work` が指すフォルダは失敗の値を受け取った時点で実在する。テストの後始末は `WorkDir::drop` が根ごと消す。

**「本当に戻せなかった巻き戻し」で元の木が残る形（2.6・2.2）**:

- `commit_all` を通しては作れない。理由: テストが先に開いたハンドルは、宛先側なら 1 手目の `rename` を、組み上げ側なら 2 手目の `rename` を塞ぐ（場合 ⑴⑵）。「確定は通るが巻き戻しは塞がる」には、確定と巻き戻しの間に割り込む第 2 のプロセスか、`rename` は許して削除だけ拒む共有モード（`FILE_SHARE_DELETE` 付きで開いた子を持つフォルダを `rename` できるかは未確認・Research Needed）が要る。
- **確定の部品の単位**なら決定論で作れる: 作業フォルダに `old-0/`（元の木）を置き、宛先に新しい木を置いてその中のファイルを `hold` し、`roll_back(vec![Undo::Restore { old, dest }], committed, failure)` を呼ぶ。`unwind` の `remove_tree(dest)` が掴まれたファイルで失敗（共有に削除が無いので os error 32）→ `stuck = dest` → `rolled_back: false`。このとき `old-0/` は触られず、元のバイト列のまま残る。既存の「退避先が消えている」形との違いは **`old` が実在する**こと——2.2 の「中身で判定」はこちらでしか成り立たない。
- 欄の実在の判定は `tree(old)` を呼ぶ前の木と `assert_eq!`（`install_tests.rs` の `tree` は相対パスとバイト列で写す）。

**欄の中身（2.1〜2.4・§7 決定 3）**: 3 つの読み方がある。

| 形 | 値 | 2.1「命名規則を知らなくてよい」 | 2.4「`work` と同じ場所」 | 実装 |
|---|---|---|---|---|
| F1: 作業フォルダ | `Some(<根>/.nar-work/<pid>-<連番>)`（`rolled_back` が偽のときだけ） | △ 呼び手は「この下の `old-<番号>`」を知る必要がある | ○ 逐語で同じ | `place` が `area.path()` を持っているので、`NarError::Io` を組む 1 か所で `surviving: (!failure.rolled_back).then(\|\| area.path().to_path_buf())` の 1 行 |
| F2: `old-<k>` そのもの | `Some(<作業フォルダ>/old-<k>)`（最初に躓いた `Restore` の `old`） | ○ | △ 「配下」と読む | `unwind` の躓きの報告に `old: Option<PathBuf>` を足し、`CommitError` にも足す。躓いたのが `Undo::Remove`（新規の宛先を消せなかった）なら `None`——**このとき利用者の元の木は無い**（宛先は新規だった）ので `None` が正しい |
| F3: 一覧 `Vec<(宛先, old)>` | 躓いた `Restore` 全部 | ○ | △ | `unwind` が最初の躓きしか報告しない設計を変える必要がある。要件 2.8（手順を変えない）に触れる |

**F1 の見落とし**: `rolled_back` が偽でも、躓いたのが `Undo::Remove` だけなら作業フォルダに `old-` は 1 つも無い（利用者の元の木はそもそも無い）。F1 で `Some(作業フォルダ)` を返すと「元の木が生き残っている」と読める場所に何も無い。F1 を採るなら「`old-` が 1 つでも在るときだけ `Some`」の条件を足す（`unwind` に「躓いた `Restore` があったか」の真偽を持たせる）か、欄の意味を「作業フォルダ（元の木が在れば `old-<番号>` の下）」と注釈する。**F2 が最も要件の字義に近く、追加は `unwind`→`roll_back`→`CommitError`→`place` の 4 か所に `old` を運ぶだけ**（各 1〜2 行）。

**`CommitError` にも欄を足すか**: `place` は `CommitError` を `NarError::Io` へ写す。F1 なら `CommitError` は触らない。F2 なら `CommitError` に `old` の欄が要る（`install_commit_tests.rs` の既存 4 場合＋巻き戻しの 1 本が `CommitError` を組む／読むので、欄を足しても `..` で受けている箇所は追随不要）。

**2.5 の注釈**: `log_failure` の注釈「`rolled_back` が真なら空」を「この走行が作業フォルダを掘ったならその場所（巻き戻せたかに関わらず）」へ直す。実装は変えない。3.3 のテストが `rolled_back: true` で `work` が非空であることを判定するので、注釈の側が固定される。

### 3.4 要件 2.7: 棚の片付けの例外

現行の `prepare_shelf`: 棚 `<根>/.nar-work/` の全項目を消す。消せない項目は、自分の番地なら失敗（`StageError`）・他なら `residue` へ。`residue` は `WorkArea::residue()` → `commit_all` の成功時に `InstallOutcome.leftovers` へ → `lib.rs` `install` が warn「work folder left behind」を 1 件ずつ出す。

**足す条件**: 項目がフォルダで、その直下に `old-` で始まる名前の子が 1 つでも在れば、消さずに `residue` へ載せる。`read_dir` を 1 段だけ増やす（5〜8 行）。

**論点**:

1. **自分の番地に `old-` が在るとき**（プロセス識別子の再利用＋連番 0 の一致・起こり得る）: 現行は「消せなければ失敗」。例外を足すと「消さない」→ その直後の `create_dir_all(dir)` は通り、前回の `old-` と今回の組み上げが同じ番地に同居する。選択肢は⒜ `StageError` で止める（利用者が `old-` を退避するまでその根へ入れられない）⒝ 連番を進めて別の番地を取る（`NEXT_SERIAL.fetch_add` をもう 1 回・番地が空くまで繰り返す）。⒝ は 3〜4 行で、利用者の作業を止めない。
2. **失敗の経路では `residue` が捨てられる**: `place` が `WorkArea::create` の後で失敗すると `area.residue()` はどこにも出ない。2.7 の「報告し続ける」は成功の経路でだけ成り立つ。失敗の経路でも出すなら `log_failure` に `leftovers` を渡す配線が要る（`place` の失敗の値に `residue` を載せるか、`work` と同じく預かり先で渡す）。要件は「報告し続ける（既存の出口）」なので、成功の経路だけで足りると読めるが、設計で明記する。
3. **消さない番地は丸ごと残す**（`<k>/` の組み上げ済みの木も一緒）。`old-` だけ残して `<k>/` を消す形は「部分的に消す」新しい手順になるので採らない（2.8）。
4. `sample-ghost-kit` の開発用の根は再インストールを通らない（完了 spec 6.5・7.x）ので、開発用の根に `old-` が積み上がる経路は無い。

**テスト**: `install_tests.rs` に「`999999-0/old-0/descript.txt` を置いて `WorkArea::create` → 残っている・`residue()` に載る」の 1 本と、既存 `stale_work_folders_are_swept_before_staging_starts`（`old-` の無い残骸は消える）が対照になる。論点 1 を ⒝ で採るなら「自分の番地に `old-` → 別の番地が返る」の 1 本。

### 3.5 要件 4: 文書と「触らない」の明示

- `doc/COMPAT_ARCHITECTURE.md` §8: 4 列の表の末尾に 1 行。項目＝「`.nar` の 1 要素の相対パスの長さ」、裁量＝「上限 N（UTF-16 の単位・`install.txt` の `directory` 系にも同じ値）」、根拠＝「正典 `descript_install` は『半角英数推奨』のみで長さに沈黙・zip の名前欄は 16 bit（最大 65,535 バイト）・検体 6 体の最長 53・`MAX_PATH` の算術」、出典＝本 spec。
- 「触らない」の判定は `git diff --stat` で `doc/ukadoc-coverage/`・`vendors/sample_ghost/`・`crates/sample-ghost-kit/`・`.kiro/steering/` が 0 であることを完了時に確かめる（実装の一部ではなく検証手順）。`areka-nar/src` に `// ukadoc:` が 0 行のままであることは `git grep` 1 回。

## 4. 実装の選択肢

### 案 A: 既存の関数に足す（推す）

- `names.rs`: 定数 1・`validate_one` の P1 に 3 行・`is_valid_one_level_name` に 1 条件・（V1 なら）`error.rs` に 1 変種。
- `install.rs`: `prepare_shelf` に「`old-` を含むなら残す」の 5〜8 行・（F2 なら）`unwind`/`roll_back`/`CommitError` に `old` を運ぶ 4 か所・（論点 1 ⒝ なら）`WorkArea::create` に番地の取り直し。
- `lib.rs`: `NarError::Io` の組み立て 3 か所に欄・`log_failure` の注釈。
- テスト: `lib_tests.rs`（境界 4 本＋較正 1 本＋確定の失敗 1 本）・`lib_vocabulary_tests.rs`（V1 なら `cases()` に 1 件＋数）・`install_commit_tests.rs`（`roll_back` 直接呼びで `old` が残る 1 本・欄を外すと赤）・`install_tests.rs`（片付けの例外 1〜2 本）・`error_tests.rs`（V1 なら見本＋数）・`names_tests.rs`（順序に 1 行）。
- 利点: 新しいファイル 0（テストの行数が 1,000 に迫るなら `lib_commit_tests.rs` を 1 本足す程度）。既存の見張り（字面・記録の発火点・語彙の完全一致）が全て効いたまま。
- 難点: `lib_tests.rs` が確定の失敗のテストで `OpenOptionsExt` を持つようになる（`install_commit_tests.rs` と同じ `hold` の複製 1 つ）。複製を避けるなら `install_tests.rs` の助手を `pub(super)` にして借りるが、`lib_tests` → `install::tests` の可視性の道が無いので、複製 10 行のほうが素直。

### 案 B: 新しい部品を建てる

- 例: `limits.rs`（長さの規則）・`survivor.rs`（在りかの型）を新設。
- 利点: 無し。規則は 1 定数＋1 比較で、型は欄 1 つ。
- 難点: 字面の見張り（`names.rs` の走査は `include_str!("names.rs")` なので新設ファイルは見られない）・記録の見張り・`the_deflate_scan_looks_at_real_files_and_catches_the_spelling` の一覧など、見張りの対象を増やす手間が生じる。**採らない**。

### 案 C: 混合

- 案 A に加えて、確定の失敗のテストだけを新しい兄弟ファイル `lib_commit_tests.rs`（`lib_tests.rs` から `#[path]` で繋ぐ）に置く。
- 採る条件: `lib_tests.rs` が 1,000 行に近づくとき、または `hold`＋`tree`＋`capture` を並べたテストが 150 行を超えるとき。現状 390 行なので、案 A のまま `lib_tests.rs` に置いて足りる見込み。`the_new_files_stay_under_the_line_limit` の一覧に新設ファイルを足す。

## 5. 規模とリスク

- **規模: S**（1〜3 日）。本番の差分は 4 ファイルで合計 40〜60 行。テストは 5 ファイルで 200〜300 行。文書 1 行。
- **リスク: 低**。全て既存の型と関数の延長。未知の技術は無い。唯一の未確認は `\\?\` の自動付与と `FILE_SHARE_DELETE` の挙動だが、どちらも**設計の根拠**に関わるだけで実装の手順は変わらない（Research Needed 2 件・§6）。
- 並走の干渉: `crates/areka-nar/src/` を触る A1 の他の枝は 0（roadmap 干渉台帳）。`doc/COMPAT_ARCHITECTURE.md` §8 の末尾だけ `shiori-loadu` と競合し得る。

## 6. 設計段階へ持ち越す調査（Research Needed）

1. **Rust std の `\\?\` 自動付与**: このツールチェーン（Rust 2024・Windows 11）で `std::fs::write` に 260 超の絶対パスを渡したとき通るか。§8 の根拠の書き方が変わる（「areka 自身が落ちる」か「読む側が落ちる」か）。1 回の実測で足りる。
2. **`FILE_SHARE_DELETE` で開いた子を持つフォルダの `rename`**: 通るなら「確定は通り・巻き戻しの削除だけ失敗」を `commit_all` 越しに決定論で作れ、要件 2.6 のテストを公開の入口まで引き上げられる。通らなければ §3.3 の部品の単位の形で確定。設計で 1 回試す価値はあるが、無くても要件は満たせる。
3. **`is_valid_one_level_name` の長さが `existing_target_ghost` に効く副作用**の扱い（明記して受け入れるか、`target_ghost` だけ別の判定にするか）。別にする案は「土台を二重に書く」ことになり、`names.rs` の注釈が禁じている形なので、明記して受け入れるのが自然。

## 7. 設計判断の候補（要件ディスカッションへ渡す・番号順）

1. **上限の値と数え方**（要件の裁定候補 1）: 200（`MAX_PATH` の算術で作業フォルダの段まで収まる）か 255（NTFS の要素と同じ・覚えやすい）か。数え方は UTF-16 の単位で固定でよいか。→ 答えで変わるのは定数 1 つと §8 の 1 行の文言だけ。
2. **棚の片付けの例外**（要件の裁定候補 2）: 採るか。採るなら「自分の番地に `old-` が在るとき」の扱い＝⒜ 失敗で止める／⒝ 連番を進めて別の番地を取る（§3.4 論点 1）。失敗の経路でも `residue` を報告するか（論点 2）。
3. **失敗の値に足す欄の中身**（§3.3 の F1／F2／F3）: 作業フォルダ（1 行・要件 2.4 の字義）か `old-<k>`（4 か所・要件 2.1 の字義・`Undo::Remove` で躓いた場合は `None` が正しい）か。要件 2.1 と 2.4 のどちらを字義で守るかの問題。推すのは F2。
4. **長さの拒否の語**（要件 1.8・§3.1 の V1／V2）: 新しい語 `PathTooLong`（語彙 14・手書きの数 7 か所を同じ変更で改める）か `UnsafeWhy::TooLong`（`UnsafePath.name` をこの場合だけ切り詰める）か。推すのは V1。
5. **`InvalidDirectoryName { value }` が長い値を丸ごと載せること**（要件 1.4 と 1.5 の間）: 既存の理由のまま（`install.txt` の値は書き手が自分で書いたもので、書庫のエントリ名ほど敵対的でないと見る）か、この理由も先頭だけに切り詰めるか（既存の理由の形を変える＝付録 A の「触らない」に抵触）。推すのは既存のまま＋設計で明記。
6. **検査の位置**（§3.1 の P1／P2／P3）: 復号の直後（長い文字列を後段へ渡さない・順序テストに 1 行）か、正規化の後か。推すのは P1。答えでテストの固定入力が 1 行変わる。

## 8. 完了 spec への影響の確認

付録 A の突合は本書の実測と一致した。追加で気付いた点は 1 つ——`plan.rs` `existing_target_ghost` が `is_valid_one_level_name` を共有しているので、1.5 の変更は完了 spec 要件 5.7（宛先ゴーストの名前の検証）にも**足す**形で及ぶ。完了 spec の要件には反しない（長い名前のゴーストはそもそも 1.5 を通らないと作れない）が、付録 A の表に 5.7 の行を「足すだけ」として設計で補う。

## 9. 要件ディスカッションでの扱い（2026-09-23）

§7 の 6 件と §6 の調査を、要件ディスカッションで次のように振り分けた。

- **要件で確定した（要件を改めた）**
  - §7-1 上限の値と数え方 → **200・UTF-16 の単位**（要件 1.6・付録 B）。
  - §7-3 失敗の値に足す欄の中身 → 要件 2.1 を「戻せなかった宛先**ごと**の生き残りのフォルダを**全て**」、2.4 を「`work` の作業フォルダの**配下**」、2.3 に「新規の宛先を消す手だけが躓いた場合は 0 件」を足した。§3.3 の F2 の「最初に躓いた 1 つだけ」では、2 つ目以降に躓いた宛先の元の木を呼び手が知る手段が無い（`unwind` は 1 つ戻せなくても残りを戻し続ける）。`unwind` が躓きを全て集めて返すのは手順の変更ではない（要件 2.8 に触れない）。失敗の値の `path` は今までどおり最初の 1 つ。
  - §7-5 `InvalidDirectoryName` が長い値を丸ごと載せること → 要件 1.5 に「上限を超えた値の全体は理由にも記録にも載せない」を足した（`install.txt` の値は zip の名前の欄を通らないので、エントリ名よりさらに長くなり得る）。載せ方の形は設計で決める。
  - §2.1 の前提の訂正（65,535 バイト）→ 要件の「いま何が起きているか」と 1.4 を改めた。
  - §8 の 5.7 の行 → 要件の付録 A に足した。
- **設計へ持ち越す（how）**
  - §7-4 拒否の語（`PathTooLong` か `UnsafeWhy::TooLong` か）。要件 1.8 が設計に委ねている。
  - §7-6 検査の位置（P1／P2／P3）。
  - §7-3 の表し方（`Vec<PathBuf>` か宛先との組か）と、`unwind`→`roll_back`→`CommitError`→`place` への運び方。
  - §7-5 の載せ方（`InvalidDirectoryName` の欄の形）。
  - §7-2 の細部（自分の番地に `old-` が在るときの扱い⒜／⒝・失敗の経路での残り物の報告）＝片付けの例外を採る場合のみ。
  - §6 の調査 3 件（`\?\` の自動付与・`FILE_SHARE_DELETE` で開いた子を持つフォルダの `rename`・`existing_target_ghost` への副作用は明記して受け入れる）。
- **開発者と決めた**: §7-2 片付けの例外 → **7 日の期限付きで残す**（要件 2.7〜2.10）。開発者の条件は「手で消さない限り永久に残るのは不可・期限があって消えるなら可」。期限は展開の開始時の片付けでだけ判定する。設計へ持ち越すのは、失敗した時刻を何で持つか（作業フォルダの更新時刻・名前に刻む・印のファイル）と、テストで実際の日数を待たずに期限の内外を作る方法（時刻を渡す口か、フォルダの時刻を書き換えるか）。
- **`ghost-install` へ申し送った**: 巻き戻せなかった元の木を利用者にどう見せるか（在りかを告知に載せるか・「7 日後に自動で消える」と伝えるか・連番付きの別ゴーストとして救い出す形を採るか）。

## 10. 設計段階の調査と決定（2026-09-23・`design.md` の根拠）

> 設計の入力は §1〜§9。ここでは §6 の調査 3 件の実測と、§9 で「設計へ持ち越す」とした 5 件の決定を記す。実測は本ブランチ（HEAD `50c3fef5`・rustc 1.98.1・Windows 11・`HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled = 1`）で、使い捨ての小さな実行体（scratch）を 1 回走らせて取った。結論は `design.md` に再掲してあり、本節は背景。

### 10.1 調査の結果（Research Log）

#### 長い絶対パスに対する Rust 標準ライブラリの挙動（§6-1）

- **Context**: §8 の根拠を「areka 自身が落ちる」と書くか「読む側が落ちる」と書くか。
- **Sources**: 実測（UTF-16 で 431 単位の絶対パス）・Rust std の Windows 実装（長い絶対パスへ `\\?\` を自動で付ける）。
- **Findings**: `create_dir_all`・`write`・`rename`（フォルダ）・`remove_dir_all` の 4 つが全て成功した。ただし本機は OS の設定 `LongPathsEnabled` も 1 なので、「設定が 0 の機体で std の自動付与だけで通るか」は本機では切り分けられない（std の実装上は通るはず）。
- **Implications**: §8 の根拠は「areka 自身の書き込みは 260 を超えても通り得る。上限の意味は、長いパスに対応しない読む側（`LoadLibrary` 等）を守ることと、異常な名前を入口で見つけること」と書く。上限の値（200）の算術（要件 付録 B）は変えない。

#### `FILE_SHARE_DELETE` で開いた子を持つフォルダの `rename`／削除（§6-2）

- **Context**: 「確定は通るが巻き戻しは失敗」を `commit_all` 越しに 1 本の走行で作れるか（要件 2.6 のテストを公開の入口まで引き上げられるか）。
- **Findings**: 子を `FILE_SHARE_READ` だけで開いた場合: 親フォルダの `rename` は os error 5・`remove_dir_all` は os error 32（既存のテストの前提どおり）。子を `FILE_SHARE_READ | FILE_SHARE_DELETE` で開いた場合: 親フォルダの `rename` は**やはり os error 5**・`remove_dir_all` は成功（子はその場で名前空間から消える）。
- **Implications**: 確定の `rename` と巻き戻しの `rename` は同じ操作なので、テストが先に開いたハンドルはどちらも同じように塞ぐ。共有モードを変えても非対称は作れない。要件 2.6 が許す「確定の部品の単位」（`roll_back` を直接呼ぶ）で固定する（`design.md` 在りかの収集）。

#### `is_valid_one_level_name` の長さが `existing_target_ghost` に及ぶ副作用（§6-3）

- **Findings**: `plan.rs` の `existing_target_ghost` は同じ土台を使うので、200 単位を超える名前のゴーストは宛先に選べなくなる。そうしたゴーストは要件 1.5 を通らないと作れない。`TargetGhostMissing.target` は呼び手が渡す値で書庫由来ではない。
- **Implications**: 明記して受け入れる（`plan.rs` は変更 0・`target` は有界にしない）。

### 10.2 設計決定（Design Decisions）

#### 決定 1: 拒否の語は新しい変種 `PathTooLong`（§7-4・V1）

- **Alternatives**: V1 新しい語 `PathTooLong { index, length, limit, head }`／V2 `UnsafeWhy::TooLong` で `UnsafePath.name` をこの場合だけ切り詰める。
- **Selected**: V1。宣言の位置は `NameUndecodable` の直後（検査の順序＝復号 → 長さ）。
- **Rationale**: `UnsafePath.name` の契約（全体を載せる）を割らない。`kind()` が独立の語になり `ghost-install` がそのまま写せる。手書きの数（4 ファイル 7 か所）は同じ変更で改め、`ALL_KINDS` との突合が漏れを赤にする。
- **Trade-offs**: 語彙が 14 になり、完了 spec の文書の「13」は動かさない（完了 spec は書き換えない）。

#### 決定 2: 検査の位置は復号の直後（§7-6・P1）

- **Selected**: `validate_one` の復号直後・NUL の前。測る対象は末尾の `/` を除いた名前。
- **Rationale**: 長い名前を後段の `UnsafePath { name: 全体 }` へ一度も写さない（要件 1.4 の趣旨）。受理される名前では末尾の `/` を除いた形と正規化後の `path` が一致するので、1.1 の「正規化を終えた全体」と同じ長さになる。
- **Follow-up**: `names_tests.rs` の順序のテストに「長さが全ての理由より先に勝つ」を 1 本。モジュール注釈の順序の列挙を改める。

#### 決定 3: 在りかは `Vec<SurvivingTree { destination, path }>`（§7-3 の表し方）

- **Alternatives**: `Vec<PathBuf>`（`old` だけ）／宛先との組。
- **Selected**: 宛先との組を持つ小さな公開型。`unwind` が躓いた `Restore` ごとに `old.is_dir()` を見て集め、`Unwound { stuck, survivors }` → `roll_back` → `CommitError.survivors` → `place` → `NarError::Io.survivors`。
- **Rationale**: 呼び手が「どのゴーストの元の木がどこに在るか」を対で利用者へ伝えられる。`Undo::Remove` の躓きは元の木が無いので集めない（要件 2.3）。`old` が実在しない躓きも集めない（要件 2.2 の「実在する」を型の値で守る。実運用で起きるのは外から消された場合だけ）。
- **Trade-offs**: `CommitError` にも欄が増え、`install_commit_tests.rs` の既存 4 場合は `..` で受けているので追随不要。記録の欄は増やさない（在りかは `work` の配下＝要件 2.4）。

#### 決定 4: `InvalidDirectoryName.value` は形を変えず中身を有界にする（§7-5）

- **Selected**: `names.rs` の `bounded_value`（上限の内側なら全体・超えていれば先頭 32 単位＋測った長さ＋上限）を `check_one_level` と `parse_mask` の `value` に通す。
- **Rationale**: 既存の理由の形（欄）を変えない（付録 A の「足すだけ」）。`InvalidMaskEntry` の警告も同じ助手を通し、記録に載る値をどの出口でも有界にする。
- **Trade-offs**: `value` の意味が「値または有界の表示」になる。注釈で断り、`bounded_value(v) == v`（上限の内側）を不変条件として書いた。

#### 決定 5: 保持の時刻は作業フォルダの更新時刻・テストは `now` を引数で渡す・自分の番地は避ける（§7-2 の細部）

- **Alternatives**（時刻の根拠）: ⑴ 作業フォルダ `<pid>-<連番>/` の更新時刻／⑵ `old-<k>/` の更新時刻／⑶ 印のファイル／⑷ 名前への刻印。
- **Selected**: ⑴。`old-<k>` は作業フォルダの直下へ `rename` で入るので、直下が最後に変わった時刻＝失敗した走行の最後の操作の時刻（失敗の直前）。⑵ は `rename` でフォルダ自身の更新時刻が変わらないため利用者のゴーストの最終更新時刻になり不適。⑶⑷ は失敗の経路に新しい書き込みを足す。
- **Alternatives**（テストでの時刻）: 時刻を引数で渡す／フォルダの更新時刻を書き換える。
- **Selected**: `WorkArea::create_at(root, now)`（本番の `create` は `SystemTime::now()` を渡す薄い皮）。更新時刻の書き換えは新しい依存かフォルダのハンドルの書き込み権が要るので採らない。
- **Alternatives**（自分の番地が保持中）: ⒜ `StageError` で止める／⒝ 連番を進めて別の番地を取る。
- **Selected**: ⒝ `next_address(shelf, now, serial)`。連番の供給源を閉包で受け、テストは 0 始まりの閉包で「`<pid>-0` が保持中なら `<pid>-1`」を決定論で判定する（`NEXT_SERIAL` はプロセス全体で共有され並走するテストが進めるので、値を予測するテストは書けない）。
- **Rationale**: ⒜ はプロセス識別子の再利用と連番 0 の一致で、利用者が最長 7 日その根へ入れられなくなる。⒝ は数行で、保持されていない自分の番地が消せないときの既存の失敗はそのまま残る。
- **Trade-offs**: 更新時刻は失敗の時刻の近似（同じ走行の中）。期限切れ後の削除が途中で失敗すると更新時刻が進み再び保持側に倒れ得るが、消せない物は元々 `residue` に載るので利用者の見える結果は同じ。

### 10.3 統合の 3 つの見方（Synthesis）

- **一般化**: 4 か所（書庫のエントリ名・`install.txt` の `directory` 系・mask の要素・宛先ゴーストの名前）に同じ 1 つの定数と同じ数え方が効く。定数と数え方と有界の表示を `names.rs` に集め、他は呼ぶだけ。
- **作るか採るか**: 数え方は `str::encode_utf16().count()`（std）。時刻は `std::time`＋`Metadata::modified`。新しい crate 0。
- **簡素化**: 新しいファイル 0・新しい記録の欄 0・失敗の経路での残り物の報告 0・「最新の 1 件だけ残す」規則 0・印のファイル 0。`Unwound` と `SurvivingTree` の 2 つの小さな型だけを足す。

### 10.4 残るリスク（Risks & Mitigations）

- 復号できない 65,535 バイトの名前は `NameUndecodable.raw_hex` が 131,070 文字を記録に載せる（付録 A により本仕様は触らない）→ `design.md` の Open Questions に記し、`ghost-install` の告知の設計時に再検討。
- `doc/COMPAT_ARCHITECTURE.md` §8 の末尾は `shiori-loadu` と競合し得る → 後着が取り込む（要件どおり）。
- 手書きの数（13→14）の直し漏れ → `ALL_KINDS` との完全一致の判定と `all_kinds_matches_every_variant_in_declaration_order` が赤にする。
