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

## 項目 2: `\![change,ghost,lastinstalled]` で切り替わる（タスク 11.2）

- 日時: 2026-09-29 13:39〜13:40 JST（記録の時刻は UTC で 04:39〜04:40）
- HEAD: `2745cb4f`（タスク 11.1 のコミット）
- 実行体: `tools/package-alpha.ps1` で HEAD から組んだ `areka-alpha-x64-20260929-2745cb4.zip`（全段 緑）を新しい根 `C:\areka-su3` へ展開した
- 検体（画面を操作せずに台本だけで一周させる形）:
  - `ghost\rpost_script\`: `vendors/sample_ghost/R_POST_and_KOMAINU.nar` の中身の写し（SHIORI は里々・32bit の補助プロセス経由）。差し替えは `ghost\master\dic02_Event.txt` の 3 か所と `descript.txt` の `name`（`台本役ポスト`）だけ（Shift_JIS のまま）
    - `＊OnFirstBoot`・`＊OnBoot` の本文: `：台本で入れます。\_w[1000]\![execute,install,path,C:/areka-nar/R_POST_and_KOMAINU.nar]`
    - `＊OnInstallComplete` の本文: `：入れたので切り替えます。\_w[1500]\![change,ghost,lastinstalled]`
  - `C:\areka-nar\R_POST_and_KOMAINU.nar`: `vendors/sample_ghost/` の原本の複製（宛先 `ghost\R_POST_and_KOMAINU`＝根に無い）
  - アプリの記憶 `profile\areka\sylphya.toml` に `[last] ghost = "rpost_script"` だけを置き、検体から起動した
- 環境変数: `NO_COLOR=1`・`AREKA_NO_ALERT=1`・`RUST_LOG=info,areka=debug,kanade=trace,areka_sakura=debug`・`AREKA_APP_SMOKE_EXIT_MS=45000`（自分で終わる）
- 操作: なし（起動して 45 秒で自動終了）

### 結果: 切り替わった

```
04:39:20.447841 shiori_request method=GET id=OnFirstBoot references=["0"]
04:39:21.890299 install_order_queued origin=Script count=1
04:39:21.890808 install_begin archive=C:/areka-nar/R_POST_and_KOMAINU.nar origin=Script
04:39:21.898854 shiori_request method=GET id=OnInstallBegin
04:39:22.924157 install_accept verdict="accepted" accept=None target_ghost=None
04:39:22.947594 install_done kind=Ghost places=["C:\\areka-su3\\ghost\\R_POST_and_KOMAINU"]
04:39:22.951589 install_names_updated ghost="Ｒポストと狛犬" object="Ｒポストと狛犬" published=true
04:39:22.957617 shiori_request method=GET id=OnInstallCompleteEx references=["ghost", "Ｒポストと狛犬", "C:\\areka-su3\\ghost\\R_POST_and_KOMAINU"]
04:39:22.965670 shiori_request method=GET id=OnInstallComplete references=["ghost", "Ｒポストと狛犬", ""]
04:39:22.967954 install_order_done ends=[Installed([ghost Ｒポストと狛犬])]
04:39:25.142547 ghost_switch_resolved name=lastinstalled to=R_POST_and_KOMAINU position=None
04:39:25.142885 ghost_switch_requested from=Some("台本役ポスト") to=Ｒポストと狛犬 raise_event=false origin="automatic"
04:39:25.307411 ghost_switch_booted ghost=Some("R_POST_and_KOMAINU") attempt=Target
04:39:25.736528 ghost_switch_done ghost=Some("R_POST_and_KOMAINU") attempt=Target
04:40:04.656635 app_exit origin=Smoke closed=4
04:40:04.692761 session_mark_cleared
```

- 判定:
  - 台本の入口（要件 1.5）から入り、`ghost\R_POST_and_KOMAINU` へ展開された（`descript.txt` が在る）
  - 入れた後、ゴーストが台本で頼むまでの約 2.2 秒は切り替わらず、今のゴースト（台本役ポスト）のまま（`install_done` 04:39:22.9 → `ghost_switch_requested` 04:39:25.1 は `OnInstallComplete` の台詞の `\_w[1500]` の後）
  - `\![change,ghost,lastinstalled]` が入れたゴーストへ解け（`ghost_switch_resolved name=lastinstalled to=R_POST_and_KOMAINU`）、起こして定常に入った（`ghost_switch_done`）。アプリの記憶の `[last] ghost` も `R_POST_and_KOMAINU` になった
  - `OnInstallComplete`（旧仕様）の Reference2 は空（入れた物が 1 つなので 2 件目の名前が無い）
  - ERROR 0 件。WARN はバルーンの面の縮退（`balloons2.png`／`balloons3.png` が本体側の系列へ）だけで、インストールと関わらない
  - 終了コード 0・起動中の印を消した。走行の後、`C:\areka-su*` 配下のプロセスは 0 件

## 項目 1・4（タスク 11.2）

未記録（メニューと利用条件の画面を手で操作する）。
