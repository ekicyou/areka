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

## 項目 1・4: メニューから入れる・利用条件の画面（タスク 11.2・開発者の手）

- 日時: 2026-09-29 14:08〜14:14 JST（記録の時刻は UTC で 05:08〜05:14）
- HEAD: `0e39694d`（実行体は `2745cb4f` から組んだ `areka-alpha-x64-20260929-2745cb4.zip`。`2745cb4f` 以後の変更は `signoff.md` だけ）
- 根: `C:\areka-su2`（新しく展開した根・ゴーストは emo2 だけ・アプリの記憶は前の走行の `[last] ghost = "emo2"`）
- 検体（`C:\areka-nar\`）:
  - `R_POST_and_KOMAINU.nar`: `vendors/sample_ghost/` の原本の複製（宛先 `ghost\R_POST_and_KOMAINU`・SHIORI は里々）
  - `konnoyayame-terms.nar`: `vendors/sample_ghost/konnoyayame.nar`（宛先 `ghost\konnoyayame`・SHIORI は YAYA）の中身に、最上位の `terms.txt`（UTF-8）を足した写し
- 起動: 開発者が起動した。案内したコマンドは `$env:…` が bash に先に展開されて環境変数が効かず、記録は既定の `info`（色つき）になった＝下の抜粋は色の符号を外したもの。`install_*`・`menu_*`・`ghost_switch_*` は info なので判定に足りる（SHIORI への送出の trace は無い）
- 操作: 開発者が画面を操作した

### 項目 1: 結果＝合格

```
05:09:04 menu_shown scope=0 items=4
05:09:06 menu_selected frame=Install
05:09:16.216857 install_accept archive=C:\areka-nar\R_POST_and_KOMAINU.nar verdict="accepted" accept=None target_ghost=None
05:09:16.237403 install_done archive=C:\areka-nar\R_POST_and_KOMAINU.nar kind=Ghost places=["C:\\areka-su2\\ghost\\R_POST_and_KOMAINU"]
05:09:16 install_event id="OnInstallCompleteEx" → id="OnInstallComplete"
05:09:22 menu_shown scope=0 items=5
05:09:24.904233 menu_selected frame=Ghost
05:09:24.908223 ghost_switch_requested from=Some("えも？？") to=Ｒポストと狛犬 … origin="manual"
05:09:29.777810 ghost_switch_done ghost=Some("R_POST_and_KOMAINU") attempt=Target
```

- 入れた後もえも？？のまま（`install_done` 05:09:16 から利用者がメニューで選ぶ 05:09:24 まで切替の要求は 0 件）
- 次に開いたメニューの項目が 4 → 5 に増え（「ゴースト」枠に入れたゴーストが出た）、選ぶと切り替わった

### 項目 4: 結果＝「はい」は合格・画面は前面に出た・「いいえ」は手では押していない

- 1 回目（05:10:22）: 検体の `terms.txt` に `charset` の行も BOM も無かったため、正典（「1行目にcharset,UTF-8と書いておくか、BOMを含んでおけばShift JIS以外も使える」）と要件 4.3 のとおり Shift_JIS として読まれ、**本文が化けた**＝検体の誤り（areka の欠陥ではない）。この画面は「はい」で閉じられた（`install_terms answer="accept"` 05:11:21 → `install_done`）
- 検体を 1 行目 `charset,UTF-8` つきに作り直した
- 2 回目（05:12:06）: 本文は日本語で読めた（`charset` の行は本文に出ない）。**画面は前面に出た**。「はい」で受諾:

  ```
  05:12:06.662274 install_accept archive=C:\areka-nar\konnoyayame-terms.nar verdict="accepted"
  05:12:06.662587 ask title="利用条件 - はろーYAYAワールド" suppressed=false
  05:12:16.468231 install_terms file="terms.txt" answer="accept"
  05:12:16 install_event id="OnGhostTermsAccept"
  05:12:16.627285 install_done kind=Ghost places=["C:\\areka-su2\\ghost\\konnoyayame"]
  05:12:16 install_event id="OnInstallCompleteEx" → id="OnInstallComplete"
  05:12:27.753081 ghost_switch_requested … origin="manual"（「ゴースト」枠から選んだ）
  05:12:28.473938 ghost_switch_done ghost=Some("konnoyayame") attempt=Target
  ```

- 「いいえ」の道は、画面を出さない形で別に通した（下）。手で「いいえ」を押す操作は通していない。「いいえ」と抑止の違いは、画面の戻り値を写す 1 行（`alert.rs` の `ask_yes_no` の `IDNO => YesNo::No`）と記録の水準・`answer` の語だけで、その後の道（`OnGhostTermsDecline` を送って `Declined` を返す）は `procedure.rs` の利用条件の関数の中で同じ

#### 拒否の道（画面を出さない形・2026-09-29 14:14 JST）

- 根 `C:\areka-su3` の `rpost_script` の起動の台詞の書庫を `C:/areka-nar/konnoyayame-terms.nar` に差し替え、`AREKA_NO_ALERT=1`（抑止＝拒否として扱う）・`RUST_LOG=info,areka=debug,kanade=trace`・`AREKA_APP_SMOKE_EXIT_MS=30000` で起動した

  ```
  05:14:26.256368 install_order_queued origin=Script count=1
  05:14:26.281345 install_accept archive=C:/areka-nar/konnoyayame-terms.nar verdict="accepted"
  05:14:26.281531 ask title="利用条件 - はろーYAYAワールド" suppressed=true
  05:14:26.281815 WARN install_terms file="terms.txt" answer="suppressed"
  05:14:26.289724 shiori_request method=GET id=OnGhostTermsDecline references=[]
  05:14:26.290650 install_order_done ends=[Declined]
  05:14:54.402572 session_mark_cleared
  ```

- 拒否の後は `OnGhostTermsDecline` だけで、`OnInstallFailure`・`OnInstallRefuse`・`OnInstallComplete*` は 0 件。`ghost\konnoyayame` は作られていない（`Test-Path` が False）

### 共通

- ERROR 0 件（`C:\areka-su2\run.log`）。WARN 15 件はバルーンの面の縮退・折り返し基準・全透明の要素で、インストールと関わらない
- 終了はメニューの「終了」（`app_exit origin=KanadeStopped(Quit)`・`session_mark_cleared`）
