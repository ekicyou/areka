# areka — 伺か互換のデスクトップマスコット・ベースウェア

> Rust 製・Windows 用。デスクトップにキャラクター（ゴースト）を住まわせるアプリです。

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE-MIT)
[![Windows](https://img.shields.io/badge/Platform-Windows_10%2F11_(x64%2Farm64)-0078D6?logo=windows)](https://www.microsoft.com/windows)

<p align="center"><img src="doc/images/emo2-boot.png" alt="同梱のゴースト えも？？ が起動の挨拶をしているところ" width="560"></p>

<p align="center"><sub>同梱のゴースト「えも？？」。シェル: \0 側「コンフィズリー」（ゆゆぴか）・\1 側「City-Pop'n」（大槻）／バルーン: emo2-kakukaku（素材: フキダシデザイン）。絵の利用条件はそれぞれの作者に従います（<a href="dist/README.txt">dist/README.txt</a> の「同梱物とライセンス」）。</sub></p>

---

## これは何か

**areka** は「伺か」のベースウェアです。[ukadoc](https://ssp.shillest.net/ukadoc/manual/) を正典として、SSP の代わりに既存のゴースト・シェル・バルーンを動かすことを目指しています。

- 描画は Windows.UI.Composition（WUC）＋ Direct2D ＋ DirectWrite で、透過窓と、透明な所のクリックを下のアプリへ通す動きを GPU 合成のまま実現しています。
- 32 ビットの SHIORI（`shiori.dll` など）は、32 ビットの補助 exe（`shiori-host32-helper.exe`）の中で動かします。本体は 64 ビット（x64・arm64）です。
- 同梱のゴーストは **えも？？**（`emo2`・SHIORI は pasta の 32 ビット版）です。
- 将来は、areka ならではの表現（縦書きのタイプライター・シェルの中のバルーンなど）を使う旗艦ゴースト **ぱすたさん** も入れる予定です（まだ作れていません。構想は [doc/PASTA_PROFILE.md](doc/PASTA_PROFILE.md)）。

---

## いまできること（α 版・2026-10-02 時点）

α 版の目標は「第三者がデスクトップマスコットを管理できる」ことです。

- ゴーストの起動・会話・撫で・右クリックメニュー・終了
- `.nar` のインストール（メニューから、またはキャラクターへのドラッグ＆ドロップ）
- ゴースト・シェル・バルーンの切り替え
- ネットワーク更新
- 前回使ったゴースト・シェル・バルーンを覚えて、次の起動で戻す
- 前回きれいに終わらなかったときは、同梱の えも？？ で立ち上がる

動作を確かめているゴーストは次のとおりです（どれも `vendors/sample_ghost/` に配布形のまま置いています）。

| ゴースト | SHIORI | 備考 |
| --- | --- | --- |
| えも？？（`emo2`） | pasta（32 ビット） | 同梱。areka の適合の物差し |
| `R_POST_and_KOMAINU` | 里々 | 標準テンプレート |
| 紺野ややめ（`konnoyayame`） | YAYA | 標準テンプレート。シェルが CC BY-NC-ND のため開発用の検体としてだけ使い、配布物には入れません |
| 悪役令嬢クローディア（`claudia`） | YAYA | 同梱バルーンを 2 つ持つ検体 |

既定のバルーンは CC0 の [StayseeBalloon](https://github.com/ponapalt/StayseeBalloon) です。

表現力は「えも？？ が普通に動く」水準までで、ゴーストによっては表情や演出の一部が欠けます。複数体の同時表示・SSTP などはまだありません。制限の一覧は [dist/README.txt](dist/README.txt) の「既知の制限」にあります。

---

## 入手と起動

まだ GitHub Releases での配布はしていません。今は手元で配布物の zip を組みます（下の「ビルド」）。

zip を展開して `areka.exe` を開けば、同梱の えも？？ が立ちます。起動・終了・メニュー・`.nar` の入れ方・更新・記憶の置き場は、利用者向けの説明書 [dist/README.txt](dist/README.txt) にまとめてあります。

---

## ビルド

### 前提

- Windows 10/11
- Rust（stable・2024 Edition）と Visual Studio Build Tools（Windows SDK）
- PowerShell 7（`pwsh`）
- rustup のターゲット `i686-pc-windows-msvc`（32 ビットの補助 exe とテスト用 DLL）。`tools/` のスクリプトが自分で追加します
- arm64 版を作るときだけ: ターゲット `aarch64-pc-windows-msvc` と、Visual Studio の部品 `Microsoft.VisualStudio.Component.VC.Tools.ARM64`

ターゲットをまたぐビルドは PowerShell から実行してください（Git Bash では別の `link.exe` が先に見つかってリンクに失敗します）。

### コマンド

```powershell
# ビルド
cargo build

# 全体テスト（i686 の成果物の準備込み）
pwsh -NoProfile -File tools/test-all.ps1

# 配布物の zip（target/package/areka-<版>-<arch>.zip と .sha256）
pwsh -NoProfile -File tools/package.ps1            # x64
pwsh -NoProfile -File tools/package.ps1 -Arch all  # x64 と arm64
pwsh -NoProfile -File tools/package.ps1 -Check     # 組んだ zip を展開して起動まで確かめる
```

素の `cargo test --workspace` は使わないでください。32 ビットの SHIORI を扱うテストは i686 の成果物を要するので、それが無いと「先にビルドせよ」で止まります。`tools/test-all.ps1` は準備から判定までを 1 本で行います。

---

## 構成

伺かの仕組みを、7 つのエンジンに分けて作っています。

| エンジン | 役割 | 主なクレート |
| --- | --- | --- |
| ⓪ ghost | ゴーストの起動と各エンジンの結線・ゴーストの目録 | `areka-ghost` |
| ① shiori | SHIORI との通信（32 ビットの DLL は補助 exe 越し） | `shiori-abi`・`shiori-host32-*` |
| ② parsers | さくらスクリプトと `descript.txt`・`surfaces.txt` などの読み取り | `areka-parsers` |
| ③ kanade | 起動・会話・終了の進行（いつ・何を SHIORI に尋ねるか） | `areka-kanade` |
| ④ sakura | SHIORI が返したさくらスクリプトを時刻つきの台本へ変換して再生 | `areka-sakura` |
| ⑤ seriko | SERIKO のアニメーション | `areka-seriko` |
| ⑥ emo | 画像の合成と表示・文字とバルーンの描画 | `areka-emo-*` |

その下に共通の土台として、Windows の UI フレームワーク **`wintf`**（ECS（bevy_ecs）＋ WUC ＋ Direct2D ＋ DirectWrite・縦書き対応）と、時刻つきの台本を再生する **`dola`** があります。アプリ本体は `areka`（bin）で、`.nar` のインストールは `areka-nar`、ネットワーク更新は `areka-update`、MCP サーバ（`127.0.0.1`。ツールはこれから整備）は `areka-mcp` が受け持ちます。

詳しくは次を参照してください。

- [.kiro/steering/structure.md](.kiro/steering/structure.md) — クレートとモジュールの構成
- [doc/COMPAT_ARCHITECTURE.md](doc/COMPAT_ARCHITECTURE.md) — 互換ベースウェアとしての設計判断（正本）
- [doc/ukadoc-coverage/](doc/ukadoc-coverage/) — ukadoc の項目ごとの対応状況

---

## 開発の進め方

仕様駆動（Kiro 方式）で、1 つの機能を「要件 → 設計 → タスク → 実装」の順に進めています。

- [.kiro/steering/](.kiro/steering/) — プロジェクト全体の決めごと（`product.md`・`tech.md`・`structure.md`）
- [.kiro/steering/roadmap.md](.kiro/steering/roadmap.md) — ロードマップと次に着手する仕様（正本）
- [.kiro/specs/](.kiro/specs/) — 個々の仕様（完了したものは `completed/`）
- [doc/CONSTITUTION.md](doc/CONSTITUTION.md) — 設計理念と責務の分け方

---

## ライセンス

areka 本体は MIT ライセンスです（[LICENSE-MIT](LICENSE-MIT)）。本体が使っているライブラリの著作権表示は [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) にあります。

配布物に同梱するゴースト・バルーンには MIT は及ばず、それぞれの作者の条件に従います（[dist/README.txt](dist/README.txt) の「同梱物とライセンス」）。`vendors/sample_ghost/` の検体の出どころと条件は [vendors/sample_ghost/README.md](vendors/sample_ghost/README.md) にあります。

---

<sub>areka — 「あなたのデスクトップに、もうひとりの住人を」</sub>
