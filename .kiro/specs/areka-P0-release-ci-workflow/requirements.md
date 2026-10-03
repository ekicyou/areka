# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: areka を GitHub Releases（と、その後段の crates.io・winget）で配りたい開発者。リリースのたびに x64 と arm64 の zip を作り、SHA256 を添えて GitHub Release を作る作業を、人の手でなく機械に任せたい。ただし**きっかけは開発者が決める**＝普通の PR のマージでは何も起きず、版を上げるコミットに開発者が付けたタグ（`v0.0.2` の形）が押されたときだけ動く。無料で済ませる（公開リポジトリ向けの GitHub Actions の無料枠・Windows の実行環境）。

**今の状態**: `.github/` は無い。GitHub Releases は 0 件。タグは `v0.0.1` が 1 つ（完了 `crate-name-reservation` が crates.io の名前を押さえたとき）。`.kiro/steering/tech.md` は「外部 CI は持たない」と書く（GUI・WUC・GPU のテストがホスト型の実行環境で再現できないため）。zip を作るのは配布スクリプト `tools/package.ps1`（完了 `release-package-versioned` が版入りの名前・SHA256・arm64・CI 向けの呼び方を足した）。署名の仕組みは無い（未署名で出す＝テーマの決めごと）。

**何を変えるか**: ① タグ `v*` が押されたときだけ動く workflow を置き、版とタグの一致を確かめ、配布スクリプトで x64 と arm64 の zip と SHA256 を作り、Release を公開する。② 失敗したら Release を残さない。③ 手で Release を作らない乾いた走りを試せる。④ `tech.md` の「外部 CI は持たない」を「テストの門は手元・ビルドと配布は Actions」に改める。

> 起票: 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）。要件生成: 2026-10-03（main `d4f9e93d`）。同日、要件生成中に上げた 3 つの問い（後段の起こし方・初回の実走・マージ前の実行環境での走り）と版の決め方への答えを開発者から受け、下の「要件生成で決めた点」に記録した。

## Introduction

本仕様は、開発者がタグを押したときだけ動く GitHub Actions の workflow（以下「リリース workflow」）を置き、配布物の zip と SHA256 を x64 と arm64 で作って GitHub Release を公開できるようにする。テスト（門）は手元のフルテストのまま変えず、ビルドと配布だけを Actions に乗せる。

要件生成時に引き直した事実（設計はこれを再検証する）:

- 配布スクリプト `tools/package.ps1`（スクリプトの版 `2.0.0`）の引数は `-Arch`（`x64`・`arm64`・`all`・既定 `x64`）・`-Check`・`-CheckDir`・`-KeepExpanded`・`-SmokeExitMs`。説明の `.EXAMPLE` が「CI の形」として `-Arch all`（起動確認なし）を挙げる。
- 作る物は `target/package/areka-{版}-{arch}.zip` と `target/package/areka-{版}-{arch}.zip.sha256`（`{arch}` は `x64`・`arm64`）。完成品の名前の物は全段が緑（終了コード 0）のときだけ現れ、失敗した走りは完成品を消して終わる。終了コードは 0＝成功・1＝段の失敗（段の名前を印字）・2＝起動確認の否・3＝引数・前提の不正（道具が無いときも）。
- 配布スクリプトは前提として `cargo about`（版 0.9.2）と `cargo deny`（版 0.20.2）を要し、arm64 を作るには Visual Studio の ARM64 の道具（`Microsoft.VisualStudio.Component.VC.Tools.ARM64`）を要する（無ければ終了コード 3）。i686 と arm64 の rustup のターゲットは配布スクリプトが自分で足す。
- 配布スクリプトは始めと終わりの `git status --porcelain` が同じことを確かめる（追跡しているファイルを書き換えない）。検体の `emo2` はリポジトリに入っている `vendors/sample_ghost/emo2.nar` から展開する。
- 版の正本は `Cargo.toml` の `[workspace.package] version`（今は `0.0.1`）。配布スクリプトが受け付ける版の形は `^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$`（`+` の付記は受けない）。
- `tech.md` の該当の段落（「外部 CI は持たない」で始まる段落・2026-09-24 に GitHub Actions への移管を見送った経緯）と、`structure.md` の「その他の最上位」の行（`.github/` の記載は無い）。
- 手元で `-Check` を通してから版を上げることは、`release-cycle` の brief の手順 1（前提の確認）に既に置かれている。
- GitHub Actions の決まり（2026-10-03 に公式文書で確認）: その走りに GitHub が発行する一時のトークン（`GITHUB_TOKEN`）で起こした出来事は、手で始める型（`workflow_dispatch`）とリポジトリへの合図（`repository_dispatch`）を除き、**新しい workflow の走りを起こさない**。また、手で始める型は **workflow のファイルが既定の枝（main）に在るときだけ**受け付けられる。

## Boundary Context

- **In scope**: タグ `v*` だけで動くきっかけ／版とタグの一致の検査／配布スクリプトの呼び出しによる x64・arm64 の zip と SHA256／Release の公開（失敗したら残さない）／乾いた走り／秘密と権限の扱い／後段が見分けられる終わり方（後段への口）／マージ前に実行環境で 1 回通した証跡／`tech.md`・`structure.md` の改め。
- **Out of scope**: テストの実行（門は手元のフルテスト `tools/test-all.ps1`）／配布スクリプトの中身（完了 `release-package-versioned`）／crates.io への公開（`areka-P0-crates-io-publish`・自分の workflow）／winget への提出（`areka-P0-winget-manifest-submission`）／署名（`areka-P0-release-code-signing`）／版を選ぶこと・版を上げること・タグを打つこと（`areka-P0-release-cycle`＝「指定が無ければ +0.0.1」の決まりも含めてリリースの手順の持ち物。リリース workflow は版を上げる入力を持たない）／zip の起動確認（窓を出すので手元）／初回の実走 `v0.0.2`（本仕様のマージの後に `release-cycle` の初回が起こし、見守る）／後段の workflow を呼ぶこと・合図を送ること。
- **変更 0 と明記するもの**: `crates/` の下・`tools/` の下（`tools/package.ps1`・`tools/test-all.ps1` を含む）・各 `Cargo.toml`・`Cargo.lock`・`dist/README.txt`・既存のタグ `v0.0.1`。配布スクリプトに直しが要ると分かったら、本仕様の中で直さず、直す spec を起票して止める（`release-package-versioned` は完了済みのため）。
- **Adjacent expectations**: `release-cycle` は、手元のフルテストと `-Check` を通し、版を上げる PR を squash マージし、main のそのコミットに `v{版}` のタグを打って押す。リリース workflow はそのタグを受けて動く。`crates-io-publish`・`winget-manifest-submission` の workflow は、リリース workflow の走りが終わったこと（`workflow_run`）を受け、「タグの push で始まった走り」かつ「結果が成功」のときだけ動く（乾いた走りと失敗した走りでは動かない）。リリース workflow は後段を呼ばず、合図も送らない。両 brief にある「`release: published` で動く」は、`GITHUB_TOKEN` で公開した Release では後段が起きないため、この `workflow_run` の形に置き換わる（両 brief の書き換えは本仕様の外）。

## Requirements

### Requirement 1: タグが押されたときだけ動く

**Objective:** As a 開発者, I want リリースのきっかけを自分のタグだけにする, so that 普通の PR のマージで版や Release が勝手に動かない

#### Acceptance Criteria

1. When 開発者が `v` で始まるタグをリポジトリへ押した, the リリース workflow shall そのタグの指すコミットを取り出し、そのコミットの中身だけでリリースの手順を始める。
2. If PR の作成・更新・マージ、またはタグを伴わない枝への push（main への push を含む）が起きた, the リリース workflow shall main へ入った定義のもとでは走りを始めない（マージ前の作業の枝に一時的に置くきっかけは要件 9 の扱い）。
3. The リリース workflow shall タグを打たず、消さず、版（`Cargo.toml`）を書き換えず、リポジトリへコミットを押さず、版を選ぶ・上げるための入力を持たない（版を決めるのは `release-cycle`）。
4. The リリース workflow shall main へ入った定義で、走りを始めるきっかけをタグ `v*` の push と手で始める乾いた走り（要件 5）の 2 つだけにする。

### Requirement 2: 版とタグの一致の検査

**Objective:** As a 開発者, I want タグと `Cargo.toml` の版が食い違ったリリースを止める, so that zip の名前・Release の名前・crates.io の版がずれない

#### Acceptance Criteria

1. When タグで走りを始めた, the リリース workflow shall タグの名前から先頭の `v` を除いた文字列と、取り出したコミットの `Cargo.toml` の `[workspace.package] version` を、文字どおりに比べる。
2. If 両者が一致しない（例: タグ `v0.0.2` に対し版 `0.0.1`・タグ `v0.0.2-x`・タグ `v`）, the リリース workflow shall zip を作る前に止まり、タグの版と `Cargo.toml` の版の両方を走りの記録に印字し、Release を作らずに失敗で終える。
3. If そのタグの Release（下書きを含む）が既に在る, the リリース workflow shall zip を作る前に止まり、既存の Release に一切触らず、既に在ることを印字して失敗で終える（同じ版で出し直さない＝`release-cycle` の決まり）。

### Requirement 3: x64 と arm64 の zip と SHA256

**Objective:** As a 開発者, I want 手元と同じ配布スクリプトで両方の CPU 種別の zip とハッシュを作る, so that 手元で確かめた形と同じ物が配られる

#### Acceptance Criteria

1. When 版とタグの一致を確かめた, the リリース workflow shall 配布スクリプトを起動確認なしで x64 と arm64 の両方について呼び、`areka-{版}-x64.zip`・`areka-{版}-x64.zip.sha256`・`areka-{版}-arm64.zip`・`areka-{版}-arm64.zip.sha256` の 4 つのファイルを得る。
2. If 配布スクリプトが 0 以外の終了コードで終わった、または 4 つのファイルのうち 1 つでも欠けた, the リリース workflow shall Release を作らずに失敗で終え、配布スクリプトが印字した失敗の段の名前を走りの記録に残す。
3. The リリース workflow shall 配布スクリプトの起動確認（窓を出す自己検査 `-Check`）を呼ばない（起動確認は `release-cycle` の手順で開発者が手元で通す）。
4. The リリース workflow shall 配布スクリプトが前提とする道具（ライセンス検査の道具・謝辞の生成の道具を、配布スクリプトが求める版で）と arm64 のリンクの道具を、配布スクリプトを呼ぶ前に揃え、2 回目以降の走りでは前の走りで用意した物を再利用して毎回ソースから入れ直さない。
5. If 実行環境に arm64 のリンクの道具が無く、揃えることもできなかった, the リリース workflow shall x64 だけで Release を作らず、失敗で終える（x64 と arm64 は必ず両方を揃えて出す）。

### Requirement 4: Release の公開

**Objective:** As a 開発者, I want 揃った 4 つのファイルを添えた Release が公開の状態で現れる, so that 利用者と後段がそのまま使える

#### Acceptance Criteria

1. When 4 つのファイルが揃った, the リリース workflow shall タグ `v{版}` の Release を、そのタグの間の変更から自動で生成したリリースノートつきで、下書きでなく公開の状態で作り、4 つのファイルを添える。
2. The リリース workflow shall Release に添える zip と `.sha256` を、同じ走りで配布スクリプトが作った物そのものとする（作り直さない）。添えた `.sha256` の値は、添えた zip から計算した SHA256 と一致する。
3. If Release の作成またはファイルの添付が途中で失敗した, the リリース workflow shall そのタグの Release を下書きとしても公開としても残さず、失敗で終える。
4. The リリース workflow shall 4 つのファイルの一部だけを添えた Release を公開の状態にしない（公開された Release は常に 4 つを揃えて持つ）。

### Requirement 5: 乾いた走り

**Objective:** As a 開発者, I want Release を作らずに同じ手順を試せる, so that タグを打つ前に実行環境で通ることを確かめられる

#### Acceptance Criteria

1. When 開発者が手で乾いた走りを始めた, the リリース workflow shall 選ばれた枝またはタグのコミットで、要件 2（版とタグの検査）と要件 3（zip と SHA256）と同じ手順を行い、Release を作らず、タグも打たない。
2. When 乾いた走りをタグでなく枝で始めた, the リリース workflow shall 版とタグの比べを「タグが無い」と印字して飛ばし、`Cargo.toml` の版で 4 つのファイルを作る。
3. When 乾いた走りをタグで始めた and そのタグの Release が既に在る, the リリース workflow shall 既に在ることを印字し、止まらずに残りの手順を続ける（何も作らないので既存の Release に触れない）。
4. When 乾いた走りが緑で終わった, the リリース workflow shall 作れた 4 つのファイルの名前と、各 zip の SHA256 を走りの記録に印字する。

### Requirement 6: 秘密・権限・費用

**Objective:** As a 開発者, I want 長生きする秘密を置かず、無料枠の中で動かす, so that 漏れる物が無く、費用もかからない

#### Acceptance Criteria

1. The リリース workflow shall リポジトリにも workflow の設定にも長生きするトークンを置かず、その走りのために GitHub が発行する一時の権限だけを使う。
2. The リリース workflow shall トークンの値・リモートの URL を走りの記録に出さない（リモートの一覧を印字する手順を持たない）。
3. The リリース workflow shall 一時の権限を、リポジトリの中身への書き込み（Release を作り、ファイルを添えるのに要る分）だけに絞り、それ以外の権限を求めない。
4. The リリース workflow shall 公開リポジトリの無料枠の中で、1 回のリリースを Windows の実行環境 1 本で回す。
5. If 走りが設計で定める上限の時間を超えた, the リリース workflow shall その走りを止め、Release を作らずに失敗で終える。

### Requirement 7: 文書の追随

**Objective:** As a 開発者とエージェント, I want steering が「テストは手元・配布は Actions」を正しく書く, so that 「外部 CI は持たない」を読んでテストを CI に移したり、配布を手で続けたりしない

#### Acceptance Criteria

1. The 本仕様 shall `tech.md` の「外部 CI は持たない」の段落を、「テストの門は手元のフルテストのまま・ビルドと配布だけを GitHub Actions に乗せる（タグ `v*` のときだけ）」の趣旨に改め、zip の起動確認を CI で回さない理由（窓を出す）を書く。見送った経緯（2026-09-24）はテストの門の話として残す。
2. The 本仕様 shall `structure.md` の最上位の説明に `.github/`（`workflows/release.yml`＝タグで動くリリース）を足す。

### Requirement 8: 後段が見分けられる終わり方

**Objective:** As a 後段の workflow（crates.io への公開・winget への提出）を持つ開発者, I want リリース workflow の走りの結果だけで「Release が揃って公開されたか」を見分けられる, so that 後段は乾いた走りや失敗した走りで動かず、公開された Release だけを相手にできる

#### Acceptance Criteria

1. When タグの push で始まった走りが成功の結果で終わった, the リリース workflow shall その時点で、そのタグの Release が 4 つのファイルを揃えて公開（下書きでない）の状態で既に在るようにする。
2. If タグの push で始まった走りで、4 つのファイルを揃えた Release を公開できなかった（版の不一致・既存の Release・zip の失敗・公開の失敗・上限の時間の超過のどれでも）, the リリース workflow shall その走りを成功以外の結果で終える。
3. The リリース workflow shall Release の公開に成功した走りを成功で終える（公開の後に、走りを失敗にしうる手順を持たない＝Release が在るのに結果が失敗、の食い違いを作らない）。
4. When 乾いた走りが終わった, the リリース workflow shall 走りの始まりの種別を「手で始めた」のままにして、タグの push で始まった走りと見分けられるようにする（結果が成功でも Release は無い）。
5. The リリース workflow shall 後段の workflow を呼ばず、後段への合図も送らない（後段は自分で本 workflow の走りの終わりを受け、「タグの push で始まった」かつ「成功」で絞る）。

### Requirement 9: 完了の証跡

**Objective:** As a 開発者, I want マージの前に本番と同じ実行環境で 1 回通したことを確かめる, so that 初回のリリースが道具の欠け（arm64 のリンクの道具など）で赤になるのを避けられる

#### Acceptance Criteria

1. The 本仕様 shall main へマージする前に、作業の枝に一時的に置いたきっかけで、本番と同じ Windows の実行環境の上で要件 2〜3 の手順（版の検査・4 つのファイルの作成）を Release を作らずに 1 回緑で通し、その走りの記録（arm64 のリンクの道具が在ったか・揃えたか、各 zip の SHA256、所要時間）を証跡に残す。
2. The 本仕様 shall main へマージする workflow の定義に、その一時的なきっかけを残さない（要件 1.4 のとおり、きっかけはタグの push と手で始める乾いた走りだけ）。
3. The 本仕様 shall 完了の判定を要件 9.1 の 1 回の緑と静的な確かめで行い、初回の実走（`v0.0.2`）の成否を完了の条件にしない（`v0.0.2` は本仕様のマージの後に `release-cycle` の初回が起こして見守り、赤なら直す spec を起票して同じ版では出し直さない）。

## 要件生成で決めた点（2026-10-03・開発者の答え）

要件生成中に上げた問いへの答え。いずれも上の条項に反映済み。

- **後段の起こし方＝案 A（`workflow_run`）**: `GITHUB_TOKEN` で公開した Release は後段の workflow を起こさない（GitHub Actions の決まり）ため、後段（`crates-io.yml`・`winget.yml`）はリリース workflow の走りの終わり（`workflow_run`）を受け、「タグの push で始まった」かつ「成功」で絞る。本仕様は権限 `contents: write` だけのまま、後段を呼ばず合図も送らない。本仕様が後段へ約束するのは要件 8 の「成功で終わったら Release は揃って公開済み・乾いた走りと失敗は見分けられる」だけ。両 brief の「`release: published` で動く」の書き換えは本仕様の外（各 spec の持ち物）。退けた案: リリース workflow から後段へ合図を送る（`repository_dispatch`）／長生きするトークンで Release を作る（brief の決めごとに反する）。
- **初回の実走（`v0.0.2`）**: 初めての本当のリリースは `v0.0.2` でよいが、本仕様のマージの後に `release-cycle` の初回で起こる。本仕様の完了は要件 9.1 の 1 回の緑（と乾いた走りの形）で判定する（要件 9.3）。
- **マージ前の実行環境での走り**: 実装中に作業の枝を GitHub へ押し、一時的なきっかけ（その枝への push）で 1 回通す。arm64 のリンクの道具の有無と、通しのビルドを測る。一時的なきっかけはマージ前に外す（要件 9.1・9.2）。**作業の枝を押すたびに、その時点で開発者の了承を得る**（手順の決まり・条項にはしない）。
- **版の決め方**: リリース workflow は版を上げず、タグも打たない。本当のきっかけはタグの push だけで、版とタグの一致の検査は残す（要件 1.3・2）。「指定が無ければ +0.0.1」の決まりはリリースの手順（`release-cycle`）の持ち物で、リリース workflow に版を上げる入力は足さない。
- **乾いた走りの版の検査**: 枝で始めたときはタグが無いので比べを飛ばして印字し、タグで始めたときは本番と同じに比べる（要件 5.2）。既存の Release は乾いた走りでは止める理由にしない（何も作らないため・要件 5.3）。