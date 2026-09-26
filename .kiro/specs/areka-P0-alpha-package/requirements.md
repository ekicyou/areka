# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: α を受け取る第三者と、α の完成宣言を出す開発者。今日は配布物を作る手段が無い——zip を組むスクリプトも、第三者が最初に読む説明書も無い。さらに根の `README.md` のライセンス表記が実物と食い違っている（バッジと本文が「MIT OR Apache-2.0」・バッジのリンク先 `LICENSE` は実在しない／`Cargo.toml` は `license = "MIT"`・実物は `LICENSE-MIT` だけ）。これを α の最後の段（`alpha-release-signoff`）でまとめてやると、再配布の条件の確認の答え次第で既定ゴーストの差し替えまで最後に押し寄せる。

**今の状態**: `tools/` の直下は `tools/test-all.ps1` と `tools/perf/` だけで、配布物を組むものは無い。根の形（`<根>/ghost/<名>/`・`<根>/balloon/<名>/`・根は exe の隣）と記憶の置き場（`<exe>/profile/areka`）は完了 `areka-P0-baseware-root-layout` で確定している。既定ゴーストは `emo2`（SHIORI は 32 ビットの pasta.dll＝helper が要る）、既定バルーンは CC0 の `StayseeBalloon`。検体の展開した木は `nar-sample-path` が作る。

**何を変えるか**: 配布スクリプト 1 本で、release ビルドの本体・32 ビットの helper・既定ゴーストと既定バルーン・第三者向け README・ライセンスと謝辞を 1 つの zip に組み、別の場所へ展開した zip が有界の自動終了で起動確認を通ることを確かめられるようにする。第三者向け README の骨子を置き、根の `README.md` のライセンス表記と古い数を実物に揃える。`emo2` の同梱は 2026-09-26 の開発者裁定どおり（シェルは MIT でない・ファーストゴーストとしては使えるがシェルの抜き出し利用は不可・シェルの説明書を zip に残す）。

> 起票: 2026-09-26 `/kiro-discovery` 再入（棚卸⑰）で `alpha-release-signoff` から切り出し。brief の事実は main `13b72893` の起票時実測で、要件生成時（2026-09-26・main `2f5bd24a`）に引き直した。

## Introduction

本仕様は、α の配布物（zip）を**組む・確かめる・説明する**ための道具と文書を用意する。areka の本体（`crates/` のソース）は 1 行も変えない。変えるのは配布スクリプト（新規）・第三者向け README（新規）・根の `README.md` の表記だけで、議題 ⑷ の答えしだいで `.gitignore` の 1 行が加わる（下の「要件討議で決める 3 点」。議題 ⑵ は MIT 単独に決まり、`LICENSE-APACHE`・根の `Cargo.toml`・謝辞の雛形の変更は 0）。

要件生成時に引き直した事実（設計はこれを再検証する）:

- `tools/` の直下は `test-all.ps1`（i686 ターゲットの導入・i686 の helper のビルド・段ごとの合否の一覧・検査したコミットと未コミットの変更の件数の印字を持つ前例）と `perf/` だけ。`scripts/` は無い。
- 本体は release ビルドでだけ窓のアプリ（コンソールを持たない）になる（`main.rs` 先頭の `windows_subsystem` の属性）。
- helper の置き場は exe の隣の `shiori-host32-helper.exe`（`boot_config.rs` の `default_helper_exe_path` が綴る）。記憶の置き場は `default_app_profile_dir` が決める `<exe>/profile/areka`（環境変数 `AREKA_PROFILE_DIR` で上書きされる）。根は exe の隣（環境変数 `AREKA_ROOT` で上書きされる）。
- 既定の名前は `boot_resolve.rs` の `DEFAULT_GHOST_FOLDER`＝`emo2`・`DEFAULT_BALLOON_FOLDER`＝`StayseeBalloon`。バルーンの決まる順は「記憶 → ゴーストの同梱 → 唯一 → 既定 → 無作為」で、同梱の名が根に無ければ警告（`companion_balloon_not_found`）を 1 行残して次の段へ進む。
- `emo2.nar` の中身: `install.txt` が `balloon.directory,emo2-kakukaku` を宣言し、同梱バルーン `emo2-kakukaku` を書庫の中に持つ。`ghost/master/pasta.dll` は 32 ビット（PE の機種 0x14c）。シェルの説明書 `shell/master/readme.txt` と、ゴースト全体の `readme.txt`（シェル・バルーン画像素材の作者の記載を含む）が在る。`install.txt` の読み手は `crates/areka-ghost/src/catalog.rs`。
- 検体の展開は `cargo run -p sample-ghost-kit --bin nar-sample-path -- <検体>`（出力の形は `crates/sample-ghost-kit/tests/nar_sample_path_test.rs` が固定）。呼ぶたびに `target/nar-samples/manual/<検体>/` を消して作り直す＝同じ検体で実機を回している最中に呼ぶと、その木を消す。
- smoke の自動終了は `main.rs` の `SMOKE_EXIT_ENV`＝`AREKA_APP_SMOKE_EXIT_MS`。告知のモーダルは `alert.rs` の `NO_ALERT_ENV`＝`AREKA_NO_ALERT` で抑えられる。前例 `crates/areka/tests/emo2_real_run.rs` は子プロセスの出力を grep し、終了コード 0 と「ゴーストが立った」記録を番犬付きで判定している。
- ライセンス: 根の `Cargo.toml` の `[workspace.package]` は `license = "MIT"`、全 crate が `license.workspace = true`、実物のファイルは `LICENSE-MIT` だけ。根の `README.md` はバッジ（リンク先 `LICENSE` は不在）とライセンスの節の 2 か所が「MIT OR Apache-2.0」。謝辞の雛形 `about.hbs` は「areka 自身のライセンスは MIT です（ルートの `LICENSE-MIT` を参照）」と書き、生成物 `THIRD-PARTY-NOTICES.md` にもその文が入る。`deny.toml` の冒頭の注記も「areka の MIT 化を守る」。
- 根の `README.md` の「現在の到達点」は「57件の仕様を完了し、基盤レイヤーの約70%を構築済み」（`.kiro/specs/completed/` の直下は仕様のフォルダ 200 と `.md` 1 本の計 201 項目＝仕様の数は 200）。
- `Cargo.lock` は追跡外（`.gitignore` の 2 行目）。`THIRD-PARTY-NOTICES.md` は cargo の依存だけを載せ、cargo 依存でない資産（ゴースト・シェル・バルーン）は載らない（steering `tech.md`）。

## Boundary Context

- **In scope**: ① 配布スクリプト（`tools/package-alpha.ps1`）で zip を組む／② 組んだ zip を別の場所へ展開して起動確認する（有界の自動終了）／③ 第三者向け README の骨子（新規ファイル）と同梱物の条件の明記／④ 根の `README.md` のライセンス表記と古い数の是正／⑤ 議題 ⑷ の答えしだいの付随（`.gitignore` の `Cargo.lock` の行）。
- **Out of scope**: 検証項目表と第三者の手順の実機一周・第三者向け README の仕上げ（`.nar` の入れ方の手順を含む）・署名と宣言（`alpha-release-signoff`）／既定ゴーストの差し替え（議題「emo2 を入れてよいか」は同梱可で決着。万一将来要るときも `boot_resolve.rs` は `ghost-shell-balloon-switch`・`shell-balloon-switch` と共有するので本仕様では行わない）／インストーラ（msi 等）・署名付きの exe・自動更新／ARM64 版の zip（α の zip は x64 版 1 種）／常時のテストへの組み込み。
- **変更 0 と明記するもの**: `crates/` の変更は 0（触る必要が出たら `alpha-release-signoff` へ送る）。`tools/test-all.ps1` の変更は 0。`vendors/sample_ghost/` の変更は 0。`THIRD-PARTY-NOTICES.md` を手で直す箇所は 0（直すなら生成器で作り直す）。ワークスペースの常時のテストに足すテストは 0。
- **Adjacent expectations**: 完了 `areka-P0-baseware-root-layout`（根の形・記憶の置き場・起動の解決）・`areka-P0-default-balloon-nar-fold`（既定バルーンの `.nar`）・`areka-P0-nar-install`（`nar-sample-path`）の上に建つ。下流の `areka-P0-alpha-release-signoff` は本仕様の zip で実機一周をし、第三者向け README の空欄を仕上げる。同じウェーブで並走する `ghost-shell-balloon-switch`・`shell-balloon-switch`・`ghost-install`・`network-update` は `tools/` と根の `README.md` に触らない（共有 0）。ただし `ghost-shell-balloon-switch` が着地すると右クリックメニューに項目が増えるので、第三者向け README のメニューの欄は着地した main に合わせて `alpha-release-signoff` が仕上げる。

## 要件討議で決める 3 点（開発者の決めごと）

議題 ⑶「`emo2` を zip に入れてよいか」は 2026-09-26 に開発者が裁定済み（同梱可・要件 2.2 と 5.2 に反映）で、残りは次の 3 点。各議題は、答えによって変わる条項を「Where 〔議題 ⑷＝…〕」の形で分けて書いた（裁定済みの議題は条項を 1 つに畳んだ）。どちらの答えでも他の条項は変わらない。

- ~~**⑴ zip に `emo2-kakukaku` を入れるか**~~ → **2026-09-26 要件討議で開発者裁定＝入れる**（要件 2.5・3.5・5.5 に反映）。根拠: 開発者が配布サイトで公開している `emo2.nar` にすでに `emo2-kakukaku` が同梱されており、zip はその中身をそのまま写すのと同じ形。初回に立つバルーンは `emo2` の同梱の `emo2-kakukaku`（決まる順の「同梱」の段）で、`emo2` の説明書（「利用バルーン: kakukaku for emo-gs」）と一致する。画像素材はフキダシデザインのもの（規約は「アプリ・ゲームへの組み込みは 20 点まで無料・表記不要・データの再配布は禁止」・画像は 15 本）なので、第三者向け README には `emo2` のシェルと同じ書き方で「画像の抜き出し利用は不可」を明記する。
- ~~**⑵ areka 自身のライセンスは MIT 単独か MIT OR Apache-2.0 か**~~ → **2026-09-26 要件討議で開発者裁定＝MIT 単独**（要件 6.2・6.4 に反映・旧 6.3 は削除）。開発者の整理: 実装のコードは MIT。「MIT OR Apache-2.0」は Rust の crate の慣習（依存の crate や pasta の `Cargo.toml`）から来た書きぶりで、areka の実物（`LICENSE-MIT`・全 crate の `license.workspace`＝`"MIT"`）とは別。画像（シェル・バルーン）は各作者の条件で別に管理する。`emo2` の `pasta.dll` は MIT、ゴーストの辞書には利用条件の主張が無い（要件 5.8 に反映）。
- **⑷ `Cargo.lock` を追跡するか**（要件 7.3）。
  - 追跡する: 別の機械で組み直しても同じ依存の版になり、zip と謝辞が再現できる。`.gitignore` の 1 行を外し `Cargo.lock` を加える。並走する枝が依存を変えるたびに `Cargo.lock` が衝突する。
  - 追跡しない: 今日のまま。zip の謝辞は組んだ機械の依存の版で作る（要件 7.1・7.2 はどちらの答えでも満たす）。

## Requirements

### Requirement 1: 配布スクリプト 1 本で zip を組める

**Objective:** As a α の完成宣言を出す開発者, I want 1 つのコマンドで、今のソースから配布物の zip を組めること, so that 手作業の取り違えなく、毎回同じ中身の配布物を作れる

#### Acceptance Criteria

1. When 開発者が配布スクリプトを実行する, the 配布スクリプト shall 本体（`areka.exe`・x64）を release ビルドで、helper（`shiori-host32-helper.exe`）を 32 ビット（i686）の release ビルドで、その場のソースから作り直してから zip に入れる（以前の成果物を黙って拾わない）。
2. When 32 ビットのビルドに要るターゲットが開発機に無い, the 配布スクリプト shall それを導入してから続ける（前例 `tools/test-all.ps1` と同じ扱い）。
3. When 配布スクリプトが既定ゴーストと既定バルーンを zip に入れる, the 配布スクリプト shall 検体の `.nar` を展開した木を既存の展開の窓口（`nar-sample-path`）から得て写す（展開の仕組みを 2 つ持たない）。
4. When zip を組み終える, the 配布スクリプト shall zip の置き場所の絶対パスと、組んだコミットと、組み始めた時点の未コミットの変更の件数を印字し、同じコミットと件数を zip の中にも記録として残す。
5. If どれかの段（ビルド・展開・写し・圧縮・中身の検査）が失敗する, then the 配布スクリプト shall 失敗した段の名前を印字して終了コード非 0 で終わり、完成品に見える zip を残さない。
6. The 配布スクリプト shall 追跡しているファイルを 1 つも書き換えず、zip と作業の途中物は追跡外の場所に置く（実行の前後で `git status` が変わらない）。
7. The 配布スクリプト shall 使い方の説明に「同じ検体で実機を回している最中に実行すると、その走行の展開した木を消す」ことを書く。

### Requirement 2: zip の中身は決めたものだけ

**Objective:** As a α を受け取る第三者, I want zip を展開しただけで areka が動き、入っていてはいけないものが入っていないこと, so that 追加の手順なしで起動でき、再配布の条件に反するものを受け取らない

#### Acceptance Criteria

1. The 配布スクリプト shall zip を展開した最上位に、`areka.exe`・`shiori-host32-helper.exe`・`ghost/emo2/`・`balloon/StayseeBalloon/`・第三者向け README・areka 自身のライセンス文書・謝辞（`THIRD-PARTY-NOTICES.md` に相当するもの）を、完了 `baseware-root-layout` の根の形（根は exe の隣）どおりに置く。
2. The 配布スクリプト shall `emo2` のシェルの説明書（`shell/master/readme.txt`）と `emo2` の `readme.txt` を、`emo2.nar` の中のものと 1 バイトも違わない形で zip に残す。
3. The 配布スクリプト shall zip に起動記録（areka の記憶の置き場 `profile/` と、ゴーストの中の `profile/`）を 1 つも入れない（展開した直後の起動が初回の起動になる）。
4. The 配布スクリプト shall zip に既定ゴースト `emo2` 以外のゴーストを入れず、`StayseeBalloon` と要件 2.5 のもの以外のバルーンを入れない（`konnoyayame`・`claudia`・`R_POST_and_KOMAINU`・`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit` は入れない）。テスト用の DLL と実行ファイル（`shiori-host32-testdll` 等）も入れない。
5. The 配布スクリプト shall `emo2` の同梱バルーン `balloon/emo2-kakukaku/` を zip に入れる（議題 ⑴ の裁定）。
6. When zip を組み終える, the 配布スクリプト shall 要件 2.1〜2.5 と 2.7 を zip の実物から判定し（印字するだけでなく合否を決め）、1 つでも外れれば要件 1.5 の失敗として扱う。判定には「zip の `shiori-host32-helper.exe` が 32 ビット（PE の機種 0x14c）であること」を含める（`cargo build --workspace` が x64 の helper を `target/release/` に置くことがあり、取り違えると 32 ビットの SHIORI が読めない）。
7. The zip の実行ファイル（`areka.exe`・`shiori-host32-helper.exe`）shall Windows 10 以降に標準で在る DLL だけに頼り、Visual C++ 再頒布可能パッケージ（`VCRUNTIME140.dll` 等）を入れていない機械でも起動できる（開発機には必ず在るので、要件 3 の起動確認だけではこの欠けを見つけられない。どう満たすか＝静的に結ぶか等は設計で決める）。

### Requirement 3: 展開した zip で起動確認が通る

**Objective:** As a α の完成宣言を出す開発者, I want 組んだ zip を別の場所へ展開して、有界の自動終了で起動確認できること, so that 開発機の環境に助けられて動いているだけの zip を配らずに済む

#### Acceptance Criteria

1. When 開発者が起動確認を求める, the 配布スクリプト shall 組んだ zip をリポジトリの外の新しい空の、パスの短い場所へ展開し、展開した `areka.exe` を引数なしで起動する（根も記憶の置き場も展開した場所から決まる形で）。パスを短くするのは、`emo2` の SHIORI（pasta）が初回に `ghost/master/profile/` の奥へ書き出すファイルのパスが長すぎると、接続の失敗を出さずに黙るため（`research.md` §4.2）。
2. While 起動確認の走行中, the 配布スクリプト shall 開発機の環境変数のうち areka の振る舞いを変えるもの（`AREKA_` で始まるもの。根と記憶の置き場を上書きする `AREKA_ROOT`・`AREKA_PROFILE_DIR` を含む）を子プロセスへ持ち越さず、告知のモーダルを抑える指定（`AREKA_NO_ALERT`）と有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）だけを渡す（判定に要る記録の出し方の指定は設計で決める）。
3. When 子プロセスが終わる, the 配布スクリプト shall 終了コードが 0 であること・ゴーストの窓が立った記録があること・SHIORI の接続の失敗の記録が無いこと・ゴーストの会話が始まった記録があることを判定し、合否を印字して、否なら終了コード非 0 で終わる。「会話が始まった」を含めるのは、SHIORI がすべてのイベントに空の応答を返して黙る失敗と、接続の結果より先に自動終了が来る空振りを、ほかの 3 条件だけでは通してしまうため（どの記録を目印にするか・自動終了を何ミリ秒にするかは設計で決める）。
4. If 子プロセスが決めた上限の時間までに終わらない, then the 配布スクリプト shall 自分が起こしたその子プロセスだけを止め、起動確認を否として終了コード非 0 で終わる（他のプロセスは止めない）。
5. When 子プロセスが終わる, the 配布スクリプト shall 初回に立ったバルーンが `emo2` の同梱の `emo2-kakukaku`（決まる順の「同梱」の段）であることも判定する。
6. If 起動確認が否になる, then the 配布スクリプト shall 子プロセスの記録の置き場所を印字して残す（開発者があとで読める）。
7. The 起動確認 shall 実表示のある開発機で開発者が手で回すものとし、ワークスペースの常時のテストと `tools/test-all.ps1` には入れない（release ビルドを要するため）。

### Requirement 4: 第三者向け README の骨子がある

**Objective:** As a α を受け取る第三者, I want zip を開いて最初に読む説明書があること, so that 起動・終了・メニュー・記憶の置き場・既知の制限を、開発者に聞かずに知れる

#### Acceptance Criteria

1. The 第三者向け README shall リポジトリで追跡する新しいファイルとして置き、根の `README.md`（開発者向け）とは別にする。言語は日本語とする。
2. The 第三者向け README shall 少なくとも「起動」「終了」「右クリックメニュー」「記憶の置き場」「既知の制限」「`.nar` の入れ方」「同梱物とライセンス」の欄を持つ。
3. The 第三者向け README shall 「`.nar` の入れ方」の欄は見出しと「未記入（`alpha-release-signoff` が仕上げる）」の目印だけを置き、手順を書かない（入れ方は `ghost-install` の入口で決まるため）。ほかの欄でも、本仕様の時点で書けない中身は同じ目印で空けておく。
4. The 第三者向け README shall 欄に書く事実（起動のしかた・終了のしかた・メニューの項目・記憶の置き場所・根の形）を、実装の着手時の main のソースで確かめてから書き、確かめられないことは書かない。
5. The 第三者向け README shall 「既知の制限」の欄に、少なくとも「exe に署名が無い（Windows が警告を出すことがある）」「Windows 専用」「深いフォルダに展開しない（パスが長いと既定ゴーストが黙ることがある）」「α の時点でできないこと（ゴーストの切替など、着手時の main で未着地のもの）」を書く。
6. When 配布スクリプトが zip を組む, the 配布スクリプト shall リポジトリの第三者向け README をそのまま zip の最上位へ入れる（zip のために別の文面を作らない）。

### Requirement 5: 同梱物の条件を正しく載せる

**Objective:** As a α を受け取る第三者, I want zip に入っているゴースト・シェル・バルーンのそれぞれについて、誰の作品でどんな条件で使えるかが書いてあること, so that areka の MIT がすべてに及ぶと誤解して、シェルを抜き出して使うようなことをしない

#### Acceptance Criteria

1. The 第三者向け README shall 「同梱物とライセンス」の欄に、zip に入れた第三者の資産（ゴースト・SHIORI の DLL・シェル・バルーン）を 1 つずつ挙げ、作者と条件とその出どころ（資産に同梱された文書の名前、または配布元）を書く。
2. The 第三者向け README shall `emo2` について「シェルは MIT ではなく、シェル作者の条件に従う」「areka のファーストゴースト（既定ゴースト）として使うことはできるが、シェルを抜き出して利用することはできない」と明記し、シェルの説明書が zip の中のどこに在るか（`ghost/emo2/shell/master/readme.txt`）を示す。シェルは作者が 2 人（\0「コンフィズリー」＝ゆゆぴか・\1「City-Pop'n」＝大槻）で、説明書に条件の本文があるのは \0 だけなので、\1 の条件は要件 5.6 に従い「未確認」と書き作者のサイトを示す（同梱すること自体は議題 ⑶ の裁定＝`emo2` 同梱可に含まれる）。
3. The 第三者向け README shall `StayseeBalloon` の条件が CC0 であることを書く。
4. The 第三者向け README shall areka 自身のライセンスは areka の本体（`areka.exe`・`shiori-host32-helper.exe`）に及び、同梱の第三者の資産には及ばないことを書き、cargo の依存の謝辞は同梱の謝辞の文書にあると示す。
5. The 第三者向け README shall `emo2-kakukaku` の作者（ekicyou）と、バルーン画像素材の出どころ（フキダシデザイン）とその条件を載せ、`emo2` のシェルと同じ書き方で「areka と `emo2` のバルーンとして使うことはできるが、画像を抜き出して利用することはできない」と明記する。
6. If 資産の条件をその資産の文書や配布元から確かめられない, then the 第三者向け README shall 推測で条件を書かず「未確認」と書き、その資産を未確認のまま zip に入れるかどうかを開発者の判断に回す（要件討議か `alpha-release-signoff` で決める）。
7. The 配布スクリプト shall `konnoyayame`（シェルが CC BY-NC-ND）を zip に入れない（要件 2.4 の再掲・再配布の条件による理由をここに記す）。
8. The 第三者向け README shall `emo2` を部分ごとに分けて書く: SHIORI の `pasta.dll` は MIT（pasta の `LICENSE` の実物どおり。pasta の `Cargo.toml` の「MIT OR Apache-2.0」の書きぶりの是正は本仕様の外）／ゴーストの辞書には利用条件の主張が無いことをそのまま書き、推測で条件を足さない／画像（シェル・バルーン）は各作者の条件に従い、要件 5.2・5.5 のとおり抜き出し利用はできない。

### Requirement 6: 根の README の表記が実物と一致する

**Objective:** As a リポジトリを読む人, I want 根の `README.md` のライセンス表記と到達点の数が実物と一致していること, so that 存在しないファイルへのリンクや古い数に惑わされない

#### Acceptance Criteria

1. The 根の `README.md` shall ライセンスのバッジのリンク先を実在するファイルにする（今日のリンク先 `LICENSE` は実在しない）。
2. The 根の `README.md` shall バッジとライセンスの節の 2 か所を「MIT」とし、`LICENSE-MIT` を指す（議題 ⑵ の裁定＝MIT 単独）。根の `Cargo.toml`・`about.hbs`・`deny.toml`・`crates/` の README の変更は 0（すでに MIT）。`LICENSE-APACHE` の追加は 0。
3. （削除: 議題 ⑵ が MIT 単独に決まり、MIT OR Apache-2.0 の条項は不要になった）
4. The リポジトリ shall areka 自身のライセンスを述べる箇所（根の `README.md` の 2 か所・根の `Cargo.toml` の `license`・`about.hbs`・第三者向け README・zip に入れるライセンス文書）を MIT に揃え、食い違いを 0 にする。
5. The 根の `README.md` shall 「現在の到達点」の節の数（「57件の仕様を完了」「約70%」）を、着手時の実物から数え直した数と数えた日付に直すか、実物から導けない数を消す（古い数を残さない）。仕様の数は `.kiro/specs/completed/` の直下の**フォルダ**の数で数える（直下の `.md` は仕様ではない）。「約70%」は実物から導けないので消す。
6. The 根の `README.md` shall 第三者向け README の置き場所を 1 行で案内する。
7. The 根の `README.md` shall 本仕様で直す範囲を、ライセンスの表記・到達点の数・要件 6.6 の 1 行の案内に限り、それ以外の古い記述（クレート構成の説明など）は本仕様で直さない。

### Requirement 7: zip の謝辞が zip の中の本体と食い違わない

**Objective:** As a α を受け取る第三者, I want zip に入っている謝辞が、zip の中の実行ファイルに実際に組み込まれた依存と一致していること, so that 依存のライセンス表示が正しい

#### Acceptance Criteria

1. When 配布スクリプトが zip を組む, the 配布スクリプト shall zip に入れる謝辞を、zip に入れる実行ファイルを組んだのと同じ依存の版から生成器で作る（リポジトリの `THIRD-PARTY-NOTICES.md` が古くても zip の謝辞は古くならない）。生成の出力先は追跡外の場所とし、リポジトリの `THIRD-PARTY-NOTICES.md` は書き換えない（要件 1.6）。
2. If 謝辞の生成（またはその前提のライセンスの検査）が失敗する, then the 配布スクリプト shall 要件 1.5 の失敗として扱い、謝辞の無い zip を完成品として残さない。
3. Where 〔議題 ⑷＝追跡する〕, the リポジトリ shall `Cargo.lock` を追跡し（`.gitignore` の該当の 1 行を外す）、同じコミットから組んだ zip の依存の版と謝辞が機械によらず同じになる。Where 〔議題 ⑷＝追跡しない〕, the リポジトリ shall `.gitignore` と `Cargo.lock` の扱いを変えず（変更 0）、zip の謝辞は組んだ機械の依存の版で作られることを配布スクリプトの使い方の説明に書く。
