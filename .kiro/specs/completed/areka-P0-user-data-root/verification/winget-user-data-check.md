# 実機の確かめ: winget で入れた形・機械の全員向け・zip の形（タスク 8.1・8.2）

- 日時: 2026-10-17（+09:00）08:35〜09:04
- 確かめた版: このブランチ `claude/areka-p0-user-data-root-12e486` のコミット `bba43e5c`。`tools/package.ps1 -Arch x64` で組んだ zip（SHA256 `0AF9920B979446E930619AD186CBCDDDB13E72994AE9FEE25FA98C979F579EFB`）
- 手順と物差し: 出どころの spec の `completed/areka-P0-winget-manifest-submission/verification/winget-local-check.md` と同じ（確かめ専用のマニフェスト・一覧の比べ方・右クリックメニューでの状態づくり・後片付けの確かめ）
- 作業の場所: ワークツリーの `target\user-data-check\`（マニフェスト・一覧・記録・道具）。利用者ごとの置き場は `target\ul`・`target\um`・`target\uw`・`target\uzl` の下

## 結果（要件 9.1・10.8〜10.10）

| 数えた物 | 結果 |
|---|---|
| 0.0.2 からの最初の上げ直しで消えた物（9.1） | **6**（出どころの spec と同じ数え方: 後から入れたゴースト 1＝`ghost\claudia`・後から入れたバルーン 3＝`balloon\claudia`・`balloon\claudia_vertical`・`balloon\emo2-kakukaku-wplimit`・ゴーストの記憶 2＝えも？？の `ghost\emo2\ghost\master\profile\areka` とクローディアの `ghost\claudia\ghost\master\profile\areka`。クローディアの記憶はゴーストのフォルダの中なので、ゴースト 1 と重ねて数えている）。ファイルの数では 358 → 152 で 207 消え、1 増えた（`areka-mcp-bridge.exe`）。消えた 207 の内わけは `ghost\claudia` 69・`balloon\claudia` 37・`balloon\claudia_vertical` 31・`balloon\emo2-kakukaku-wplimit` 20・同梱の えも？？ の `ghost\master\profile\`（areka と pasta）50。areka の記憶（`profile\areka\sylphya.toml`）は同じ SHA256 で残り、この版が最初の起動で書ける根へ引き継いだ |
| 利用者向けの上げ直し（この版 → この版）で消えた物（10.8） | **0**（書ける根の 301 ファイルが、大きさと SHA256 まで同じ） |
| 外し方で消えた利用者の物（10.8） | **0**。ただし**入れ先のフォルダは残った**（要件 10.8 の「入れ先のフォルダが残らないこと」には届いていない）。残った物はファイル 1（`profile\areka\sylphya.toml`）と空のフォルダ `.nar-work`。どちらも 0.0.2 の時代の物で、記憶は 0.0.2 が隣へ書き、この版が引き継いだ後も消さない元（要件 5.4 の「隣は消さない」）。`.nar-work` は 0.0.2 で .nar を入れたときにできた物と推定（一覧はファイルの単位なので空のフォルダの出どころは区別できない）。0.0.2 を経ずに**この版だけを入れて外す場合は、下の「追加の実測」で入れ先のフォルダが残らないこと（残った物 0）を確かめた** |
| 機械の全員向け: 記憶の書き込みの失敗（10.8） | **0**。「インストール…」でゴーストを入れられた。同梱のゴーストの記憶は写しの中に書けた |
| 実行ファイルのフォルダの下に作った物（4.1） | **0**（利用者向け・機械の全員向けとも、起動の前後の一覧が同じ。一覧はファイルの単位で、フォルダの数も前後で同じ＝利用者向け 25・機械の全員向け 22） |
| `%LOCALAPPDATA%\VirtualStore\` の下の areka の物 | **0**（フォルダは在り、中は 0 件。始める前も 0 件。外した後の読みは件数 0 だったので一覧のファイルは空のまま書かれていない） |
| zip の形（10.9） | 書ける根を使わない（`mode="beside" reason="none"`）。隣の `profile\areka` に記憶が書かれ、2 回目の起動で `route=Memory` として効いた |

- 説明書（`dist/README.txt`）の 0.0.2 の注意の行は、上の 6 の内わけに合わせて書き方を確定した（このタスクが説明書で直したのはこの 1 行だけ）。

## 追加の実測: この版だけを入れて外す（2026-10-17 09:09〜09:19・開発者の依頼）

上の外し方は 0.0.2 から上げ直した後なので、0.0.2 の名残（記憶 1 ファイル・空の `.nar-work`）が入れ先に残った。0.0.2 を経ない場合を測り直した。

| 数えた物 | 結果 |
|---|---|
| 外し方で消えた利用者の物 | **0**（書ける根 `target\uf\areka` の 299 ファイルが、外す前と後で大きさと SHA256 まで同じ） |
| 入れ先のフォルダ | **残らなかった（残った物 0）**。要件 10.8 の「入れ先のフォルダが残らないこと」を満たす |
| 実行ファイルのフォルダの下に作った物 | **0**（起動の前後でファイルの一覧が同じ・フォルダの数も 22 のまま） |
| winget が残した利用者の PATH の項目 | 1 件（範囲外。開発者が手で除いた） |

| 時刻 | 操作 | 誰 |
|---|---|---|
| 09:09 | 手元の口を起こし直した（PID 30988） | AI |
| 〜09:16 | `winget settings --enable LocalManifestFiles`（管理者の端末） | 開発者 |
| 09:16 | `winget install --manifest …\manifest-0.0.3.2`（普段の権限・入れ先は無い状態から） | AI |
| 09:16〜09:18 | 起こし（`LOCALAPPDATA` は `target\uf`）、「インストール…」で `claudia.nar` と `emo2-kakukaku-wplimit.nar` を入れ、「終了」。記録は `user_data_layout mode="user_root" reason="winget_install"` | AI |
| 09:18 | `winget uninstall --id "ARP\User\X64\Areka.Areka.Portable__DefaultSource" --exact`（`--purge` なし）。winget は「ファイルはインストール ディレクトリに残ります」を出さず、入れ先は無くなった | AI |
| 09:18 | 手元の口を止めた（コマンドラインで自分の物と確かめてから） | AI |
| 〜09:19 | `LocalManifestFiles` を戻した（管理者の端末）・利用者の PATH の残り 1 件を除いた | 開発者 |
| 09:19 | 後片付けを読んだ: 管理者向けの設定は 6 つとも `false`・利用者の PATH と機械の PATH は始める前の文字列と 1 字も違わない・入れ先は 2 つとも無い・動いている areka は 0 | AI |

## 設計からずれたこと（開発者の裁定 2026-10-17）

- **書ける根の向け先**: 設計は `AREKA_USER_ROOT` で `target\` の下へ向けるとしていたが、それでは判断の理由が `env_var` になり、winget の控えと書けない場合の道を通らない。開発者の裁定で、**起こす子のプロセスだけ `LOCALAPPDATA` を `target\` の下へ差し替えた**。areka の判断の道は本物のまま通り、書ける根は `target\` の下に作られた。差し替えた `LOCALAPPDATA` の下には、OS が空のフォルダ `Microsoft\Windows\Caches` を作った（areka の物ではない）。
- **機械の全員向けの理由**: 設計は機械の全員向け（`Program Files`）で `reason=unwritable` を見るとしていたが、winget は機械の全員向けにも控えのファイル（`Areka.Areka.Portable__DefaultSource.db`）を置くので、判断の順（控え → 書けない）どおり **`reason="winget_install"`** になった。書けない場合の道は、zip を `target\ux` に展開し、自分の利用者から「物を足す」権限（`WD,AD`）だけを外したフォルダで 1 度確かめた（**`reason="unwritable"`**・`marker=""`）。外した権限は確かめの直後に戻した。

## 記録の抜き書き

`<wt>` はワークツリーの根、`%LOCALAPPDATA%` は本物の利用者のフォルダ。

利用者向け・この版の最初の起動（0.0.2 から上げ直した後・`2-run1`）:

```text
user_root_created dir=<wt>\target\ul\areka
app_memory_migrated from=%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\profile\areka to=<wt>\target\ul\areka\profile\areka files=1
user_data_layout mode="user_root" reason="winget_install" user_root=<wt>\target\ul\areka origin="default" marker="Areka.Areka.Portable__DefaultSource.db"
root_listed path=<wt>\target\ul\areka origin="default" user_root="default" found=true occasion="boot"
root_listed path=%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource origin="default" user_root="" found=true occasion="boot"
user_data_copied kind="ghost" from=%LOCALAPPDATA%\...\ghost\emo2 to=<wt>\target\ul\areka\ghost\emo2 files=93 bytes=6908857 ms=172
```

上げ直した後の起動（`2-run2`）: `user_data_layout` は同じ形、`ghost_resolved route=Memory dir=<wt>\target\ul\areka\ghost\claudia`・`balloon_resolved route=Memory dir=<wt>\target\ul\areka\balloon\claudia`（写しは作り直さない）。

機械の全員向け（`4-run1`）:

```text
user_data_layout mode="user_root" reason="winget_install" user_root=<wt>\target\um\areka origin="default" marker="Areka.Areka.Portable__DefaultSource.db"
root_listed path=C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource origin="default" user_root="" found=true occasion="boot"
user_data_copied kind="ghost" from=C:\Program Files\...\ghost\emo2 to=<wt>\target\um\areka\ghost\emo2 files=93 bytes=6908857 ms=185
```

書けないフォルダ（`5-unwritable`）:

```text
user_data_layout mode="user_root" reason="unwritable" user_root=<wt>\target\uw\areka origin="default" marker=""
```

zip の形（`6-zip-run1`）:

```text
user_data_layout mode="beside" reason="none" user_root= origin="" marker=""
root_listed path=<wt>\target\uz origin="default" user_root="" found=true occasion="boot"
```

## 流れ（誰が何をしたか）

| 時刻 | 操作 | 誰 |
|---|---|---|
| 08:35 | 始める前の状態を読んだ（管理者向けの設定は 6 つとも `false`・利用者の PATH 12 項目・機械の PATH 18 項目・`VirtualStore` 0 件・入れ先は無い） | AI |
| 08:38 | `winget settings --enable LocalManifestFiles`（管理者の端末） | 開発者 |
| 08:39 | zip を `127.0.0.1:8765` で配る手元の口を起こした（PID 28956）。確かめ専用のマニフェスト 2 つ（版 `0.0.3.1`・`0.0.3.2`、x64 だけ）は `winget validate` が通った | AI |
| 08:39〜08:40 | `winget install --manifest dist\winget\0.0.2`（普段の権限・GitHub の 0.0.2 の zip） | AI |
| 08:40〜08:43 | 0.0.2 を起こし、右クリックメニューで `claudia.nar` と `emo2-kakukaku-wplimit.nar` を入れ、クローディアへ切り替え、「終了」 | AI（開発者の許しを得て、自分が起こした areka の窓だけを押した） |
| 08:43 | `winget upgrade --manifest …\manifest-0.0.3.1`（0.0.2 → この版）。一覧を比べて消えた物を数えた | AI |
| 08:44〜08:46 | この版を起こし、同じ 2 つを入れ、クローディアへ切り替え、「終了」。「置き場を先頭へ ▸」の子は 2 つとも選べず、「置き場を外す ▸」は書ける根だけ選べないことを画面で見た | AI |
| 08:46 | `winget upgrade --manifest …\manifest-0.0.3.2`（この版 → この版）。有界で起こし直した | AI |
| 08:47 | `winget uninstall --id "ARP\User\X64\Areka.Areka.Portable__DefaultSource" --exact`（`--purge` なし） | AI |
| 08:53 | `winget install --manifest …\manifest-0.0.3.2 --scope machine`（管理者の端末） | 開発者 |
| 08:53〜08:54 | 普段の権限で起こし、「インストール…」でクローディアを入れ、「終了」 | AI |
| 08:54〜08:55 | 書けないフォルダの確かめと、zip の形の 2 回の起動（どちらも有界） | AI |
| 08:56 | 手元の口を止めた（コマンドラインで自分の物と確かめてから） | AI |
| 〜09:04 | 機械の全員向けを `--purge` で外し、機械の PATH の残り 1 件を除き、`LocalManifestFiles` を戻した（管理者の端末）。利用者向けの入れ先の残りを消し、利用者の PATH の残り 1 件を除いた | 開発者 |
| 09:04 | 後片付けを読んだ: 管理者向けの設定は 6 つとも `false`・利用者の PATH と機械の PATH は始める前の文字列と 1 字も違わない・入れ先は 2 つとも無い・動いている areka は 0 | AI |

## 表の外で分かったこと

- この画面（Claude のアプリ）の Bash から動かすと、`%LOCALAPPDATA%` への読み書きがアプリの箱の中（`%LOCALAPPDATA%\Packages\Claude_…\LocalCache\Local\`）へ逸らされる。PowerShell から動かすと逸らされない。winget と areka は、すべて PowerShell から動かした。
- winget の `--id Areka.Areka.Portable` では外せない（入れた物の ID は `ARP\User\X64\Areka.Areka.Portable__DefaultSource`）。出どころの spec と同じく、`winget list` で引いた ID を `--exact` で渡した。
- `--purge` なしで外した後も、winget は利用者の PATH の項目を消さなかった（出どころの spec と同じ）。機械の全員向けの PATH の項目も、開発者が手で除いた。
