# ギャップ分析: audit-pilot

> 2026-11-17 `kiro-validate-gap`（`/kiro-start` の中のサブエージェント）。範囲は `crates/pilot/` のうち git に載ったファイル全部から `examples/pilot-windows-rs-next/` の下を除いたもの（要件 1.1 の【仮定 1】のとおり）。この範囲だけに `/ponytail:ponytail-audit` の作法をかけた。作法のうち 2 つは brief のとおり外した＝「最大 20 件」を外して全件を出す・「読まなかった所（Not checked）」を残さない（31 ファイル 9,000 行を全部、頭から終わりまで読んだ）。番号は audit の通し番号で、以後の段はこの番号で指す。形は完了した `audit-tools-scripts` の research.md に合わせた。
> 回さなかったもの: 先進坑の実走（画面・GPU・i686 の helper・本物の `pasta.dll`）・全体テスト・`cargo build`／`cargo test`（このワークツリーには `target\` が無く、組み立て直しに時間がかかるため）。回したのは `git` の読むだけのコマンド・`cargo metadata --no-deps`（組み立てない）・grep だけ。

## 1. 要約

- 範囲は先進坑 5 本（`pilot-balloon-asset-swap`・`pilot-clickthrough-alpha-toggle`・`pilot-dropfiles-on-wuc-window`・`shiori-host-32`・`wintf-winmsg-executor`）と雛形 `_template`・空の `lib.rs`・`Cargo.toml`。指摘は **27 件**＝必ず直す 4・直したほうがよい 13・できれば 10。
- いちばん重いのは **1**＝クリック透過の先進坑で、ドラッグの途中にマウスの捕まえ（キャプチャ）を他へ取られると「ドラッグ中」の印が立ったままになり、窓がカーソルに付いて動き、窓の矩形の全部がクリックを吸う（次のクリックまで戻らない）。次に **2**・**3**＝32bit の先進坑の「go 基準(2)」と異常系の確かめが、起きていないことを「合格」と出しうる（helper は UNLOAD が来なくても終了コード 0 で終わる・送りの失敗を全部「時間切れ」と呼ぶ）。
- **実行例の中のテスト 49 本は、全体テストでは走らないどころか組み立てもされない**（要件 1.9）。`crates/pilot` の実行例は全部 `test = false`（cargo の既定）で、`cargo test --workspace` は実行例を普通のプログラムとして組むだけなので `#[cfg(test)]` の部分は外れる（5 の件・7 節）。要件 4.3「直したバグごとに全体テストで走るテスト」を満たすには、この形を変えるかを先に決める必要がある（13 節の 1）。
- `pilot-clickthrough-alpha-toggle/main.rs` は 1,006 行で、1,000 行の番人の例外表に「今も越えている」として載っている。**6 行より多く減らすと番人が赤になる**（表の項目が越えなくなると赤＝`every_over_limit_exception_is_still_over_the_limit`）。この件に触る直し（1・15・16・17・27）の行数の出入りしだいで、C22 の約束 ⑵ に当たる（13 節の 2）。
- SSP との互換や ukadoc の正典に関わる指摘は 0。規模は M・リスクは中。タスクは 15〜18 の見立てで 20 に収まる（11 節）。

## 2. 測り直し（要件 1.2）

- 取り込んだ `main`: `5749a4c7`（このブランチの `HEAD~1`＝spec の初期化のコミットの親）。範囲の最後の変更は `73c4f87a`（2026-11-14 `wintf-winmsg-executor-import`・`Cargo.toml` の依存の書き換え）。起票の時（`e3c2cbe4`）からの行数の変化は 0。
- 物差しの版: brief は ponytail 5.3.2 と書くが、今入っているのは **5.3.3**。キャッシュに残る 5.3.2 の `skills/ponytail-audit/SKILL.md` と 5.3.3 のものは 1 字も違わない（diff 0 行）。作法は 5.3.3 の同ファイルに従った。
- 範囲: **31 ファイル**（`.rs` 22・文書 8・`Cargo.toml` 1）。`.rs` は 8,081 行で、brief と同じ数え方（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数える）で**本番のコード 7,550 行・テスト 531 行**。ほかに、本番のファイルの中の `#[cfg(test)] mod tests` が約 662 行ある（数え方の上ではコードに入る）。文書 855 行・`Cargo.toml` 64 行。

| ファイル | 行数 | `#[test]` の数 | 全体テストで走るか |
|---|---:|---:|---|
| `Cargo.toml` | 64 | — | — |
| `src/lib.rs` | 15 | 0 | （空の lib・テスト無し） |
| `README.md` | 67 | — | — |
| `examples/_template/{main.rs,README.md}` | 15＋27 | 0 | — |
| `pilot-balloon-asset-swap/main.rs` | 410 | 2 | 走らない |
| 同 `capture.rs` | 435 | 2 | 走らない |
| 同 `observe.rs` | 852 | 0（隣の `observe_tests.rs`） | — |
| 同 `observe_tests.rs` | 454 | 11 | 走らない |
| 同 `signature.rs` | 323 | 0（隣の `signature_tests.rs`） | — |
| 同 `signature_tests.rs` | 77 | 2 | 走らない |
| 同 `swap.rs` | 928 | 5 | 走らない |
| 同 `README.md` | 292 | — | — |
| `pilot-clickthrough-alpha-toggle/main.rs` | **1,006** | 7 | 走らない |
| 同 `README.md`・`REPORT.md` | 36・82 | — | — |
| `pilot-dropfiles-on-wuc-window/main.rs` | 305 | 2 | 走らない |
| 同 `dropfiles.rs` | 362 | 4 | 走らない |
| 同 `README.md` | 176 | — | — |
| `shiori-host-32/main.rs` | 684 | 0 | — |
| 同 `helper.rs` | 133 | 0 | — |
| 同 `helper_window.rs` | 432 | 0 | — |
| 同 `ipc.rs` | 408 | 5 | 走らない |
| 同 `parent_window.rs` | 217 | 0 | — |
| 同 `process_host.rs` | 267 | 4 | 走らない |
| 同 `sample_support.rs` | 24 | 0 | — |
| 同 `shiori3.rs` | 109 | 3 | 走らない |
| 同 `shiori_proxy.rs` | 272 | 2 | 走らない（i686 でしか意味が無い 1 本を含む） |
| 同 `README.md` | 79 | — | — |
| `wintf-winmsg-executor/{main.rs,README.md}` | 353＋96 | 0 | — |

- 1,000 行の近く: `pilot-clickthrough-alpha-toggle/main.rs` 1,006（例外表に載っている）・`pilot-balloon-asset-swap/swap.rs` 928・`observe.rs` 852。
- 範囲の中の `shortcut:`・`ponytail:` の注釈: 1 件（`pilot-dropfiles-on-wuc-window/dropfiles.rs:168`「文脈の Box は解放しない（窓 1 枚・使い捨て）」）。窓は 1 枚しか作らないので、書いた限界は今の負荷を越えていない＝作法どおり指摘にしない（要件 2.2 ⑵ に当たる、わざと置いた印）。
- 範囲の中の `#[allow(dead_code)]`: 8 か所（`observe.rs:5`・`pilot-clickthrough-alpha-toggle/main.rs:144`・`shiori-host-32` の 6 ファイルの頭）。理由が今も本当なのは 2 か所（`observe.rs:5`＝ログでしか読まない 3 欄・`main.rs:144`＝落とすと合成が崩れる COM の持ち続け）。残りの 6 か所は「後続タスクで配線するまで」の理由で、配線は済んでいる（10・11・12・13 の件）。

## 3. この範囲が何をするか・想定した負荷

**何をするか**: 二坑モデルの先進坑（使い捨ての試し掘り）のコード置き場。`lib.rs` は空で、コードは全部 `examples/<名前>/` の下の実行例にある（出荷するクレートは `pilot` に頼れない＝葉の隔離）。各先進坑は、開発者が 1 度ずつ手で走らせて方向を確かめ（go／違う／直す）、結果を隣の `README.md`（と `REPORT.md`）に一次記録として残した。5 本とも判定は済んでいる（go 4・直す 1）。

| 先進坑 | 何を確かめたか | 走らせ方 | 要るもの |
|---|---|---|---|
| `pilot-balloon-asset-swap` | 走っているバルーン窓の資産を差し替えたとき、どのフレームにも崩れが出ないか（Desktop Duplication で画面を取り込んで数える） | `cargo run -p pilot --example pilot-balloon-asset-swap`（上限 90 秒・終了コード 0〜3） | 実の画面・実 GPU・拡大率 100% |
| `pilot-clickthrough-alpha-toggle` | `WS_EX_TRANSPARENT` の付け外しで、DComp の描画を保ったまま別のプロセスへクリックが抜けるか | 同 `--example pilot-clickthrough-alpha-toggle`（ダブルクリックで終わる） | 実の画面・実 GPU・人の操作 |
| `pilot-dropfiles-on-wuc-window` | WUC 合成・クリック透過つきの窓に `WM_DROPFILES` が届くか | 同 `--example pilot-dropfiles-on-wuc-window`（上限 180 秒） | 実の画面・エクスプローラからの落とし物 |
| `shiori-host-32` | x64 の親が i686 の helper 越しに 32bit の `pasta.dll` を動かせるか（`WM_COPYDATA` の往復） | helper を `--target i686-pc-windows-msvc` で組み、`HELPER_EXE` を渡して親を走らせる。`--selftest-errors` で異常系 | i686 の組み立て・検体 emo2 の本物の `pasta.dll`（画面は要らない） |
| `wintf-winmsg-executor` | 外の executor へ UI の土台を移せるか（3 窓を 8 秒動かす） | 同 `--example wintf-winmsg-executor` | 実の画面 |

**想定した負荷**: 開発者 1 人が、方向を確かめるときに 1 本を 1 回（数十秒〜3 分）手で走らせる。同時に走らせる人も他のプロセスも無い。ただし**全体テストは毎回この範囲の実行例を全部組み立てる**（x64・テストではない普通の組み立て）ので、組み立てが壊れると全体テストが赤になる。

## 4. 範囲の外の読み手と外への約束（いま確かめた事実）

| 外への約束 | 読み手 | 確かめた所 |
|---|---|---|
| `pilot-clickthrough-alpha-toggle/main.rs` が**今も 1,000 行を越えている**こと | `crates/log-capture-kit/tests/file_length_guard_test.rs:75`（例外表・件数 4）＝`audit-test-kits` の範囲・C22 の約束 ⑵ | 読んだ |
| 同じファイルが**在る**こと（名前・置き場） | `crates/log-capture-kit/tests/workspace_scan_test.rs:319`（列挙が拾うべき既知のファイル） | 読んだ |
| `shiori-host-32/main.rs` の形（相対パスの検体の置き方をしない） | `crates/areka-kanade/tests/kanade/real_helper_test.rs:43`・`crates/areka/examples/emo-present.rs:152`（注記で名指し） | 読んだ |
| 検体の旧い置き場の綴り（`shiori-host-32/fixtures`）を書かないこと | `crates/log-capture-kit/tests/sample_path_guard_test.rs:77`・`:485`・`:500`・`:624`（番人） | 読んだ |
| `pilot-clickthrough-alpha-toggle/REPORT.md` の知見 | `crates/wintf/src/win_style.rs:45`（注記で名指し） | 読んだ |
| `crates/pilot` の構造（空の lib・実行例だけ）・`_template` | `.kiro/steering/two-tunnel.md`・`structure.md:433-435`・`workflow.md` | 読んだ |
| 実行例が全体テストで組み立てられること | `tools/test-all.ps1:75`（`cargo test --workspace --no-fail-fast -j 4`）・根の `Cargo.toml` の `members = ["crates/*"]` | 読んだ |
| dev 依存の `sample-ghost-kit` の公開の形（`SampleRoot::acquire`・`folder`・`balloon`） | この範囲が使う側。`audit-test-kits` の範囲（C22 の約束） | 読んだ |
| 1,000 行の番人ほか `log-capture-kit` の番人は `crates/**/*.rs` を全部歩く（`examples/` と `pilot-windows-rs-next/` を含む） | `crates/log-capture-kit/tests/workspace_scan/mod.rs:80-117` | 読んだ |
| 完了した spec の文書が範囲のファイルを名前で指す | `.kiro/specs/completed/` の 193 ファイル（`crates/pilot` を含む行）。消す・名前を変える直しは無いので、どれも壊れない | grep |

- `crates/pilot` の実行例を呼ぶコード・テストは範囲の外に 0（葉の隔離のとおり）。範囲の外へ直しが及ぶのは、上の表の 2 つの番人（行数・在ること）と、5 の件で全体テストの形を変える場合だけ。

## 5. 指摘の一覧（27 件）

旗の読み方:
- **(a) 一次記録**: 直しが、`README.md`・`REPORT.md` に書いた測った結果のもとになったコードを変えるか（要件 1.6・5.3）。「変える（記録の場所）」「なし」。
- **(b) ウェーブの約束**: C22 の約束（`main.rs` を割らない・1,000 行の番人の表と件数に触らない・依存を足さない・`sample-ghost-kit` の公開の形を変えない）に触れるか（要件 1.7）。
- **(c) 実機**: 確かめに実の画面・実 GPU・i686 の helper と本物の `pasta.dll` が要るか（要件 1.8）。「要る（何が）」「一部」（筋は決定論のテストで縛れる）「要らない」。
- **(d) 残す候補**: 要件 2.2 の ⑴〜⑶ に当たりうるか。候補を挙げるだけで、決めない。⑴ のうち「先進坑の品質の基準は緩くてよい（`two-tunnel.md` 5.4）」は、整形・命名・テストの厳しさに関わる件の全部に当たりうるので、当たる件に「⑴（5.4）」と書く（要件 2.7 の【仮定 2】で扱う）。

### 必ず直す（Must fix）

**1. クリック透過の先進坑で、ドラッグ中にマウスの捕まえを他へ取られると「ドラッグ中」が戻らず、窓がカーソルに付いて動き、窓の全部がクリックを吸う** （`pilot-clickthrough-alpha-toggle/main.rs:319-339`・`:342-367`・`:368-383`・`:722-726`）
- **何か**: 円の上を押すと `SetCapture` して `dragging=true` にし、`WM_MOUSEMOVE` で窓をカーソルに付けて動かし、`WM_LBUTTONUP` で `dragging=false` に戻して透過の付け外しを再開する。ドラッグ中は透過を外したまま（窓がクリックを受ける）にする。
- **困る場面**: ボタンを押したまま Alt+Tab・Win キー・UAC の確かめの画面などで捕まえが外れると、`WM_CAPTURECHANGED` が来て、離した `WM_LBUTTONUP` は別の窓へ行く。`dragging` は `true` のまま残り、透過の付け外しは止まったまま。窓（REPORT の観測で約 2880×1550 物理 px＝ほぼ画面全体）はボタンを押していなくても `WM_MOUSEMOVE` のたびにカーソルへ付いて動き、その矩形の全部が背後のアプリへのクリックを吸う。次にどこかをクリックして離すまで戻らない（そのクリックは背後へ届かず、円の色も変わる）。REPORT が「最重要の罠」と書いた「ドラッグ中は透過を外したまま」の作りが、終わりの合図を 1 つしか見ていない。
- **直し方**: `WM_CAPTURECHANGED` を受けて、`dragging.swap(false)` が `true` だったら `state_changed.notify(usize::MAX)` する（`WM_LBUTTONUP` の後半と同じ 2 行）。あるいは `WM_MOUSEMOVE` で `wParam` の `MK_LBUTTON` が落ちていたら同じ片付けをする。どちらも数行足すだけ。
- **直さないと**: 先進坑を手で触った人が、画面全体のクリックが効かない状態に入り、原因が分からない。本坑へ申し送った「ドラッグの罠」の知見にも、この抜けが書かれていない。
- **旗**: (a) 変える（`REPORT.md:80-86` の「追加検証で得た知見（ドラッグ移動・終了）」・`:107` の申し送り。測った結果は書き換えず、書き足しで扱う＝【仮定 3】） ／ (b) 行数が増える（1,006 → 約 1,014。番人は「越えている」のままなので赤にならない。ほかの件で減らす分と合わせて 13 節の 2） ／ (c) 要る（実の画面で、押したまま Alt+Tab して確かめる。判断の分岐を純粋な関数に出せば筋は決定論で縛れる） ／ (d) なし

**2. 32bit の先進坑の helper は、UNLOAD が来ずに上限の 30 秒で抜けても終了コード 0 で終わるので、「go 基準(2)＝clean unload」が起きていなくても合格と出る** （`shiori-host-32/helper_window.rs:249-285`・`:304-330`・`main.rs:374-389`・`:397-420`）
- **何か**: helper は UNLOAD（`want_quit`）か上限（`max_run`＝30 秒）までループを回し、どちらでも `clean_unload()` して `0` を返す。親は UNLOAD を送り（失敗しても「helper は backstop で自律停止」として続け）、helper の終了コードが 0 なら「clean unload → exit code=0 kind=Clean を観測 PASS」と出す。
- **困る場面**: UNLOAD の送りが失敗する（helper の窓が詰まっている・HWND が違う）と、helper は 30 秒の上限で抜けて 0 を返し、親は約 30 秒待った後に「go 基準(2) PASS」と出す。README の go 基準(2)「UNLOAD → `unload` → `FreeLibrary` → 終了コード 0（kind=Clean）を親が観測」は、UNLOAD で抜けたのか上限で抜けたのかを終了コードで見分けられない。`pasta.dll` を読めなかった（`entries=None`）helper も 0 で終わる。
- **直し方**: `run_until_unload_or` が「UNLOAD で抜けたか」を返し、`run_helper` は上限で抜けたら 0 でない値（例 4）を返す。親の判定はそのまま（0 でなければ不成立）で足りる。
- **直さないと**: 次に誰かがこの先進坑で確かめ直したとき、起きていない clean unload を合格と読む。
- **旗**: (a) 変える（`shiori-host-32/README.md:64-66` の go 基準(2) の記録・`:22` の helper の説明） ／ (b) なし ／ (c) 一部（「上限で抜けたら 0 でない」の分岐は決定論で縛れる。本物の往復の効き目は i686 の helper と本物の `pasta.dll` で） ／ (d) なし

**3. 32bit の先進坑の送りの層が、`SendMessageTimeout` の失敗を全部「時間切れ」と呼び、異常系の確かめ err(1) には下限が無いので、時間切れが起きていなくても合格と出る** （`shiori-host-32/ipc.rs:198-203`・`:297-306`・`main.rs:553-581`・`:290-295`）
- **何か**: `send_copydata`・`send_request` は `SendMessageTimeoutW` が 0 を返したら必ず `IpcError::Timeout` を返す（「GetLastError で区別可能だが先進坑では Timeout 既定」）。err(1) は「`Timeout` が返り、経った時間が 3 秒未満かつ 1.5 秒以下なら PASS」と判定する。
- **困る場面**: 宛先の窓が無い・消えた（`ERROR_INVALID_WINDOW_HANDLE`）と、`SendMessageTimeoutW` は待たずに 0 を返す。err(1) はそれを「IPC timeout observed in 0ms … PASS」と出す（下限が無い）。go(1) の失敗の文も、無効な HWND を「再入受領が i686↔x64 で不成立の可能性・named pipe 後退を人間判断」と読ませ、確かめる向きを誤らせる。helper の RESPONSE の送りの失敗も「Timeout」と出る。
- **直し方**: 0 のときに `GetLastError()` を見て `ERROR_TIMEOUT`（1460）なら `Timeout`、ほかは今は使われていない `IpcError::SendFailed` を返す（7 の件で 2 つの関数を 1 つにしてから 1 か所で）。err(1) は「経った時間が `ERR_IPC_TIMEOUT` の近く以上」も見る。
- **直さないと**: 異常系の「ハングしない」の確かめが、送れなかったことを時間切れと取り違えたまま合格を出す。
- **旗**: (a) 変える（`shiori-host-32/README.md:68-69` の「無応答 helper に対し `SendMessageTimeout` が所定 500ms で Timeout を返す」） ／ (b) なし ／ (c) 要らない（無効な HWND へ送れば `SendFailed`・別スレッドの詰まった窓なら `Timeout`、を窓だけの決定論のテストで縛れる。画面も i686 も要らない） ／ (d) なし

**4. 32bit の先進坑の親が、helper の exe の場所を「`HELPER_EXE` が無ければ最初の引数」から取るので、`--selftest-errors` を exe の名前として起こそうとする** （`shiori-host-32/main.rs:62-69`・`:86`・`:593-600`）
- **何か**: `resolve_helper_exe` は環境変数 `HELPER_EXE` が無いと `std::env::args().nth(1)` を helper の exe とみなす。異常系は `--selftest-errors` を引数で渡す。
- **困る場面**: `HELPER_EXE` を付けずに `cargo run -p pilot --example shiori-host-32 -- --selftest-errors` と打つと、err(1) は通った後、err(2) が `--selftest-errors` という名前のプログラムを起こそうとして「helper 起動失敗: program not found」で落ちる。用意した文「HELPER_EXE 未設定かつ argv に helper exe なし」（`:596-598`）は出ない。`-- <exe> --selftest-errors` の順なら動き、`-- --selftest-errors <exe>` の順では動かない。
- **直し方**: `std::env::args().skip(1).find(|a| !a.starts_with("--"))` にする（1 行）。
- **直さないと**: 手順書（README）どおりに `HELPER_EXE` を付ける人は困らないが、付け忘れたときの失敗の文が原因を指さない。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない（引数の並びから exe を選ぶ部分を関数にすれば決定論で縛れる） ／ (d) なし

### 直したほうがよい（Should fix）

**5. 実行例の中のテスト 49 本が、全体テストで走らないどころか組み立てもされない（要件 1.9）** （`crates/pilot/Cargo.toml:39-45`・12 ファイルの `#[cfg(test)]`・`tools/test-all.ps1:75`）
- **何か**: `crates/pilot` の実行例 7 本は、どれも cargo の既定の `test = false`（`cargo metadata` で確かめた）。`cargo test --workspace` は実行例を「組み立てが通るか」だけのために**テストではない普通のプログラムとして**組むので、`#[cfg(test)]` の部分（`observe_tests.rs` の 11 本・`signature_tests.rs` の 2 本ほか、計 49 本）は組み立てにも入らない。README は「数え方の規則の検査: `cargo test -p pilot --example pilot-balloon-asset-swap`」と手で回す形だけを書く。
- **困る場面**: 観測の数え方（`judge_frame`・観測の窓・較正の合否）や、IPC のタグ・HWND の往復のような判断の分岐を誰かが変えても、全体テストは緑のまま。テストのコードが壊れても（型が変わる・関数が消える）気付かない。2026-11-14 の `wintf-winmsg-executor-import` のような依存の付け替えの後に、これらのテストがまだ組み立てられるかは誰も確かめていない（このギャップ分析でも組み立てていない＝12 節の 1）。この spec の要件 4.3「直したバグごとに全体テストで実際に走るテスト」は、この形のままでは満たせない。
- **直し方**: 案は 13 節の 1。いちばん小さいのは、`Cargo.toml` にテストのある実行例 5 本の `[[example]]`（`name`・`path`・`test = true`）を足す形（新しい依存は 0）。ただし `shiori_proxy.rs` の `load_pasta_dll_and_call_load_without_crash` は 32bit の `pasta.dll` を読むので、x64 で走ると必ず赤になる＝`#[cfg(target_arch = "x86")]` で囲むか、helper だけは `test = true` にしない。`process_host.rs` の 4 本は `cmd.exe`・`ping 127.0.0.1` を起こす（約 1 秒・ネットへは出ない）。
- **直さないと**: 先進坑のテストは、書いた時に 1 度回しただけの飾りのまま残る。この spec で足すテストも同じ運命になる。
- **旗**: (a) なし ／ (b) `Cargo.toml` の書き足しだけ（依存は足さない）。全体テストで新しく 49 本前後が走る＝赤が出たら、それは今まで見えていなかった壊れ ／ (c) 要らない（走るテストはどれも純粋な関数か、窓を使わないプロセスの起こし） ／ (d) **⑴⑵ 候補**: `two-tunnel.md` 4.4「機械チェック（… examples 腐敗検出等）の追加を別途依頼できる。本仕様ではこれを実装しない（YAGNI 方針）」と 5.4「テストの厳格さを求めない」・`tech.md`「examples は手動検証とグラフィックス挙動確認の補助であり、テストの代替ではない」。実行例のテストを全体テストへ乗せるのは、この先送りを解くことになりうる

**6. 32bit の先進坑の親が失敗で抜けると、起こした helper を止めずに残し、helper は最大 30 秒 `pasta.dll` を読んだまま動き続ける** （`shiori-host-32/main.rs:129-135`・`:244-263`・`:290-308`・`process_host.rs:39-44`）
- **何か**: `HelperHandle` は `std::process::Child` を持つだけで、`Drop` で子を止めない（std の `Child` は落としても子を止めない）。親は go(1) の途中の失敗（HELLO が来ない・往復の失敗・`Value:` が空）で `Err` を返し、`process::exit(1)` で終わる。
- **困る場面**: helper は `PARENT_HWND` の親が消えても気付かず、上限の 30 秒まで `pasta.dll` を読んだまま残る。すぐに走らせ直すと 2 本目の helper が同じ検体の `pasta.dll` を読む。pasta のゴーストが動いていると `127.0.0.1:9276` を握って次の LOAD を失敗させることがある（記憶「SSP などで pasta のゴーストが起きていると 127.0.0.1:9276 を握り、emo2 の実機テストが LOAD 失敗で赤になる」）ので、1 本目の失敗が 2 本目を「違う」に見せうる。
- **直し方**: `HelperHandle` に `Drop` を足し、まだ動いていれば `kill()` してから `wait()` する（`wait_clean`／`wait_kind` で消費した後は動いていないので何もしない）。数行。
- **直さないと**: 失敗の後の走らせ直しが、前の走行の残りに邪魔される。
- **旗**: (a) なし（成功の道は変わらない） ／ (b) なし ／ (c) 一部（`Drop` で子が止まることは `cmd.exe` の stand-in で決定論に縛れる。本物の helper での効き目は i686 と本物の `pasta.dll` で） ／ (d) なし

**7. 送りの層の `send_request` が、`send_copydata` の中身（`COPYDATASTRUCT` の組み立て・`SendMessageTimeoutW`・失敗の読み）を丸ごと写している** （`shiori-host-32/ipc.rs:166-204` と `:264-307`）
- **何か**: `send_copydata` は片道の送り、`send_request` は「受け皿を空にする → 送る → 受け皿から取る」。`send_request` の真ん中の 25 行は `send_copydata` と同じ。
- **困る場面**: 3 の件の直し（失敗の見分け）を 2 か所に入れる必要があり、片方だけ直すと、HELLO・RESPONSE・UNLOAD と REQUEST で失敗の呼び名がずれる。
- **直し方**: `send_request` は `slot.clear(); send_copydata(...)?; slot.take().ok_or(IpcError::Timeout)` の 3 行にする（約 -20 行）。
- **直さないと**: 送りの決まりを変えるたびに 2 か所を揃える必要が残る。
- **旗**: (a) 変える（README の「共有規約（`ipc.rs`）」が説明する送り方の中身。振る舞いは同じ） ／ (b) なし ／ (c) 一部（3 と同じテストで縛れる。往復は本物で） ／ (d) ⑴（5.4）

**8. 受け取った `WM_COPYDATA` から中身を取り出す `copydata_payload` が、親と helper の 2 ファイルに 1 字違わず写してある** （`shiori-host-32/parent_window.rs:74-90`・`helper_window.rs:100-117`）
- **何か**: どちらも `dwData` の下 32 ビットをタグに、`cbData` バイトを `Vec` に写す。`ipc.rs` は親にも helper にも `#[path]` で取り込まれる「跨ぐ決まりの唯一の置き場」と書かれている（`ipc.rs:3-6`）のに、受け側の読み方だけがその外に 2 つある。
- **困る場面**: タグの載せ方（下 32 ビット）を変えると、2 か所を揃える必要がある。片方だけ直すと、x64 と i686 で読み方がずれる（この先進坑がいちばん避けたかったずれ）。
- **直し方**: `ipc.rs` へ 1 つ移し、2 ファイルからは消す（約 -18 行）。
- **直さないと**: 決まりの唯一の置き場が、受け側では唯一でないまま。
- **旗**: (a) なし ／ (b) なし ／ (c) 一部（取り出しは決定論のテストで縛れる。往復は本物で） ／ (d) ⑴（5.4）

**9. 「別のスレッドから自分の窓へ 50〜100 ms ごとに `WM_NULL` を送ってループを起こし、条件か期限で抜ける」仕組みが 3 か所に写してある** （`shiori-host-32/parent_window.rs:181-216`・`helper_window.rs:249-285`・`main.rs:497-551`）
- **何か**: 親の HELLO 待ち・helper の UNLOAD 待ち・err(1) の詰まった窓の 3 か所が、それぞれスレッドを起こし、HWND を u32 の 4 バイトへ符号化してから戻し（`HWND` が `Send` でないため）、`PostMessageW(WM_NULL)` を撃つ。
- **困る場面**: 3 か所で間隔（50 ms・100 ms・1 回だけ）と止め方がばらばらで、期限の判定の直しを 1 か所に入れても他は変わらない。HWND を 4 バイトにして戻す回り道も 3 回写っている。
- **直し方**: `ipc.rs` に「`hwnd` の窓で、`pred()` が真になるか `deadline` まで回す」関数を 1 つ置き、2 か所（親・helper）はそれを呼ぶ。スレッドの代わりに窓へ `SetTimer` を掛ければ、ループが自分で `WM_TIMER` を受けて起きるので、起こし役のスレッドそのものが要らない（どちらにするかは設計で）。err(1) の詰まった窓は止めるための 1 回の `PostMessageW` だけなので、そのままでよい。
- **直さないと**: 待ちの直しが 1 か所に入らない。
- **旗**: (a) なし ／ (b) なし ／ (c) 一部（期限で抜けることは窓だけの決定論のテストで縛れる。往復は本物で） ／ (d) ⑴（5.4）

**10. 32bit の先進坑の 6 ファイルの頭の `#![allow(dead_code)]` と、その理由にした「本タスク・後続タスクで配線する」の注記が古い** （`ipc.rs:8-15`・`:150`・`:160-161`・`shiori3.rs:116`・`parent_window.rs:16`・`process_host.rs:12-21`・`:38`・`:80-82`・`helper_window.rs:9-13`・`:31`・`shiori_proxy.rs:30`・`main.rs:1-10`・`:14`・`:19-20`・`:25`・`helper.rs:213-224`・`:252-253`・`:299-303`・`:312-317`）
- **何か**: 「本タスク（1.1）は scaffolding のみ」「応答待ちは後続タスク 4.1 の範囲であり本モジュールには含めない」「helper スケルトンは現状これらを無視する」「`helper_hwnd` は常に `None`（確定は task 4.1）」などの注記が、配線し終えた後も残っている（同じ `ipc.rs` に応答待ちの `send_request` がある・helper は引数を読む）。`#![allow(dead_code)]` はその注記を理由に、ファイルの全部の未使用の警告を消している。`shiori_proxy.rs:30` の 1 か所は理由も書いていない。
- **困る場面**: 11〜14 の件の死んだコードは、この 6 つの `allow` のせいでコンパイラが 1 つも教えない。注記を信じた人は、動いているコードを「まだ骨組み」と読む。
- **直し方**: 6 つの `#![allow(dead_code)]` を消し、コンパイラが挙げる未使用を 11〜14 の件のとおり消す（消せないものは無い見込み）。古い「本タスク・後続タスク」の注記を消すか、今の姿に書き直す。
- **直さないと**: 死んだコードが黙って増え続ける。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない（コンパイルが通ることで確かめる。ただし helper の i686 の組み立ては全体テストに無い＝12 節の 2） ／ (d) ⑴（5.4）。⑵ には当たらない（理由が今は本当でない）

**11. 送りの層 `ipc.rs` に、使われていない・同じことの言い換えだけの物が残っている** （`ipc.rs:41-42`・`:72-85`・`:91-101`・`:119-126`・`:144-147`・`:220-230`・`:381-385`・`helper_window.rs:175-178`・`main.rs:73`・`helper.rs:317`）
- **何か**: ⑴ `MsgTag::Load`＝誰も送らず、helper は受けても何もしない分岐だけを持つ。⑵ `From<MsgTag> for u32`・`TryFrom<u32> for MsgTag`＝テストからしか呼ばれない（本番は `as_u32`・`try_from_u32`）。⑶ `PAYLOAD_HEADER_LEN = 0`＝「構造を持たない」ことの宣言の印で、読むのはそれ自身のテスト（`assert_eq!(PAYLOAD_HEADER_LEN, 0)`）だけ。⑷ `IpcError::PeerGone`・`SendFailed`＝作られない（`SendFailed` は 3 の直しで使う）。⑸ `decode_hwnd_le`＝`u32::from_le_bytes` をそのまま呼ぶだけ。⑹ `ResponseSlot` は `#[derive(Default)]` と同じ中身の `new()` を両方持つ。⑺ `let _ = ipc::DEFAULT_TIMEOUT;`（親と helper の `main`）＝「取り込まれていることの確認」で、何もしない。
- **困る場面**: 読む人が `Load` の道や `PeerGone` の扱いを探して時間を使う。`From`・`TryFrom` と `as_u32`・`try_from_u32` の 2 組のどちらを使うかが揺れる。
- **直し方**: ⑴〜⑺ を消す（`PeerGone` を消し、`SendFailed` は 3 で使う。`Load` の分岐は `Ok(other)` の「想定外」へ落ちる）。約 -50 行。
- **直さないと**: 決まりの置き場に、決まりでない物が混ざったまま。
- **旗**: (a) 一部（`README.md:23` の `MsgTag` の説明は種類を挙げていないので書き換え不要。測った結果には関わらない） ／ (b) なし ／ (c) 要らない ／ (d) ⑴（5.4）

**12. 子のプロセスの見張り `process_host.rs` に、使われない欄と、同じことをする関数の組が残っている** （`process_host.rs:39-44`・`:97-100`・`:103-120`・`:132-143`・`main.rs:397-400`）
- **何か**: ⑴ `HelperHandle.helper_hwnd`＝いつも `None` で、誰も読まない。⑵ `poll_exit`＝テストからしか呼ばれず、終了コードの無い終わり方を `-1` にする（本番は `poll_exit_kind`）。⑶ `wait_clean`（コードを返す）と `wait_kind`（分類を返す）が同じ `wait()` の 2 つの言い換えで、親は `wait_clean` の後に自分で `ExitKind::classify` を呼んでいる。
- **困る場面**: `poll_exit` の `-1` と `poll_exit_kind` の `Terminated` が、同じ「コードが無い終わり方」を違う形で返す。どちらを信じるかで読みが変わる。
- **直し方**: ⑴⑵ を消し、⑶ は `wait_kind` 1 つにする（親は `Abnormal(c)` からコードを出す）。テスト 4 本のうち `poll_exit` を見る 1 本は `poll_exit_kind` で書き直す。約 -40 行。
- **直さないと**: 同じ問いに 2 つの答えの形が残る。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない（`cmd.exe` の stand-in のテストのまま） ／ (d) ⑴（5.4）

**13. 親と helper の窓の状態に、書くだけで誰も読まない数え・欄・状態が残っている** （`parent_window.rs:40-45`・`:59-62`・`:111`・`:113`・`:124`・`:130`・`:135`・`:162-164`・`helper_window.rs:47-54`・`:68-77`・`:90`・`:136-137`・`:155`・`:158`・`:171`・`:186`・`:231`・`:296`・`:417-419`・`:429-432`）
- **何か**: 親の `hellos`・`responses`・`unexpected`、helper の `requests_handled`・`responses_sent`・`unknown_tags` は足すだけで読まれない（ログにも出ない）。`ParentShared::helper_hwnd()`・`ParentMessageWindow::hwnd()` は呼ばれない。`HelperShared.reply_timeout` はいつも `ipc::DEFAULT_TIMEOUT`。`HelperState`（4 つの状態）は書くだけで、読むのは selftest の「`clean_unload` の後は `CleanExit`」の 1 か所＝`clean_unload` が自分で入れた値を確かめているだけ。`helper_window::default_fixture_ghostdir` は helper の `main` が同じ名前の自分の関数を使うので呼ばれない。
- **困る場面**: 状態機械（`Started → Pumping → Unloading → CleanExit`）が判定に使われているように見え、読む人が分岐を探す。数えは観測に使われていないのに「観測カウンタ」と書いてある。
- **直し方**: 上の欄・関数・`HelperState` を消す（selftest の最後の確かめは「`clean_unload` から戻った」で足りる）。約 -60 行。
- **直さないと**: 使われない状態が、使われている状態と見分けにくいまま。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない ／ (d) ⑴（5.4）

**14. SHIORI の組み立て `shiori3.rs` に、取り込みの確認だけの関数と、使わない引数が残っている** （`shiori3.rs:120-123`・`:134-141`・`main.rs:74`・`:268`・`:662`）
- **何か**: `module_loaded()` は `true` を返すだけで、親の `main` が `let _ =` で呼ぶ。`build_onboot(ghostdir)` は引数を捨てる（「シグネチャ整合のため保持」）ので、生存後の追加の要求は `Path::new("")` を渡している。
- **困る場面**: `build_onboot` に ghostdir を渡す呼び手は、要求に ghostdir が載ると読み違える（載らない）。
- **直し方**: `module_loaded` を消し、`build_onboot()` を引数なしにする。約 -15 行。
- **直さないと**: 使わない引数が、使っているように見えるまま。
- **旗**: (a) なし（要求のバイト列は同じ） ／ (b) なし ／ (c) 要らない ／ (d) ⑴（5.4）。⑶ には当たらない（SHIORI/3.0 の要求の形は変えない）

**15. クリック透過の先進坑の後片付けの「適用係を止める旗と起こし」が、何もしていない** （`pilot-clickthrough-alpha-toggle/main.rs:697`・`:711-713`・`:868-876`・`:881`・`:895-904`）
- **何か**: `block_on` から戻った後に `applier_done` を立てて `state_changed` を鳴らし、「applier の pending な listener.await を即起床させる … 明示フラグ＋notify で確実にループを畳む（drop 任せにしない）」と書いてある。
- **困る場面**: `wintf-winmsg-executor` の `block_on` は「`block_on()` が戻った後は、spawn したタスクは全部止まる」（`crates/wintf-winmsg-executor/src/lib.rs:121`）。旗を立てて鳴らしても誰もタスクを進めないので、ループの頭の `applied_done` の確かめは 1 度も真にならない。プロセスが正常に終わる（REPORT の T8）のは、この旗のおかげではない。注記を信じた人は、ほかの所でも「旗＋起こしで後片付けできる」と読み違える。
- **直し方**: 旗・引数・確かめ・後片付けの 2 行と注記を消す（約 -16 行）。
- **直さないと**: 効かない後片付けが「唯一の確定 shutdown 経路」として残る。
- **旗**: (a) 変える（`REPORT.md:67` の T8「close→done→join→`DestroyWindow` の単一経路」のもとのコード。適用係の止め方は記録に書かれていないので、書き足しで足りる） ／ (b) **行数が 1,006 → 約 990 になり、1,000 行の番人が赤になる**（13 節の 2） ／ (c) 一部（終わり方は実の画面で 1 度走らせれば確かめられる。消す前後で振る舞いは変わらない見込み） ／ (d) ⑴（5.4）

**16. クリック透過の先進坑が、押した点が円の中かを `alpha_is_opaque` を使わずに書き直している** （`pilot-clickthrough-alpha-toggle/main.rs:282-293` と `:79-91`・`:57-62`）
- **何か**: `RADIUS` の説明は「円中心の算出（窓矩形中心）と半径を本定数経由で同一にすること。値や中心算出をここ以外で重複定義しない」と書く。`WM_LBUTTONDOWN` は押した点の内外の判定（中心 `cr.right / 2`・二乗の比較）を自分で 6 行で書いている。
- **困る場面**: 境の扱い（`<=`）や中心の出し方を `alpha_is_opaque` だけで直すと、受け取りのログの「円内（正常受領）／円外（!! 透過漏れ !!）」が別の円で判定され、REPORT の T2・T3 の証跡の読みがずれる。
- **直し方**: `let inside = alpha_is_opaque(POINT { x: click_x, y: click_y }, cr);`（`cr` はクライアントの矩形で左上が 0 なので、中心は同じ）。約 -5 行。
- **直さないと**: 判定の唯一の置き場の約束が、自分のファイルの中で破れたまま。
- **旗**: (a) 変える（`REPORT.md:62-63` の T2・T3 の証跡のログを出すコード。判定の結果は同じ） ／ (b) 行数が減る（-5。単独なら 1,001 で番人は緑。15 と合わせると赤＝13 節の 2） ／ (c) 要らない（`alpha_is_opaque` には既に 7 本のテストがある＝ただし 5 の件で走っていない） ／ (d) ⑴（5.4）

**17. クリック透過の先進坑の注記が、この先進坑が測って見つけた 2 つの知見と逆のことを書いている** （`pilot-clickthrough-alpha-toggle/main.rs:411-412`・`:416-419`・`:797-802`・`:259-262`・`:3-5`・`:77-78`・`:168-172`・`:849`）
- **何か**: ⑴ `build_pipeline` の説明は「`WS_EX_LAYERED` は使わない」（`:411`）と書くが、窓は `WS_EX_LAYERED` を「同伴フラグ」として立てていて、それが成立の必須条件だった（README・REPORT の最重要の知見）。⑵ 同じ関数の中の注記は「DComp の CreateTargetForHwnd は**クライアント領域**に合成する」（`:416-418`）と書き、同じファイルの `:797-802` と REPORT は「**ウィンドウ原点**から合成」（だから枠なし窓が要る）と書く。⑶ `WM_SETCURSOR` の注記は「手カーソルにする」だが、コードは十字（`IDC_CROSS`）で、REPORT も十字と書く。⑷ 「REPORT.md はタスク 6.1 で作成する（本ファイルは骨組みのみ）」（`:5`）・「色トグル … 後続で配線される」（`:168-172`）・「消費側 StateApplier は 4.2 で配線」（`:849`）は済んだことを未来形で書く。⑸ `alpha_is_opaque` の説明は「窓矩形」を渡すと書くが、見張り役が渡すのはクライアントの矩形（`:597-623`）。
- **困る場面**: 本坑は README を読むことになっているが、このコードを開いた人（後の先進坑の書き手・本坑の設計者）は、測って否定された「LAYERED 無し」「クライアント領域に合成」を事実として読む。
- **直し方**: ⑴⑵⑶⑸ の行を今の事実に書き直し、⑷ の未来形を消すか過去形にする。行を増やさずに書き換えれば行数は変わらない。
- **直さないと**: 一次記録と逆のことがコードに残る。
- **旗**: (a) なし（注記だけ。README・REPORT とは揃う方向） ／ (b) 行を消すと番人に近づく（書き換えなら 0） ／ (c) 要らない ／ (d) ⑴（5.4）

### できれば（Nice to have）

**18. 32bit の proxy の `shiori_unload` は、いつも `Ok` の `Result` を返し、説明は「続けて FreeLibrary する」と書くがしない。i686 のテストが組めないという理由の注記も古い** （`shiori_proxy.rs:151-158`・`:223-228`・`:254-255`・`:79-92`・`helper.rs:252-253`）
- **何か**: `shiori_unload` は `unload()` の答えを捨てて必ず `Ok(())` を返す。`FreeLibrary` は `ShioriEntries` の `Drop` がする。`selftest_load` とテストの説明は「`ipc.rs` の `#[cfg(test)]` が i686 でビルド不能（`usize >> 32` overflow）」と書くが、`ipc.rs` のテストは既に `u64` で書き直されている（`ipc.rs:329-331`）。`load_dll` は `GetProcAddress` が 1 つでも引けないと `?` で戻り、読んだ DLL を `FreeLibrary` しない。
- **困る場面**: 呼び手が `shiori_unload` の `Err` を扱う道を書くが、来ない。`--selftest-load` を残す理由が「テストが組めないから」と読めるが、今は組める。`request` を出していない DLL を読むと、その DLL は helper の終わりまで載ったまま。
- **直し方**: `shiori_unload` は `()` を返し、説明から `FreeLibrary` を消す。古い理由の注記を消す。`load_dll` は `ShioriEntries` を組む前の失敗で `FreeLibrary(module)` する（または `module` を先に包む）。
- **直さないと**: 説明と振る舞いのずれが残る（実害は小さい）。
- **旗**: (a) なし ／ (b) なし ／ (c) 一部（`load_dll` の片付けは本物の DLL でしか起きない。ほかは要らない） ／ (d) ⑴（5.4）

**19. `parse_value` が `\r\n` で割った後にさらに `\n` で割っている** （`shiori3.rs:161`）
- **何か**: `text.split("\r\n").flat_map(|l| l.split('\n'))`。
- **困る場面**: 無い（結果は `text.lines()` と同じ）。読む人が 2 段の割り方の訳を探す。
- **直し方**: `text.lines()`。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない（既存の 3 本のテスト） ／ (d) ⑴（5.4）。⑶ には当たらない（応答の読み方は同じ）

**20. 親が helper へ ghostdir と親の HWND を、引数と環境変数の両方で渡し、helper は両方を順に読む** （`process_host.rs:84-96`・`helper.rs:320-340`）
- **何か**: `spawn` は `argv[1]=ghostdir`・`argv[2]=parent_hwnd` と `GHOSTDIR`・`PARENT_HWND` を両方付ける。helper は環境変数を先に、無ければ引数を読む。
- **困る場面**: 同じ値の通り道が 2 つあり、片方だけ変えると、どちらが効いているかが分かりにくい。開発者の端末に `PARENT_HWND` が残っていると、引数を渡しても環境変数が勝つ。
- **直し方**: どちらか 1 つにする（設計で選ぶ）。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない ／ (d) ⑴（5.4）

**21. 32bit の先進坑の説明が、コードとずれている（3 か所）** （`helper_window.rs:121-122`・`:254`・`ipc.rs:163-165`・`:261-263`）
- **何か**: ⑴ `handle_message` の説明は「sender が有効ならそれを優先する」と書くが、コードは送り主（`wParam`）を見ずに `parent_hwnd` へ返す。⑵ 「别スレッド」（`别` は中国語の字）。⑶ `send_copydata`・`send_request` は `unsafe` でない関数なのに `# Safety` の節を持つ（呼び手が守るべき約束が無い）。
- **直し方**: ⑴ の文を消すか今の振る舞いに直す・⑵ を「別」に・⑶ の節を普通の説明にする。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない ／ (d) ⑴（5.4）

**22. バルーン差し替えの先進坑の `observe.rs` が、3 つの欄のためにファイル全部の未使用の警告を消している** （`pilot-balloon-asset-swap/observe.rs:5`・`:33-36`）
- **何か**: `#![allow(dead_code)] // TickRecord の h_p・h_q・h_both（Debug 出力のログでしか読まない）`。
- **困る場面**: 理由は本当だが、同じファイル（852 行）のほかの物が使われなくなっても、コンパイラは教えない。
- **直し方**: `allow` を 3 つの欄だけに付け替える（`#[allow(dead_code)]` を欄に、またはログの行で欄を読む）。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない ／ (d) **⑵ 候補**（実在の理由を書いた `#[allow(dead_code)]`。ただし語彙の口ではない）・⑴（5.4）

**23. バルーン差し替えの先進坑のテストで、説明が別のテストに付いている** （`pilot-balloon-asset-swap/observe_tests.rs:426-428`）
- **何か**: 「当たり判定の閾値の境（0.9 と 0.1 は含む）」の説明が、すぐ下の `tick_rect_takes_the_move_queued_in_the_same_tick_only` に付き、その下の `hit_rule_thresholds_are_inclusive` には説明が無い。
- **直し方**: 説明の行を 1 つ下のテストへ移す。
- **旗**: (a) なし ／ (b) なし ／ (c) 要らない ／ (d) ⑴（5.4）

**24. `crates/pilot/Cargo.toml` の注記が「`[dependencies]` は空」と書き、`event-listener` だけが木の依存の表を使わずに版を書いている** （`Cargo.toml:12-15`・`:20`・根の `Cargo.toml:28`）
- **何か**: 「[dependencies] は空（最小依存）」の直後に依存が 4 つ並ぶ。`event-listener = "5"` は根の `[workspace.dependencies]` にも `event-listener = "5"` がある（ほかの依存は `{ workspace = true }`）。
- **困る場面**: 根の `event-listener` の版を上げると、`pilot` だけが古い版の範囲に残りうる（同じ版を 2 か所で持つ）。
- **直し方**: 注記を今の姿に直し、`event-listener = { workspace = true }` にする（新しい依存は 0・`Cargo.lock` は変わらない見込み）。
- **旗**: (a) なし ／ (b) 依存は足さない（書き方だけ） ／ (c) 要らない ／ (d) ⑴（5.4）

**25. `wintf-winmsg-executor` の先進坑の README の「概要」が今のコードとずれている** （`wintf-winmsg-executor/README.md:20`・`:39-42`・`main.rs:148`）
- **何か**: 「実行法: … wall-clock 約 6 秒で自動終了」だが `RUN_SECS = 8`。「`main.rs`・約 240 行」だが 353 行。「依存: `wintf-winmsg-executor = "0.0.5"`」だが、今は木の中の写しを `{ workspace = true }` で使う（`Cargo.toml:17-19`）。
- **困る場面**: 走らせた人が 8 秒の終わりを「6 秒を越えた＝おかしい」と読む。
- **直し方**: 「概要」の 3 か所を今の値に直す（「検証結果」の測った値＝6 秒・8 秒の走行は書き換えない）。
- **旗**: (a) 変える（README の「概要」。「検証結果」は変えない） ／ (b) なし ／ (c) 要らない ／ (d) なし

**26. `crates/pilot/README.md` の置き方の決まり「各フォルダは `main.rs` 必須」に、11-17 に着地した独立のワークスペースの例外が書かれていない** （`README.md:105-122`）
- **何か**: 「1 仕様 = 1 フォルダ・`main.rs` 必須（無いと Cargo が example として認識しない）」と書く。`examples/pilot-windows-rs-next/` は `main.rs` を持たず、自分の `Cargo.toml`（`[workspace]`・17 メンバー）と `Cargo.lock` を持つ。
- **困る場面**: 次の先進坑の書き手が、独立のワークスペースを置いてよいのか、置いたら全体テストに乗らないのかを、この README からは読めない。
- **直し方**: 「本線から独立したワークスペースを置く例外（`pilot-windows-rs-next`・全体テストに乗らない）」の 1〜2 行を足す。
- **旗**: (a) なし（crate の README で、先進坑の一次記録ではない） ／ (b) なし ／ (c) 要らない ／ (d) なし。範囲から外す置き場（【仮定 1】）を説明するだけで、その中には触らない

**27. クリック透過の先進坑の `DCompPipeline` は、持ち続けるだけの欄のために構造体ごと未使用の警告を消している** （`pilot-clickthrough-alpha-toggle/main.rs:144-163`）
- **何か**: `#[allow(dead_code)] // d3d/d2d_device/target/visual は保持専用（ドロップ抑止）`。
- **困る場面**: 理由は本当だが、`width`・`height` のような読んでいる欄が使われなくなっても教えない。
- **直し方**: 持ち続けるだけの欄の名前を `_d3d` のように `_` で始め、`allow` を消す（行数は変わらない）。
- **旗**: (a) なし ／ (b) 行数は変わらない ／ (c) 要らない ／ (d) **⑵ 候補**（実在の理由を書いた `#[allow(dead_code)]`）・⑴（5.4）

## 6. 指摘の数と旗の集計

| 組 | 件数 | 番号 |
|---|---:|---|
| 必ず直す | 4 | 1〜4 |
| 直したほうがよい | 13 | 5〜17 |
| できれば | 10 | 18〜27 |
| 計 | 27 | |

| 先進坑ごと | 件数 | 番号 |
|---|---:|---|
| `shiori-host-32` | 16 | 2・3・4・6・7・8・9・10・11・12・13・14・18・19・20・21 |
| `pilot-clickthrough-alpha-toggle` | 5 | 1・15・16・17・27（5 の一部もここのテスト） |
| `pilot-balloon-asset-swap` | 2 | 22・23 |
| `wintf-winmsg-executor` | 1 | 25 |
| `crates/pilot` 全体（`Cargo.toml`・README・テストの形） | 3 | 5・24・26 |
| `pilot-dropfiles-on-wuc-window`・`_template`・`lib.rs` | 0 | （見たが指摘は無い＝9 節） |

| 旗 | 件数 | 番号 |
|---|---:|---|
| (a) 一次記録（README・REPORT に書いた測った結果のもとのコード）を変える | 8 | 1（REPORT のドラッグの知見）・2（shiori README の go 基準(2)）・3（同 異常系）・7（同 共有規約の説明）・11（一部）・15（REPORT の T8）・16（REPORT の T2・T3 のログ）・25（README の「概要」だけ） |
| (b) ウェーブの約束: `main.rs` の行数（1,000 行の番人） | 5 | 1（+約 8）・15（-約 16）・16（-5）・17（書き換えなら 0）・27（0）。**1・16 だけなら 1,009 で緑、15 を入れると赤** |
| (b) ウェーブの約束: 依存・`sample-ghost-kit` の公開の形・`main.rs` を割る | 0 | 5・24 は `Cargo.toml` を書き換えるが依存は足さない |
| (c) 実の画面・実 GPU が要る | 1（一部 1） | 1（押したまま Alt+Tab）・15（終わり方の確かめを実機でするなら） |
| (c) i686 の helper と本物の `pasta.dll` が要る | 一部 5 | 2・6・7〜9（往復の効き目。筋は決定論で縛れる） |
| (c) 窓は使うが画面も i686 も要らない（決定論のテストにできる） | 3 | 3・8・9 |
| (d) ⑴ 開発者の裁定（`two-tunnel.md` 4.4 の先送り・5.4 の品質の基準） | 20（4.4 は 5 の 1 件・5.4 は 20 件で 5 を含む） | 5・7〜24・27（5.4 は要件 2.7 の【仮定 2】で扱う） |
| (d) ⑵ わざと置いた先送りの印 | 2 候補 | 22・27（実在の理由を書いた `allow`。語彙の口ではない）。`dropfiles.rs:168` の `ponytail:` は限界を越えていないので指摘にしていない |
| (d) ⑶ SSP・ukadoc の互換 | 0 | 14・19 は SHIORI/3.0 の要求・応答の形を変えない |

## 7. 要件 1.9 の確かめ（実行例のテストは全体テストで走るか）

- **答え: 1 本も走らない。組み立てもされない。**
- 根拠: ⑴ `cargo metadata --no-deps` で `pilot` の実行例 7 本（`_template`・`pilot-balloon-asset-swap`・`pilot-clickthrough-alpha-toggle`・`pilot-dropfiles-on-wuc-window`・`shiori-host-32`・`shiori-host-32-helper`・`wintf-winmsg-executor`）が全部 `test: false`・`doctest: false`。`lib` だけが `test: true`（中身は空）。⑵ 全体テストの x64 の段は `cargo test --workspace --no-fail-fast -j 4`（`tools/test-all.ps1:75`）で、`--examples` も `--all-targets` も付けない。cargo は `test = false` の実行例を「組み立てが通るか」だけのために普通のプログラムとして組むので、`#[cfg(test)]` の部分は外れる。⑶ i686 の段は `shiori-host32-helper`・`shiori-host32-ipc` の 2 クレートだけで、`pilot` を含まない（`:77`）。
- 範囲のテストごとの答え:

| ファイル | 本数 | 全体テスト | 手で回すなら |
|---|---:|---|---|
| `pilot-balloon-asset-swap/{main,capture,swap}.rs`・`observe_tests.rs`・`signature_tests.rs` | 22 | 組み立てない・走らない | `cargo test -p pilot --example pilot-balloon-asset-swap`（README に書いてある） |
| `pilot-clickthrough-alpha-toggle/main.rs` | 7 | 同上 | `cargo test -p pilot --example pilot-clickthrough-alpha-toggle`（README に無い） |
| `pilot-dropfiles-on-wuc-window/{main,dropfiles}.rs` | 6 | 同上 | `cargo test -p pilot --example pilot-dropfiles-on-wuc-window`（README に書いてある） |
| `shiori-host-32/{ipc,process_host,shiori3}.rs` | 12 | 同上 | `cargo test -p pilot --example shiori-host-32`（`ipc.rs` は helper にも入るので `--target i686-pc-windows-msvc --example shiori-host-32-helper` でも） |
| `shiori-host-32/shiori_proxy.rs` | 2 | 同上 | `--target i686-pc-windows-msvc --example shiori-host-32-helper` だけ（`load_pasta_dll_and_call_load_without_crash` は 32bit の `pasta.dll` と検体 emo2 を読むので、x64 では必ず赤） |

- 今の 49 本が今も組み立てられるかは、このギャップ分析では確かめていない（12 節の 1）。
- この形は 5 の件として挙げた。直すかどうかと直し方は 13 節の 1。

## 8. 範囲から外した置き場（`examples/pilot-windows-rs-next/`）を入れた場合

audit はかけていない（【仮定 1】）。入れた場合に変わることだけを書く。

- 大きさ: git に載ったファイル **104**（`.rs` 61 本 6,403 行・`Cargo.toml` 19・`Cargo.lock` 1・文書 3〔`README.md`・`REWRITE_TABLE.md`・`UPSTREAM_DRAFT.md`〕・証跡の画像 5・`.txt` 14・`.gitkeep` 1）。今の範囲（31 ファイル・`.rs` 8,081 行）の約 3 倍のファイル数・`.rs` で約 0.8 倍の行数。いちばん大きい `.rs` は 501 行（`q7-vertical/src/bin/vt-balloon.rs`）。
- 本線の全体テストとの関係: 自分の `Cargo.toml` に `[workspace]`（17 メンバー・`resolver = "3"`）を持つ独立のワークスペースで、根の `members = ["crates/*"]` には入らない＝`tools/test-all.ps1` の `cargo test --workspace` では組まれも走りもしない。直しを全体テストで確かめられない（要件 4.3・4.4 を満たす道が無い）。windows-rs を上流の git の 1 つの rev に固定しているので、組むたびにその rev を取りに行く。
- 番人との関係: `log-capture-kit` の番人（1,000 行・検体の置き場・一時フォルダほか）は `crates/**/*.rs` を全部歩くので、この置き場の 61 本も既に見張られている（1,000 行を越えるものは無い）。
- 指摘の数の見込み: 今の範囲の 27 件から見て、20 件を越えやすい。越えたら要件 2.6 のとおり、この置き場を同じ範囲の 2 本目の spec に切る。

## 9. 見たが指摘にしなかったもの（「Not checked」ではない・全部読んだ）

- **`pilot-dropfiles-on-wuc-window` の受け口の `Box` を解放しない**（`dropfiles.rs:168-170`）: `ponytail:` の注釈が限界（窓 1 枚）を書き、窓は 1 枚しか作らない＝作法どおり決めごととして扱う。
- **`pilot-balloon-asset-swap/main.rs` と `pilot-dropfiles-on-wuc-window/main.rs` の起動・上限時間・終了コードの骨組みが似ている**: 後者は前者から写したと書いてある。2 つは独立の先進坑で、一緒に変える必要が無い（ponytail の「merge」の条件「一緒に変えねばならない写し」に当たらない）。まとめると 1 仕様 = 1 フォルダの独立（`crates/pilot/README.md`「並列安全」）を崩す。
- **`shiori-host-32/sample_support.rs` の `static LazyLock<SampleRoot>` が落とされない**（静的な値は `Drop` されない・`process::exit` でも同じ）: 複製の木と生存の札が残るが、次の `acquire` の掃除が「消せた札＝持ち主の居ない木」として片付ける（`crates/sample-ghost-kit/src/devroot.rs:43-55`）。溜まらない。
- **上限時間の環境変数 `AREKA_APP_SMOKE_EXIT_MS=0` が即時の打ち切りになる**: 本体の `smoke_exit_ms_from` も `"0"` を 0 ms として受け取る（`crates/areka/src/smoke_exit.rs:46-58`）＝同じ規則。
- **観測と取り込みの記録（`Shared::ticks`・`frames`）が増えるだけ**: 走行は上限 90 秒（実測 1,340 tick・1,430 フレーム前後）で終わるので、想定した負荷では問題にならない。
- **`alpha_is_opaque` が足し算と引き算を `i32` でしてから `i64` へ写す**: 画面の座標（±数万）では溢れない。溢れる座標は想定した使い方に無い。
- **`WM_COPYDATA` の送り主を確かめない**（親・helper とも）: 同じ机の他のプロセスが HELLO や RESPONSE を偽れるが、開発者が 1 分ほど手で走らせる先進坑で、守る境が無い。
- **窓の生成の `.expect`**（クリック透過・`wintf-winmsg-executor`）: 窓が作れなければ先に進めない致命の失敗で、決まり「panic は致命に限る」の内。
- **`_template`・`lib.rs`**: 中身は雛形と空の lib だけ。

## 10. 要件と今の資産の対応

| 要件 | 今の資産 | 隙間 |
|---|---|---|
| 1.1〜1.9 洗い出し | この文書 | 無し（7 節の組み立ての確かめだけ残る＝12 節の 1） |
| 2.1〜2.7 対象の確定 | 6 節の (d) の欄 | ⑴ の候補 20 件（5.4 が 20・うち 5 は 4.4 にも当たる）・⑵ の候補 2 件の裁定が要る（13 節） |
| 3.1 全部直す | — | 27 件の直し（Missing） |
| 3.2 範囲の外の読み手 | 4 節の表 | `file_length_guard_test.rs`（行数）・`workspace_scan_test.rs`（在ること）は名前と行数を守れば触らない（Constraint） |
| 3.3 消す前の確かめ | 11〜14 の件は範囲の中で grep 済み。範囲の外に呼び手は無い（葉の隔離） | i686 の helper の組み立てで最後に確かめる（12 節の 2） |
| 4.1・4.2 かけ直し | — | 実装の最後（Missing） |
| 4.3 全体テストで走るテスト | 範囲の 49 本は走らない | **5 の件の裁定しだい**（13 節の 1・Unknown） |
| 4.4 全体テスト | `tools/test-all.ps1` | 無し |
| 4.5 画面の操作は排他テストの机で | 机の道具 | 1 の件の確かめ（Constraint） |
| 5.1・5.2 葉の隔離・本坑へ写さない | 空の lib・実行例だけの形 | 5 の案 A は実行例のままテストを走らせるだけ＝形を崩さない |
| 5.3・5.4 一次記録 | 各 README・REPORT | 書き足しの形（【仮定 3】）の裁定 |
| 6.1 1,000 行 | 番人 | `swap.rs` 928・`observe.rs` 852 には触る件が無い（22 は 1 行の付け替え） |
| 6.2〜6.7 決まり | — | 1〜4 の直しは `eprintln!`／`println!`（この先進坑の記録の形）で失敗を残す。新しい一時フォルダは作らない |
| 7.1〜7.6 ウェーブの約束 | 6 節の (b) | `main.rs` の行数（13 節の 2・Constraint） |

## 11. 直し方の選択肢・規模とタスクの見立て

### 案 A: 今のファイルの中で直す（テストは各ファイルの `#[cfg(test)]` に足し、実行例のまま全体テストに乗せる）

- 1〜4・6〜24・27 を今のファイルの中で直し、各バグの直しには純粋な関数に出した判断の分岐のテストを隣に置く（例: 1＝「`WM_CAPTURECHANGED` を受けたらドラッグを終える」の判断・2＝「UNLOAD で抜けたか」から終了コードを出す関数・3＝無効な HWND へ送ると `SendFailed`・4＝引数の並びから exe を選ぶ関数・6＝`Drop` で子が止まる）。
- 5 は `crates/pilot/Cargo.toml` に `[[example]] … test = true` を 5 本足し、`shiori_proxy.rs` の本物の DLL のテストを `#[cfg(target_arch = "x86")]` で囲む。
- ✅ 新しいファイル・依存・道具の段が 0。葉の隔離（空の lib・実行例だけ）を崩さない。
- ❌ 全体テストに今まで組んでいなかった 49 本が乗る＝組み立てが通らないものがあれば、それも直す仕事が増える（12 節の 1）。`process_host.rs` のテストが `cmd.exe`・`ping` を起こす（約 1 秒）。

### 案 B: テストは手で回すまま（全体テストを変えない）

- 5 を直さない（⑴⑵ の裁定で残す）。各バグのテストは足すが、`cargo test -p pilot --example <名前>` で手で回す。
- ✅ 全体テストの形も時間も変わらない。`two-tunnel.md` 4.4 の先送りをそのまま守る。
- ❌ 要件 4.3「全体テストで実際に走る」を満たせない＝要件を書き換える必要がある。

### 案 C: 全体テストに pilot の実行例のテストの段を足す

- `tools/test-all.ps1` に `cargo test -p pilot --examples`（と i686 で `--example shiori-host-32-helper`）の段を足す。
- ✅ `Cargo.toml` の `test = true` を書かずに済む。i686 の helper の組み立ても段にできる（12 節の 2 の答えにもなる）。
- ❌ 範囲の外（`tools/`・`tech.md` の段の一覧・スキル `kiro-complete` の段の一覧）に触る。`tools/` の見直しは完了済みで、段の名前を読む手順書が多い（`audit-tools-scripts` の 4 節の表）。

### 規模とリスク

- **規模: M（3〜7 日）**。直しの多くは消すこと（`shiori-host-32` で約 -200 行）と数行の足しで、新しい仕組みは無い。重いのは、5 の形の決めと、i686 の helper・本物の `pasta.dll`・実の画面での確かめ直し。
- **リスク: 中**。今まで組まれていなかったテストが全体テストに乗る（案 A）・`main.rs` の行数と番人・i686 の組み立てが全体テストに無い、の 3 つが読めない所。
- **タスクの見立て: 15〜18**（20 に収まる）。⑴ テストの形（5・`Cargo.toml` と cfg） 1〜2 ⑵ クリック透過（1・15・16・17・27・行数の申し合わせ） 2 ⑶ 送りの層（3・7・8・9・11・21） 2 ⑷ helper の終わり方と子の片付け（2・6・4） 1〜2 ⑸ 死んだコードと古い注記（10・12・13・14・18・19・20） 2 ⑹ バルーン差し替え（22・23） 1 ⑺ 文書と `Cargo.toml`（24・25・26） 1 ⑻ 実機の確かめ（i686 の往復・異常系・クリック透過の押したまま Alt+Tab） 1〜2 ⑼ かけ直しと全体テスト 1〜2。範囲を外した置き場を入れると 20 を越えやすい（8 節）。

## 12. 設計へ持ち越す調べもの（Research Needed）

1. **実行例の 49 本は今もテストとして組み立てられるか**: このワークツリーには `target\` が無く、組み立てに時間がかかるので確かめていない。設計の段で `cargo test -p pilot --examples --no-run`（x64）と `cargo test -p pilot --example shiori-host-32-helper --target i686-pc-windows-msvc --no-run`（PowerShell で）を 1 回ずつ回し、通らないものを数える。
2. **i686 の helper の組み立てが全体テストに無い**: `ipc.rs`・`helper_window.rs`・`shiori_proxy.rs` の直し（3・7〜13・18）は、x64 の組み立て（`helper.rs` は x64 でも実行例として組まれる）でしか全体テストに確かめられない。i686 の組み立てを実装の最後に手で 1 回回すか、案 C で段にするか。
3. **1 の件の直しを `WM_CAPTURECHANGED` でするか `MK_LBUTTON` でするか**: `wintf-winmsg-executor` の `new_ex` の窓手続きに `WM_CAPTURECHANGED` が届くか（届くはず）を、実の画面の確かめで見る。
4. **9 の件を `SetTimer` にできるか**: message-only の窓（`HWND_MESSAGE` の子）でも `WM_TIMER` は届く（届くはず）。`MessageLoop::run` のフィルタが `WM_TIMER` を受けることを確かめる。
5. **`log-capture-kit` のほかの番人**（`temp_path_guard_test.rs`・`sample_path_guard_test.rs`・`with_default_guard_test.rs`・`doc_tree_guard_test.rs`）が範囲のファイルに何を求めるか: 直しで新しい一時フォルダや検体の置き方を書かないので当たらない見込みだが、テストを足すときに綴りを確かめる。

## 13. 要件の討議で決めること（設計の判断の項目）

1. **実行例のテストを全体テストで走らせるか**（5 の件・要件 4.3）: 案 A（`Cargo.toml` に `test = true`・新しい依存 0）／案 B（走らせない＝⑴⑵ で残し、要件 4.3 を「手で回すテスト」に書き換える）／案 C（`tools/test-all.ps1` に段を足す・範囲の外）。`two-tunnel.md` 4.4「examples 腐敗検出等の追加…本仕様ではこれを実装しない」との関係を開発者に確かめる。
2. **`pilot-clickthrough-alpha-toggle/main.rs` の行数の予算**（1・15・16・17・27・要件 7.3）: 今 1,006 行で、1,001 行を下回ると番人が赤になる。1（+約 8）と 16（-5）だけなら 1,009 で緑。15（-約 16）を入れると約 993 で赤。選択肢: ⒜ `audit-test-kits` のセッションと申し合わせて表の項目と件数を外してもらう ⒝ 15 を次のウェーブへ申し送る（要件 7.6） ⒞ 15 の後片付けを消す代わりに、注記だけを事実に書き直す（行数 0）＝「効かない旗」は残る。
3. **一次記録との食い違いの扱い**（1・2・3・7・15・16・25・【仮定 3】）: 測った結果は書き換えず、「YYYY-MM-DD audit-pilot の件 N でコードが変わった（測った後の変更）」を README・REPORT の末尾に書き足す形でよいか。25 は「概要」の誤りなので書き換えてよいか。
4. **⑴（`two-tunnel.md` 5.4「品質の基準は緩くてよい」）に当たりうる 20 件を全部直すか**（要件 2.7 の【仮定 2】）: 方針「とにかく指摘事項は全部直す」を先に立てて全部直す（推す）か、整形・命名・説明だけの件（19・21・23・24・25・27）を残すか。
5. **⑵ の候補 2 件**（22・27）: 理由が今も本当な `#[allow(dead_code)]` を、狭める（欄だけ・`_` の名前）か、そのまま残すか。
6. **実機で確かめ直すか**（2・3・6・7〜9・1・15）: i686 の helper と本物の `pasta.dll` の往復（`cargo run -p pilot --example shiori-host-32`・`--selftest-errors`）と、クリック透過の実の画面での確かめを、実装の最後に 1 回ずつ回すか、決定論のテストとコンパイルだけで足りるとするか。回すなら、SSP などで pasta のゴーストが動いていないこと（`127.0.0.1:9276`）と、画面の操作は排他テストの机で順を取ること（要件 4.5）。
7. **20 の件（ghostdir・親の HWND の通り道）をどちらに寄せるか**: 引数だけ／環境変数だけ。README の実行法は環境変数 `HELPER_EXE` だけを書くので、helper への渡し方はどちらでも README は変わらない。
8. **範囲（【仮定 1】）**: `examples/pilot-windows-rs-next/` を外す（推す・8 節）。入れるなら 2 本目の spec に切る（要件 2.6）。

### 要件の討議での振り分け（2026-11-17）

- 要件の討議の議題にしたもの: 13 節の 1・2・3・4（5 を含めた）・6・8。
- 設計（`/kiro-design`）で決めるもの:
  - 13 節の 7（20 の件を引数だけにするか環境変数だけにするか）。
  - 12 節の 1〜5（49 本が今も組み立てられるか・i686 の helper の組み立てをどこで確かめるか・1 の件を `WM_CAPTURECHANGED` でするか `MK_LBUTTON` でするか・9 の件を `SetTimer` にできるか・ほかの番人が足すテストに求めること）。
  - 1 の件の判断の分岐のテストの置き場（`pilot-clickthrough-alpha-toggle/main.rs` の中か、隣の兄弟のファイルか。1,006 行のファイルの行数の出入りに効く）。
  - 5 の件で `shiori-host-32-helper` の実行例を `test = true` にするか（x64 で読めない本物の `pasta.dll` のテストを `#[cfg(target_arch = "x86")]` で囲むか、helper だけ外すか）。

## 14. 範囲の外で気付いたこと（その範囲の持ち物として残す）

- 無し。範囲の外のファイルで直すべき所は見つけていない（4 節の読み手は、名前と行数を守れば壊れない）。

## 15. 設計の段の調べと決め（2026-11-17・`kiro-spec-design`）

> 設計の段（`/kiro-design` の中のサブエージェント）が、12 節の調べものをコードを読んで解き、13 節の振り分けで設計へ回った点を決めた記録。決めの本文と約束は `design.md` が正本で、ここは根拠と測りだけ。

### 15.1 調べの種類と使った物

- 調べの種類: **light**（今の実行例の中の直しだけ・新しい依存も外の API も無い）。外の文書は引いていない。
- 使ったスキルと手引き: `kiro-spec-design`（`rules/design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）と、ponytail の「いちばん小さく完全な直し」の作法（消せる物は消す・呼び元／テスト／見本まで追う・非自明な直しには小さなテストを 1 本）。
- 読んだコード: `shiori-host-32` の 9 ファイル全部、`pilot-clickthrough-alpha-toggle/main.rs` 全部、`crates/pilot/Cargo.toml`、`crates/wintf-winmsg-executor/src/lib.rs:110-270`・`src/util/window.rs:200-304`、`crates/log-capture-kit/tests/file_length_guard_test.rs:55-110`、`log-capture-kit` のほかの番人の頭、`.kiro/steering/structure.md:148-186`（テストの置き場の決まり）。
- 気付いたこと: 5 節のうち `shiori-host-32/helper.rs`・`shiori3.rs`・`crates/pilot/README.md`・`REPORT.md` を指す行番号のいくつかは、今のファイルの行数を越えている（例 `shiori3.rs:161`・ファイルは 109 行）。指摘の中身（何をどう直すか）は今のコードと合っていたので、設計は行番号でなく名前で指す。

### 15.2 12 節の調べものの答え

1. **実行例の 49 本は今もテストとして組み立てられるか**: **組み立てられる。** `cargo test -p pilot --examples --no-run`（x64・PowerShell・このワークツリーの `target\` を新しく作った）を 1 回回し、7 本の実行例（`_template`・`pilot-balloon-asset-swap`・`pilot-clickthrough-alpha-toggle`・`pilot-dropfiles-on-wuc-window`・`shiori-host-32`・`shiori-host-32-helper`・`wintf-winmsg-executor`）のテストの exe が全部できた。1 分 55 秒・終了コード 0・`pilot` の組み立てに警告 0。**走らせてはいない**（赤が出るかは実装の最初のタスクで見る）。走るテストの中身は読んだ: どれも純粋な関数か、見えない窓・`cmd.exe`／`ping 127.0.0.1` を起こすもので、画面・GPU・検体を使わない。例外は `shiori_proxy.rs` の `load_pasta_dll_and_call_load_without_crash`（32bit の本物の `pasta.dll`）だけ。
2. **i686 の helper の組み立てが全体テストに無い**: `tools/` に段を足さない（11 節の案 C は採らない）。実装の最後の実機の確かめ ⑴（要件 4.8）で、どのみち i686 の helper を組むので、同じ日に `cargo test -p pilot --example shiori-host-32-helper --target i686-pc-windows-msvc` を 1 回回す。README の学び 3「共有モジュールは i686 でも `cargo test --target i686-pc-windows-msvc` を回すこと」とも合う。
3. **1 の件は `WM_CAPTURECHANGED` か `MK_LBUTTON` か**: **`WM_CAPTURECHANGED`。** `wintf-winmsg-executor` の窓の手続き（`wndproc_typed`）は全部のメッセージを閉包へ渡し、閉包が `None` を返したときだけ `DefWindowProcW` へ回す（`window.rs:263-304`）＝届く。捕まえを失った瞬間に 1 回だけ来るので取りこぼしが無く、`MK_LBUTTON` を見る形のように次のマウスの動きまで遅れない。`WM_LBUTTONUP` の中の `ReleaseCapture` が自分の窓へ送る `WM_CAPTURECHANGED` は、`dragging.swap(false)` が `false` を返すので二重に片付けない。
4. **9 の件は `SetTimer` にできるか**: **できる。** `MessageLoop::run` は `GetMessageW(None, 0, 0)` でスレッドの全部のメッセージを受け、目覚ましの印でないものは全部フィルタへ渡す（`lib.rs:182-205`）。`SetTimer(hwnd, ..)` の `WM_TIMER` はこのスレッドの列に来るのでフィルタが毎回見る。message-only の窓も `WM_TIMER` を受ける。窓の手続きは `WM_TIMER` に `None` を返し、`DefWindowProcW` は何もしない。i686 での実走は実機の確かめ ⑴ が初めて。
5. **ほかの番人が足すテストに求めること**: `temp_path_guard_test.rs`＝`std::env::temp_dir` の呼び出しを例外表の外に置かない／`sample_path_guard_test.rs`＝検体の置き場の綴りを窓口の外に書かない／`with_default_guard_test.rs`＝捕捉の直接の呼び出しを書かない。設計が足すテストはどれも一時フォルダ・検体・ログの捕捉を使わないので当たらない。`workspace_scan_test.rs` は新しいファイルを足しても壊れない（在るべきファイルを消さない）。

### 15.3 13 節の振り分けで設計へ回った点の決め

- **20 の件（ghostdir と親の HWND の通り道）**: **引数だけ**にする（`argv[1]=ghostdir`・`argv[2]=parent_hwnd`）。理由: helper のほかのモード（`--selftest-load`・`--selftest-loop`）も引数で、通り道が 1 つになる。開発者の端末に残った環境変数が勝つ道が無くなる。README の実行法（`HELPER_EXE` だけ）は変わらない。
- **1 の件のテストの置き場**: 兄弟のファイル `pilot-clickthrough-alpha-toggle/main_drag_tests.rs`（`#[path]` つきの接続宣言・モジュール名 `drag_tests`）。`main.rs` の中の `mod tests` に足すと、直しの後の約 990 行が 1,000 行を越え直す（要件 6.1）。新しいテストのモジュールは兄弟のファイルに置く決まり（`structure.md`「Unit Tests」）にも合う。既存の 7 本は動かさない（ファイルを割らない＝C22 の約束 ⑵）。
- **5 の件で helper の実行例を `test = true` にするか**: **する。** x64 で走れない本物の `pasta.dll` のテストだけを `#[cfg(target_arch = "x86")]` で囲む（要件 4.6「走れる組み立てだけに限る」）。helper にだけある `shiori_proxy.rs` の ANSI のテストと、新しい `helper_window_tests.rs`（終了コード）を全体テストで走らせられる。

### 15.4 まとめ方（synthesis）

- **まとめられたもの（generalization）**: 起こし役のスレッドつきの待ち 2 つ（親の HELLO 待ち・helper の UNLOAD 待ち）は「この窓で、条件か期限まで回す」1 つの `ipc::pump_until` になる。`WM_COPYDATA` の中身の取り出し 2 つは `ipc::copydata_payload` 1 つに、送り 2 つは `send_copydata` 1 つ（1 往復は「受け皿を空に→送る→取る」の 3 行）になる。HWND の下 32 ビットの取り出しは `ipc::hwnd_to_u32` 1 つ。
- **作るか使うか（build vs adopt）**: 待ちの起こしは、自前のスレッドで `WM_NULL` を撃つのをやめ、Windows の `SetTimer` を使う。子のプロセスを止めるのは std の `Child::kill`／`wait` をそのまま使う（新しい道具を作らない）。送りの失敗の見分けは Win32 の `GetLastError` の値をそのまま持つ（`SendFailed(u32)`）。
- **削ったもの（simplification）**: `MsgTag::Load`・`From`／`TryFrom`・`PAYLOAD_HEADER_LEN`・`PeerGone`・`encode_hwnd_le`／`decode_hwnd_le`・`DEFAULT_TIMEOUT`・`HelperState`・数え 6 つ・使われない窓のアクセサ・`poll_exit`・`wait_clean`・`HelperHandle.helper_hwnd`・`module_loaded`・`build_onboot` の引数・環境変数の通り道・`applier_done` の旗。
- **片側だけが使う物の置き場**: `ipc.rs` は親と helper の両方に取り込まれるので、6 つの `#![allow(dead_code)]` を消すと片側だけの物が警告になる。受け皿セル `ResponseSlot` と 1 往復の送りは親だけが使うので `parent_window.rs` へ移し（`ParentMessageWindow::request`）、`ipc.rs` には両方が使う物だけを残す。項目ごとの `allow` で済ませる形は採らない（決め 3 の「許す範囲を狭める」の向きに、移す方が強く合う）。README の学び 1 は `ResponseSlot` を名前だけで書き、置き場を書いていないので食い違わない（書き足しは 7 の件で書く）。
- **応答が来ないことの呼び名**: 送りが通ったのに受け皿が空のことを `Timeout` と呼ぶのをやめ、`request` の `Ok(None)` にする（新しい型を足さない）。go(1) の失敗の文は、時間切れ・送れない・応答なしの 3 つで、確かめる向きを分けて出す。
- **終了コード**: helper は UNLOAD を受けずに上限で抜けたら 4、pasta を読めないまま UNLOAD で抜けたら 5（ギャップ分析の 2 の「困る場面」が挙げた `entries=None` の 0 も直す）。親の判定（0 だけが合格）は変えない。

### 15.5 設計の討議へ回す点（OPEN QUESTIONS・仮定つき）

`design.md` は下の仮定で書いた。討議で違う答えになったら、その箇所だけを直す。

1. 応答が来なかったことを `ParentMessageWindow::request` の `Ok(None)` で分ける（3 の件の延長）。仮定: 分ける。
2. helper の終了コードに 5（pasta を読めないまま UNLOAD で抜けた）を足す。仮定: 足す（4 だけにする案もある）。
3. 20 の件は引数だけ（環境変数を消す）。仮定: 引数だけ。
4. 9 の件の待ちは `SetTimer`（起こし役のスレッドを消す）。仮定: `SetTimer`。i686 の実機で動かなければスレッドを `pump_until` の中の 1 か所に戻す。
5. 1 の件は `WM_CAPTURECHANGED`。仮定: `WM_CAPTURECHANGED`（`MK_LBUTTON` は見ない）。
6. 新しいテストのファイル 3 本（`main_drag_tests.rs`・`helper_window_tests.rs`・`parent_window_tests.rs`）を足す。要件 4.6 の「新しい依存とファイルは足さない」は、実行例のテストを全体テストに乗せる配線（`Cargo.toml` の `test = true`）について言ったもので、テストの置き場の決まり（`structure.md`）と要件 6.1 で足すテストのファイルは含まない、と読んだ。仮定: この読み。
7. 18 の件のテストは、テストの exe が読んでいない System32 の DLL を 1 つ選び、前と後に `GetModuleHandleW` で「読まれていない」を確かめる（前の確かめが外れたら黙って通らず落ちる）。候補は `dciman32.dll`。仮定: この形（実装で別の DLL に替えてよい）。
8. 一次記録の書き足しは、要件 5.3 が挙げた 1・2・3・7・11・15・16 の 7 件だけにする。6（子を止める）・9（待ちの起こし方）・12（`wait_kind`）・20（引数だけ）も走らせ方のコードを変えるが、記録の文に書かれた事柄ではないので書き足さない。仮定: 7 件だけ。
9. `ipc.rs` のテストは親と helper の 2 つのテストの exe で 2 回走る（約 1 秒ずつ増える）。仮定: 受け入れる。
