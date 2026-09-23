# Implementation Plan

> 正本は design.md（Option C）。コードは「何の定義か」で指し、行番号では指さない。
> 足すテストは判断分岐だけ（要件 8.4）。配線（マウント→SHIORI→sink・窓の配置・終了の指示）は smoke 3 方向が実プロセスで証明する。

## Tasks

- [x] 1. 永続に鍵の族 `[last]` を足す
- [x] 1.1 永続のテストを兄弟ファイルへ移すだけの下ごしらえ
  - `persist/mod.rs` 末尾の `mod tests` の本体を `persist/persist_tests.rs` へそのまま移し、`#[cfg(test)] #[path = "persist_tests.rs"] mod tests;` で繋ぐ
  - テストの本文・本数・名前を 1 字も変えない（挙動 0 変更・独立にレビューできる 1 コミット）
  - 完了の姿: `cargo test -p areka-sylphya` が移設前と同じ本数で緑、`persist/mod.rs` が 1,000 行の目安の内側
  - _Requirements: 3.6, 7.3, 7.6_
- [x] 1.2 鍵 3 つ（`areka.last.ghost`／`areka.last.balloon`／`areka.last.shell`）と TOML の表 `[last]` を足す
  - `PersistKey` に `LastGhost`／`LastBalloon`／`LastShell` を足し、正準名の写像・`apply_entry`・`doc_to_entries`・`all_families()` を追随させる
  - `FormatDoc` に 3 欄を足し、`[last]` の直列化と読取・`is_all_absent` へ含める（値が無ければ表を書かない）。`load_scope` の返り順の末尾は ghost → balloon → shell
  - `persist_tests.rs` に「3 鍵を書いて読み戻す往復」「無い鍵は返らない」「`[last]` だけ保存しても `[window]` 等が残る」「`parse_dotted` との往復」を足す。既存 4 族のテストは触らない
  - 完了の姿: 追加テストと既存の全テストが緑、`actor.rs`・`prop_sink.rs`・`placement/persist.rs` に差分が無い
  - _Requirements: 3.1, 3.6, 3.7, 9.4_

- [x] 2. 根の下の目録と根の決め方
- [x] 2.1 (P) 目録 `catalog` の本体（根の値型・3 種の列挙・素性・同梱バルーン・ゴーストの判定）
  - `areka-ghost` に `BasewareRoot`（根と `ghost/`・`balloon/` のパスを組むだけ・実在検査はしない）と `Identity`（7 項目）・`GhostEntry`／`ShellEntry`／`BalloonEntry` を置き、`lib.rs` から公開する
  - ゴースト・シェル・バルーンの列挙を直下 1 段の走査で返す: ゴーストは `ghost/master/descript.txt` の有無、シェルは `descript.txt` の有無と `menu,hidden` の除外、バルーンは `type` が無いか `balloon` のものだけ（他の値は `warn!`＋除外）
  - descript は既存の charset 復号と `parse_kv` で読み、鍵は ASCII 小文字化、`menu`／`type` の値は trim＋小文字化で比べる。素性の値は無加工。説明書の所在は `readme` 鍵（無ければ `readme.txt`）が最上位に実在するときだけ、`thumbnail.png` は有無だけ
  - 縮退: 格納フォルダが無ければ 0 件・descript が読めなければ `warn!`＋除外・非 UTF-8 のフォルダ名は `warn!`＋除外。並びはフォルダ名のバイト順で `recommended.*` は使わない
  - 同梱バルーン名は `install.txt` の `balloon.directory` の 1 鍵だけを読み（読めなければ `warn!`＋無し）、「そのフォルダはゴーストか」は `ghost/master/descript.txt` の実在だけで判定する
  - `fn resolve` を呼ばず・変えず、`areka-sylphya`・`areka-nar` に依存せず、外部依存を足さない
  - 完了の姿: `cargo build -p areka-ghost` が通り、根を渡すと 3 種の目録がバイト順で返る
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10, 5.3, 7.3, 7.7_
  - _Boundary: areka-ghost catalog_
- [x] 2.2 目録の決定論テストと一時の根の組み立てヘルパ
  - 一時フォルダに根を組むヘルパ（ゴースト 2 体・`menu,hidden` を含むシェル 2 つ・`type,balloon` のバルーン・`type,plugin` のフォルダ・`descript.txt` の無いフォルダ・フォルダ名 `StayseeBalloon` の偽バルーン・`Shift_JIS` 宣言の `name`／`craftmanw`）を `temp-path-kit` の下に置く
  - 検体の根（`SampleRoot::acquire("emo2")`＝ゴースト 1・バルーン 1・同梱 `emo2-kakukaku`）と一時の根の両方で、件数・採否・7 項目・バイト順・`Shift_JIS` の復号を突き合わせる
  - 格納フォルダ不在 → 0 件、読めない descript → `warn!` 1 件＋除外（`log-capture-kit`）、ゴーストの判定の真偽を踏む
  - 完了の姿: `cargo test -p areka-ghost catalog` が緑で、要件 8.1 の各構成が 1 本以上のテストに現れる
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 5.3, 8.1, 8.4_
- [x] 2.3 (P) 根の決め方（`AREKA_ROOT` → 実行ファイルの隣・実在検査）
  - `boot_config` に、env の値と実行ファイルの場所を注入して根を決める純粋な口と、env と `current_exe()` を読むだけの薄い口を足す。根の出所（環境変数／exe の隣）と、決まらない理由（exe の場所が取れない／フォルダでない）を型で返す
  - `AREKA_ROOT` があれば exe を見ない。相対の値はカレント基準で `std::path::absolute` により絶対化してから検査する（`canonicalize` は使わない）。exe が取れず env も無ければ `"."` へ倒さず失敗
  - アプリの記憶の保存先の既定（`"."` フォールバックを含む）は変えない
  - `main_config_input_tests.rs` に 6 通り（env あり実在／env あり不在／env 無し exe 実在／env 無し exe 不在／両方無し／相対 env が絶対で返る）を足す。プロセスの env は書かない
  - 完了の姿: 6 通りのテストが緑。この段では既存の既定パス 2 関数と `resolve_config_inputs` はまだ残す（撤去は 5.1）
  - _Requirements: 1.2, 1.3, 1.4, 1.7, 8.2, 9.1_
  - _Boundary: areka bin boot_config_

- [ ] 3. 無いときの告知 `alert`
  - 場面を 1 つの型で数える: 根なし（根が決まらない理由を載せる）／ゴーストなし（格納フォルダと、argv 起動なら渡されたパス）／バルーンなし（格納フォルダ）／起動窓を開けない（失敗の内容）
  - 題名と本文を純粋に組む（本文は「何が無いか」「置くべき場所の絶対パス」「置くものの形」の 3 行・冒頭は「根が決まりません」「ゴーストが見つかりません」「バルーンが見つかりません」「起動窓を開けません」）
  - 告知の口は `error!(event = "alert")` を必ず 1 件残し、抑止でなければ `MessageBoxW`（`hwnd` 無し・`MB_OK | MB_ICONERROR`）を出す。`unsafe` はこの 1 か所に閉じ、終了コードは呼び手に任せる
  - 抑止は `AREKA_NO_ALERT`: 未設定・空・空白のみ・trim 後 `"0"` は出す、それ以外は抑える。値から判断する純粋な口と env を読む薄い口に分ける
  - `alert_tests.rs` で 4 場面を抑止ありで呼び、`error!` が 1 件ずつ・本文に格納フォルダの絶対パスが入ることを `log-capture-kit` で確かめる。抑止の判断は 5 通り（`None`／`""`／`"0"`／`"1"`／`" 1 "`）
  - `main.rs` に `mod alert;` を宣言だけ足す（呼び出しの結線は 5.1・5.2）
  - 完了の姿: `alert_tests.rs` が緑で、新規の外部依存が 0（既存 feature の `MessageBoxW` だけ）
  - _Requirements: 6.1, 6.2, 6.5, 6.7, 7.7, 9.2_
  - _Depends: 2.3_

- [ ] 4. 起動解決の判断と記憶の読み書き `boot_resolve`
- [ ] 4.1 ゴースト 6 分岐・バルーン 7 分岐の純粋な判断
  - 既定の定数 `DEFAULT_GHOST_FOLDER = "emo2"`・`DEFAULT_BALLOON_FOLDER = "StayseeBalloon"` をこのファイルだけに置き、ゴーストは段 4・バルーンは段 5 だけが参照する
  - ゴースト: argv → 記憶（列挙に在れば）→ 唯一 → 既定 → 無作為 → 0 体。バルーン: argv → 記憶 → 同梱 → 唯一 → 既定 → 無作為 → 0。argv があれば記憶・列挙を見ない
  - 記憶・同梱の指す先が列挙に無ければ `warn!`（`last_ghost_not_found`／`last_balloon_not_found`／`companion_balloon_not_found`）して次の段へ、無作為は `info!`（`ghost_picked_randomly`／`balloon_picked_randomly`）を残す
  - 判断は I/O・時計・乱数源を持たず、無作為は「候補数 → 添字」の関数を注入する。本番の添字は `std::hash::RandomState` から作る（`rand` は入れない）。0 体／0 は格納フォルダを載せた型で返す
  - `boot_resolve_tests.rs` で 13 分岐＋記憶の指す先が無い 2 通り＋同梱の指す先が無い 1 通りを固定添字で踏む。既定の分岐は `listed` に名前を並べるだけ（検体の登記に依存しない）
  - `main.rs` に `mod boot_resolve;` を宣言だけ足す（呼び出しの結線は 5.1・5.3）
  - 完了の姿: 分岐テストが全部緑で、既定の定数への参照がそれぞれ 1 段だけ
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.9, 4.11, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 8.3, 8.4, 9.3, 9.4_
  - _Depends: 2.1_
- [ ] 4.2 起動前の記憶の直読みと、起動成功時の記憶の書き込み
  - 起動前（アクター不在）に `load_scope` を `FsPersistIo` で直接呼び、App スコープの `areka.last.ghost` と、起動するゴーストの Ghost スコープ（boot と同じ `profile_areka_root(<ゴースト>/ghost/master)`）の `areka.last.balloon` を読む口を置く。読めなければ無し
  - 起動成功時に書く内容を 1 つの値にまとめ、App へ `LastGhost`（argv 以外のとき）、Ghost へ `LastBalloon`（argv 以外のとき）＋`LastShell`（常に）を `persist_put` で投函する。argv の側は書かず `info!(event = "last_used_skipped_argv")`、書いたら `last_used_recorded`
  - 追加の `barrier()` は置かない（反映は `shutdown` の手順 10 に任せる）。`areka.last.shell` は起動時の解決に使わない
  - テストは `spawn_sylphya`＋`FakePersistIo` で、argv／非 argv それぞれの書き分けを `barrier` 後の `load_scope` で読み戻す（`main_persist_wiring_seam_tests.rs` と同じ形）
  - 完了の姿: 読み戻しテストが緑で、「argv のゴースト → App に書かれない・Ghost の shell は書かれる」が固定される
  - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.8, 4.2, 5.2, 9.4, 9.5_
  - _Depends: 1.2_

- [ ] 5. `main` への結線とダミー窓の退役
- [ ] 5.1 起動前の解決の結線と既定パスの撤去
  - `main` の構成入力の位置（`WinApp` 構築の前）を「根 → 列挙 → ゴースト解決 → バルーン解決 → `ConfigInputs`」へ置き換える。argv がある側は列挙も記憶も読まず、argv のゴーストは「ゴーストか」の 1 検査だけ
  - 根・ゴースト・バルーンが決まらなければ告知の口を呼んで `Err(E_FAIL)`（終了コード 1）を返す。`root_resolved`・`ghost_resolved`・`balloon_resolved`（経路と場所）の `info!` を残す
  - 「根が無くても `warn!` で続ける」存在確認ループ・既定パス 2 関数・`resolve_config_inputs` を消し、それを直接見る `main_config_input_tests.rs` の既定パス 4 本を除く。`is_benign_boot_error` の doc と、`default_ghost_root()` を根拠に挙げる `main.rs`・`emo2_boot/mod.rs` のコメントを「解決後の消失に限られる」へ書き換える（`emo2_boot` はコメントだけでコードは触らない）
  - 決まった `GhostDecision`／`BalloonDecision`（経路とフォルダ）を `main` の中で boot の分岐まで持ち越す（5.3 が記憶の書き込みに使う）
  - 常設 smoke の `run_smoke` に `AREKA_NO_ALERT=1` をこの段で足す（引数なし方向が告知のモーダルで 60 秒止まらず、非 0 で即座に落ちるようにする）。3 方向への作り直しは 6.1
  - `ConfigInputs` の形と 3 消費者（`open_startup_window`・`wire_emo2_boot`・`ghost_boot_options`）は変えない
  - 完了の姿: `cargo test -p areka --bins` が緑で、`default_ghost_root`／`default_balloon_root`／`resolve_config_inputs` の出現が `crates/` で 0 件（常設 smoke の旧「引数なし」方向は 6.1 まで赤＝非 0 で即座に落ちる。5.1〜5.3 の間は `--workspace` と smoke を完了判定に使わない）
  - _Requirements: 1.4, 1.5, 4.7, 4.8, 4.10, 5.8, 5.11, 6.3, 6.6, 7.1, 7.4_
  - _Depends: 2.3, 3, 4.1, 4.2_
- [ ] 5.2 起動窓の失敗を終了へ写し、ダミー窓一式を退役させる
  - `open_startup_window` は準備の失敗をそのまま返し、`main` は「起動窓を開けない」の告知＋`Err(E_FAIL)` にする。smoke の自動終了の投入は成功時だけに移す。「作者基準 DPI を既定へ縮退」の分岐は消える
  - `spawn_dummy_window`・`on_dummy_pressed`・`DummyWindowMarker`・`is_benign_placement_error` と不要になった `use` を消し、`main_seam_tests.rs` を削除、`main_startup_window_tests.rs` のダミー窓 5 本を除く（smoke 自動終了の 6 本は残す）
  - `app_exit` から `ExitOrigin::DummyWindow`・`on_dummy_os_close` を消し、窓の一括消去をゴースト窓だけへ縮める。`app_exit_tests.rs` のダミー窓の 2 か所をゴースト窓へ差し替える。`quit_app` の形・`menu/`・`runtime.rs`・kanade の握手は触らない
  - ダミー窓とフォールバックに触れる doc（`main.rs` 冒頭・smoke 自動終了のログ文言・placement の doc コメント）を言い換える（コードは placement では触らない）
  - 完了の姿: `cargo test -p areka --bins` が緑で、`DummyWindow`／`spawn_dummy_window`／`is_benign_placement_error` の出現が `crates/` で 0 件、`main.rs`・`boot_config.rs` が 1,000 行の目安の内側
  - _Requirements: 6.4, 6.6, 7.2, 7.4, 7.6, 9.2_
- [ ] 5.3 boot 成功の直後に記憶を書く
  - wired／fallback の両アームで、boot が `Ok` を返した直後（`insert_persist_wiring` と同じ場所）に起動成功時の記憶の書き込みを 1 回呼ぶ。シェルのフォルダ名は `mount().shell.dir` の末尾を写す
  - 完了の姿: `cargo build -p areka` が通り、書き込みの呼び出しが boot `Ok` の直後の 1 か所だけ
  - _Requirements: 3.2, 3.3, 3.4, 3.5, 9.5_

- [ ] 6. 実プロセスの検査
- [ ] 6.1 常設 smoke テストを 3 方向へ更新する
  - 起動の口を「引数と env の組」を受ける形へ広げ、全方向で `AREKA_APP_SMOKE_EXIT_MS=500`・`AREKA_NO_ALERT=1`。60 秒の見張りと終了コードの判定は残す
  - ① 本物方向（argv 絶対パス・目印は今までどおり・終了コード 0）、② 根方向（argv なし・`AREKA_ROOT` に検体の根・`AREKA_PROFILE_DIR` は一時フォルダ・目印は `route=Only`／`route=Companion`・終了コード 0）、③ 0 体方向（空の一時の根・終了コード非 0・「ゴーストが見つかりません」を含み本物の窓の目印を含まない）
  - モニタ 0 台では ①② が「起動窓を開けない」の `error!` と非 0 終了を受理する（旧フォールバック方向を置き換える）
  - `tests/emo2_real_run.rs` の子プロセスにも `AREKA_NO_ALERT=1` を渡す（見え方は変えない）
  - 完了の姿: `cargo test -p areka --test smoke_boot_loop_exit` が 3 方向とも緑
  - _Requirements: 1.6, 7.1, 7.5, 9.2_
- [ ] 6.2 (P) 台帳「配布物の素性」の判定を更新する
  - `doc/ukadoc-coverage/ledger/assets.toml` で読むようになった欄（バルーン `name`／`craftman`／`craftmanw`／`id`／`type`／`readme`、シェル `craftman`／`craftmanw`／`id`／`readme`／`menu,hidden`、ゴースト `name`／`craftman`／`craftmanw`／`id`）を `implemented`・owner 本仕様へ。既に実装済みのゴースト `readme`・シェル `name` は note に列挙の読み手を足すだけ。読まない欄は触らない
  - 完了の姿: `cargo test -p ukadoc-survey` が緑
  - _Requirements: 2.4, 2.9_
  - _Boundary: ukadoc-coverage ledger_
  - _Depends: 2.1_
- [ ] 6.3 全体の回帰と実機確認
  - ワークスペースのテスト（既存の永続 4 族・`fn resolve` の 4 呼び手のテストを含む）が緑であること、新規の外部依存が 0 であること（`Cargo.lock` に新しい crate が無い）を確かめる
  - 実機 4 点: ① `AREKA_ROOT` に検体の根・argv なしで同梱バルーンの会話が出る（`balloon_resolved route=Companion`）② 終了後にアプリとゴーストの `sylphya.toml` の `[last]` を確認し、再起動で `route=Memory` ③ 空の根で告知が出て閉じると終了コード非 0（`AREKA_NO_ALERT` は設定しない）④ 有界終了後にプロセスが残らない。止めるのは自分が起こしたと確認できたプロセスだけ
  - 裁定 1〜5 を覆す必要が見えたら実装を止め、開発者へ議題として上げる
  - 完了の姿: 回帰が緑で、実機 4 点の結果（ログ抜粋と終了コード）が記録に残る
  - _Requirements: 7.3, 7.7, 8.5, 8.6, 9.6_

## Implementation Notes

- 1.2: `format.rs` の既存テスト `round_trip_preserves_all_string_values` は `FormatDoc` を全欄で書くため、欄の追加でコンパイルのための追随が要った（既存 4 族の値と比較は不変）。構造体リテラルで全欄を並べるテストは族を足すたびに同じ追随が要る。
- 2.1→2.2: 空の値（trim 後に空）はどの鍵も「無し」に揃えた（`catalog.rs` の `lowercased` の 1 か所）。`type,` はバルーンとして残り、`menu,` は隠さない。design R5 に追記済み。
- 2.3: 空の `AREKA_ROOT` は「設定あり」（`AREKA_PROFILE_DIR` と同じ）→ `NotADirectory { dir: "", source: EnvVar }`。`dir` が絶対でない唯一の例外なので、告知（3）は空のとき「環境変数 AREKA_ROOT が空です」の類いの文言にする。`RootError` 等の item 単位 `#[allow(dead_code)]` は 5.1 で外す。
