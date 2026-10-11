# workflow の確かめの記録

道具（`tools/release-notes.ps1`）の手元の走り、workflow の静的な確かめ、段の本文の手元の走り、乾いた走りの結果を、1 項目 1 行で残す。後ろのタスクは、自分の節を末尾に足す。

## 詳しいリリースノートの生成の手元の走り（タスク 2.2）

- 回した日: 2026-10-11。枝 `claude/kiro-spec-release-notes-95477d` の先頭 `a8a30331`（作業木にタスク 2.2 の変更を置いた状態）。コマンドは作業木の根で回した。書き出しの先は、どれも作業木の `target\release-notes-task22\` の下。
- 自分の検査: `pwsh -NoProfile -NonInteractive -File tools/release-notes.ps1 -SelfTest` → 終了コード 0（`selftest: all passed`。詳しいリリースノートの判定は 4 つ: 定型文の鍵・見本の題名が 1 つ以上・見本の出力と 1 字違わず同じ・一つ前のタグが空で fix と perf を除いた出力と同じ）。
- 檻が効くこと: 写しの道具で決まりを 1 つずつ崩して自分の検査を回すと、7 つとも赤（終了コード 1）: spec 名の印を外さない／perf を見出しに当てない／PR のリンクを付けない／0 件の見出しを出す／スコープを見ない／最初からの範囲の文を使わない／件数を固定する。崩していない写しは緑。
- 文字コードの検査: `pwsh -NoProfile -NonInteractive -File tools/encoding-check.ps1` → 終了コード 0（`FAIL` の行 0・`PASS tools/release-notes.ps1`）。
- `v0.0.2` から枝の先頭まで: `-Notes -Version 0.0.3 -PrevTag v0.0.2 -To HEAD -Repository ekicyou/areka -Out target\release-notes-task22\notes-0.0.3.md` → 終了コード 0・`- ` で始まる行 41＝`git rev-list --first-parent --count v0.0.2..HEAD` の 41・`areka-P0-` 0 件（PR へのリンクの付いた行 37・付かない行 4）。
- `v0.0.1` から `v0.0.2` まで: `-Notes -Version 0.0.2 -PrevTag v0.0.1 -To v0.0.2 -Repository ekicyou/areka -Out target\release-notes-task22\notes-0.0.2.md` → 終了コード 0・`- ` で始まる行 614＝`git rev-list --first-parent --count v0.0.1..v0.0.2` の 614・`areka-P0-` 0 件（PR へのリンクの付いた行 194・付かない行 420。うち 2 行は題名の末尾が `(PR#番号)` の形で、番号を文のまま載せている）。
- 一つ前のタグが空: `-Notes -Version 0.0.3 -PrevTag '' -Repository ekicyou/areka -Out target\release-notes-task22\notes-from-start.md` → 終了コード 0・`- ` で始まる行 763＝`git rev-list --first-parent --count HEAD` の 763・`areka-P0-` 0 件・範囲の文は「最初から v0.0.3 まで…（763 件）」。
- 端末の文字コード 932 の子: 窓の無い自分だけの端末を持つ子（`Invoke-Utf8Child … -OwnConsole`）が自分の端末を 932 にしてから `v0.0.1`〜`v0.0.2` を出す → 終了コード 0・その端末では素の `git log` の読みが日本語を壊す（較正）・出力は、端末を 65001 にした子の出力と 1 バイトも違わない（どちらも SHA256 `d9f18ec83d172f47c9a7de1e9e2fbe50a0edb420c2256ebf8a2634ec7b601fb5`）・親の端末の文字コードは前後で同じ（932/932）。
- 書き出しの形: `notes-0.0.2.md` は BOM なし・CR 0 個・U+FFFD 0 個・末尾は LF 1 つ。
- 2 回回す: 同じ引数（`v0.0.1`〜`v0.0.2`）でもう 1 回出す → 1 バイトも違わない（SHA256 が上と同じ）。
- 別のフォルダから: `target\release-notes-task22\elsewhere\sub` を今のフォルダにして、道具を絶対パスで呼び、`-Out ..\new-dir\rel.md` → 終了コード 0・無かったフォルダ `elsewhere\new-dir` が作られ、中身は上と 1 バイトも違わない。
- 変更が 0 件の範囲: `-PrevTag v0.0.2 -To v0.0.2` → 終了コード 0・題の行と範囲の文（0 件）だけで、見出しは 0。
- 見えないタグ: `-PrevTag v9.9.9` → `FAIL git log (exit 128): fatal: bad revision 'v9.9.9..HEAD'`・終了コード 1・`-Out` のファイルは作られない。
- `-` で始まる先: `-To '--output=target/x'` → `FAIL git log (exit 128): fatal: bad revision …`・終了コード 1（`git` のオプションとして働かず、ファイルは作られない）。
- 鍵の欠けた定型文（写しの道具で）: `FAIL strings: cannot read tools/release-notes.strings.json, or a key is missing or empty`・終了コード 1・`-Out` のファイルは作られない。

## workflow の今の決まりの確かめと、段の本文の手元の走り（タスク 4.3）

- 回した日: 2026-10-11。枝 `claude/kiro-spec-release-notes-95477d` の先頭 `7c6637a5`（作業木の workflow は先頭と同じ: `git diff --quiet HEAD -- .github/workflows/release.yml` が 0）。比べる元は、workflow を直す前の `8c76fff5`。下の値は、どれもこのタスクで測り直した。
- 測り方: `.github/workflows/release.yml` を `git show` で取り出し、PyYAML 6.0.3 で読む。段の本文（`run:` の中身）を BOM なしの UTF-8 のファイルへ書き出し、作業木の根から、GitHub が段を回すのと同じ形 `pwsh -NoProfile -NonInteractive -Command ". '{ファイル}'"` で回す（PowerShell 7.6.6）。置き場は作業木の `target\task43\` の下で、終わってから `target\release-notes\` と一緒に消した。push・本物の `gh` の呼び出し・workflow の走りは、どれも 0 回。
- 権限（5.8）: `permissions` は workflow の段に `contents: write` の 1 つだけ・ジョブ（`release` の 1 つ）に `permissions` は無い・ファイルの中の `permissions:` の行は 1 → 合格。
- GitHub の式（5.8）: 段の本文 11 個の中の `${{` は 0。ファイル全体では 11 で、置き場は `concurrency` の `group` が 1、ジョブの `env` の `PUBLISH` が 1、段の `env:` の値が 9（`VERSION` 5・`GH_TOKEN` 3・`PREV_TAG` 1）→ 合格。
- タグの名前で Release を探す行（5.8）: `releases?per_page=100` を含む行は 2（段「既存の Release の検査」と段「後始末」に 1 つずつ）・形は 1 通り（228 字・SHA256 `3b64798a7bb24513e071b2c8647745872f42b085b7216825e077ae194b1b2ebf`）・`8c76fff5` の行とも同じ → 合格。
- 端末の文字コード（5.8）: workflow の全文（コメントを含む）を、大文字と小文字を分けずに数えて、`chcp` 0・`OutputEncoding` 0・`InputEncoding` 0・`[Console]::` 0・`SetConsoleCP` 0・`SetConsoleOutputCP` 0・`65001` 0・`PSDefaultParameterValues` 0・`PYTHONIOENCODING` 0・`PYTHONUTF8` 0 → 合格。同じ数え方は、末尾に `chcp 65001` と `[Console]::OutputEncoding = [Console]::InputEncoding` の 2 行を足した写しでは `chcp` 1・`OutputEncoding` 1・`InputEncoding` 1・`[Console]::` 2・`65001` 1 を数える（較正）。
- 段の本文の字（5.8）: 11 個の本文のうち、コメントでない行で ASCII の外の字を持つ行は 0 → 合格。同じ見分け方で、ASCII の外の字を持つコメントの行は 47（較正）。11 個の本文の PowerShell の構文の誤りは 0。
- 本文の作り方のオプション（5.1）: workflow の全文で `--generate-notes` 0（`8c76fff5` では 1）・`--notes-start-tag` 0（同 1）・`--notes-file` 1（同 0）・`--draft` 0（同 0）→ 合格。
- 段の並び（3.1・3.3）: 13 段＝改行の設定／取り出し／Rust の固定／版の検査／見どころの検査／既存の Release の検査／リリースノートを作る／道具の用意／環境の記録／zip を作る／5 つの確かめ／Release を公開／後始末。「見どころの検査」は「版の検査」の直後（5 番目）で、「道具の用意」（8 番目）と「zip を作る」（10 番目）より前。「リリースノートを作る」は「既存の Release の検査」の直後（7 番目）。`if:` を持つ段は「Release を公開」と「後始末」の 2 つだけで、新しい 2 段と「5 つの確かめ」の `if:` は 0（タグの走りでも乾いた走りでも回る）→ 合格。
- 直す前と変わらない段（5.4・5.5）: `8c76fff5` と比べて、8 段（改行の設定・Rust の固定・版の検査・既存の Release の検査・道具の用意・環境の記録・zip を作る・後始末）は、名前・条件・環境変数・本文のすべてが同じ。「後始末」と「zip を作る」の本文の差分は、どちらも 0 行。`git diff 8c76fff5 HEAD -- tools/package.ps1` も 0 行 → 合格。
- 変わった段（5.5）: 「取り出し」は `with` に `fetch-depth: 0` が足されただけ（`uses` の固定と `persist-credentials: false` は同じ）。「4 つの確かめ」→「5 つの確かめ」は、消えた行が 2 行でどちらもコメント、足された行が 19 行（今の 4 つの判定の行は 1 行も消えていない）。「Release を公開」は `env` から `PREV_TAG` が消え、本文は消えた行 6・足された行 7 で、`gh release create $tag @files @opts` の行と `--verify-tag` は `8c76fff5` と同じ。新しい段は 2 つ（見どころの検査・リリースノートを作る）→ 合格。
- 文字コードの検査（5.8）: `pwsh -NoProfile -File tools/encoding-check.ps1` → 終了コード 0・`FAIL` の行 0・`PASS .github/workflows/release.yml`・`PASS .github/workflows/release.yml: E Invoke-Utf8Child metadata`・`PASS own console code pages unchanged (output 932, input 932)`・最後の行は `encoding check: all passed` → 合格。
- 段「見どころの検査」・見どころの在る版（4.1 の本文・3.1）: `VERSION=0.0.2` → `OK highlights v0.0.2 (5 item(s))`・終了コード 0 → 合格。
- 段「見どころの検査」・見どころの無い版（3.2・3.4）: `VERSION=9.9.9` → `FAIL missing-file line 0: the highlights file of this version does not exist`・終了コード 1（欠けたものの名前 `missing-file` が走りの記録に出る）→ 合格。
- 段「リリースノートを作る」・通る走り（4.2 の本文・5.7）: `VERSION=0.0.2 PREV_TAG=v0.0.1 GITHUB_REPOSITORY=ekicyou/areka GITHUB_STEP_SUMMARY={置き場のファイル}` → `release notes end at 'refs/tags/v0.0.2'`・`OK release notes v0.0.2 (614 change(s))`・`OK release body v0.0.2`・終了コード 0。`target/release-notes/` のファイルは 2 つ（`release-notes-v0.0.2.md` 77638 バイト・`release-body-v0.0.2.md` 3884 バイト）→ 合格。
- 詳しいリリースノートの中身（4.1・4.4）: `- ` で始まる行 614＝`git rev-list --first-parent --count v0.0.1..v0.0.2` の 614・`areka-P0-` 0 件・SHA256 `d9f18ec83d172f47c9a7de1e9e2fbe50a0edb420c2256ebf8a2634ec7b601fb5`（タスク 2.2 の走りと同じ値）。同じ入力でもう 1 回回しても SHA256 は同じ → 合格。
- 本文の中身（5.1）: 先頭から `doc/release-notes/v0.0.2.md` の文と 1 字違わず同じで、その後ろは区切りの線 `---` の 1 行と、添付 `release-notes-v0.0.2.md` へのリンクの 1 文だけ（`https://github.com/ekicyou/areka/releases/download/v0.0.2/release-notes-v0.0.2.md`）。`areka-P0-` 0 件 → 合格。
- 走りの結果の頁（5.7）: `GITHUB_STEP_SUMMARY` のファイルは 81611 バイト・UTF-8 として誤りなく読める・BOM なし・U+FFFD 0 個・CR 0 個。中身は「ASCII の見出しの行 `## release body (release-body-v0.0.2.md)`・本文の全文・ASCII の見出しの行 `## release notes (release-notes-v0.0.2.md)`・詳しいリリースノートの全文」を並べた物と 1 字違わず同じ（見出しの行は 2）→ 合格。
- 段「リリースノートを作る」・一つ前のタグが空（4.2）: `PREV_TAG=`（空）→ 終了コード 0・`- ` で始まる行 722＝`git rev-list --first-parent --count v0.0.2` の 722（`-PrevTag` を渡さず、最初のコミットから並ぶ）→ 合格。
- 段「リリースノートを作る」・見どころの無い版（5.6）: `VERSION=9.9.9` → `release notes end at 'HEAD'`（その版のタグが無いので先は先頭）・`OK release notes v9.9.9 (661 change(s))` の後に `FAIL missing-file line 0: …`・終了コード 1・`release-body-v9.9.9.md` は作られない・走りの結果の頁へは 1 バイトも足されない → 合格。
- 段「リリースノートを作る」・見えないタグ（5.6）: `PREV_TAG=v9.9.9` → `FAIL git log (exit 128): fatal: bad revision 'v9.9.9..refs/tags/v0.0.2'`・終了コード 1・走りの結果の頁へは 1 バイトも足されない（取り出しが浅いときに落ちる場所）→ 合格。
- 段「5 つの確かめ」・6 つとも在る（4.2 の本文・5.2）: 偽の根 `target\task43\root\` に、`target/package/` の 4 つ（見本の zip 2 つと、その SHA256 を書いた `.sha256` 2 つ・名前は版 0.0.2 の物）と、上の走りが作った 2 つを `target/release-notes/` に置き、偽の根を今のフォルダにして `VERSION=0.0.2` → `ok:` の行 6・終了コード 0。前後で 6 つのファイルの名前と SHA256 は同じ（読むだけ）→ 合格。
- 段「5 つの確かめ」・詳しいリリースノートを除く（5.6）: `missing: release-notes-v0.0.2.md (dir …\target\release-notes)`・終了コード 1 → 合格。
- 段「5 つの確かめ」・本文を除く（5.6）: `missing: release-body-v0.0.2.md (dir …\target\release-notes)`・終了コード 1 → 合格。
- 段「5 つの確かめ」・本文を 0 バイトにする（5.6）: `empty: release-body-v0.0.2.md (dir …\target\release-notes)`・終了コード 1 → 合格。
- 段「5 つの確かめ」・今の 4 つの判定（5.4）: `areka-0.0.2-arm64.zip.sha256` を除く → `missing: areka-0.0.2-arm64.zip.sha256 (dir …\target\package)`・終了コード 1。戻すと終了コード 0 → 合格。
- GitHub が本文の前後に足す行を付けた形: 3 つの段の本文の頭に `$ErrorActionPreference = 'stop'`、末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }` を足した写しでも、上の走り（見どころの検査の 2 つ・リリースノートを作るの通る走りと見どころの無い版・5 つの確かめの 6 つとも在る走りと除く走り 2 つ）の終了コードと印字は同じ → 合格。
- 公開の段が渡す物（5.2・5.5）: 段「Release を公開」の本文を、`gh` を「受けた引数をファイルへ書き留めて 0 で返すだけの関数」に差し替えて回す（本物の `gh` は呼ばれない・偽の根・`VERSION=0.0.2`）→ 終了コード 0・引数は 15 個で、`release create v0.0.2` の後ろにファイル 5 つ（`areka-0.0.2-x64.zip`・`areka-0.0.2-x64.zip.sha256`・`areka-0.0.2-arm64.zip`・`areka-0.0.2-arm64.zip.sha256`・`release-notes-v0.0.2.md`。5 つとも偽の根に在る）、その後ろに `--repo {リポジトリ} --verify-tag --notes-file {…\release-body-v0.0.2.md} --title v0.0.2` → 合格。本物の `gh` が下書きで作って公開する所は、手元では確かめていない（設計の「本 spec では確かめられないもの」）。
- 使う道具（5.9）: 変えた 4 つの段の本文が呼ぶ外の道具は、見どころの検査が `pwsh`、リリースノートを作るが `git` と `pwsh`、5 つの確かめが 0、Release を公開が `gh`（PowerShell の構文木からコマンドの名前を取り出し、種類が外の実行ファイルの物を数えた）。`gh`・`git`・PowerShell 7 の外の道具は 0 → 合格。
- workflow の直し: このタスクでは 0 行（上の確かめは、どれも合格）。

## 枝での乾いた走り（タスク 7.1）

- 記録した日: 2026-10-11。走りの URL は https://github.com/ekicyou/areka/actions/runs/38124827102 。走りが使ったコミットは、枝 `claude/kiro-spec-release-notes-95477d` の `db73ef9f`。この節の値は、「進行役の報告」と断った 2 行を除いて、タスク 7.1 の実装係が読むだけの操作（`gh api` の GET・`gh run view --log`・`gh run list`・`gh release list`）で測った。実装係の push・走りの起動・やり直し・取り消し・Release への書き込みは、どれも 0 回。走りの記録は作業木の `target\task71\` に保存して読み（1428 行・144846 バイト・U+FFFD 0 個）、終わってから消した。
- 進行役の報告（実装係は測っていない）・承認と起動: 2026-10-11 に開発者が会話で承認した（「全て許可。推奨で。」）。進行役が `git push -u origin claude/kiro-spec-release-notes-95477d` で枝を origin に作った（先頭 `db73ef9f4aad7cb21abfd64fcbd55434b258a106`）。1 回目の `gh workflow run` は自動モードの見張りに止められ、開発者がユーザー設定に許可の決まりを足した後、進行役が `gh workflow run release.yml --repo ekicyou/areka --ref claude/kiro-spec-release-notes-95477d` で走りを起こした。
- 進行役の報告（実装係は測っていない）・走りの結果の頁（5.7）: ログインしたブラウザで走りの頁を読むと、Markdown の塊が 1 つ（42,470 字）在り、先頭は見出し `release body (release-body-v0.0.2.md)`。中に、見どころの挨拶の行・`h3` の見出し 5 つ・見出し `release notes (release-notes-v0.0.2.md)`・題「areka v0.0.2 の詳しいリリースノート」・範囲の文「v0.0.1 から v0.0.2 までに入ったすべての変更です（614 件）。」・箇条書き 614 個が在り、U+FFFD は 0 個。ログインしていないブラウザでは、走りの結果の頁の本文は出ない。頁の示す全体の時間は 12m 31s → 合格（進行役の読み）。
- 走りの全体: 結論 `success`・起こり方 `workflow_dispatch`・枝 `claude/kiro-spec-release-notes-95477d`・コミット `db73ef9f4aad7cb21abfd64fcbd55434b258a106`（今の origin の枝の先頭と同じ）・試行 1 回目・始まり `2026-10-11T08:26:50Z`・終わり `2026-10-11T08:39:21Z`・全体 751 秒（12 分 31 秒。進行役の報告の 12m 31s と同じ）。ジョブは 1 つ（`release`・id `114426914893`・`success`・`windows-2025-vs2026`・`08:26:53Z`〜`08:39:20Z` の 747 秒）。走りの印は `PUBLISH: false` が 12 行・`PUBLISH: true` が 0 行（乾いた走り）→ 合格。
- 走りが使った物と今の作業木の差: `git diff --quiet db73ef9f HEAD -- .github/workflows/release.yml tools/ doc/release-notes/` は 0（走りの後に足したコミットは、workflow も道具も見どころのファイルも変えていない）。
- 段ごとの結果と秒数（ジョブの API の `started_at`・`completed_at` から）: Set up job 成功 1／改行の設定 成功 2／取り出し 成功 16／Rust の固定 成功 12／版の検査 成功 1／見どころの検査 成功 1／既存の Release の検査 成功 8／リリースノートを作る 成功 2／道具の用意 成功 4／環境の記録 成功 2／zip を作る 成功 693／5 つの確かめ 成功 0（1 秒未満）／Release を公開 `skipped`／後始末 `skipped`／Post 取り出し 成功 2／Complete job 成功 0。失敗の段は 0。
- 取り出しの時間: 16 秒（`08:26:57Z`〜`08:27:13Z`）。走りの記録に `fetch-depth: 0` の行が在り、取り込みの行は枝とタグの全部（`+refs/heads/*:refs/remotes/origin/* +refs/tags/*:refs/tags/*`）で、`[new tag] v0.0.1` と `[new tag] v0.0.2` が出る。履歴を浅く取っていた前の 2 回は、タグ `v0.0.2` の走り（`37464630569`）が 11 秒、2026-10-06 の main の乾いた走り（`37458177482`）が 12 秒。増えたのは 4〜5 秒 → 記録（合否は付けない）。
- 版の検査: `no tag; skipping the comparison (Cargo.toml version '0.0.2')`（乾いた走りなので、版は Cargo.toml の `0.0.2`）。
- 見どころの検査（3.3）: 成功・1 秒。印字は `OK highlights v0.0.2 (5 item(s))` の 1 行。段は「版の検査」の直後・「道具の用意」と「zip を作る」より前に回った（タグの走りと同じ場所・条件なし）→ 合格。
- 一つ前のタグ（今の段「既存の Release の検査」が求めた値）: `release for tag 'v0.0.2' already exists (URL https://github.com/ekicyou/areka/releases/tag/v0.0.2, draft false)`・`dry run; continuing`・`previous tag: 'v0.0.1'`。次の段の環境変数は `PREV_TAG: v0.0.1`。
- リリースノートを作る（5.7）: 成功・2 秒。印字は 3 行: `release notes end at 'refs/tags/v0.0.2'`・`OK release notes v0.0.2 (614 change(s))`・`OK release body v0.0.2`。一覧の先がタグ `refs/tags/v0.0.2` なのは、版 0.0.2 のタグがリポジトリに在るからで（取り出しがタグを取り込んでいる）、枝の先頭までの変更は混ざらない。範囲は `v0.0.1` から `v0.0.2` までで、614 件は手元の `git rev-list --first-parent --count v0.0.1..v0.0.2` の 614 と同じ（手元の 2 つのタグの指すコミットは、API の示す `61d5ba9b`・`e3293457` と同じ）→ 合格。
- 走りの結果の頁へ足す所（5.7）: 実装係は、頁の中身を読んでいない（上の進行役の報告が受ける）。実装係が見たのは、足す行を最後に持つ段「リリースノートを作る」が成功で終わったことと、件数 614 が進行役の読んだ箇条書き 614 個と同じであること。
- zip を作る: 成功・693 秒（11 分 33 秒）。末尾は `commit db73ef9, uncommitted changes 0`・`OK   finalize (0 s)`・`all steps green`。前の 2 回は 640 秒と 707 秒。
- 5 つの確かめ: 成功・1 秒未満。`ok:` の行は 6: `ok: areka-0.0.2-x64.zip (SHA256 993bb2d678bce8791f96ceaca1f7d8307048f507ba6b485f66be96ea8e5cf767)`・`ok: areka-0.0.2-x64.zip.sha256`・`ok: areka-0.0.2-arm64.zip (SHA256 14213b33285b2455a814d946e8e523feab9345ebe651a2030034536a9e85072a)`・`ok: areka-0.0.2-arm64.zip.sha256`・`ok: release-notes-v0.0.2.md (77638 bytes)`・`ok: release-body-v0.0.2.md (3884 bytes)`。その後ろに `free space on work drive D:\: 144.7 GB` → 合格。
- 2 つのファイルの大きさ: 走りの `release-notes-v0.0.2.md` 77638 バイト・`release-body-v0.0.2.md` 3884 バイトは、タスク 4.3 の手元の走りの 77638・3884 と同じ（どちらも差 0 バイト）→ 合格。
- 公開の段が動かないこと（5.7）: 「Release を公開」は `skipped`・「後始末」も `skipped`。走りの記録の中で、段が回した行に `gh release` は 0 件。`GH_TOKEN` を受けた段は「既存の Release の検査」の 1 つだけ（読むだけの段）→ 合格。
- Release が作られていないこと: `gh release list --repo ekicyou/areka --limit 5` は `v0.0.2`（Latest・公開 `2026-10-06T12:49:21Z`）の 1 行だけ。API の一覧も 1 件（id `404699435`・下書きでない）で、下書きは 0・新しい版の名前の物は 0。タグは `v0.0.2` と `v0.0.1` の 2 つのまま → 合格。
- 走りの間に `v0.0.2` の Release が変わっている件: Release の `updated_at` は `2026-10-11T08:29:25Z`、添付 `release-notes-v0.0.2.md`（77638 バイト）の作られた時刻は `2026-10-11T08:27:05Z` で、どちらも走りの間に当たる。これは走りの仕業ではない: 添付を載せたのは `ekicyou`（もとの 4 つは `github-actions[bot]`）で、2 つの時刻は `v0.0.2-page-fix.md`（タスク 7.2 の頁の直しの記録）の時刻と同じ。走りの側は、`08:29:25Z` には鍵を持たない段「zip を作る」の中に居て、公開の段は回っていない。
- 警告と注記: ジョブの注記（`check-runs/114426914893/annotations`）は 0 件。走りの記録の `##[warning]`・`##[error]`・`##[notice]` の行は 0。
- 走りの記録へ出す自分の文（5.8 のうち、走りで見える分）: 3 つの段が印字した行（見どころの検査 1・リリースノートを作る 3・5 つの確かめ 7）のうち、ASCII の外の字を持つ行は 0 → 合格。5.8 の残り（権限・式・Release を探す行・端末の文字コード）は、上のタスク 4.3 の静的な確かめが受ける。
- 赤の項目: 0。走りのやり直しは要らない（走りは 1 回・試行 1 回目のまま）。
- この走りで確かめていないもの: 「Release を公開」と「後始末」は 1 回も回っていない。5 つの添付と本文で Release を作る所が初めて本当に動くのは次の本番のリリース（`v0.0.3`）で、リリースの手順の公開の後の確かめが受ける（設計の「本 spec では確かめられないもの」と Risks のとおり）。見どころの検査が赤で止まる走りも、枝では起こしていない（手元の走りはタスク 4.3 の `VERSION=9.9.9` の行）。
