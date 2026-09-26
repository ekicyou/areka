# ギャップ分析と設計の調べ物: areka-P0-alpha-package

> §1〜§10 は 2026-09-26・worktree の HEAD `d386c580`（main `2f5bd24a` の上に spec の初期化 1 コミット）での実測（`/kiro-validate-gap`＋要件討議）。依存の数だけは本流の作業コピーの `Cargo.lock`（2026-09-26 20:01 更新）を `--locked` で読んだ。
> §11〜§15 は同日の `/kiro-spec-design`（HEAD `a8fde786`）で足した設計時の調べ物と決定。
> 本文のコードの引用は「何を定義している行か」で指す。

## 1. 要約

- **土台はほぼ揃っている**。起動の解決（根は exe の隣・記憶は `<exe>/profile/areka`・既定の名前）、`.nar` の展開の窓口（`nar-sample-path`）、有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）、告知の抑止（`AREKA_NO_ALERT`）、判定に使える記録の行（「バルーンを決めました」の `route=`・「本物のゴースト窓を開きました」・「SHIORI が動かなくなりました」）は全部すでに在る。**`crates/` を変えずに要件 1〜7 を満たせる**見込みで、`crates/` の変更は 0 のまま進められる。
- **無いもの**は 3 つ: 配布スクリプト本体（`tools/package-alpha.ps1`）・第三者向け README・zip に入れる謝辞を「zip の exe と同じ依存の版」で作る段。どれも新規ファイルで、既存のファイルへの変更は根の `README.md`（と議題しだいの数ファイル）だけ。
- **要件に書かれていない落とし穴が 4 つ見つかった**（§4）: ① release の `areka.exe` は `VCRUNTIME140.dll` に依存し、開発機では必ず在るので起動確認で見つからない ② 展開先のパスが長いと pasta の Lua の読み込みが失敗し、接続の失敗の記録を出さずにゴーストが黙る ③ 「接続の失敗の記録が無い」は自動終了が早すぎると空振りで通る ④ 謝辞の今の作り方（`--workspace`・対象の機種の指定なし）は zip の exe に入らない依存まで載せる。
- **前例**: スクリプトの形は `tools/test-all.ps1`（段ごとの合否・コミットと未コミットの件数の印字・i686 の準備）、release の `areka.exe` を子として起こして出力をファイルへ受ける形は `tools/perf/invoke-perf-run.ps1`（`Start-Process -RedirectStandardOutput`・番犬で `Kill()`）、判定の目印と環境変数の渡し方は `crates/areka/tests/smoke_boot_loop_exit.rs`（根方向）にある。
- 規模は **S〜M**（4〜6 タスク）、危険度は **中**（release ビルドが重い・記録の出る時刻に依存する判定・第三者の機械の差）。

## 2. 今あるもの（要件ごとの対応表）

凡例: 在る＝そのまま使える／**欠**＝無い（作る）／**未知**＝設計で調べる／**制約**＝既存の作りが課す条件。

| 要件 | 使える既存物 | 判定 |
|---|---|---|
| 1.1 release で作り直す | `Cargo.toml` の `[profile.release]`（`opt-level='z'`・`lto=true`・`codegen-units=1`・`panic='unwind'`・`strip=false`）。helper は `crates/shiori-host32-helper`（`[dependencies]` は `shiori-host32-ipc`・`wintf-winmsg-executor`・`thiserror`・`windows` だけ） | 在る（コマンドを呼ぶだけ）／**制約**: 同じ `target/` に x64 の helper が居ることがある（下の 2.6） |
| 1.2 i686 ターゲットの導入 | `tools/test-all.ps1` の「i686 ターゲット導入」段（`rustup target add`）と、Git Bash の `link.exe` を PATH から外す前置き | 在る（写せる） |
| 1.3 展開は `nar-sample-path` から | `crates/sample-ghost-kit/src/bin/nar-sample-path.rs`（`root=`／`folder=`／`balloon.<名>=` を 1 行ずつ）。中身は本番の展開器 `areka-nar` の `install` を通る（`devroot.rs` の原本を組む関数）。読み手の前例は `tools/perf/perf-loop.common.ps1` と `tools/perf/invoke-followup-checks.ps1` の `Get-NarSampleMap` | 在る |
| 1.4 置き場・コミット・未コミット件数 | `tools/test-all.ps1` の `$head = git rev-parse --short HEAD`／`$dirty = @(git status --porcelain).Count` | 在る（写せる）。zip の中の記録の形式は **欠** |
| 1.5 段の名前と非 0・半端な zip を残さない | `tools/test-all.ps1` の `Step` 関数（ただし赤でも最後まで回す作り＝本スクリプトは最初の赤で止める必要があり、形が違う） | **欠**（止め方と「作業場所で組んでから改名で置く」は新規） |
| 1.6 追跡ファイルを書き換えない | `.gitignore` が `target`・`Cargo.lock` を無視。`nar-sample-path` は `target/nar-samples/manual/` に書く | **制約**: 謝辞の生成を `tools/test-all.ps1` と同じ `-o THIRD-PARTY-NOTICES.md` で呼ぶと追跡ファイルを書き換える＝出力先を `target/` の下にする必要がある |
| 1.7 使い方に「実機の木を消す」 | `nar-sample-path.rs` の doc と `vendors/sample_ghost/README.md` の「取り出し方」に同じ注意書きが在る | 在る（文面を写す） |
| 2.1 最上位の形 | `boot_config.rs` の `default_helper_exe_path`（exe の隣の `shiori-host32-helper.exe`）・`resolve_root_from`（`AREKA_ROOT` が無ければ exe の親）・`boot_resolve.rs` の `DEFAULT_GHOST_FOLDER`＝`emo2`／`DEFAULT_BALLOON_FOLDER`＝`StayseeBalloon` | 在る |
| 2.2 説明書 2 本を 1 バイトも違えず | `emo2.nar` に `readme.txt`（3,105 バイト）と `shell/master/readme.txt`（2,903 バイト）が在る。`nar-sample-path` の展開はバイトの複製 | 在る。判定（`.nar` の中身と比べる）は **欠** |
| 2.3 起動記録を入れない | `emo2.nar` の中の `profile` を含む名前は **0 件**（実測）。`nar-sample-path` は呼ぶたびに作り直す | 在る。ただし写す元の木を開発者が同時に起動していると `profile/` が生える（1.7 の注意と同じ理由）→ 判定は **欠** |
| 2.4 入れてはいけないもの | 検体の登記表 `sample-ghost-kit` の `SAMPLES`（7 本） | 判定は **欠**。**制約**: `smoke_boot_loop_exit.rs` の i686 を見分ける関数の注記どおり「`cargo build --workspace` は x64 の helper を `areka.exe` の隣へ置く」ので、`target/release/` をそのまま写すと x64 の helper が入る |
| 2.5 `emo2-kakukaku` の有無 | 展開した木の `balloon/emo2-kakukaku/`。ゴーストの木には同梱の取り出し元は残らない（`areka-nar` の `plan.rs` の「同梱バルーンの取り出し元は本体の側に複製しない」）。`ghost/emo2/install.txt` は残る | 在る（写すか写さないかだけ） |
| 2.6 実物の一覧から判定 | なし | **欠** |
| 3.1 外へ展開・引数なしで起動 | `smoke_boot_loop_exit.rs` の根方向（引数なし・`AREKA_ROOT` に検体の根）が同じ形を debug で通している | 在る（debug）／release での実走は本仕様が初。**未知**（下の 4.2 の長いパス） |
| 3.2 環境変数 | `smoke_boot_loop_exit.rs` の共通の起こし方（`AREKA_ROOT`・`AREKA_PROFILE_DIR` を外し、`AREKA_NO_ALERT=1`・`NO_COLOR=1`・`AREKA_APP_SMOKE_EXIT_MS` を渡す） | 在る（写せる）。**制約**: 実行時に読む `AREKA_*` はほかに 10 種ある（§4.5） |
| 3.3 終了コード 0・窓が立った・接続の失敗なし | 目印の行: `ghost_session.rs` の「本物のゴースト窓を開きました」・`emo2_boot/mod.rs` の「実 sink 結線が成立しました（wire 成立）」・`alert.rs` の `SHIORI_FAULT_TITLE`「SHIORI が動かなくなりました」・`areka-kanade/src/shiori/real.rs` の `event = "connect_failed"`／`"helper_exited"`。SHIORI の失敗で終わると終了コード 1（`main.rs` の `finish_after_run` が Fault なら `Err`） | 在る。**制約**: 記録が出る時刻（§4.3） |
| 3.4 上限で自分の子だけ止める | `tools/perf/invoke-perf-run.ps1` の番犬（締切を過ぎたら `$proc.Kill()`）。helper は `shiori-host32-host/src/job.rs` の「親が終わると OS が helper を道連れにする」仕組み（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）で片付く＝**止めるのは `areka.exe` 1 つで足り、他のプロセスへ手を出す必要は 0** | 在る |
| 3.5 初回のバルーン | `boot_config.rs` の `event = "balloon_resolved"` の行（「バルーンを決めました」・`route=`・`dir=`）。`boot_resolve.rs` の `resolve_balloon`: 同梱が在れば `route=Companion`、無ければ `companion_balloon_not_found` の warn の後、バルーンが 1 本だけなので `route=Only` | 在る |
| 3.6 記録の置き場所を残す | `invoke-perf-run.ps1` の `run.log`／`err.log` の形 | 在る（写せる） |
| 3.7 手で回す | 変更 0 | 在る |
| 4.x 第三者向け README | なし。事実の出どころ: メニューの項目名は `menu/captions.rs` の `FRAME_CAPTIONS`（ゴースト・シェル・バルーン・ネットワーク更新・インストール…・説明書・終了）、終了は同じ表の「終了」と OS の閉じる要求（`app_exit.rs` の `quit_app`）、記憶の置き場は `default_app_profile_dir` | **欠**（新規ファイル） |
| 5.x 同梱物の条件 | `vendors/sample_ghost/README.md`（`StayseeBalloon` の CC0・`konnoyayame` の CC BY-NC-ND）。`emo2` は書庫の中の `readme.txt`・`shell/master/readme.txt` | **欠**（文面）。**未知**: バルーン画像素材とシェル City-Pop'n の条件（§5） |
| 6.x 根の README | 根の `README.md` のバッジ（リンク先 `LICENSE`・不在）とライセンスの節（「MIT OR Apache-2.0」）、「現在の到達点」の「57件の仕様を完了し、基盤レイヤーの約70%」 | 直す箇所は特定済み |
| 7.x 謝辞 | `about.toml`（`ignore-dev-dependencies = true`・`accepted` 9 種）・`about.hbs`・`deny.toml`・`tools/test-all.ps1 -License` の `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md` | 在る。**制約**: 範囲と出力先（§4.4） |

### 2.1 前例の置き場（実装で真似る形）

- **スクリプトの骨組み**: `tools/test-all.ps1`（66 行）。`Set-Location (Split-Path $PSScriptRoot -Parent)`・`link.exe` の除外・`Step` 関数・結果の一覧。
- **release の `areka.exe` を子に起こす**: `tools/perf/invoke-perf-run.ps1` の「実走の起動と CPU 時系列の採取」の節。`Start-Process ... -RedirectStandardOutput $runLogPath -RedirectStandardError $errLogPath -NoNewWindow -PassThru` で、**コンソールを持たない release の exe でも出力をファイルに受けて grep している**（この道具は release で計測した実績がある）。環境変数は自分のプロセスに設定して継承させ、終わったら元に戻す形。
- **判定の目印・環境変数の外し方**: `crates/areka/tests/smoke_boot_loop_exit.rs` の `run_smoke`・`assert_no_shiori_fault`・`assert_line_has`・`accepted_as_no_monitor`。
- **i686 の PE の見分け**: 同じファイルの `is_i686_pe`（PE ヘッダの機種 `0x014c`）。PowerShell で同じ判定を数行で書ける。

## 3. 要件の実現性

### 3.1 技術的に要るもの

| 項目 | 中身 | 新規か |
|---|---|---|
| ビルド | `cargo build --release -p areka`（x64）＋`cargo build --release -p shiori-host32-helper --target i686-pc-windows-msvc` | 呼ぶだけ |
| 展開 | `cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2`／`-- StayseeBalloon` の `folder=`・`balloon.emo2-kakukaku=` | 呼ぶだけ |
| 組み立て | 作業フォルダ（`target/` の下）に最上位の形を作り、zip に固めてから置き場へ改名 | 新規 |
| 中身の判定 | zip を開き直して名前の一覧を得て、要件 2.1〜2.5 と照合。説明書 2 本は `emo2.nar` の中のものとバイト比較 | 新規 |
| 謝辞 | `cargo about generate`（範囲と機種は §4.4）＋前提の `cargo deny check`（`licenses` だけか全部かは設計） | 呼ぶだけ（出力先だけ変える） |
| 起動確認 | 短いパスの空フォルダへ展開 → `Start-Process` → 番犬 → 出力の grep | 新規（前例あり） |
| 文書 | 第三者向け README（新規）・根の `README.md` の 3 か所＋案内 1 行 | 新規／修正 |

### 3.2 複雑さの種類

外部の道具を順に呼ぶ手順（ワークフロー）と、文書。アルゴリズムの難所は 0。難しさは「開発機でだけ通る」経路を塞ぐこと（§4）に集まっている。

## 4. 要件に書かれていないギャップと制約（設計で扱う）

### 4.1 release の `areka.exe` は VC++ ランタイムに依存する（**第三者の機械で起動しない恐れ**）

- `dumpbin /DEPENDENTS` の実測: 本流の `target/debug/areka.exe` は `VCRUNTIME140.dll` と `api-ms-win-crt-*`（UCRT）を読む。リポジトリにも `CARGO_HOME` にも `crt-static` の設定は **0 件**（`.cargo/config.toml` は無い）なので、release でも同じはず（release の exe の実測は未＝**調べる**）。i686 の helper も同じ設定で組まれるので `VCRUNTIME140.dll`（32 ビット版）を読むはず（実物が手元に無く未確認）。
- 一方 `emo2.nar` の中の `pasta.dll` は `VCRUNTIME140` を**読まない**（実測: `user32`・`kernel32`・`bcryptprimitives`・`ws2_32`・`ntdll`・`userenv` など）。完了 spec `areka-P0-newline-defer` の tasks.md にある「pasta.dll＋VCRUNTIME140 x86」の書きぶりは推測だったことになる。
- UCRT は Windows 10 以降に標準で入るが、`VCRUNTIME140.dll` は Visual C++ 再頒布可能パッケージを入れた機械にしか無い。**開発機には必ず在るので、要件 3 の起動確認では見つからない**（要件 3 の目的「開発機の環境に助けられて動いているだけの zip を配らない」に正面から当たる）。
- 選べる手: ⒜ 配布スクリプトだけで `RUSTFLAGS=-C target-feature=+crt-static` を渡して静的に結ぶ（`crates/` と追跡ファイルの変更 0。ビルドの鍵が変わるので普段の `target/` と取り合わないよう `--target-dir` を分けるのが自然）⒝ README の「既知の制限」に再頒布可能パッケージ（x64 と x86 の両方）が要ると書く ⒞ zip の中の exe の依存 DLL を検査して、Windows 標準に無いものが在れば赤にする（⒜ の見張りにもなる）。

### 4.2 展開先のパスが長いと、接続の失敗を出さずにゴーストが黙る

- 記憶の注記（2026-09-18 実測）: pasta は初回の起動で `ghost/master/profile/pasta/pasta_scripts/…` へ Lua を自己展開し、その最も深いファイル（`…\pasta\shiori\event\virtual_dispatcher.lua`）が 260 文字を超えると Lua の `require` が落ち、**以後すべてのイベントに 204 を返す**。SHIORI の接続そのものは成り立つので `connect_failed` も「SHIORI が動かなくなりました」も出ない。
- zip の最上位から数えた末尾は `\ghost\emo2\ghost\master\profile\pasta\pasta_scripts\pasta\shiori\event\virtual_dispatcher.lua`（約 93 文字）なので、**展開先のフォルダは概ね 160 文字未満**である必要がある。セッションの作業フォルダ（`…\Temp\claude\C--home-maz-git-areka--claude-worktrees-…\<uuid>\scratchpad\…`）はそれだけで超えうる。
- 影響: 要件 3.1 の「リポジトリの外の新しい空の場所」の選び方と、第三者向け README の「既知の制限」（深いフォルダに置かない）。
- 要件 3.3 の「接続の失敗の記録が無い」だけではこの失敗を見逃す。判定に**会話が始まった正の目印**を足すかが設計の分かれ目（候補: `areka-kanade/src/schedule/boot.rs` の `event = "boot_talk"`「起動グリーティングを再生起動」・`event = "boot_complete"`。204 だけが返る状態では `boot_talk` が出ない見込み＝**調べる**）。

### 4.3 「接続の失敗の記録が無い」は時刻に依存する

- `main.rs` の自動終了は `AREKA_APP_SMOKE_EXIT_MS` のミリ秒後に全窓を閉じる。SHIORI の接続（helper の起動＋`pasta.dll` の読み込み＋初回の Lua の自己展開）は起動を待たずに裏で進む（`emo2_real_run.rs` の冒頭の注記「boot は actor spawn で完了し SHIORI 接続の成否を待たない」）。
- 前例の値: 根方向の smoke は 500ms（窓が立つことだけを見る）、`emo2_real_run.rs` は 3000ms、SHIORI の失敗方向の smoke は「失敗（数秒）を自動終了より先に起こす」ために 20000ms。
- **自動終了が接続の結果より先に来ると、失敗していても「失敗の記録なし・終了コード 0」で通る**。値の決め方（固定値か、正の目印を待つか）と上限の時間（要件 3.4）の関係は設計で決める。

### 4.4 謝辞の範囲と出力先

- 今の作り方（`tools/test-all.ps1 -License`）は `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`。`about.toml` に対象の機種（`targets`）の指定は **0 件**。
- 実測（本流の `Cargo.lock`・`cargo tree -e normal --locked`・x64 の既定の機種）: `areka` が引く外部の crate は **146**、i686 の helper は **19**（すべて `areka` の 146 に含まれる）、`--workspace` は **148**（差は `adler2`・`miniz_oxide` の 2 本＝`areka-nar` と検体の窓口が引くもので、今の `areka.exe` には入らない）。一方、今の `THIRD-PARTY-NOTICES.md` は **219 crate**（MIT 213・BSD-3 2・zlib 2・Apache-2.0 1・Unicode-3.0 1）を載せている＝機種を絞っていないので Windows 以外でしか使わない依存まで数えている見込み（**調べる**）。
- `cargo-about 0.9.2` は `-m/--manifest-path`（その crate だけ）・`--target`（複数可）・`--locked`（`Cargo.lock` を変えない保証）・`--fail` を持つ。**同じ作業コピーでビルドの直後に `--locked` で生成すれば、zip の exe と同じ版から作ったことが保証できる**（要件 7.1）。`Cargo.lock` が追跡外でもこれは成り立つ。
- 出力先を `-o THIRD-PARTY-NOTICES.md` のままにすると追跡ファイルを書き換える（要件 1.6 違反）。`target/` の下へ出す必要がある。
- 選べる手: ⒜ `--workspace`＋`--target x86_64-pc-windows-msvc --target i686-pc-windows-msvc`（148 前後・わずかに多めに載せる・呼び出し 1 回）⒝ `-m crates/areka/Cargo.toml` と `-m crates/shiori-host32-helper/Cargo.toml --target i686-…` の 2 回（zip の exe と一致・2 つの文書かつなぎ合わせ）⒞ 今のまま `--workspace` で機種指定なし（219・多めに載せるのは法的には安全だが「一致」とは言いにくい）。
- 道具の有無: 開発機には `cargo-about 0.9.2`・`cargo-deny 0.20.2` が在る。無い機械で走らせたときの扱い（段の失敗として止める）は要件 1.5・7.2 で足りる。

### 4.5 子へ渡す環境変数は 2 つを外すだけでは足りない

- 実行時のコードが読む環境変数のうち `AREKA_` で始まるものは 14 種（`AREKA_ROOT`・`AREKA_PROFILE_DIR`・`AREKA_NO_ALERT`・`AREKA_APP_SMOKE_EXIT_MS` のほか、`AREKA_TICK_GATE`・`AREKA_SHIORI_REQUEST_TIMEOUT_MS`・`AREKA_BALLOON_TIMEOUT_MS`・`AREKA_SHIORI_DEMO`・`AREKA_DIAG_OUT`・`AREKA_TRANSITION_LOG`・`AREKA_PERF_THREAD_REPORT_SEC`・`AREKA_WINTF_VBLANK_DEADLINE`・`AREKA_WINTF_REAL_WINDOW_ZORDER`・`AREKA_CHOICE_HOVER_INJECT`、ほか `WINTF_CROSSHAIR`）。開発者のシェルに残っていると、開発機の設定に助けられた（または壊された）起動確認になる。
- `RUST_LOG` が `warn` などに絞られていると info の目印（「バルーンを決めました」「本物のゴースト窓を開きました」）が出ず、判定が偽の否になる。`emo2_real_run.rs` は `RUST_LOG` を明示している。
- `NO_COLOR=1` を渡さないと、パイプでも欄の名前と値の間に着色の制御文字が挟まり `route=Companion` をそのまま照合できない（`smoke_boot_loop_exit.rs` の冒頭の注記・記憶の注記と一致）。
- 要件 3.2 は外すものを 2 つだけ名指ししているので、「`AREKA_` で始まるものを全部外し、要るものだけ入れ直す」かは設計で決めてよい範囲（要件の文言には反しない）。

### 4.6 その他の制約

- **x64 の helper の混入**（要件 2.4・2.6）: `target/release/shiori-host32-helper.exe` が x64 で残っていることがある。i686 の出力（`target/i686-pc-windows-msvc/release/`）から写し、PE の機種 `0x014c` を判定に含めるのが安全。
- **半角でない名前**: `emo2.nar` の中に半角でない名前が **1 件**（`shell/master/CityPop/頬差分.pdn`）、`StayseeBalloon.nar` は **0 件**。.NET の `ZipFile` は UTF-8 の印を付けて書く。Windows のエクスプローラーの「すべて展開」がこの印を読むか（文字化けしないか）は **調べる**。なお `StayseeBalloon/readme.txt` は Shift_JIS だが中身なので影響は 0。
- **release のビルド時間**: `lto=true`・`codegen-units=1` で長い。メモリ不足（os error 1455）の前例があり `-j 4` の扱いを `tools/test-all.ps1` に揃えるか。
- **`nar-sample-path` は呼ぶたびに `target/nar-samples/manual/<検体>/` を消す**（要件 1.7）。同じ検体で実機を回している開発者の木を消すので、使い方の説明に書く（要件どおり）。
- **debug の exe での代用は不可**: debug はコンソールを持ち、D3D11 のデバッグ層を必ず求める（steering `tech.md`）。起動確認は release の exe でしか意味を持たない（要件どおり）。

## 5. 同梱物の条件の事実（要件 5 と議題 ⑴ の材料）

| 資産 | 作者 | 条件の出どころ | 条件（原文の要旨） |
|---|---|---|---|
| ゴースト `emo2`（辞書・スクリプト） | えちょ（ekicyou）＝areka の開発者本人 | `readme.txt`・配布サイト `https://ekicyou.github.io/ghost_dev/emo2/` | 書庫の中に利用条件の記載は **0 件**。開発者本人が決められる |
| SHIORI `pasta.dll`（32 ビット） | ekicyou | pasta のリポジトリ（本流の作業コピーの `vendors/pasta/`・この worktree は未取得） | `LICENSE` は MIT、`Cargo.toml` の `license` は「MIT OR Apache-2.0」＝**pasta 自身も areka と同じ食い違いを持つ**。`pasta.dll` は Lua（`mlua`）などを静的に抱えており、**その謝辞は areka の謝辞（cargo の依存だけ）には載らない** |
| シェル \0「コンフィズリー」 | ゆゆぴか | `shell/master/readme.txt` | 禁止: フリーシェルとしての再配布（作者失踪時のみ可・説明書の同梱が条件）・伺か関連物以外での使用・商用利用・立ち絵の左右反転。許可: 原型を留める加工・使わない画像の削除。お願い: 製作者名「ゆゆぴか」の記載 |
| シェル \1「City-Pop'n」 | 大槻 | `shell/master/readme.txt` の見出しと `readme.txt` のクレジット | 説明書には見出し（作成者・サイト `http://th88.blog.shinobi.jp/`）だけで、**条件の本文は 0 行**。→ 要件 5.6 の「未確認」の候補 |
| バルーン `emo2-kakukaku`（kakukaku for emo-gs） | えちょ（ekicyou） | `readme.txt`「利用バルーン」・`emo2-kakukaku/descript.txt` | 画像素材は「フキダシデザイン」（`https://fukidesign.com/`）より。書庫の中にバルーン自体の条件は 0 件 |
| バルーン画像素材 | フキダシデザイン | `https://fukidesign.com/terms`（2026-09-26 に取得・要約） | 著作権表記は不要。商用可・加工可。アプリ・ゲームへの組み込みは 20 点まで無料（超えると条件あり）。**「データの再配布」は禁止**＝伺かのバルーン（画像ファイルがそのまま入った配布物）がこの「再配布」に当たるかは本文から一意に読めない（**調べる**） |
| バルーン `StayseeBalloon` | ぽな | 書庫の中の `LICENSE`（CC0 1.0 全文）・`readme.txt` | CC0（転載・再配布・改変自由） |

- `emo2-kakukaku` の画像は 15 本（`balloons0.png`・`balloonk0.png`・`balloonc1〜4.png`・`arrow0/1.png`・`marker.png`・`online0〜3.png`・`sstp.png`・`sstp_new.png`）と、編集元の `online.pdn`。どれがフキダシデザインの素材由来かは書庫からは分からない。

## 6. 実装の進め方の選択肢

### 選択肢 A: 既存の `tools/test-all.ps1` に `-Package` を足す

- 触るもの: `tools/test-all.ps1`。
- ✅ i686 の準備と PATH の前置きを共有できる。❌ 要件の境界「`tools/test-all.ps1` の変更は 0」に反する。❌ 「赤でも最後まで回す」と「最初の赤で止め、半端な zip を残さない」は流れが逆。→ **要件上とれない**（比較のために挙げた）。

### 選択肢 B: 新しいスクリプト 1 本（`tools/package-alpha.ps1`）に全部を入れる

- 触るもの: 新規 `tools/package-alpha.ps1`（組む＋`-Check` で起動確認）・新規の第三者向け README・根の `README.md`。
- ✅ 要件どおりの 1 本。✅ 前例の形を写すだけ（i686 の準備・`Start-Process`・番犬）。❌ `test-all.ps1` の i686 の前置き（`link.exe` の除外）を写すので同じ数行が 2 か所に並ぶ。❌ 1 本の長さ（組む・判定・謝辞・起動確認で 250〜400 行の見込み）。
- 起動確認を「組んだ直後に続けて回す」か「スイッチで別に回す」かは要件 3.1 の「開発者が起動確認を求める」の読みで決める。

### 選択肢 C: スクリプト 2 本（組む／確かめる）＋共有の小片

- 触るもの: `tools/package-alpha.ps1`（組む）と `tools/check-alpha-package.ps1`（zip を受けて確かめる）、必要なら共通の関数を 1 ファイル。
- ✅ 「別の場所で組んだ zip」も確かめられる（`alpha-release-signoff` が既成の zip で実機一周するときに使える）。✅ 各本が短い。❌ 要件の文言は「配布スクリプト」が起動確認もする形なので、呼び口を 1 本に保つ工夫（組む側から確かめる側を呼ぶ）が要る。❌ ファイルが増える。

### 文書側（どの選択肢でも同じ）

- 第三者向け README の置き場の候補: 根の `README-ja.md`／`docs/alpha/README.md`／`dist/README.txt` など。zip の最上位へそのまま入れる（要件 4.6）ので、**zip の中での名前**（`README.txt` か `.md` か・文字コードと改行）も同時に決める。Windows の「メモ帳」で開く第三者には `.txt`・UTF-8（BOM の有無）・CRLF が読みやすい。steering の `structure.md` は `docs/` を「単発の技術メモ」と説明しているので、置き場に使うなら説明と食い違う。

## 7. 規模と危険度

- **規模: S〜M（4〜6 タスク）**。スクリプトは前例の組み合わせで新しい仕組みは無いが、判定（中身・記録）と §4 の塞ぎ（VC++ ランタイム・長いパス・時刻・謝辞の範囲）が上乗せになる。文書 2 本は軽い。
- **危険度: 中**。release ビルドの時間とメモリ、記録が出る時刻に依存する判定、開発機と第三者の機械の差（§4.1・§4.2）が読みにくい。技術そのものは既知で、未知の技術は 0。

## 8. 設計へ持ち越す調べ物

1. release の `areka.exe` と i686 release の helper の依存 DLL を実測する（`VCRUNTIME140.dll` の有無）。`+crt-static` で組んだとき、`panic='unwind'`・`human-panic`・`windows` crate との組み合わせで問題が出ないか。
2. release の `areka.exe`（コンソールなし）を `Start-Process -RedirectStandardOutput` で起こしたとき、tracing の出力がファイルに全部残るか（`invoke-perf-run.ps1` の前例で成り立っているはずだが、本仕様の目印の行で確かめる）。
3. 会話が始まった正の目印（`boot_talk` など）が、pasta が 204 だけを返す状態では出ないこと。自動終了の値を何ミリ秒にすれば接続の結果より後になるか（release・初回の Lua の自己展開込み）。
4. `cargo about` の範囲と機種の指定（§4.4 の ⒜〜⒞）と、今の 219 crate がどこから来ているか。
5. .NET の `ZipFile` が書く UTF-8 の名前を Windows のエクスプローラーが正しく展開するか（`頬差分.pdn` 1 件）。
6. フキダシデザインの「データの再配布」の禁止が、伺かのバルーンとして画像を同梱する形に当たるか（議題 ⑴ は 2026-09-26 の要件討議で「入れる」に裁定済み＝公開済みの `emo2.nar` と同じ形のため問い合わせは要しない。README の書き方は要件 5.5）。City-Pop'n の条件の出どころ（作者のサイト）。

## 9. 要件討議へ出す決めごと（事実だけを添える）

### 議題 ⑴ zip に `emo2-kakukaku` を入れるか

- 入れる: 初回のバルーンは `route=Companion`（`smoke_boot_loop_exit.rs` の根方向と同じ経路＝debug で毎回通っている）。`emo2` の作者が意図した見た目（`budoux_newline,1` で単語の境目で折り返す設定もこのバルーンの `descript.txt` に在る）。バルーンの作者は開発者本人だが、**画像素材はフキダシデザインのもので、規約は「アプリ・ゲームへの組み込み可（20 点まで無料）」「データの再配布は禁止」「表記不要」**。バルーンとしての同梱がどちらに当たるかは規約の本文から一意に読めない（§5）。
- 入れない: 初回は `companion_balloon_not_found` の warn が 1 行出てから `route=Only` で `StayseeBalloon`（CC0・条件の確認は済み）。2 回目からは記憶の段で決まる。`ghost/emo2/install.txt` は `balloon.directory,emo2-kakukaku` を宣言したまま（中身は変えない）。第三者が見る初回のバルーンは `emo2` の説明書（「利用バルーン: kakukaku for emo-gs」）と食い違う。
- 数: `emo2-kakukaku` は 20 ファイル・34,238 バイト（画像 15 本＋`.pdn` 1 本＋文字 4 本）。

### 議題 ⑵ areka 自身のライセンス

- 経緯（git の記録）: 根の `README.md` を「MIT OR Apache-2.0」と書いたのは 2026-02-14 の README の書き直し。`LICENSE-MIT` と `Cargo.toml` の `license = "MIT"` は翌 2026-02-15。2026-07-16 のライセンスの門（`deny.toml`・`about.hbs`・#57）は冒頭に「MIT 化を守る」「MIT で配布し続けられる」と書く。`LICENSE-APACHE` がリポジトリに在ったことは **0 回**。
- 作者は 1 人（`git shortlog` で ekicyou のみ・724 コミット）＝**今どちらに決めても、後で変えるのに他の人の同意は要らない**。
- 要件 6.4 の一覧に無い箇所: `crates/wintf/README.md`・`crates/dola/README.md`・`crates/areka/README.md` の 3 本が「MIT」と書いている。MIT OR Apache-2.0 を選ぶとこの 3 本も食い違いになる（要件 6.4 の「食い違いを 0」の範囲に入れるかを決める必要がある）。MIT 単独なら 3 本はそのままで一致。
- pasta（`pasta.dll` の出どころ）も同じ食い違い（`LICENSE` は MIT・`Cargo.toml` は MIT OR Apache-2.0）を持つ。第三者向け README に `pasta.dll` の条件を書くとき、どちらを書くかが同じ問いになる（pasta 側の修正は本仕様の外）。
- MIT OR Apache-2.0 の利点として一般に挙げられるのは、Apache-2.0 の特許の許諾と Rust の慣習への一致。費用は §「要件討議で決める 3 点」のとおり 5〜6 ファイル（＋上の 3 本）。

### 議題 ⑷ `Cargo.lock` を追跡するか（2026-09-26 要件討議で「追跡する」に裁定＝要件 1.6・7.3〜7.5）

- 今: `Cargo.lock` が在るのは本流の作業コピーだけ（2026-09-26 20:01 更新・264 パッケージ）。worktree は最初のビルドでその日の最新の版を解決するので、**枝ごと・日ごとに依存の版が上下する**（記憶の注記「謝辞の版の上下は環境差」と一致）。
- 追跡しない場合でも、配布スクリプトが「ビルドの直後に同じ作業コピーで `cargo about --locked`」とすれば、**zip の中の exe と謝辞の版は一致する**（要件 7.1）。追跡の有無が効くのは「別の機械・別の日に同じコミットから組み直して同じ zip になるか」だけ。
- 追跡する場合: スクリプトは `cargo build --locked` にしないと、ビルドが追跡ファイル `Cargo.lock` を書き換えて要件 1.6（`git status` が変わらない）に反しうる。
- 衝突の頻度の目安: 直近 30 日の 80 コミットのうち、いずれかの `Cargo.toml` を触ったものは 10（約 8 本に 1 本）。ワークスペースの中の crate を足すだけでも `Cargo.lock` は変わる。衝突したら `cargo` に作り直させれば解ける種類の衝突。
- Cargo の公式の案内は（2023 年の改訂以降）ライブラリも含めて `Cargo.lock` の追跡を既定としている。

### そのほか要件討議で確かめたいこと（議題でなく確認）

- **VC++ ランタイム**（§4.1）: 静的に結ぶ（配布スクリプトの `RUSTFLAGS`・`crates/` 変更 0）か、README の既知の制限に書くか。第三者の最初の起動の成否に直結する。
- **展開先の長さ**（§4.2）: 起動確認の展開先を短いパスに固定すること、README に「深いフォルダに置かない」を書くかどうか。
- **起動確認の正の目印**（§4.2・§4.3）: 要件 3.3 の 3 条件に「会話が始まった」を足すか（足さないと pasta が黙る失敗を通す）。
- **完了フォルダの数の数え方**（要件 6.5）: `.kiro/specs/completed/` の直下はフォルダ 200 と `.md` 1 本の計 201 項目。brief と要件の「201」は項目の数で、仕様の数はフォルダの 200。README に書く数はどちらかを決めておく。
- **謝辞の範囲**（§4.4）: 「一致」をどこまで厳密に取るか（`--workspace` で多めに載せてよいか）。

## 10. 設計で決めること（要件討議で設計へ回した項目・2026-09-26）

要件は「何を満たすか」までを決め、次の「どう満たすか」は `/kiro-spec-design` で決める。

1. **VC++ ランタイムに頼らない方法**（要件 2.7）: 配布スクリプトだけで `+crt-static` を渡して静的に結ぶか（`--target-dir` を分けるか）、それ以外か。判定は zip の exe の読み込む DLL の一覧を PE から読むか（`dumpbin` は VS が要る）。release の exe と i686 の helper の実測を先に取る（§8.1）。
2. **会話が始まった目印と自動終了の値**（要件 3.3・3.4）: 目印の候補は `boot_talk`・`boot_complete`（§4.2）。204 だけが返る状態で出ないことの確かめ、release・初回の Lua の自己展開込みで接続の結果より後になるミリ秒、上限の時間との関係（§4.3・§8.3）。
3. **子へ渡す環境変数の組み方**（要件 3.2）: `AREKA_` で始まるものを全部外して要るものだけ入れ直す形。記録の出し方（`RUST_LOG` の明示・`NO_COLOR=1`）は判定の前提として設計で決める（§4.5）。
4. **展開先の短いパス**（要件 3.1）: どこに作るか（例: ドライブ直下や `%TEMP%` の短い名前）と、上限の目安（§4.2 の約 160 文字）。
5. **謝辞の範囲**（要件 7.1）: §4.4 の ⒜ `--workspace`＋機種 2 つ／⒝ crate ごとの `-m`／⒞ 今のまま。前提のライセンスの検査（`cargo deny` の範囲）も同時に。
6. **スクリプトの形**（要件 1・3）: 1 本（`-Check` 等のスイッチ）か 2 本（組む／確かめる）か（§6 の選択肢 B・C）。
7. **第三者向け README の置き場と zip の中での名前**（要件 4.1・4.6）: `.md` か `.txt` か・文字コード（BOM）・改行（§6「文書側」）。
8. **zip の書き方**: .NET の `ZipFile` の UTF-8 の名前をエクスプローラーが読むか（`頬差分.pdn` 1 件・§4.6・§8.5）。作業フォルダで組んでから改名で置く形（要件 1.5）。
9. **release ビルドの並列度**: メモリ不足（os error 1455）の前例に合わせ `-j` を絞るか（§4.6）。
10. **`Cargo.lock` の追跡の始め方**（要件 1.6・7.3〜7.5）: 配布スクリプトのビルドと謝辞の生成に `--locked` を付ける形・`Cargo.lock` の改行の扱い（`.gitattributes` を要するか）・追跡を始めるコミットの切り方（`Cargo.lock` と作り直した `THIRD-PARTY-NOTICES.md` を同じコミットに入れる）。

## 11. 設計時の調べ物（2026-09-26・`/kiro-spec-design`・worktree HEAD `a8fde786`）

§8 の持ち越しのうち、読み取りだけで済むものはここで実測した。release ビルドと実走を要するものは「実装の最初のタスクで実測して確定」とし、既定値を設計に書いた。

### 11.1 実測した（結論は design.md に写した）

| § | 調べ物 | 方法 | 結果 |
|---|---|---|---|
| 8.1（一部） | exe の依存 DLL | PowerShell で PE の取り込み表を読む関数を書き、本流の `target/debug/areka.exe`（PE32+）と `emo2.nar` の `pasta.dll`（PE32）に当てた | `areka.exe`（debug）: `VCRUNTIME140.dll`＋`api-ms-win-crt-{math,string,runtime,stdio,locale,heap}-l1-1-0.dll` を読む（研究どおり）。`pasta.dll`（機種 `0x014c`）: `user32`・`kernel32`・`bcryptprimitives`・`api-ms-win-core-synch-l1-2-0`・`ws2_32`・`ntdll`・`userenv` だけ＝CRT 系 0。読み手は PE32／PE32+ の両方で正しく動いた（design の判定 6 の実装の下敷き）。release の実物と `+crt-static` の効果は未（実装で） |
| 8.3（一部） | 204 だけが返る状態で「会話が始まった」の目印が出ないこと | `areka-kanade/src/schedule/boot.rs` の `to_baseware_version` を読む | `event = "boot_talk"` は **2 つの経路で出る**: Value 側「起動グリーティングを再生起動」と、204＋`first_boot_epilogue` 非空の側「epilogue-only 起動記録トークを再生起動（挨拶トーク皆無）」。**`event` の名前で数えると 204 の失敗を通す**ので、判定は文言「起動グリーティングを再生起動」で行う。`boot_complete` は 204 でも出るので目印にしない。自動終了のミリ秒は未（実装で・既定 20000） |
| 8.4 | `cargo about` の範囲 | 本流の `Cargo.lock`（264 パッケージ・2026-09-26 20:01）を worktree へ複製（追跡外）し、`--locked` で 3 通り生成 | 機種の指定なし `--workspace`＝**225** crate（リポジトリの `THIRD-PARTY-NOTICES.md` は 220＝より古い lock の版・`ambassador 0.5.1→0.5.0`・`anstream 1.0.0→0.6.21` 等の差＝「版の上下」そのもの）／`--workspace --target x86_64-pc-windows-msvc --target i686-pc-windows-msvc`＝**189**／`-m crates/areka/Cargo.toml --target x86_64-…`＝**175**。生成は各 15〜35 秒。`cargo about generate` は `--locked`・`--target`（複数可）・`-m`・`--fail` を持つ（`--help` で確認）。`cargo deny` は大域の `--locked`／`--offline`／`--frozen` と `check licenses` を持つ |
| 8.5 | .NET `ZipFile` の UTF-8 の名前をエクスプローラーが読むか | `頬差分.pdn` と `plain.txt` を `ZipFile::CreateFromDirectory` で固め、ローカルヘッダの汎用目的ビットを読み、`Shell.Application`（エクスプローラーの zip フォルダそのもの）で列挙と `CopyHere` を行った | `頬差分.pdn` にだけ bit 11（UTF-8）が立つ（`0x0800`）。エクスプローラーの列挙も展開も `頬差分.pdn` を正しい名前で出した。`Expand-Archive` も同じ。**文字化けの心配は 0** |
| （3.1） | 既定の展開先の長さ | `[IO.Path]::GetTempPath()` | `C:\Users\<名>\AppData\Local\Temp\`＝34 文字。`areka-alpha-check-HHmmss` を足しても約 60 文字＝上限 160 の下 |
| （7.3） | `Cargo.lock` の改行と git の設定 | `file Cargo.lock`・`git config core.autocrlf`・根の `.gitattributes` | lock は LF。`core.autocrlf=true`。根に `.gitattributes` は無い |
| （6.5） | 完了 spec の数 | `.kiro/specs/completed/` 直下 | フォルダ **200**・`.md` 1 本（`graphics-rendering-stability.md`） |
| （2.2・2.3） | `emo2.nar` の中身 | `ZipFile::OpenRead` で列挙 | 110 項目・`profile` を含む名前 0・半角でない名前 1（`shell/master/CityPop/頬差分.pdn`）・`readme.txt` 3,105 バイト・`shell/master/readme.txt` 2,903 バイト・`install.txt` 135 バイト・`emo2-kakukaku/install.txt` 78 バイト |
| （1.3） | `nar-sample-path` のバルーン検体の出力 | `sample-ghost-kit/src/lib.rs` の `manual_paths`・`tests/nar_sample_path_test.rs` | 鍵は `root=`・`folder=`（`<根>/balloon/<名>`）・同梱があれば `balloon.<名>=`。`StayseeBalloon` は `folder=` が `<根>/balloon/StayseeBalloon` |

引き直した実在（design.md に綴ったもの・すべて今の木に在る）: `main.rs` の `windows_subsystem` 属性・`SMOKE_EXIT_ENV`・「smoke 自動 close ゲート有効」・`finish_after_run`・`EnvFilter::new("info")` の既定／`alert.rs` の `NO_ALERT_ENV`・`SHIORI_FAULT_TITLE`・`suppressed_from`（`""`・`"0"` 以外は抑止）／`boot_config.rs` の `resolve_root_from`・`default_helper_exe_path`・`default_app_profile_dir`・`event = "balloon_resolved"`／`boot_resolve.rs` の `DEFAULT_GHOST_FOLDER`・`DEFAULT_BALLOON_FOLDER`・`resolve_balloon`（`BalloonRoute::Companion`）／`ghost_session.rs` の「本物のゴースト窓を開きました」／`emo2_boot/frame/attach.rs` の「emo2 attach: 装着計画を実行」／`areka-kanade/src/shiori/real.rs` の `connect_failed`・`helper_exited`／`menu/captions.rs` の `FRAME_CAPTIONS`（7 行）／`app_exit.rs` の `quit_app`／`smoke_boot_loop_exit.rs` の `is_i686_pe`・`run_smoke`／`.claude/skills/kiro-complete/SKILL.md` の License Gate (b) の「環境差として戻す」の段落。

### 11.2 実装の最初のタスクで実測して確定するもの（設計は既定値を置いた）

1. `+crt-static` の release の `areka.exe`（x64）と `shiori-host32-helper.exe`（i686）の取り込み表に CRT 系（`vcruntime`・`msvcp`・`api-ms-win-crt-*`・`ucrtbase`）が 0 であること。`panic='unwind'`・`human-panic`・`windows` crate との組み合わせでビルドとリンクが通ること。
2. release の `areka.exe`（コンソール無し）を `Start-Process -RedirectStandardOutput` で起こしたとき、正の目印 4 種が `run.log` に残ること（`invoke-perf-run.ps1 -Build release` の前例は「有界の証跡の行」で成り立っている）。
3. `SMOKE_EXIT_MS`＝20000 で「起動グリーティングを再生起動」が自動終了より前に出ること。短くできるなら較正値を下げる（前例: debug の実走は 3000 で挨拶まで通る）。
4. `cargo build --locked` が CRLF で取り出された `Cargo.lock` を書き換えようとしないこと（cargo は行単位で比べるので起きない見込み・起きたら `.gitattributes` に `Cargo.lock -text` を足す＝設計の外の是正として報告）。
5. `cargo update -w` が「`Cargo.toml` の差分だけを lock に反映し他の版を動かさない」こと（`workflow.md` に書く衝突の解き方の前提）。

## 12. アーキテクチャの選択肢の評価（設計フェーズ）

| 選択肢 | 中身 | 長所 | 短所 | 判定 |
|---|---|---|---|---|
| B: 1 本＋`-Check` | `tools/package-alpha.ps1` に組む・判定・起動確認を全部入れる | 要件の文言どおり 1 本。前例を写すだけ。共有するファイル 0 | 300〜400 行。`link.exe` の除外が `test-all.ps1` と 2 か所に並ぶ | **採用** |
| C: 2 本＋共有の小片 | 組む／確かめる を分ける | 既成の zip を確かめられる | ファイルが増える。要件 1.1「その場のソースから作り直す」が常に先に立つので「既成の zip だけ確かめる」入口は本仕様に無い | 却下（要るときは `alpha-release-signoff` で `-Check` を切り出せばよい） |
| A: `test-all.ps1` に `-Package` | 既存 1 本に足す | i686 の準備を共有 | 境界「`test-all.ps1` の変更 0」に反する。止め方が逆 | 却下 |

## 13. 設計の決定（要件討議で設計へ回した 10 項目の答え）

### 決定 1: VC++ ランタイムは静的に結ぶ（要件 2.7）
- **代替**: ⒝ README の既知の制限に再頒布可能パッケージを書く／`VCRUNTIME140.dll` を zip に入れる。
- **選択**: 配布スクリプトだけが `RUSTFLAGS=-C target-feature=+crt-static` を設定し、`--target` 明示・`--target-dir target/alpha`。見張りは zip の exe の取り込み表を PE から読み、CRT 系の名前の拒否表に当たれば否。
- **理由**: 第三者の最初の起動を落とさない。`crates/`・追跡ファイルの変更 0。`--target` 明示でホスト側（ビルドスクリプト・手続きマクロ）に波及しない。`--target-dir` を分けるので普段の増分ビルドを壊さない。
- **代償**: release ビルドの成果物が普段の `target/release/` と別に 1 組増える。実物の確認は release ビルドを要する（実装の最初のタスク）。

### 決定 2: 会話の目印は文言「起動グリーティングを再生起動」・自動終了は 20000 ms（要件 3.3・3.4）
- **代替**: `event="boot_talk"` で数える（204 の失敗を通す＝却下）／`boot_complete`（204 でも出る＝却下）／目印を待って自分で止める（areka に外から止める口が無い＝不可）。
- **選択**: 文言で数える。`SMOKE_EXIT_MS`＝20000（`-SmokeExitMs` で上書き）、番犬＝＋60 秒。
- **理由**: §11.1 の読取で 2 経路の文言が違うことを確認。20000 は失敗方向の前例と同じで、接続（helper 起動＋`pasta.dll`＋Lua の自己展開）の結果より後になる余裕を持つ。

### 決定 3: 環境変数は `AREKA_*`・`WINTF_*` を全部外して 4 つだけ入れる（要件 3.2）
- `AREKA_APP_SMOKE_EXIT_MS`・`AREKA_NO_ALERT=1`・`RUST_LOG=info`・`NO_COLOR=1`。`RUST_BACKTRACE` は入れない（判定に使わない・human-panic が報告を書く）。

### 決定 4: 展開先は `GetTempPath()` の下・上限 160 文字（要件 3.1）
- `-CheckDir` で上書き可。上限を超えたら起動せず 3 で止める。展開先は合否に関わらず消さない（あとで読める・削除のコードを持たない）。

### 決定 5: 謝辞は `--workspace`＋機種 2 つ（要件 7.1）
- §11.1 の 189 crate。リポジトリの `THIRD-PARTY-NOTICES.md`（225・機種なし）とは範囲が違い版は同じ。前提の検査は `cargo deny --locked check licenses`（advisories は含めない）。`--fail` は足さない（完了の手順と同じ厳しさ）。

### 決定 6: スクリプトは 1 本（§12）。

### 決定 7: 第三者向け README は `dist/README.txt`・UTF-8（BOM）・CRLF・zip でも `README.txt`（要件 4.1・4.6）
- `docs/` は steering の説明（単発の技術メモ）と食い違うので使わない。`dist/` は steering `structure.md` に 1 行登記する。

### 決定 8: zip は .NET `ZipFile`・`stage/` で組んで `.zip.tmp` → 判定 → 改名（要件 1.5）
- UTF-8 の名前は §11.1 で実測済み（エクスプローラーが読む）。

### 決定 9: release ビルドの `-j` は絞らない
- 前例の os error 1455 は `cargo test --workspace` の多数の並行リンク。1 本の release ビルドで出たら較正値として `-j 4` を足す。

### 決定 10: `Cargo.lock` の追跡は `cargo generate-lockfile` → 謝辞の作り直し → `.gitignore` の 1 行 を 1 コミットで（要件 7.3〜7.5）
- `.gitattributes` は作らない（§11.2 の 4）。並走 worktree の取り込み方（追跡外の lock を先に消す・衝突は main の lock を採って `cargo update -w`）を `workflow.md` に 1 節。完了の手順の「環境差として戻す」は「戻さず原因を確かめる」へ。

## 14. 統合（synthesis）の記録

- **一般化**: 「判定は集めてから 1 回主張する」を中身の判定と記録の判定の共通の形にした（1 つ目で止めると残りの赤が見えない）。較正値は冒頭 1 か所（`invoke-perf-run.ps1` と同じ体裁）。
- **作る／借りる**: ビルド・展開・謝辞・検査・圧縮はすべて既存の道具を借り、新しく作るのは「呼ぶ順」「写す表」「判定」だけ。PE の読み手は `is_i686_pe` の読み方（PowerShell で十数行）で、`dumpbin` を要しない。
- **削ったもの**: `run-meta.txt` のような別の記録ファイル（`BUILD-INFO.txt` 4 行で足りる）・既成の zip だけを確かめる入口・`RUST_BACKTRACE`・`--fail`・`-j` の較正値・展開先の削除・`.gitattributes`・helper を名前で探して止めるコード（OS のジョブが道連れにする）。

## 15. 危険と手当て

- release ビルドが長い（`lto`・`codegen-units=1`）— 1 本の走行で 2 回（x64・i686）。手で回す道具なので許容。
- `+crt-static` で未知のリンク失敗 — 実装の最初のタスクで実測。失敗したら決定 1 の代替（README に書く）へ戻して報告。
- 記録の文言の変更で判定が偽の否 — design の Revalidation Triggers に列挙。較正値の 1 か所を直せば済む。
- `Cargo.lock` の追跡開始が並走 5 本に波及 — `workflow.md` の 1 節で手順を固定。衝突は `cargo` に作り直させれば解ける種類。
