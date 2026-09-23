# Implementation Plan

> 変更の中心は 32bit の助け手の SHIORI 確立部 1 ファイル（design.md「Boundary Commitments」）。同じファイルを触る 3.x は並べない。2 本目の偽 DLL（別クレート）と文書だけが並走できる。
> 既存の偽 DLL・助け手の既存テスト・親（host）の e2e・助け手の `main.rs` は **全タスクを通して無改変**（要件 6.2・6.8・4.5）。

- [x] 1. 実装前の実機の赤と検体の入口の取り直し
  - **助け手に手を入れる前に行う**（3.1 以降はこのタスクの完了を待つ）。今日のコードのまま x64 の areka を build し、i686 の助け手を PowerShell で build してその実行ファイルの隣へ複製したうえで、検体 3 体（`konnoyayame`・`R_POST_and_KOMAINU`・`emo2`）の入口の有無を `dumpbin /exports` で取り直し、requirements の Introduction の表（YAYA・pasta＝`loadu` あり／里々＝なし）と突き合わせる
  - 既定コードページに無い字（絵文字など）を含む浅いフォルダへ検体を複製し、有界 auto-exit で起動して今日の壊れ方（`load` に化けたパスが渡る・警告 0 行）を stderr と親のログで記録する（`emo2` は絶対パスかつ短いパス・親の `RUST_LOG` は判定の分岐の水準まで開ける）
  - 完了の状態: research.md に「入口の表の再確認の結果」と「今日の赤（配置・コマンド・ログの抜粋）」が追記されている。表が食い違えば要件 7.1〜7.2 の期待値を実物に合わせて改めたことも記録されている
  - _Requirements: 7.3, 7.4, 7.5_

- [x] 2. (P) `loadu` を持つ 2 本目の偽 32bit DLL
  - 新しい fixture クレートを作り（`[lib] name = "shiori_loadu"`＝出力は既存の `shiori.dll` と衝突しない `shiori_loadu.dll`・cdylib・publish しない）、`loadu`・`load`・`unload`・`request` の 4 つを公開する。`loadu`／`load` は受け取ったバイト列をコピーしてから入力メモリを自分で解放し、`<入口名>\t<小文字 16 進>\n` を記録 env が指すファイルへ追記する
  - `loadu` は偽返却の注入 env が `1` のとき 0、それ以外は 1 を返す。`load` と `unload` は常に 1、`request` は固定の 400 応答を新しいメモリで返す。戻りは Win32 `BOOL` と同じ 4 バイト整数
  - 注入 env は既存と同じ `HOST32_TESTDLL_` 接頭辞のテスト専用 2 つ（記録・偽返却）。依存は既存 fixture と同じもののみ。unsafe の各ブロックに Safety 根拠を書く
  - 自身の単体テスト（x64）で、`loadu` を直接呼ぶと記録が `loadu\t<hex>` の 1 行になること・注入で 0 を返すこと・`load` が `load\t…` の行を書くことを確かめる。env を触るテストはクレート内の自前の Mutex で直列化する
  - 完了の状態: `cargo test -p shiori-host32-testdll-loadu` が緑で、`cargo build -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` が `target/i686-pc-windows-msvc/debug/shiori_loadu.dll` を出す。既存の偽 DLL クレートの差分は 0
  - _Requirements: 6.1, 6.2, 9.1, 9.2, 9.3_
  - _Boundary: shiori-host32-testdll-loadu_

- [x] 3. 助け手の SHIORI 確立部（入口の選択・符号化・呼出）
- [x] 3.1 入口の選択の判断と入口の解決・戻り値の型
  - `loadu`・`load`・`unload` の戻りを 1 バイト整数で受ける型に改め、呼出側の判定を「0 なら初期化が偽を返した失敗・0 以外は成功」（`== 1` とは書かない）に改め、`unload` の戻りは今日どおり捨てる（`request` の署名は不変）。Safety 文に 1 バイト受けの根拠と天井（C 製 DLL が下位バイト 0 の非 0 値を返すと失敗扱い）を書く
  - 「`loadu` の有無 × `load` の有無」の 4 通りから入口を 1 つだけ選ぶ純関数と、fn ポインタを 1 つだけ持つ入口の型を新設する。両方無いときの失敗の名札は既存テストが固定する `"load"` のまま
  - 入口の解決を「`loadu`→`load` を任意で引いて選択の純関数へ渡し、その後で `unload`・`request` を必須で引く」順に改め、確立途中の失敗は今日どおりライブラリを解放してから返す。読む者の無い `load` の記録欄を外す。この段階の呼出側は選ばれた入口を既存の既定コードページ経路で呼ぶだけでよい（入口別の符号化は 3.3）。この「`loadu` にも既定コードページのバイト列が渡る」状態は 3.3 までの一時的なもので、i686 の実読テストと実機確認は 3.1 単独の状態では行わない
  - 兄弟テストファイルを新設して接続し、判断表 4 行（両方→`loadu`／`loadu` のみ→`loadu`／`load` のみ→`load`／両方無い→入口が無い失敗）を x64 常時のテストで fn ポインタの同一性まで固定する
  - 完了の状態: 判断表の 4 本が x64 で緑、優先順を逆にすると 1 本目が赤になることを一度確かめている。既存 `mod tests`（`kernel32_yields_entry_not_found` を含む）が無改変で緑
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.7, 1.8, 5.1, 5.2, 5.3, 5.4, 5.5, 6.3, 9.3, 9.5_
  - _Boundary: shiori_proxy, shiori_proxy_loadu_tests_
  - _Depends: 1_

- [x] 3.2 コードページを引数に取る符号化と表せない字の検出
  - 既定コードページへの変換を「コードページを引数に取る内側」へ移し、変換の作法（フラグ 0・既定文字は OS 既定・2 回呼び）は今日のまま保つ。変換したバイト列を同じコードページで UTF-16 に戻し、元と一致しなければ「表せない字あり」とする（best-fit も捕まる・コードページの値で分岐しない）
  - 戻す側の変換が判定できない（0 以下）ときは「表せない字あり」に寄せ、往路の変換が 0 以下なら今日と同じ符号化失敗にする。空パスは空・表せない字なし
  - 既存の既定コードページ符号化の関数は署名を保つ薄い包みにする（既存テスト 3 本の保護）
  - 兄弟テストに x64 常時の検出テストを足す: US-ASCII（20127）で ASCII だけ→なし・恒等／非 ASCII を含む→あり、UTF-8（65001）で絵文字を含む→なし・UTF-8 と一致。機械の既定コードページを読まない
  - 完了の状態: 検出テストが x64 で緑、既存の `ansi_encode_*` 3 本が無改変で緑（CP932 機。65001 機で元から赤の 1 本は本仕様と無関係である旨を実装記録に 1 行残す）
  - _Requirements: 3.2, 3.3, 3.4, 3.6, 6.5, 6.8_
  - _Boundary: shiori_proxy, shiori_proxy_loadu_tests_

- [x] 3.3 入口別のバイト列・観測の 2 行・呼出と判定
  - 入口別のバイト列を作る前段を新設する: `loadu` なら置き場所のパス文字列の UTF-8 をそのまま（置き換え無し・NUL 無し・新依存も新しい OS 呼出も無し。UTF-8 にできなければ符号化失敗）、`load` なら 3.2 の既定コードページ符号化を使い、表せない字があれば固定語句の警告を 1 行（元のパス付き）出してそれでも同じバイト列を渡す。`loadu` の枝では検出を行わない
  - 呼出は既存のメモリ確保（callee 解放）→ 入口名の固定語句の行（`[helper] SHIORI 初期化の入口: loadu|load`）を呼ぶ直前に 1 回 → 選ばれた入口を 1 回だけ呼ぶ → 0 なら「初期化が偽を返した」失敗・0 以外は成功。`loadu` が 0 でも `load` へは落ちない。失敗の種別の 1 行は `main.rs` の既存の行に任せ、`main.rs` と失敗の種類は増やさない
  - モジュール冒頭の「確立シーケンス」の説明を 4 つの入口と入口の選択に合わせて書き直し、入口が無い失敗の名札の doc に「両方無いときは `"load"`」を書く
  - 兄弟テストに x64 常時の UTF-8 固定バイト列のテストを足す（CP932 に在る字と無い字を含むパスで、`loadu` の前段が返すバイト列が文字列の UTF-8 と一致・NUL 無し）
  - 完了の状態: UTF-8 のテストが x64 で緑、PowerShell で `cargo build -p shiori-host32-helper -p shiori-host32-testdll --target i686-pc-windows-msvc`（i686 の target 導入済み）が通り、`--target i686-pc-windows-msvc` の helper テストで既存の i686 テスト（courtesy `unload`・request 往復・loopback）が無改変で緑。`shiori_proxy.rs` は 1,000 行未満
  - _Requirements: 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.5, 4.1, 4.2, 4.3, 4.4, 4.5, 5.2, 5.3, 6.4, 8.4, 9.4, 9.6_
  - _Boundary: shiori_proxy, shiori_proxy_loadu_tests_

- [x] 4. 2 本目の偽 DLL を i686 で実際に読むテスト
  - 兄弟テストに i686 限定（既存の ignore の作法）の 2 本を足す。所在の解決は自前（テスト専用 env の上書き → i686 の debug／release 成果物 → 無ければ 2 本目の先ビルドの命令を書いて panic）、env を触るテストの直列化も自前で持つ（既存の私有の解決器と直列化には触らない）
  - 一時の置き場所は既定コードページに無い字を含む名前にする。1 本目は確立が成功し、記録が `loadu\t<置き場所の UTF-8 の 16 進>` の 1 行だけで `load\t` の行が 0。2 本目は偽返却の注入で「初期化が偽を返した」失敗になり、記録は `loadu` の 1 行で `load\t` の行が 0
  - 完了の状態: PowerShell で `cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` の後、`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` で 2 本が緑、選択の優先順を一時的に逆にすると 1 本目が赤（記録が `load` になる）ことを一度確かめている
  - _Requirements: 6.6, 6.7, 6.9, 9.5_
  - _Boundary: shiori_proxy_loadu_tests_
  - _Depends: 2, 3.3_

- [ ] 5. 文書・台帳の追随
- [ ] 5.1 (P) 裁定の登記・先ビルド手順・steering
  - `doc/COMPAT_ARCHITECTURE.md` §8 の表末尾に裁定 3 行（`loadu` だけの DLL を受け入れる／`loadu` が偽でも `load` へ落ちない／`load` へ落ちて表せない字があれば警告して渡す）を、正典の沈黙箇所と出典 spec 付きで足す
  - 親クレートの README の「手順（コピペ可）」に 2 本目の先ビルドを 1 行足す。親の e2e の doc コメントと panic 文言には足さない
  - steering `structure.md` の「Test DLL Fixture Crates」に 2 本目の fixture を 1 項目足す
  - 完了の状態: 3 ファイルの差分が上記の追記だけで、完了 spec の文書の差分が 0
  - _Requirements: 6.9, 8.1, 8.5, 8.6_
  - _Boundary: COMPAT_ARCHITECTURE, host README, steering_
  - _Depends: 2_

- [ ] 5.2 (P) 台帳の正本 → 写し → 生成物の撮り直し
  - 正本 `briefing-shiori.md` の群 14c（判断の根拠の場所・共通 note の壊れ方／ログ／根拠の場所）・「今ある物」・「足りない物」⑴ を実装（`loadu` 優先・無ければ `load`・表せない字の警告・4 つの入口）に合わせて手で直し、2026-09-06 の経緯段落 2 か所には「（2026-09-23 以降は 4 つ全てを引く）」を足す
  - 次に写しの台帳 `shiori.toml` の冒頭注釈（群 14c）と `ukadoc:spec_dll` の `note` を正本に揃える。`status` は `degraded` のまま
  - 生成器で `report/shiori.md`・`report/summary.md` を撮り直し（手で直さない）、正本を `loadu` と「3 つ」で grep して事実と違う文が残っていないことを確かめる
  - 完了の状態: `cargo test -p ukadoc-survey` が緑、台帳の `status` の差分が 0、grep で残った「3 つ」「loadu は引かない」は日付付きの経緯段落だけ
  - _Requirements: 8.2, 8.3, 8.5_
  - _Boundary: ukadoc-coverage ledger_
  - _Depends: 3.3_

- [ ] 6. 全体の検証と実機確認
- [ ] 6.1 workspace 全体の回帰と無改変の確認
  - i686 の先ビルド（助け手・既存の偽 DLL・2 本目の偽 DLL）の後に `cargo test --workspace` を回す
  - 既存の偽 DLL クレート・助け手の `main.rs` と兄弟テスト群（`main_*.rs`）・親クレートの src と e2e・完了 spec の文書について、ブランチの差分が 0 であることを git で確かめる（pathspec の実在も確かめる）。変更される `shiori_proxy.rs` の中の既存 `mod tests` は、ブロックの中身を分岐元の版と比べて一致を確かめる。新しい依存クレートと本番 env の追加が 0 であることも確かめる
  - 完了の状態: `cargo test --workspace` が緑で、上記の無改変の確認がすべて差分 0
  - _Requirements: 4.5, 6.2, 6.8, 6.10, 8.5, 9.1, 9.2_
  - _Depends: 4, 5.1, 5.2_

- [ ] 6.2 実機確認（両方の枝と表せない字のフォルダ）
  - 実機の直前に検体 3 体の入口を `dumpbin /exports` で取り直して表と一致を確かめる。x64 の areka を build し、i686 の助け手を PowerShell で build してその実行ファイルの隣へ複製し（workspace の x64 ビルドが同名の助け手を上書きする罠に注意）、有界 auto-exit で起動し、親の `RUST_LOG` を判定の分岐の水準まで開ける
  - `konnoyayame`・`emo2` → stderr に `SHIORI 初期化の入口: loadu` が 1 行・`入口: load` が 0 行・ゴーストが喋る。`R_POST_and_KOMAINU` → `入口: load` が 1 行・警告 0 行
  - タスク 1 と同じ既定コードページに無い字のフォルダで `konnoyayame` → `loadu`・喋る。同じ場所の里々 → `入口: load` 1 行＋警告 1 行
  - 完了の状態: 各起動のコマンドと grep の結果（行数）が research.md に記録され、タスク 1 の赤と並べて「直った」ことが読める
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_
  - _Depends: 1, 3.3, 6.1_

## Implementation Notes

- 3.2: 既存 `ansi_encode_mixed_japanese_is_multibyte` は既定コードページが 65001 の機械では本仕様と無関係に元から赤（`assert_ne!(bytes, UTF-8)` が、CP_ACP=65001 では CP_ACP のバイト列が UTF-8 と一致するため失敗する）。6.8 の「無改変で緑」は CP932 機での確認（本機 ACP=932・レジストリ `Nls\CodePage\ACP` で確認）。
