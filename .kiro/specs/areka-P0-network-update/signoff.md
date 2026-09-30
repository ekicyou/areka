# 実機確認の記録（areka-P0-network-update・要件 9.16・9.17）

design の Testing Strategy「実機」の項目 ⑴〜⑷。3 回走らせた。1 回目（emo2）は配布サイトの不備で失敗の経路だけを踏み、2 回目（あやめ）で要件 5.9 の欠けが見つかり、直した後の 3 回目（あやめ）で全項目が通った。

## 共通

- 環境: Windows 11 Pro 10.0.26200
- 実行体: `pwsh -NoProfile -File tools/package-alpha.ps1` で HEAD から組んだ配布物を、ワークツリーの `target\nu\root*` へ展開した。本番の SHIORI は 32bit（補助プロセス `shiori-host32-helper.exe` 経由）
- 環境変数: `NO_COLOR=1`・`RUST_LOG=info,areka=debug,areka::update=debug,areka_update=debug,kanade=trace`・`AREKA_APP_SMOKE_EXIT_MS`（安全弁）
- 起動: 引数なし。`Start-Process <根>\areka.exe -WorkingDirectory <根>`（標準出力を `<根>\run.log` へ）
- 操作: 開発者が右クリックメニューの「ネットワーク更新」を選び、最後に「終了」を選んだ（Claude の画面操作は開発者が断った）

## 1 回目: emo2（本物の配布サイト・https）— 失敗の経路

- 日時: 2026-09-30 07:37 JST（記録は UTC の 22:37）・HEAD `096f86f`・根 `target\nu\root`
- 更新先: ゴースト `https://ekicyou.github.io/ghost_dev/emo2/emo2/`・バルーン（kakukaku）`https://raw.githubusercontent.com/ekicyou/emo-gs/stable/ghost/emo-gs/emo-kakukaku/`・シェルは無し
- 結果:
  - ゴースト: 24 件（変更 3・手元に無い 21）の 7 件目 `emo2-kakukaku/balloonk0s.txt` で `md5 miss`。エンジンは戻して残り 0（`rolled_back=true`）。読み直しは頼まない
  - シェル: 更新先が無いので飛ばした（`update_target_skipped reason=no_homeurl kind=shell`）
  - バルーン: 定義ファイルが無い（`updates2.dau`・`updates.txt` とも 404）＝`ManifestMissing` → `OnUpdateOtherFailure(404)`
  - 総括 `OnUpdateResult` = `ghost\x01NG\x01md5 miss\x01emo2-kakukaku/balloonk0s.txt`・`balloon\x01NG\x01404`。ゴーストは台詞で失敗を伝えた（メッセージボックスは 0）
- 原因（サイト側）: サイトのテキストのファイル 7 件がすべて定義の大きさより小さい。サイトの本文の LF を CRLF に戻すと MD5 が `updates.txt` と完全に一致した（4 件で確認）＝`updates.txt` は CRLF の中身から作られ、サイトには LF で載っている。正典（ukadoc「ネットワーク更新への対応」）も改行を変えて上げると更新に失敗すると書く。SSP に入っているのは別のゴースト「emo」（更新先は emo-gs）で、emo2 のサイトは SSP でも試されていない
- 副次の確認: https の取得そのものは動いた（7 件取得・404 も受けた）。ゴーストの更新で同梱バルーンの複製 `emo2-kakukaku/` をゴーストの中へ取りに行くのは SSP も同じ（SSP の emo の中に `emo-kakukaku` が在る）

## 2 回目: あやめ（`http://ms.shillest.net/ghost/konnoyayame/`）— 要件 5.9 の欠け

- 日時: 2026-09-30 08:00 JST・HEAD `096f86f`・根 `target\nu\root2`（検体 `vendors/sample_ghost/konnoyayame.nar` を `ghost\konnoyayame` へ展開・アプリの記憶を `[last] ghost = "konnoyayame"`）
- 更新先は SHIORI のリソース `homeurl`（`descript.txt` には無い）＝定常到達の照会で写しに載った（`update_homeurl_copied`）。サイトとの差は 6 件（`yaya.dll`・辞書 3・`readme.txt`・`delete.txt`）で、サイトの本文の MD5 はすべて定義どおり
- ⑴ `changed`（6 件・MD5 すべて一致）→ 完了の台詞の後に `update_reload_requested verdict=Accepted` → 同じゴーストが起き直った。総括 `ghost OK 6`・`balloon OK 0`（StayseeBalloon は `none`）
- ⑵ `none` → 読み直さない
- ⑶ シェル（`Master`）は更新先が無いので飛ばした
- ⑷ **予想と違った**: 読み込み中の古い `yaya.dll` は `old\` へ移せたが消せず、作業場所 `.update-work\2456-0\` に印 `committed` と古い写しが残った（ここまでは設計どおり）。⑵ の差分なしの走行はエンジンが作業場所を作る前に抜けるので、棚の掃除が走らず**残りが消えなかった**
- 開発者の判断（2026-09-30）:「DLL を降ろした後に消さないといけない」＝要件 5.9 を改訂し、タスク 10 を足した。あわせて起動時に黒いコンソール窓が出る件（32bit の補助プロセスをコンソール窓付きで起こしていた既存の欠陥・閉じると補助プロセスが殺される）をタスク 11 として足した

## 3 回目: あやめ — 直した後（全項目 合格）

- 日時: 2026-09-30 19:20〜19:21 JST（記録は UTC の 10:20〜10:21）・HEAD `3d7169cf`（`tools/test-all.ps1` 全段 緑）・根 `target\nu\root3`
- **コンソール窓**: 出ない（開発者が目視で確認）
- 記録（`run.log` から抜粋・時刻は UTC）:

  ```
  10:20:57.424 OnUpdateBegin   ["はろーYAYAわーるど", "<根>\ghost\konnoyayame", "", "ghost", "manual"]
  10:20:57.654 OnUpdateReady   ["5", "readme.txt,delete.txt,ghost/master/yaya.dll,…"]
  10:20:58.200 OnUpdateComplete ["changed", "readme.txt,delete.txt,ghost/master/yaya.dll,…"]
  10:20:58.200 WARN update_leftover（読み込み中の古い yaya.dll を移した作業場所）
  10:20:58.260 OnUpdateResult  ["ghost\x01OK\x016", "balloon\x01OK\x010"]
  10:20:58.266 update_reload_requested verdict=Accepted folder=konnoyayame
  10:21:01.977 OnGhostChanged  ["紺野ややめ", "", "はろーYAYAわーるど", "<根>\ghost\konnoyayame", …]
  10:21:01.985 ghost_switch_done ghost=Some("konnoyayame") attempt=Target
  10:21:01.989 update_purge_done removed=1 dir=<根>\ghost\konnoyayame
  10:21:12.604 OnUpdateBegin   （2 回目）
  10:21:12.654 OnUpdateComplete ["none", "", "", "ghost", "manual"]
  10:21:12.696 OnUpdateResult  ["ghost\x01OK\x010", "balloon\x01OK\x010"]
  10:21:19.861 OnClose         ["user", "0", "0"]（開発者がメニューで終了）
  ```

- ⑴ 差分 6 件が入り、`OnUpdateBegin` → `OnUpdateReady` → 各ファイル（`OnUpdate.OnDownloadBegin`・MD5 照合）→ `OnUpdateComplete(changed)` → `OnUpdateResult`。完了の台詞の後に引っ込んで戻った。読み直しの間に `OnGhostChanging`・`OnClose`・`OnBoot` は 0 件で、`OnGhostChanged`（自分→自分）が届いた。補助プロセスは入れ替わり、新しい `yaya.dll`（`49b38897…`＝サイトの定義）を読み込んでいる
- ⑵ `none` → 読み直さない（`update_reload_requested` 0 件）
- ⑶ シェルは `update_target_skipped reason=no_homeurl` で飛ばし、総括にはゴーストとバルーンだけ
- ⑷ ⑴ の確定で古い `yaya.dll` の写しが印つきで残り（`update_leftover` 1 件）、**読み直しの切替が終わった 4 ms 後に消えた**（`update_purge_done removed=1`・`update_purge_held` 0 件）。終了後の `.update-work` は無い
- 終了: 開発者がメニューで終了（`app_exit origin=KanadeStopped(Quit)`）。アプリの記憶の起動中の印は空（`running = ""`）
- メッセージボックス: 0

## 既知の制限・観察（判定は変えない）

- https で最後まで差し替える走行は、emo2 の配布サイトの改行の不備で未確認（https の取得そのものは 1 回目で動いた）。サイトを直せば同じ手順で確かめられる
- emo2 のバルーン（kakukaku）の `homeurl` は 404（emo-gs ではバルーンはゴーストの更新の一部として配られ、単独の更新先が無い）
- 読み直しの直後に `chain_finalize: 初期配置の確定が続けて見送られている（deferrals=600）` の `warn!` が 1 件出た（配置の既存の見張り・本仕様の範囲外）
- i686 の `shiori_proxy::tests::testdll_drop_invokes_courtesy_unload` が検証中に 1 度だけ落ちた（既存の不安定・補助プロセスの crate は本仕様の変更に依存しない）
