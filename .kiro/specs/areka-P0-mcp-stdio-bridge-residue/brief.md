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
- **Out**: 中継の答えの形（要件 1〜4 の約束は変えない）・組み込みの答え（設計 D-18）・areka の受け口の答え（`mcp-malformed-call-code`）・SSE や取り消しへの対応（`completed/areka-P0-mcp-stdio-bridge/design.md`「残る縛り」）。

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
- **Adjacent**: `tools/package.ps1` を触る spec（`release-code-signing`・`install-live-target-hazards`・`winget-release-automation`）。`release-cycle`（9 の走りはここで起きる）。`mcp-malformed-call-code`（`real_receiver_test.rs` が重なる）。

## Constraints

- 中継の exe をテストで起こすときは、番号（0 か自分で取った番号）と控えの置き場（`target\` の下）を必ず渡す（机の本物の 9801〜9830 と `%LOCALAPPDATA%` に触れない）。
- 全体テストの負荷で偽の赤になりうるテストの一覧が `completed/areka-P0-mcp-stdio-bridge/tasks.md` の Implementation Notes（3.4・4.1・4.2・4.4・4.5・5.2）に在る。
- 区分 C（バグでない持ち越し）。規模 S〜M（6〜10）。
