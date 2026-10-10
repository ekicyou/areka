# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: areka を winget で入れたい利用者と、areka を winget で配りたい開発者。利用者は `winget install Areka.Areka.Portable` の 1 行で areka（zip をそのまま置くポータブル版）を入れたいが、winget のコミュニティの置き場（microsoft/winget-pkgs）に areka のマニフェストが無いので引けない。

**今の状態**: `v0.0.2` の GitHub Release が 2026-10-06 に公開され、`areka-0.0.2-x64.zip`・`areka-0.0.2-arm64.zip` とそれぞれの `.sha256` が置いてある（マニフェストの `InstallerUrl`・`InstallerSha256` が書ける）。リポジトリにマニフェストは無い（`dist/winget/` は無い）。名乗りは開発者が 2026-10-02 に `Areka.Areka.Portable` と決めた（`Areka.Areka` は後で作るインストーラー版のために空けておく）。入れ物は zip のまま（zip の中のポータブルな exe）・x64 と arm64 を最初から並べる・無料・未署名。areka は利用者のゴーストと記憶を exe の隣に置くので、winget で入れると、それらは winget の入れ先のフォルダの中に住む。外すとき・上げ直すときにそれらが残るかは測っていない。

**何を変えるか**: ① マニフェスト 4 ファイル（version・installer・既定のロケール＝英語・追加のロケール＝日本語）の雛形を `dist/winget/` に置く。② 手元で `winget install --manifest` から入れて `areka` の 1 語で起動することを x64 の実機で確かめ、外すとき・上げ直すときに利用者のゴーストと記憶が残るかも測る。③ 開発者が winget-pkgs へ初回の提出（PR）を出し、自動の検査が通るところまで進める。更新の PR を自動で出す workflow（`winget.yml`）と説明書の winget の行は、初回の提出が取り込まれた後に `areka-P0-winget-release-automation` が受け持つ（2026-10-10 に分けた）。

> 起票: 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）。2026-10-10 に `areka-P0-winget-release-automation` を切り出した。要件生成: 2026-10-10（main `414d43eb`）。詳細は brief.md。

## Introduction

本仕様は、areka のポータブル版を winget のコミュニティの置き場（microsoft/winget-pkgs）へ**初めて載せるところまで**を受け持つ。作る物は、提出するマニフェストの雛形（リポジトリの中で内容を見られるようにする写し）と、手元の実機の確かめの記録と、初回の提出の手順の 3 つ。完了の線は「winget-pkgs への PR を出し、その自動の検査が通った」所で、人の承認による取り込み（実例で約 2 日）は待たない。

要件生成時に引き直した事実（設計はこれを再検証する）:

- リポジトリの `dist/` で追跡しているのは `dist/README.txt` だけで、`dist/winget/` は無い。`.github/workflows/` は `release.yml`・`crates-io.yml` の 2 本で、`winget.yml` は無い。
- 版の正本（根の `Cargo.toml` の `[workspace.package]` の `version`）は `0.0.2`。同じ欄の `license` は `MIT`、`repository` は `https://github.com/ekicyou/areka`。根に `LICENSE-MIT` が在る。
- GitHub Release は `v0.0.2` の 1 件だけ（公開は 2026-10-06 12:49 UTC・下書きでも先行版でもない）。置いてある物は `areka-0.0.2-x64.zip`・`areka-0.0.2-x64.zip.sha256`・`areka-0.0.2-arm64.zip`・`areka-0.0.2-arm64.zip.sha256` の 4 つ。
- 配布スクリプト `tools/package.ps1` の中身の検査は、zip の最上位を `areka.exe`・`shiori-host32-helper.exe`・`ghost`・`balloon`・`README.txt`・`LICENSE-MIT`・`THIRD-PARTY-NOTICES.md`・`BUILD-INFO.txt` の 8 つに限る＝`areka.exe` は zip の根に在る。zip の中に `profile/` を含む項目が在れば否にする＝記憶のファイルは zip に入っていない。`dist/` から zip へ写すのは `dist/README.txt` の 1 つだけ＝`dist/winget/` を足しても zip の中身は変わらない。
- 利用者向けの説明書 `dist/README.txt` の「■ 記憶の置き場」は、areka の記憶を `areka.exe` の隣の `profile\areka\`、ゴーストの記憶を `ghost\<ゴースト>\ghost\master\profile\areka\`、シェルの記憶を `ghost\<ゴースト>\shell\<シェル>\profile\areka\` と書いている。利用者が入れたゴーストは `ghost\` の下に入る。どれも winget の入れ先のフォルダの中になる。
- areka は、起動した exe がリンクのときにリンクの先から根を引く（`crates/areka/src/boot_config.rs` の `follow_exe_links`・`exe_location`）。使った根は起動の記録の `root_resolved` の行の `root=` に出る。ゴーストの窓が開くと `本物のゴースト窓を開きました` の行が出る（`crates/areka/src/ghost_session.rs`）。
- 2026-10-03 の実測（完了 `areka-P0-release-package-versioned` の `verification/winget-local-check.md`・winget v1.29.380）: `winget install --manifest` には、管理者の手で winget の設定 `LocalManifestFiles` をオンにすることが要る。手元のマニフェストで入れた物は、`winget list` での ID が `ARP\User\X64\Areka.Areka.Portable__DefaultSource` になり、`--id Areka.Areka.Portable` では外せない（`winget list` で引いた ID に `--exact` を付ける）。入れ先は `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\`。`ArchiveBinariesDependOnPath` を付けないマニフェストでは、OS の開発者モードがオンのとき winget はリンク（`%LOCALAPPDATA%\Microsoft\WinGet\Links\areka.exe`）を作り、オフのときはリンクの代わりに入れ先のフォルダを PATH に足す。このときの後片付けは `--purge`（入れ先のフォルダを丸ごと消す）だったので、`--purge` を付けない外し方と上げ直しは測っていない。今の開発機の winget も v1.29.380。
- 外の文書（2026-10-10 に引き直し）:
  - winget-pkgs の文書に、マニフェストの書式 1.12.0 の installer ファイルの説明が在り、`ArchiveBinariesDependOnPath`（zip からポータブルな物を入れるときの環境変数の扱いを決める欄）・`NestedInstallerType`・`NestedInstallerFiles`（`RelativeFilePath`・`PortableCommandAlias`）・`ReleaseDate` が載っている（https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/installer.md ）。
  - Microsoft Learn の提出の説明（https://learn.microsoft.com/en-us/windows/package-manager/package/repository ）: 提出の前に `winget validate` を通す。置き場の並びは `manifests/<発行者の頭文字の小文字>/<発行者>/<アプリ>/<版>/` で、名乗りと版がこの並びと一致していなければならない。1 つの PR に入れられるのは 1 つのパッケージの 1 つの版だけ。`InstallerUrl` は https で、発行者の配布元から直接取れること。PR を出すと自動の検査（マニフェストの検査・URL の検査・ウイルス対策の走査・無人でのインストールとアンインストール）が走り、進み具合は PR のラベルで示される。検査を通り終えて承認待ちになった印は `Azure-Pipeline-Passed`、取り込まれる印は `Validation-Completed`。直しを求める印 `Needs-Author-Feedback` が付いたまま 10 日応えないと、PR は自動で閉じられる。自動の検査の後に人（モデレーター）の審査が在る。
  - Microsoft Learn の `winget uninstall` の説明（https://learn.microsoft.com/en-us/windows/package-manager/winget/uninstall ）: ポータブルな物には `--purge`（入れ先のフォルダの中をすべて消す）と `--preserve`（パッケージが作った物をすべて残す）が在る。どちらも付けないときに、winget が置いていないファイル（利用者のゴースト・記憶）がどうなるかは、この頁に書かれていない＝要件 4 で測る。
- brief の「portable の上げ直しは古い版を外してから新しい版を入れる」は、外の文書でも実機でも確かめていない（要件 4 で測る）。
- winget のソースの読み（2026-10-10 のギャップ分析・開発機と同じ版 v1.29.380・`research.md` の 3.1。**実機ではまだ測っていない**）: winget は zip の最上位の項目だけを覚え、覚えたフォルダは、上げ直しでも `--purge` を付けない外し方でも中身ごと消す。areka の zip の最上位のフォルダは `ghost` と `balloon` なので、利用者が後から入れたゴースト・バルーンと、ゴーストとシェルの記憶は消える見込み。残る見込みなのは最上位の `profile\areka\`（areka の記憶）だけ。brief の「アンインストールで消えるのは winget が置いたファイルだけ（利用者のゴーストは残る）」は、この読みと合わない。何を消すかは「いま入っている版が覚えた項目」で決まるので、ある版を winget で入れた人の最初の上げ直しは、後の版の zip をどう直しても変えられない見込み。正本は要件 4 の実測とする。

## Boundary Context

- **In scope**: ① マニフェストの雛形 4 ファイル（`dist/winget/` の下）／② 手元の `winget install --manifest` で入れて `areka` の 1 語で起動する確かめ（x64 の実機）／③ 外すとき・上げ直すときに利用者のゴーストと記憶が残るかの実測／④ winget-pkgs への初回の提出の手順と、その実行（開発者の手）／⑤ 確かめと提出の記録／⑥ 後ろの spec への申し送り。
- **Out of scope**:
  - `.github/workflows/winget.yml`（更新の PR を自動で出す workflow）・`README.md` と `dist/README.txt` の winget の行と既知の制限の文・`max-versions-to-keep` の数・workflow が使うトークンの登録 → `areka-P0-winget-release-automation`。
  - zip を作ること（完了 `areka-P0-release-package-versioned`）・Release を作ること（完了 `areka-P0-release-ci-workflow`）・リリースの手順（`areka-P0-release-cycle`）・署名（`areka-P0-release-code-signing`）。
  - インストーラー版（名乗り `Areka.Areka`）・スタートメニューのショートカット・winget 以外の置き場（Scoop・Chocolatey）。
  - 利用者のゴーストと記憶の置き場を変えること（要件 4 で消えると分かったら、別の spec として起票する）。
  - winget-pkgs の人の審査による取り込みを待つこと。
  - arm64 の実機で入れて起動する確かめ。
- **変更 0 と明記するもの**: `README.md`・`dist/README.txt`・`.github/workflows/**`・`crates/**`・`tools/**`・各 `Cargo.toml`・配布 zip の中身。破る必要が出たら、作業を止めて開発者へ報告する（同じウェーブのほかの spec と触るファイルを重ねない約束）。
- **Adjacent expectations**:
  - 上流: `v0.0.2` の Release と、その zip・`.sha256`（公開済み）。zip の名前の規則 `areka-{版}-{arch}.zip` と、`areka.exe` が zip の根に在ることは、完了 `areka-P0-release-package-versioned` の決まりのまま使う。
  - 下流: `areka-P0-winget-release-automation` は、本仕様の提出が winget-pkgs に取り込まれてから着手する。本仕様は、その brief へ実測の結果と PR の場所を書き足す（要件 6.2）。
  - `areka-P0-release-cycle` の手順は、タグのコミットに `winget.yml` が在るときだけ winget-pkgs への PR を見守る。本仕様は `winget.yml` を作らないので、本仕様の前後のどこでもリリースを回せる。
  - `areka-P0-release-cycle` の要件・設計は `winget.yml` の持ち主を本仕様の名前で書いている。持ち主は `areka-P0-winget-release-automation` に替わったが、この書き換えを誰がするかは brief が決めていない（下の議題 6）。本仕様の要件には入れていない。

## 要件生成で決めた点と、要件討議の議題（仮置き）

brief が要件討議へ回した議題は、下の仮置きで要件を書いた（答えで変わる条項を併記）。どれも範囲（何を In とするか）の食い違いではなく、範囲の中の値の決めである。

- **議題 1 `ArchiveBinariesDependOnPath` を付けるか** → **決定（2026-10-10 要件討議）＝付ける**（brief の Desired Outcome の 1 と、steering の `roadmap.md`「配布と公開」のとおり。どちらでも `areka` と打てば起動するが、付けると全員が同じ入り方に揃う）。付けると、winget はリンクを作らず、入れ先のフォルダを利用者の PATH に足す。付けないと、OS の開発者モードがオンの利用者はリンク、オフの利用者は PATH になり、利用者の環境で入り方が分かれる（2026-10-03 実測）。areka はリンクを解けるので、どちらでも起動はする。付ける側の代償は、PATH に areka のフォルダが丸ごと載ること。影響する条項: 要件 1.6・3.3・6.2。
- **議題 2 Tags の語** → **決定（2026-10-10 要件討議）**＝英語の側は `ukagaka`・`desktop-mascot`・`mascot`・`ghost`・`shiori`・`sakurascript`、日本語の側は `伺か`・`デスクトップマスコット`・`ゴースト`。ほかのアプリの名前（`ssp` など）はタグに使わない。影響する条項: 要件 1.7。
- **議題 3 提出する版** → **決定（2026-10-10 要件討議）＝`v0.0.2`**（要件 4 の実測の結果に関わらず、この版で出す。下の議題 5）。着手までに次の版が出ていたら、どの版で出すかを着手のときに開発者が決める。影響する条項: 要件 5.3。
- **議題 4 説明文の言語（既定のロケール）** → **決定（2026-10-10 要件討議）＝既定を英語（en-US）、追加で日本語（ja-JP）**。winget は利用者の言語に合うほうを出すので、日本語の Windows では日本語、ほかでは英語の説明になる。審査の人は英語で読める。brief の「3 ファイル」は 4 ファイルに改める。影響する条項: 要件 1.3・1.7・1.8・1.10。
- **議題 5 実測で「利用者のゴーストか記憶が消える」と分かったときに、初回の提出を進めるか** → **決定（2026-10-10 要件討議）＝1 回だけ出して、名乗りを押さえる**。winget-pkgs に名乗りを先に押さえる仕組みは無く、名乗りは 1 版でも取り込まれていることでしか押さえられない。開発者「消えるなら winget は使えない。アプリをインストーラー形式にするとか、ファイル置き場を指定できるようにするとか、整備が必要」「ダメだった場合でも、1 回リリースはしておいて、そのままにしておけば名前は押さえられる」。消える物が在ると分かったときは、① マニフェストに注意書きを付けて `v0.0.2` を出す ② 置き場を直す仕事を別の spec として起票する ③ 置き場が直るまで、説明書に winget の行を載せず、winget-pkgs へ次の版を出さないことを後ろの spec へ申し送る。消える物が 0 なら、注意書きも ③ も要らない。影響する条項: 要件 1.10・4.6・6.2。
- **議題 6 `areka-P0-release-cycle` の文書にある `winget.yml` の持ち主の名前の直しを、どの spec がするか** → **仮置きなし**（本仕様の要件に入れていない。手順は変わらず、名前が替わるだけ）。

生成時に決めた点（勝者が明白なので議題にしない・討議で覆してよい）:

- **手元で入れる確かめは、提出するマニフェストそのもので行う**。`v0.0.2` の Release が公開済みなので、`InstallerUrl` は本物の https の URL のまま入れられる（2026-10-03 の実測で使った手元の http の配り方は、Release が無かったための代わりで、今は要らない）。上げ直しの実測のために要る「より新しい版を名乗るマニフェスト」だけが確かめ専用で、それはリポジトリで追跡しない。
- **リポジトリの雛形は、提出した内容の写し**とする（brief の Approach「提出する内容を PR で見られるようにするため」）。自動の検査を受けてマニフェストを直したら、雛形へ写し戻す。取り込みの後の版ごとの書き換えは持たない（`areka-P0-winget-release-automation` の brief が「更新の PR は winget-pkgs の側の今のマニフェストを元に作られる見込み」としている）。
- **提出の操作と、winget・OS の設定の変更は開発者の手で行う**（完了 `areka-P0-release-package-versioned` の要件 7.4 の裁定と同じ。AI は設定を変えず、開発者のトークンを扱わない）。
- **steering の `structure.md` の `dist/` の説明を追随させる**（今は「配布物へそのまま入れる文書」とだけ書いてあり、zip に入らない `dist/winget/` と食い違うため）。

## Requirements

### Requirement 1: マニフェストの雛形の中身

**Objective:** As a winget で areka を入れる利用者, I want `Areka.Areka.Portable` の名乗りで、自分の機械に合う areka の zip が版とハッシュを固定して引けること, so that 1 行のコマンドで、公開された Release と同じ中身の areka が入る

#### Acceptance Criteria

1. The マニフェストの雛形 shall 名乗り（`PackageIdentifier`）を `Areka.Areka.Portable` とし、`Areka.Areka` をどの欄にも名乗りとして使わない。
2. The マニフェストの雛形 shall 表示名を「areka (portable)」、発行者（`Publisher`）を `ekicyou` とし、発行者の素性を示す URL（`PublisherUrl`）とパッケージの URL を持つ。
3. The マニフェストの雛形 shall version・installer・既定のロケール（en-US）・追加のロケール（ja-JP）の 4 ファイルで成り、4 つとも同じ名乗り・同じ版・同じ書式の版（brief の指定は 1.12.0。提出の時点で winget-pkgs が受け付ける版であること）を持つ。
4. The マニフェストの雛形 shall 入れ物を zip、その中身をポータブルな exe とし、入れる exe を zip の根の `areka.exe` の 1 つ、コマンド名を `areka` とする（`InstallerType: zip`・`NestedInstallerType: portable`・`NestedInstallerFiles` に `areka.exe`・`PortableCommandAlias: areka`）。
5. The マニフェストの雛形 shall x64 と arm64 の 2 項目を持ち、それぞれの `InstallerUrl` を GitHub Release のその版の `areka-{版}-{arch}.zip` を指す https の URL、`InstallerSha256` を同じ Release の `areka-{版}-{arch}.zip.sha256` に書かれた値と同じにする（大文字と小文字の違いは問わない）。
6. The マニフェストの雛形 shall `ArchiveBinariesDependOnPath: true` を持つ（議題 1 の決定）。
7. The マニフェストの雛形 shall ライセンスを MIT とし、短い説明・Release の公開日（`ReleaseDate`）・タグを持ち、タグを英語のロケールでは `ukagaka`・`desktop-mascot`・`mascot`・`ghost`・`shiori`・`sakurascript`、日本語のロケールでは `伺か`・`デスクトップマスコット`・`ゴースト` とする（議題 2 の決定）。
8. The マニフェストの雛形 shall 既定のロケールを英語（en-US）とし、追加のロケールとして日本語（ja-JP）を 1 つ持ち、表示名・短い説明を両方の言語で書く（議題 4 の決定）。
9. The マニフェストの雛形 shall 署名・インストーラー・スタートメニューのショートカット・ほかのパッケージへの依存を前提にした欄を持たない。
10. Where 要件 4 の実測で、利用者のゴースト・バルーン・記憶のどれかが消えると分かった, the マニフェストの雛形 shall 入れた直後に利用者へ示される欄に、上げ直しと外すときに何が消えるかと、その前に写しておくことを、英語と日本語の両方のロケールに書く（消える物が 0 のときは書かない）。

### Requirement 2: 雛形の置き場と、リポジトリへの影響の範囲

**Objective:** As a 提出の内容を PR で見る開発者, I want 提出するマニフェストがリポジトリの決まった場所に在り、ほかの物を動かさないこと, so that 何を winget-pkgs へ出したかを後から確かめられ、同じ時期に進むほかの spec と触るファイルが重ならない

#### Acceptance Criteria

1. The リポジトリ shall マニフェストの雛形 4 ファイルを `dist/winget/` の下に置く（その下の並びは設計で決める）。
2. When 雛形を置く、または直す, the 開発の手順 shall その雛形に `winget validate` を通し、成功したことを記録に残す。
3. The リポジトリの雛形 shall winget-pkgs へ提出したマニフェストと同じ内容である（提出の後に自動の検査を受けて直したときは、雛形へ写し戻す）。
4. The 配布 zip shall `dist/winget/` を足す前と同じ中身の構成である（`dist/winget/**` は zip に入らない）。
5. The 雛形と記録 shall トークン・パスワードなどの秘密を含まない。
6. The 本仕様 shall リポジトリで追加・変更するファイルを、`dist/winget/**`・本仕様のフォルダの中の文書・要件 2.7 の steering の 1 か所・要件 6.2 の申し送り先の brief に限り、`README.md`・`dist/README.txt`・`.github/workflows/**`・`crates/**`・`tools/**` の変更を 0 とする（完了の手続き `/kiro-complete` が直す文書は、ここでは数えない）。
7. The steering の `structure.md` shall `dist/` の説明で、`dist/winget/` が winget へ提出するマニフェストの雛形の置き場で、配布 zip には入らないことを述べる。
8. If 要件 2.6 の「変更 0」のファイルを触らないと先へ進めないと分かる, then the 本仕様 shall 作業を止め、触る必要のあるファイルと理由を開発者へ報告する。

### Requirement 3: 手元で入れて `areka` で起動する確かめ

**Objective:** As a 初回の提出を出す開発者, I want 提出するマニフェストそのもので winget から入れた areka が `areka` の 1 語で起動することを、x64 の実機で確かめること, so that 取り消しにくい提出の前に、マニフェストの欠けが無いと分かる

#### Acceptance Criteria

1. When 雛形ができる, the 開発者 shall x64 の実機で、提出するマニフェストそのもの（`InstallerUrl` は公開済みの Release の URL）を `winget install --manifest` で普段の利用者の権限から入れ、ハッシュの検証が通ってインストールが完了したことを記録に残す。
2. When 入れた後に新しい端末で `areka` と打つ, the 実機の確かめ shall areka が起動してゴーストが立ったことを、起動の記録（`本物のゴースト窓を開きました` の行が 1 件以上・`root_resolved` の行の `root=` が winget の入れ先のフォルダ）で判定する。
3. The 実機の確かめ shall `areka` の解決先（`(Get-Command areka).Source`）が winget の入れ先のフォルダの中の `areka.exe` でリンクではないことと、利用者の PATH に入れ先のフォルダが足されたことを記録に残す。
4. Where 確かめのために winget や OS の設定を変える必要がある, the 開発者 shall 自分の手でその設定を変え、何を・いつ変えたかと、確かめの後に元へ戻したことを記録に残す（AI はこれらの設定を変えない）。
5. The 実機の確かめ shall 確かめのために自分で作る物（確かめ専用のマニフェスト・起動の記録・検体）をワークツリーの `target\` の下に置き、リポジトリで追跡しない。
6. When 実機の確かめ（要件 4 の実測を含む）が終わる, the 開発者 shall winget で入れた物を winget で外し、入れ先のフォルダ・コマンド名 `areka`・PATH に足された項目が残っていないことを確かめて記録に残す。
7. The 実機の確かめ shall arm64 の項目を実機で入れることを含めず、arm64 について確かめたのは `winget validate` の成功と `InstallerSha256` が Release の `.sha256` と一致することまでであることを、既知の制限として記録に残す。
8. If 入れられない、または `areka` で起動しない, then the 開発者 shall 原因を記録に残し、マニフェストの直しで済むときは直して要件 2.2 と本要件をやり直し、原因が areka 本体か zip の側に在るときは提出へ進まずに開発者の判断を仰ぐ（本仕様では `crates/**`・`tools/**` を直さない）。

### Requirement 4: 外すとき・上げ直すときの、利用者のゴーストと記憶の実測

**Objective:** As a winget で入れた areka にゴーストを足して使っている利用者, I want areka を上げ直したり外したりしたときに、自分のゴーストと記憶がどうなるかが事前に分かっていること, so that 上げ直しで黙ってゴーストや記憶を失わない

#### Acceptance Criteria

1. When 要件 3 の起動の確かめが合格する, the 開発者 shall 入れた areka に、利用者が後から入れたゴースト 1 体・利用者が後から入れたバルーン 1 つ・areka の記憶・ゴーストの記憶とシェルの記憶（同梱のゴーストの分を含む）が入れ先のフォルダの中に在る状態を作り、在ることを記録に残す。
2. When その状態から、より新しい版を名乗る確かめ専用のマニフェストで `winget upgrade` を行う, the 実測 shall 次の 6 つを項目ごとに記録に残す: 利用者が後から入れたゴーストが残ったか／利用者が後から入れたバルーンが残ったか／areka の記憶が残ったか／ゴーストの記憶とシェルの記憶が残ったか／同梱のファイルが新しい版の中身に置き換わったか／上げ直しの後に `areka` で起動して、上げ直しの前に使っていたゴーストで立ったか。
3. When その状態から、`--purge` も `--preserve` も付けない `winget uninstall` を行う, the 実測 shall 利用者が後から入れたゴースト・利用者が後から入れたバルーン・areka の記憶・ゴーストの記憶とシェルの記憶のそれぞれが残ったかと、入れ先のフォルダが残ったかと、winget が出した文を記録に残す。
4. The 実測の記録 shall 各項目を「残った」「消えた」「置き換わった」のどれかで明示し、消えた物が無いときも「消えた物は 0」と書く。
5. The 実測の記録 shall この結果が手元のマニフェストで入れた形（入れ先のフォルダの名前と `winget list` の ID が winget-pkgs から入れた形と違う）でのものであることを、既知の制限として述べる。あわせて、winget の設定のうち「外すときにポータブルな物の入れ先を丸ごと消す」（`--purge` を既定にする設定）が既定のまま（オフ）で測ったことを書く。
6. If 利用者のゴースト・バルーン・記憶のどれかが消えると分かる, then the 本仕様 shall 結果を開発者へ報告し、利用者の物が winget の上げ直しと外し方で消えないようにする仕事を別の spec として `/kiro-discovery` で起票し、要件 1.10 の注意書きを雛形に足して要件 2.2 と要件 3.1 をやり直したうえで、名乗りを押さえるために初回の提出を 1 回だけ進める（議題 5 の決定。本仕様では置き場を変えない）。

### Requirement 5: winget-pkgs への初回の提出

**Objective:** As a areka を winget で配りたい開発者, I want 初回の提出の手順が書かれていて、そのとおりに PR を出して自動の検査を通せること, so that 人の承認を待つだけの状態まで、迷わずに進められる

#### Acceptance Criteria

1. The 本仕様 shall 初回の提出の手順（開発者のアカウントに winget-pkgs のフォークを用意する・使う道具と入れ方・打つコマンド・トークンをリポジトリにも記録にも書かないこと）を、本仕様のフォルダの中の文書に書く（道具は `komac` か `wingetcreate` のどちらかを設計で選ぶ）。
2. When 要件 3 が合格し、要件 4 の記録が済む, the 開発者 shall 自分の手で、自分のアカウントの winget-pkgs のフォークから microsoft/winget-pkgs へ、`Areka.Areka.Portable` の 1 つの版だけを含む PR を出す（AI は開発者のトークンを扱わず、提出の操作をしない）。
3. The 初回の提出 shall 提出する版を着手のときに決めて記録に残し（議題 3 の決定＝`v0.0.2`）、その版の公開済みの Release の zip を指す。
4. When PR を出す, the 本仕様 shall PR の URL と、自動の検査の結果（付いた印と、赤のときはその文）を記録に残す（PR の印と文は、AI が GitHub への読むだけの問い合わせで読んでよい。提出の操作とトークンは開発者の手のまま）。
5. If 自動の検査が赤で、マニフェストの直しで消える, then the 開発者 shall 同じ PR の中で直し、直した内容を雛形へ写し戻して、要件 2.2 をやり直す。
6. If 自動の検査の赤がマニフェストの直しでは消えない（`areka.exe` がウイルス対策の走査に掛かる・無人のインストールかアンインストールが通らない、など）, then the 本仕様 shall 原因を記録に残して開発者へ報告し、原因を直す仕事を別の spec として `/kiro-discovery` で起票する（本仕様では `crates/**`・`tools/**` を直さない）。
7. The 本仕様 shall 「PR の自動の検査が通り、人の承認待ちになった」ことを完了の線とし、winget-pkgs への取り込みを待たない。自動の検査が通るまでは完了としない。
8. The 本仕様 shall 更新の PR を自動で出す workflow と、説明書の winget の行を作らない（`areka-P0-winget-release-automation` が、取り込みの後に作る）。

### Requirement 6: 記録と、後ろの spec への申し送り

**Objective:** As a 後ろの spec（`areka-P0-winget-release-automation`）を始める開発者, I want 本仕様で確かめたことと提出の状態が、決まった場所に書かれていること, so that 説明書の文と自動化を、測った事実の上に作れる

#### Acceptance Criteria

1. The 本仕様 shall 確かめと提出の記録を本仕様のフォルダの中（`verification/`）に置き、機械と winget の版・コミット・変えた設定と戻した時刻・手順と結果・PR の URL を書く。
2. When 本仕様が完了する, the 本仕様 shall `areka-P0-winget-release-automation` の brief へ次を書き足す: 要件 4 の実測の結果（項目ごとの「残った」「消えた」「置き換わった」）／`ArchiveBinariesDependOnPath` をどうしたかと、`areka` の解決のされ方／winget-pkgs への PR の URL と、その時点の状態（承認待ち）／直しを求める印が付いたら期限（要件生成時の文書では 10 日）までに応えないと PR が閉じられること／要件 4 で消える物が在ったときは、要件 4.6 で起票した spec の名前と、その spec が着地するまで説明書に winget の行を載せず、winget-pkgs へ次の版を出さないこと（上げ直しのたびに利用者の物が消えるため。初回の 1 版は名乗りを押さえるために載せたままにする）。
3. If 実機の確かめか実測で、areka の側の未対応か不具合のためにうまくいかなかった件が見つかる, then the 本仕様 shall 本仕様の範囲の外でも、その件をすべて `/kiro-discovery` で起票する。
