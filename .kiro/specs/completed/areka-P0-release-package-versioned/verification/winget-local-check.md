# 実機の確かめ: winget から入れてリンク経由で起動する（要件 7）

- 日付: 2026-10-03
- 機械: 開発機（Windows 11 Pro 10.0.26300・x64）・winget v1.29.380・Python 3.13.15
- コミット: `b8986d78`（タスク 4.2 の `exe_location` を含む）
- 判定: **合格**（`LinkType` が `SymbolicLink`・`root=` がパッケージのフォルダ・窓の行が 1 件）

## 変えた設定（開発者の手で・要件 7.4）

| 設定 | 変える前 | オンにしたことを確かめた時刻 | 戻した時刻 |
|---|---|---|---|
| winget の `LocalManifestFiles`（管理者の `winget settings --enable LocalManifestFiles`） | `false` | 13:09:37 | 13:54:33 に `false` を確認 |
| Windows の開発者モード（`AppModelUnlock` の `AllowDevelopmentWithoutDevLicense`） | 値なし（オフ） | 13:47:24（値 1） | 13:54:33 に値 0（オフ）を確認 |

開発者モードは、管理者でない winget がリンクを作るのに要る。オフのままだと winget はリンクの代わりにパッケージのフォルダを PATH に足すので、リンク経由の起動の確かめにならない。

## 手順と結果

1. `pwsh -NoProfile -File tools/package.ps1 -Check` で x64 の zip と `.sha256` を作り直した（6.1 の zip は使わない）。全段 緑・終了コード 0。
2. 手元のマニフェスト 3 ファイルを `target\package\winget-local\0.0.1\` に書いた（`Areka.Areka.Portable`・`InstallerType: zip`・`NestedInstallerType: portable`・`PortableCommandAlias: areka`・`InstallerUrl: http://127.0.0.1:8765/areka-0.0.1-x64.zip`・`InstallerSha256` は `.sha256` の値を大文字にしたもの・`ArchiveBinariesDependOnPath` は書かない）。`winget validate` は成功。
3. 13:47 に `python -m http.server 8765 --bind 127.0.0.1 --directory target\package` を起こした（要件 7.6）。
4. 普段の利用者の権限で `winget install --manifest target\package\winget-local\0.0.1` を実行した。ハッシュの検証が通り、「コマンド ライン エイリアスが追加されました: "areka"」「インストールが完了しました」と出て、終了コード 0。
5. `areka` の解決先とリンクの種類:

   | 項目 | 値 |
   |---|---|
   | `(Get-Command areka).Source` | `C:\Users\maz-o\AppData\Local\Microsoft\WinGet\Links\areka.exe` |
   | `LinkType` | `SymbolicLink` |
   | リンクの先 | `C:\Users\maz-o\AppData\Local\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\areka.exe` |
   | 利用者の PATH に足された WinGet のフォルダ | `…\WinGet\Links` だけ（パッケージのフォルダは足されていない） |

6. 13:48:12 に `areka` の 1 語で有界に起動した（`AREKA_*`・`WINTF_*` を外し、`AREKA_APP_SMOKE_EXIT_MS=10000`・`AREKA_NO_ALERT=1`・`RUST_LOG=info`・`NO_COLOR=1`。作業フォルダは `target\package\winget-local\0.0.1`）。13:48:23 に終了コード 0 で自分から終わった。記録は同じフォルダの `run.log`／`run.stderr.log`。
7. 記録の判定:

   | 項目 | 結果 |
   |---|---|
   | `root_resolved` の `root=` | `C:\Users\maz-o\AppData\Local\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource`（`source=ExeDir`）＝リンクの置き場（`…\WinGet\Links`）ではなくパッケージのフォルダ |
   | `本物のゴースト窓を開きました` の行 | 1 件 |
   | `exe_link_unresolved` の警告 | 0 件 |

8. 13:48:55 に外した。`winget uninstall --id Areka.Areka.Portable --purge` は「一致するインストール済みのパッケージが見つかりませんでした」で外せなかった。手元のマニフェストで入れた物は `winget list` で `ARP\User\X64\Areka.Areka.Portable__DefaultSource` という ID になっていたので、`winget uninstall --id 'ARP\User\X64\Areka.Areka.Portable__DefaultSource' --exact --purge` で外した（「インストール ディレクトリを破棄しています」「正常にアンインストールされました」）。その後、リンクとパッケージのフォルダはどちらも無い。
9. 13:49 に http.server を止めた。ポート 8765 で待ち受けている物は 0 件。

## 置き場（要件 7.3）

確かめに使った物（zip・`.sha256`・マニフェスト・`run.log`）はすべてワークツリーの `target\package\` の下にあり、git では追跡しない。入れた物は上の 8 で外した。

## 既知の制限（要件 7.5）

- arm64 の zip は、arm64 の実機で入れて起動する確かめをしていない。arm64 の zip で確かめたのは、中身の検査 8 項目（機械種別を含む）と `.sha256` だけ（`package-runs.md`）。
- winget に提出する本物のマニフェストでの確かめは `winget-manifest-submission` の仕事。今回の手元のマニフェストは確かめ専用。
