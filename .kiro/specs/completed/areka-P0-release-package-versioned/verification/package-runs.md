# 配布スクリプトの実走の記録（タスク 6.1）

- 日付: 2026-10-03（12:43〜12:59）
- コミット: `a169560`（HEAD `a1695602`）・各走りの開始時点で未コミットの変更 0 件（走り 3 の版の書き換え中と走り 5 の書き換え中だけ 1 件。どちらも戻した）
- 機械: Windows 11 Pro 10.0.26300（AMD64）・PowerShell 7.6.6・cargo／rustc 1.99.0
- 起動の形: どの走りも `pwsh -NoProfile -File tools/package.ps1 <引数>`（PowerShell から）
- 対応する設計: design.md「Testing Strategy／配布スクリプトの検査（実走・結果を `verification/` に記録）」の表

## 走りごとの結果

| # | 走り（引数・前提） | 終了コード | 印字の要点 | 判定 |
|---|---|---|---|---|
| 1 | `-Arch all -Check`（前の作業の 0.0.1 の 2 組が在る状態） | 0 | 始めに「前回の物を消した」4 行・全段 OK・x64／arm64 とも「判定 1〜8 すべて合」・記録の判定 7 項目すべて合（窓 1 件・挨拶 1 件・バルーンは同梱の `emo2-kakukaku`）・「展開した木を消した: …\check-124307」・「記録（残す）: …\check-124307-logs」・完成で zip と sha256 の絶対パスと `version: 0.0.1` を 2 組印字・「全段 緑」 | 期待どおり |
| 2 | `-Arch all -Check -KeepExpanded` | 0 | 「展開した木を残した（-KeepExpanded）: …\check-125308」・記録の置き場も印字・木（9 項目）と `-logs` が残った | 期待どおり |
| 3a | `-Arch foo` | 3 | `-Arch は x64・arm64・all のどれかで指定する（受け取った値: 'foo'）` | 期待どおり |
| 3b | `-Check -Arch arm64` | 3 | `-Check の起動確認には x64 の zip が要る（-Arch x64 か all と組み合わせる・受け取った値: 'arm64'）` | 期待どおり |
| 3c | `-Check -CheckDir <リポジトリ>\doc` | 3 | `-CheckDir はリポジトリの中なら <リポジトリ>\target\ の下を指定する（受け取った値: …\doc）` | 期待どおり |
| 3d | 引数なし・`Cargo.toml` の `[workspace.package] version` を一時的に `0.0.1+x` に変えた状態 | 3 | `版の形が違う（'0.0.1+x'・受け付ける形 ^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$。+ の付記は付けない）` | 期待どおり |
| 4 | `-Check -CheckDir <リポジトリ>\target\package\deep\d…(60 字)\e…(40 字)` | 3 | `展開先が長すぎる（206 文字・上限 160）: …\check-125548 — -CheckDir に短いパス…を指定するか…` | 期待どおり |
| 5 | `-Arch all`・`Test-ZipContent` の本体の期待の機械種別を `0x014c` に書き換えた状態（走り 2 の 2 組が在る状態） | 1 | 始めに「前回の物を消した」4 行・`否 5 機種が違う: areka.exe は 0x8664（期待 0x014c）`・`FAIL x64 中身の判定（0 秒・中身の判定で否が 1 件）`。終わりに `areka-0.0.1-*` は 0 件・`.tmp` 0 件 | 期待どおり |
| 6 | `-Arch x64`・子のプロセスだけに `CARGO_BUILD_JOBS=0` を入れてビルドを落とす（走り 7 の 2 組が在る状態） | 1 | 始めに x64 の zip と `.sha256` の「前回の物を消した」2 行・`error: jobs may not be 0`・`FAIL i686 helper ビルド（0 秒・終了コード 101）`。終わりに x64 の完成品 0 件・`.tmp` 0 件（頼んでいない arm64 の組はそのまま） | 期待どおり（2.4・2.5） |
| 7 | `-Arch all`（`-Check` なし・`-NonInteractive`・入力は空） | 0 | 段は 前提の確認〜arm64 SHA256 → git status 不変の確認 → 完成 だけで、「短いパスへ展開」「起動」の段も「子のプロセス番号」の行も無い・入力を待たずに 38 秒で終わった・2 組ができた | 期待どおり |
| 8 | `-Arch all -Check`（戻した後の締めの緑の走り） | 0 | 「前回の物を消した」は arm64 の 2 行（x64 は走り 6 で消えている）・全段 OK・記録の判定すべて合・「展開した木を消した: …\check-125800」・「全段 緑」 | 期待どおり |

走り 3a〜3d と 4 は、どれも印字が「==> 前提の確認」と上の 1 行だけでビルドの段に進んでいない。走りの前後で `target/package/` 直下のファイル（zip・`.sha256`・置いた空ファイル）の名前・更新時刻・大きさを比べ、差は 0 件（新しい zip は無い）。走り 4 の `deep` フォルダも作られていない。

### 成功の走り（1 と 8）の追加の確かめ

| 確かめ | 走り 1 | 走り 8 |
|---|---|---|
| `target/package/` の完成品 | zip 2・`.sha256` 2 | zip 2・`.sha256` 2 |
| `.tmp` の件数（`target/package/` 以下を再帰） | 0 | 0 |
| `check-<HHmmss>` | 消えて `-logs` だけ残った | 消えて `-logs` だけ残った |
| Git Bash で `cd target/package && sha256sum -c areka-0.0.1-x64.zip.sha256 areka-0.0.1-arm64.zip.sha256` | 両方 `OK` | 両方 `OK` |
| zip の中の `BUILD-INFO.txt` | x64: `version=0.0.1`・`arch=x64`／arm64: `version=0.0.1`・`arch=arm64`（`commit=a169560`・`dirty=0`） | 同じ |
| `git status --porcelain`（走りの前後） | 空のまま | 空のまま |
| 段「後片付け」の秒数 | 0 秒 | 0 秒 |

## 後片付けの秒数

走り 1・8 とも段「後片付け」は「0 秒」と印字された（印字は整数秒への丸めなので 0.5 秒未満）。1 回目の削除で消えており、再試行（5 回×1 秒）には入っていない。3.1 で見た 4 秒は今回は出なかったので、較正値 `REMOVE_RETRY`・`REMOVE_RETRY_WAIT_SEC` は変えなくてよい。

## `%TEMP%`（`C:\Users\maz-o\AppData\Local\Temp`）の比較

直下の名前の一覧を走りの前後で比べた。

- 走り 1 の前（10,700 件）と後: 増えたのは `b.log`・`t.log`・`tt.log`（12:43〜12:46 作成・中身は `cargo test` の dev ビルドとテスト 823 件の出力）と `host32-load-e2e-{ok,fail,absent}-27924-*`・`host32-request-e2e-req-32740-0`（12:51 作成・プロセス番号 27924／32740 のテスト）。どれも他のセッションのテストが作った物で、本スクリプト（release ビルドだけでテストを回さない・起動した子は 11744）の物ではない。減ったのは無関係の `lnk{…}.tmp` 1 件。
- 走り 8 の直前と直後: 差は 0 件。

## 一時的に変えた物と戻したことの確かめ

| 変えた物 | 変え方 | 戻し方 | 戻ったことの確かめ |
|---|---|---|---|
| `Cargo.toml` の 8 行目 `version = "0.0.1"` | 走り 3d の間だけ `"0.0.1+x"` に | 走りの前に `target\` の下へ取った写しを書き戻し、更新時刻を今にした | SHA256 が変更前と同じ（`184CB822…7103`）・`git diff -- Cargo.toml Cargo.lock` 空。`Cargo.lock` は走り 3d で書き換わっていない（SHA256 `4C8DC791…FA87` のまま） |
| `tools/package.ps1` の `Test-ZipContent` の `'areka.exe' = $ARCHS[$Arch].Machine` | 走り 5 の間だけ `'areka.exe' = 0x014c` に | 写しを書き戻し、更新時刻を今にした | SHA256 が変更前と同じ（`B173AF8A…35BA`）・`git diff -- tools/package.ps1` 空 |
| `CARGO_BUILD_JOBS=0` | 走り 6 の子の pwsh の中だけで設定 | 子の終了とともに消える | 呼んだ側のシェルでは未設定のまま。その後の走り 8 が緑 |

最後に `git diff --stat -- tools/package.ps1 Cargo.toml Cargo.lock` が空・`git status --porcelain` が空であることを確かめた（この記録のファイルを足す前）。

## 残した物・消した物

- 残した: 走り 8 の完成品 2 組と記録 `target\package\check-125800-logs\`（締めの緑の走りの手がかり）。走りの出力と写しは `target\verify61\` に置いた（`target\` の下なので追跡外）。
- 消した: 走り 1 の `check-124307-logs`・走り 2 の `check-125308` と `check-125308-logs`・置き場の変化を見るための空ファイル。`target\` の外には何も作っていない。
