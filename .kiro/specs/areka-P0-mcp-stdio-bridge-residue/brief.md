# Brief: areka-P0-mcp-stdio-bridge-residue

> 2026-10-11 `/kiro-discovery` で起票（`areka-P0-mcp-stdio-bridge` の完了時の棚卸）。出どころは同 spec の `tasks.md` の Implementation Notes（最終検証の「警告で手を付けなかったもの」と、タスク 4.1・4.6・6.3 の「既知の限界」）。場所の記述は起票時（main `95b34055` を取り込んだ同 spec のブランチ）＝着手時に引き直す。

## Problem

中継 `areka-mcp-bridge` は実機で動いている（Claude Desktop から areka のツールを呼べる）。ただ、最終検証で「直さなくても今は困らないが、檻が無い・同じ形が 2 か所に在る・端で動きが止まりうる」と分かった所が 8 つ残っている。放っておくと、後で中継や配布スクリプトを触る人が、守られていない所を知らずに壊す。

## Current State

どれも、areka の受け口と Claude Desktop が相手なら今は起きないか、起きても害が小さい。

1. **配布スクリプトが「窓を出さない作り」を確かめていない**。`areka.exe` と `areka-mcp-bridge.exe` はどちらも `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` で建ち、実物は GUI の形と確認した。しかし `tools/package.ps1` の中身の判定に Subsystem の検査は無い（この 1 行を消しても配布の検査は緑のまま。中継が黒い窓を出すようになる）。
2. **偽の受け口のテスト 2 本が 2 回走る**。`tests/common/fake_server.rs` を `src/lib.rs`（`cfg(test)`）と `tests/exe_test.rs` の両方が `#[path]` で読むので、その中のテストが両方の束で走る。
3. **上限つきの接続が同じ形で 2 か所に在る**。`scan.rs` の `open` と `http.rs` の `OwnHttp::post`（`tokio::time::timeout` で `TcpStream::connect` を包む形）。
4. **`id` を持つ要求に areka が 202 を返したとき**の枝にテストが無い（応答の上限まで待って `timed out` になる）。
5. **「標準入力が閉じて 5 秒を越える要求」**の枝にテストが無い（待ち時間の上限の枝へ落ちる）。
6. **`AREKA_MCP_PORT` が UTF-8 でない・読めない値のときの記録**が、areka 本体と共有の関数の日本語の行で、「待ち受ける」と書く（中継は待ち受けない）。場所は `crates/areka-mcp/src/port.rs`。
7. **壊れた行への答えの書きが詰まっていると、6 秒の見切りが効かない**。輸送（`stdio.rs`）が次の行を読まないので標準入力の終わりを見られない。Claude Desktop は壊れた行を送らない。
8. **areka の前に黙る相手が 10 個以上並ぶと、その回の探索で areka を飛ばす**。先に開いた接続は、前の候補の黙る相手 1 つにつき確かめの 3 秒を待たされ、受け口の 30 秒の上限で閉じる（`scan.rs`）。SSP はすぐ名乗るので実機では起きない。

このほかに、確かめが残っているもの:

9. **arm64 の機械で中継が起きること**は未確認。配布スクリプトは arm64 の zip の中身の判定まで通した（開発者の机は x64）。`release.yml` はタグの push か手動の実行でしか走らないので、次のリリースの走りの結果で見る。

## Desired Outcome

- 1: 配布の検査が、zip の中の自分で建てた exe の Subsystem を見て、GUI でなければ赤になる。
- 2・3: 重複が 1 つになる（2 は走る束を 1 つに、3 は関数を 1 つに）。動きは変えない。
- 4・5: それぞれ決定論テストが 1 本ずつ在る。
- 6: 記録の行が中継の事実に合う（中継の側で別の文にするか、共有の文を中立に直すかは設計で決める）。
- 7・8: 直すか、限界として `dist/README.txt` か差の記録に書いて閉じるかを決め、決めたとおりにする。
- 9: arm64 の走りの結果を記録に残す（赤なら直す）。

## Approach

拾い残しをまとめて 1 本で片づける（`impl-watch-residue`・`popup-menu-residue` と同じ型）。7・8 は「直さず限界として書く」が安い見込みだが、要件の段で開発者に聞く。

## Scope

- **In**: `crates/areka-mcp-bridge/**`・`tools/package.ps1` の中身の判定 1 つ・`crates/areka-mcp/src/port.rs` の記録の 1 行（6 を共有の側で直すとき）・`doc/ssp-mcp/bridge-diff-areka.md` と `dist/README.txt` の該当の行。
- **Out**: 中継の答えの形（要件 1〜4 の約束は変えない）・組み込みの答え（設計 D-18）・areka の受け口の答え（`mcp-receiver-bugs` の件 1）・SSE や取り消しへの対応（`completed/areka-P0-mcp-stdio-bridge/design.md`「残る縛り」）。

## Boundary Candidates

- 配布スクリプトの検査（1）／中継のコードの掃除（2・3）／テストの穴（4・5）／記録の文（6）／端の動き（7・8）／arm64 の確かめ（9）。

## Out of Boundary

- 新しい外部のクレート（足さない）。
- Claude Desktop の側の振る舞い（中継を 2 つ起こす・エラーの後に起こし直す、など。記録は `completed/areka-P0-mcp-stdio-bridge/verification/signoff.md`）。

## Upstream / Downstream

- **Upstream**: `mcp-stdio-bridge`（着地済み）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし（完了した `mcp-stdio-bridge` の拾い残し）。
- **Adjacent**: `tools/package.ps1` を触る spec（`release-code-signing`・`install-live-target-hazards`・`winget-release-automation`）。スキル `/release`（9 の走りはここで起きる）。`mcp-receiver-bugs` の件 1（`real_receiver_test.rs` が重なる）。

## Constraints

- 中継の exe をテストで起こすときは、番号（0 か自分で取った番号）と控えの置き場（`target\` の下）を必ず渡す（机の本物の 9801〜9830 と `%LOCALAPPDATA%` に触れない）。
- 全体テストの負荷で偽の赤になりうるテストの一覧が `completed/areka-P0-mcp-stdio-bridge/tasks.md` の Implementation Notes（3.4・4.1・4.2・4.4・4.5・5.2）に在る。
- 区分 C（バグでない持ち越し）。規模 S〜M（6〜10）。

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）

- 前提の変化: 中継のコード（`crates/areka-mcp-bridge/`）・共有の番号の読み（`crates/areka-mcp/src/port.rs`）・配布スクリプト（`tools/package.ps1`）は、中継の着地（10-11）から 1 行も動いていない。9 件の記述はどれも今も当たる（偽の受け口は `src/lib.rs` の 17 行目と `tests/exe_test.rs` の 24 行目の 2 か所から読まれる・上限つきの接続は `scan.rs` の `open` と `http.rs` の `post` の 2 か所・番号の記録の 2 行〔`port.rs` の 46 行目と 67 行目〕は今も「待ち受ける」と書く・配布スクリプトに窓の作りの検査は無い）。
- 件 1 は小さく済む: 配布スクリプトには PE のバイト列から機種と取り込みの DLL を読む関数が既に在る（`tools/package.ps1` の 349 行目あたり）。同じ読みに窓の作りの欄を 1 つ足せばよい。
- 触るファイル: `crates/areka-mcp-bridge/` の `src/lib.rs`・`scan.rs`・`http.rs`・`stdio.rs` と兄弟のテスト・`tests/exe_test.rs`・`tests/common/fake_server.rs`、`tools/package.ps1`（746 行）の中身の判定、要るなら `port.rs`（76 行）の 2 行、`doc/ssp-mcp/bridge-diff-areka.md` と `dist/README.txt` の自分の行。新しいファイルは無い。1,000 行に近いのは `service_tests.rs`（869 行）だけで、ここへは足さない。
- 規模: 6〜10 タスク（変わらず）。新しい依存は足さない。人の手の確かめも、静かな机も、前後の全体テストの比べも要らない（最後の全体テスト 1 回だけ）。
- 先に要るもの: なし（先頭）。今の版の 2 本（`user-data-root`・`write-failure-notice`）とは触るファイルが重ならない。ただし次の 2 つの約束を付ける。
  - 中継の控えの置き場を決める `crates/areka-mcp-bridge/src/config.rs` には触らない（下の穴＝`user-data-root` が触る見込み）。
  - 件 9 は外の待ちなので、この spec の範囲から外す（下）。
- 今の版の席に入れられるか: 上限 10 は `user-data-root` の下限 12 以下＝`user-data-root` と同じウェーブなら入れられる。`write-failure-notice`（6〜10）だけのウェーブでは上限が下限 6 を超えるので入れない。
- 優先度の区分: C（完了した中継の拾い残し・バグではない）。
- 要件定義のモデル: Opus（新しい境目も、並行や順の保証の新しい設計も無い。今ある中継の掃除とテストの足し）。
- 分割の案: なし。
- 見つけた穴・古くなった記述:
  - 件 9（arm64 の機械で中継が起きるか）は、次のリリース `v0.0.3` の走りでしか見られず、`v0.0.3` は今の版の 2 本が着地した後に打つ。しかもその走りは arm64 の組み立てまでで、arm64 の機械で起こすことはしない（開発者の机は x64）。＝この spec の中では閉じられない外の待ち。範囲から外し、`/release` の走りの記録で見る申し送りにするのがよい。
  - 中継の控えの置き場は、areka の記憶の置き場の決め方を中継の側に写したもの（`config.rs` の 82〜98 行目＝`AREKA_PROFILE_DIR` → exe の隣の `profile\areka` → `LOCALAPPDATA`）。`user-data-root` が areka の記憶を exe の隣から動かすなら、中継の側もそろえないと控えだけ exe の隣に残り、winget の上げ直しで消える。`user-data-root` の brief はこのファイルを挙げていない（向こうの穴・こちらは触らない）。

## 2026-10-16 棚卸㉖のウェーブの約束（C8・割り込み）
- 件 9（arm64 の機械で中継が起きるか）は範囲から外し、「次のリリースへの申し送り」へ移した（`/release` の `v0.0.3` の走りの記録で見る）。
- 触らないもの: `crates/areka-mcp-bridge/src/config.rs`（同じウェーブの `user-data-root` が、控えの置き場を areka の記憶の置き場にそろえるために持つ）。`tools/package.ps1` は件 1 の PE の読み取りの所だけ。
