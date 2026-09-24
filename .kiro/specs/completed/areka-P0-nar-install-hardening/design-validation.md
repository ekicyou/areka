# Design Validation: areka-P0-nar-install-hardening

> 2026-09-24・本ブランチ HEAD `d588b670`（design.md 生成済み）。入力は `requirements.md`（4 要件・34 受入基準）・`design.md`・`research.md` §10・`brief.md`・steering（`structure.md`・`tech.md`・`logging.md`）。設計が既存コードについて述べている事実は `crates/areka-nar/src/` の実物（`install.rs`・`names.rs`・`lib.rs`・`error.rs`・`manifest.rs`・`container.rs`・兄弟テスト 6 本）に当たって確かめた。引用は行番号でなく「何の定義か」で指す。

## Review Summary

設計は要件 34 項目を全て既存の関数・型への追加だけで満たす形になっており（新ファイル 0・新依存 0・新しい記録の出口 0）、既存コードについての記述は実物と一致している（下の「事実の突合」）。実装の道筋は関数の単位まで具体的で、着手可能。残る懸念は 2 件で、いずれも設計の記述を数行足すだけで解けるもの（振る舞いの選択を 1 つ明文化する・既知の穴の登記を 1 つ広げる）であり、構造の見直しは要らない。

## 事実の突合（設計の主張 vs. 実物）

| 設計の主張 | 実物 | 判定 |
|---|---|---|
| `install_commit_tests.rs` の `hold` は `share_mode(1)`（`FILE_SHARE_READ`） | `const SHARE_READ: u32 = 1;`・`fn hold(path) -> fs::File` が `OpenOptionsExt::share_mode(SHARE_READ)` で開く | 一致 |
| 失敗の記録の `work` は `place` が `WorkArea::create` 直後に預け、巻き戻せたかに関わらず載る。注釈だけが「`rolled_back` が真なら空」 | `place` が `*work = Some(area.path().to_path_buf())`・`log_failure` の注釈が「`rolled_back` が真なら空」 | 一致（注釈の側を直す方針は妥当） |
| 全数対応の固定入力は 13・手書きの数は 4 ファイル | `lib_vocabulary_tests.rs` `observed.len() == 13`・`error_tests.rs` `all_kinds_has_thirteen_entries`・`lib_tests.rs` `before.len() >= 14`・`error.rs` 注釈「13 変種」 | 一致（`lib_vocabulary_tests.rs` の注釈は 3 か所でなく 4 か所＝「13 件はすべて拒否」の行も在る。実害なし） |
| `unwind` は最初の躓きだけを返す・`Undo::Remove`／`Restore { old, dest }` の 2 変種 | `fn unwind(undo: Vec<Undo>) -> Option<StageError>`・`stuck.get_or_insert(...)` | 一致 |
| `prepare_shelf` は棚の全項目を消し、自分の番地が消せなければ `StageError`・他は `residue` | `if path == dir { return Err(StageError {...}) } residue.push(path)` | 一致 |
| `is_valid_one_level_name` の呼び手は `check_one_level`・`parse_mask`・`plan.rs` `existing_target_ghost` の 3 つ | grep で同 3 か所 | 一致 |
| `names.rs` の字面の見張りは `fs::`・`path::`・`process::` の 3 語 | `FORBIDDEN_SPELLINGS.len() == 3` の較正あり | 一致（`encode_utf16` は当たらない） |
| `NarError::Io` を全欄で組む箇所は `lib.rs` 3・`error_tests.rs` 2 | 同数。`error_tests.rs` の破壊的束縛は `..` 付き | 一致（`survivors` を足す追随は 5 か所で済む） |
| 行数: `lib_tests.rs` 390・`lib_vocabulary_tests.rs` 351・`install_tests.rs` 709・`install_commit_tests.rs` 689・`names_tests.rs` 684・`error_tests.rs` 383 | `wc -l` で同値 | 一致（見積り後も全て 1,000 未満） |

## Critical Issues

### 🔴 Critical Issue 1: 保持の判定「`old-` を直下に持つ」は、成功した展開の後片付けの失敗にも当たる

**Concern**: `commit_all` は全配置を確定した後に `remove_tree(area.path())` を 1 回呼び、失敗しても確定は取り消さず `leftovers` に載せる（既存）。このとき作業フォルダには入れ替え済みの旧木 `old-<k>/` がそのまま残る。設計の「棚の片付けの判定」は `old-` の有無だけを見るので、この残骸も **7 日のあいだ消さずに `residue` へ載せ続ける**。今は毎回消しにいく（掴んでいたハンドルが離れれば次の展開で消える）が、設計後は 7 日間 1 度も試みない。
**Impact**: 要件 2.7 が守りたいのは「巻き戻せなかった元の木」であり、成功後の旧木はその外。要件 2.8 は「元の木を含まない残骸は毎回消す（変更 0）」と言うが、この残骸は旧木を含むので字義ではどちらにも収まらず、設計が決めていない。利用者から見える差は「成功した展開の残り物の警告が最長 7 日続く」。
**Suggestion**: 振る舞いを変えずに**明文化する**のが最小。「成功後の後片付けに失敗した作業フォルダも `old-` を含めば同じ規則で 7 日守る（入れ替え前の木がそのまま残るので、利用者にとっては同じ価値がある）」を「棚の片付けの判定」と「棚の保持」の Postconditions に 1 行ずつ足し、`ghost-install` への申し送り（告知の文面）にも同じ 1 行を足す。区別したい場合は成功経路で `old-<k>` を先に消す 1 手を足せるが、手順の変更（2.11）に触れるので推さない。
**Traceability**: 要件 2.7・2.8・2.11
**Evidence**: design.md「System Flows › 棚の片付けの判定」（`HasOld` の腕）・「棚の保持 › Postconditions」／`install.rs` `commit_all` の後片付け（`remove_tree(area.path()).is_err()` で `leftovers` へ）

### 🔴 Critical Issue 2: 長い名前が記録の 1 行に丸ごと載る穴は `NameUndecodable` だけではない

**Concern**: `container.rs` の `unsupported(...)` は `RefuseReason::UnsupportedEntry { index, name, what }` の `name` に `String::from_utf8_lossy(name_raw)`（生バイト全体）を載せ、これは `read_central_directory` の中＝**長さの検査（`validate_entry_names`）より前**に起きる。暗号化・zip64・非対応の圧縮方式を持つ 65,535 バイトの名前は、設計後も記録の 1 行にそのまま載る。設計の「Open Questions / Risks」は `NameUndecodable.raw_hex` だけを既知の穴として登記しており、こちらが漏れている（`IntegrityMismatch.name` は名前の検査の後なので上限の内側＝問題なし）。
**Impact**: 要件 1.4 の字義（上限超えの拒否の理由）には反しないが、要件が禁じたい形（65,535 バイトの名前が記録の 1 行になる）が別の理由で残る。`ghost-install` の告知設計で再検討する際、登記が無いと見落とす。
**Suggestion**: 「Open Questions / Risks」の `raw_hex` の項に `UnsupportedEntry.name`（`container.rs` の `unsupported`）を並記し、「塞ぐなら `from_utf8_lossy` の写しを先頭の有界の一部に切る 1 行だが、`container` → `names` の向き上 `bounded_value` は使えないので、切り詰めの助手を `error.rs` に置くか別途扱う」と書く。本仕様の範囲は変えない（付録 A「2.1〜2.7 触らない」に従う）。
**Traceability**: 要件 1.4（趣旨）・付録 A 2.5
**Evidence**: design.md「Open Questions / Risks › 復号できない名前の `raw_hex`」／`container.rs` `fn unsupported`（`String::from_utf8_lossy(name_raw).into_owned()`）と `lib.rs` `read` の順序（`read_central_directory` → `validate_entry_names`）

## Design Strengths

1. **既存の継ぎ目だけで組み、見張りを 1 つも弱めない。** 新ファイル 0・新依存 0（時刻操作 crate を入れず `create_at(root, now)` の引数 1 つで決定論を得る）・記録の出口 0 追加。`names.rs` の字面の見張り・記録の発火点の見張り・語彙の全数一致・宣言順の突合が全て効いたまま、手書きの数の直し漏れは生成器との突合で赤になる。要件 2.6 の「欄を外すと赤」を型の欄（`CommitError.survivors`・`NarError::Io.survivors`）で組めなくする形で満たしているのも堅い。
2. **調査の結果を正直に設計へ反映している。** `FILE_SHARE_DELETE` を足しても親フォルダの `rename` が失敗する実測から「確定は通るが巻き戻しは失敗」を 1 本の走行では作れないと結論し、要件 2.6 が許す部品の単位（`roll_back` 直接呼び＋新しい木の側を掴む）で 2.2 の「中身で判定」を成立させている。時刻の根拠（作業フォルダの更新時刻・`old-<k>` 自身の時刻は不適）の選択理由も明確。

## Final Assessment

**Decision: GO**

**Rationale**: 既存アーキテクチャとの不整合は無く、34 の受入基準はそれぞれ関数の単位まで対応先が示され、既存コードについての記述は全て実物と一致した。上の 2 件はどちらも設計文書に数行足せば閉じる（1 件目は振る舞いの明文化、2 件目は既知の穴の登記の拡張）ので、実装を止める理由にならない。

**Next Steps**:
1. 設計ディスカッションで Issue 1 の扱い（明文化して受け入れる／成功経路で区別する）と Issue 2 の登記の文面を決め、`design.md` に反映する。
2. `/kiro-spec-tasks areka-P0-nar-install-hardening` でタスクを生成する。
3. 実装時の注意（非致命）: `lib_vocabulary_tests.rs` の「13」の注釈は 3 か所でなく 4 か所（「13 件はすべて拒否」の行を含む）。同じ変更で改める。
