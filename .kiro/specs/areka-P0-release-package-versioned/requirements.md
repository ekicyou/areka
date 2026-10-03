# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: areka を winget と GitHub Releases で配りたい開発者と、winget で areka を入れる利用者。今の配布スクリプト（`tools/package-alpha.ps1`）は x64 の zip だけを、日付とコミットの入った名前（`areka-alpha-x64-{日付}-{コミット}.zip`）で作り、中身のハッシュ（SHA256）を出さない。winget は同じ URL の中身が替わると検査（`Validation-Hash-Verification-Failed`）で落ち、既に出ている版のインストールまで失敗するので、**版ごとに固定の名前**の zip とそのハッシュが要る。arm64 の zip は作ったことがない。さらに winget の portable は PATH から呼べるようにリンク（`%LOCALAPPDATA%\Microsoft\WinGet\Links\areka.exe`）を作るが、areka は根・補助 exe・記憶の置き場をすべて「起動した exe の場所」から引くので、リンクの場所を exe の場所と取り違えると winget から入れた areka は起動しない。あわせて、起動確認（`-Check`）のたびに展開した zip と起動の記録が `%TEMP%` に残り、誰も片付けない（2026-10-02 に 38 個を手で消した）——開発者の決まり「一時フォルダはワークツリーの `target\` の下だけ」に道具の既定のふるまいが反している（合流した `package-check-temp-cleanup`）。

**今の状態**: 版の正本は `Cargo.toml` の `[workspace.package] version`（今は `0.0.1`）1 か所。配布スクリプトは版を読まない。`BUILD-INFO.txt` は `commit=`・`dirty=`・`built=`・`script=`・`rustflags=` を持ち、`version=` は無い。中身の検査（8 項目）の機械種別の確認は x64（`areka.exe`＝0x8664）に固定。`-Check` は既定で `%TEMP%` の下へ展開し、`-CheckDir` はリポジトリの中を断り、後始末は展開した木も記録も消さない。`.EXAMPLE` は `-CheckDir C:\t` を勧めている。areka 本体の根・補助 exe・記憶の置き場は、起動した exe のパスをそのまま使い、リンクを解かない。

**何を変えるか**: ① `-Check` の展開先と記録を `target\` の下へ移し、判定の後に展開した木を片付ける。② 配布 zip を版入りの固定の名前（`areka-{版}-x64.zip`・`areka-{版}-arm64.zip`）で作り、隣に SHA256 を置く。③ x64 と arm64 を同じ手順で作り分ける。④ `BUILD-INFO.txt` に版を記す。⑤ 手元でも CI でも同じスクリプトで動く形にする。⑥ areka がリンク経由で起動されても、リンクの先（exe の本当の場所）から根・補助 exe・記憶の置き場を引く。

> 起票: 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）。同日 `package-check-temp-cleanup` を合流。要件生成: 2026-10-03（main `1ce4c74e`）。

## Introduction

本仕様は、配布物の zip を **winget と GitHub Releases に乗せられる形**（版入りの固定の名前・SHA256・x64 と arm64）で作れるようにし、**winget の portable が作るリンク経由で起動しても areka が動く**ようにする。あわせて、配布スクリプトの起動確認が `%TEMP%` に残していた展開物と記録を、ワークツリーの `target\` の下へ移して片付ける（道具のバグの合流）。

要件生成時に引き直した事実（設計はこれを再検証する）:

- 配布スクリプト `tools/package-alpha.ps1`（541 行）の引数は `-Check`（スイッチ・既定は付けない＝起動確認をしない）・`-CheckDir`・`-SmokeExitMs` の 3 つ。終了コードは 0＝成功・1＝組む段の失敗（段の名前を印字・git status の変化もここ）・2＝起動確認の否・3＝引数の不正か前提の欠け（直し方を 1 行印字）。
- zip の名前は `target/alpha/areka-alpha-x64-<yyyyMMdd>-<コミット 7 桁>[-dirty].zip`。版はどこからも読んでいない。ビルドの三つ組（x64・i686）は固定で、arm64 の分岐は無い。
- 中身の検査 8 項目のうち 5 番（機械種別）は `areka.exe`＝0x8664・`shiori-host32-helper.exe`＝0x014c・`pasta.dll`＝0x014c を期待する。arm64（0xAA64）の場合は無い。6 番は 2 つの exe が Visual C++ 再頒布可能パッケージの DLL を取り込まないことを確かめる。
- `-Check` の展開先は `-CheckDir` か `%TEMP%` の下の `areka-alpha-check-<HHmmss>`、記録は同じ名前に `-logs` を付けた隣のフォルダ。展開先のフルパスの上限は 160 字（`emo2` の SHIORI が長いパスで黙るため）。後始末（`Invoke-Cleanup`）は作りかけの zip の削除・子プロセスの停止・環境変数の復元だけで、展開先と記録を消さない。番犬は自分が起こした子プロセスだけを止める。起動の記録の判定は 7 行。
- `-CheckDir` のリポジトリの中を断る理由は完了 `areka-P0-alpha-package` の要件 3.1（「リポジトリの外の新しい空の、パスの短い場所へ展開」）。
- 長さの実測（棚卸⑳）: ワークツリーの根が 64 字のとき `<根>\target\alpha-check\areka-alpha-check-HHmmss` は約 107 字＝160 字に収まる。
- areka 本体で起動した exe のパス（`current_exe()`）を使うのは `crates/areka/src/boot_config.rs` の 3 か所だけ（根＝`resolve_root`・補助 exe＝`default_helper_exe_path`・記憶の置き場＝`default_app_profile_dir`）。根の解決は `canonicalize` を使わない（長いパスの接頭辞 `\\?\` を持ち込まないため・関数の説明に明記）。`AREKA_ROOT`・`AREKA_PROFILE_DIR` が在ればそれが勝つ。
- `.github/` は無い。`dist/README.txt` は zip の名前にも配布スクリプトにも触れていない（「入れ方」の文面の変更は本仕様では 0 で済む）。
- 配布スクリプトの自己検査（`tools/` の中のテスト）は無い。`tools/test-all.ps1` は配布スクリプトを呼ばない。

## Boundary Context

- **In scope**: ① `-Check` の展開先・記録の置き場・後始末（合流した道具のバグ）／② 版入りの固定の名前と SHA256／③ `-Arch` による x64・arm64 の作り分けと、arm64 の zip の中身の検査／④ `BUILD-INFO.txt` の `version=`／⑤ 手元と CI で同じスクリプトが動く形（`-Check` を付けなければ窓を出さず、問いかけもしない）／⑥ areka がリンク経由で起動されたときの場所の解決（本体のコードと決定論テスト）／⑦ x64 の実機で 1 回、手元のマニフェストで winget から入れて起動する確かめ／⑧ 配布スクリプトの使い方の説明と steering（`structure.md`・`tech.md`）の追随。
- **Out of scope**: GitHub Actions の workflow そのもの（`areka-P0-release-ci-workflow`）／winget のマニフェストの作成と提出（`areka-P0-winget-manifest-submission`）／版を上げる手順とタグ（`areka-P0-release-cycle`）／crates.io への公開（`areka-P0-crates-io-publish`）／コード署名（`areka-P0-release-code-signing`）／exe の `--version` と Windows のファイル版の資源／インストーラー（Inno など・winget の名乗り `Areka.Areka` はインストーラー版に空けておく）／補助 exe の arm64 版（32 ビットの SHIORI を動かすため i686 のまま＝設計どおり）／arm64 の実機での起動の確かめ／手元と CI のバイト単位で同じ zip（再現可能ビルド）／中継 exe の同梱（`areka-P0-mcp-stdio-bridge`・本仕様の後）。
- **変更 0 と明記するもの**: `dist/README.txt`（`crates-io-publish`・`winget-manifest-submission` が触る）・`crates/areka/src/main.rs`・`crates/areka/src/emo2_boot/`・`crates/areka/src/install/`・各 `Cargo.toml` の依存と版・zip の中身の構成（最上位の項目・ゴースト・バルーンの許可表）・起動の記録の判定の各行・`tools/test-all.ps1`・過去の記録（`alpha-release-signoff` の受入記録に書いた `%TEMP%` のパス）。
- **上書きする完了 spec の要件**: 完了 `areka-P0-alpha-package` の要件 3.1 のうち「リポジトリの外へ展開」を、本仕様の要件 1 の「ワークツリーの `target\` の下へ展開し、判定の後に片付ける」で置き換える（「新しい空の、パスの短い場所」と 160 字の上限は保つ）。同 spec の Out にあった「ARM64 版の zip」を本仕様が引き取る。
- **Adjacent expectations**: 下流の `release-ci-workflow` は本仕様の配布スクリプトを CI から `-Check` なしで呼び、`areka-{版}-{arch}.zip` と `.sha256` を Release へ載せる。`winget-manifest-submission` は zip の名前と SHA256 を使う。`install-companion-canon`（C2）は `boot_config.rs` を触りうるので本仕様の後に着手する。完了 `areka-P0-baseware-root-layout` の根の規則（根は exe の隣・環境変数が勝つ）は変えず、「exe の隣」の「exe」をリンクの先と読むだけにする。

## 要件生成で決めた点と、要件討議の議題（仮置き）

brief が要件討議へ回した議題 3 件は、下の仮置きで要件を書いた（答えで変わる条項を併記）。**2026-10-03 の要件討議で 3 件とも仮置きのまま確定した**（いずれも勝者が明白で、答えで作業が変わる余地が小さいため議題に上げず、結果だけを開発者へ報告した。⑴ は要件 8.6 の旧名の直しも伴う）。

- **議題 ⑴ スクリプトの名前を `tools/package.ps1` へ改めるか** → 仮置き＝**改める**（α を出し終えた後も使う道具で、名前の「alpha」が誤解を招く。下流の `release-ci-workflow` が呼ぶ前に決めるのが安い）。旧名の置き換え用の薄い口は作らない（呼び手は文書だけ）。過去の記録にある旧名は書き換えない。影響する条項: 要件 8.1・8.3。
- **議題 ⑵ 判定が否のときに展開した木を証拠として残すか** → 仮置き＝**否のときは残す**（置き場を印字する）。合格のときは消す。どちらのときも、残す引数を付ければ残す。理由: 否の原因を調べるには展開した木（areka が書いた記憶を含む）が要り、残さないと release ビルドからやり直しになる。置き場は `target\` の下なので開発者の決まりには反しない。影響する条項: 要件 1.4・1.5。
- **議題 ⑶ 展開した木を消せなかったときの終了コード** → 仮置き＝**新しい番号は作らず 1（段の失敗）**。「後片付け」を段の 1 つとして扱い、段の名前・消せなかったパス・理由を印字する（今の作りで「git status の変化」も 1 に入る形に合わせる）。影響する条項: 要件 1.7・5.4。

生成時に決めた点（勝者が明白なので議題にしない・討議で覆してよい）:

- **`-Arch` の既定は `x64`**。arm64 の道具の無い機械で既定の実行が止まらないようにするため。CI（`release-ci-workflow`）は `-Arch all` を明示して呼ぶ。
- **リンクを解くのは「起動した exe のパスがリンクのとき」だけ**。リンクでないときは今と同じパス（ドライブ文字・`subst`・割り当てたネットワークドライブの綴りも含めて）を使う。理由: 常に最終パスへ解くと、短い場所に見せるために `subst` したドライブなどが長い実パスへ展開され、`emo2` の 160 字の上限を超えて黙る起動を新しく生みうる。brief の Approach が挙げた最終パスへの解決は、このふるまいを満たす範囲で設計が手段を選ぶ。
- **`BUILD-INFO.txt` に CPU 種別の行も足す**（要件 4.2）。zip の名前を変えられても、中の記録だけで x64 か arm64 かが分かるようにするため（brief は `version=` だけを挙げる・1 行の追加）。
- **起動確認（`-Check`）は x64 の zip で行う**（brief の Constraints「実機の確かめは x64」）。arm64 の zip は作れて中身の検査が通るところまで。

## Requirements

### Requirement 1: 起動確認の展開物と記録を `target\` の下に置き、片付ける

**Objective:** As a 配布物を作る開発者, I want 起動確認（`-Check`）が展開した zip と記録をワークツリーの `target\` の下に置き、判定の後に展開した木を片付けること, so that `%TEMP%` に誰も消さないフォルダが溜まらず、一時フォルダを `target\` の下に限る決まりを道具が守る

#### Acceptance Criteria

1. When 開発者が `-CheckDir` を付けずに `-Check` を実行する, the 配布スクリプト shall 展開先と起動の記録の置き場を、そのワークツリーの `target\` の下の新しい空のフォルダに作り、`%TEMP%`（利用者の一時フォルダ）には何も作らない。
2. If 展開先のフルパスが 160 字を超える, then the 配布スクリプト shall 展開せずに終了コード 3 で止め、パスの長さと上限と短くする手段を印字する（既定の置き場でも `-CheckDir` を付けたときでも同じ）。
3. When 開発者が `-CheckDir` にリポジトリの中を指定する, the 配布スクリプト shall それがワークツリーの `target\` の下なら受け付け、`target\` の外なら終了コード 3 で断る（リポジトリの外の指定は今どおり受け付ける）。
4. When 起動確認の判定が合格する, the 配布スクリプト shall 展開した木（起動した `areka.exe` が書いた記憶を含む）を消し、起動の記録（`run.log`・`run.stderr.log`）だけを残してその置き場を印字する。
5. If 起動確認の判定が否になる, then the 配布スクリプト shall 展開した木と起動の記録を残し、両方の置き場を印字する（議題 ⑵ の仮置き）。
6. Where 開発者が展開した木を残す引数を付ける, the 配布スクリプト shall 判定の合否にかかわらず展開した木を消さず、その置き場を印字する。
7. If 展開した木を消せない, then the 配布スクリプト shall 消せなかったパスと理由と段の名前を印字して終了コード 1 で終わる（黙って残さない・議題 ⑶ の仮置き）。
8. The 配布スクリプト shall 展開した木を消すのを、自分が起こした子プロセスが終わった（または番犬が止めた）後に限り、自分が起こしたと確かめていないプロセスを止めない。
9. The 配布スクリプト shall 最後の「git status 不変の確認」の段を今どおり行い、`target\` の下への書き込みで合否が変わらない。
10. The 配布スクリプト shall 使い方の説明（`Get-Help` で出る欄）の例と引数の説明で、`target\` の外の一時フォルダ（`C:\t` など）を勧めない。

### Requirement 2: 版入りの固定の名前と SHA256

**Objective:** As a 配布物を winget と GitHub Releases に載せる開発者, I want zip の名前が版とCPU 種別だけで決まり、隣にそのハッシュが置かれること, so that 版ごとに URL とハッシュが固定され、winget の検査が通り続ける

#### Acceptance Criteria

1. When 配布スクリプトが zip を組む, the 配布スクリプト shall 版をワークスペースの `Cargo.toml` の `[workspace.package]` の `version` から読み、zip の名前を `areka-{版}-{arch}.zip`（`{arch}` は `x64` か `arm64`・例 `areka-0.0.1-x64.zip`）にする（版を引数で受け取らない・日付・コミット・未コミットの印を名前に入れない）。
2. When zip を組み終える, the 配布スクリプト shall 同じフォルダに `areka-{版}-{arch}.zip.sha256` を置き、その中身を「zip の SHA256（16 進 64 字）・空白 2 つ・zip のファイル名」の 1 行にする（`sha256sum -c` がそのまま検証できる形）。
3. If 版を読めない（`Cargo.toml` が読めない・`version` が空など）, then the 配布スクリプト shall 何もビルドせずに終了コード 3 で止め、読めなかった理由を印字する（仮の名前の zip を作らない）。
4. When 同じ版・同じ CPU 種別の zip か `.sha256` が出力先に既に在る, the 配布スクリプト shall ビルドを始める前にその組を消し、成功したときだけ新しい zip と `.sha256` の組を置く（前回の zip が今回の失敗の後に完成品に見えて残らない・zip と `.sha256` の組を食い違わせない）。
5. If どこかの段が失敗する, then the 配布スクリプト shall 完成品に見える zip も `.sha256` も残さない（作りかけは消す・完了 `alpha-package` の要件 1.5 を `.sha256` へ広げる）。
6. When zip を組み終える, the 配布スクリプト shall 作った zip と `.sha256` の絶対パスと版を CPU 種別ごとに印字する。

### Requirement 3: x64 と arm64 を同じ手順で作り分ける

**Objective:** As a x64 と arm64 の利用者へ配る開発者, I want 1 つの引数で x64・arm64・両方の zip を作り分けられること, so that arm64 の zip も x64 と同じ中身の規則と検査で作れる

#### Acceptance Criteria

1. The 配布スクリプト shall `-Arch` 引数で `x64`・`arm64`・`all`（両方）を受け付け、省いたときは `x64` として扱う。
2. If `-Arch` に上の 3 つ以外の値が渡される, then the 配布スクリプト shall 何もビルドせずに終了コード 3 で止め、受け付ける値を印字する。
3. When `-Arch` が `arm64` か `all` である, the 配布スクリプト shall arm64 の `areka.exe` をその場のソースから release ビルドし、zip に arm64 の `areka.exe` と i686（32 ビット）のままの `shiori-host32-helper.exe` を入れる。
4. If `-Arch` が `arm64` か `all` で、arm64 のビルドに要る道具が開発機に無い, then the 配布スクリプト shall 何もビルドせずに終了コード 3 で止め、足りない道具と入れ方を印字する（黙って x64 だけを作らない）。
5. The 配布スクリプト shall arm64 の zip にも x64 の zip と同じ中身の検査 8 項目を効かせ、機械種別の検査では `areka.exe` が arm64（0xAA64）、`shiori-host32-helper.exe` と `pasta.dll` が 32 ビット（0x014c）であることを判定する（Visual C++ 再頒布可能パッケージに頼らない検査も arm64 の `areka.exe` に効かせる）。
6. The 配布スクリプト shall arm64 の zip の中身の構成（最上位の項目・ゴースト・バルーン・文書）を x64 の zip と同じにし、違いを `areka.exe` の CPU 種別だけにする。
7. When `-Arch` が `all` である, the 配布スクリプト shall 両方の zip と `.sha256` がそろったときだけ成功（終了コード 0）とし、どちらかが失敗すればもう一方を完成品として残さず、失敗した段と CPU 種別を印字する。
8. When `-Check` が付き、作る zip に x64 が含まれる, the 配布スクリプト shall x64 の zip で起動確認を行い、arm64 の zip を作ったときは「arm64 の起動確認はしていない」ことを印字する。
9. If `-Check` が付き、`-Arch` が `arm64` だけである, then the 配布スクリプト shall 何もビルドせずに終了コード 3 で止め、起動確認には x64 の zip が要ることを印字する。

### Requirement 4: `BUILD-INFO.txt` に版を記す

**Objective:** As a 配布物を受け取った人と開発者, I want zip の中の `BUILD-INFO.txt` で版とコミットを確かめられること, so that 手元の zip がどの版のどのコミットから組まれたかを取り違えない

#### Acceptance Criteria

1. When 配布スクリプトが zip を組む, the 配布スクリプト shall `BUILD-INFO.txt` に `version={版}` の行を、今ある `commit=`・`dirty=`・`built=`・`script=`・`rustflags=` の行と並べて書く（版は要件 2.1 と同じもの）。
2. The 配布スクリプト shall `BUILD-INFO.txt` に CPU 種別を示す行を書き、zip の名前の `{arch}` と一致させる。
3. When 中身の検査を行う, the 配布スクリプト shall `BUILD-INFO.txt` の `version=` と CPU 種別の行が zip の名前と一致することを判定し、外れれば段の失敗（終了コード 1）とする。

### Requirement 5: 手元と CI で同じスクリプトが動く

**Objective:** As a 配布を CI に任せる開発者, I want 手元と CI の Windows ランナーで同じスクリプトを同じ引数で呼べること, so that 手元で確かめた手順のまま、タグをきっかけに CI が同じ名前と中身の zip を作れる

#### Acceptance Criteria

1. While `-Check` が付いていない, the 配布スクリプト shall 窓を出さず、利用者への問いかけ（入力待ち）をせず、表示の無い環境でも最後まで動く。
2. The 配布スクリプト shall 手元と CI で同じ引数から同じ名前の zip を作り、同じ中身の検査で合否を決める（バイト単位で同じ zip であることは求めない）。
3. If 前提の道具（`cargo about`・`cargo deny`・ビルドのターゲット・arm64 の道具など）が欠ける, then the 配布スクリプト shall 終了コード 3 で止め、欠けたものと入れ方を 1 行で印字する（CI の workflow がそのまま読める形）。
4. The 配布スクリプト shall 終了コードの意味（0＝成功・1＝段の失敗・2＝起動確認の否・3＝引数の不正か前提の欠け）を今どおりに保ち、新しい番号を足さない。
5. The 配布スクリプト shall zip と `.sha256` の置き場を、リポジトリの中の追跡外のフォルダ（`target\` の下）の決まった場所にし、その場所を使い方の説明に書く（CI が拾う場所を固定する）。

### Requirement 6: リンク経由で起動されても本当の場所から引く

**Objective:** As a winget で areka を入れた利用者, I want コマンド名 `areka` で（winget が作ったリンク経由で）起動しても areka が正しく立ち上がること, so that 入れた直後にコマンド 1 つで使い始められる

#### Acceptance Criteria

1. When `areka.exe` がシンボリックリンク経由で起動される, the areka shall 根・補助 exe（`shiori-host32-helper.exe`）・記憶の置き場（`profile\areka`）を、リンクの置き場ではなくリンクの先の `areka.exe` が在るフォルダから引く。
2. When リンクが別のリンクを指している、または相対のパスで先を指している, the areka shall 最後にたどり着いた `areka.exe` の在るフォルダから要件 6.1 の 3 つを引く。
3. When `areka.exe` がリンクを経ずに起動される, the areka shall 今と同じパス（ドライブ文字・`subst` したドライブ・割り当てたネットワークドライブの綴りを含めて）から要件 6.1 の 3 つを引く（今の起動のふるまいを変えない）。
4. While 環境変数 `AREKA_ROOT`・`AREKA_PROFILE_DIR` が設定されている, the areka shall 今どおりその値を使い、リンクの解決はその値に及ぼさない。
5. The areka shall リンクを解いて得た場所を、長いパスの接頭辞（`\\?\`）を付けない普通の綴りで根・補助 exe・記憶の置き場・SHIORI へ渡す（`emo2` の 160 字の上限を接頭辞で食わない）。
6. If リンクの先を解けない, then the areka shall 解けなかった理由と起動した exe のパスを警告として記録に残し、起動した exe のパスをそのまま使って続ける（黙らない・今のふるまいへ戻る）。
7. The areka shall 要件 6.1〜6.6 の判断を決定論テストで固定する（シンボリックリンクを作れない環境でも判断の分岐が検査される形にする）。
8. The areka shall 使った根のフォルダを起動の記録に 1 行出す（今ある info の行「ベースウェアの根を決めました」（`root_resolved`）の `root=` を使い、新しい行は足さない。実機の確かめで、リンクの先から引いたことを記録で判定できるようにする）。

### Requirement 7: winget から入れて起動する実機の確かめ

**Objective:** As a α の後の最初の配布を出す開発者, I want 手元のマニフェストで winget から入れた areka が `areka` の 1 語で起動することを x64 の実機で 1 回確かめること, so that 本物のマニフェストを提出する前に、リンク経由の起動の欠けが無いと分かる

#### Acceptance Criteria

1. When 本仕様の実装が終わる, the 開発者 shall 本仕様の配布スクリプトで作った x64 の zip を、手元のマニフェスト（portable・zip の入れ子）で `winget install --manifest` から入れ、PowerShell で `areka` と打って起動し、ゴーストが立つことを確かめる。
2. The 実機の確かめ shall リンク経由の起動の経路を実際に通すため、手元のマニフェストに PATH へ入れ先のフォルダを足す指定（`ArchiveBinariesDependOnPath`）を付けずに行い、`areka` がリンク（`%LOCALAPPDATA%\Microsoft\WinGet\Links\` の下・`(Get-Command areka).Source` の `LinkType` が `SymbolicLink`）を指していたことと、起動の記録の根（要件 6.8 の `root=`）がリンクの先のフォルダであったことを、両方記録に残す。winget がリンクを作らずに入れ先のフォルダを PATH へ足した場合は、起動できても本要件の確かめは済んでいないものとして扱う（片方だけでは PATH 経由の起動と見分けられない）。
3. The 実機の確かめ shall 手元のマニフェストと検体の置き場・記録をワークツリーの `target\` の下に置き、リポジトリで追跡しない（提出用のマニフェストは `winget-manifest-submission` の持ち物）。確かめの後に winget で入れたものは winget で外す。
4. Where 確かめのために winget や OS の設定を変える必要がある, the 開発者 shall 自分の手でその設定を変え、何を変えたかと、確かめの後に元へ戻したかを記録に残す（2026-10-03 要件討議の裁定＝確かめの間だけ OS の開発者モードをオンにし〔管理者でない利用者の権限でも winget がシンボリックリンクを作れるようにするため〕、管理者の手で winget の `LocalManifestFiles` をオンにし、`winget install` そのものは普段の利用者の権限で走らせる。終わったら両方を元へ戻す。AI はこれらの設定を変えない）。
5. The 実機の確かめ shall arm64 の zip の起動を含めず、「arm64 の実機での起動は開発者の手元に機械が無ければ利用者の報告待ち」であることを既知の制限として記録に残す。
6. The 実機の確かめ shall 手元のマニフェストへ渡す zip を、ローカルのファイルのパスでなく、`target\` の下の zip を配る `localhost` の http の URL で渡す（ローカルのパスを `InstallerUrl` に書くと期待どおり動かない既知の問題〔winget-cli #4358〕を避ける・配る道具は設計で決める）。

### Requirement 8: 使い方の説明と steering の追随

**Objective:** As a 配布スクリプトを使う開発者と後続の spec, I want 新しい名前・引数・置き場・arm64 の手順が説明と steering に書かれていること, so that 下流の CI と winget の spec が実物と食い違う前提で作られない

#### Acceptance Criteria

1. The 配布スクリプト shall `tools/package.ps1` の名前で置き、旧名 `tools/package-alpha.ps1` を残さない（議題 ⑴ の仮置き）。
2. The 配布スクリプト shall 使い方の説明（`Get-Help` で出る欄）に、`-Arch` の値と既定・zip と `.sha256` の名前と置き場・`-Check` の展開先と記録の置き場・展開した木を残す引数・終了コードの意味・「`-Check` は窓を出すので CI では付けない」ことを書く。
3. The steering の `structure.md` shall `tools/` の説明で、配布スクリプトの新しい名前と役割（版入りの zip と SHA256・x64 と arm64・起動確認）を述べる。
4. The steering の `tech.md` shall arm64 の zip を作るのに要る道具（arm64 のビルドのターゲットと Visual Studio の ARM64 の道具）と、補助 exe は arm64 の zip でも i686 のままであることを書く。
5. The リポジトリ shall 過去の記録（完了 spec の文書・受入記録・`roadmap-history.md`）に書かれた旧名と `%TEMP%` のパスを書き換えない。
6. The リポジトリ shall いま生きている文書の旧名 `tools/package-alpha.ps1`（steering の `roadmap.md`・`product.md` と、まだ着手していない spec の brief〔`mcp-stdio-bridge`・`mcp-server-core`・`release-ci-workflow`〕）を新しい名前へ直す（下流の spec が旧名を前提に作られないようにする）。
