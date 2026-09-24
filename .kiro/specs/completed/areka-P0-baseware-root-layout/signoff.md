# 実機確認と全体の回帰（タスク 6.3）

- 実施日: 2026-09-24
- ブランチ: `claude/areka-p0-baseware-root-layout-6ae6de`（HEAD `ec53a3fb`・分岐元 `92f5f448`）
- 対象要件: 7.3・7.7・8.5・8.6・9.6
- 生ログは作業外の一時フォルダに置いた（リポジトリには入れない）

## 1. 回帰（要件 7.3・7.7）

### 外部依存（要件 7.7）

`git diff main...HEAD -- Cargo.lock Cargo.toml '**/Cargo.toml'` の出力は **0 行**。新しい crate は 0 件。

### ワークスペースのテスト

```
cargo build -p shiori-host32-testdll -p shiori-host32-helper --target i686-pc-windows-msvc   # 成功
cargo test --workspace --no-fail-fast -j 4                                                     # exit 101
```

- 合計: 成功 8,294 件・**失敗 3 件**（失敗した対象は `-p log-capture-kit --test temp_path_guard_test` の 1 つだけ）
- 永続化の既存 4 族（`areka-sylphya`）と `fn resolve` の 4 呼び手のテストは緑。smoke 3 方向（`smoke_boot_loop_exit.rs`）も緑。
- **赤 3 件の原因は本ブランチ**: 本ブランチで新しく足した 2 つのテストファイルが、一時パスの入口 `std::env::temp_dir()` を共通窓口（`temp-path-kit` の `TempPath`）を通さずに呼んでいる。どちらのファイルも `main` には無い。
  - `crates/areka/src/alert_tests.rs` の `fn abs`（`std::env::temp_dir().join("areka-alert-tests")`）
  - `crates/areka/src/boot_resolve_tests.rs` の `fn root`（`std::env::temp_dir().join("areka-boot-resolve-tests")`）
  - 失敗したテスト: `no_temp_dir_entry_point_lives_outside_the_gateway_and_the_allow_table`・`the_measurement_is_not_vacuous_and_matches_the_allow_table`・`dropping_a_known_exception_turns_the_guard_red`
  - どちらもファイルを作らず、実在しない絶対パスを組むだけ。そのため動作上の害は無いが、このガードは赤になる。
- **是正後**: 2 か所を実在を問わない固定の絶対パス（`C:\areka-alert-tests`・`C:\areka-boot-resolve-tests`）へ替え、`cargo test --workspace --no-fail-fast -j 4` を取り直した → **exit 0・成功 8,297 件・失敗 0 件**。

## 2. 実機 4 点（要件 8.5・8.6）

共通の設定（パスはすべて短い絶対パス）:

- 根 `C:\home\maz\tmp\areka-root-6-3\`: `vendors\sample_ghost\emo2.nar` を展開し、`emo2-kakukaku/` を `balloon\emo2-kakukaku\` へ、それ以外を `ghost\emo2\` へ置いた（110 ファイル・`profile` は含まない）
- `AREKA_PROFILE_DIR=C:\home\maz\tmp\areka-prof-6-3\`（開発者の記憶には触れない）
- `RUST_LOG=info,areka=debug`・`NO_COLOR=1`・`AREKA_APP_SMOKE_EXIT_MS=20000`
- **`AREKA_NO_ALERT` は設定しない**
- 実行ファイル: `target\debug\areka.exe`（隣に i686 の `shiori-host32-helper.exe`）。argv は無し
- 起動は `Start-Process -PassThru` で行い、pid と子プロセス（`Win32_Process.ParentProcessId`）を記録した

### ① 根に検体を置き、argv なしで起動 → 同梱バルーンで会話が出る

終了コード **0**（21.2 秒）

```
INFO areka::boot_config: ベースウェアの根を決めました event="root_resolved" root=C:\home\maz\tmp\areka-root-6-3 source=EnvVar
INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Only dir=C:\home\maz\tmp\areka-root-6-3\ghost\emo2
INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Companion dir=C:\home\maz\tmp\areka-root-6-3\balloon\emo2-kakukaku
INFO areka::boot_resolve: [boot_resolve] 最後に使ったものを記憶へ書きました（- は argv なので書いていない） event="last_used_recorded" ghost="emo2" balloon="emo2-kakukaku" shell="master"
INFO areka: 本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓） scopes=[0, 1]
INFO kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1
INFO areka::emo2_boot::balloon_visibility::phase: [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true
INFO areka::emo2_boot::balloon_visibility::phase: [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="content" visible=true
INFO kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
INFO areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=Smoke closed=4
```

- 次の警告は **0 件**: `last_ghost_not_found`・`last_balloon_not_found`・`companion_balloon_not_found`・`catalog_*`・`event="alert"`
- WARN は 4 件あり、どれも本仕様とは関係ない。検体素材の全透明の要素が 2 件、バルーン定義の折返し基準が 1 件、有界終了による `force_quit` が 1 件。

### ② 記憶が書かれ、再起動で同じゴースト・同じバルーンが立つ

①の終了後に記憶を確認した。

App スコープ `C:\home\maz\tmp\areka-prof-6-3\sylphya.toml`:

```toml
format-version = 1

[last]
ghost = "emo2"
```

Ghost スコープ `C:\home\maz\tmp\areka-root-6-3\ghost\emo2\ghost\master\profile\areka\sylphya.toml`:

```toml
format-version = 1

[boot]
count = "1"

[last]
balloon = "emo2-kakukaku"
shell = "master"
```

同じ条件で再起動した。終了コードは **0**（21.0 秒）:

```
INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Memory dir=C:\home\maz\tmp\areka-root-6-3\ghost\emo2
INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Memory dir=C:\home\maz\tmp\areka-root-6-3\balloon\emo2-kakukaku
INFO kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1
INFO areka::emo2_boot::balloon_visibility::phase: [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true
```

`*_not_found` と `catalog_*` の警告はここでも 0 件。

### ③ 空の根で告知が出て、閉じると終了コードが 0 以外

- 根 `C:\home\maz\tmp\areka-empty-6-3\`（空）、`AREKA_PROFILE_DIR=C:\home\maz\tmp\areka-prof-empty-6-3\`、`AREKA_NO_ALERT` は未設定
- 起動から 162 ms で、自分が起こした pid の最上位の窓に表題「areka を起動できません」が出た。その窓（`MainWindowHandle`）へだけ `PostMessage(WM_CLOSE)` を送って閉じた。
- 終了コード **1**（1.7 秒）

```
ERROR areka::alert: [alert] 起動できないことを利用者へ告げます event="alert" scene=GhostMissing { ghost_store: "C:\\home\\maz\\tmp\\areka-empty-6-3\\ghost", argv: None } title="areka を起動できません" body="ゴーストが見つかりません。\n置く場所: C:\\home\\maz\\tmp\\areka-empty-6-3\\ghost\n置くもの: ghost\\master\\descript.txt を持つゴーストのフォルダ" suppressed=false
Error: Error { code: HRESULT(0x80004005), message: "エラーを特定できません" }
```

- 空の根と、その回の記憶の場所には、ファイルが 1 つも書かれていなかった（窓を開く前に止まっている）。

### ④ 有界終了のあとにプロセスが残らない（要件 8.6）

終了から 2 秒後に、自分が起こした pid が生きているかを `Get-Process -Id` で調べた。

| 走行 | 起こした pid | 子 pid | 終了コード | 終了後に残ったもの |
|---|---|---|---|---|
| ① | areka.exe 28324 | shiori-host32-helper.exe 28892 | 0 | なし |
| ② | areka.exe 24240 | shiori-host32-helper.exe 12296 | 0 | なし |
| ③ | areka.exe 28624 | （なし） | 1 | なし |

プロセスは 1 つも止めていない（止める必要が無かった）。自分が起こしたもの以外には触れていない。

## 3. 裁定 1〜5 について（要件 9.6）

実機の結果には、裁定を覆す必要を示すものは無かった。
- ①: ゴーストは唯一の 1 体、バルーンは同梱のものが選ばれた
- ②: 記憶は argv 以外の経路で書かれ、バルーンの記憶は Ghost スコープにある
- ③: 告知のあと 0 以外で終了した

## 4. 残った事項

- §1 の赤（`temp_path_guard_test` の 3 件）は是正済み。ワークスペースの回帰は緑。
- 書類の食い違い（design.md の設計判断 #10 が「実機 ③ も `AREKA_NO_ALERT=1`」と書き、Testing Strategy・要件 8.5 と逆だった）は、#10 を「smoke だけ設定・実機 ③ は設定しない」へ直した。
