# 設計レビュー: areka-P0-release-package-versioned

> 2026-10-03・`/kiro-validate-design`（非対話・サブエージェント）。対象は `design.md`（2026-10-03 生成）・`requirements.md`（確定）・`research.md` §1〜§9・`brief.md`・steering（`structure.md`・`tech.md`・`roadmap.md`・`product.md`）。設計がコードを引く主張は、ブランチ `claude/areka-p0-release-package-688ad4` の実物（`tools/package-alpha.ps1`・`crates/areka/src/boot_config.rs`・`crates/areka/src/main.rs`）で確かめた。

## まとめ

設計は「配布スクリプトを改名して 1 ファイルのまま育てる」「本体は判断を純粋な関数に置き、I/O を注入し、結果を 1 回だけ覚える」の 2 本で、要件 1〜8 の 63 条項すべてに対応する部品と判断が表で引ける。コードを引く主張（起動した exe のパスを使うのは `boot_config.rs` の 3 関数だけ・ログの初期化が `resolve_boot` と `default_helper_exe_path` より前・`main.rs` は変更 0 で済む）はいずれも実物と一致した。実装に進める品質で、下の 2 点を設計討議で詰めれば十分。

## 実物で確かめた設計の主張

| 主張（design.md） | 確かめた場所 | 結果 |
|---|---|---|
| 起動した exe のパス（`current_exe()`）を使うのは `resolve_root`・`default_helper_exe_path`・`default_app_profile_dir` の 3 か所だけ | `crates/areka/src/` を `current_exe` で全文検索 | 一致（本文の呼び出しは `boot_config.rs` の 3 関数のみ。他は説明文） |
| ログの初期化は `resolve_boot` と `default_helper_exe_path` の呼び出しより前なので初回の警告が記録に載る | `main.rs` の `fn main`: `tracing_subscriber::fmt().init()` → `resolve_boot(&args)` → `default_helper_exe_path()` の順 | 一致 |
| 3 関数の呼び手（`main.rs`・`ghost_session.rs`・`shiori_host.rs`）は変更 0 | 3 関数の名前と戻りの型は変えない設計 | 成り立つ（呼び手は関数名でしか結びついていない） |
| 配布スクリプトは `.zip.tmp` → 改名の流儀・`Exit-Script` → `Invoke-Cleanup` の後始末・番犬は自分の子だけ | `package-alpha.ps1` の「圧縮」「完成」の段・`Invoke-Cleanup`・「番犬」の段 | 一致 |
| 中身の検査 5 番の機械種別の表は固定・8 番は `commit=`・`dirty=` だけ | `Test-ZipContent` の `$machines` と `BUILD-INFO.txt` の検査 | 一致（設計の変更点がそのまま当たる） |
| 旧名の在る生きている文書は `roadmap.md`（3）・`product.md`（1）・brief 3 本 | steering と `.kiro/specs/*/brief.md` を検索 | 一致（`mcp-stdio-bridge`・`mcp-server-core`・`release-ci-workflow` の brief） |
| `vswhere.exe` は VS Installer に同梱 | `C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe` の実在と `where.exe vswhere` | 実在する。**ただし PATH には無い**（下の重要な点 1） |

## 重要な点（最大 3）

### 1. `vswhere.exe` の探し方が設計に無い（PATH に無いので名前で呼ぶと必ず止まる）

- **問題**: 設計は前提の確認で「`-Arch` に arm64 を含むときだけ `vswhere` と ARM64 の部品」を見て、無ければ終了コード 3 とする。しかし `vswhere.exe` は VS Installer のフォルダ（`%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\`）に在るだけで PATH には載らない（この開発機で実測: `where.exe vswhere` は見つからず、固定の場所には在る）。設計の較正値の表にも S1 にも、固定の場所で探すことが書かれていない。
- **影響**: 名前で呼ぶ実装になると、ARM64 の部品がそろった機械でも `-Arch arm64|all` が常に「vswhere.exe が無い」で 3 になり、arm64 の zip が一度も作れない。GitHub の Windows ランナーでも同じ（`vswhere` は固定の場所に在り PATH には無い）。「黙って x64 だけ作らない」の守りが「常に止まる」へ倒れる。
- **提案**: 較正値に `VSWHERE_PATH = "$([Environment]::GetFolderPath('ProgramFilesX86'))\Microsoft Visual Studio\Installer\vswhere.exe"` を 1 行足し、S1 は「固定の場所 → 無ければ PATH の `vswhere`」の順で探す。3 で止めるときの文には探した場所も載せる。`tech.md` の追記（8.4）にも同じ場所を書く。
- **要件**: 3.4・5.3（・8.4）
- **根拠**: design.md「S1 前提の確認と版と置き場」・「Technology Stack」の `vswhere` の行・「較正値」の表（`ARM64_VS_COMPONENT` だけで場所の項が無い）

### 2. 「完成品が在る ⇔ 終了コード 0」を支える `Invoke-Cleanup` が終了コードを受け取る口を持たない

- **問題**: 設計 S7 は「`Invoke-Cleanup` は `.tmp` と、終了コードが 0 でないときの `$script:Finalized` を消す」とする。今の `Invoke-Cleanup` は引数を取らず、`Exit-Script($Code)` からも最後の成功の経路からも同じ形で呼ばれる。終了コードを渡す契約を設計に書かないと、実装が「`$script:Finalized` を常に消す」（成功の最後で完成品を消す）か「常に残す」（改名の途中で落ちたとき片方の完成品が残り 3.7 が破れる）のどちらかへ転びうる。
- **影響**: 2.5・3.7 の「作りかけも片方だけの完成品も残さない」の根拠がスクリプトの中の 1 関数の暗黙の約束になる。実装者が気付けば一瞬で直るが、設計の不変条件の説明が実物の関数の形と食い違っている。
- **提案**: 設計 S7 の `Invoke-Cleanup` を `Invoke-Cleanup([int]$Code)` と書き、「`Exit-Script` は自分の `$Code` を渡す・成功の最後は `Invoke-Cleanup 0`」の 2 行を足す。あわせて「`$script:Finalized` は改名が済んだものから順に登録する」（途中で落ちたときに済んだ分だけ消える）を明記する。
- **要件**: 2.5・3.7（・5.4）
- **根拠**: design.md「S7 完成と後始末」・「Error Handling › 配布スクリプト」の「失敗の経路はすべて `Exit-Script` → `Invoke-Cleanup`」。実物は `package-alpha.ps1` の `function Invoke-Cleanup`（引数なし）と `function Exit-Script([int]$Code, …)`。

## 設計の強み

1. **「完成を最後へ」の 1 つの決め方で 2.4・2.5・3.7 がそろう**。全 CPU 種別の zip と `.sha256` を仮の名前で作り、git status の確認の後の最後の段で一度に改名する。起動確認は `.zip.tmp` のまま展開できるので段の並びも崩れない。今のスクリプトの `.zip.tmp` → 改名の流儀をそのまま広げた形で、新しい仕組みを 1 つも足していない。
2. **本体側は「判断は純粋・I/O は注入・1 回だけ覚える」で、境界を 1 つも破らない**。`follow_exe_links` は `resolve_root_from` と同じ注入の型で、リンクを作れない開発機でも全分岐（7 件）を決定論テストで踏める。`std::fs::read_link` の綴りをそのまま使い `canonicalize` を避けるので、6.3（リンクでないときは今どおり）と 6.5（`\\?\` を付けない）が同じ仕組みで満たされる。`Cargo.toml`・`main.rs` の変更 0 は実物の呼び手の形から成り立つ。

## 残る危うさ（議題にはしない・実装と実機の確かめで見る）

- **winget が `http://127.0.0.1` の `InstallerUrl` から zip を取れるか**は、要件 7.6 で手段が決まっているが、この機械ではまだ通していない。実機の確かめの手順（`verification/winget-local-check.md`）で取れなかったときに「何が出たか」を記録に残し、必要なら配る口を代える（設計は開発者の裁量と明記済み）。
- `AREKA_PROFILE_DIR` が勝つ分岐（6.4）の決定論テストは新設しない設計。コードは変わらず、`crates/areka/tests/smoke_boot_loop_exit.rs` が子プロセスにこの env を渡して実際に踏んでいるので、今回は足りる。
- `TooManyHops` のテストで「`probe` の呼ばれた回数が上限と一致」と書くとき、上限 32 を「32 回辿ったら止める」と「33 回目で止める」のどちらに読むかを実装時にテストの名前で決める（どちらでも要件は満たす）。

## 判定

**GO**。既存の骨組みの延長で新しい依存が 0、要件の全条項が部品と判断に結びつき、コードを引く主張は実物と一致した。上の 2 点は設計の文言の補いで済み（較正値 1 行と関数の引数 1 つ）、実装の形を変えない。

### 次の段

1. 設計討議で重要な点 1・2 を設計に書き足す（`VSWHERE_PATH` の較正値と探す順・`Invoke-Cleanup([int]$Code)` の契約）。
2. `/kiro-spec-tasks areka-P0-release-package-versioned` でタスクを生成する。
