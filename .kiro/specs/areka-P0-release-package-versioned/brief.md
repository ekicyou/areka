# Brief: areka-P0-release-package-versioned

> 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）で起票。**同日、`package-check-temp-cleanup`（道具のバグ）を合流し、ウェーブ C1 へ前倒しした**（開発者「インストーラー関係は優先リリースしたい」・末尾の節）。テーマの決めごとは `.kiro/steering/roadmap.md`「配布と公開」節。file:line は起票時値（main `76e17654`）。

## Problem

配布物を winget と GitHub Releases に乗せるには、zip の名前に版が入り、中身のハッシュ（SHA256）が出て、x64 と arm64 の 2 つが同じ手順で作れなければならない。今の `tools/package-alpha.ps1` は x64 だけを `areka-alpha-x64-{日付}-{コミット}.zip` の名前で作り、ハッシュを出さない。同じ URL のまま中身を替えると winget の検査（`Validation-Hash-Verification-Failed`）が落ち、既に出ている版のインストールまで全部失敗するので、**版ごとに固定の名前**が要る。

また winget の portable は、PATH から呼べるようにリンク（`%LOCALAPPDATA%\Microsoft\WinGet\Links\areka.exe`）を作る。リンク経由で起動すると、隣の DLL や補助 exe を見失う既知の問題がある（winget-cli #2889・#2711）。areka は「根は exe の隣」「補助 exe `shiori-host32-helper.exe` は exe の隣」「プロファイルは exe の隣の `profile\areka`」と、すべて `current_exe()` から引く（`crates/areka/src/boot_config.rs` の `resolve_root_from`・`default_helper_exe_path`・`default_app_profile_dir`）。リンクの場所を exe の場所と取り違えると、winget から入れた areka は起動しない。

## Current State

- 版の正本は `Cargo.toml` の `[workspace.package] version = "0.0.1"` 1 か所。21 クレートが `version.workspace = true` で継承し、SHIORI へ渡す版（`crates/areka-ghost/src/config.rs`）と `baseware.version` もここから出る。exe に `--version` は無く、git のハッシュは zip の `BUILD-INFO.txt` にだけ書かれる。
- `tools/package-alpha.ps1`（542 行）: 道具の確認 → `rustup target add i686` → `cargo build --locked --release`（x64 本体・i686 補助 exe・`+crt-static`）→ 静的リンクの検査 → `cargo deny` → `cargo about` → 検体の展開 → staging → zip → 中身の検査 8 項目 → `-Check`（展開して起動する自己検査）。出力は `target/alpha/`。終了コード 0〜3。
- arm64 の zip は作ったことがない（完了 `alpha-package` の Out に「ARM64 版の zip」）。本体の arm64 ビルドは `tools/test-all.ps1` の仕組みの外で、`.kiro/steering/tech.md` の「ARM64 Windows ビルド」の手順（VC.Tools.ARM64・PowerShell）に従う。補助 exe は arm64 でも i686 のまま（32bit SHIORI を動かすため）。
- `resolve_root_from`（`boot_config.rs`）: `AREKA_ROOT` が在ればそれ、無ければ `current_exe().parent()`。リンクを解く処理は無い。

## Desired Outcome

1. 配布スクリプトが、**版入りの固定の名前**で zip を作る: `areka-{版}-x64.zip`・`areka-{版}-arm64.zip`（版は `Cargo.toml` から読む・手で渡さない）。隣に `areka-{版}-{arch}.zip.sha256`（`SHA256  ファイル名` の 1 行）。
2. `-Arch x64|arm64|all` で作り分けられる。arm64 の zip には arm64 の `areka.exe` と、i686 のままの `shiori-host32-helper.exe` が入る。中身の検査（PE の機械種別の確認を含む）は arm64 にも効く。
3. `BUILD-INFO.txt` に `version=` が加わる（`commit=` と並ぶ）。
4. CI（GitHub Actions の Windows ランナー）でも手元でも同じスクリプトで同じ zip ができる（`-Check` は窓を出すので CI では省けるようにする＝引数で分ける）。
5. areka は、リンク経由で起動されても自分の本当の場所（リンクの先）から根・補助 exe・プロファイルを引く。決定論テストで固定し、実機では `winget install --manifest <手元のマニフェスト>` で入れて `areka` と打って起動することで確かめる。
6. `dist/README.txt` の「入れ方」に winget の 1 行が増える準備（文面は `winget-manifest-submission` が入れる＝本 spec は版入りの名前への追随だけ）。

## Approach

- スクリプトは `tools/package-alpha.ps1` を育てる（名前は `tools/package.ps1` へ改めてよい。`package-check-temp-cleanup`（C1）が `-Check` の後片付けを先に直すので、その後に着手する）。版は `cargo metadata --no-deps --format-version 1` から読む。
- arm64 は `--target aarch64-pc-windows-msvc` のビルドを x64 の段と同じ形で足す。arm64 の道具が無い機械では「作れない」を終了コード 3 で止める（黙って x64 だけ作らない）。
- リンクの解決は `boot_config.rs` の `current_exe()` を使う 3 か所が共通の 1 関数（例 `real_exe_path()`）を通る形にし、Windows の最終パス（`GetFinalPathNameByHandleW` か `std::fs::canonicalize` の `\\?\` を外した形）で解く。パスの長さは emo2 の上限 160 字の決まりに注意（`\\?\` を付けたまま SHIORI へ渡さない）。
- 採らない案: zip の中にトップレベルのフォルダを作る（今の形を変えない）／zip の名前に日付とコミットを残す（版と重ねると長く、winget の道具の自動判定が迷う。日付とコミットは `BUILD-INFO.txt` にある）。

## Scope

- **In**: 版入りの名前・SHA256・`-Arch`・arm64 の zip・`BUILD-INFO.txt` の `version=`・CI で動く形（`-Check` の省略）・リンク経由の起動での場所の解決（コードと決定論テスト）・実機 1 回（手元のマニフェストで `winget install --manifest`）。
- **Out**: GitHub Actions の workflow そのもの（`release-ci-workflow`）／マニフェストの作成と提出（`winget-manifest-submission`）／版を上げる手順（`release-cycle`）／コード署名／exe の `--version`・Windows のファイル版の資源（欲しくなったら別に切る）／インストーラー（Inno など）。

## Boundary Candidates

- 配布スクリプト（名前・ハッシュ・arm64・CI 向けの引数）
- areka の自分の場所の解決（`boot_config.rs` の 1 関数）

## Out of Boundary

- 補助 exe の arm64 版（32bit SHIORI を動かす都合で i686 のまま＝設計どおり）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-alpha-package`（スクリプトの土台）・完了 `areka-P0-baseware-root-layout`（根の規則）。
- **Downstream**: `areka-P0-release-ci-workflow`（このスクリプトを CI から呼ぶ）・`areka-P0-winget-manifest-submission`（zip の名前と SHA256 を使う）・`areka-P0-release-cycle`。

## Existing Spec Touchpoints

- **Extends**: 完了 `alpha-package`（Out にあった「ARM64 版の zip」を引き取る）。
- **Adjacent**: `areka-P0-mcp-stdio-bridge`（同じスクリプトへ中継 exe の同梱を足す＝本 spec の後）・`areka-P0-install-companion-canon`（`boot_config.rs` を触りうる＝C2 へ回した）。

## Constraints

- 触るのは `tools/package-alpha.ps1`（改名するなら新しい名前）・`crates/areka/src/boot_config.rs` とその兄弟テスト・`.kiro/steering/structure.md`・`tech.md`（arm64 の手順の追記）。**`dist/README.txt` には触らない**（同じウェーブ C1 の `install-live-target-hazards` が触る。zip の名前の変化は説明書の本文に出てこない）。`crates/areka/src/main.rs`・`emo2_boot/`・`install/`・`Cargo.toml` の依存には触らない。
- 一時フォルダと実機の根はワークツリーの `target\` の下だけ。
- 実機の確かめは x64。arm64 の zip は「作れて中身の検査が通る」まで（arm64 の実機は開発者の手元に無ければ利用者の報告待ち＝既知の制限に書く）。

## 想定

- 規模 M（**合流後 11〜17 タスク**）。**ウェーブ C1**。議題 3 件（スクリプトの名前を `package.ps1` へ改めるか／判定が否のときに展開した木を証拠として残すか／消せなかったときの終了コード）。Opus で足りる。


---

## 合流した spec: `package-check-temp-cleanup`（2026-10-02・開発者「インストーラー関係は優先リリースしたい」）

> 同じ `tools/package-alpha.ps1` を触る道具のバグ（`-Check` の展開先と記録が `%TEMP%` に残る・`-CheckDir` で `target\` を指せない）を、本 spec の**先頭のタスク**として引き取った。別々の spec のままだと同じスクリプトを順に触ることになり、配布の列が 1 ウェーブ遅れる。合流しても 11〜17 タスクで上限 20 の内。以下は合流元の brief の全文（見出しは 1 段下げた）。

> 2026-10-02 `/kiro-discovery` で起票（`alpha-release-signoff` の完了の手順の中・開発者指示「あ、起票はあとでやってくれますよね。「実装完了を承認」スキルは最後に実施しますし。その前提で、今は実装に戻ってください。」）。roadmap「alpha-release-signoff の持ち越し」節。出どころは `alpha-release-signoff` の完成判定 `verification/alpha-completion.md` §6 と受入記録 `verification/acceptance-record.md` §8.6。**道具のバグ**（利用者には無関係・開発者の決まりに反する）。本文のソースの指し先は起票時（`c430480d`）の実測＝着手時に引き直すこと。

#### Problem

- **開発者**: `tools/package-alpha.ps1 -Check` を回すたびに `%TEMP%` の下に `areka-alpha-check-<HHmmss>`（展開した zip）と `areka-alpha-check-<HHmmss>-logs`（起動の記録）が残り、誰も片付けない。2026-10-02 に開発者の求め（「あとさっきの、テンポラリにある余計なフォルダの削除もお願い。」）で、AI が 38 個を手で消した。
- 開発者の絶対ルール「実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ（`C:\` 直下も `C:\tmp` も不可）」に、道具の既定のふるまいが反している。しかも `-CheckDir` でワークツリーの `target\` を指すことができない（下）。

#### Current State

- 展開先の親は `-CheckDir` が無ければ `[IO.Path]::GetTempPath()`、その下に `areka-alpha-check-<HHmmss>` を作る（「前提の確認」の段）。記録の置き場は展開先の名前に `-logs` を付けたもの（「短いパスへ展開」の段の `$script:LogDir`）。
- 展開先のフルパスの長さの上限 `EXPAND_DIR_MAX_CHARS = 160`（較正値の一覧）。短くする理由は完了 `alpha-package` の要件 3.1＝「リポジトリの外の新しい空の、パスの短い場所へ展開」——emo2 の SHIORI（pasta）が初回に `ghost/master/profile/` の奥へ書くファイルのパスが長すぎると、接続の失敗を出さずに黙る（同 spec の `research.md` §4.2）。ワークツリーのパス（例 `C:\home\maz\git\areka\.claude\worktrees\<名>\`）は深いので、上限に収まるかを着手時に実測する。
- **`-CheckDir` はリポジトリの中を断る**（終了コード 3「-CheckDir はリポジトリの外を指定する」）。理由はスクリプトのコメント「展開先はリポジトリの外（要件 3.1）。中だと target/ の下は git status でも捕まらない」＝完了 `alpha-package` の要件 3.1 と、最後の段「git status 不変の確認」の検査が `target\`（追跡外）の書き込みを捕まえられないこと。なお zip と途中物はもともと追跡外の `target/alpha/` に置いている。
- 後始末（`Invoke-Cleanup`）は `.zip.tmp` の削除と環境変数の復元だけで、展開先と記録は消さない。
- **記録は署名の根拠に引かれる**: `alpha-release-signoff` の受入記録 §1 は `-Check` の記録を生の記録の置き場へ `check-215146-logs\`・`check-221148-logs\` として写してから引いた。記録を残す手段は要る。

#### Desired Outcome

- `-Check` の既定の展開先と記録の置き場が、ワークツリーの `target\` の下になる（160 字の上限を守る）。`%TEMP%` には何も残らない。
- 展開した木（起動した `areka.exe` が書いた記憶などを含む）は、判定の後に片付く。片付けを止めて残す手段（引数）がある。
- 起動の記録（`run.log`・`run.stderr.log`）は、署名や受入記録が引けるように残る（置き場を印字する）。
- 「git status 不変の確認」は今どおり効く（`target\` の下の書き込みは追跡外なので、検査の意味は変わらない）。
- 子のプロセスは自分が起こしたものだけを止める今の作り（番犬）を保つ。

#### Approach

- 既定の親を `<リポジトリ>\target\alpha-check\` へ替え、`-CheckDir` のリポジトリの中を断る検査は `target\` の下に限って通す（要件 3.1 の理由＝「git status で捕まらない」は、片付けと記録の印字で埋める）。
- 判定の後に展開先を消し、記録だけを残す。消せなかったときは理由を印字して終了コードで知らせる（黙って残さない）。
- 完了 `alpha-package` の要件 3.1 を上書きすることになるので、本 spec の要件にその旨を書く。

#### Scope

- **In**: `tools/package-alpha.ps1` の展開先・記録の置き場・後始末・引数の説明（`Get-Help` の欄）・較正値の一覧、`tools/` の自己検査があればその追随、`.kiro/steering/structure.md` などの道具の説明の追随。
- **Out**: zip の中身と判定の 8 項目・起動の記録の判定の 6 条件・`test-all.ps1`・性能改善ループの道具（登記だけの行「`tools/perf` の自己検査の赤」）。過去の記録（`alpha-release-signoff` の受入記録に書いた `%TEMP%` のパス）は書き換えない。

#### Boundary Candidates

- 展開先と記録の置き場の決め方（「前提の確認」の段）
- 後始末（`Invoke-Cleanup` と `-Check` の各段）

#### Out of Boundary

- 配布物の組み方・release のビルド・謝辞の生成。

#### Upstream / Downstream

- **Upstream**: α の完成宣言（`alpha-release-signoff`）。完了 `alpha-package` の上に建つ。
- **Downstream**: 次の配布（α の更新版）の `-Check` と、その署名の記録。

#### Existing Spec Touchpoints

- **Extends**: なし（完了 `alpha-package` の要件 3.1 を上書きする）。
- **Adjacent**: 登記だけの行「`tools/perf` の自己検査の赤」（別の道具）。

#### Constraints

- **一時フォルダはワークツリーの `target\` の下だけ**（開発者の絶対ルール）。展開先のフルパスは 160 字以内（ワークツリーの深いパスでも収まることを確かめる）。
- 追跡しているファイルを 1 つも書き換えない（最後の段の検査を保つ）。プロセスは自分が起こしたと確かめたものだけを止める。
- `cargo`・`crates/` には触らない見込み。


---

#### 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **段はバグ（道具）・ウェーブ C1**。規模 XS〜S（3〜5 タスク）・Opus で足りる。
- brief の記述はすべて実物と一致した。**足りなかった事実**: スクリプトの `.EXAMPLE` が `-CheckDir C:\t` を勧めている（「一時フォルダはワークツリーの `target\` の下だけ」に反する）＝直す対象に足す。
- 長さの実測: ワークツリーの根が 64 字のとき `<根>\target\alpha-check\areka-alpha-check-HHmmss` は約 107 字＝emo2 の上限 160 字に収まる。
- **触るファイル**: `tools/package-alpha.ps1`（541 行）・`.kiro/steering/structure.md`（道具の説明）。`crates/` には触らない。
- **後ろに居る spec**: `mcp-stdio-bridge` が同じスクリプトへ中継 exe の同梱（`$ALLOWED_EXECUTABLES`・ビルドの段・配置と CPU 種別の検査）を足す＝本 spec が先。
- 小さな議題 2 つ: 判定が否のときに展開した木を証拠として残すか／消せなかったときの終了コードを新しく作るか。
