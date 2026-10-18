# `/release` の手順（正本）

areka のリリース 1 回分の手順。段・コマンド・判定・表はこの文書が正本で、`SKILL.md` は約束と段の要約だけを持つ。もとは繰り返し spec `areka-P0-release-cycle` の設計とタスクで、2026-10-11 にここへ移した（同 spec は `.kiro/specs/completed/` の記録で、実行には使わない）。

使う道具は main に在る物だけ（`tools/test-all.ps1`・`tools/package.ps1`・`tools/crates-io.ps1`・`tools/release-notes.ps1`・`gh`・`git`・`cargo`）。この手順はスクリプトを足さず、判定は主文脈がコマンドの出力を読んで表に当てはめる。

## 読み方

- **置き換える語**: `{remote}`＝`git remote`（名前だけを出す。`-v` は付けない）で読んだリモートの名前。`{枝}`＝`git branch --show-current` で読んだ、ハーネスが用意した作業の枝。`{版}`＝その回に出す版。`{旧版}`＝上げる前の版。`{sha}`＝版上げの PR の squash のコミット。`{確かめた main}`＝手元の確かめ（段 3）を回した作業木が取り込んでいる main の先端の SHA（1.1 で覚える）。`{番号}`＝版上げの PR の番号。`{id}`＝走りの番号。
- **`git` の決まり**: `git fetch`・`git push`・`git ls-remote` には必ず `--quiet` を付け、標準エラーは捨てる。成否は終了コードと、その後の読み直し（`ls-remote`・`gh`）で決める。
- **判定**: 各項目の「判定」は、その場で取り直したコマンドの出力と終了コードで確かめる。当たらなければ、その項目に書いた止まり方で止まる。
- **始める場所**: ハーネスのワークツリーの枝で始める。`{枝}` が `main` なら止まる。手順は枝を作らない。新しい回は、main から切った新しいワークツリーの枝で始めるのが基本（前の回を出した枝で次の回を打つときは、版を引数で指示する＝「再開のときの事実の読み方」）。
- **順**: 上から順に回す。並べて回せる項目は無い。

### してはいけない事

- 接続先の URL を印字するコマンド（`git remote -v` など、`doc/crates-io-publish.md` 7 節の一覧）。
- `git push --tags`・`git push --follow-tags`・`--force` の付く push（手元にだけ在るタグを押し出さない）。
- `git tag -d`・`git push --delete`。押したタグは、どの段でも動かさず、消さない。
- `tools/test-all.ps1 -Format`（ソースを書き換える）。
- `git add -A`・`git add .`。足す物は名前を挙げる。
- main へ直接 push する操作（0 回）。
- 手元からの `gh release create`・`gh release upload`・`cargo publish`。公開はリリース CI が行う。
- `tools/`・`.github/workflows/`・各クレートの書き換え。

### 止まり方の決まり

- **順調な経路では止まらない**: PR のマージとタグの push は、その前の判定が全部当たったときだけ、開発者に聞かずに行う。
- **止まる所**: 判定が当たらないとき・走りが赤のとき・この文書に無い状態を見たとき・許可の判定にコマンドを断られたとき。止まったら、見えた事実と、どの項目で止まったかを報告する。
- **取り返しのつく所で止まる**: 段 1〜3 の欠けは、PR を出す前に止まる。main にも GitHub にも何も出ていない。作業木は消さずに残し、欠けた確かめの名前を報告する。
- **タグの後は直さない**: 赤の原因がタグのコミットに在れば、同じ版で出し直さず、起票して次の版で出す。タグは動かさない。
- **マージの後にタグを打てなくなったとき**（段 5 の判定が外れた・許可の判定に断られた）: main の版は上がったまま待つ。後で再開すれば、同じ squash のコミットにタグを打てる。
- **`git` の失敗**: 標準エラーを捨てているので、終了コードが 0 でないときは、事実の読み直し（`ls-remote`・`gh pr view`）で何が起きたかを確かめてから報告する。接続先を出して調べることはしない。
- **許可の判定に断られたとき**: 言い換えず、別の道具へ回さない。そのコマンドと断られた理由を示して止まる。
- **机の待ちは止まりに数えない**: 「机の段」の排他テストの机の番待ちと、停止要請の後の再開待ちは止まりではない。開発者に承認を求めず、1 行だけ伝えて、待ちが終わればそのまま続ける。机が使えないときも止まらず、机の段を飛ばして続ける。

## 段 1: 前提を確かめる

### 1.1 作業木と枝を確かめ、main の最新を取り込む

- `git status --porcelain` が 0 行であること。
- `git fetch --quiet {remote} main` の後、`git merge-base --is-ancestor {remote}/main HEAD` が 0 でなければ `git merge {remote}/main` で取り込む。衝突したら止まる。
- `git diff --name-only {remote}/main...HEAD` を読む。0 件であること。
- `git rev-parse {remote}/main` を「確かめた main」として覚える（段 3 の確かめは、この main を取り込んだ作業木で回す。途中で `fetch` が挟まっても、確かめていない main を「確かめた」と取り違えない）。
- 判定: `git merge-base --is-ancestor {remote}/main HEAD` が終了コード 0 で、枝の差分が 0 件で、「確かめた main」の SHA を覚えた。欠けたら、どれが欠けたかを報告して止まる。

### 1.2 根の 3 つのファイルを触る開いた PR が 0 本であることを確かめる

- `gh pr list --state open --limit 200 --json number,title,files` を読み、`files` に根の `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を持つ PR を数える。
- `files` がちょうど 100 件の PR は、`gh pr diff {番号} --name-only` で読み直す（`files` は 1 本あたり 100 件までしか返らない）。
- この 3 つを触らない PR が開いていても止まらない。
- 判定: 当たる PR が 0 本。在れば、その PR の番号と題を示して止まる。

### 1.3 今の版と、そのタグを確かめる

- 根の `Cargo.toml` の `[workspace.package]` の `version` と `[workspace.dependencies]` の `dola` の `version` を読み、同じ値なら `{旧版}` とする。違えば止まる。
- `git ls-remote --quiet --tags {remote} refs/tags/v{旧版}` が 1 行であること。0 行なら前の回が「マージの後・タグの前」で止まっている。版を上げずに止まり、題が `chore(release): v{旧版}` のマージ済みの PR を `gh pr list --state merged --search "chore(release): v{旧版} in:title" --json number,mergeCommit` で示して、その squash のコミットで段 5 から続けるかを開発者に聞く。
- 判定: 2 行の版が一致し、`v{旧版}` が在る。

### 1.4 次のリリースへの申し送りを読む

- `.kiro/steering/roadmap.md` の節「次のリリースへの申し送り」を読む。項目の頭には、書いたときの最新の版が「`v{その版}` の後」の形で付いている。
- `{旧版}` がその版より新しい項目は、前の回で済んだ物として読み飛ばし、最後の報告に「消してよい申し送り」として挙げる（この手順は roadmap を書き換えない）。
- 残った項目を 2 つに分ける。
  - **版の決め方・見どころの材料**: 1.5 の版と 2.3 の見どころで使う。使う人が直さなければならない変更は、2.3 で必ず見どころにする。
  - **版上げの 5 つのファイルの外を触る仕事**: この手順では行わない（版上げの PR に版上げ以外を混ぜない）。1 つでも残っていたら、項目を示して止まり、先に別の PR で片づけるか、次の回へ送るかを開発者に聞く。
- 判定: 残った項目が 0 件、または全部が「材料」で、使う段を決めた。

### 1.5 次の版を決め、そのタグが無いことを確かめる

- 次の版を決める。指示が無ければ `{旧版}` の 3 つ目の数字に 1 を足す。指示があればその版（頭の `v` は外す）。数字 3 つを点でつないだ形でなければ聞き直す。
- 1.4 の材料が、指示の無い回の「3 つ目の数字に 1 を足す」と違う版を求めているときは、自分で決めずに、止まって開発者に聞く。
- `git ls-remote --quiet --tags {remote} refs/tags/v{版}` が 0 行であること。在れば止まる（同じ版で出し直さない）。
- `cargo set-version --version` が終了コード 0 かを読み、2.1 のやり方（いつものやり方か代わりのやり方）を決めておく。
- 判定: `v{版}` が無く、`{版}` が数字 3 つの形である。

## 段 2: 版を上げる

### 2.1 版上げの道具で根の `Cargo.toml` と `Cargo.lock` の版を上げる

- **いつものやり方**: 1.5 で道具が在ると分かったら、指示が無い回は `cargo set-version --bump patch --workspace`、指示がある回は `cargo set-version {版} --workspace` を回す。`--locked` は付けない（`Cargo.lock` を書き直すので、付けると失敗する）。
- **代わりのやり方**（道具が無いか、根の `Cargo.toml` の 2 行目＝`dola` の行が動かなかったとき）: 2 行を Edit で直し、`cargo update -w` で `Cargo.lock` を揃える。
- どちらのやり方を使ったかを覚え、最後の報告に書く。
- 判定: 根の `Cargo.toml` の 2 行がどちらも `{版}` を読む。正しく上がったかの判定は 3.2 が持つ。

### 2.2 `dist/README.txt` の「時点」の行の日付を直す

- 冒頭の「この説明書は …… 時点の内容です。」の日付を、その日の日付に Edit で直す。
- ファイルは BOM 付き・CRLF。その 1 行の置き換えだけにして、ほかの行と改行を動かさない。
- 判定: `git diff -U0 -- dist/README.txt` が、足した行 1・消した行 1 の「時点」の行だけを返す（同じ日の 2 回目の版上げなら 0 行）。

### 2.3 その版の見どころを書く

- 変更の一覧を手元で出す。`pwsh -NoProfile -File tools/release-notes.ps1 -Notes -Version {版} -Repository ekicyou/areka -PrevTag v{旧版} -To {確かめた main} -Out target/release-notes/release-notes-v{版}.md`（`{確かめた main}` は 1.1 で覚えた SHA）。出てくるのは、Release に添える詳しいリリースノートと同じ形の一覧。置き場は `target/` の下なので、git には入らない。
- 先を `-To` で main に指す理由: 道具は本線（`git log --first-parent`）だけを読む。作業の枝が main を merge で取り込んでいると、枝の先頭から読んだ本線は枝の側をたどり、main に入った変更が 1 行も出ない。
- `v{旧版}` のタグが手元に無いとき（`git rev-parse -q --verify refs/tags/v{旧版}` が 0 でない）は、先に `git fetch --quiet {remote} refs/tags/v{旧版}:refs/tags/v{旧版}` で、その 1 本だけを取る。
- 一覧から見どころを選び、`doc/release-notes/README.md` の決まり（形・口調・必ず載せるもの・書かないもの）に従って、`doc/release-notes/v{版}.md` を新しく書く。決まりは同書が正本で、ここには写さない。ほかの版の見どころのファイルは触らない。
- 書いたら、同書の「必ず載せるもの」を読み直し、一覧と 1.4 の材料に当たる変更が全部載っているかを自分で確かめる（口調と「必ず載せるもの」は機械では見ない。開発者は最後の報告で全文を読む）。
- その場で `pwsh -NoProfile -File tools/release-notes.ps1 -Check -Version {版}` を回し、欠けが出たら直してから段 3 へ進む（3.1 の全体テストも同じファイルの形を見るので、欠けたまま進むと、長い全体テストが終わりの近くで赤になる）。
- 判定: `doc/release-notes/v{版}.md` が在り、その場の `-Check` が終了コード 0。PR を出す前の判定は 3.5 が持つ。

## 机の段

段 3 の全体テスト（3.1）と起動の確かめ（3.3）、4.2 の 4 の回し直しは、ほかのセッションの全体テスト・UI テストと重ならないよう、走行の間だけ `areka-impl-watch.exe` の**排他テストの机**を持って回す。机の決まり・判定の基準・コマンドの正本は手順書 `doc/impl-watch.md` で、この節は `release` からの呼び方だけを持つ（`kiro-impl` のスキルは読まない）。

- **マージの机は取らない。** `release` は roadmap も台帳も触らず、main が動いたときの取り込みと確かめ直し（4.2）を手順の中にすでに持つ。`merge`・`merged` は呼ばない。
- **机は助けである。** 机が使えないとき（下の `desk: skip …`・直らない失敗）は、その回は机の段を飛ばして手順を続け、訳を段 7 の報告に書く。机のせいでリリースを止めない。
- **机の待ちは止まりではない。** 番待ちと再開待ちの間は 1 行（`排他テストの机の番待ち`・`止まった（停止要請: <識別>）`）だけ伝え、開発者に承認を求めない（「止まり方の決まり」）。待ちのコマンドが終わればそのまま続ける。
- 見回り・時間を決めた繰り返し・定期の `status` はしない。`clear` は呼ばない。ほかのセッションの見張りや待ちのプロセスを止めない。`AREKA_IMPL_WATCH_HOME` を書き替えない（`/c/…` への読み替え・`cd`・先頭への `AREKA_IMPL_WATCH_HOME=…` もしない）。

### 前置き（段 3 に入る前に 1 回）

次の 1 回の Bash（前景）で、識別とリポジトリを求める。4.2 の 4 から始める再開（「再開のときの事実の読み方」）や、値を覚えていないときも、使う前に同じものを 1 回回す（同じ値が出る）。

```bash
ID=$(basename "$(git rev-parse --show-toplevel)"); R=$(basename "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")")
if [ -z "$AREKA_IMPL_WATCH_HOME" ]; then echo "desk: skip no-home"
elif [ ! -f "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" ]; then echo "desk: skip no-exe"
elif ! [[ "$ID" =~ ^[A-Za-z0-9._-]{1,100}$ && "$R" =~ ^[A-Za-z0-9._-]{1,100}$ ]]; then echo "desk: skip bad-name $ID $R"
else echo "desk: id=$ID repo=$R"; fi
```

- `desk: id=<ID> repo=<R>`: 2 つの値を覚え、以後のコマンドへ字のまま書く（Bash の変数は次の呼び出しに残らない）。
- `desk: skip no-home|no-exe|bad-name`: 別の値をこしらえず、その回は机のコマンドを 1 つも呼ばない。次の 1 行を伝え、段 7 の報告にも書く。
  - `no-home`: `机の段を飛ばしました（環境変数 AREKA_IMPL_WATCH_HOME が空）`
  - `no-exe`: `机の段を飛ばしました（置き場所に areka-impl-watch.exe が無い）。置くには: <置き方のコマンド>`。置き方のコマンドは `pwsh -NoProfile -File <本体の作業コピー>\tools\impl-watch-install.ps1` で、`<本体の作業コピー>` は本体の作業コピーの絶対パス（`git rev-parse --path-format=absolute --git-common-dir` の親を `C:\…` の形にした物）。手順の側からは回さない。
  - `bad-name`: `机の段を飛ばしました（作業木かリポジトリの名前が識別の形に合わない: <ID> <R>）`

### 呼ぶ形

どのコマンドも次の形で呼び、最後の `exit=<n>` とその前の 1〜2 行だけを読む（`2>&1` で標準エラーの 1 行も同じ出力に入る）。

```bash
"$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" <コマンド> <引数> 2>&1; echo "exit=$?"
```

- バックグラウンド（Bash の `run_in_background`）: `watch`・`request --wait`・`stopped --wait`・`resume`。立てたら、その種類と走りの番号を覚える（自分の待ち）。
- 前景: `return`・`leave`・`cancel`。`status` は訳を調べるときだけ。
- `release` が呼ぶコマンド（`{版}` は置き換える）:
  ```bash
  "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" watch --id <ID> --repo <R> --name release-v{版} 2>&1; echo "exit=$?"
  "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" request --id <ID> --repo <R> --desk test --purpose "release {版} 3.1" --wait 2>&1; echo "exit=$?"
  "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" request --id <ID> --repo <R> --desk test --purpose "release {版} 3.3" --wait 2>&1; echo "exit=$?"
  "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" return --id <ID> 2>&1; echo "exit=$?"
  "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" leave --id <ID> 2>&1; echo "exit=$?"
  "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" stopped --id <ID> --wait 2>&1; echo "exit=$?"
  "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" resume --id <ID> 2>&1; echo "exit=$?"
  "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" cancel --id <ID> 2>&1; echo "exit=$?"
  ```
  番は `granted test` で来る。`return` は名指しなしで、持っている机を全部返す（`returned <机>`）。

### 流れ

1. **参加**: 3.1 の前に `watch` をバックグラウンドで立てる。
2. **3.1**: `cargo metadata` の確かめは机なしで回す。全体テストの前に `request … --purpose "release {版} 3.1" --wait` をバックグラウンドで回し、`排他テストの机の番待ち` を 1 行伝える。`granted test` で終わってから全体テストを回し、終わりの知らせを受けたら（成否によらず）、判定より先に `return` を前景で回す。
3. **3.2**: 机を持たずに読む。
4. **3.3**: 起動の確かめの前に `request … --purpose "release {版} 3.3" --wait`、`granted test` で終わってから回し、終わりの知らせを受けたら（成否によらず）、判定より先に `return`。
5. **参加を終える**: 3.3 の `return` の後に `leave` を前景で回す。3.4・3.5 は机を持たず、参加者でもない。
6. **4.2 の回し直し**: 4 で 3.1 を回し直すときは、そのときにまた `watch` → `request … --purpose "release {版} 3.1" --wait` → 番が来てから全体テスト → `return` → `leave`。
- **途中で止まるとき**（段 3 の欠け・4.2 の止まり・「この文書に無い状態」など、手順のどこで止まっても）: 机を持っていれば `return`、参加していれば `leave` を回してから止まる。机の申し込みや待ちを残したまま報告しない。
- 机を持つのは走行の間だけ。赤の後の調べ・直しの相談の間は持たない（この手順は回し直しを自分で決めない）。

### 停止要請

見張りが `exit=0` `stop requested by <X>` で終わったら、ほかのセッションの負荷テストのための停止要請である。

- 走っている確かめ（机を持つ走行なら、その終わりの知らせと `return` まで、机の要らない項目ならその項目の判定まで）を終えた切れ目で、次の項目に入らずに `stopped --id <ID> --wait` をバックグラウンドで回し、1 行 `止まった（停止要請: <X>）` を伝える。走っているテストを途中で止めない。
- 番待ちの間に受けたら（何も走っていない）、そこが切れ目である。申し込みの待ちは残したまま、直ちに `stopped --wait` を回す（止まった者には番が来ず、再開と同じ呼び出しで来る）。
- 再開の待ちが `resumed` で終わったら、`watch` を立て直してから次の項目へ進む。開発者の入力は待たない。

### 終わり方の扱い

| 終わったもの | 出力 | すること |
|---|---|---|
| `watch` | `exit=0`、`stop requested by <X>` | 上の「停止要請」 |
| `watch` | `exit=0`、`already stopped; …` | 「止まった」を記録し直さず、`resume` をバックグラウンドで回す |
| `watch` | `exit=1`、`AREKA_IMPL_WATCH_HOME …` | その回の机の段をやめて続ける。標準エラーの 1 行をそのまま伝え、段 7 に書く |
| `watch` | `exit=1`、`a watch wait for <ID> is already running` | 自分の見張りが走っていれば何もしない。覚えが無ければ前のセッションの残りなので、下の「残りの見張りの外し方」をバックグラウンドで回し、1 行伝える |
| `watch` | `exit=1`、ほかの文 | 直ちに 1 回だけ立て直す。立て直しも 1 で終わったら `doc/impl-watch.md` 5 節の終了コード 1 の表に従い、直らなければ机の段をやめて続ける |
| `watch` | `exit=3` | 最後に立てた見張りでない・参加を終えた（`leave`）後なら何もしない。それ以外は 1 行伝えて `watch` を立て直し、番を待っていた申し込みがあれば申し込み直す。机を持って走行中だったなら申し込み直さない（走っている確かめは止めず、終わったら `return`。返しが 3 でもそのまま続ける） |
| `stopped --wait`・`resume` | `exit=0`、`resumed` | `watch` を立て直して次の項目へ |
| `stopped --wait`・`resume` | `exit=0`、`stop requested again by <X>` | 次の項目に入らず、もう一度 `stopped --wait` |
| `stopped --wait`・`resume` | `exit=3` | `watch` を立て直して続ける |
| `stopped --wait`・`resume` | `exit=1`、`a resume wait for <ID> is already running` | 前の再開の待ちがまだ走っている。何もしない |
| `request --wait` | `exit=0`、`granted test` | 番が来た。その走行を 1 回回し、終わったら（成否によらず）直ちに `return` |
| `request --wait` | `exit=3`、`not applied: desks are held; only a merge holder may add the test desk` | 机を持ったまま申し込んだ。状態は変わっていない。`status` は読まずに `return` してから申し込み直す。1 行伝える |
| `request --wait` | `exit=3`、`not applied: a different desk request is already waiting` | `release` は `test` しか申し込まないので、同じ識別の前のセッションの残りである。`cancel --id <ID>` を前景で回し、直ちに申し込み直す。1 行伝える |
| `request --wait` | `exit=3`、`not applied: a merge request is waiting; …` | 同じ識別でマージの机を待つ者が居る（`release` はマージの机を取らない）。`cancel` はしない（相手のマージの申し込みまで消える）。その回の机の段をやめて手順を続け、1 行伝えて段 7 に書く |
| `request --wait` | `exit=3`、`gone: request gone` | 自分で `cancel` した後なら何もしない。それ以外は 1 行伝え、先に `watch` を立て直してから申し込み直す |
| `request --wait` | `exit=1`、`a request wait for <ID> is already running` | 自分の待ちが走っていれば何もしない。覚えが無ければ `cancel --id <ID>` の後に申し込み直す。1 行伝える |
| `return` | `exit=3`、`not applied: not the holder of that desk`／`not applied: holds no desk to return` | 机はもう自分の物でない（記録が消えた など）。返し直さず、1 行伝えて続ける |
| `return`・`leave`・`cancel` | `exit=1`、下の行のほか | 標準エラーの 1 行をそのまま伝え、`doc/impl-watch.md` 5 節の終了コード 1 の表に従う。直らなければ机の段をやめて続け、段 7 に書く |
| どれでも | `exit=1`、`state.lock is busy …` | 間を置かずに 1 回だけ呼び直す。続けば机の段をやめて続け、段 7 に書く |
| どれでも | `exit=2` | 呼び方の誤り＝この節の文の誤り。机の段をやめて続け、段 7 に書く |

- 立て直しと申し込み直しは、同じ出来事で待ちが 2 つ続けて終わっても 1 回ずつ。2 つ目が `already running` で終わっても害は無い。
- 終わり方は出力の文で読む。「自分の見張り・待ちか」の覚えは、`already running` の自分か残りかの分け方にだけ使う。覚えを失ったら残りとして扱う。
- 「机の段をやめて続ける」とき: 待ちが走っていれば放っておき（記録が消えれば 3 で終わる）、以後はその回の机のコマンドを呼ばない。走行は机なしで回す。

**残りの見張りの外し方**: 前のセッションの見張りは、次の見回り（1 秒ごと）で自分の記録が消えたと知るまで識別の見張りの錠を持つ。外して、待って、立て直すまでを 1 回のバックグラウンドの Bash で行う。

```bash
"$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" leave --id <ID> 2>&1; sleep 3; "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" watch --id <ID> --repo <R> --name release-v{版} 2>&1; echo "exit=$?"
```

- 最後の `exit=` と見張りの行だけを読む。これを最後に立てた見張りとして覚える。この `leave` は参加の終わりに数えない。待っていた申し込みは、この 1 本が終わる前に申し込み直す（`leave` で消えるため）。
- これがまた `already running` で終わったら繰り返さず、机の段をやめて続け、段 7 に書く。

## 段 3: 手元で確かめる

上から順に、1 つずつ回す（同時に回さない。`cargo` の置き場と記憶を取り合う）。3.1 の全体テストと 3.3 の起動の確かめは、「机の段」のとおり排他テストの机を取って回す（入る前に机の段の前置きを 1 回）。どれかが欠けたら、版上げの PR を出さずに止まり、欠けた確かめの名前を報告する。作業木はそのまま残し、消したり戻したりしない。

### 3.1 lock の整合と全体テストを回す

- `cargo metadata --locked --no-deps --format-version 1` が終了コード 0 であること。出力の `packages` の数を `N` として覚える。
- 机の段の「流れ」の 1・2: `watch` を立て、`request --desk test --purpose "release {版} 3.1" --wait` の番が来てから次を回す。
- `pwsh -NoProfile -File tools/test-all.ps1 -License` を裏で回し、終わりの知らせを待つ（約 19 分）。`-Format` は付けない。付けなくても `cargo fmt --all -- --check` は回る。`-License` は `THIRD-PARTY-NOTICES.md` を作り直す。
- 終わりの知らせを受けたら（成否によらず）、判定より先に `return`。
- 判定: 2 つとも終了コード 0。

### 3.2 版上げの差分の範囲を判定する

主文脈が `git diff --numstat HEAD` と、ファイルごとの `git diff -U0 HEAD -- {ファイル}` を読んで、下の表に当てはめる。変更かどうかは中身の差分（`git diff`）で見て、`git status` の印では見ない（改行だけの違いは数えない）。数は決め打ちにせず、3.1 の `N` を使う。

| 見る物 | コマンド | 当たりの条件 |
|---|---|---|
| 動いたファイルの集まり | `git diff --numstat HEAD` | `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` の 3 つが在り、ほかに在ってよいのは `dist/README.txt` だけ。その版の見どころのファイルは、まだ git に入っていない新しいファイルなので、ここには出ず、次の行に出る |
| 新しいファイル | `git status --porcelain` の `??` の行 | `doc/release-notes/v{版}.md` の 1 行だけ（ほかの物は 1 行も無い） |
| `Cargo.toml` | `git diff -U0 HEAD -- Cargo.toml` | 足した行 2・消した行 2。`[workspace.package]` の `version` の行と `dola` の行で、どちらも `{旧版}` が `{版}` になっただけ |
| `Cargo.lock` | `git diff -U0 HEAD -- Cargo.lock` | 消した行は全部 `version = "{旧版}"`、足した行は全部 `version = "{版}"` で、どちらも `N` 行。それ以外の行（外部のクレートの行を含む）は 0 行 |
| `THIRD-PARTY-NOTICES.md` | `git diff -U0 HEAD -- THIRD-PARTY-NOTICES.md` | 消した行は全部「`- {ワークスペースのクレートの名前} {旧版}`」、足した行は全部同じ名前の「`{版}`」で、数が同じ。それ以外の行は 0 行 |
| `dist/README.txt` | `git diff -U0 HEAD -- dist/README.txt` | 足した行 1・消した行 1 で、「時点」の行の日付だけ（同じ日に 2 回目の版上げなら 0 行でよい） |

- 判定: 表の全部の行が当たる。1 つでも当たらなければ、その変更を取り込まずに止まり、当たらなかった行を開発者へ示す。

### 3.3 起動の確かめ（x64）を回す

- 机の段の「流れ」の 4: `request --desk test --purpose "release {版} 3.3" --wait` の番が来てから次を回す。
- `pwsh -NoProfile -File tools/package.ps1 -Check` を裏で回し、終わりの知らせを待つ（窓を出す）。3.1 と同時に回さない。
- 終わりの知らせを受けたら（成否によらず）、判定より先に `return`、続けて `leave`（机の段の「流れ」の 5）。
- 判定: 終了コード 0。

### 3.4 公開前の確かめを回す

- `pwsh -NoProfile -File tools/crates-io.ps1 -Verify -Version {版}` を回す。
- 判定: 終了コード 0。赤なら、赤の理由も報告する。

### 3.5 見どころの検査を回す

- `pwsh -NoProfile -File tools/release-notes.ps1 -Check -Version {版}` を回す（読むだけで、すぐ終わる。何も書き換えない）。
- 同じ検査を Release の走りが組み立ての前に回し、欠けていれば何も公開せずに止まる。タグは打った後に動かせないので、PR を出す前にここで通す。
- 判定: 終了コード 0。赤なら、道具が印字した欠けの名前も報告する。

## 段 4: 版上げの PR を出し、squash マージする

### 4.1 版上げをコミットし、PR を出して中身を読み直す

- `git add Cargo.toml Cargo.lock THIRD-PARTY-NOTICES.md dist/README.txt doc/release-notes/v{版}.md` と名前を挙げて足す。題 `chore(release): v{版}` でコミットする。
- `git push --quiet {remote} HEAD` の後、`gh pr create --base main --title "chore(release): v{版}"`。本文に版・5 ファイル（同じ日に 2 回目の版上げなら、`dist/README.txt` を除く 4 つ）の `--numstat`・3.1〜3.5 の結果・見どころのファイルの所在（`doc/release-notes/v{版}.md`）を書く。
- `gh pr view {番号} --json files` で PR のファイルを読み直す。上の 5 ファイル（同じ日に 2 回目の版上げなら 4 つ）だけであること。違えば止まる。
- 判定: PR が開いていて、ファイルの集まりが上の条件に当たる。

### 4.2 main が動いていたら取り込んで確かめ直す

- `git fetch --quiet {remote} main` の後、先端が「確かめた main」の SHA と同じなら何もしない。
- 違えば、次を上から行う。
  1. 新しく main に入った変更の題名を `git log --first-parent --format=%s {確かめた main}..{remote}/main` で読んで控える（`{確かめた main}` は覚えてある SHA。覚え直す前に読む）。
  2. `git merge {remote}/main` で取り込む。衝突したら止まる。
  3. 1 で控えた題名が 1 件でも在れば、2.3 の一覧を、先を `-To {remote}/main` にして出し直し、新しく入った変更に `doc/release-notes/README.md` の「必ず載せるもの」に当たる物が在るかを読む。在れば見どころのファイルに足し、3.5 を回し直して、`git add doc/release-notes/v{版}.md` だけを足したコミットを 1 つ作る（題は `chore(release): v{版} の見どころの直し`。版上げの PR に載り、squash で 1 つのコミットになる）。無ければ何もしない。どちらでも止まらない。控えた題名と、足したかどうかを最後の報告に書く。
  4. 3.1 を回し直す。全体テストは机の段の「流れ」の 6 のとおり `watch` → `request --desk test --purpose "release {版} 3.1" --wait` → 番が来てから回す → 終わったら（成否によらず）`return` → `leave`。後で作業木がきれいでなければ、コミットせずに止まって報告する（回し直した `-License` が `THIRD-PARTY-NOTICES.md` を変えた＝main に依存の変更が入った）。
  5. 3.2 を回し直す。版上げはもうコミット済みなので、比べる相手を読み替える: 表の `git diff … HEAD` を `git diff … {remote}/main HEAD` にする。見どころのファイルはコミット済みなので、「動いたファイルの集まり」に足したファイルとして出て、`??` の行は 1 行も無い。
  6. `git push --quiet {remote} HEAD` の後、`gh pr view {番号} --json files` が 4.1 と同じファイルの集まりを返すことを確かめ、新しい先端を「確かめた main」として覚え直す。
- 再開で「確かめた main」を覚えていないとき（「再開のときの事実の読み方」）は、main が動いていなくても 2〜6 を 1 回行う。3 は題名の有無によらず行う。
- 判定: `{remote}/main` の先端が「確かめた main」の SHA と同じ。

### 4.3 squash マージする

- マージの直前に main をもう一度読み（`git fetch --quiet {remote} main`）、動いていたら 4.2 に戻る。
- `gh pr merge {番号} --squash`（`--delete-branch` は付けない。枝は、公開の後に見どころの直しを頼まれたときに使う）。
- 判定: `gh pr view {番号} --json state,mergeCommit` が `MERGED` を返す。squash のコミット `{sha}` を覚える。マージの成否はこの読み直しだけで決める。

## 段 5: タグを打つ

### 5.1 タグを打つコミットを確かめる

- `git fetch --quiet {remote} main` の後、`git merge-base --is-ancestor {sha} {remote}/main` が 0 であること。
- `git show {sha}:Cargo.toml` の 2 行の版が `{版}` と 1 字違わず同じであること。タグの名前は `v{版}`。
- `git ls-remote --quiet --tags {remote} refs/tags/v{版}` が 0 行であることを、もう一度確かめる。
- 手元のタグを読む。`git rev-parse --verify --quiet refs/tags/v{版}^{commit}` が失敗する（手元に無い）か、`{sha}` を返すこと。別のコミットを返したら止まる（手元のタグも消さない）。
- 判定: 4 つとも当たる。外れたら止まって報告する。

### 5.2 タグを打って push する

- 手元に `v{版}` が無ければ `git tag v{版} {sha}`（注釈なし）。5.1 で `{sha}` を返した回は、打ち直さない。
- push の前に、`git rev-parse --verify --quiet refs/tags/v{版}^{commit}` が `{sha}` を返すことを確かめる。返さなければ push せずに止まる。
- `git push --quiet {remote} refs/tags/v{版}`。タグは名前を指して 1 本だけ押す。
- タグを打つのは squash のコミットで、その後に main へ入ったコミットには打たない。`{sha}` を名指しするので、main が先へ進んでいても変わらない。
- 判定: `git ls-remote --quiet --tags {remote} refs/tags/v{版}` が `{sha}` を返す。

## 段 6: 見守る

### 6.1 2 本の走りを見つけて裏で待つ

- `gh run list --workflow release.yml --event push --json databaseId,headSha,headBranch,status,conclusion` から、`headBranch` が `v{版}` で `headSha` が `{sha}` の走りを取る。`crates-io.yml` も同じ。数分待っても見つからなければ報告する。
- 走りごとに `gh run watch {id} --exit-status --interval 60` を裏で回し、終わりの知らせを待つ。Release の走りは最長 150 分、公開の走りは Release の緑を最長 120 分待ってから出す。裏の待ちが上限で切れたら掛け直す。走りの状態は `gh run view {id} --json status,conclusion` でいつでも読み直せる。
- 判定: 2 本の走りの番号が取れ、どちらも終わった（`gh run view {id} --json status` が `completed`）。

### 6.2 緑を判定する

| 相手 | やり方 | 緑の条件 |
|---|---|---|
| Release の走り | `gh run view {id} --json conclusion` | `success` |
| GitHub Release | `gh release view v{版} --json isDraft,assets,url,body` | `isDraft` が `false`。添わった物がちょうど 5 つ＝`areka-{版}-x64.zip`・`areka-{版}-x64.zip.sha256`・`areka-{版}-arm64.zip`・`areka-{版}-arm64.zip.sha256`・`release-notes-v{版}.md`。`body`（本文）が、タグのコミットの見どころのファイル（`git show {sha}:doc/release-notes/v{版}.md`）の中身で始まる |
| 公開の走り | `gh run view {id} --json conclusion` | `success` |
| crates.io | `pwsh -NoProfile -File tools/crates-io.ps1 -Pending -Version {版}`（作業木の版が `{版}` のまま回す） | 終了コード 0 で、標準出力が 0 行 |
| winget | `git cat-file -e {sha}:.github/workflows/winget.yml` が 0 のときだけ、winget-pkgs へのその版の PR を `gh pr list --repo microsoft/winget-pkgs` で探す | PR が在る |

- 本文の比べ方: どちらも改行を LF にそろえ、見どころのファイルの末尾の空白と空行を除いて、バイトの並びで比べる。`git show` と `gh` の出力を、端末の文字コードを通して読まない（文字コードが 932 の端末では日本語が壊れ、偽の赤になる）。出力はファイルへ流してから読む（`gh … --jq .body` は末尾に LF を 1 つ足す）。
- 見どころの直しの PR（題 `docs(release): v{版} の見どころの直し`）が `MERGED` の版は、本文を比べる相手を `{remote}/main` のファイル（`git show {remote}/main:doc/release-notes/v{版}.md`）にする。
- `winget.yml` がタグのコミットに無い回は、winget を見守りの相手に入れない。
- 判定: 表の全部の行が緑。Release の URL（と在れば winget の PR の URL）を覚える。赤の行が在れば 6.3 へ。

### 6.3 赤のときは決まりを当てはめ、開発者に決めてもらう

見えた事実を下の表に当てはめて示し、止まる。やり直すかどうかは、どの行でも開発者が決める。自分では決めない。

| 見えたこと | 当てはめ | してよいこと |
|---|---|---|
| Release の走りが赤 | その版の Release が残っていない（下書きも無い）、かつ原因が通信の失敗・時間切れ・取り消し | 開発者が決めたら、同じ走りを「Re-run」する（`gh run rerun {id}` か画面） |
| Release の走りが赤 | 上の 2 つのどちらかを満たさない。または、タグのコミットの直しが要る | 同じ版で出し直さない。タグを動かさない。原因を直す spec を `/kiro-discovery` で起票し、直したら次の版で出す |
| 公開の走りが赤 | `doc/crates-io-publish.md` 5 節の切り分けに当てはめる | 同じ版のまま「Re-run」か「Run workflow」で残りを出してよい。次の版で出し直すのは、タグのコミットのコードや公開前の確かめのスクリプトの誤りのときだけで、そのときは起票する |
| 公開の走りだけが赤 | Release の走りは `release.yml` が固定した版の Rust、公開の走りは最新の安定版 | 版の差を先に疑う |
| Release の公開より後の段（winget など。公開の走りを除く）が赤 | — | 同じ版で出し直さない。原因を直す spec を起票する |

- 起票は、開発者の判断を得てから、主文脈がこの作業の枝で `/kiro-discovery` を回す。起票のコミットは、この枝から起票だけの PR として出す（マージは開発者の指示で）。`/kiro-discovery` が brief と roadmap に書く分は、起票の成果物で、この手順の「roadmap を書き換えない」には当たらない。起票の PR が main に入るまで、見どころの直しは待つ（枝に起票のコミットが残っていると、直しの PR に混ざる）。
- 判定: 赤が 0 件、または赤のすべてに開発者の判断があり、やり直しの結果が緑か、起票した spec の名前を覚えた。赤が無い回は何もしない。

## 段 7: 報告する

日本語で、次を 1 つの報告にまとめて終える。記録のコミットは作らず、roadmap も書き換えない（記録は GitHub の Release の頁。6.3 の起票で `/kiro-discovery` が書く分は除く）。

- 版・タグのコミット `{sha}`・GitHub Release の URL。
- Release の走りと公開の走りの結果（在れば winget の PR の URL・赤で起票した spec の名前）。
- 版上げのやり方（いつものやり方か代わりのやり方）。
- 途中で main を取り込んだ回は、取り込んだ題名と、見どころに足したかどうか。
- 使った申し送りと、「消してよい申し送り」（1.4）。
- 途中で気付いた事（範囲の外の問題は、開発者の判断を得て `/kiro-discovery` で起票する）。
- 机の段を飛ばした・途中でやめたときは、その訳（`desk: skip …` の訳や、出た文）。
- **見どころの全文**: `doc/release-notes/v{版}.md` の中身をそのまま載せる。開発者はここで読む。「直しが要れば言ってください。公開の頁と main のファイルを直します」と添える。

## 公開の後に見どころの直しを頼まれたら

開発者が最後の報告を読んで直しを頼んだときだけ行う。タグのコミットの中のファイルは直せない（タグは動かさない）。直すのは、公開の頁の本文と、main のファイルの 2 つ。

1. `git fetch --quiet {remote} main` の後、`git merge {remote}/main` で作業の枝に main を取り込む。
2. `doc/release-notes/v{版}.md` を直し、`pwsh -NoProfile -File tools/release-notes.ps1 -Check -Version {版}` が終了コード 0 であること。
3. 頁の本文を作り直す。`pwsh -NoProfile -File tools/release-notes.ps1 -Body -Version {版} -Repository ekicyou/areka -Out target/release-notes/release-body-v{版}.md`（リリース CI が本文を作るのと同じ道具。末尾の区切りの線と添付へのリンクの文も付く）。
4. `gh release edit v{版} --notes-file target/release-notes/release-body-v{版}.md`。
5. `git add doc/release-notes/v{版}.md` だけを足し、題 `docs(release): v{版} の見どころの直し` でコミットする。`git push --quiet {remote} HEAD` の後、`gh pr create --base main --title "docs(release): v{版} の見どころの直し"`、`gh pr view {番号} --json files` でその 1 ファイルだけであることを確かめて、`gh pr merge {番号} --squash`。
6. 判定: `gh release view v{版} --json body` の本文が、直した見どころのファイルの中身で始まる（比べ方は 6.2 と同じ）。PR が `MERGED`。

## 再開のときの事実の読み方

進み具合はどのファイルにも書いていない。`/release` を打たれたら、段 1 より前に必ず次を読み、取り返しのつかない操作（マージ・タグの push）を二度しないよう、始める位置を事実から決める。セッションが切れたときは、同じワークツリーで `/release` を打ち直す。

**見る PR**: `gh pr list --head {枝} --state all --json number,state,mergeCommit,title` のうち、題が `chore(release): v` で始まる物だけ（同じ枝から出したほかの PR は数えない）。その中で版がいちばん新しい 1 本を「この枝の版上げの PR」とし、その題の版を `{版}` とする。先に `git fetch --quiet {remote} main` を済ませ、上から順に当てはめて、最初に当たった行に従う。

| 見えた事実 | 始める位置 |
|---|---|
| この枝の版上げの PR が無く、作業木の変更が版上げの 5 つのファイルの内側だけで、根の `Cargo.toml` の 2 行が同じ版 | PR を出す前に切れた回。`{版}` は作業木の `Cargo.toml` の版、`{旧版}` は `git show HEAD:Cargo.toml` の版。「確かめた main」は `git merge-base {remote}/main HEAD`（枝が取り込んである main）。1.2 と、1.3・1.5 のタグの確かめ（`v{旧版}` が在り、`v{版}` が無い）を回してから、2.2 から続ける（見どころのファイルが在れば書き直さず、2.3 の読み直しと `-Check` から。段 3 が全部を判定し直す） |
| この枝の版上げの PR が無く、作業木がきれいで、先頭のコミットの題（`git log -1 --format=%s`）が `chore(release): v` で始まる | コミットの後・PR の前に切れた回。`{版}` は `git show HEAD:Cargo.toml` の版。4.1 の push から。「確かめた main」は覚えていない物として扱う |
| この枝の版上げの PR が無く、上の 2 行に当たらず、`git status --porcelain` が 0 行 | 段 1 から（新しい回） |
| この枝の版上げの PR が開いている | `git status --porcelain` が 0 行でなければ止まる。`{旧版}` は `git show {remote}/main:Cargo.toml` の版。「確かめた main」は覚えていない物として、4.2 から（4.2 の 2〜6 を必ず 1 回行う。3.1 を回していない main との組み合わせをマージしない） |
| この枝の版上げの PR が `MERGED` で、`git ls-remote --quiet --tags {remote} refs/tags/v{版}` が 0 行 | 段 5 から（段 2〜4 をやり直さない）。`{sha}` はその PR の `mergeCommit`。手元にだけタグが在るときの扱いは 5.1 と 5.2 が持つ |
| この枝の版上げの PR が `MERGED` で、リモートのタグが在る。引数の版の指示は無いか、`{版}` と同じ | 段 6 から。`{sha}` はその PR の `mergeCommit` で、リモートのタグがそれを指すことを確かめる（違えば止まる）。6.1 で走りを読み直し、6.2 を当てる。全部緑なら段 7 の報告だけを行って終える。**新しい版は始めない** |
| この枝の版上げの PR が `MERGED` で、リモートのタグが在る。引数で `{版}` より新しい版が指示されている | 前の回の 6.2 を先に当て、全部緑のときだけ、指示の版の新しい回として段 1 から。緑でなければ、前の回の段 6 から |

- 上のどれにも当たらなければ、止まって、見えた事実（PR・作業木の変更・枝の差分）を報告する。
- マージの後、日が空いても、タグを打つ相手は同じ squash のコミットのまま変わらない。
- 頼みが公開済みの版の見どころの直しのときは、この表を当てはめず、「公開の後に見どころの直しを頼まれたら」だけを行う。`{版}` は開発者の言う版。言われなければ、この枝の版上げの PR の版。

## 見直すきっかけ

次が変わったら、この文書を見直す。

- `release.yml`・`crates-io.yml` のきっかけ（タグ `v*` の push）・待ちの上限・Release に添える物の数（今は 5 つ）・Release の本文のもと（今はその版の見どころのファイル）。
- 版の置き場（今は根の `Cargo.toml` の 2 行だけ）。3 行目が増えたら段 2 と 3.2 を直す。
- `tools/test-all.ps1`・`tools/package.ps1`・`tools/crates-io.ps1`・`tools/release-notes.ps1` の引数の名前や終了コードの意味。
- `areka-impl-watch.exe` のコマンドの名前・引数・出力の 1 行・終了コードの意味（`doc/impl-watch.md`）。変われば「机の段」の呼ぶ形と終わり方の表を合わせ直す。
- 見どころのファイルの置き場所・名前（今は `doc/release-notes/v{版}.md`）。
- `winget.yml` が main に入る（6.2 の見守りの相手が増える）。
- この文書の節の名前（「再開のときの事実の読み方」「止まり方の決まり」「机の段」「公開の後に見どころの直しを頼まれたら」「開発者の設定」）。`SKILL.md` が名前で指している。

## 開発者の設定（初めて打つ前に）

この手順は、開発者に聞かずに `gh pr merge` とタグの push を行う。自動モードの見張りがこれを止めるときは、開発者の設定（`C:\Users\maz-o\.claude\settings.json` の `autoMode.allow`）に、次の文のルールを足してもらう。手順の側から設定は触らない。

> Release: when the user's message is the `/release` command (optionally with a version), the agent may push its harness worktree branch, create the version-bump PR with `gh pr create --base main` titled `chore(release): v{version}`, squash-merge that PR with `gh pr merge --squash`, create the tag `v{version}` on that squash commit and push only that tag (`git push <remote> refs/tags/v{version}`). When the user then asks for a fix of the highlights of that version, the agent may also run `gh release edit v{version} --notes-file` and create and squash-merge a PR that changes only `doc/release-notes/v{version}.md`. This does not cover `--admin`, bypassing required checks, merging any other PR, pushing directly to main, `--force`, `--tags`, moving or deleting a pushed tag, `gh run rerun`, or `gh release delete`.

外へ出るコマンド（`git push`・`gh pr create`・`gh pr merge`・`gh release edit`・`gh run rerun`）は、1 回のツール呼び出しに単独の 1 行で打つ。`cd … &&` や `; echo` とつなぐと、許可の規則（`Bash(gh pr merge:*)` など）に当たらず、断られる。
