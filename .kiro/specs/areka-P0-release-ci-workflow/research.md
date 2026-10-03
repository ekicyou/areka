# ギャップ分析: areka-P0-release-ci-workflow

> 2026-10-03・基準コミット `c43d3a08`（枝 `claude/areka-p0-release-ci-a887d8`）。入力は確定済みの `requirements.md`・`brief.md`・steering。外の事実は末尾の「出典」に URL を付けた。ここは選択肢と事実を並べるところで、決めるのは設計と要件の話し合い。

## 1. 要約

- **作る物はほぼ新しいファイル 1 本**（`.github/workflows/release.yml`）と、steering の 2 か所の書き換えだけ。配布スクリプト `tools/package.ps1` は CI 向けの呼び方（`-Arch all`・`-Check` なし・終了コード 0〜3・置き場 `target/package/` 固定）を既に持っており、今回の調べでは**直しが要る所は見つからなかった**（要件の「変更 0」を守れる見込み）。
- **実行環境の道具は揃っている**: 今の `windows-latest`（Windows Server 2025＋Visual Studio 2026）に、arm64 のリンクの道具 `Microsoft.VisualStudio.Component.VC.Tools.ARM64` も、x86/x64 の道具も、`vswhere`・`gh`・PowerShell 7.6 も入っている。arm64 の道具を足す段は要らない見込み（要件 9.1 の走りで確かめる）。
- **要件 3.4（前の走りの物を再利用）は、GitHub のキャッシュの決まりとぶつかる**: タグの走りで作ったキャッシュは、別のタグの走りからは読めない（読めるのは同じ参照と既定の枝 main の物だけ）。さらに 7 日使わないと消える。リリースの間が空けば、毎回ほぼ冷えた状態から始まる。道具 2 つは「ビルド済みの実行ファイルを落とす」方式なら毎回数秒で揃うが、それは「前の走りの物の再利用」ではない。**要件の言い回しと手段の擦り合わせが要る**（7 章の 1）。
- **「下書きを残さない」は `gh release create` が大半を受け持つ**: ファイルを添えて作ると、gh は一度下書きで作り、全部載せてから公開し、載せる途中や公開で失敗したら自分で下書きを消す（gh のソースで確認）。ただし**時間切れや取り消しで gh が途中で殺されたときは消されない**ので、失敗・取り消しのときだけ動く後始末の段が要る。後始末は「既に在った Release に触らない」（要件 2.3）と両立させる必要がある。
- **時間の見込み**: 手元の温まった状態で `-Arch all` は 38 秒（完了 `release-package-versioned` の実走の記録）。CI ではキャッシュがほぼ効かないので冷えたビルドになる。リリース用の設定（最適化 `z`・LTO・コード生成単位 1）で、本体を x64 と arm64 の 2 回、補助 exe を i686 で 1 回、検体を出す道具をデバッグで 1 回ビルドする。4 コアの実行環境で **20〜40 分程度と見積もる**（実測は要件 9.1 の走りで取る）。

## 2. 今あるもの

### 2.1 配布スクリプト `tools/package.ps1` が CI の上で要るもの

スクリプトの冒頭の「較正値」の並びと、段「前提の確認」以降を読んで、新しい実行環境で何が要るかを書き出した。

| 要る物 | スクリプトの中の場所 | CI での扱い |
|---|---|---|
| PowerShell 7 以上 | ファイル冒頭の `#Requires -Version 7` | 実行環境に 7.6.6 が在る。`pwsh -NoProfile -File tools/package.ps1 -Arch all` で別のプロセスとして呼べば、workflow 側の `$ErrorActionPreference` などの設定に影響されない |
| git（作業ツリーとして取り出してあること） | 段「前提の確認」の `git rev-parse --short=7 HEAD` と `git status --porcelain` | `actions/checkout` で足りる。浅い取り出し（既定の深さ 1）でも 2 つとも動く |
| `cargo about`・`cargo deny` | 段「前提の確認」の `cargo about --version`／`cargo deny --version`（無ければ終了コード 3 と入れ方の 1 行） | **実行環境に無い**。スクリプトは在るかどうかだけを見て、版は見ない（入れ方の文言に 0.9.2／0.20.2 と書くだけ）。手元は 0.9.2／0.20.2。版を揃えるのは workflow の役目 |
| `Cargo.lock` | 段「前提の確認」の `Test-Path 'Cargo.lock'` | 追跡済み。全ビルドが `--locked` |
| arm64 のリンクの道具 | 段「前提の確認」の `vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64`（`vswhere.exe` は固定の置き場 `%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\` を先に見る） | 実行環境に在る（3 章）。無ければ終了コード 3 で止まる＝要件 3.5「x64 だけで出さない」はスクリプトが既に守る |
| rustup の i686・arm64 のターゲット | 段「i686 ターゲット導入」「arm64 ターゲット導入」の `rustup target add` | スクリプトが自分で足す。workflow に段は要らない（ネットへ出る） |
| 版の読み取り | 段「前提の確認」の `cargo metadata --no-deps --locked` から `areka` パッケージの `version` を読み、形 `^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$` を確かめる | `areka` は `[workspace.package] version` を継いでいるので、要件 2.1 の比べる相手と同じ値になる |
| 検体 | 段「検体の展開」が `cargo run -q --locked -p sample-ghost-kit --bin nar-sample-path -- emo2`（と `StayseeBalloon`）を呼び、`vendors/sample_ghost/*.nar` を `target/nar-samples/manual/` へ展開 | `.nar` は本体のリポジトリに追跡済み（サブモジュールではない）。展開先は「`CARGO_TARGET_DIR` が在ればそこ、無ければ実行ファイルの祖先の `target`」（`sample-ghost-kit` の `devroot.rs` の置き場の決め方）。workflow で `CARGO_TARGET_DIR` を設定しなければ作業ツリーの `target/` の下に収まる |
| サブモジュール `vendors/pasta` | 使わない | 調査資料だけ。取り出さなくてよい（`[patch.crates-io]` も無い） |
| ネイティブのビルドの道具 | `Cargo.lock` に C/C++ を組む重い依存（`ring`・`aws-lc-sys`・`cmake` など）は無く、`cc` だけ | MSVC が在れば足りる |
| PATH の掃除 | ファイル冒頭近くの「Git Bash の `link.exe` が MSVC の `link.exe` を遮る」対策（`cl.exe` を持たない `link.exe` のフォルダを PATH から外す） | 実行環境の PATH に Git の `usr\bin` が載っていても、スクリプトが自分で外す。後始末で戻す |

**出す物と置き場**: 関数 `Get-ArtifactNames` が名前を決める唯一の場所。`target/package/areka-{版}-{arch}.zip` と `…zip.sha256`（中身は「小文字の 16 進 64 字・空白 2 つ・zip のファイル名」と LF の 1 行＝`sha256sum -c` の形）。完成品の名前の物は段「完成」で仮の名前（`.tmp`）から改名されて初めて現れ、終了コードが 0 でないときは後始末が今回の完成品も消す。workflow は「終了コード 0 ⇔ 4 つが在る」を前提にできるが、要件 3.2 は「1 つでも欠けたら」も見ろと言うので、workflow 側でも 4 つの在りかを確かめる段を置くことになる。

**終了コード**: 0＝全段が緑（CPU 種別ごとに `zip:`・`sha256:`・`version:` の行を印字）／1＝段の失敗（`FAIL {段の名前}（{秒}・{理由}）` と外部コマンドの出力の末尾 40 行を印字）／2＝起動確認の否（CI では `-Check` を付けないので起きない）／3＝引数・前提の不正。要件 3.2 の「失敗の段の名前を記録に残す」はスクリプトの印字で既に満たされる。

**作業ツリーを汚さないこと**: 段「git status 不変の確認」が始めと終わりの `git status --porcelain` を比べる。workflow がスクリプトの走っている間に作業ツリーの中へ追跡外のファイルを作ると赤になる。また、始める時点の未コミットの変更の数が zip の中の `BUILD-INFO.txt` に `dirty=` で入るので、workflow は作業ツリーを空のまま渡すのがよい（道具やキャッシュは作業ツリーの外か `target/` の下へ）。

**環境変数**: スクリプトはビルドの段の間だけ `RUSTFLAGS` を `-C target-feature=+crt-static` に差し替え、`CARGO_ENCODED_RUSTFLAGS`・`CARGO_BUILD_RUSTFLAGS` を外す。workflow が `RUSTFLAGS` を設定しても本体のビルドには効かない（検体を出す道具のデバッグビルドには効く）。

### 2.2 リポジトリの状態（2026-10-03 に `gh` で確認）

- 公開リポジトリ（`visibility: PUBLIC`）・既定の枝 `main`・Actions は有効で、使える action に制限なし（`allowed_actions: all`・SHA での固定は求められていない）。
- **workflow のトークンの既定の権限は読み取りだけ**（`default_workflow_permissions: read`）。workflow の中で `permissions: contents: write` と明示すれば Release を作れる（要件 6.3 の形と合う）。
- ルールセット（タグの保護など）は無い。Release は下書きを含め 0 件。リモートのタグは `v0.0.1` だけ（手元には `pre-rebase-*` の 2 つも在るが押されていない）。
- `.github/` は無い（作業ツリーの `target/nar-samples/` の下に検体のゴーストが持つ `.github` が在るだけで、本リポジトリの物ではない）。
- 根に `.gitattributes` は無く、手元の Git は `core.autocrlf=true`（Git for Windows のシステム設定）。`vendors/sample_ghost/.gitattributes` は `* -text` で、`.nar` は改行の変換から外されている。

### 2.3 文書

- `tech.md` の Testing の節の最後の段落が「**外部 CI は持たない**（2026-09-24 に GitHub Actions への移管を検討し見送り）…常設ゲートはローカルのフルテストである」。要件 7.1 はここを書き換える。同じファイルの「マルチアーキテクチャ・ターゲット」の節は arm64 の道具と `vswhere` の置き場を既に書いており、変える所は無い。
- `structure.md` の「**その他の最上位**」の行に `tools/`・`vendors/`・`assets/`・`docs/`・`dist/` は在るが `.github/` は無い。要件 7.2 はここへ足す。
- `roadmap.md` の「配布と公開」節は決めたことの 6 で「『外部 CI は持たない』はテストの門の話」と既に書いている（今回の書き換えの趣旨と一致）。

## 3. 実行環境（`windows-latest`）の事実

runner-images の一覧（2026-10-03 時点）では、`windows-latest` は `windows-2025-vs2026`（Windows Server 2025＋Visual Studio 2026）を指す。2026 年 6 月に Visual Studio 2022 から 2026 へ移った。その中身の一覧（Image Version `20260925.250.1`）から:

| 項目 | 値 | 要件との関係 |
|---|---|---|
| Visual Studio | Enterprise 2026 `18.10.12217.157` | − |
| `Microsoft.VisualStudio.Component.VC.Tools.ARM64` | `18.10.12020.329` が**在る** | 要件 3.4・3.5。足す段は要らない見込み。名前で問うので、Visual Studio の世代が変わっても部品名が同じなら `vswhere` で見つかる |
| `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` | `18.10.12020.329` | i686 の補助 exe と x64 の本体 |
| VSWhere | `3.1.7` | 段「前提の確認」 |
| Rust／Cargo／rustup | `1.98.1`／`1.98.1`／`1.29.1`（clippy・rustfmt のみ） | 手元は `1.99.0`。1 つ古い（7 章の 5） |
| GitHub CLI | `2.101.0` | Release の作成 |
| PowerShell | `7.6.6` | 配布スクリプト |
| 機械 | 公開リポジトリの標準の Windows x64: 4 コア・メモリ 16 GB・SSD 14 GB（表記） | 公開リポジトリでは「無料・無制限」（要件 6.4） |

旧い一覧（`Windows2025-Readme.md`・Visual Studio 2022 版）にも同じ部品名が載っている。どちらでも在る。

**注意**: `windows-latest` は名前の指す先が年に何度か変わる（今年 6 月の移り変わりがその例）。名前を固定（`windows-2025` など）するか、`latest` のままにするかは設計で決める（7 章の 6）。

## 4. 要件ごとの対応表

記号: **在る**＝既存の物で足りる／**欠け**＝作る／**不明**＝調べが要る／**制約**＝外の決まりで形が縛られる。

| 要件 | 今あるもの | 状態 | 書き留め |
|---|---|---|---|
| 1.1 タグの指すコミットだけで始める | − | 欠け | `on: push: tags: ['v*']`。`actions/checkout` は既定でそのタグのコミットを取り出す |
| 1.2 PR・枝への push で動かない | − | 欠け | `push` に `tags` だけを書き `branches` を書かなければ、枝への push では動かない（GitHub の決まり）。`pull_request` を書かない |
| 1.3 タグ・版・コミットに触らない | − | 欠け／制約 | `gh release create` はタグが無いと**既定の枝の最新から自分でタグを作る**。`--verify-tag` を付ければ、タグが無いとき止まる。workflow は `git push` も `Cargo.toml` の書き換えも持たない |
| 1.4 きっかけは 2 つだけ | − | 欠け | `push: tags` と `workflow_dispatch`。要件 9 の一時的なきっかけはマージ前に外す |
| 2.1 タグの版と `[workspace.package] version` を文字どおり比べる | 配布スクリプトの段「前提の確認」が `cargo metadata` で `areka` の版を読む（ビルドの前） | 欠け | workflow は zip の前に比べる必要があるので、スクリプトの印字を待てない。読み方は 3 案（5 章の「版の読み方」） |
| 2.2 不一致で止め、両方を印字 | − | 欠け | タグ `v` だけ・`v0.0.2-x` も「文字どおりの比べ」で落ちる |
| 2.3 既存の Release（下書きを含む）で止める | − | 欠け／不明 | `gh release view {タグ}` は、公開の物はタグで、下書きは「予定のタグ名」で探す（gh のソースの関数 `FetchRelease` の説明）。見つからないときの終了コードは 0 以外だが、通信の失敗も 0 以外なので、「無い」と「調べられなかった」を見分ける形が要る |
| 3.1 `-Arch all` で 4 つ | 配布スクリプト | 在る | `pwsh -NoProfile -File tools/package.ps1 -Arch all` |
| 3.2 0 以外か欠けで止め、段の名前を残す | スクリプトの `FAIL {段の名前}` の印字 | 在る＋欠け | 4 つの在りかの確かめは workflow 側に足す。段の名前を GitHub の注記（`::error::`）にも上げるかは任意 |
| 3.3 `-Check` を呼ばない | − | 在る | 付けないだけ |
| 3.4 道具を揃え、前の走りの物を再利用 | − | 欠け／**制約** | 下の「キャッシュの決まり」。`cargo install` でソースから入れると 2 つで数分〜10 分程度。ビルド済みの実行ファイルを落とす方式なら数秒 |
| 3.5 arm64 の道具が無ければ x64 だけで出さない | スクリプトの段「前提の確認」が終了コード 3 で止まる | 在る | 実行環境には在る（3 章）。足す段を持つか、確かめて止めるだけにするかは設計 |
| 4.1 自動のリリースノートつき・公開で作り 4 つを添える | − | 欠け／不明 | `gh release create v{版} {4 つ} --generate-notes --verify-tag`。**初回はまだ Release が 1 つも無いので、ノートが最初のコミットからの全部（PR 200 本超）になるおそれ**（GitHub の説明は「前の Release を自動で選ぶ」とだけ書く）。`--notes-start-tag` で前の `v*` タグを渡す案がある（7 章の 7） |
| 4.2 添える物はスクリプトが作った物そのもの・`.sha256` が合う | スクリプトの段「SHA256」 | 在る＋欠け | 添える前に `Get-FileHash` で計算し直して `.sha256` の値と比べる段を足せる。GitHub は載せたファイルの SHA256 を `digest` として返すので、下書きの段階で照らし合わせる案もある |
| 4.3 途中で失敗したら下書きも公開も残さない | − | 欠け／一部在る | gh はファイルの添付の途中の失敗と公開の失敗で、自分の作った下書きを消す（gh のソースの `cleanupDraftRelease`）。**プロセスが殺された（時間切れ・取り消し）ときは消えない**。`if: failure() \|\| cancelled()` の後始末の段が要る |
| 4.4 一部だけの Release を公開にしない | gh の「下書きで作り、全部載せてから公開」の流れ | 在る | gh に任せる案と、自前で下書き→確かめ→公開する案（5 章） |
| 5.1〜5.3 乾いた走り | − | 欠け | `workflow_dispatch` で `github.ref_type` が `branch` か `tag` かで分ける。**手で始める型は workflow のファイルが main に在るときだけ使え、走る定義は選んだ枝やタグの物**。`release.yml` を持たない古いタグ（`v0.0.1`）では乾いた走りはできない |
| 5.4 名前と SHA256 を印字 | スクリプトが `zip:`・`sha256:` の行を印字 | 在る＋欠け | `.sha256` の中身を印字する 1 段を足せば足りる |
| 6.1 長生きするトークンを置かない | − | 欠け | `GITHUB_TOKEN` だけ |
| 6.2 トークン・リモートの URL を出さない | スクリプトは `git remote` を呼ばない | 在る＋欠け | `actions/checkout` の `persist-credentials: false` で `.git/config` にトークンを残さない。gh へは `GH_TOKEN` をその段だけに渡す（配布スクリプトの段には渡さない） |
| 6.3 権限は `contents: write` だけ | − | 欠け | キャッシュの保存はトークンの権限を使わないので足さなくてよい |
| 6.4 無料枠・Windows 1 本 | − | 欠け／制約 | 公開リポジトリの標準の実行環境は無料・無制限 |
| 6.5 上限の時間で止める | − | 欠け | `timeout-minutes`（job と段の両方に置ける。既定は 360 分） |
| 7.1・7.2 文書 | 2.3 の 2 か所 | 欠け | 書き換えだけ |
| 8.1〜8.3 成功なら Release は揃って公開済み・公開の後に失敗しうる段を持たない | − | 欠け／**制約** | **action の「後処理」（`actions/checkout` の後片付け・`Swatinem/rust-cache` の保存など）は、全部の段の後に自動で走る**。公開の段を最後に置いても、後処理が失敗すると「Release が在るのに結果は失敗」になりうる。後処理の無い action だけにするか、保存を止める（`save-if: false`）か（5 章） |
| 8.4 乾いた走りは「手で始めた」のまま | − | 在る | 後段は `github.event.workflow_run.event` で `push` と `workflow_dispatch` を見分けられる。タグの push のときの `head_branch` はタグ名 |
| 8.5 後段を呼ばない | − | 在る | 何も書かないだけ |
| 9.1 マージ前に 1 回緑・証跡 | − | 欠け | 作業の枝への `push`（`branches: [作業の枝]`）を一時的に足す。push のきっかけは押したコミットの中の定義で動くので、main に無くても走る。押すたびに開発者の了承が要る（要件生成で決めた手順） |
| 9.2 一時的なきっかけを残さない | − | 欠け | マージ前に定義から外し、静的に確かめる（`branches:` の行が無いこと） |
| 9.3 初回の実走は条件にしない | − | 在る | − |

### キャッシュの決まり（要件 3.4 に効く）

GitHub の「依存のキャッシュ」の説明より:

- 走りが読めるキャッシュは「今の枝」と「既定の枝（main）」で作られた物だけ。
- **タグの名前が違えば、互いのキャッシュは読めない**（例として `release-a` で作った物は `release-b` の走りから読めない、と明記）。
- 7 日使われないキャッシュは消える。リポジトリ全体で 10 GB まで。

つまり、`v0.0.2` の走りで保存したキャッシュは `v0.0.3` の走りから読めない。読めるのは main で作った物だけだが、本 workflow は main への push では動かない（要件 1.2）。main を選んだ乾いた走り（手で始める型）なら main の扱いで保存できるが、7 日で消える。**Rust の依存のビルドの再利用（`Swatinem/rust-cache`）も同じ理由でリリースではほぼ効かない**ので、時間の見積もりは冷えたビルドで立てる。

## 5. 実装の進め方の選択肢

### Release の作り方

**案 A: 1 つの job・`gh release create` に任せる**

- 版と既存の Release を確かめる → 配布スクリプト → 4 つの在りかと `.sha256` を確かめる → `gh release create v{版} {4 つ} --generate-notes --verify-tag` を最後の段にする。失敗・取り消しのときだけ動く後始末の段が、このタグの下書きが在れば消す。
- ✅ 段が少ない。下書き→全部載せる→公開→途中の失敗で消す、を gh が既に持つ。
- ❌ 公開の前に「載ったファイルの SHA256」を照らし合わせる余地が無い（載せる前の手元での照らし合わせはできる）。後始末は gh が殺された場合の残り物だけを相手にする。

**案 B: 1 つの job・下書き→確かめ→公開を自前で**

- `gh release create … --draft` で作り、4 つを載せ、GitHub が返す各ファイルの `digest`（SHA256）と数を `.sha256` と照らし、合えば `gh release edit --draft=false` で公開する。失敗・取り消しのときは後始末が下書きを消す。
- ✅ 公開の直前に「4 つ揃って、中身も合っている」を確かめてから公開できる（要件 4.2・4.4・8.1 を最も直接に示せる）。
- ❌ 段が増える。下書きの消し漏れの経路が gh 任せより広い（後始末が全部を受け持つ）。

**案 C: Windows の job でビルド・別の job で公開**

- Windows の job が 4 つを成果物として上げ、次の job（Linux でも可）が取り込んで Release を作る。
- ✅ 公開の job に重い action や後処理が無いので、要件 8.3 の「公開の後に失敗しうる段」を作りにくい。トークンの書き込み権限を公開の job だけに絞れる。
- ❌ 成果物の上げ下ろしが挟まる（要件 4.2「作り直さない」は満たせるが、確かめる箇所が増える）。要件 6.4「Windows の実行環境 1 本」を「Windows は 1 本・ほかに Linux を 1 本」と読めるかの解釈が要る。

**後始末の共通の注意**: 要件 2.3 は「既に在る Release に一切触らない」。後始末が「このタグの下書きを消す」形だと、走りの始めに既に在った物を消すおそれがある。始めの確かめで「無い」ことを見てから進むので、その後に現れた下書きはこの走りの物と言える。ただし同じタグで 2 つの走りが重なると崩れるので、`concurrency`（タグごとの組・後から来た方を待たせる）で重なりを無くすのが前提になる。消すときは `gh release delete {タグ} --yes` を `--cleanup-tag` なしで使い、タグは残す（要件 1.3）。

### 道具 2 つの揃え方（要件 3.4）

| 案 | 毎回の時間 | 版の固定 | 要件 3.4 の言い回しとの合い方 |
|---|---|---|---|
| ① `cargo install --locked --version` ＋ `~/.cargo/bin` のキャッシュ | 当たれば数秒・外れれば数分〜10 分程度 | 確実 | 「前の走りの物を再利用」に字面で合うが、4 章のとおりタグの走りでは当たらない |
| ② ビルド済みの実行ファイルを落とす（`taiki-e/install-action` で `cargo-about@0.9.2`・`cargo-deny@0.20.2`。両方とも Windows の配布物を EmbarkStudios の GitHub Releases から取る） | 数秒 | 版の指定ができる | 「ソースから入れ直さない」には合う。「前の走りの物の再利用」ではない |
| ③ ①＋②（落とせなければソースから） | 数秒 | 確実 | 同上 |

### キャッシュ（Rust の依存）

| 案 | 効き | 注意 |
|---|---|---|
| 使わない | − | 毎回冷えたビルド。後処理も無い |
| `Swatinem/rust-cache` を読むだけ（`save-if: false`、main の乾いた走りでだけ保存） | main で温めた 7 日の間だけ | 出力先 `target/package` を `workspaces: ". -> target/package"` で教える要。保存の後処理が公開の後に走らない |
| `Swatinem/rust-cache` で毎回保存 | タグの走りでは別のタグから読めない＝ほぼ無駄 | 保存の後処理が公開の後に走る（要件 8.3 の危うさ）。10 GB の枠を食う |

### 版の読み方（要件 2.1）

| 案 | 中身 | 注意 |
|---|---|---|
| `cargo metadata --no-deps` で `areka` の版 | 配布スクリプトと同じ読み方 | 厳密には `[workspace.package] version` そのものではなく、それを継いだ `areka` の版（今は同じ） |
| 実行環境の Python の `tomllib` で `Cargo.toml` を読む | `[workspace.package] version` を名指しで読める | Python に頼る（実行環境には在る） |
| PowerShell の正規表現 | 道具は要らない | TOML を正しく読まない。表の区切りを誤るおそれ |

## 6. 規模と危うさ

- **規模: S（1〜3 日）**。新しいファイル 1 本と文書 2 か所。ただし実行環境での確かめは押すたびに開発者の了承が要り、1 回の往復が冷えたビルドの 20〜40 分かかる見込みなので、手戻りが出ると日数は延びる。
- **危うさ: 中**。手元では再現できない外の仕組み（GitHub のキャッシュの範囲・後処理・取り消しの扱い・ログの文字化け）に頼る所が多く、確かめは実行環境の上でしかできない。配布スクリプトそのものは CI 向けの形が既に揃っているので、そこが崩れる見込みは低い。

## 7. 設計で決めること（話し合いへ渡す候補）

> 2026-10-03 要件の話し合いで仕分けた: 1 と 7 は要件側を直して解決（要件 3.4＝「毎回ソースから組み直さない・前の走りの残り物を前提にしない」、要件 4.1＝「一つ前の `v*` タグ（初回は `v0.0.1`）からの変更でノート」）。2〜6 と 8〜11 は要件が何を約束するかは決まっており、手段だけが残る＝設計（`/kiro-design`）で決める。

1. **要件 3.4 の「前の走りの物の再利用」**: タグの走りは別のタグのキャッシュを読めず、7 日で消える（4 章）。(a) 言い回しを「毎回ソースから組まない（ビルド済みを落とすか、キャッシュに当たればそれを使う）」に改める／(b) 字面どおり保つために main の乾いた走りでキャッシュを温める運用を `release-cycle` に頼む／(c) 当たらなくても要件違反とはしない、のどれにするか。要件の書き直しが要るかもしれない。
2. **Release の作り方**: 案 A（gh に任せる）・案 B（下書き→照らし合わせ→公開を自前）・案 C（job を分ける）のどれにするか。案 C は要件 6.4「Windows 1 本」の読み方とも絡む。
3. **後始末の範囲**: 失敗・取り消しのときに「このタグの下書き」を消す段を置くとして、「この走りの物だけを消す」をどう保証するか（タグごとの `concurrency` で重なりを無くす前提でよいか）。
4. **時間の上限（要件 6.5）**: job の `timeout-minutes` をいくつにするか（見積もり 20〜40 分に対し、例えば 90 分）。ビルドの段と公開の段に別の上限を置き、上限で止まったときも後始末が動く形にするか。
5. **Rust の版**: 実行環境の 1.98.1 をそのまま使うか、`rustup update stable` で手元と同じ最新の安定版に上げるか、版を workflow に書いて固定するか（根に `rust-toolchain.toml` を足すのは「変更 0」の外で手元の開発にも効くので、workflow の中で閉じる案が無難）。今のコードが 1.98 で通るかは確かめていない。
6. **実行環境の名前**: `windows-latest` のままか、`windows-2025` などに固定するか（今年 6 月のように中身が変わる）。
7. **初回のリリースノート**: Release がまだ 1 つも無いので、自動のノートが最初からの全部になるおそれ。前の `v*` タグ（初回は `v0.0.1`）を `--notes-start-tag` で渡すか、そのままにするか。
8. **公開の後の後処理**: `actions/checkout` や キャッシュの action の後処理が公開の後に走る（要件 8.3）。後処理を持つ action を使うなら、保存を止める・後片付けしか残さない、で足りるとするか。
9. **改行の扱い**: 根に `.gitattributes` が無く、手元は `core.autocrlf=true`。実行環境の Git の既定が違うと、zip に入る `README.txt`・`LICENSE-MIT`・謝辞の改行が手元と変わる（配布スクリプトの中身の判定は取り出した作業ツリーと比べるので通る）。取り出す前に `core.autocrlf` を手元に合わせる段を置くか。
10. **action の固定のしかた**: 書き込みの権限を持つ走りで第三者の action（キャッシュ・道具の取り込み）を使う。版の名前（`@v2`）で書くか、コミットの SHA で固定するか。
11. **乾いた走りの記録と証跡（要件 9.1）**: arm64 の道具の在りか（`vswhere` の出力）・各 zip の SHA256・所要時間（段ごとの秒はスクリプトが印字する）・空き容量を、走りの記録のどこに出すか。証跡のファイルを spec のどこに置くか。

## 8. 設計へ持ち越す調べ物

- **ログの文字化け**: 配布スクリプトの印字は日本語。Windows の実行環境の PowerShell は、出力の文字コードが既定で UTF-8 でないことがあり、日本語が `?` などに化けるという報告が多い。化けると要件 2.2・3.2 の印字が読めない。workflow の段で `[Console]::OutputEncoding` を UTF-8 にしてから呼べば足りるかを、要件 9.1 の走りで確かめる（配布スクリプトには触らない）。
- **時間切れ・取り消しのときに後始末の段が確かに動くか**: `if: always()`・`cancelled()` の段は、殺された段の子プロセスを全部止めてから動くとされる。猶予は数分。gh が添付の途中で殺されたときに下書きを消せるかを設計で確かめる。
- **`gh release view` の「無い」と「調べられない」の見分け**: 見つからないときの文言（`release not found`）で分けるか、`gh api` で Release の一覧（下書きを含む・書き込み権限のトークンなら見える）を取って `tag_name` で探すか。
- **空き容量**: 表記は SSD 14 GB。3 つのターゲットのリリースのビルドと検体の展開で足りるか（実際の空きは表記より大きいことが多い）。要件 9.1 の走りで印字して確かめる。
- **Visual Studio 2026 と Rust の MSVC の見つけ方**: 実行環境は今年 6 月から Visual Studio 2026。Rust 1.98 のリンカの探し方が問題なく見つけるかは、要件 9.1 の走りで分かる（手元は Visual Studio 2022）。
- **実行環境の Git の `core.autocrlf` の既定**: 一覧の文書には書かれていない。要件 9.1 の走りで `git config --show-origin core.autocrlf` を印字して確かめる。
- **`cargo about generate` がネットへ出るか**: 依存のソースを読むので、ビルドで落とした物で足りるはず。実行環境はネットへ出られるので止まりはしないが、時間に効くかを見る。

## 出典

- runner-images の一覧（`windows-latest` の指す先）: https://github.com/actions/runner-images
- Windows Server 2025＋Visual Studio 2026 の中身: https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-VS2026-Readme.md
- Windows Server 2025（Visual Studio 2022 版）の中身: https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md
- `windows-latest` の Visual Studio 2026 への移り変わり（2026 年 6 月）: https://github.com/actions/runner-images/issues/14017
- GitHub の実行環境の性能と無料の範囲: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- キャッシュの読める範囲（タグ同士は読めない・7 日で消える・10 GB）: https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching
- `gh release create` の流れ（下書きで作り公開・失敗で下書きを消す・`--verify-tag`・タグを自分で作る既定）: https://github.com/cli/cli/blob/trunk/pkg/cmd/release/create/create.go
- `gh release view` が下書きを予定のタグ名で探すこと: https://github.com/cli/cli/blob/trunk/pkg/cmd/release/shared/fetch.go
- リリースノートの自動生成の範囲: https://docs.github.com/en/rest/releases/releases#generate-release-notes-content-for-a-release
- 道具 2 つのビルド済みの取り込み: https://github.com/taiki-e/install-action/blob/main/TOOLS.md
- Rust の依存のキャッシュ: https://github.com/Swatinem/rust-cache
- 手元の温まった状態での `-Arch all` の 38 秒: `.kiro/specs/completed/areka-P0-release-package-versioned/verification/package-runs.md` の走り 7 の行
