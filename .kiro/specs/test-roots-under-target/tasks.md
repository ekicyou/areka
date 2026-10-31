# Implementation Plan

> 触るファイルは設計の File Structure Plan の 4 本だけ（窓口の `lib.rs`・`lib_tests.rs`、迂回の見張り、`devroot.rs` のコメント）。呼び出し側・例外表・`Cargo.toml` は変えない。走行の記録は末尾の `## Implementation Notes` に書く。

- [x] 1. 入口を替える前の、実物の `pasta.dll` の較正の走行
  - PowerShell 7 で i686 の `shiori-host32-helper` をビルドし、検体 `emo2` の `pasta.dll` の絶対パスを `nar-sample-path` で得る（取り寄せは要らない）
  - 同じ PowerShell の中だけで `PASTA_DEBUG` を消し `HOST32_PASTA_DLL` を与えて、`real_pasta_test.rs` の 1 本を有界で 1 度回す（コードは変えない）
  - Implementation Notes に、緑か赤か・赤なら止まった所（`boot` の失敗か・観測の 60 秒切れか・終了か）・`ghost\master\pasta.dll` までのパスの字数を「前」として書いてある
  - _Requirements: 3.2_

- [ ] 2. 窓口と見張りの手当て
- [x] 2.1 `TempPath::new` の置き場を `target\test-roots\` へ替え、説明を合わせる
  - `new` の本体を `under_target` と同じ置き場の式を通す形に替える。署名・名前・連番・破棄・作れないときの panic は変えない
  - crate の利用手順に置き場の短い節を足し、`new`・`under_target`・置き場の式の doc を「2 つの入口は同じ置き場・同じ働き、どちらを使ってもよい」に合わせる。廃止の印は付けない。doctest の例は変えない
  - `cargo test -p temp-path-kit` で既存の 8 本と doctest 1 本が緑のまま（今の `under_target` の 1 本も緑）
  - _Depends: 1_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 2.2, 2.6, 3.1_

- [x] 2.2 窓口の自己テストで 2 つの入口の置き場を確かめる
  - 今の `under_target` の置き場の 1 本を、`new` と `under_target` を表で回す 1 本に改める。各入口で、親が `test-roots`・その親が `target`・そのまた親に `crates/temp-path-kit/Cargo.toml`・破棄で消える、の 4 つを確かめ、失敗の文に入口の名前を入れる
  - OS の一時フォルダを引く呼び出しは使わない。module doc に 2 つの入口を回すことを 1 文足す
  - `cargo test -p temp-path-kit` が 9 本＋doctest 1 本で緑
  - _Requirements: 1.1, 1.2, 1.5, 4.1_

- [ ] 2.3 (P) 迂回の見張りを案 B に改める
  - 窓口の定義を走査から外す仕組み（外す範囲の定数・窓口だけを集める関数・全ソースからの絞り込み）を消し、窓口のファイルも主検査に入れる
  - 窓口の 2 ファイルを名指しする定数を足し、外した領域に 1 件あることを確かめていた 1 本を、⑴ 2 ファイルが走査の一覧に在る ⑵ 2 ファイルをプロセス識別子の語で走査すると `lib.rs` に 1 件以上当たる ⑶ 入口の語の当たりが 0 件（当たればファイルと行を示す）、の 1 本に置き換える
  - 主検査を `no_temp_dir_entry_point_lives_outside_the_allow_table` に改名し、module doc の規則を「例外表の中にしか存在してはならない（窓口の中にも置かない）」に改める。例外表の doc の 2026-08-27 の測り方には、当時の記録であることを一言添える。例外表の中身と件数の定数は変えない
  - `cargo test -p log-capture-kit --test temp_path_guard_test` が 16 本で緑
  - _Boundary: 迂回の見張り_
  - _Depends: 2.1_
  - _Requirements: 1.2, 4.2, 4.3, 4.4, 4.5_

- [x] 2.4 (P) `devroot.rs` の冒頭の doc の古い 1 文を直す
  - 「窓口は OS の一時フォルダの下に置き場を取る」を、窓口も `target\test-roots\` に取ること、検体の根はそれと分けた `nar-samples` を使うことに書き直す。コメントだけで、コードの行の差分は 0
  - `cargo test -p sample-ghost-kit` が今のまま緑
  - _Boundary: 検体の作業フォルダの説明_
  - _Requirements: 2.3_

- [ ] 3. 確かめと記録
- [ ] 3.1 入口を戻すと見張りが赤になることを一度だけ確かめる
  - `new` の本体を一時的に OS の一時フォルダへ戻し、自己テストの 1 本が `new` の行で赤、迂回の見張りの主検査と窓口の 1 本が `lib.rs` の行を示して赤になるのを見てから戻す（コミットしない）
  - 戻した後に 2 つのテストが緑で、`git diff` に一時の変更が残っていない。結果を Implementation Notes に書いてある
  - _Depends: 2.2, 2.3_
  - _Requirements: 4.1, 4.2, 4.3_

- [ ] 3.2 入口を替えた後の、実物の `pasta.dll` の走行と静的な見積もりの記録
  - タスク 1 と同じ手順でもう 1 度回し、Implementation Notes に「後」として、緑か赤か・止まった所・パスの字数を書く。前と後を比べた読みを書く
  - 確かめられる範囲は helper の作業フォルダと `pasta.dll` の読み込みまでで、Lua がファイルを開く経路は確かめていない、と書く
  - 静的な見積もり 3 つを書く: ⑴ `inproc_wiring` の `LoadLibraryW` のテストの長さと余白 ⑵ `emo2.nar` の `ghost/master/` の下で最も長い相対パスを数え直し、`real_pasta_test.rs` と `snapshot_capture_test.rs` の場合の長さ ⑶ pasta が走行中に新しく書くファイルの深さは測っていないこと
  - 後だけが赤、または前と違う所で止まったときは、名前を短くする備えを入れずに裁定へ上げてある
  - _Depends: 1, 2.1_
  - _Requirements: 3.1, 3.2, 3.4_

- [ ] 3.3 差分の範囲・行数・全体テストを確かめる
  - `git diff --stat main...HEAD -- crates` に出るのが設計の 4 本だけ（呼び出し側の書き換え 0・`Cargo.toml`／`Cargo.lock` 0）
  - 1,000 行の番人が緑で、4 本のどれも 1,000 行を下回る
  - 全体テストを 1 回回し、前に緑だったテストが緑のまま。置き場の移動で赤に変わるテストが出たら、呼び出し側を書き換えずに名前・赤の文・パスの長さを添えて裁定へ上げてある
  - 着地の前に、`areka-test-binary-stability` が静かな机で測っていないことを `areka-impl-watch.exe status` と `roadmap.md` の行で確かめ、結果を Implementation Notes に書いてある
  - _Depends: 2.4, 3.1, 3.2_
  - _Requirements: 2.1, 2.3, 2.4, 2.5, 3.3, 3.4, 4.6, 5.1, 5.2_

## Implementation Notes
- 2.4（2026-11-01）: `cargo test -p sample-ghost-kit` は 81 緑・2 赤。赤の 2 本（`every_registered_sample_lands_where_its_registry_row_says`・`every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`）は、取り寄せの検体 `pasta-in-windows` が `target\nar-samples\fetched\` に無いためで、コメントだけの差分とは関わりが無い。全体テストの前に `tools\sample-fetch.ps1` で取り寄せる（取り寄せは開発者の承認が要る）
- タスク 1（前・較正・2026-11-01）: 入口を替える前に `real_pasta_boot_observes_a_cue_then_shuts_down_cleanly` を 1 回回し、結果は赤。止まった所は観測の 60 秒切れ（起動の失敗でも終了でもない・60.41 秒・終了コード 101・`real_pasta_test.rs:223` の panic）。helper は DLL を読み込んだ後、pasta.dll の load が false を返した（`LOAD 失敗（観測・ack[0]）: LoadReturnedFalse { last_error: 203 }`）。フィクスチャが pasta.dll だけを写して辞書を写さないための、既知の赤と同じ形。`ghost\master\pasta.dll` までのパスは、OS の一時フォルダ（`C:\Users\maz-o\AppData\Local\Temp`）の下の `areka-ghost-real-pasta-boot-observes-a-cue-then-shuts-down-cleanly-{pid}-{serial}\ghost\master\pasta.dll` で 131 字（実際のパスは出力に出ないので、pid 5 桁・serial 0 と置いてコードの組み立てから数えた）。この走行で確かめられるのは helper の作業フォルダと DLL の読み込みまでで、Lua がファイルを開く経路には触れない。i686 の helper は `target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe`、pasta.dll は `target\nar-samples\manual\emo2\ghost\emo2\ghost\master\pasta.dll`。
