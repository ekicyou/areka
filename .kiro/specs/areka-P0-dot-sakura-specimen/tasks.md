# 実装計画

- 書庫を断られたら（要件 2.7）、2.3 の時点で枝に入る。登記の行・専用テスト・README の表の行を戻し、順を 2.3 → 5.1（起票）→ 3.1（README の行と節に断られた理由と起票先）→ 4.3（短い形: 断られた理由・測った値・起票先だけの `verification.md`）に替えて閉じる。3.2・4.1・4.2・5.2 は行わない。
- 各タスクは自分の成果をコミットして終わる（レビューの取り出しで未コミットの作業が消えないように）。控えた値は 4.3 で `verification.md` に写す。
- 本 spec で `crates/` の下を変えてよいのは `crates/sample-ghost-kit/src/` の `lib.rs`（`SAMPLES` の 1 行）・`lib_tests.rs`・`devroot_fetched_tests.rs` だけ（2.6 の枝のときは見張りも）。areka の本番のコードは変えない。

- [x] 1. 検体の書庫を受け取ったバイト列のまま置く
- [x] 1.1 書庫を取り寄せて、大きさと SHA-256 を brief の値と照らす
  - 開発者のサイトの書庫を、ワークツリーの `target\` の下の取り寄せ用のフォルダへ取る
  - 大きさ 1,425,534 バイトと SHA-256 `0c602c6a…b7a472` の両方を測り、測った値を控える
  - どちらかが違えば、リポジトリへ写さずに止め、測った値を開発者へ伝える
  - 終わった状態: 両方の値が brief と一致したことが控えに残っている（違えば作業が止まっている）
  - _Requirements: 1.1, 1.2_
- [x] 1.2 書庫をリポジトリの検体の置き場へ写し、改行の変換が効かないことと配布物に入らないことを確かめる
  - 取った書庫をバイトのまま検体の置き場へ写す。畳み直し（`fold-samples`）は通さない
  - 検体の置き場の属性で、この書庫の改行の変換が止まっている（`text` が `unset`）ことを確かめる
  - 配布物を組む道具に `dot_sakura` の綴りが 0 件であることを確かめ、控える
  - 書庫だけをコミットし（2.1 が登記の行を足すまで「書庫を置く行だけが書庫を持つ」テストは赤の見込み。PR は squash でまとまるので構わない）、Git Bash で追跡されたファイルを取り出し直した SHA-256（`git cat-file -p HEAD:… | sha256sum`）が 1.1 の値と同じことを確かめて控える。違えばコミットを戻して属性を確かめ直し、分からなければ止めて報告する
  - 終わった状態: 検体の置き場に 1,425,534 バイトの書庫がコミットされ、`git check-attr` が `unset`、取り出し直しの SHA-256 が一致、配布物の道具の当たりが 0 件と控えてある
  - _Requirements: 1.3, 1.4, 1.5_

- [x] 2. 検体 `dot_sakura` を登記し、登記のテストを 10 行・書庫を置く行 9 行に合わせる
- [x] 2.1 登記表に `dot_sakura` の 1 行を足し、数と名前の一覧を直す
  - 登記表の末尾（`pasta-in-windows` の後）に、書庫を置く形・ゴースト・同梱バルーン `bottle` の行と、その上の 1 行の注記を足す。窓口のほかのコードは触らない
  - 登記のテストの数（10 行・書庫を置く行 9 行）・説明文・名前の一覧を設計の「数と名前の直し」の表のとおりに直す。未登録の名前のテストは名前を 10 個の 1 つの配列にまとめ、既知の名前との比べと失敗の表示の確かめの両方に使う（`claudia` の抜けを消す）
  - 取り寄せ形の検体のテストの注記の「今の 8 本」を「今の 9 本」に直す
  - 検体の README の見出しの「書庫を置く行（今の 8 本）」を「今の 9 本」に、「書庫を置く行の検体」の表の末尾に `dot_sakura.nar` の 1 行（種別・項目の数 204・1,425,534 バイト）を足す（README の手順 5: 登記の行と同じコミットで足す）
  - 終わった状態: 窓口のソースを「9 つ」「9 本」「nine」「8 行」「8 本」で引いて、検体の数として残る所が 0 か所（スレッドの数は除く）と控えてあり、登記の行と README の表の行が同じコミットに在る
  - _Requirements: 2.1, 2.3, 3.1_
- [x] 2.2 `dot_sakura.nar` を空の根へ入れた結果を固定する決定論のテストを足す
  - えみりの書庫のテストの直後に、設計の「決定論のテスト」の名前と主張で 1 本足す（入った物 2・飛ばした同梱 0・警告 0・残った作業フォルダ 0・最上位の `delete.txt` と `updates.txt` が本体の直下の普通のファイル・本体の直下に取り出し元のフォルダが無い・同梱の `descript.txt` が在る）
  - 同梱の名は登記の行の `balloons[0]` から引く。見張りの語の綴り（設計に挙げた 5 つの形）をコードにもコメントにも書かない
  - 終わった状態: 新しいテストが 1 本在り、`lib_tests.rs` が 1,000 行に収まる
  - _Requirements: 2.4, 5.7_
- [x] 2.3 窓口と見張りのテストを回し、書庫が断られたか・バルーンが入らないか・見張りが赤かの枝を決める
  - `cargo test -p sample-ghost-kit`（結合のテスト `nar_sample_path_test` もこの 1 回に含まれる）と `cargo test -p log-capture-kit --test sample_path_guard_test` を回す
  - 往復のテストと専用のテストが「受理されるはず」で落ちれば書庫を断られた枝: 行・テスト・README の表の行を戻して数を 9 のまま残し、理由を控える（以後は冒頭の枝の順）
  - ゴーストだけ入りバルーンが入らなければ、行の同梱を空にして注記に理由を書き、控える
  - 見張りが赤なら、見張りの側の決まりで解き、理由を控える（名前と書庫は変えない）
  - 終わった状態: 両方のテストが緑で、本数と終了コードが控えてある（または枝に入った理由が控えてある）
  - _Requirements: 2.2, 2.4, 2.5, 2.6, 2.7, 2.8_

- [x] 3. 検体の README と steering を登記表に合わせる
- [x] 3.1 検体の README に `dot_sakura.nar` の節を書く
  - えみりの節の後・ぱすたさんの節の前に節を置き、URL・SHA-256・大きさ・取り寄せた日・畳み直していないこと、書庫とシェルの `readme.txt` の条件の逐語、2026-10-10 の開発者の言葉の逐語と配布物に入れないこと、見る物、URL の行にしない理由（`.sha256` が 404・2026-10-22）、入れた結果（専用テストの名前で指す）を書く
  - 断られた枝では、表の行と節に断られた理由と起票先（5.1 の後）を書き、見出しの数を登記の実数に合わせる
  - 終わった状態: README に節が在り、表と節の数・大きさ・SHA-256 が登記と 1.1 の値に食い違わない
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
- [x] 3.2 (P) steering の検体の顔ぶれと適合の検体の表を、10 本・4 系統に合わせる
  - `structure.md` の検体の段落を 10 本・ゴースト 7 体・4 系統に直し、`dot_sakura` を並び・受け取ったバイト列のまま・配布物に入れないの並びに足す
  - `product.md` の見出しと表の前の言い方を 4 系統と日付に、表の末尾に `emily4`・`pasta-in-windows`・`dot_sakura` の 3 行を足す
  - 終わった状態: 2 つの文書に「3 系統」「9 本」の検体の数の言い方が残っておらず、`dot_sakura` が載っている
  - _Boundary: 顔ぶれの文書_
  - _Depends: 2.3_
  - _Requirements: 4.1, 4.3_

- [x] 4. 実機で 1 回起こして記録する
- [x] 4.1 実機の 1 回の支度をする（ビルドと根）
  - debug 版の `areka` と i686 の補助プログラム（`--target i686-pc-windows-msvc`）をビルドする
  - `nar-sample-path dot_sakura` で手で使う根を作り直し、根の下が `ghost\dot_sakura` と `balloon\bottle` だけで、一番深いファイルの絶対パスが 260 文字に収まることを確かめる
  - 記憶・記録・撮った物の置き場を `target\dot-sakura-verify\` の下に作る
  - 終わった状態: 2 つのビルドが通り、根の絶対パスと中身の数が控えてある
  - _Requirements: 5.1_
- [x] 4.2 排他テストの机を取り、設計の環境変数で 1 回起こして観察し、終わらせて机を返す
  - 机を取ってから、起動の直前に `sync-i686-helper.ps1` を回して終了コード 0 と `helper is i686` を控える（待つ間に他のセッションの全体テストが x64 版へ替えることがあるため）。i686 でなければ回し直し、直らなければ起こさずに机を返して報告する
  - 設計の環境変数の表のとおりに置き（`AREKA_USER_ROOT` は消す）、絶対パスの命令で裏で起こして番号を控え、終了コードをファイルに書く
  - MCP で `get_active_ghost_list`（`dot_sakura` が返ること）・`get_status`・`get_property`・`dump_surface`（1 人目・2 人目）・`dump_balloon` を取る
  - `raise_event OnClose` を送り、返った台本の `\-` の有無を控える。無ければ `sakurascript` で `\-` を送る。終わらなければ自動終了を待ち、残れば控えた番号のプロセスだけを止める
  - ネットワーク更新・メニューからのインストール・画面の操作はしない。撮った物は `target\` の下に置くだけ
  - 終わった状態: 控えた番号のプロセスが無く、机を返してあり、記録のファイルと終了コードが `target\dot-sakura-verify\logs\` に在る
  - _Requirements: 5.2, 5.3, 5.4, 5.5, 5.9, 5.11_
- [x] 4.3 実機の記録 `verification.md` を書く
  - 設計の 9 つの節（同定・機械の構成・環境変数・走行・項目ごとの結果・読めないときの扱い・ERROR と WARN の数・崩れた所・コードの変更）で書く
  - 前のタスクで控えた値を写す: 1.1 の大きさと SHA-256・1.2 の取り出し直しの SHA-256 と配布物の道具の当たり 0 件・2.1 の数の残りの grep の結果・2.3 の本数と終了コード（と枝に入った理由）・4.2 の補助プログラムの確かめ
  - 項目ごとの結果は 8 項目とネットワーク更新（「確かめていない」と理由）を「出た」「出ない」「確かめていない」のどれか 1 つと根拠で。SHIORI の版は設計の 3 段で決める。読めない・2.x だけなら残りを「確かめていない」とする
  - 崩れは 1 件ずつ症状・根拠・見立て・扱いで、扱いごとの件数を 4 つとも（0 も）書く。どっとさくらの中身から出る ERROR・WARN は理由を書いて外す
  - コードの変更は `git diff --stat main...` で、変えた `crates/` の下のファイルが許した 3 つだけと示す
  - 終わった状態: `verification.md` が 9 つの節を持ち、項目ごとの結果に空欄が無い
  - _Requirements: 1.4, 1.5, 2.3, 2.6, 2.7, 2.8, 5.6, 5.7, 5.8, 5.10, 5.11, 6.1, 6.4, 6.5, 6.6_

- [x] 5. 崩れを起票し、根の README に実機の結果を書く
- [x] 5.1 崩れを全部持ち主へ渡す（`/kiro-discovery` は対話なので、実装の係に任せず主の会話で開発者と回す）
  - 持ち主の在る件は、その spec の `brief.md` の末尾に「どっとさくらで見えた事実」の節を足し、症状と根拠を書く（実機で見た物だけ）
  - 持ち主の無い件は `/kiro-discovery` で起票する。触る場所が同じなら 1 本にまとめる（見込みは別名の件と `alternativestart` の族の件の 2 本）
  - 記録の崩れの表の扱いの欄に、起票した spec の名前か足した brief を書き込む（断られた枝では記録がまだ無いので、起票先は 4.3 が書く）
  - 終わった状態: 記録の崩れが全部「起票した」「既存の spec の brief に足した」「方針で受けない」「ゴーストの側の作り」のどれかで、起票先の `brief.md` が在る
  - _Requirements: 6.1, 6.2, 6.3_
- [x] 5.2 根の `README.md` の動作を確かめているゴーストの表に `dot_sakura` の 1 行を書く
  - えみりの行の後に、実機の 1 回で分かったこと（起動まで通ったか、通らなければその旨と起票先の spec の名前）と、開発用の検体としてだけ使い配布物に入れないことを書く。ぱすたさんの行は足さない
  - 終わった状態: 根の `README.md` の表に `dot_sakura` の行が在り、備考が記録の結果と食い違わない
  - _Requirements: 4.2_

## Implementation Notes

- 控え 1.1: 2026-10-22T23:08:02+09:00 に `target\dot-sakura-fetch\dot_sakura.nar` へ取り寄せ。大きさ 1,425,534 バイト・SHA-256 `0c602c6a431dcbec48c02d934d98e342337d2db9628095e0a6f90d3e83b7a472`（brief と一致）。
- 控え 1.2: 書庫は `5dc6288e` で書庫だけをコミット。`git check-attr text` は `unset`。Git Bash の `git cat-file -p HEAD:vendors/sample_ghost/dot_sakura.nar | sha256sum` は `0c602c6a…b7a472`（1.1 と一致・1,425,534 バイト）。`grep -rn dot_sakura tools/` は 0 件（`package.ps1` が検体から取るのは `emo2` と `StayseeBalloon` だけ）。
- 控え 2.1: `grep -rnE "9 つ|9 本|nine|8 行|8 本" crates/sample-ghost-kit/src/` で検体の数として残る所は 0 か所（当たりは `devroot_fetched_tests.rs:67` の「今の 9 本」・`devroot_sweep_tests.rs` のスレッドの「8 本」・`lib_tests.rs:6` の主張の「8 本」だけ）。冒頭の「主張は 8 本」・主張の列挙・参照する spec は 2.2 の専用テストと一緒に直す。
- 2.1: ワークツリーで `sample-ghost-kit` の取り寄せ形のテストを緑にするには `pwsh -NoProfile -File tools/sample-fetch.ps1` を 1 回回す要がある（`pasta-in-windows` の書庫が `target
ar-samplesetched\` に無かった・本 spec と無関係の準備）。
- 控え 2.3（`697c7c9a` の上・2026-10-22）: `cargo test -p sample-ghost-kit` は終了コード 0（単体 83・`nar-sample-path` の bin 0・結合 `nar_sample_path_test` 6・ほか 5・3 本、全部緑）。`cargo test -p log-capture-kit --test sample_path_guard_test` は終了コード 0（21 本緑）。書庫は断られず（2.7 の枝に入らない）、同梱 `bottle` も入り、見張りも緑（2.5・2.6 の枝に入らない）。
- 控え 4.1（`10893218` の上）: `cargo build -p areka` 終了コード 0・`cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` 終了コード 0。`nar-sample-path dot_sakura` 終了コード 0、`root=C:\home\maz\gitreka\.claude\worktreesreka-p0-dot-sakura-specimen-0f24e9	arget
ar-samples\manual\dot_sakura`。根の下は `ghost\dot_sakura`（ファイル 158）と `balloonottle`（ファイル 38）だけ・計 196。一番深いファイルの絶対パスは 173 文字（`...\ghost\dot_sakura\ghost\master\dotvent_OpeningTalkDebug.txt`）。`target\dot-sakura-verify\{profile,logs,shots}` を作った。
- 控え 4.2（コミット `59b99e54`・2026-10-22）: 机は 23:36:35 頃に `granted test`、23:39:16 に `returned test`。起動の直前の `tools\sync-i686-helper.ps1` は終了コード 0・`helper is i686: target\debug\shiori-host32-helper.exe`。起動の命令は `target\dot-sakura-verify\run1.ps1`（設計の環境変数の表どおり・`AREKA_USER_ROOT` を消す）、番号 37048、起動 23:36:52、終了 23:39:06、終了コード 0（`logs\run1.exitcode.txt`）。記録は `logs\run1.stdout.log`・`run1.stderr.log`（補助プログラムの 1 行）。経路はゴースト `Only`・バルーン `Companion`（`bottle`）。MCP: `get_active_ghost_list` は `．さくら`、`get_status` は空（待機）、`get_property` は `baseware.name`=`areka` だけ答え `currentghost.*` は `NG:Cannot find such property name.`（areka の MCP の今の作り）、`dump_surface` は 1 人目・2 人目とも `NG:No surface has been shown in this scope yet`、`dump_balloon` は 1 人目・2 人目とも `bottle` の枠（瓶の絵・文字なし・`shots\01`・`02`）。SHIORI へのすべての要求（`username`・`homeurl`・`OnFirstBoot`・`OnBoot`・`OnSecondChange`×121・`OnClose`）が状態 400（`shiori_error_response` 124 件）→ 設計の 3 段の ⑵「2.x だけの見込み」で 5.8 の枝。`raise_event OnClose` は「the ghost returned no script」（`\-` 無し）、`sakurascript` で `\-` を送って `talk_done_quit`・`app_exit` で終わった。WARN の大半は `cursor.blendmethod ROP 系は MN 未対応` の 16,403 件（毎フレーム級の繰り返し）、ほかに未知の合成メソッド `move` 22・`alternativestart` 5、バルーン面の縮退 6、全透明の element 4 など。
