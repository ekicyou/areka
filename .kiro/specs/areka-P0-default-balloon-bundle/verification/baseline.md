# 着手前の基準値（タスク 1.1）

採取日: 2026-09-18。採取した作業木: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-balloon-bundle-6fe55e`（ブランチ `claude/areka-p0-balloon-bundle-6fe55e`）。

この記録は要件 8.4（「着手前と同じ本数で緑」）と設計 C8（非回帰の検査）の突き合わせ相手である。後段のタスク 6・2.8・5.1 はここの数値を基準に使う。各値には採り方（実際に打った命令）を添えた。

## 1. 基準コミット

| 項目 | 値 |
|---|---|
| 基準コミット | `082379b3003f3e2096c574585d15b194b369e81e`（`docs(areka-P0-default-balloon-bundle): generate tasks (tasks.md)`） |
| 採取時点の HEAD | `0abc746ea08b014fc8d4e19d451ada22c689a718`（タスク 1.2 の資産保管が着地済み） |

採り方: `git rev-parse HEAD`。基準コミットは**本仕様の実装着手前（タスク生成の直後）の HEAD** であり、その時点の作業木は完全にクリーンだった。タスク 1.1 の計測はタスク 1.2 の着地後に行ったので、計測時点の HEAD は `0abc746e` である（§2.3 参照）。基準を `0abc746e` に取らないのは、そうするとタスク 1.2 が保管した 29 ファイルが差分から消え、要件 2.7／8.5 の検査が保管フォルダに対して盲になるため。

### 非回帰の差分がこの基準から採れることの確認

```
git diff --stat 082379b3003f3e2096c574585d15b194b369e81e..HEAD
```

→ 30 ファイル・227 行追加・1 行削除。内訳は `vendors/sample_ghost/StayseeBalloon/` の新規 29 ファイル（タスク 1.2）と `.kiro/specs/.../tasks.md` の 1 行（チェック）だけ。

設計 C8 が要求する各 pathspec を基準コミットから引いた結果:

| 検査（設計 C8） | 命令 | 結果 |
|---|---|---|
| 本番コード 0 行（8.1, 6.6, 7.2） | `git diff --stat <base>..HEAD -- 'crates/*/src'` | 出力なし＝0 |
| 依存記述とロック 0（8.2） | `git diff --stat <base>..HEAD -- '**/Cargo.toml' Cargo.toml Cargo.lock` | 出力なし＝0 |
| 作業木の未コミット（8.5） | `git status --porcelain` | **この記録を置く前**の値: ` M .kiro/specs/areka-P0-default-balloon-bundle/tasks.md` の 1 行のみ。本ファイルを置いた後は `?? .kiro/specs/areka-P0-default-balloon-bundle/verification/` が増えて 2 行になる（印字した数はその場で古びるので、タスク 6 は必ず自分で採り直すこと） |

基準コミットは実在し、そこからの差分が機械で採れることを確認した。

## 2. 着手前のテスト本数

### 2.1 前提（i686 の成果物）

`cargo test --workspace` は i686 の成果物を 2 つ要求する。片方だけでは通らない。

| 成果物 | 置き場所 | 採り方 |
|---|---|---|
| host-32 helper | `target/debug/shiori-host32-helper.exe` | `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` → `target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe` を `target/debug/` へ上書き複写 |
| host-32 testdll | `target/i686-pc-windows-msvc/debug/shiori.dll` | `cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc`（PowerShell で実行。exit 0・154,624 バイト） |

ワークスペースのビルドは `target/debug/shiori-host32-helper.exe` を x64 版で上書きするので、**テストを走らせる直前に置き直す**必要がある。今回は次の順で行った。

1. `cargo build --workspace --tests -j 4`（exit 0）
2. helper を i686 版で上書き複写
3. `cargo test --workspace -j 4 --no-fail-fast`

走行の後に PE ヘッダの Machine 欄を読み直し、`target/debug/shiori-host32-helper.exe` が `0x014C`（i686）のままであることを確認した（x64 は `0x8664`）。すなわち走行中ずっと i686 版が置かれていた。

### 2.2 本数（この値がタスク 6 の突き合わせ相手）

命令: `cargo test --workspace -j 4 --no-fail-fast`（他の cargo を 1 つも並走させずに単独で走らせた。壁時計デッドラインを持つ既存テストが飢餓しないようにするため）

**タスク 6 も同じ旗で採ること**（`-j 4 --no-fail-fast`）。理由:

- `--no-fail-fast` を外すと `cargo test` は最初の赤でその場で打ち切る。**全緑のときだけ**素の `cargo test --workspace` と同本数になり、赤が 1 件でも出ると母数が縮んで「着手前と同じ本数」の比較そのものが成り立たなくなる（今回、親の 1 回目の走行がまさにこれで 81→60 ターゲット・7,668→5,411 本に縮んだ）。旗をそろえておけば、赤が出ても母数が保たれて差が読める。
- `-j 4` は `cargo test --workspace` が E0786「invalid metadata」で落ちるのを避けるため。この症状の実体はページングファイル不足（os error 1455）で、並列度を落とすと通る。

| 数え方 | 値 |
|---|---|
| 走行の exit code | 0 |
| `Running` 行の本数（`^\s*Running ` の件数） | **81** |
| `Doc-tests` 行の本数 | **22** |
| `test result:` 行の本数 | **103**（＝81＋22。取りこぼし 0 の裏取り） |
| `passed` の合計 | **7,668** |
| `failed` の合計 | **0** |
| `ignored` の合計 | **40** |
| `filtered out` の合計 | **0** |

内訳（`Running` 側と `Doc-tests` 側に分けて数え直したもの）:

| 区分 | ターゲット数 | passed | ignored |
|---|---|---|---|
| `Running`（単体・結合） | 81 | 7,647 | 13 |
| `Doc-tests` | 22 | 21 | 27 |
| 合計 | 103 | 7,668 | 40 |

数え方（すべて走行の全文に対して掛けた。`Select-Object -First N` や `| tail` は上流の cargo を止めて部分的な緑を全体の緑に見せるので使っていない）:

```
grep -cE '^[[:space:]]*Running ' <全文>
grep -cE '^[[:space:]]*Doc-tests ' <全文>
grep -c 'test result:' <全文>
grep -o '[0-9]\+ passed'  <全文> | awk '{s+=$1} END{print s}'
grep -o '[0-9]\+ failed'  <全文> | awk '{s+=$1} END{print s}'
grep -o '[0-9]\+ ignored' <全文> | awk '{s+=$1} END{print s}'
```

赤は 0 件（`grep -n 'FAILED\|^error: test failed'` が無出力）。

走行の全文は `C:\Users\maz-o\AppData\Local\Temp\claude\C--home-maz-git-areka--claude-worktrees-areka-p0-balloon-bundle-6fe55e\24d31e9a-0d49-4a53-a1e0-89993adfab8c\scratchpad\baseline2-workspace-test.txt`（一時領域なので消えうる。上の数値が正本）。

### 2.3 この時点の HEAD を「着手前の本数」として使ってよい理由

計測した HEAD（`0abc746e`）は基準コミットよりタスク 1.2 の分だけ先行しているが、そのタスクが足したのは `vendors/sample_ghost/StayseeBalloon/` の資産 29 ファイルだけである。

```
grep -rn "sample_ghost" --include=*.rs crates/
```

→ 無出力（exit 1）。Rust のコードからこの資産を参照している行は 1 本も無い。ビルド対象にもテスト対象にもならないので、この資産追加はワークスペースのテスト本数を 1 本も動かさない。よって 7,668 本を「着手前の本数」として扱う。

### 2.4 最初の走行が赤だったこと（申し送り）

同じ命令を最初に走らせたときは `shiori-host32-host` の `lifecycle_cyclic_e2e::cyclic_run_and_clean_shutdown` が赤になり、60 ターゲット・5,411 本の時点で走行が打ち切られた（`cargo test` は既定で最初に落ちたターゲットで止まる）。原因は i686 の **testdll**（`shiori-host32-testdll`）が未ビルドだったことで、helper だけを置いても足りない。間欠の赤ではなく、前提の取りこぼしである。

タスク 6 で同じ命令を走らせるときは、§2.1 の 2 つの成果物を**両方**そろえてから走らせること。そろえなければ 5,411 本で打ち切られ、本数の突き合わせが成立しない。

## 3. 台帳側の現在値

### 3.1 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml`

採り方: 各 `[entry."..."]` ブロックを `awk` で切り出して目視。

| 項目 id | `status` | `owner` | `priority` |
|---|---|---|---|
| `ukadoc:descript_balloon:use_self_alpha_2c_5024:1`（本仕様が触る） | `absent` | `""`（空） | `A15` |
| `ukadoc:descript_balloon:use_input_alpha_2c_6570_5024:1`（触らない） | `absent` | `""`（空） | `A15` |
| `ukadoc:descript_balloon:paint_transparent_region_black_2c_6570_5024:1`（触らない） | `absent` | `""`（空） | `A15` |

3 項目とも `values = ["装い"]`・`links = []`。`introduced` は順に `""`・`"2.5.40"`・`""`。

本仕様が触る項目の `note` の現行文（要件 6.2 で書き換える対象）は、次の 10 行から成る:

1. `壊れ方: 黙って壊れる。記録: なし。`
2. `areka はバルーン側の透過の扱いを宣言で切り替えられない。`
3. バルーンの定義を読むのは `balloon::read_descript_layer` と `placement::load_balloon_author_dpi` の 2 つだけで、`balloon::map_merged` は名前を数え上げた欄しか引かない、という説明
4. `areka-emo-atlas` の `normalize::UseSelfAlpha` は実在するが値は呼ぶ側の決め打ち、という説明
5. シェルの `seriko.use_self_alpha` は綴りの違う別項目、という注意
6. 両方の表のすべての行を当たったが名指しで引き受ける行は無い、という記述
7. `areka-P0-balloon-parse` の brief の範囲外、という記述
8. 担当 spec は `areka-P0-emo-atlas`、という記述
9. 束「透過とはみ出しの描き方」・束の順位 9、という記述
10. 宛先を空にした理由（`areka-P0-emo-atlas` が完了して封じられているため）

タスク 6 は隣の 2 項目の行が差分に現れないことを確かめる。上の表の 2 行目・3 行目が「変わっていないこと」の突き合わせ相手である。

### 3.2 ロードマップ草案 `doc/ukadoc-coverage/roadmap-draft.md`

| 値 | 現在値 | 採り方 |
|---|---|---|
| `[briefs].count` | **27** | `grep -n -A3 '^\[briefs\]'`（`snapshot_on = "2026-09-13"`） |
| `[[spec]]` 行の実数 | **27** | `grep -c '^\[\[spec\]\]'` |

表示されている数（27）と自分で数え直した実数（27）は一致している。要件 6.4 と設計 C5 は両方を 28 にする。

### 3.3 ブリーフィング `doc/ukadoc-coverage/briefing.md`

`[[barrier]] page = "descript_balloon"` の現在値（採り方: `grep -n -A12 'page = "descript_balloon"'`）:

| 欄 | 現在値 |
|---|---|
| `implemented` | 24 |
| `vocabulary_only` | 9 |
| **`degraded`** | **6** |
| **`absent`** | **123** |
| `alias` | 0 |
| `not_applicable` | 0 |

要件 6.5 と設計 C5 が動かすのは太字の 2 数値だけ（`degraded` 6→7・`absent` 123→122）。他の 4 数値は触らない。

### 3.4 `doc/COMPAT_ARCHITECTURE.md` §8「沈黙ルール対応表」

| 値 | 現在値 |
|---|---|
| 表の位置 | 本文 126 行目（見出し行）〜241 行目 |
| 縦棒で始まる行の総数 | 116 |
| 見出し行＋区切り行 | 2 |
| **データ行（裁量の登記）** | **114** |

採り方: `awk 'NR>=122 && NR<=245 && /^\|/'` で範囲と件数を採り、同じ範囲に縦棒で始まらない行が 1 本も無いこと（表が途切れていないこと）を確かめた。§8 にはこの表のあと `### areka 裁量の性能目標` という別の節が続くが、そこは別の記述で、この表には含まれない。

タスク 3.1 は末尾に 1 行足す。したがって着地後のデータ行は 115 行になる。

## 4. 完了状態の確認

| タスク 1.1 の完了条件 | 記録した場所 |
|---|---|
| 着手前のテスト本数 | §2.2（`Running` 81・`Doc-tests` 22・passed 7,668・failed 0・ignored 40） |
| 基準コミット | §1（`082379b3`） |
| 台帳側の 4 値 | §3.1（台帳 3 項目）・§3.2（草案 2 数値）・§3.3（ブリーフィング 2 数値）・§3.4（§8 の行数 114） |
| 後段の非回帰検査が突き合わせられること | §1 の 3 本の差分検査が基準コミットから実際に採れることを実測で確認済み |
