# ギャップ分析: areka-P0-mcp-ghost-name-match

> 2026-10-05・本ブランチ（main `44fc0a61` の上・spec の初期化コミット `3cb4dcea`）の実物で調べた。コードは「何の定義か」（関数名・型名・テスト名＋ファイルパス）で指し、行番号では指さない。事実の正本は `doc/ssp-mcp/survey.md` §7.2・§7.4。本書は判断の材料と選択肢を並べるもので、決めるのは要件ディスカッションと設計。

## 1. 分析の要約

- **照合の判断そのものは小さく直せる。** `crates/areka/src/mcp/resolve.rs` の `resolve` の最初の分かれ目から「空文字を省略と同じに扱う腕」を外し、名前との比べ方を「前後の空白を除いた上で半角の英字の大小を同じとみなす」に替え、本体側名（`sakura.name`）を比べる相手に足せば、要件 1〜3 の答えはそろう。空文字と空白だけは、何とも一致しないので自然に `Cannot find` になる（0 体のときも）。標準ライブラリの `str::trim`（descript を読むときと同じ関数）と `str::eq_ignore_ascii_case` で足り、新しい道具は要らない。
- **ウェーブ C4 の約束（`resolve.rs`・`resolve_tests.rs`・`mcp_tests.rs` の 3 つだけを触る）は、このままでは守れない。** 理由は 2 つ。⑴ 本体側名を持たせるために型 `ActiveGhost` に欄を足すと、この型を字面（`ActiveGhost { name, root }`）で組んでいる**約束の外のテストファイル 10 本・12 か所**がコンパイルできなくなる。そのうち `get_status_tests.rs`（C4-⑥ `mcp-get-status` の持ち物）・`dump_surface_tests.rs`・`dump_balloon_tests.rs`（C4-⑧ `mcp-dump-images-residue` の持ち物）は同じウェーブの並走 spec と字面が重なる。⑵ 本体側名と関係なく、**`crates/areka/src/mcp/get_log_tests.rs` の `unmatched_empty_and_no_ghost_are_cannot_find` が英字の大小違い（`emily/phase4.5`）を「外れ」と固定している**。要件の「今の形を固定しているテスト」の列挙（要件の「実物で引き直して確かめた点」5）から漏れていた 1 本で、要件 1.1 を満たした時点で赤になる。
- **振り分け（`mcp/mod.rs`）・各ツールの処理・`get_log.rs`・`crates/areka-mcp/`・`ghost_session.rs` の本番コードは変えずに済む**ことは確かめた。空文字はプロトコル側で `Some("")` のまま届き（`null` と欄なしだけが `None`）、`get_expression_table` の空文字の文言は `resolve` の中で決まり、`get_log` は解決の後に一覧の値（名前）で記録を絞るので、本体側名や大小違いで指しても同じ記録が返る。
- **選択肢**: A（`ActiveGhost` に欄を足し、外の 12 か所を 1 行ずつ直す）・B（`resolve` の引数で本体側名を別に渡す）・C（並べ方で解く＝⑥・⑧ の着地の後に回す、または先に着地させて向こうが追随する）・D（本体側名だけを後へ送る）。どれを取ってもウェーブの約束か要件のどちらかを改める必要があり、**開発者の判断が要る**（下の「設計の判断が要る項目」1・2）。
- **規模は S・危険は低（論理）／中（並走との調整）。** 判断のコードは 10 行前後。手間の大半はテストの書き換えと、外のテストファイルへの波及の扱い。

## 2. 今の姿（調べた実物）

### 2.1 照合の 1 か所と、その入出力

| 何の定義か | 場所 | 今の振る舞い |
|---|---|---|
| 型 `ActiveGhost`（欄 `name: Option<String>`・`root: PathBuf`） | `crates/areka/src/mcp/resolve.rs` | 起動中のゴースト 1 体を表す値。本体側名は持たない。`Debug, Clone, PartialEq, Eq` を導出。`Default` は無い |
| 関数 `active` | 同上 | World の置き場 `GhostSlot` から 1 体を読み、`GhostSession::names()` の `name` だけを取る（空なら `None`）。ルートは `std::path::absolute` で絶対化 |
| 関数 `resolve` | 同上 | `None | Some("")` を「省略」として扱い、`Omitted::Reject` なら `NOT_ACTIVE`、`Omitted::UseActive` なら起動中の 1 体（0 体なら `NOT_ACTIVE`）。それ以外は `name` と**完全一致**か `same_path` で一致すれば解決、外れは `CANNOT_FIND` |
| 関数 `same_path` | 同上 | 全文字の小文字化・`/` を `\` へ・末尾の `\` を落として比べる。**前後の空白は除かない**（要件 2.3・4.1 と同じ＝変えない） |
| 関数 `listed_value` | 同上 | 一覧に出す値（`name`、無ければ末尾の区切りなしのフルパス）。`get_active_ghost_list`・`get_log`・`get_property`・振り分けの記録の欄が使う |
| 定数 `NOT_ACTIVE`・`CANNOT_FIND` | 同上 | SSP と同じ本文 |

### 2.2 呼び手（変えずに済むことを確かめた）

- `crates/areka/src/mcp/mod.rs` の `drain` が要求ごとに `resolve::active(world)` を読み直し、`dispatch` のマクロ `resolved!` が `resolve::resolve(active, args.ghost_name.as_deref(), 扱い)` の結果をそのまま `NG:` か処理の呼び出しへ渡す。`get_expression_table` だけが `Omitted::Reject`、残る 7 本が `Omitted::UseActive`、`get_log` は解決を通らず自分の `handle` へ。→ `resolve` の中身を替えれば、`get_expression_table` の空文字の文言も `mod.rs` を触らずに `Cannot find` になる（要件 2.4・5.3）。
- `crates/areka/src/mcp/get_log.rs` の純粋な答え `answer` は、`ghost_name` が `Some("")` なら解決を呼ばずに `CANNOT_FIND`、それ以外は `Omitted::Reject` で解決し、失敗をすべて `CANNOT_FIND` に読み替え、成功なら `listed_value` で記録を絞る。→ 照合を直せば空白だけ・前後の空白・大小違い・本体側名が `get_log` にもそのまま効き、記録の絞り込みは名前そのもので指したときと同じになる（要件 1.5）。`Some("")` の腕は直した後は要らなくなるが、残しても答えは同じ。**ただしこの腕の注釈「解決は空を省略として扱う」と、`get_log_tests.rs` の注釈「解決へ渡すと 0 体で Specified ghost is not active になる」は、着地の後は事実と食い違う**（直すなら約束の外のファイルに触る）。
- 各ツールの処理（`get_status.rs`・`get_property.rs`・`dump_surface.rs` など）は `args.ghost_name` を読まない（`ghost_name` を読むのは `resolve.rs`・`mod.rs`・`get_log.rs` だけ＝grep で確かめた）。受け取った `ActiveGhost` は綴りに関係なく同じ値なので、答えの中身は綴りで変わらない（要件 4.4）。
- プロトコル側 `crates/areka-mcp/src/tools/mod.rs` の `optional_string` は `Value::as_str` で取るので、空文字は `Some("")` のまま、`null` と欄なしは `None` になる（`crates/areka-mcp/src/tools/tools_tests.rs` が `{"ghost_name": ""}` → `Some("")`・`{"ghost_name": null}` → `None` を固定）。`crates/areka-mcp/src/check.rs` の `check_arguments` は `ghost_name` を必須の検査から外している。→ 省略と空文字の区別はプロトコル側で既にできており、`crates/areka-mcp/` は触らずに済む（要件 3）。

### 2.3 本体側名と空白の在りか

- 構造体 `GhostNames`（`crates/areka-parsers/src/package/model.rs`・`#[non_exhaustive]`）が `name`・`sakura_name`・`sakura_name2`・`kero_name` を持つ。組むのは `crates/areka-parsers/src/package/resolve_shell.rs` で、descript の値をそのまま `map.get("sakura.name").cloned()` で入れる（値が空なら `Some("")`）。`GhostSession::names()`（`crates/areka/src/ghost_session.rs`）がそれを返す。→ `active` で `sakura_name` を取って空を落とせば済み、`ghost_session.rs` も parsers も触らない。
- descript の読み取り `parse_kv`（`crates/areka-parsers/src/kv/parse.rs`）は値を `str::trim` で前後の空白を除いて持つ。`str::trim` が除くのは Unicode の空白の性質を持つ文字（半角の空白・タブ・改行・全角の空白〔U+3000〕・ノーブレークスペースなど）。→ `ghost_name` を同じ `str::trim` で除けば、要件 2.1「descript の値を読むときに前後から除くのと同じ文字」に字義どおり一致する。
- 半角の英字だけの大小の畳みは標準の `str::eq_ignore_ascii_case` がそのもの（全角の英字・ギリシャ文字・かなは畳まない＝要件 1.3 と同じ）。リポジトリの中でも `emo2_boot/shell_balloon_resolve.rs` などが同じ関数を使っている。

### 2.4 テストの置き方と道具

- テストは本番ファイルと同じフォルダの兄弟ファイルに置き、本番側は `#[path = "..._tests.rs"]` の接続だけ（`structure.md` の Unit Tests）。1 ファイル 1,000 行の目安に対し、`resolve.rs` 93 行・`resolve_tests.rs` 177 行・`mcp_tests.rs` 356 行＝余裕がある。
- 本物の単位で通すリグ `SwitchRig`（`crates/areka/src/emo2_boot/ghost_switch_test_support.rs`）は検体の descript を `name,<フォルダ名>`・`sakura.name,<フォルダ名>のさくら` に書き替える。→ `mcp_tests.rs` の中で、`active` が本体側名を World から読めていること（`Aのさくら` で解決・`a` で大小違いの解決）を、約束の内側のファイルだけで檻に入れられる（既存の `real_unit_answers_get_active_ghost_list_in_one_frame` と同じ形）。

## 3. 要件と実物の対応（Requirement-to-Asset Map）

| 要件 | 使う実物 | ギャップ | 印 |
|---|---|---|---|
| 1.1 英字の大小 | `resolve` の名前の比べ方 | 完全一致を `eq_ignore_ascii_case` へ | Missing（小） |
| 1.2 本体側名・`name` 無しでも本体側名 | `active`・`ActiveGhost`・`GhostNames.sakura_name` | `ActiveGhost` に本体側名の持ち場が無い。足すと型の字面が外の 10 ファイル 12 か所で壊れる | Missing＋**Constraint（ウェーブの約束）** |
| 1.3 かな・全角・全角英字の大小は外れ | `eq_ignore_ascii_case` | 既定でそうなる。テストだけ要る | — |
| 1.4 `kero.name`・`sakura.name2` は外れ | `active` が取らない | 既定でそうなる。テストだけ要る | — |
| 1.5 `get_log` の絞り込み | `get_log.rs` の `answer`・`listed_value` | 変更不要（解決が直れば効く） | — |
| 2.1 名前の前後の空白を除く | `str::trim` | 名前との比べ方の前に `trim` | Missing（小） |
| 2.2 途中の空白は除かない | `str::trim` | 既定でそうなる | — |
| 2.3 パスの前後の空白は除かない | `same_path` | 渡すのを `trim` 前の値にするだけ | — |
| 2.4 空文字・空白だけは全ツールで `Cannot find`（0 体も） | `resolve` の最初の分かれ目 | `Some("")` を省略の腕から外す。空の綴りは名前（空を落としてある）にもパスにも当たらず自然に `CANNOT_FIND` | Missing（小） |
| 3.1〜3.3 省略は今のまま | `resolve` の `None` の腕・`get_log` の `None` の腕 | 変更不要 | — |
| 4.1〜4.4 変えないもの | `same_path`・`listed_value`・各処理 | 変更不要。テストは残す | — |
| 5.1 判断の決定論テスト | `resolve_tests.rs` | 欄を足した `ActiveGhost` の見本と、列挙された全場合のテストを足す | Missing |
| 5.2 古い期待の書き換え | `resolve_tests.rs` 5 本・`mcp_tests.rs` 1 本・**`get_log_tests.rs` 1 本** | `get_log_tests.rs` の 1 本は約束の外 | **Constraint** |
| 5.3 振り分けを通したテスト | `mcp_tests.rs` の `get_expression_table_omitted_is_not_active_even_with_one_ghost` | 空文字・空白だけを `CANNOT_FIND` 側へ分ける | Missing（小） |
| 5.4 一覧の値の往復 | `resolve_tests.rs` の `listed_value_resolves_back_to_the_same_ghost` | 残す（本体側名を持つ見本でも往復することを足すとよい） | — |
| 6.1〜6.3 実機確認 | 配布形・emo2・curl の `tools/call`（`mcp-get-property` の signoff と同じ道） | 手順と記録の置き場は設計で決める | Unknown（手順） |

## 4. 今の形を固定しているテストの数え（全数）

grep（`CANNOT_FIND`・`NOT_ACTIVE`・`Cannot find`・`is not active`・`Some("")`・`ActiveGhost {` を crates 全体）で数えた。

### 4.1 期待が変わる（赤になる）テスト＝7 本

| # | テスト | ファイル | 変わる所 | 約束の内か |
|---|---|---|---|---|
| 1 | `name_differing_only_in_case_does_not_resolve` | `resolve_tests.rs` | 大小違いが「外れ」→「解決」 | 内 |
| 2 | `sakura_name_does_not_resolve` | `resolve_tests.rs` | 本体側名が「外れ」→「解決」（今の見本は本体側名を持たないので、見本に本体側名を持たせて書き直す） | 内 |
| 3 | `empty_or_omitted_with_use_active_resolves_to_the_active_one` | `resolve_tests.rs` | `Some("")` の側が「解決」→`CANNOT_FIND` | 内 |
| 4 | `empty_or_omitted_with_reject_is_not_active` | `resolve_tests.rs` | `Some("")` の側が `NOT_ACTIVE`→`CANNOT_FIND` | 内 |
| 5 | `no_active_ghost_omitted_is_not_active` | `resolve_tests.rs` | 0 体の `Some("")` の側が `NOT_ACTIVE`→`CANNOT_FIND` | 内 |
| 6 | `get_expression_table_omitted_is_not_active_even_with_one_ghost` | `mcp_tests.rs` | `Some("")` の側が `NOT_ACTIVE`→`CANNOT_FIND` | 内 |
| 7 | `unmatched_empty_and_no_ghost_are_cannot_find` | **`get_log_tests.rs`** | 6 つの場合のうち `(Some(&g), "emily/phase4.5")` が「外れ」→「解決」になり、`assert_eq!` が赤。空文字の 2 つの場合は `get_log` の自前の腕で今も `Cannot find` なので緑のまま（注釈だけ古くなる） | **外**（要件の列挙から漏れていた） |

3〜5 はテスト名が「空と省略」をまとめているので、書き換えでは名前ごと分ける（要件 5.2「古い期待と新しい期待を両方残さない」）。

### 4.2 期待は変わらないが、`ActiveGhost` に欄を足すとコンパイルが落ちる字面＝16 か所

| ファイル | 字面の数 | 持ち主・並走 |
|---|---|---|
| `resolve.rs`（`active` の中） | 1 | 内 |
| `resolve_tests.rs`（`named`・`unnamed`・`listed_value_is_full_path_without_trailing_separator_when_name_absent`） | 3 | 内 |
| `mcp_tests.rs`（`ghost`） | 1 | 内 |
| `get_status_tests.rs` | 1 | **C4-⑥ `mcp-get-status` が触る** |
| `dump_surface_tests.rs` | 1 | **C4-⑧ `mcp-dump-images-residue` が触る**（`dump_surface*.rs`） |
| `dump_balloon_tests.rs` | 1 | **C4-⑧ が触る**（`dump_balloon*.rs`） |
| `get_expression_table_tests.rs` | 1 | 外（C4 に持ち主なし） |
| `get_property_tests.rs` | 1 | 外（同上） |
| `get_active_ghost_list_tests.rs` | 2 | 外（同上） |
| `get_log_tests.rs`（`emily`・`nameless_ghost_matches_records_named_by_its_full_path`） | 2 | 外（同上） |
| `sakurascript_tests.rs` | 1 | 外（`mcp-kanade-tools` は C5） |
| `raise_event_tests.rs` | 1 | 外（同上） |
| `reload_tests.rs` | 1 | 外（`mcp-reload` は C5） |

約束の外＝**10 ファイル・12 か所**。どれも `sakura_name: None,` を 1 行足すだけの機械的な変更だが、ファイルとしては触る。さらに、⑥ が `get_status_tests.rs` に、⑨ `mcp-author-tools` が新しいツールのテストに `ActiveGhost { name, root }` の字面を**新しく書く**と、git の取り込みは衝突なしで通ってもコンパイルが落ちる（字面の上で重ならない意味の衝突）。

### 4.3 変わらないことを確かめたもの

- `mcp_tests.rs` の `eight_with_no_active_ghost_are_not_active`（省略・0 体）・`eight_with_a_wrong_name_cannot_find`（`Someone else`）・`get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure`（省略）・`drain_resolves_from_the_world`（省略・0 体）は緑のまま。
- `resolve_tests.rs` のパスの 4 本・`no_active_ghost_with_name_cannot_find`・`reject_with_name_still_resolves`・一覧の値の 3 本・World からの読み取りの 3 本は緑のまま（字面の 3 か所だけ直す）。
- 他のツールのテストは `ghost_name` を `Some("Emily/Phase4.5")`（完全一致）か `None` で渡すだけ＝期待は変わらない。`get_expression_table_tests.rs` の本物の単位のテストは `Some("A")`（完全一致）。
- `crates/areka-mcp/` の `bridge_tests.rs` は `NG:Specified ghost is not active` を運ぶ道を試すだけで、照合とは無関係。
- `crates/areka/tests/mcp_get_log_real_run.rs` は `ghost_name` を使わない。

## 5. 実装の選択肢

### 選択肢 A: `ActiveGhost` に本体側名の欄を足す（既存の型を広げる）

- 変えるもの: `resolve.rs`（欄 `sakura_name: Option<String>`・`active` で `names()` の `sakura_name` を空を落として取る・`resolve` の比べ方）・`resolve_tests.rs`・`mcp_tests.rs`、**加えて外の 10 ファイル 12 か所に `sakura_name: None,` を 1 行ずつ**、`get_log_tests.rs` の 1 本の期待。
- 判断のコードの形（参考・決定ではない）: 省略（`None`）だけ今の腕へ。それ以外は `given.trim()` を `name`・`sakura_name` と `eq_ignore_ascii_case` で、`given`（`trim` 前）を `same_path` で比べ、どれにも当たらなければ `CANNOT_FIND`。名前は `active` で空を落としてあるので、空・空白だけは何にも当たらない。
- ✅ 型が「起動中のゴーストの照合に要る値」を全部持つ素直な形。後の多重起動（2 体以上）の spec もこの値を鍵に使える。
- ✅ 本番コードの変更は `resolve.rs` だけ。
- ❌ ウェーブの約束を破る（外の 10 ファイル）。うち 3 ファイルは ⑥・⑧ と字面が重なり、⑥・⑨ の新しい字面とは意味の衝突が起きうる＝着地の順の取り決めが要る。
- 変種 A′: 同時に `resolve.rs` にテスト用の作り手（例: 名前とルートから組む関数）を置き、外の字面をそれへ置き換える。次に欄が増えたときの波及は止まるが、今回触るファイルは同じで、差分は大きくなる。今の段では要らない（次に欄が増える spec が来たときに足せば足りる）。

### 選択肢 B: 型は変えず、`resolve` の引数で本体側名を別に渡す

- 変えるもの: `resolve` の引数（または `active` の戻り値を組に）と、その呼び手 `mod.rs` の `drain`・`dispatch`・`get_log.rs` の `handle`・`answer`。
- ✅ `ActiveGhost` の字面 16 か所は触らない。
- ❌ 本番のファイル `mod.rs`（C4-⑨ `mcp-author-tools` が触る）と `get_log.rs` を触る＝約束をより悪い形で破る。照合に要る値が型と引数に分かれ、読み手に優しくない。
- 評価: A より不利。挙げるだけ。

### 選択肢 C: 並べ方で解く（A の中身のまま、着地の順を決める）

- C-1: 本 spec を ⑥ `mcp-get-status`・⑧ `mcp-dump-images-residue` の着地の後へ回す（rebase してから外の字面を直す）。⑨ が新しい字面を書くなら、⑨ の後でもよい。
- C-2: 本 spec を先に着地させ、⑥・⑧・⑨ が rebase のときに自分の字面へ 1 行足す（コンパイルが落ちるので見落としは起きない）。
- ✅ コードの形は A のまま。手戻りは 1 行の追随だけ。
- ❌ ウェーブ C4 の「⑦〜⑨ は互いのファイルに触らない」を改める判断が要る（棚卸の約束の改め＝開発者の判断）。

### 選択肢 D: 本体側名（要件 1.2）だけを後へ送る

- 3 ファイル＋`get_log_tests.rs` の 1 本で、大小・空白・空文字の 3 点だけを直す。本体側名は追跡の spec へ。
- ✅ ウェーブの並走の約束（⑥・⑧・⑨ と字面が重ならない）は守れる。
- ❌ 要件 1.2・5.1 の一部と brief の Desired Outcome（4 点のずれを無くす）を削る。`get_log_tests.rs` には結局触る。小さな spec をさらに割ることになり、割るほどの規模の理由が無い。
- 評価: 要件を削る側の案。挙げるだけ。

### どの案でも残る点

- `get_log_tests.rs` の `unmatched_empty_and_no_ghost_are_cannot_find` の 1 か所（と注釈）は、要件 1.1 を満たすなら必ず直す。C4 で `get_log*.rs` を触る並走 spec は無い（`mcp-log-history` は完了・⑨ の触るファイルに無い）＝衝突はしないが、約束の 3 ファイルからは外れる。
- `get_log.rs` の `Some("")` の腕と注釈は、残しても答えは同じ。直すなら本番ファイルに触る＝今回は触らず、古い注釈が残ることを記録に残すか、触って直すかを決める。

## 6. 規模と危険

- **規模: S**（判断のコード 10 行前後・テストの書き換え 7 本＋新しい場合のテスト十数件・字面の 1 行の追随 12 か所・実機確認 1 回）。タスクは 3〜5 の見立てのまま。
- **危険: 低（論理）／中（調整）。** 論理は標準ライブラリ 2 つで決まり、既存の分かれ目の形にそのまま載る。危険は並走 spec との字面・意味の衝突と、SSP の未実測の細部（下の調べもの）に限られる。

## 7. 設計の段へ持ち越す調べもの（Research Needed）

1. **SSP の未実測の細部**（要件 6.3）: 全角の英字の大小（`ＡＢＣ` と `ａｂｃ`）・全角の空白とタブで前後を挟んだ名前・`sakura.name2`。本書の時点では SSP の MCP（9801）の応答を確かめていない。机で SSP が動けば、実機確認で同じ指定を当てて記録する。結果が要件の推奨案と違えば、要件の「要件の段で決めた細部」1・2・4 を改める。
2. **`ghost_name` に `null` を渡したときの SSP の答え**: areka はプロトコル側で `null` を省略（`None`）に読む。SSP が `null` を空文字と同じ「外れ」に扱うなら食い違う。本 spec の範囲外（プロトコル側の引数の扱い）だが、実機確認のついでに 1 行足して記録できる。
3. **`str::trim` と SSP の空白の範囲の差**: `str::trim` はノーブレークスペース・各種の幅の空白・行区切りまで除く。SSP が除く範囲が半角の空白だけなら、全角の空白・タブで要件 6.3 の答えが分かれる。1 と同じ実機確認で決まる。
4. **実機確認の手順と記録の置き場**: `mcp-get-property` の `verification/signoff.md` と同じ形（配布形・emo2・curl の `tools/call`）で足りる見込み。英字の大小の行は名前に半角の英字を含むゴースト（`えも2DEBUG` 相当）が要る（要件 6.2）＝emo2 の配布形の名前に半角の英字が入っているかを設計で確かめる。

## 8. 設計の判断が要る項目（要件ディスカッションへ）

1. **ウェーブ C4 の約束を改めるか**: 本体側名を持たせるには、`ActiveGhost` の字面を約束の外の 10 ファイル 12 か所で直す必要がある（うち 3 ファイルは ⑥・⑧ と重なる）。選択肢 A＋C（どちらが先に着地し、後の側が 1 行追随するか）・B・D のどれを取るか。要件の Boundary Context は「破る必要が出たら止めて報告する」と書いている＝ここが止める所。
2. **`get_log_tests.rs` の 1 本を書き換えの対象に足すか**: 要件 1.1 を満たすと `unmatched_empty_and_no_ghost_are_cannot_find` の大小違いの場合が必ず赤になる。要件の「今の形を固定しているテスト」の列挙と Boundary Context（触るファイル）に `get_log_tests.rs` を足す改めが要る（並走の衝突は無い）。
3. **`get_log.rs` の空文字の腕と古くなる注釈を直すか**: 答えは変わらないので残しても要件は満たす。触れば本番ファイルが約束の外に 1 つ増える。触らないなら「注釈が事実と食い違う」ことを記録に残す。
4. **本体側名が空（`sakura.name,` だけ）のときの扱い**: `name` と同じく空を「無い」とみなす（空文字の `ghost_name` は先に外れるので答えは変わらないが、型の上の約束として揃える）ことを設計で明記するか。
5. **未実測の細部（全角の英字の大小・全角の空白とタブ・`sakura.name2`）を SSP で確かめられなかったときの扱い**: 推奨案（要件の表）のまま着地させ、追試を記録の残り物とするか、着地の前に SSP での追試を条件にするか。
6. **実機確認で `null` の行も当てるか**: 範囲外だが、同じ curl の手順で 1 行足せる。食い違いが出たら起票（`/kiro-discovery`）する前提で記録だけ取るか。

## 9. 次の一歩

- 要件ディスカッション（`/kiro-requirements-discussion areka-P0-mcp-ghost-name-match`）で上の 1・2 を先に決める（触るファイルの範囲が決まらないと設計が書けない）。
- 決まったら `/kiro-design areka-P0-mcp-ghost-name-match` で設計へ。
