# 実機確認の記録（areka-P0-ghost-install・要件 11.11・11.12）

design の Testing Strategy「実機」の 4 項目。項目 3 はタスク 8.2（`switch_to` に呼び出しを入れた 8.1 の直後に先に通す）、残りの 3 項目はタスク 11.2 で記録する。

## 項目 3: 起動中のゴーストの `.nar` をメニューから入れる（タスク 8.2）

- 日時: 2026-09-29 09:54〜09:57 JST（記録の時刻は UTC で 00:54〜00:57）
- HEAD: `bf4f247a`（タスク 8.1 のコミット・ブランチ `claude/areka-p0-ghost-install-bd7d9c`）
- 環境: Windows 11 Pro 10.0.26200
- 実行体: `pwsh -NoProfile -File tools/package-alpha.ps1` で HEAD から組んだ配布物 `areka-alpha-x64-20260929-bf4f247.zip`（全段 緑・未コミットの変更 0 件）を `C:\areka-su` へ展開した。本番の SHIORI は同梱の emo2（`pasta.dll`・32bit の補助プロセス `shiori-host32-helper.exe` 経由）
- 検体: `vendors/sample_ghost/emo2.nar` を `C:\areka-nar\emo2.nar` へ複製（絶対パスの短い場所）。起動中のゴースト `ghost\emo2` と同じゴースト
- 環境変数: `RUST_LOG=info,areka=debug,kanade=trace`・`AREKA_APP_SMOKE_EXIT_MS=300000`（安全弁。働く前に手で終了した）・`AREKA_NO_ALERT` は外した（選ぶ画面を出すため）
- 起動: `Start-Process C:\areka-su\areka.exe -WorkingDirectory C:\areka-su`（標準出力を `C:\areka-su\run.log` へ）
- 操作: 開発者の指示で Claude が画面を操作した。キャラクターを右クリック → メニューの「インストール…」 → 選ぶ画面のファイル名に `C:\areka-nar\emo2.nar` を入れて開く → 戻った後に右クリック → 「終了」

### 結果: 付け替えられた（設計へ戻る判断は要らない）

- 見た様子:
  - メニューに「ゴースト」「インストール…」「説明書」「終了」が出た
  - 選ぶ画面（`GetOpenFileNameW`）は **前面に出た**。フィルタは「書庫 (*.nar;*.zip)」
  - 開いてから約 2 秒でキャラクター 2 体が引っ込み、約 6 秒の時点で 2 体とも戻って話し始めた。別れの台詞は出なかった
- 記録（`run.log` から抜粋・時刻は UTC）:

  ```
  00:56:08.271853 install_order_queued origin=Menu count=1
  00:56:08.273276 install_begin archive=C:\areka-nar\emo2.nar origin=Menu
  00:56:08.280885 shiori_request method=GET id=OnInstallBegin
  00:56:08.338051 install_accept verdict="accepted" accept=None target_ghost=None
  00:56:08.347093 ghost_switch_requested from=Some("えも？？") to=えも？？ raise_event=false origin="automatic"
  00:56:08.347360 install_overwrite_requested folder=emo2
  00:56:08.415347 ghost_switch_down_ms ms=2
  00:56:08.421966 windows_closed_for_restart closed=4
  00:56:08.699635 install_overwrite_done ms=277 ok=true folder=emo2
  00:56:09.004910 ghost_switch_booted ghost=Some("emo2") attempt=Target
  00:56:10.833700 ghost_switch_done ghost=Some("emo2") attempt=Target
  00:56:10.836128 install_done kind=Ghost places=["C:\\areka-su\\ghost\\emo2", "C:\\areka-su\\balloon\\emo2-kakukaku"]
  00:56:10.850030 install_names_updated ghost="えも？？" object="えも？？" published=true
  00:56:10.861344 shiori_request method=GET id=OnInstallCompleteEx references=["ghost\u{1}balloon", "えも？？\u{1}kakukaku for emo-gs", "C:\\areka-su\\ghost\\emo2\u{1}C:\\areka-su\\balloon\\emo2-kakukaku"]
  00:56:10.890232 shiori_request method=GET id=OnInstallComplete references=["ghost", "えも？？", "kakukaku for emo-gs"]
  00:56:10.894327 install_order_done ends=[Installed([ghost えも？？, balloon kakukaku for emo-gs])]
  00:56:52.167544 app_exit origin=KanadeStopped(Quit) closed=4
  00:56:52.214081 session_mark_cleared
  ```

- 判定:
  - 降ろした後（`ghost_switch_down_ms` の後）に宛先 `ghost\emo2` と同梱バルーン `balloon\emo2-kakukaku` へ展開でき、`install_overwrite_done ok=true`・**所要 277 ms**
  - `OnGhostChanging` と `OnClose` の送出は 0 件（`run.log` の `id=OnClose`・`id=OnGhostChanging` の件数が 0）
  - 同じゴースト（emo2）が起き直して定常に入り、その後に締めの知らせ `OnInstallCompleteEx` → `OnInstallComplete` が出た
  - 終了はきれいに終わった（終了コード 0・起動中の印を消した）。走行の後、`C:\areka-su` 配下のプロセスは 0 件
- 気付いたこと（インストールの手続きの外）:
  - 起き直しの後に `WARN chain_finalize: 初期配置の確定が続けて見送られている deferrals=600 scope=Some(1) reason=scope 1: 実表示寸が未確定（初回表示が未成立）` が 1 件出た（00:56:14）。画面では 2 体とも表示されていた。配置の側の記録で、この spec の境界の外

## 項目 1・2・4（タスク 11.2）

未記録。
