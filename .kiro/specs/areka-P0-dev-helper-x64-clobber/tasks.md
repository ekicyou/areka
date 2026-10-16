# Implementation Plan

- [x] 1. 素の `cargo build` が隣の helper を x64 版へ戻すかを確かめて記録する（ビルドだけ・重い回 0）
  - `target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe` が無ければ `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` で作る
  - その i686 版の helper を `target\debug\` の `areka.exe` の隣へ置き、見出しの機械の種類が `014C` であることを見る
  - その後にワークスペースの根で素の `cargo build`（作り直しが要らない状態のまま）を打ち、隣の helper の機械の種類を見直す。続けて `cargo build -p shiori-host32-helper` でも同じことを見る
  - 確かめの後は i686 版を隣へ戻しておく（実機の根や作業場は `target\` の下だけ）
  - 完了の状態: 打ったコマンド・前後の機械の種類・結論（戻る／戻らない）が research.md に 1 節として残っている。結論が「戻らない」でも設計は変えず、文書の書き替え（タスク 4）の根拠として書く
  - _Requirements: 1.6_

- [x] 2. areka が helper を起こす直前に向きを見て知らせる
- [x] 2.1 (P) helper の見出しの判定と記録を作り、兄弟のテストで分かれ道を固定する
  - 見出しのバイト列だけから「i686・ほかの機械の種類・PE でない」を見分ける純粋な判定を作る（std だけ・先頭 4 KiB の上限を `ponytail:` の注記で残す）
  - 判定の結果を記録する関数を作る。宛先は出す側の名前の定数（`charset.rs` と同じ書き方）、i686 は記録 0 行、ほかの機械の種類はエラーの 1 行（パス・機械の種類の 16 進・直し方）、PE でない／読めないは警告の 1 行（パス・理由か誤り）
  - 兄弟のテストのファイルを足し、合成の見出しで i686・x64・arm64・`MZ` 無し・`PE\0\0` 無し・`e_lfanew` が範囲外・長さ 0 を確かめ、記録のほうは log-capture で件数・レベル・`event`・`path` の欄・本文の「i686」と直し方を確かめる
  - 完了の状態: `cargo test -p shiori-host32-host` が緑で、新しいテストが判定 7 通りと記録 4 通り（i686 で 0 件・x64 でエラー 1 件・PE でないと読めないで警告 1 件ずつ）を通している。`Cargo.toml`・`Cargo.lock` に差分が無い
  - _Boundary: HelperImageCheck_
  - _Requirements: 3.1, 3.2, 3.4, 3.5_

- [x] 2.2 helper を起こす関数の入口で判定を呼び、実ファイルで働くことを確かめる
  - helper を起こす関数の最初の文で判定を 1 回呼ぶ。引数・戻り値・起こし方・下位の起こし口（テストが `cmd.exe` を渡すもの）は変えない。モジュールの頭の文書に 1 段落足す
  - 兄弟のテストに、テストの実行体そのもの（x64）を渡してエラー 1 件、無いパスを渡して警告 1 件、を確かめる実ファイルのテストを足す
  - 既存の「無い helper を渡すと起こしの誤りが返る」テストが今と同じ結果のままであることを見る
  - 完了の状態: `cargo test -p shiori-host32-host` が緑で、実ファイルの 2 件と既存の無いパスのテストが通る。`real_connect`・kanade・`boot_config.rs` に差分が無い
  - _Boundary: HelperImageCheck_
  - _Requirements: 3.1, 3.3, 3.5, 3.6_

- [x] 3. 全体テストの最後に隣の helper を i686 版へ揃える
- [x] 3.1 (P) 揃えの道具を作り、自分の検査で 6 つの分かれ道を固定する
  - PE の見出しを読んで i686 かを返す判定（無い・短い・読めないは偽）と、置き先と写し元を受けて揃える関数を作る。引数なしは既定の置き先 `target\debug\` と写し元 `target\i686-pc-windows-msvc\debug\` を使う
  - 置き方は一時名へ写してから置き換える形。失敗したら一時名を消す。終了コードは最後に置き先をもう一度判定した 1 か所だけで決める
  - 端末の文は固定の英文だけ。例外は型の名前と `HResult` の 16 進だけを出す
  - `-SelfTest` は `target\` の下のプロセス番号つきの作業場に偽の小さな PE を作り、⑴ 置き先が既に i686 → 0・写さない ⑵ 置き先が x64・写し元が i686 → 0 ⑶ 置き先が無い → 0 ⑷ 写し元が無い → 1 ⑸ 写し元が x64 → 1 ⑹ 置き先を塞いだまま → 1・`*.tmp` が残らない、を終了コードと文の先頭で確かめ、作業場を消す
  - 完了の状態: `pwsh -NoProfile -File tools/sync-i686-helper.ps1 -SelfTest` が終了コード 0 で 6 つの分かれ道の合格を ASCII で出し、作業場が残っていない。分かれ道の 1 つを壊すと終了コード 1 になることを一度見ている
  - _Boundary: SyncI686HelperStep_
  - _Requirements: 1.3, 1.5_

- [x] 3.2 全体テストのスクリプトに 2 つの段を足す
  - 俯瞰の HTML の道具の自分の検査の段の後に自己検査の段 `i686 helper sync check` を、`-License` の段の後・一覧を出す前に最後の段 `i686 helper beside areka.exe` を置く。どちらも別のプロセスで道具を呼ぶ
  - スクリプトの説明（`.DESCRIPTION`）に 2 つの段を書き足す。今ある段の中身と順は変えない
  - 完了の状態: 段だけを抜き出して回すと（重い回 0・道具を直接呼ぶ）、自己検査が 0、隣が x64 のときに最後の段が i686 版を置いて 0 を返す。`git diff tools/test-all.ps1` が段 2 つと説明の追記だけである
  - _Depends: 3.1_
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4_

- [x] 4. 開発の手順の文書のビルドのコマンドを `cargo build -p areka` に替える
  - `README.md` の「# ビルド」の下の 1 行と、`.kiro/steering/tech.md` の Common Commands のビルドの 2 行（debug と `--release`）を `-p areka` 付きに替える。ほかの説明文は変えない
  - 完了の状態: 両ファイルに素の `cargo build`（`-p` 無し）をビルドの手順として示す行が残っていない
  - _Depends: 1_
  - _Requirements: 1.6_

- [ ] 5. 実機と最後の全体テストで確かめる（順番を守る）
- [ ] 5.1 隣を x64 版にして claudia を起こし、知らせの 1 行を確かめる
  - 根は先例（完了 spec `ghost-standard-balloon` の 6.2）と同じ形で作る: `target\` の下の短い絶対パスに根を作り、`vendors/sample_ghost/` の `claudia.nar` から `ghost\claudia` と `balloon\claudia` を展開して並べ、`AREKA_ROOT` にその根、`AREKA_PROFILE_DIR` に同じ `target\` の下の空のフォルダを渡す。使った絶対パスを記録に書く
  - `cargo build -p areka` の後に `cargo build -p shiori-host32-helper`（x64）で隣を x64 版にし、その根で claudia を有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）つきで起こす
  - 後片付けは続く 5.2 の全体テストの揃えの段に任せる（ここで i686 版へ戻さない）
  - 完了の状態: ログに `event=helper_not_i686` の 1 行がパス・機械の種類・直し方つきで 1 件あり、`connect_failed` より前に出ている。ゴーストは今と同じく SHIORI を読めなかった扱いで続いている
  - _Depends: 2.2_
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 5.2 実装の最後の全体テストを 1 回回す
  - 5.1 の後に、`tools/test-all.ps1` を 1 回だけ回す（回す前に所要時間を伝える）
  - 完了の状態: 一覧に `OK  i686 helper sync check` と `OK  i686 helper beside areka.exe` が出ていて、ほかの段の合否が今と同じであり、終わった後の `target\debug\shiori-host32-helper.exe` の機械の種類が `014C` である
  - _Depends: 3.2, 4, 5.1_
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4_

- [ ] 5.3 写す手間なしで claudia を起こす
  - 5.2 の直後に、何も写さず `target\debug\areka.exe` で、5.1 と同じ根・同じ環境変数（記憶のフォルダは空に戻す）で claudia を有界の自動終了つきで起こす
  - 完了の状態: `0x800700C1` が出ず、`connect_failed` と `helper_not_i686` がどちらも 0 件で、claudia が起きている
  - _Depends: 5.2_
  - _Requirements: 1.4, 3.4_

## Implementation Notes
- タスク 1: 写した i686 版の helper は写し元の古い更新時刻を持つので、素の `cargo build` は helper を作り直して隣を x64 版へ戻す（research.md 10 節）。`cargo build -p areka` は隣を触らない。
- タスク 3.1: 置き先を塞いだときの HResult は `Move-Item -Force` の都合で 0x800700B7 になる（設計の 0x80070020 は書式の例）。自己検査は値に依らない。
