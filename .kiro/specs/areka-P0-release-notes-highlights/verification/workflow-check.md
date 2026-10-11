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
