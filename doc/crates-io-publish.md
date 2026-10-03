# crates.io への公開の手順

リリースのたびに、汎用のライブラリ `wintf`・`dola` を、リリースと同じ版で crates.io へ出す。ふだんは GitHub Actions の公開の段（`.github/workflows/crates-io.yml`）が自動で出すので、開発者がすることは、最初に一度だけの Trusted Publishing の設定と、止まったときのやり直しだけである。

crates.io に一度出した版は、消すことも差し替えることもできない（できるのは取り下げ＝yank だけ）。公開の段も本書の手順も、上げる前に確かめて止まる形にしてある。

## 1. 何を出すか

- 出すのは `wintf`（Windows の窓・描画・縦書きの土台）と `dola`（宣言的なアニメーションの記述と再生）の 2 つだけ。どちらも areka の外でも使える汎用のライブラリで、crates.io に 0.0.1 が既に在る。
- `wintf` は `dola` に依存するので、出す順は `dola` → `wintf`。順番は cargo が決めるので、手で並べない。
- ほかのクレート（`areka` 本体と部品）は出さない。各クレートの `Cargo.toml` に `publish = false # 理由` の行がある。crates.io の `areka` は名前の確保（0.0.1）だけで、利用者は配布の zip で入れる。
- 何を出すかの決め方と、新しいクレートを足すときの決めごとは `.kiro/steering/tech.md` の「crates.io への公開」の節に在る。

## 2. Trusted Publishing の設定

Trusted Publishing は、GitHub Actions から、長く使える鍵を置かずに crates.io へ出す仕組みである。公開の段はこれだけで認証する。

**最初の自動の公開（初回のリリースのタグ `v0.0.2`）より前に、`wintf`・`dola` の両方に済ませておく。** 済んでいないと、公開の段は段「鍵」で止まる（何も上がらない）。

crates.io に持ち主のアカウントでログインし、`wintf` と `dola` のそれぞれで次を行う。

1. クレートの頁の **Settings** を開き、**Trusted Publishing** の欄で追加を選ぶ。
2. 発行元に **GitHub** を選び、次のとおり入れる。

   | 欄 | 入れる値 |
   |---|---|
   | Repository owner | `ekicyou` |
   | Repository name | `areka` |
   | Workflow filename | `crates-io.yml` |
   | Environment name (optional) | 空のまま |

3. 保存し、一覧に 1 行増えたことを確かめる。

environment を空にするのは、公開の段が GitHub の environment（人の承認を挟む仕組み）を使わないためである。workflow のファイル名を変えたら、この設定も直す。

## 3. いつもの流れ

1. 版を上げる PR を squash マージし、`v{版}` のタグを打って push する。
2. タグの push で、Release を作る workflow（`release.yml`）と公開の段（`crates-io.yml`）が同時に動き出す。`release.yml` は公開の段を呼ばない。公開の段がタグの push を自分で受ける。
3. `release.yml` が、組み立て・zip・GitHub Release の公開を行う。その間、公開の段は段「release を待つ」で、同じタグの `release.yml` の回が緑で終わるのを待つ（30 秒ごとに見て、最長 45 分）。
4. 公開の段が、次の段を上から順に行う。どれかが失敗したら、そこから先は「記録」以外を飛ばす。

| 段 | すること |
|---|---|
| 版の形 | タグが `v` と数字 3 つの形か（手で起動したときは、入力 `version` が数字 3 つの形か）確かめる |
| release を待つ | タグの push のときだけ。同じタグの `release.yml` の回が緑で終わるのを待つ。赤で終わった・45 分で緑にならない・`release.yml` が無いときは、何も上げずに止まる |
| 取り出し | タグ `v{版}` のコミットを取り出す |
| Release の確認 | その版の GitHub Release が在って、下書きでないことを確かめる（Release より先に crates.io へ出さない） |
| 道具 | Rust を最新の安定版にする |
| 公開前の確認 | `tools/crates-io.ps1 -Verify -Version {版}`（組み立てまで確かめる。ワークスペースの版が入力と違えば止まる） |
| 残りの判定 | `tools/crates-io.ps1 -Pending -Version {版}` で、その版がまだ crates.io に無いクレートを求める |
| 鍵 | 残りが在るときだけ、Trusted Publishing で短命の鍵を受け取る |
| 公開 | 残りが在るときだけ、残りのクレートを `cargo publish` で出す |
| 記録 | 必ず走る。2 クレートのそれぞれが crates.io に在るか無いかを実行の要約に書き、残りが 0 でなければ失敗にする |

開発者は見守るだけでよい。終わったら次の節で確かめる。

`release.yml` が main に無い間（Release を作る workflow ができる前）は、タグの push で `release` の回が始まらないので、段「release を待つ」が 45 分待ってから「まだ始まっていない」と示して止まる（何も上げていない。`release.yml` が GitHub に一度も登録されていなければ「`release.yml` が無い」ですぐ止まる）。そのときは Release を手で公開してから、5 節の「Run workflow」で版を渡して起動する。

## 4. 出たことを確かめる

その版のコミット（タグ `v{版}` のコミット、または次の版上げまでの `main`）で、残りの判定を手元で走らせる。

```powershell
$left = pwsh -NoProfile -File tools/crates-io.ps1 -Pending -Version 0.0.2
"終了コード $LASTEXITCODE・残り: $left"
```

- 画面に「在る dola 0.0.2」「在る wintf 0.0.2」が出て、終了コードが 0、残りが空なら、2 クレートともその版で出ている。
- 「OK 判定「版」」や「在る・無い」の行は人が読むための行で、標準エラーに出る。標準出力（`$left`）には、まだ出ていないクレートの名前だけが 1 行ずつ出る。
- ワークスペースの版が `-Version` と違うときは、索引を読む前に二つの版を示して失敗する。違うコミットに居るので、その版のコミットへ移ってから走らせる。
- 公開の段の実行の要約（段「記録」が書く）でも、2 クレートの在る・無いを見られる。

上げた直後は、crates.io の索引への反映が少し遅れることがある。「無い」と出たら、時間を置いてもう一度走らせる。

## 5. 止まったときのやり直し

公開の段は、同じ版で何度起動し直してもよい。既に出たクレートは段「残りの判定」が飛ばすので、残りだけを出して終わる。残りが 0 なら、何も上げずに緑で終わる。

起動し直しは、GitHub の Actions の画面のボタンだけでできる。コマンドの行は要らない。

- **その回を走らせ直す（Re-run）**: Actions の画面で止まった `crates-io` の回を開き、「Re-run jobs」→「Re-run all jobs」を押す。同じタグ・同じ workflow のファイルで、最初の段から走る。
- **版を渡して起動する（Run workflow）**: Actions の画面の左の一覧で `crates-io` を選び、「Run workflow」を押す。「Use workflow from」は `main` のまま、「出す版」に `v` を付けない版（例 `0.0.2`）を入れて、緑の「Run workflow」を押す。この回は `main` の workflow のファイルで動き、段「release を待つ」は飛ばす（代わりに段「Release の確認」が、その版の Release が公開されていることを確かめる）。

コマンドの行から起動したいときは `gh workflow run crates-io.yml -f version=0.0.2` でも「Run workflow」と同じになる（使わなくてよい）。

どこで止まったかは、GitHub の Actions の画面で、その回の段ごとの結果と実行の要約を見る。止まり方ごとに、次のとおり切り分ける。

- **release が赤で止まった**（段「release を待つ」が「release が failure で終わった」などと示す）: 何も上がっていない。先に release を緑にする。緑になったら、止まった `crates-io` の回を「Re-run」する（段「release を待つ」がすぐ緑を読んで先へ進む）。「Run workflow」で同じ版を渡してもよい。release をその版でやり直さずに次の版で出すなら、この回は止まったままでよい。
- **45 分待っても release が緑にならない**: release の回が終わるのを待ち、緑で終わったら、止まった `crates-io` の回を「Re-run」する。
- **`release` の回が始まらない・`release.yml` が無い**: 3 節の末尾に従う。
- **workflow のファイルの誤り**（段の書き方・権限・入力の扱いなど）: タグの push で動いた回と、その回の「Re-run」は、**タグのコミットに在る** workflow のファイルで動く。`main` で直しても「Re-run」には効かない。`main` で直してから、「Run workflow」（`main` から）で同じ版を渡す。
- **`tools/crates-io.ps1` の誤り**: スクリプトは、取り出したタグのコミットの物が使われる。`main` で直しても、その版の起動し直しには効かない。その版は 6 節の予備の手順で出す。直したスクリプトは次の版から効く。ただし予備の手順でも、タグのコミットの同じスクリプト（`-Verify`・`-Pending`）を手元で走らせる。誤りが手元でも再現するなら、その版は予備の手順でも出せない。その場合は `main` で直して次の版で出す。
- **「公開」が緑で「記録」だけが赤**: 上げた直後で、crates.io の索引への反映を待っている。時間を置いて 4 節の確かめ方を手元で走らせるか、その回を「Re-run」する（何も上げずに緑で終わる）。
- **Release が無い・下書き**: Release を公開してから、その回を「Re-run」する（または「Run workflow」で同じ版）。
- **公開前の確認が赤**: 赤の理由で分ける。
  - 通信などの一時的な失敗（crates.io の索引を読めないなど）: 時間を置いて、その回を「Re-run」する。
  - タグのコミットのコードや設定の誤り（クレートが包めない・組み立てに失敗する・版が合わないなど）: そのコミットのままでは出せない。`main` で直し、次の版で出す。
  - スクリプトそのものの誤り: 直上の「`tools/crates-io.ps1` の誤り」に従う。

版を上げる手順に在る「赤なら同じ版で再実行しない」という決めごとは、Release を作る `release.yml` の話である。公開の段には当てはまらない。公開の段は同じ版で何度起動し直してもよい。

## 6. 予備の手順

公開の段が使えないとき（Trusted Publishing の不具合、スクリプトの誤りで同じ版をやり直せないときなど）に、手元の cargo から出す。公開する一覧に新しいクレートを足し、そのクレートが crates.io に 1 つも版を持たないとき（段「残りの判定」がその旨を示して止まる）も、この手順で最初の版を出し、続けて 2 節の設定をそのクレートにも行う。

### 必達

次のすべてを満たしてから鍵を作る。

1. 作業木がきれいで、タグ `v{版}` のコミットに居る（`git switch --detach v0.0.2` で移り、`git status --porcelain` が何も出さない）。cargo は汚れた作業木からの公開を断る。
2. 全体テストが緑。

   ```powershell
   pwsh -NoProfile -File tools/test-all.ps1
   ```

3. 組み立てまでの公開前の確認が緑。

   ```powershell
   pwsh -NoProfile -File tools/crates-io.ps1 -Verify -Version 0.0.2
   ```

### 手順

1. まだ出ていないクレートを求める（4 節と同じ）。

   ```powershell
   $left = pwsh -NoProfile -File tools/crates-io.ps1 -Pending -Version 0.0.2
   "終了コード $LASTEXITCODE・残り: $left"
   ```

   - 終了コードが 0 で残りが空なら、出す物は無い。ここで終わる。
   - 終了コードが 0 で残りが在るなら、残りの名前が出す物である。
   - 終了コードが 0 でなければ、画面の「失敗」の行を読む。
     - 新しく足したクレートが「crates.io に 1 つも版が無い」と示されたなら、そのクレートも出す物に加え、手順 4 の `-p` に足す。判定はそこで止まるので、公開する一覧のうち「在る」と出なかったクレートはすべて出す物にする。
     - それ以外の失敗（索引を読めない・版が違うなど）なら、ここで止まり、原因を直してから 1 からやり直す。
2. crates.io の **Account Settings** の **API Tokens** で、新しい鍵を作る。
   - 期限を切る（その日のうちに切れる短さ）。
   - 対象のクレートを、出すクレート（公開する一覧。新しく足すクレートがあればその名前も含める）に絞る。
   - できることは、既に在るクレートの新しい版を出すこと（publish-update）だけにする。新しいクレートの最初の版を出すときだけ、新しいクレートを出すこと（publish-new）も付ける。
3. 鍵を手元の cargo に渡す。

   ```powershell
   cargo login
   ```

   鍵は、cargo に聞かれてから貼る。`cargo login` の後ろに鍵を書かない（コマンドの履歴に残る）。
4. 1 で決めた出す物だけを出す。順番は cargo が決める。

   ```powershell
   cargo publish --locked -p dola -p wintf
   ```

   出す物の数だけ `-p` を並べる（1 つだけならその 1 つだけ。新しいクレートがあればその `-p` も足す）。cargo が `crate {名前}@{版} already exists` で断ったときは、何も上がっていない。その `-p` を外して出し直す。
5. 鍵を手元から消す。

   ```powershell
   cargo logout
   ```

6. crates.io の **API Tokens** で、作った鍵を取り消す（Revoke）。
7. 4 節の確かめ方で、2 クレートとも「在る」になったことを確かめる。

鍵は、開発者の手元の cargo の設定にだけ置く。リポジトリにも、GitHub の秘密の置き場（Secrets）にも置かない。

## 7. してはいけない操作

リポジトリの接続先の URL や認証の情報は、画面にもログにも印字しない。接続先の URL には認証の情報が含まれていることがある。次の操作は、この手順のどこでも行わない。

- `git remote -v`
- `git config --get remote.origin.url`
- `.git/config` の中身の表示
- cargo の認証のファイル（`credentials.toml`）の中身の表示
- `cargo login` の後ろに鍵を書くこと、鍵を環境変数に入れて表示すること
- crates.io の鍵を、リポジトリのファイルや GitHub の秘密の置き場（Secrets）に置くこと
