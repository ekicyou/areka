# 検体ゴーストの置き場

第三者が配布したゴースト・バルーンを、**配布形（`.nar`）のまま**使うための場所である。展開した木は追跡しない。

検体の登記（`crates/sample-ghost-kit/src/lib.rs` の `SAMPLES`）には 2 つの形がある。

| 形 | 登記の `source` | 書庫の在処 | 引く前に要ること |
|---|---|---|---|
| 書庫を置く行（今の 9 本） | `SampleSource::Vendored` | このフォルダの `<名前>.nar`（リポジトリが追跡する） | 何も要らない |
| URL の行 | `SampleSource::Fetched { url }` | 取り寄せ先の URL。手元では `target\` の下の置き場（リポジトリは追跡しない） | 取り寄せの段を 1 回回す（下の「取り寄せの段」） |

書庫を置く行の検体:

| ファイル | 種別 | 中のファイル数 | 大きさ |
|---|---|---|---|
| `emo2.nar` | ゴースト（同梱バルーン `emo2-kakukaku`） | 114 | 4,663,324 バイト |
| `R_POST_and_KOMAINU.nar` | ゴースト | 43 | 1,625,603 バイト |
| `emo2-kakukaku-offsetdpi.nar` | バルーン | 20 | 35,788 バイト |
| `emo2-kakukaku-wplimit.nar` | バルーン | 20 | 33,906 バイト |
| `konnoyayame.nar` | ゴースト（YAYA 標準テンプレート「紺野ややめ」・**配布物そのまま**） | 125 | 820,368 バイト |
| `StayseeBalloon.nar` | バルーン（areka の既定バルーン・CC0） | 29 | 73,021 バイト |
| `claudia.nar` | ゴースト（YAYA・「悪役令嬢クローディア」・同梱バルーン `claudia`／`claudia_vertical`・**配布物そのまま**） | 135 | 3,651,357 バイト |
| `emily4.nar` | ゴースト（YAYA 32bit・SSP 同梱の「Emily/Phase4.5」・同梱バルーン `emily4` は書庫に無く入らない・**受け取ったバイト列そのまま**＝下の節） | 260 | 2,677,923 バイト |
| `dot_sakura.nar` | ゴースト（華和梨 32bit・「．さくら」・同梱バルーン `bottle`・**受け取ったバイト列そのまま**＝下の節） | 204 | 1,425,534 バイト |

URL の行の検体:

| 名前 | 種別 | 置き方 | 中のファイル数 | 大きさ |
|---|---|---|---|---|
| `pasta-in-windows` | ゴースト（「窓際のぱすたさん」・同梱バルーン無し） | URL（取り寄せの段が取る＝下の節） | 版で変わる | 版で変わる |

- `StayseeBalloon.nar` の出どころとライセンス: CC0（書庫の中の `LICENSE`・`readme.txt`）。上流は <https://github.com/ponapalt/StayseeBalloon>。areka の既定バルーンで、畳み直してよく、areka の配布物へ同梱してよい。29 本のハッシュと上流との突き合わせは `.kiro/specs/completed/areka-P0-default-balloon-bundle/verification/provenance.md` の §1・§3 にある。

### `konnoyayame.nar` の出どころとライセンス

- 出どころ: <https://github.com/YAYA-shiori/konnoyayame/releases> のタグ `35438213310`（2026-09-19 10:44 UTC 公開・自動リリース）の添付 `konnoyayame.nar`。あちらは辞書のチェックが通るたびに最新版を出し直すので、**タグと次のハッシュで 1 本に固定している**。
  sha256 `e15637f1e286a016608432216dc3489e1b4a6b4691b277a29b8732c7245d7cd2`
- ライセンス（書庫の中の `README.md`「ライセンス」節と `shell/master/descript.txt` が出どころ）: 全体は Public Domain (Unlicense)。**ただしシェル `shell/master/` は SATO M 氏（サトウ Ｍ・<http://sgmh.sakura.ne.jp/tcg/>）の作品で CC BY-NC-ND 2.1 JP**（<http://creativecommons.org/licenses/by-nc-nd/2.1/jp/>）。このリポジトリの MIT ライセンスは**この 1 本には及ばない**。
- だから守ること 2 つ——**畳み直さない**（改変禁止なので `fold-samples` を通さず、ダウンロードしたバイト列をそのまま置いた。全エントリが deflate 圧縮なのはそのため）。**営利の配布物に入れない**（開発用の検体としてだけ使い、areka の配布物へ同梱しない）。
- 差し替えるとき: 新しいタグの `.nar` をそのまま置き、この節のタグ・ハッシュと上の表の 2 つの数を書き直す。
- 書庫にはゴースト作者向けの開発キット（`.claude/`・`.mcp.json`・`CLAUDE.md`・`AGENTS.md`・`tools/*.ps1`）が入っており、展開先 `ghost/konnoyayame/` にもそのまま置かれる。areka はこれらを読まない。**展開先のフォルダを作業場所にして Claude Code などを起動しないこと**（よその設定とフックが効いてしまう）。

### `claudia.nar` の出どころとライセンス

- 出どころ: <https://github.com/ponapalt/claudia/releases> のタグ `35799705884` の添付 `claudia.nar`（2026-09-22 23:55 UTC 公開・`github-actions[bot]` の自動リリース・2026-09-24 取得）。`konnoyayame` から作られたゴーストで、同じく出し直されるので、**タグと次のハッシュで 1 本に固定している**。
  **上流は最新の 1 本しかリリースを残さない。** 2026-09-24 に確かめたところ、両リポジトリともリリースは 1 本だけで、上の `konnoyayame` の固定タグ `35438213310` は既に 404 だった。だから**このように上流のリリースが消えていく検体は、URL の行にせず書庫をこのフォルダに置く**（URL だけを登記すると数日で取れなくなる）。URL の行を使ってよいのは、開発者が管理していて、書庫の隣にハッシュのファイル `<URL>.sha256` を公開する取り寄せ先から取るときだけである（今は `pasta-in-windows` の 1 本＝下の節。取り寄せ先に求める条件は「検体を 1 本足す手順」）。
  sha256 `93a34c502231581c550c236052a64e1a915d8e3dac277153caed6774d23a790a`
- ライセンス（書庫の中の `README.md`「ライセンス」節が出どころ）: 辞書・シェル画像・同梱バルーン 2 つを含め全体が Unlicense（パブリックドメイン）。改変・再配布・二次利用は自由。
- 手元で畳み直していない（ダウンロードしたバイト列をそのまま置いた。全エントリが deflate 圧縮）。Unlicense なので畳み直してもよいが、揃える利得が無い。
- 同梱バルーンが 2 つ（`balloon0.directory,claudia`・`balloon1.directory,claudia_vertical`）で、**同梱が複数の検体はこれが唯一**。
- 差し替えるとき: `konnoyayame` と同じく、新しいタグの `.nar` をそのまま置き、この節のタグ・ハッシュと上の表の 2 つの数を書き直す。

### `emily4.nar` の出どころとライセンス

- 出どころ: 開発者が SSP のインストール先の `ghost/emily4/` から作った `.nar`（2026-10-07 作成・2026-10-08 追加）。SSP の作者陣が SSP 本体に同梱している既定ゴーストで、単独の配布ページは無い。SSP 専用に作られたゴーストとして、areka が SSP 前提の作りをどこまで受けられるかを見る検体である。
  sha256 `ebc87c6daa108116b9afdc6608059aa35ffbe183e0da83eb1b01888c824d39c0`
- 中身は SSP の `ghost/emily4/` から起動記録（`ghost/master/profile/`・`shell/master/profile/`・`ghost/master/yaya_variable.cfg`）を除いたもの。**SSP の走行の残り `ghost/master/yaya_ssp_debug.log`（356 バイト）が 1 本入っている**が、受け取ったバイト列のまま置いた（deflate と無圧縮が混在＝239／21）。
- ライセンス（書庫の中の `readme.txt` が出どころ）: 「Emily/Phase4.5（ghost/emily4フォルダ以下）」は **CC BY-NC 4.0**（<https://creativecommons.org/licenses/by-nc/4.0/deed.ja>）。このリポジトリの MIT ライセンスは**この 1 本には及ばない**。`konnoyayame` のシェルと同じく、**開発用の検体としてだけ使い、areka の配布物へ同梱しない**。
- **同梱のバルーンは入らない。** `install.txt` が `balloon.directory,emily4`・`balloon.source.directory,balloon` を名乗るのに、書庫に `balloon/` が無い（SSP ではバルーンが別の `balloon/emily4/` に入っているため、ゴーストのフォルダだけを畳むと抜ける）。`areka-nar` は取り出し元の無い同梱のバルーンを断らずに飛ばし、ゴースト `emily4` だけを入れる（`<根>/balloon/emily4/` は作らない）。飛ばしたことは記録に 1 件（鍵 `balloon.source.directory`・値 `balloon`・置き場 `emily4`）残り、areka の画面からのインストールでは警告のダイアログが 1 つ出る。起動するとバルーンは、ほかに当たるものが無ければ既定のバルーンになる。登記表の行もこの形（ゴースト 1 つ・同梱のバルーン 0）で、判定は `crates/sample-ghost-kit/src/lib_tests.rs` の `emily4_nar_installs_the_ghost_alone_and_skips_its_missing_companion`。

### `dot_sakura.nar` の出どころとライセンス

- 出どころ: 開発者のサイトが公開する <https://ekicyou.github.io/old/dot-sakura/download/nar/dot_sakura.nar>（ゴースト「．さくら」・どっとさくら・2026-10-22 取得）。開発者が 2003 年ごろに作ったゴーストで、華和梨 7.4（32 ビット）の古い作りをまとめて持つ。
  sha256 `0c602c6a431dcbec48c02d934d98e342337d2db9628095e0a6f90d3e83b7a472`（1,425,534 バイト）
- 手元で畳み直していない（取り寄せたバイト列をそのまま置いた。deflate と無圧縮が混在＝129／75）。
- 取り扱いの条件（書庫の中の `readme.txt`）: 取扱種別「無償ですが著作権複数のためフリーには出来ません(^^;」、転載・配布「インターネット上は不可、それ以外は各自の責任において可」。シェルの `readme.txt`（`shell/master/readme.txt`）: リビルド「日蔭　翠」、転載・配布「インターネット上でのアーカイブの転載不可。」。
- それでもこのリポジトリに置く根拠は、開発者の言葉（2026-10-10）「どっとさくらは私の実装なので、検体として利用する分には問題ない。私のサイトで公開してます」。このリポジトリの MIT ライセンスは**この 1 本には及ばない**。`konnoyayame` のシェル・`emily4` と同じく、**開発用の検体としてだけ使い、areka の配布物へ同梱しない**。
- 見る物: 華和梨 7.4 の 32 ビットの SHIORI・Shift_JIS の辞書・α の無い絵（`tRNS` を持つ絵は 0 枚）・同梱バルーン `bottle` の連番のオンラインの印 `online0`〜`online15` と赤の抜き色。
- **URL の行にしない理由**: 取り寄せ先の隣にハッシュのファイル `dot_sakura.nar.sha256` が無い（2026-10-22 に確かめて 404）。URL の行には隣の `<URL>.sha256` が要る（「検体を 1 本足す手順」）。サイトにハッシュのファイルが置かれたら、URL の行へ移せる。
- 入れた結果: ゴースト `dot_sakura` と同梱のバルーン `bottle`（`<根>/balloon/bottle/`）が入り、飛ばした同梱は 0 件・警告も 0 件。書庫の最上位の `delete.txt`・`updates.txt`・`readme.txt` は、`ghost/dot_sakura/` の直下に普通のファイルとして置かれる（areka はインストールのときに `delete.txt` を読まない）。判定は `crates/sample-ghost-kit/src/lib_tests.rs` の `dot_sakura_nar_installs_the_ghost_and_its_balloon_without_warnings`（入る 2 件・飛ばした同梱 0・警告 0 と、`delete.txt`・`updates.txt` の置き場を確かめる）。
- 差し替えるとき: サイトの新しい `.nar` をそのまま置き、この節のハッシュ・大きさ・日と上の表の 2 つの数を書き直す。

### `pasta-in-windows`（URL の行）の出どころ

- 出どころ: 開発者のゴースト開発リポジトリ ghost_dev が GitHub Pages で公開する <https://ekicyou.github.io/ghost_dev/pasta-in-windows/pasta-in-windows.nar>（ゴースト「窓際のぱすたさん」）。隣に sha256sum の形のハッシュのファイル <https://ekicyou.github.io/ghost_dev/pasta-in-windows/pasta-in-windows.nar.sha256>（1 行 `<小文字 16 進 64 桁>  pasta-in-windows.nar` と LF）が公開され、ぱすたのリリースのたびに書庫と一緒に書き直される。
- ghost_dev のリポジトリ自体は非公開なので、`raw.githubusercontent.com` などの raw の URL はログインなしでは 404 になる。取れるのは上の GitHub Pages の URL だけである。
- **いつも最新の版を取る。** 中身はぱすたのリリースのたびに同じ URL のまま差し替わる（版ごとの固定の URL は無い）。登記は URL だけを持ち、版を固定しない。取り寄せの段は回すたびに公開のハッシュを見て、手元の書庫が合わなければ新しい版を取り寄せる。開発者の方針（2026-10-18）「ゴースト検体は常に最新をチェックしたい」——新しい版で壊れたことに気付けるので、固定に意味はあまりない。だから版ごとのハッシュと大きさはここにも上の表にも書かない（書けば次のリリースで古びる）。
- **書庫をこのリポジトリに入れない理由**: シェルの絵が寄贈されたもので、それをこのリポジトリへ置き直してよいかが分からない。`vendors/` にも配布物にも、ほかのどの追跡ファイルとしても持たない。

## 取り出し方

畳んだ `.nar` を自分で開ける必要は無い。窓口は `crates/sample-ghost-kit` である。

- テストから: `sample_ghost_kit::SampleRoot::acquire("emo2")` が起動記録の無い新品の木を配る。返った値を束縛している間だけパスを借りられ、捨てると木も消える。
- 実機の手順で絶対パスが要るとき: `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2`。標準出力に `root=` ／ `folder=` ／ `balloon.<名>=` の行が出る（`folder=` がゴースト本体の置き場）。未登録の名前は標準出力へ 1 行も出さずに終了コード 2 で断る。**このコマンドは呼ぶたびに同じ展開先を消して作り直すので、2 つの端末から同時に呼ぶと互いの走行の木を消してしまう。**実機を 2 周するときは 1 周ずつ順番に回すこと。
- URL の行の検体も呼び方・出力の形は同じである。窓口は通信せず、置き場の書庫とハッシュのファイルが合うことを確かめてから展開する。置き場に無い・合わないときは呼び出しが失敗し、検体の名前・探した場所・直し方を示す（`nar-sample-path` は標準出力へ何も出さず、標準エラーへその文を出して終了コード 3 で終わる。未登録の名前の 2 とは別）。

## 取り寄せの段

URL の行の書庫を取り寄せて確かめ、置き場に置くコマンドである。どのディレクトリからでも同じに動く。

```
pwsh -NoProfile -File <ワークツリーの絶対パス>\tools\sample-fetch.ps1
```

- **ワークツリーごとに 1 回回す。** 全体テストの入口を通さずにテスト（`cargo test` など）や `nar-sample-path` を回すときは、先にこれを 1 回回すこと。回していないと、URL の行の検体を使うテスト（登記表の全部の行を引く判定を含む）は「置き場に無い」の文で失敗する（飛ばしにも緑にもならない）。
- 2 回目からは、毎回取るのは小さいハッシュのファイル（`<URL>.sha256`）だけで、書庫は公開のハッシュが変わったとき（新しい版が出たとき）だけ取り寄せる。手元が最新なら最後の行は `sample-fetch: fetched 0, present 1, failed 0` で、終了コードは 0。
- 置き場: `<ワークツリー>\target\nar-samples\fetched\<名前>.nar` と、その隣の `<名前>.nar.sha256`（取り寄せ先が公開したハッシュのファイルそのまま）。`CARGO_TARGET_DIR` を設定しているなら `<CARGO_TARGET_DIR>\nar-samples\fetched\` の下になる。`target` の下なのでリポジトリは追跡しない。
- 印字は行ごとに `sample-fetch: <名前>: present`（在った）／`fetched`（取り寄せた）／`FAILED ...`（失敗）、最後に `sample-fetch: fetched <数>, present <数>, failed <数>`。失敗が 1 つでも在れば終了コードは 1。
- 通信できないとき（ハッシュのファイルが取れないとき）: 置き場に書庫とハッシュのファイルが両方在って互いに合えば、`could not check for a newer archive` の警告 1 行を出して `present` として通る（新しい版かどうかは確かめられていない）。どちらかが無い・合わないなら失敗する。
- 失敗したとき: 文に検体の名前・URL・理由が出る。通信の一時の失敗なら回し直す。取り寄せ先に届かない場所なら、下の「手で置く口」で 2 つのファイルを置く。取り寄せた書庫が公開のハッシュと合わないときは、置き場に何も置かずに捨てる（途中までのファイルも残さない）ので、少し待って回し直す（取り寄せ先が書き換わっている途中のことがある）。
- `cargo clean` などで `target\` が消えたら、もう一度回せば戻る。
- 取り寄せの段そのものの判定（通信しない・`file:///` の見本で 8 判定）は `-SelfTest` で回す。全部通れば最後の行が `selftest: all passed` で終了コードは 0。

  ```
  pwsh -NoProfile -File <ワークツリーの絶対パス>\tools\sample-fetch.ps1 -SelfTest
  ```

## 手で置く口

取り寄せ先に届かない場所では、書庫を手で置ける。

1. 置き場 `<ワークツリー>\target\nar-samples\fetched\`（`CARGO_TARGET_DIR` のときは `<CARGO_TARGET_DIR>\nar-samples\fetched\`）に、**2 つのファイルを両方**置く。
   - `<名前>.nar` — 書庫（例: `pasta-in-windows.nar`）。
   - `<名前>.nar.sha256` — ハッシュのファイル。公開の `<URL>.sha256` を取ってそのまま置くか、手元の書庫から 1 行 `<小文字 16 進 64 桁>  <名前>.nar`（間は空白 2 つ・末尾は LF）を書く。PowerShell で書くならこの形（`<置き場>` は上の絶対パス）。

     ```
     $h = (Get-FileHash -Algorithm SHA256 '<置き場>\pasta-in-windows.nar').Hash.ToLowerInvariant()
     [System.IO.File]::WriteAllText('<置き場>\pasta-in-windows.nar.sha256', "$h  pasta-in-windows.nar`n")
     ```

2. 確かめる: 上の取り寄せの段を回す。公開の最新のハッシュと合えば書庫を取り寄せずに `present`、通信できなければ置いた 2 つが合えば警告つきで `present` になる。公開の最新と合わなければ合わない旨を出して公開の書庫を取り寄せ直す。窓口も置いた 2 つが合わなければその書庫を使わず、合わない旨を示して失敗する。

## 検体を 1 本足す手順

足し方は 2 つの形で違う。書庫をこのフォルダに置ける検体（再配布してよいもの）は「書庫を置く行で足す」、置き直してよいか分からない書庫を開発者が管理する取り寄せ先から取る検体は「URL の行で足す」。上流のリリースが消えていく検体は URL の行にしない（`claudia.nar` の節）。

### 書庫を置く行で足す

畳むのは検体を足すときの 1 回だけなので、道具は使い捨ての実行体 `crates/sample-ghost-kit/examples/fold-samples.rs` に置いてある。

1. **一度も起動していない配布そのままの展開形を置く。** `vendors/sample_ghost/<フォルダ名>/` の**直下に `install.txt` が来る形**にする。`.nar` のファイル名はこのフォルダ名から採る。
   既に動かした木や、SSP のインストール先から複製してきた木を使うときは、**`ghost/master/profile/` を先に消す**こと。起動記録は配布物ではなく、これを消さずに次へ進むと `git add` がそのまま追跡してしまう（⑵）。
2. **`git add` して、`profile/` が入っていないことを確かめる。** 中身を選ぶ出どころは `git ls-files` だけなので、追跡していないファイルは畳んだ書庫に入らない。裏を返せば `git add` したものは全部入る。追加した直後にこれを問うこと。

   ```
   git ls-files vendors/sample_ghost/<フォルダ名> | grep -E '/profile/' | wc -l
   ```

   **0 以外なら手順 1 からやり直す**（`git rm -r --cached` で外し、`profile/` を消してから `git add` し直す）。0 を見てから、入るはずのファイルが漏れていないかを `git status` で確かめる（下の「⑵ 入ってはいけないもの」の罠に注意）。
3. **畳む。**

   ```
   cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/<フォルダ名>
   ```

   `--from` は必須である。引数を書かない形と `--check` だけの形は、`--from <展開形のフォルダ> を書くこと` を返して終了コード 1 で落ちる（登記表は展開形の在処を持たないため）。`--from` は登記表を見ないので、**まだ登記していない検体でも畳める**。畳むと同時に往復の一致まで確かめて、一致した件数を 1 行で印字する。
4. **展開形を追跡から外して消す。** `git rm -r --cached vendors/sample_ghost/<フォルダ名>` の後に実体を消す。追跡するのは `.nar` だけである。
5. **登記表に 1 行足す。** `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES` に `Sample { name, kind, balloons, source: SampleSource::Vendored }` を 1 つ書く。`name` は `.nar` のファイル名（拡張子無し）＝`install.txt` の `directory`、`balloons` は同時にインストールされるバルーンの `directory` 名。**この 1 行は畳んだ後に足す。** 手順 3 は登記表を読まないので先に書いても畳む役には立たず、逆に `.nar` の無い名前が登記表に載っている時間が生まれる。**同じコミットで、この README の先頭の「書庫を置く行の検体」の表にも 1 行足すこと**（種別・中のファイル数・大きさ）。表には判定が付いていないので、書き足さなければ黙って古びる。

後から `.nar` を書き直さずに往復だけ確かめ直したいときは、同じ呼び方に `--check` を足す（展開形が手元に在るときだけ使える）。

```
cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/<フォルダ名> --check
```

下の「守る 5 点」はこの形（畳んで置く）の話である。

### URL の行で足す

書庫はこのリポジトリに入れない。畳まず、取り寄せ先が公開する書庫をそのまま使う。

1. **取り寄せ先の URL を決める。** 取り寄せ先に求める条件は次の 4 つ（どれかが欠ける取り寄せ先なら URL の行にしない）。
   - ログインなしに取れる `https://` の URL であること（非公開のリポジトリの raw の URL は取れない）。
   - 書庫の隣に sha256sum の形のハッシュのファイル `<URL>.sha256`（最初の語が 16 進 64 桁）を公開し、**書庫と一緒に差し替える**こと。
   - 置いた書庫を消さないこと（新しい版で上書きするのはよい。消えていくリリースは不可）。
   - 以上を開発者が管理していること（開発者が管理する取り寄せ先）。
2. **登記表に 1 行足す。** `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES` に `Sample { name, kind, balloons, source: SampleSource::Fetched { url } }` を 1 つ書く。`name` は書庫の `install.txt` の `directory`、`balloons` は同時にインストールされるバルーンの `directory` 名、`url` は書庫の URL（ハッシュのファイルの URL は登記しない。取り寄せの段が `<url>.sha256` を取る）。ハッシュと大きさも登記しない（いつも最新を取るため）。`url` が `https://` で始まらない・空白か制御文字を含む・空、または名前が `--` で始まると、窓口の決定論テストが落ちる。
3. **この README を直す。** 先頭の「URL の行の検体」の表に 1 行（種別・置き方＝URL・中のファイル数と大きさは「版で変わる」）と、出どころの節（取り寄せ先の URL と隣の `.sha256`・書庫をこのリポジトリに入れない理由・ライセンス）を足す。版ごとのハッシュと大きさは書かない。
4. **取り寄せの段で確かめる。** 上の「取り寄せの段」のコマンドを回して `fetched 1`（2 回目は `present`）・`failed 0` を見て、登記表の全部の行を引く判定（`cargo test -p sample-ghost-kit`）を回す。

## 守る 5 点

### ⑴ 入れるのは追跡ファイルだけ

`fold-samples` は畳む前に `git ls-files` を引き、返ってきたファイルだけを作業フォルダへ写してから畳む。フォルダを走査して入れる作りにしていないのは、走査だと追跡外のものまで書庫に入るからである。

一つ気を付けること——`git ls-files` は**実在しないパスに対して空と終了コード 0 を返す**。つまり畳む対象が消えていても静かに「0 件」になる。手順はそれを黙って成功にしないため、追跡ファイルが 0 件なら落ちるようにしてある。

### ⑵ 入ってはいけないもの——永続化フォルダと、無視規則が黙って落とすもの

ゴーストは起動すると `ghost/master/profile/` の下に起動記録を書く。これは配布物ではないので入れてはならない。`emo2` の木は畳んだ日、ディスク上 157 ファイル・追跡 110 ファイルで **47 件ずれていた**——差は全て `profile/` の配下で、この 47 件は**一度も `git add` されていなかった**。書庫に入らなかったのは、⑴ の出どころが追跡ファイルだけだからである。

**規則が守ってくれるとは思わないこと。`profile/` に当たる無視規則は 1 つも無い。** リポジトリ直下の `.gitignore` にも `.git/info/exclude` にも `profile` の文字は無く、`core.excludesFile` は設定されていない。次の問いはどの出どころからも**終了コード 1（無視されない）**を返す。

```
git check-ignore -v --no-index vendors/sample_ghost/<フォルダ名>/ghost/master/profile/ghost.dat   # → 1（無視されない）
```

つまり手順 2 の `git add` は、`profile/` を持つ木に対しては**それを追跡して書庫に入れる**。守りは規則の側ではなく手順の側にしか無い——だから手順 1 と 2 でそれを潰す。

逆向きの罠もある。**リポジトリ直下の `.gitignore` は `*_test.txt` と `*_dump.txt` を無視する。** これは里々テンプレートの辞書 `dic09_Test.txt` のような名前を巻き込み（このリポジトリは `core.ignorecase=true` なので大小違いでも当たる）、**辞書が 1 つ欠けた検体が黙って出来上がる**。このフォルダの `.gitignore` が打ち消しの 2 行（`!*_test.txt`・`!*_dump.txt`）を持っているのはそのためで、展開形をここへ置く（手順 1）限りは巻き込まれない。

疑わしいときは名前を直に問える。無視されていれば終了コード 0、無視されていなければ 1 を返す。

```
git check-ignore --no-index vendors/sample_ghost/<フォルダ名>/ghost/master/dic09_Test.txt   # → 1（無視されない）
git check-ignore --no-index crates/areka/dic09_Test.txt                                   # → 0（無視される）
```

`--no-index` を付けること。付けないと、既に `git add` 済みのパスを「無視されない」と即答するので、答えが常に正しく見えてしまう。

### ⑶ 改行も文字コードも変換しない

取り出したバイト列が配布時と 1 バイトも違ってはならない。里々テンプレートは辞書も `surfaces.txt` も `descript.txt` も全行 CRLF で、ここが揺れると展開した木が揺れる。

- このリポジトリは `core.autocrlf=true` なので、何もしないと改行が変換されて登録される。このフォルダの `.gitattributes` の `* -text` がそれを止めている。`git check-attr text -- vendors/sample_ghost/<名>.nar` が `text: unset` を返せば効いている（属性の届かない場所では `unspecified` になる）。
- 手順 3 の写しと書き込みはバイトの複製である（テキストとして読み書きしていない）ので、文字コードの推測も変換も起きない。

このフォルダのファイルを編集するときも同じ話で、**`.gitattributes` と `.gitignore` は LF で書くこと**。`* -text` が変換を止めるこのフォルダでは、CRLF で書けばそのバイトがそのまま登録され、1 行直したつもりが全行の入れ替えとして記録される。

### ⑷ 配布形の構造にする

`.nar` は zip である。読み手（`crates/areka-nar`）が要求するのは次の 2 点。

- **`install.txt` が書庫の最上位に在ること。** 包みフォルダ 1 段は黙って剥がさず拒否する（`<名>/install.txt` は最上位ではない）。手順 1 が「フォルダの直下に `install.txt`」と言っているのはこのためである。
- 名前とその中身が `install.txt` の解釈どおりであること。展開先の写像（同梱バルーンをどこへ置くか）は `install.txt` の `directory`・`*.directory`・`*.source.directory` だけから決まり、登記表の綴りは使わない。

ここで畳んだ 4 本（`R_POST_and_KOMAINU.nar`・`emo2-kakukaku-offsetdpi.nar`・`emo2-kakukaku-wplimit.nar`・`StayseeBalloon.nar`）は**全エントリが無圧縮（圧縮方式 0）**である（`konnoyayame.nar`・`claudia.nar` と、2026-10-17 に最新版へ差し替えた `emo2.nar` は全エントリが deflate 圧縮）。`fold_tree` が圧縮を掛けないためで、git 自身の zlib が既に同じ仕事をしているので掛ける利得がほとんど無い（実測で差は 3.2%）。読み手は方式 0 と方式 8（deflate）の両方を読めるので、外から貰った `.nar` をそのまま置く分には圧縮されていて構わない。

同じ入力からは同じバイト列が出る。走査順を名前順に固定し、書庫に書く日時を 1980-01-01 00:00 に固定してあるので、畳み直しても `.nar` のハッシュは変わらない。

### ⑸ 畳んだ後に一致を確かめる

手順 3 は畳んだだけで終わらず、**その場で往復を確かめる**。畳んだ `.nar` を本番の展開器で空の根へ入れ直し、置かれた木と元のツリーを突き合わせる。見るのは両方向で、片側だけ数えても抜けは見えない。

- 展開後に置かれた全ファイルが元に在ること（元に無いものが増えていない）
- 元の全ファイルが置かれ、かつ**バイト列が同じ**であること

数が 0 のまま緑になる経路も塞いである——追跡ファイルが 0 件でないこと・作業フォルダへの写しの数が追跡の数と等しいこと・写像が 2 つの元を同じ宛先へ潰していないことを、走るたびに判定する。

印字はこの形で、末尾の「不一致 0 件」と最後の「全ての検体で一致」が揃って初めて通ったことになる（不一致があれば終了コードが 0 にならない）。

```
<フォルダ名>: 追跡 20 ファイル / 展開 20 ファイル / balloon/emo2-kakukaku-wplimit=20 / .nar 33906 バイト / 不一致 0 件
全ての検体で一致
```
