# 検証の記録: audit-tools-scripts

## タスク 8: 絵の道具 2 本の「コミット済みの出力は最新」の判定

### 比べ直しの結果（判定を足す前）

`tools/app-icon.ps1 -SelfTest` と `tools/cursor-art.ps1 -SelfTest` が `target\` の下（`app-icon-selftest\case1`・`cursor-art-selftest\case1`）に組んだ出力を、コミット済みのものとバイトで比べた（python）。5 ファイルとも同じ。

| コミット済みのファイル | バイト数 | 組んだ結果との比較 | SHA-256（先頭 16 桁） |
|---|---|---|---|
| `crates/areka/icon/app.ico` | 23315 | 同じ | `480111faaab7861f` |
| `crates/areka/icon/tray.ico` | 17773 | 同じ | `29e5a9bde8c3c50e` |
| `crates/areka/icon/icons.res` | 41684 | 同じ | `1403d0a304d0ba4a` |
| `crates/areka/cursor/hand.cur` | 236790 | 同じ | `59a0e6fb8e7cbe39` |
| `crates/areka/cursor/grip.cur` | 236790 | 同じ | `989829219576878e` |

### 判定の追加

両方の `-SelfTest` に `committed outputs are current` を足した（`same bytes twice` で組んだ結果を使い、組む回数は増やしていない）。2 本とも緑（`ok selftest committed outputs are current`・`OK selftest (3 checks)`）。

### 落ちることの確かめ

道具の台本（2 本と `tool-common.ps1`・`utf8-child.ps1`）を `target\tmp-t8\tools\` へ、`icon\`・`cursor\` を `target\tmp-t8\crates\areka\` へ写し、写しの `app.ico`・`grip.cur` の末尾 1 バイトを違えて写しの道具で `-SelfTest` を回した。どちらも落ちた（写しは確かめた後に消した）。

- `FAIL selftest committed outputs are current: app.ico (source changed or rendering changed; run tools/app-icon.ps1 and commit)`
- `FAIL selftest committed outputs are current: grip.cur (source changed or rendering changed; run tools/cursor-art.ps1 and commit)`

`crates/areka/icon/` と `crates/areka/cursor/` は書き替えていない（`git status` は `tools/` の 2 本と本記録だけ）。

## タスク 11.1: `runtime-crate-split` の着地の確認と負荷の再現の道具の選び肢

確認日: 2026-11-17

確認のコマンドと結果（読み取りだけ）:

- `git fetch` のあと `git rev-parse origin/main` は `5b3a361c95c5fb494fdf7845576cf20f1aa20c07`
- `git ls-tree -d origin/main .kiro/specs/completed/runtime-crate-split` は何も返さない（終了コード 0・出力なし）

結論: `runtime-crate-split` はまだ `main` に着地していない。

採った道: 「着地していない」の道。`tools/load-flake.ps1` に `areka-runtime-lib` の選び肢は足さない（コードの変更なし）。`runtime-crate-split` の brief には何も書かない。`-SelfTest` は変更なしで緑のまま。

引き継ぎ: `runtime-crate-split` の完了時に、そのセッションが拾う。渡す文は次のとおり。

> `runtime-crate-split` の着地後、`tools/load-flake.ps1` に `-Binary areka-runtime-lib`（`-p areka-runtime --lib`）の選び肢を足してください。あわせて使い方の例の `-Filter runtime::frame` を新しいクレートの形に直し、`-SelfTest` の `cargo args` の期待を 3 対象から 4 対象へ直してください（`audit-tools-scripts` 11.1 の申し送り）。

## タスク 11.2: 同じ範囲への audit のかけ直し

### 作法（要件 1 と同じ）

- 範囲: `tools/` のうち `tools/perf/` の外の全部（足した 4 本 `tool-common.ps1`・`exe-install.ps1`・`exe-install.selftest.ps1`・`package.selftest.ps1` を含む）。
- 測り直し（1 回目の前・ブランチ `claude/kiro-start-audit-tools-scripts-bdec39` の `219f47a45`）: 25 ファイル 5,195 行＝本番の台本 16 本 4,652 行・自分の検査の台本 2 本 406 行（`exe-install.selftest.ps1` 174・`package.selftest.ps1` 232）・見本 6 本 126 行・文言 1 本 11 行。
- 物差し: ponytail 5.3.3（`~/.claude/plugins/installed_plugins.json` の `ponytail@ponytail`）の `skills/ponytail-audit/SKILL.md`。「最大 20 件」は外して全件を出した。
- 1・2・3・5・6・7 回目は、その時点の 25 ファイルを 1 行目から最後の行まで全部読んだ（読まなかった所は無い。1〜3 回目は当時の記録のとおり、5〜7 回目は 1 つのセッションで 3 回とも全ファイルを読み直した）。4 回目だけは直しの差分を読んだだけで、この作法を満たさない（下の「回数と各回の指摘の数」）。
- 見立てた使われ方: 開発者の 1 台の Windows の机と、GitHub の Windows の走らせ手から、1 人が 1 本ずつ回す。`git`・`cargo`・`rustup`・`pwsh` は PATH に在り、リポジトリと置き場はふつうのパスに在る。要る命令が PATH に無いときは、0 でない終了コードと命令の名前の入った文で止まれば指摘にしない（緑になる・台本が自分で用意した断りの文へ届かない、のどちらかなら指摘にする）。
- 範囲の中の `shortcut:`／`ponytail:` の注釈は `crates-io.ps1` の 1 件（索引の置き場は 4 文字以上の名前の規則だけ）のまま。公開の一覧の 3 本はどれも 4 文字以上＝限界を越えていないので指摘にしない（1 回目のギャップ分析と同じ扱い）。
- 旗: (a) 外への約束を変えるか・(b) 要件 8.1 の操作でしか確かめられないか。

### 回数と各回の指摘の数

| 回 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---:|---:|---:|---:|
| 1 回目 | 1 | 1 | 3 | 5 |
| 2 回目 | 0 | 0 | 0 | 0 |
| 3 回目 | 0 | 0 | 1 | 1 |
| 4 回目（差分だけ・全部は読んでいない） | 0 | 0 | 0 | 0（作法を満たさないので、最後の 0 件に数えない） |
| 5 回目 | 1 | 0 | 0 | 1 |
| 6 回目 | 1 | 0 | 0 | 1 |
| 7 回目 | 0 | 0 | 0 | **0** |

かけ直しは 7 回目で 0 件に収まった。最後の 0 件は、25 ファイルを全部読んだ 7 回目の結果である。

- 要件 9.6 の打ち切りは使っていない: 5 回目の指摘は「必ず直す」で道具の振る舞いに関わるので、打ち切りの条件（5 回目に「必ず直す」と振る舞いに関わる指摘が 1 件も無い）に当たらず、直して 6 回目・7 回目へ進んだ。

- 1 回目の 5 件目（`test-all.ps1` の `Step`）は、1 回目と 2 回目では「指摘にしなかったもの」に入れていた。タスクのレビューが、`cargo` が PATH に無いと `all steps green` になることを再現して差し戻したので、1 回目の指摘へ戻して直した（2 回目の 0 件は、この取り違えを含んだ数）。
- 3 回目は、1 回目の 5 件目の直しの差分（`test-all.ps1`・`encoding-check.ps1`）を含めて範囲の 25 ファイルを全部読み直した（測り直し: 25 ファイル 5,246 行）。読まなかった所は無い。
- 4 回目は、3 回目の直しの差分（`release-notes.ps1` の判定 1 本と頭書の 1 行）を読み、範囲の残りは 3 回目から変わっていないことを `git diff` で確かめただけだった（測り直し: 25 ファイル 5,252 行）。タスクのレビューが「全部を読んでいないので、要件 9.1・9.3 の『要件 1 と同じ作法でかけ直した結果の 0 件』にならない」と差し戻した。4 回目の 0 件は「直しの差分が生んだ指摘は 0 件」の意味にとどまる。
- 5 回目は、25 ファイルを全部読み直した（測り直し: 25 ファイル 5,252 行・4 回目から変わりなし）。指摘 1 件（`Get-MsvcLinkPath`）。
- 6 回目は、5 回目の直しの後の 25 ファイルを全部読み直した（測り直し: 25 ファイル 5,264 行）。5 回目の直しが生んだ指摘は 0 件。5 回目に読んで「要る命令が PATH に無い場面」として指摘にしなかった `package.ps1` の `git` の読みを、読み直して指摘へ改めた（台本が自分で用意した断りの文へ届かないため）＝指摘 1 件。
- 7 回目は、6 回目の直しの後の 25 ファイルを全部読み直した（測り直し: 25 ファイル 5,274 行）。指摘 0 件。

### 1 回目の指摘と直し

1. **直したほうがよい: 検体の取り寄せの本番の行で https だけにする配線を縛る検査が無い**（`sample-fetch.ps1` の `Invoke-FetchRows`・`-HttpsOnly:(-not $AnyScheme)`）
   - 困る場面: この配線を落としても、7.2 で足した 2 つの検査（`Sync-FetchedSample`・`Get-CurlArguments` を直に呼ぶ）と見本の 10 判定はすべて緑のまま。本番の行は転送の先が `http://` でも取り寄せる形へ黙って戻る（tasks.md の Implementation Notes の 7.2 の件）。
   - 直し: `-SelfTest` に判定 `11 production rows fetch with https only` を足した。子の scope に `Sync-FetchedSample` の代役を置き（PowerShell の動的な scope で `Invoke-FetchRows` から見える）、`https://` の行を `-AnyScheme` なしで、`file:///` の行を `-AnyScheme` 付きで `Invoke-FetchRows` に通し、代役が受けた `-HttpsOnly` が `True,False` であることを見る。curl もネットも使わない。頭書の「10 checks」を「11 checks」へ。
   - 検査: 全体テストの段 `sample fetch check`（`tools/sample-fetch.ps1 -SelfTest`）から回る。配線を外した写し（`target\tmp-r1\tools\`・確かめた後に消した）では `FAIL 11 production rows fetch with https only` と終了コード 1、本物では緑。
   - 旗: (a) なし（`-SelfTest` の判定が 1 本増えるだけ。読み手の全体テストは終了コードだけを見る） ／ (b) いいえ
2. **できれば: `encoding-check.ps1` の頭書の規則 E の説明が表と違う**（`encoding-check.ps1:32`）
   - 困る場面: 説明は `tools/package.ps1 (metadata and nar-sample-path)` と書くが、表 `$RequiredCalls` は `nar-sample-path` だけ（版の読みは 1.2 で `tool-common.ps1` へ移った）。読み手は規則 E が `package.ps1` の `metadata` を縛ると読み違える（tasks.md の Implementation Notes の 2.2 の件）。
   - 直し: 説明を `tools/package.ps1 (nar-sample-path)` にした（ASCII のまま）。
   - 検査: 文の直しで分かれ道を足さないので足さない。`encoding check` が緑（規則 A は頭書のコメントを見ない）。
   - 旗: (a) なし ／ (b) いいえ
3. **できれば: `load-flake.ps1` の `Say` の説明が古い**（`load-flake.ps1:103`）
   - 困る場面: 説明は「ASCII 以外は ? に置き換える」と書くが、1.2 の寄せの後は `Show-Ascii … 0` で `\uXXXX` を出す（直しが生んだずれ）。
   - 直し: 説明を「ASCII 以外は \uXXXX にする」にした。
   - 検査: コメントだけの直しなので足さない。
   - 旗: (a) なし ／ (b) いいえ
4. **できれば: `release-notes.ps1` の頭書の自分の検査の一覧が、3.2 で足した判定を挙げていない**（`release-notes.ps1:39-48`）
   - 困る場面: 一覧に、spec-name の壊した見本の 2 通り・語の一部だけの名前が通ること・本物の一覧が `.kiro/specs/` と completed の名前を含むこと（`spec names cover active and completed`）が無い。後の判定は本物の spec のフォルダに頼るので、読み手がそれを知らずにフォルダを動かすと赤の訳が分からない。
   - 直し: 一覧の 1 行に「spec-name は一覧の名前と印の 2 通り」を足し、1 行「spec 名を語の一部にだけ含む文は通る／本物の spec 名の一覧が .kiro/specs/ と completed の名前を含む」を足した。
   - 検査: 説明の直しなので足さない。`release-notes.ps1 -SelfTest` が緑。
   - 旗: (a) なし ／ (b) いいえ

5. **必ず直す: 全体テストの段の命令が見つからないと、前の段の終了コードで緑に記録される**（`test-all.ps1` の `Step`）
   - 困る場面: `test-all.ps1` は `$ErrorActionPreference = 'Stop'` の下に無く、見つからない命令（CommandNotFound）は止まらずに続く。命令が `$LASTEXITCODE` を書かないので、`Step` は前の段の値を記録する。`sample fetch` が緑の後に `cargo`・`rustup` が PATH に無いと、Rust の段が全部 `OK` になり `all steps green` を出す。後で足す段の命令を綴り違えても緑のまま（完了の門が嘘の緑を出す）。
   - 直し: `Step` の `& $Command` の直前に `$global:LASTEXITCODE = -1` を置いた（0 だと見つからない命令が緑・`$null` だと結果の一覧が SKIP と読む）。`TestStep` は `Step` を呼ぶので同じ道を通る。外部の命令は終了コードを書くので、ふつうの段の結果の行と終了コードは変わらない（見つからない命令の段は `FAIL … exit code -1`）。`tools/package.ps1` の `Step` は前から `Stop` の下で終了コードを戻している。
   - 検査: `encoding-check.ps1` に規則 F を足した（全体テストの段 `encoding check` から回る）。`test-all.ps1` の関数 `Step` を構文木から取り出し、`test-all.ps1` と同じ形（StrictMode なし・失敗は続く）で、緑の段（`$global:LASTEXITCODE = 0`）の後に見つからない命令の段を回して、0 と 0 でない値が記録されることを見る。較正の見本 3 つ（戻さない `Step`＝赤・`Step` が無い＝赤・戻す `Step`＝緑）を毎回回す。直す前の `test-all.ps1` では `FAIL tools/test-all.ps1: F after a green step, a step whose command is not found was recorded as exit code 0` と終了コード 1。戻しの行を外した写し（`target\tmp-r2\`・確かめた後に消した）を同じ関数に通しても同じ文で赤、直した本物は緑。`test-all.ps1` の頭書の段 9 に規則 F を見ることを書き足した。
   - 旗: (a) なし（見つからない命令の段が赤になるだけ。読み手の `/kiro-complete` は `all steps green` と終了コードだけを見る） ／ (b) いいえ

外への約束を変える指摘と、要件 8.1 の操作でしか確かめられない指摘は 0 件（進行役へ返すものは無い）。

### 3 回目の指摘と直し

1. **できれば: `release-notes.ps1` の判定 `spec names cover active and completed` が、この spec を completed へ移すと前半が空振りする**（`release-notes.ps1` の `-SelfTest`）
   - 困る場面: 判定は決め打ちの `audit-tools-scripts` が一覧に在るかで「`.kiro/specs/` の直下を読む」ことを見ていた。`/kiro-complete` でこの spec が `.kiro/specs/completed/` へ移ると、`$SPECS_DIRS` から `.kiro/specs` が落ちても判定は通る（読み手を completed だけにした形で確かめた: 前の判定は真のまま）。
   - 直し: 名前を決め打ちせず、`.kiro/specs/` の直下にだけ在る名前（completed に同じフォルダが無いもの・頭の印を外した形）の全部と、completed の旧名 `actor-foundation` が一覧に在ることを見る（直下にだけ在る spec が 1 つも無ければ赤）。頭書の一覧の文と `design.md` の検査の表の行を同じに改めた。
   - 検査: `release-notes.ps1 -SelfTest`（全体テストの段 `release notes check`）。`$SPECS_DIRS` を completed だけにした写し（`target\tmp-r3\`・spec のフォルダの名前だけを空のフォルダで写した・確かめた後に消した）で `FAIL selftest spec names cover active and completed`、読み手を戻してこの spec のフォルダを completed へ移した写しでは緑、本物では緑。
   - 旗: (a) なし（自分の検査の判定の中身だけ） ／ (b) いいえ

### 5 回目の指摘と直し

1. **必ず直す: PATH にドライブの無いフォルダが残っていると、`link.exe` の手当てが投げる**（`tool-common.ps1` の `Get-MsvcLinkPath`）
   - 困る場面: PATH に `Q:\tools` のような、今は無いドライブのフォルダが残っている机（外した USB・切れたネットワークのドライブ）。`Join-Path` は無いドライブで投げるので、`$ErrorActionPreference = 'Stop'` の `package.ps1` と `crates-io.ps1 -Verify` は、ビルドの前に訳された例外の文で終了コード 1 になる（道具の入力と関わりの無い PATH の項目のせいで回せない）。`Stop` でない `test-all.ps1` は ASCII でない誤りの行を 2 行出して続く。この机で確かめた: `Get-MsvcLinkPath 'C:\Windows;Q:\nodrive\bin;C:\Windows\System32'` が `Stop` の下で投げ、`Continue` の下で誤りの行 2 つと元の文字列を返した。
   - 直し: `Join-Path` をやめ、`Test-Path -LiteralPath "$_\link.exe"`・`"$_\cl.exe"` にした（無いドライブ・引用符や `<` `|` の入った項目でも投げずに偽を返すことを確かめた）。外すフォルダの決まり（`link.exe` を持つが `cl.exe` を持たない）は変えていない。この机の本物の PATH で、直す前の式と直した関数が同じ文字列を返す。
   - 検査: `package.selftest.ps1` に判定 `msvc link path` を足した（全体テストの段 `package check`）。`target\package-selftest\<pid>\` の下に偽のフォルダ 3 つ（`link.exe` だけ・`link.exe` と `cl.exe`・空）を作り、空の項目と、この机に無いドライブのフォルダを足した文字列を渡して、`link.exe` だけのフォルダが外れ、ほかが順のまま残ることを見る。直す前は `FAIL selftest msvc link path: <ドライブが見つからない旨の文を ASCII にしたもの>` と終了コード 1、直した後は緑。
   - 旗: (a) なし（投げていた場面が通るだけ。ふつうの PATH では結果の文字列が同じ） ／ (b) いいえ

### 6 回目の指摘と直し

1. **必ず直す: `git` が PATH に無いと、配布物の道具が用意した断り（終了コード 3 と `git does not run`）へ届かない**（`package.ps1` の preflight）
   - 困る場面: 頭書は「3 … 引数・前提の不正（道具が無いときも。何を入れるかを 1 行で印字する）」と言い、preflight は `git does not run (install git and add it to PATH)` の行を持つ。ところが `git` が PATH に無いと、`Stop` の下では `git rev-parse` の行が例外を投げ、その断りの行に届く前に段の失敗になる: 終了コード 1 と `FAIL preflight (0 s, <命令が見つからない旨の訳された文を ASCII にしたもの>)`。「git を入れよ」の文は、git が入っていないときに 1 度も出ない。小さな写しで確かめた（`Stop` の `try` の中の見つからない命令は `CommandNotFoundException` で捕まる）。5 回目は「要る命令が PATH に無い場面」として指摘にしなかったが、台本が自分で用意した断りへ届かないので 6 回目に指摘へ改めた。
   - 直し: HEAD の読みを関数 `Get-HeadCommit`（`try { git rev-parse --short=7 HEAD 2>$null } catch { }`）へ出し、preflight はその戻りが空なら今の行で 3 にする。終了コードの意味と文は変えていない。この机で、関数の戻りが `git rev-parse --short=7 HEAD` と同じ 7 桁であることを確かめた。
   - 検査: `package.selftest.ps1` に判定 `head commit without git` を足した（全体テストの段 `package check`）。`PATH` を空にして（`finally` で戻す）`Get-HeadCommit` を呼び、投げずに空が返ることを見る。子は起こさない。`try` を外した形では `FAIL selftest head commit without git: <命令が見つからない旨の文を ASCII にしたもの>` と終了コード 1、直した後は緑。preflight がこの関数を呼ぶ 1 行は判定の外（配線）。
   - 旗: (a) なし（頭書と設計の終了コードの意味のとおりになる。`git` が PATH に無いときの終了コードが 1 から、頭書の言う 3 になる。読み手のリリースの workflow と全体テストは 0 かどうかだけを見る） ／ (b) いいえ（`package.ps1` の本番の走りは回していない）

5 回目と 6 回目も、外への約束を変える指摘と、要件 8.1 の操作でしか確かめられない指摘は 0 件。

### 指摘にしなかったもの（確かめた事実）

- tasks.md の Implementation Notes の 4.1（`exe-install.ps1` の理由の文が置き場のパスを含みうる）は 4.3 で直し済み（`<home>` に替え、`check throws` が縛る）。5.2 の `$a` の二重の使い方は、今は引数の確かめの結果が `$parsed`・繰り返しが `$a` で分かれている。
- `impl-watch-install.ps1` の `OK clear: …` と `clear` の標準エラーは子の出力をそのまま写す（頭書と `doc/impl-watch.md` に書かれた約束の形・7.3 の「子の出力をそのまま写す所は書き替えない」と同じ扱い）。
- `test-all.ps1` の `-License` の後の `git diff --quiet` も、`git` が見つからなければ前の段の `$LASTEXITCODE` を読む。ここは黄色の注意 1 行を出すかどうかだけで、段の合否と終了コードには関わらない（`git` が無ければ頭の `git rev-parse` が先に崩れる）。
- 範囲のほかの台本で `$LASTEXITCODE` を読む所（`crates-io.ps1`・`package.ps1`・`encoding-check.ps1`・`sync-i686-helper.ps1`）は、どれも `$ErrorActionPreference = 'Stop'` の下で、見つからない命令はその場で止まる（`test-all.ps1` と同じ取り違えは起きない）。
- `Show-Ascii` は `$Max` が 1〜3 だと落ちるが、呼び手は 0 か既定の 80 だけ。
- 要る命令が PATH に無い場面のうち、0 でない終了コードと命令の名前の入った文で止まるもの（5〜7 回目に読んだ事実）: `load-flake.ps1` の `git`（出力のフォルダを作った後・`try` の前で止まり、`summary.txt` は書かれない。`-SetEnv` の形の誤りも同じ所で止まる＝頭書の「どの場合も summary.txt を書く」は手順に入った後の話）・`release-notes.ps1 -Notes` の `git`・`crates-io.ps1` と `sample-fetch.ps1` の `cargo`・`package.ps1` の `cargo`（`cargo about not found` の行は cargo が在って cargo-about が無い場面の文で、その場面では届く）。どれも緑にはならない。
- `Invoke-Utf8Child` が命令を `Get-Command -Name` で解く所は、`[` `]` の入った絶対パスでも見つかる（`target\tmp-r5\a[1]\w.exe` を作って確かめ、消した）。
- `sync-i686-helper.ps1` の行は呼び手の渡したパス（既定は ASCII の相対パス）を写す。自分の検査の作業場の絶対パスが出るのは、作業場を消せなかった失敗の 1 行だけ。
- `crates/areka/cursor/README.md` の `OK selftest (2 checks)` は範囲の外（`crates/` は触らない約束）＝tasks.md の Implementation Notes の 8 のとおり 11.4 の申し送りへ。

### 消す前の grep

1 回目〜7 回目のどれも、消した名前・ファイルは無い（直しは検査の追加・判定の中身の改め・1 行の戻し・式の置き替え・1 行を関数へ出すこと・コメントと頭書の文だけ）。grep の対象は 0 件。直した文のうち端末へ出る行（`PASS calibration (N samples)` の数・規則 F の `PASS`／`FAIL` の行）を字面で読む所は、ツリー全体を grep して無い（読み手の全体テストは終了コードだけを見る）。5 回目・6 回目に足した判定の名前（`msvc link path`・`head commit without git`）と関数の名前 `Get-HeadCommit` も、ツリー全体（`target`・`vendors`・`.kiro/specs/completed` を除く）を grep して、在るのは `package.ps1`・`package.selftest.ps1` とこの spec の文書だけ。`Get-MsvcLinkPath` の呼び手は `test-all.ps1`・`package.ps1`・`crates-io.ps1` の 3 か所で、名前・引数・戻りは変えていない。

### 直した後の確かめ（すべて緑）

`pwsh -NoProfile -File tools/encoding-check.ps1`（規則 F の `PASS tools/test-all.ps1: F …` と較正 37 見本を含む）・`tools/release-notes.ps1 -SelfTest`・`tools/sample-fetch.ps1 -SelfTest`（11 判定）・`tools/sync-i686-helper.ps1 -SelfTest`・`tools/app-icon.ps1 -SelfTest`・`tools/cursor-art.ps1 -SelfTest`・`tools/crates-io.ps1`（引数なし）・`tools/package.ps1 -SelfTest`・`tools/exe-install.selftest.ps1`・`tools/load-flake.ps1 -SelfTest`・`tools/mouse-input.ps1 -SelfTest`・`tools/perf/perf-loop.ps1 selftest`（`ok=12 ng=0`）が終了コード 0（1 回目の直しの後に全部、3 回目の直しの後に `encoding-check.ps1`・`release-notes.ps1 -SelfTest`・`sample-fetch.ps1 -SelfTest`・`load-flake.ps1 -SelfTest` を回し直した）。`test-all.ps1` と `encoding-check.ps1` を `Parser.ParseFile` で読んで誤り 0 件。`test-all.ps1` そのものは回していない（11.3 の 1 回へ）。

6 回目の直しの後（7 回目に読んだ形）に、上の 12 本を全部回し直して終了コード 0（`package.ps1 -SelfTest` は 10 判定・`encoding-check.ps1` は較正 37 見本のまま・`perf-loop.ps1 selftest` は `code=0`）。`test-all.ps1`・`encoding-check.ps1`・`tool-common.ps1`・`package.ps1`・`package.selftest.ps1` を `Parser.ParseFile` で読んで誤り 0 件。自分の検査の作業場は `target\` の下に残っていない。

### 範囲の各ファイルの行数（7 回目に読んだ形・どれも 1,000 行の下）

| ファイル | 行数 |
|---|---:|
| `tools/package.ps1` | 781 |
| `tools/encoding-check.ps1` | 625 |
| `tools/load-flake.ps1` | 492 |
| `tools/sample-fetch.ps1` | 398 |
| `tools/release-notes.ps1` | 378 |
| `tools/cursor-art.ps1` | 348 |
| `tools/crates-io.ps1` | 335 |
| `tools/app-icon.ps1` | 334 |
| `tools/mouse-input.ps1` | 275 |
| `tools/package.selftest.ps1` | 249 |
| `tools/exe-install.selftest.ps1` | 174 |
| `tools/sync-i686-helper.ps1` | 174 |
| `tools/exe-install.ps1` | 135 |
| `tools/test-all.ps1` | 112 |
| `tools/impl-watch-install.ps1` | 103 |
| `tools/mcp-bridge-install.ps1` | 101 |
| `tools/utf8-child.ps1` | 70 |
| `tools/tool-common.ps1` | 53 |
| `tools/release-notes-fixture/expected-notes.md` | 39 |
| `tools/release-notes-fixture/expected-notes-from-start.md` | 29 |
| `tools/release-notes-fixture/subjects.txt` | 21 |
| `tools/release-notes-fixture/expected-body.md` | 16 |
| `tools/release-notes-fixture/highlights-ok.md` | 12 |
| `tools/release-notes.strings.json` | 11 |
| `tools/release-notes-fixture/spec-names.txt` | 9 |
| 計（25 ファイル） | 5,274 |
