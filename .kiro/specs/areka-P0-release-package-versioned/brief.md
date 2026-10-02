# Brief: areka-P0-release-package-versioned

> 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）で起票。テーマの決めごとは `.kiro/steering/roadmap.md`「配布と公開」節。file:line は起票時値（main `76e17654`）。

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

- `-Check` の後片付けと展開先（`package-check-temp-cleanup`）
- 補助 exe の arm64 版（32bit SHIORI を動かす都合で i686 のまま＝設計どおり）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-alpha-package`（スクリプトの土台）・完了 `areka-P0-baseware-root-layout`（根の規則）・`areka-P0-package-check-temp-cleanup`（C1・同じスクリプト＝先）。
- **Downstream**: `areka-P0-release-ci-workflow`（このスクリプトを CI から呼ぶ）・`areka-P0-winget-manifest-submission`（zip の名前と SHA256 を使う）・`areka-P0-release-cycle`。

## Existing Spec Touchpoints

- **Extends**: 完了 `alpha-package`（Out にあった「ARM64 版の zip」を引き取る）。
- **Adjacent**: `areka-P0-mcp-stdio-bridge`（同じスクリプトへ中継 exe の同梱を足す＝同時に走らせない）・`areka-P0-install-companion-canon`（C1・`boot_config.rs` を触りうる＝C1 の後）。

## Constraints

- 触るのは `tools/package-alpha.ps1`（改名するなら新しい名前）・`crates/areka/src/boot_config.rs` とその兄弟テスト・`.kiro/steering/structure.md`・`tech.md`（arm64 の手順の追記）・`dist/README.txt`（必要なら）。`crates/areka/src/main.rs`・`emo2_boot/`・`install/`・`Cargo.toml` の依存には触らない。
- 一時フォルダと実機の根はワークツリーの `target\` の下だけ。
- 実機の確かめは x64。arm64 の zip は「作れて中身の検査が通る」まで（arm64 の実機は開発者の手元に無ければ利用者の報告待ち＝既知の制限に書く）。

## 想定

- 規模 S〜M（8〜12 タスク）。議題 1 件（スクリプトの名前を `package.ps1` へ改めるか）。Opus で足りる。
