# ギャップ分析: areka-P0-host32-testdll-marker-race

- 実施日: 2026-10-04（コードの上での確認だけ。i686 の段も実機も走らせていない）
- 見たもの: `crates/shiori-host32-helper/src/` の全 8 ファイル、`crates/shiori-host32-testdll/src/lib.rs`・`Cargo.toml`、`crates/shiori-host32-testdll-loadu/` の印まわり、x64 側 `crates/shiori-host32-host/tests/` の環境変数まわり、`wintf-winmsg-executor` 0.0.5 の窓の後片付け、全体テスト `tools/test-all.ps1` の i686 の段、ワークスペースの見張り（`crates/log-capture-kit/tests/temp_path_guard_test.rs`・`file_length_guard_test.rs`）、steering の `tech.md`・`structure.md`

## 1. 要約

- 起票時の見立て（偽の DLL を load するテストは 3 本・印の環境変数を差すのは印のテストだけ・読むのは偽の DLL の unload だけ）は、コードの上ではそのまま合っていた。見立てと違うものは見つからなかった（要件 3.1・3.4）。
- 揺れの経路は 2 本ある。主な経路は loopback のテストの UNLOAD の知らせによる unload。もう 1 本は、loopback のテストが LOAD の後・UNLOAD の前に落ちたときの、窓の後片付けによる unload。どちらも印のテストの錠の外で走る。
- 直し方は 2 案ある。案 A は 3 本とも同じ錠を取る形で、テストのファイルだけで閉じる。案 B は、偽の DLL が自分の置き場と同じフォルダの印だけを書く形で、偽の DLL に手を入れる。
- 要件 3.2 の「直す前は赤・直した後は緑のテスト」は、案 B なら 1 本のテストの中で時間待ちなしに作れる。案 A では、2 本のテストの間の順番を時間待ちなしに強いる手が無いので、要件 3.3 の「経路の記録」の側になる。どちらの案を選ぶかで、証拠の形が変わる。
- 規模は XS〜S、危うさは低い。製品のコード（`ShioriByteProxy` とその Drop、窓の手続き `handle_message`）は、どちらの案でも触らない。

## 2. 今の姿（数え上げ・要件 3.1）

### 2.1 補助 exe のテストのバイナリの中身

補助 exe（`shiori-host32-helper`）には `lib.rs` が無い。`main.rs` が `mod shiori_proxy;` と、`#[path]` で読む 5 つのテストのモジュールを宣言している。`shiori_proxy.rs` は `mod tests` を自分の中に持ち、`#[path]` で `shiori_proxy_loadu_tests.rs` も読む。だから全部が **1 つのテストのバイナリ＝1 つのプロセス** で走る。テストは全部で 37 本あり、そのうち 5 本が i686 のときだけ走る（x64 では理由付きの無視の印が付く）。

| ファイル | テストの数 | 偽の DLL `shiori.dll` を load するか | `HOST32_TESTDLL_UNLOAD_MARKER` |
|---|---|---|---|
| `shiori_proxy.rs` の `mod tests` | 6 | する: `testdll_drop_invokes_courtesy_unload`（印のテスト）と `testdll_request_roundtrip_get_and_notify`（往復のテスト）の 2 本。`kernel32_yields_entry_not_found` は `kernel32.dll` を読むだけ | 印のテストだけが差して外す |
| `main_loopback_tests.rs` | 1 | する: `loopback_hello_request_proxy_driven_and_bounded_loop`（loopback のテスト） | 触らない |
| `shiori_proxy_loadu_tests.rs` | 13 | しない（群 D の 2 本は別の DLL `shiori_loadu.dll` を読む） | 触らない（`HOST32_TESTDLL_LOADU_*` だけ・自前の錠 `LOADU_SERIAL`） |
| `main_classify_tests.rs`・`main_load_ack_tests.rs`・`main_resolve_param_tests.rs` | 8・2・6 | しない（`shiori.dll` は引数の文字列として出るだけ） | 触らない |
| `main_response_flavor_hung_cage_tests.rs` | 1 | しない（自前の窓だけで DLL を読まない） | 触らない |

### 2.2 印の環境変数を読み書きするコード

- **読む**: 偽の DLL の unload の出口（`crates/shiori-host32-testdll/src/lib.rs` の `unload` の定義）だけ。呼ばれるたびに `std::env::var("HOST32_TESTDLL_UNLOAD_MARKER")` を読み、値があればそのパスへ `unloaded` を書く。書けなくても黙って続ける。各テストは DLL を自分の一時フォルダへ写してから読むが、写した DLL でも同じコードが走る。どの DLL もプロセス全体の環境変数を見るので、印の置き場は 1 か所を全員で共有していることになる。
- **差す・外す**: 印のテストの定義（`testdll_drop_invokes_courtesy_unload`）の中だけ。load の前に差し、最後の後片付けで外す。
- 2 本目の偽の DLL（`shiori-host32-testdll-loadu`）は、この環境変数を読まない。
- x64 側（`shiori-host32-host` の `tests/`）は、この環境変数を差さない。x64 側が使うのは `HOST32_TESTDLL_DLL`（所在）と `HOST32_TESTDLL_LOAD_FAIL`（load を失敗させる）だけ。

### 2.3 偽の DLL の unload が走る場所（テストごと）

unload は製品の 1 か所、`ShioriByteProxy` の Drop の定義（後片付けとして `unload` を呼んでから `FreeLibrary`）からしか呼ばれない。load に失敗したときは unload を呼ばず `FreeLibrary` だけで終わる（`ShioriByteProxy::load` の定義）。よって「unload が走る」は「load に成功した proxy が drop される」と同じ意味になる。

| テスト | unload が走る時点 | 錠 `TESTDLL_SERIAL` |
|---|---|---|
| 印のテスト | 自分の `drop(proxy)`。途中で落ちたときは、巻き戻しの中で proxy が drop される | 取る |
| 往復のテスト | 自分の `drop(proxy)`。途中で落ちたときは巻き戻しの中 | 取る |
| loopback のテスト | ① UNLOAD の知らせを送ったとき。窓の手続き `handle_message` の UNLOAD の枝が proxy を取り出して drop する。② LOAD の後・UNLOAD の前に確かめが落ちたとき。巻き戻しで補助 exe の窓が drop され、窓の後片付け（`wintf-winmsg-executor` の `Window` の Drop が `DestroyWindow` を呼び、最後の知らせ `WM_NCDESTROY` で状態を手放す）の中で、まだ持っていた proxy が drop される | **取らない** |

ふつうに最後まで走れば、loopback のテストの最後の `drop(helper)` は unload を呼ばない。UNLOAD の枝で proxy をもう取り出してあるからだ（要件の序文にある「もう一度 unload しうる」は、上の ② の落ちたときだけに当たる）。

### 2.4 直列化 `TESTDLL_SERIAL` の今の形

- `shiori_proxy.rs` の `mod tests` の中にある、外から見えない `static`。錠を取るのは同じ `mod tests` の 2 本だけ。loopback のテストは別のモジュール（`main.rs` の子 `loopback_tests`）なので、今の置き場からは届かない。
- 汚れた錠（前の持ち主が落ちた錠）は無視して続ける形（`unwrap_or_else(|e| e.into_inner())`）。だから 1 本が落ちても、錠を取る他のテストが止まることはない（要件 1.3 の土台はもうある）。

### 2.5 事実と合わなくなった説明（要件 6.1）

| 場所（定義） | 今の記述の要旨 | 事実 |
|---|---|---|
| `TESTDLL_SERIAL` の説明 | 「testdll をロードする全テストを本 mutex で直列化」 | loopback のテストは取っていない |
| 印のテストの説明 | 「本テストのみが marker env を set/remove する」「testdll を load する他テストと直列化」 | 差すのは本当に印のテストだけ。ただし loopback のテストとは直列になっていない |
| 印のテストの `set_var` の安全の説明 | 「本テストは testdll を load する唯一のテストゆえ他テストと競合しない」 | 3 本ある。loopback のテストと競合する |
| 印のテストの `remove_var` の安全の説明 | 「以後 testdll を load するテストは無い」 | 並行に走る loopback のテストが、この後も load・unload しうる |
| `mod tests` の冒頭の説明 | 3 本立ての説明（純関数・kernel32・testdll の load→drop） | 往復のテストを含む 4 節になっている（揺れには関係ない。ついでに直すかどうかは任意） |

## 3. 競合の経路（定義の名前で・要件 3.3 の下書き）

**経路 1（主）**: loopback のテストの UNLOAD の段。

1. 印のテスト（`testdll_drop_invokes_courtesy_unload`）が `TESTDLL_SERIAL` を取り、自分の一時フォルダを作り、`HOST32_TESTDLL_UNLOAD_MARKER` にそのフォルダの `unload.marker` を差す。
2. 印のテストが `ShioriByteProxy::load` で自分の写しを読む。`LoadLibraryW` で新しい写しを読むので、ここには時間がかかりうる。
3. この間に、別のスレッドで走る loopback のテスト（`loopback_hello_request_proxy_driven_and_bounded_loop`、錠を取らない）が UNLOAD の知らせを送る。窓の手続き `handle_message` の UNLOAD の枝 → `ShioriByteProxy` の Drop → loopback 側の写しの DLL の `unload`。この `unload` が環境変数を読み、**印のテストの** `unload.marker` を書く（フォルダは手順 1 で作ってあるので、書き込みは成功する）。
4. 印のテストの、load の直後・drop の前の確かめ「unload はまだ呼ばれていない」が落ちる。これは観測した症状と一致する。

**経路 2（落ちたときだけ）**: loopback のテストが LOAD の後・UNLOAD の前のどこかの確かめで落ちると、巻き戻しの中の窓の後片付けが同じ書き込みをする。手順 3 の書き手が変わるだけで、あとは同じ。

**成り立たない組み合わせ**:
- 往復のテストは、印のテストと同じ錠を取るので重ならない。
- 群 D の DLL はこの環境変数を読まない。
- 「drop の後は印が在り、中身が `unloaded`」の確かめは、他のテストに壊されない。他のテストは印を作ることはあっても、消すことはない。中身も同じ `unloaded` になる。
- 手順 4 の確かめを過ぎた後に他のテストが書いても、印のテストは落ちない。

**落ちたときの名残（関連）**: 手順 4 で印のテストが落ちると、後片付けの `remove_var` まで届かない。環境変数は差したまま残り、一時フォルダも消されない。その後、同じプロセスで走る unload は全部、印のテストのフォルダへ書く。印を確かめるテストは他に無いので、今は別のテストが赤になることはない。ただ、片付けが漏れる。

## 4. 要件と、今あるもの・足りないもの

| 要件 | 今あるもの | 足りないもの・制約 |
|---|---|---|
| 1.1・1.2 他のテストの unload に左右されない | 錠 `TESTDLL_SERIAL`（2 本だけ） | **足りない**: loopback のテストが錠の外にいる。または、印の置き場を全員で共有している |
| 1.3 1 本の失敗が連鎖しない | 汚れた錠を無視する書き方 | 案 A で錠を共有するなら、loopback のテストも同じ書き方にそろえる必要がある。印のテストが落ちたときの名残（3 節）をどこまで片付けるかは**未決** |
| 2.1 時間待ちなし | ― | 案 A・案 B のどちらも時間待ちは要らない。**制約**: 2 本のテストの間の順番を強いる仕掛けは、`--test-threads=1` のとき時間切れが無いと止まったままになるので、使えない |
| 2.2〜2.4 確かめを弱めない | 3 本とも i686 のとき走る | どちらの案でも確かめは触らずに済む |
| 3.1・3.4 数え上げ | 本書の 2 節 | 見立てとの違いは無い |
| 3.2 赤→緑のテスト | ― | **案による**（5 節）。案 B なら作れる。案 A では作りにくい |
| 3.3 経路の記録 | 本書の 3 節 | 直した後、その経路が成り立たないことを示す書き方が要る |
| 4.1〜4.3 10 回の実走・全体テスト | `tools/test-all.ps1` の i686 の段（`--no-fail-fast`・既定で並行） | 実装の後に回す |
| 5.1 製品を変えない | ― | どちらの案でも、製品のコードは変えずに済む。案 A で錠を `main.rs` に置く場合も `#[cfg(test)]` の部分だけになる |
| 5.2 偽の DLL の働き（案 B） | load・request・`HOST32_TESTDLL_LOAD_FAIL` の働き | 案 B で変えるのは unload の印の書き方だけ。x64 側はこの環境変数を差さないので影響しない |
| 5.4 1,000 行未満 | `shiori_proxy.rs` 733 行・`main.rs` 587 行・`main_loopback_tests.rs` 441 行・偽の DLL の `lib.rs` 340 行 | 余裕は十分にある |
| 6.1・6.2 説明 | 2.5 節の表 | 決まり（偽の DLL を load するテストが守ること）を 1 か所に書く。案 A なら錠の定義のそば、案 B なら偽の DLL の unload の定義のそばが自然 |

ワークスペースの見張りとの関係:
- **一時パスの見張り**（`temp_path_guard_test.rs`）はファイル単位の例外表で、`shiori_proxy.rs` と `main_loopback_tests.rs` が載っている。表に載ったファイルから一時フォルダの入口の呼び出しが消えると赤になり、表に無いファイルに入口を書いても赤になる。**一時フォルダを作る処理を新しいファイルへ移さない**限り、表は触らずに済む（表は `log-capture-kit` の中にあり、本 spec の範囲の外）。
- **新しいテストを本番のファイルに書かない決まり**（`structure.md`「新規のテストモジュールは本番ファイルの中に本体を書かない」）がある。`shiori_proxy.rs` の `mod tests` は昔からの形なので、今ある部分を直すのは構わない。ただ、赤のテストを新しく足すなら、兄弟のファイル（例 `shiori_proxy_<名前>_tests.rs`）に置くのが規約どおりになる。そのファイルが一時フォルダを作るなら、上の見張りの表に載せる必要が出る。今ある `mod tests` に 1 本足すのか、兄弟のファイルを作るのかは設計で決める。

## 5. 直し方の候補

### 案 A: 3 本とも同じ錠を取る（テストのファイルだけで閉じる）

- やること: 錠 `TESTDLL_SERIAL` を、`mod tests` と `loopback_tests` の両方から届く場所へ移す。loopback のテストの頭で、汚れた錠を無視する書き方で取る。説明を直す。
- 錠の置き場の候補:
  - (A-1) `main.rs` の根に `#[cfg(test)] static` で置く。子のモジュールは親の非公開の項目を見られる。loopback のテストはもう `use super::*` しているので、そのまま届く。`shiori_proxy::tests` からは `crate::TESTDLL_SERIAL` で届く。
  - (A-2) `shiori_proxy.rs` の上の階層に `#[cfg(test)] pub(crate) static` で置く。loopback からは `crate::shiori_proxy::TESTDLL_SERIAL` で届く。
  - (A-3) 錠を新しい兄弟ファイルに置き、ついでに 2 か所に同じ形で写してある `resolve_testdll` も 1 つにまとめる。まとめるのは任意で、揺れを直すのに要るわけではない。
- 触るファイル: `shiori_proxy.rs`、`main_loopback_tests.rs`、A-1 なら `main.rs`（`#[cfg(test)]` の部分だけ）。偽の DLL は触らない。
- よい点: 差分が最小。brief の見立てどおり。x64 側にも 2 本目の DLL にも当たらない。
- 弱い点:
  - 「偽の DLL を load するテストは錠を取る」という約束を、人が守り続ける形になる。テストを足す人が錠を忘れると、揺れが戻る（要件 6.2 の決まりの説明で補う）。
  - 時間の長い loopback のテスト（窓を組んで何往復もする）と、他の 2 本が直列になる。全体の時間はわずかに延びる。
  - 要件 3.2 の赤→緑のテストは作りにくい（6 節）。
- 直した後に経路が成り立たない理由: 3 本とも同じ錠の中でしか load から unload までを走らせない。だから、印のテストが環境変数を差してから外すまでの間に、他のテストの unload は走れない。経路 2 も、loopback のテストが落ちた巻き戻しは錠を持ったまま起きるので、同じ理由で塞がる。

### 案 B: 偽の DLL が「自分の置き場と同じフォルダの印」だけを書く

- やること: 偽の DLL の unload の定義で、環境変数のパスの親フォルダが、自分（読まれている写しの DLL）のフォルダと一致するときだけ印を書く。自分のパスは `GetModuleHandleExW`（アドレスから引く・参照数を変えない）と `GetModuleFileNameW` で引く。印のテストは今のまま、自分の写しと同じフォルダに印を置いているので、テストの側は変えずに済む。
- 変形:
  - (B-1) 環境変数をやめ、load で受け取る `load_dir` を覚えておき、unload でそこへ書く。ただし `load` の枝は既定コードページのバイト列で受け取るので、偽の DLL に文字コードの変換が要る。手間が増える。
  - (B-2) 環境変数は「書くかどうか」の旗だけにして、置き場はいつも自分のフォルダにする。環境変数の意味が変わる。
- 触るファイル: 偽の DLL の `lib.rs`。それに `Cargo.toml`（`windows` に `Win32_System_LibraryLoader` の機能を 1 つ足す。新しいクレートではなく、Cargo.lock は変わらない見込み）。brief の「触るファイル」の一覧には `Cargo.toml` が入っていない。テスト側は、説明の手直しと赤のテスト 1 本。
- よい点:
  - 約束を人が守らなくても、構造の上で他のテストの unload が印を書けなくなる（偽の DLL を load するテストを今後足しても揺れが戻らない）。
  - 要件 3.2 の赤→緑のテストを、1 本のテストの中で時間待ちなしに作れる（6 節）。
  - テストを直列にしなくて済む。
- 弱い点:
  - 偽の DLL に Win32 の呼び出しが 2 つ増える。x64 側の e2e も同じ DLL を読むので、要件 5.2 と隣の期待（x64 側の判定は変わらない）を確かめる必要がある。x64 側はこの環境変数を差さないので、書く・書かないの分かれ道に入らない見込み。
  - 「同じ名前の DLL を別のフォルダから絶対パスで読むと、別のモジュールとして読まれる」が前提になる（**要調査**・7 節）。
  - 印のテストが環境変数を差している間も、他のテストの unload はその環境変数を読む（読んで書かないだけ）。環境変数をプロセス全体で差すこと自体は残る。

### 案 C: 両方（錠も共有し、偽の DLL も絞る）

- 決まりを 2 重にする。証拠は案 B の赤→緑で取れる。差分はいちばん大きい。揺れを止めるにはどちらか 1 つで足りるので、2 重にする理由は「後からテストを足す人の取りこぼしに強い」ことだけになる。

### 採らない方がよい形（参考）

- 印のテストを別のテストのバイナリ（別プロセス）へ移す: 補助 exe には `lib.rs` が無く、`tests/` から `ShioriByteProxy` に届かない。ライブラリの入口を新しく作ることになり、製品の構造を変える。
- テストを並行させない設定（`--test-threads=1`）を全体テストに足す: 要件の範囲外（`tools/test-all.ps1` を変えない）。揺れを隠すだけになる。
- 無視の印を足す: 要件 2.4 に反する。

## 6. 赤→緑のテストの作りやすさ（要件 3.2・3.3）

- **案 A のとき**: 揺れを起こすには「loopback の unload が、印のテストの差しから確かめまでの間に入る」順番を強いる必要がある。2 本の別々のテストの間で順番を強いる仕掛け（待ち合わせ）は、テストを 1 本ずつ走らせる設定では相手が来ず、時間切れが無ければ止まったままになる。だから要件 2.1 と両立しない。
  - 1 本のテストの中で「環境変数を差す → 別の写しを load して drop → 印ができてしまう」を見せることはできる。ただしこれは「印の置き場を全員で共有している」ことの証明で、案 A では直した後も同じ結果になる。だから赤→緑にならない。
  - 代わりに、要件 3.3 の側（本書 3 節の経路の記録と、直した後に成り立たない理由）で示すことになる。「偽の DLL を load するテストのファイルは、必ず錠を取っている」を字面で確かめる検査を足す手もある（直す前は loopback のファイルで赤、直した後は緑）。ただ、字面の検査はもろい。足すかどうかは設計で決める。
- **案 B のとき**: 1 本のテストの中で、時間待ちなしに次の流れを作れる。錠を取って環境変数をフォルダ X の印に差す → フォルダ Y に置いた写しを load して drop → 「フォルダ X の印はまだ無い」を確かめる。直す前は偽の DLL が無条件に書くので赤、直した後は書かないので緑。経路 1・経路 2 の「別の写しの unload が印を書く」をそのまま切り出した形になる。

## 7. 要調査（設計へ持ち越す）

1. **同じ名前の DLL を別のフォルダから絶対パスで `LoadLibraryW` すると、別々のモジュールとして読まれるか**（案 B の前提）。もし先に読んだ同じ名前のモジュールを使い回すなら、自分のパスを引いても別のテストのフォルダが返ってくる。Microsoft の文書で、「すでに読んである同じ名前のモジュールを使う」決まりがパスを省いた検索のときだけに当てはまるのかを確かめる。案 A ではこの問いは要らない。
2. **x64 側の e2e が、偽の DLL の置き場のフォルダの中身を見ているか**（案 B-2 で「いつも自分のフォルダへ書く」形を採る場合だけ）。案 B の本線（環境変数のフォルダが一致するときだけ書く）なら、x64 側は環境変数を差さないので要らない。
3. **10 回の実走で揺れが本当に消えたと言えるか**: 揺れはもともと 1 回だけ観測されたもので、まれにしか起きない。10 回の緑は「起きにくい」の裏付けにしかならず、決定論の主張の根拠は 3 節・6 節の側になる。要件 4 はそのまま回す前提でよい（判断は不要）。直す前に揺れを手元で起こして見せることはしない（要件はそれを求めていない）。

## 8. 規模と危うさ

- 規模: **XS〜S**（案 A なら 2〜3 ファイル・数十行。案 B なら偽の DLL に 20〜40 行＋`Cargo.toml` の 1 行＋テスト 1 本）。今ある形の延長で、新しい仕組みは要らない。
- 危うさ: **低**。触るのはテストと試験用の DLL だけ。製品のコードには当たらない。案 B だけ、x64 側の e2e が同じ DLL を読む点の確認が要る。

## 9. 設計で決めること（要件の議論へ渡す）

1. 直し方: 案 A（錠を共有）・案 B（偽の DLL が自分のフォルダの印だけ書く）・案 C（両方）のどれにするか。
2. 証拠の形: 案 A なら要件 3.3 の経路の記録で示す。字面の検査を足すかどうかも決める。案 B なら要件 3.2 の赤→緑のテスト 1 本。
3. 案 A の錠の置き場: `main.rs` の根（A-1）、`shiori_proxy.rs` の上の階層（A-2）、新しい兄弟ファイル（A-3。2 つの `resolve_testdll` をまとめるかもここで決める）。
4. 印のテストが途中で落ちたときの名残（環境変数が差したまま・一時フォルダが残る）を、落ちても外れる形（drop で外す小さな片付け役）にするか。今は他のテストを赤にしないが、要件 1.3 の「連鎖しない」をどこまで求めるかによる。
5. 新しく足すテスト（赤のテストや字面の検査）を、今ある `mod tests` に入れるか、兄弟のファイルに出すか。兄弟のファイルが一時フォルダを作るなら、一時パスの見張りの表（範囲外の `log-capture-kit`）に 1 行足すことになる。
6. 案 B を採るなら: 偽の DLL の `Cargo.toml` に機能を 1 つ足すのを、触ってよいファイルに入れるか（brief の一覧の外）。7 節の要調査 1 を設計で確かめる。
7. 開発者の方針との照合（要件の議論で追記）: 方針は 2 つとも案 B 寄りである。1 つ目は「根本が 1 か所で直るなら、境界を少し広げてでもそこを直す」。揺れの根本は「どの写しの DLL も、プロセス全体の 1 つの印の置き場へ書く」ことで、案 B はそこを直す。案 A は錠でテストを並べる約束の側で抑える。2 つ目は「報告を読むだけでは原因の実在を判断できないときは、赤のテストを立てる」。案 B なら要件 3.2 の赤→緑のテストを作れる。案 B の不安は 7 節の要調査 1 と、`Cargo.toml` が brief の一覧の外にあること（同じウェーブの他の spec がこのファイルに触らないかを設計で照合する）。

---

# 設計フェーズの調査と決定（2026-10-04）

- 調査の深さ: 軽い調査（今ある仕組みの延長。新しいクレートなし）。外の文書は Microsoft の 4 ページだけを引いた。
- 要点:
  - 7 節の要調査 1（同じ名前の DLL を別のフォルダから絶対パスで読むと別々のモジュールになるか）は、Microsoft の文書で「なる」と確かめた。案 B の前提は成り立つ。
  - 直し方は案 B（偽の DLL が、自分の置き場と同じフォルダの印だけを書く）に決めた。
  - 同じウェーブ C3 のほかの 10 本は、偽の DLL のクレートにも補助 exe のテストにも触らない。

## 10. 調査の記録

### 10.1 同じ名前の DLL を絶対パスで読むと別のモジュールになるか（7 節の要調査 1）

- きっかけ: 案 B は「写しごとに別のモジュールとして読まれ、自分のパスを引くと自分の写しのパスが返る」ことが前提。
- 引いた文書と、決め手の文:
  - LoadLibraryW（引数の説明）: "If the string specifies a full path, the function searches only that path for the module." ＝絶対パスなら、そのパスだけを探す。
  - LoadLibraryW（Remarks）: "If lpFileName does not include a path and there is more than one loaded module with the same base name and extension, the function returns a handle to the module that was loaded first." および "When no path is specified, the function searches for loaded modules whose base name matches the base name of the module to be loaded." ＝「すでに読んである同じ名前のモジュールを使う」のはパスを省いたときだけ。また、同じ名前のモジュールが同時に複数読まれている状態を文書が前提にしている。
  - DLL の検索の順（Dynamic-link library search order）: 冒頭 "You can control the specific location from which any given DLL is loaded by specifying a full path. But if you don't use that method, then the system searches for the DLL at load time as described in this topic." ＝「読み込み済みのモジュールの一覧」を見る手順は、絶対パスを渡さないときの検索の一部。
  - GetModuleHandleExW: `GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS` は "The lpModuleName parameter is an address in the module."。Remarks は、同じ名前のモジュールが複数あるとき名前では引けないので "specify a memory location rather than a DLL name" と勧める。`GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT` は参照数を増やさない（得たモジュールを `FreeLibrary` へ渡してはいけない）。
  - GetModuleFileNameW: "Retrieves the fully qualified path for the file that contains the specified module." と "The string returned will use the same format that was specified when the module was loaded. Therefore, the path can be a long or short file name"。置き場が足りないときは切り詰めて、置き場の長さを返す。
- コードの側の裏付け: `ShioriByteProxy::load` の手順 1 は、受け取った絶対パスをそのまま `LoadLibraryW` へ渡す（`load_library_quiet`）。偽の DLL を load する 3 本のテストは、どれも自分の一時フォルダへ写した `shiori.dll` の絶対パスを渡す。
- 当てはまらない注意書き: LoadLibraryW の「リダイレクトのファイルがあると、アプリのフォルダの同名の DLL を優先する」は、テストの exe のそばにリダイレクトのファイルも `shiori.dll` も置かないので当たらない。
- 設計への意味:
  - 案 B の前提は成り立つ。自分のモジュールは、名前でなく DLL の中のアドレスから引く。
  - 返るパスは読んだときの書き方のままなので、文字の並びでは比べず、`std::fs::canonicalize` で両方をそろえてから比べる。
  - 文書の確認に加えて、足す 1 本のテストが「1 つ目の写しを読んだまま 2 つ目の写しを読む」形で、この前提を実走で確かめる。前提が崩れていれば、そのテストが必ず赤になる（揺れにはならない）。

### 10.2 同じウェーブの他の spec との重なり

- 見たもの: `.kiro/steering/roadmap.md` のウェーブ C3 の行（11 本の「触るファイル」）、`.kiro/specs/*/brief.md` のうち偽の DLL に言及するもの。
- 結果:
  - C3 の ②〜⑪ の「触るファイル」に、`crates/shiori-host32-testdll/`・`crates/shiori-host32-helper/` は出てこない。
  - 偽の DLL に言及する brief は `makoto-dll-host`・`property-ipc-transport`・`mcp-stdio-bridge`・`release-code-signing` の 4 本で、どれも C3 に居ない。
  - 「依存を足す spec は 1 ウェーブに 1 本」の席（C3 は `animated-image-decode`）とはぶつからない。足すのは今ある `windows` クレートの機能 1 つで、`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` は変わらない（`Cargo.lock` は機能の一覧を持たない。`windows` 0.62.2 は 1 つだけ載っており、補助 exe が同じ機能をすでに使っている）。
- 設計への意味: `crates/shiori-host32-testdll/Cargo.toml` に 1 行足しても、並走の約束を破らない。roadmap の C3-① の「触るファイル」の書き方（補助 exe のテストだけ）とは違うので、設計の議論で開発者に見せる。

### 10.3 見張りとの関係

- 一時パスの見張り（`crates/log-capture-kit/tests/temp_path_guard_test.rs`）はファイル単位の例外表。`shiori_proxy.rs` は「プロセス識別子で一意化済み」で載っている。足すテストは OS の一時フォルダの入口を呼ばず、ワークスペースの `target\` の下にフォルダを作るので、表は変えずに済む。既存の 2 本の入口の呼び出しは残るので、「表に載ったファイルに当たりが無い」の赤にもならない。
- 偽の DLL の `lib.rs` は表に載っていない。偽の DLL の側にフォルダを作る単体テストを足すと表に行が要るので、足さない。

## 11. 設計の決定

### 決定 1: 直し方は案 B

- 背景: 9 節の 1・7。
- 比べた案: A（錠を 3 本で共有）／B（偽の DLL が自分の置き場と同じフォルダの印だけ書く）／C（両方）／B の変形 1（load で受け取ったフォルダを覚える）／B の変形 2（環境変数は旗だけ・いつも自分のフォルダへ書く）。
- 選んだもの: B。
- 理由:
  - 揺れの根は「どの写しも 1 つの置き場へ書く」こと。B はそこを 1 か所で直す。A は、約束を守らなかった 1 本を約束の中へ入れ直すだけで、次に足す人がまた外せる。
  - B なら、直す前は赤・直した後は緑のテストを時間待ちなしで作れる（要件 3.2）。A では作れない（6 節）。
  - 開発者の方針 2 つ（根本が 1 か所なら境界を少し広げてでも直す／読むだけで実在を判断できないなら赤を立てる）と合う。
- 代わりに払うもの: 触るファイルが 1 つ増える（偽の DLL の `Cargo.toml`）。偽の DLL に Win32 の呼び出しが 2 つ増える（環境変数に値があるときだけ走る）。
- 採らなかった理由:
  - C: B だけで止まる。loopback のテストを直列にする時間と、約束が 1 つ増える。
  - 変形 1: load の入口は既定コードページのバイト列でフォルダを受けるので、偽の DLL に文字コードの変換が要る。
  - 変形 2: 環境変数の意味が変わる。x64 側を含め、unload のたびに DLL のフォルダへファイルができる。直す前のテストが「機能がまだ無い」で赤になり、競合の経路を示す赤にならない。
- 実装で確かめること: 足す 1 本のテストの赤と緑。

### 決定 2: 証拠の形は赤→緑のテスト 1 本（9 節の 2）

- 手順は design.md の「Testing Strategy」。1 つ目の写しを読んだまま 2 つ目を読んで drop する形にして、決定 1 の前提（別のモジュールとして読まれる）も同じテストで確かめる。
- 字面の検査（「load するテストは錠を取っている」）は足さない。案 B では錠が揺れを止める仕組みでなくなる。

### 決定 3: 錠は残し、役目を「環境変数を差すテストの直列化」に変える（9 節の 3 の置き換え）

- 環境変数はプロセスに 1 つなので、差すテスト同士（印のテストと足す 1 本）は直列が要る。
- 錠の置き場は今のまま（`shiori_proxy.rs` の `mod tests`）。loopback のテストから届く場所へ移す必要はない。名前は `MARKER_ENV_SERIAL` に変える。
- 往復のテストは錠を外す。決まりを「環境変数を差すテストだけが取る」の 1 行にするため。外さなくても揺れは止まるので、開発者が残す方を選べば説明の 1 行を変えるだけで済む。

### 決定 4: 落ちたときの片付け役は足さない（9 節の 4）

- 印のテストが落ちて環境変数が差したまま残っても、ほかの写しの unload は置き場が違うので書かない。次に差すテストは自分の値で上書きする。別のテストの赤にはつながらない。

### 決定 5: 足すテストは今ある `mod tests` に入れる（9 節の 5）

- `structure.md` の決まりは新しいテストのモジュールが対象。足すのは今あるモジュールへの 1 本で、非公開の `resolve_testdll` と錠をそのまま使える。
- フォルダはワークスペースの `target\` の下に作る（開発者の決まり「一時フォルダはワークツリーの `target\` の下」）。既存の 2 本の置き場は変えない。

### 決定 6: 偽の DLL の `Cargo.toml` に機能を 1 行足す（9 節の 6）

- 10.2 のとおり、同じウェーブの約束とぶつからない。手書きの外部関数の宣言で `Cargo.toml` を避ける手は採らない（`windows` クレートに同じものがある）。

### まとめ直し（設計の 3 つの見方）

- 一般化: 要件 1・3・6 は「印の宛先を書く側が判定する」1 つの仕組みで同時に満たせる。テストの側の約束は「環境変数を差すなら錠」の 1 つに減る。
- 作るか借りるか: 自分のパスを引くのは OS の関数 2 つ、パスをそろえるのは標準ライブラリ。自作はフォルダの比較の数行だけ。
- 削ったもの: loopback のテストの変更、錠の移し替え、`resolve_testdll` の一本化、落ちたときの片付け役、偽の DLL の単体テスト、字面の検査。

## 12. 危うさと手当て

- 判定が誤って「違う」に倒れる（パスのそろえ方の不備）: 印のテストが毎回赤になるので実装の時点で見つかる。標準エラーの 1 行が手がかり。
- 偽の DLL を直した後に i686 の成果物を作り直さず、古い DLL で回す: 赤・緑の記録が嘘になる。実装の順に「作り直す」を明記した。
- roadmap の C3-① の「触るファイル」と違う（偽の DLL の `Cargo.toml`）: 設計の議論で開発者に見せる。

## 13. 引いた文書

- [LoadLibraryW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryw) — 絶対パスならそのパスだけを探す。同じ名前のモジュールの使い回しはパスを省いたときだけ
- [Dynamic-link library search order](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order) — 検索の手順（読み込み済みの一覧を含む）は絶対パスを渡さないときのもの
- [GetModuleHandleExW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandleexw) — アドレスからモジュールを引く。参照数を変えない旗
- [GetModuleFileNameW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulefilenamew) — モジュールの完全なパス。書き方は読んだときのまま。切り詰めの返り値

---

# 実装フェーズの記録（2026-10-04）

## 14. 着手時の数え上げ（タスク 1・要件 3.1・3.4）

- やり方: 前の記録を信じず、ワークツリーの今のコードを ripgrep で引き直した。引いた語は `HOST32_TESTDLL_UNLOAD_MARKER`（`crates/` 全体）、`#[test]`・`ShioriByteProxy::load`・`shiori.dll`・`resolve_testdll`（`crates/shiori-host32-helper/src/` の全 8 ファイル）、`HOST32_TESTDLL` で始まる環境変数と錠（`crates/shiori-host32-host/tests/`・`tools/test-all.ps1`）。

### 14.1 偽の DLL `shiori.dll` を load するテスト

補助 exe のテストは全部で 37 本（`shiori_proxy.rs` の `mod tests` 6・`shiori_proxy_loadu_tests.rs` 13・`main_classify_tests.rs` 8・`main_load_ack_tests.rs` 2・`main_resolve_param_tests.rs` 6・`main_loopback_tests.rs` 1・`main_response_flavor_hung_cage_tests.rs` 1）。そのうち `shiori.dll` を写して load するのは次の 3 本だけ。

| テストの定義 | 置き場 | load の仕方 | 錠 `TESTDLL_SERIAL` |
|---|---|---|---|
| `testdll_drop_invokes_courtesy_unload`（印のテスト） | `shiori_proxy.rs` の `mod tests` | 自分の一時フォルダへ写し、`ShioriByteProxy::load` を直に呼ぶ | 取る |
| `testdll_request_roundtrip_get_and_notify`（往復のテスト） | `shiori_proxy.rs` の `mod tests` | 同上 | 取る |
| `loopback_hello_request_proxy_driven_and_bounded_loop`（loopback のテスト） | `main_loopback_tests.rs` | 自分の一時フォルダへ写し、補助 exe の窓を組んで LOAD の知らせを送る（窓の手続き `handle_message` の LOAD の枝が `ShioriByteProxy::load` を呼ぶ） | 取らない |

load しないもの（確認済み）:
- `kernel32_yields_entry_not_found`: `kernel32.dll` を読むだけ。
- `shiori_proxy_loadu_tests.rs` の群 D の 2 本: 別の DLL `shiori_loadu.dll` を読む（`load_loadu_testdll` の中・自前の錠 `LOADU_SERIAL`）。
- `main_resolve_param_tests.rs` の `env_used_when_arg_absent`: `shiori.dll` は引数の文字列として出るだけ。
- そのほかのテストは、DLL を読む呼び出しを持たない。

### 14.2 印の環境変数 `HOST32_TESTDLL_UNLOAD_MARKER` を読み書きするコード

`crates/` 全体で、この名前が出るのは 2 ファイルだけ（7 か所）。
- **読む**: 偽の DLL `crates/shiori-host32-testdll/src/lib.rs` の `unload` の定義（その説明の 1 か所と、`std::env::var` で読む 1 か所）。2 本目の偽の DLL `shiori-host32-testdll-loadu` には出てこない。
- **差す・外す**: 印のテストの定義 `testdll_drop_invokes_courtesy_unload` の中の `set_var`（load の前）と `remove_var`（最後の後片付け）だけ。
- 残りの 3 か所は説明の文: 錠 `TESTDLL_SERIAL` の説明、印のテストの説明、往復のテストの説明。
- `crates/shiori-host32-host/tests/` は、この環境変数を差さない。使うのは `HOST32_TESTDLL_DLL`（所在を読む）と `HOST32_TESTDLL_LOAD_FAIL`（`shiori_load_e2e.rs` が差して外す）だけ。
- `tools/test-all.ps1` は、`HOST32_TESTDLL` で始まる環境変数を 1 つも差さない（i686 の段は成果物を作って `cargo test` を回すだけ）。

### 14.3 結論

- **見立てと一致**。load するテストは 3 本、印の環境変数を差すのは印のテストだけ、読むのは偽の DLL の `unload` だけ、で 2 節の記録と同じだった。錠を取らないのが loopback のテストであることも同じ。
- 見立てと違うものは無いので、要件 3.4 に従って以降のタスクの対象一覧へ足すものは無い。

## 15. 直す前の赤（タスク 2・要件 3.2・2.1・2.4）

### 15.1 足したもの

- `shiori_proxy.rs` の `mod tests` に、道具 `copy_testdll_into_unique_target_dir`（札を受け取り、ワークスペースの `target\` の下に `h32m_{プロセス識別子}_{時刻のナノ秒}_{札}` のフォルダを作って偽の DLL を `shiori.dll` として写す）と、それを 2 回（札 `x`・`y`）呼ぶテスト `testdll_unload_from_another_folder_leaves_marker_untouched` を足した。
- フォルダの根は `CARGO_MANIFEST_DIR` から `parent()` を 2 回たどって組むので、パスに `..` は入らない。OS の一時フォルダの入口は呼ばない。時間待ちは無い。i686 のときだけ走る無視の印は既存の 2 本と同じ形。
- 偽の DLL は触っていない（直すのはタスク 3）。

### 15.2 実走の出力

成果物は `cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` で作った。このテストだけを回した結果:

```
> cargo test -p shiori-host32-helper --target i686-pc-windows-msvc -- testdll_unload_from_another_folder_leaves_marker_untouched
     Running unittests src\main.rs (target\i686-pc-windows-msvc\debug\deps\shiori_host32_helper-4696ad579ce35468.exe)
running 1 test
test shiori_proxy::tests::testdll_unload_from_another_folder_leaves_marker_untouched ... FAILED
thread 'shiori_proxy::tests::testdll_unload_from_another_folder_leaves_marker_untouched' (30060) panicked at crates\shiori-host32-helper\src\shiori_proxy.rs:703:9:
X has no marker: 別のフォルダ Y の写しの unload が X 宛ての印を書いた
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.66s
```

- 落ちたのは確かめ 1（X の印が無い）。Y の写しを drop した時点で、X のフォルダに `unload.marker` ができていた。競合の芯「別の写しの unload が印を書く」（3 節の経路 1・2）が、2 本のテストの間の順番に頼らずに 1 本の中で再現できた。
- 残ったフォルダの中身を見ると、X には `shiori.dll` と `unload.marker`（8 バイト・中身 `unloaded`）、Y には `shiori.dll` だけだった。Y の写しが X 宛ての印を書いたことと合う（X の写しの drop は巻き戻しの中でも走るが、その前の確かめ 1 で既に赤）。
- 行番号はこの実走の時点のもの。後で整形の都合で 1 行縮めたので、今は 702 行目が同じ確かめ。

### 15.3 補助 exe のテスト全体を回したとき

`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc`（絞らずに 38 本）を 2 回回した。
- 1 回目: 赤 2 本＝足したテストと印のテスト `testdll_drop_invokes_courtesy_unload`。印のテストの赤は、錠を取らない loopback のテストの unload が印を書いたものと見られる（3 節の経路 1。直す前の揺れそのもの）。メッセージは採り損ねたので、断定はしない。
- 2 回目: 赤 1 本＝足したテストだけ（37 本緑）。

### 15.4 後片付け

- 足したテストは確かめ 1 で panic するので、最後のフォルダの削除まで届かない。この節の実走（絞った 2 回と全体の 2 回）で `target\` の下に `h32m_*_x`・`h32m_*_y` が 1 回あたり 2 つずつ、計 8 つ残った。すべて消し、`target\` の下に `h32m_` で始まるフォルダが 0 であることを確かめた。
- 1 回目の全体の実走で印のテストが落ちたため、その一時フォルダ（OS の一時フォルダの下・既存の置き場の `host32_proxy_test_17132_…`）も 1 つ残っていたので消した。同じ場所に別のプロセスの残り（`host32_proxy_test_30400_…`・`host32_proxy_test_34796_…`）が 2 つあるが、この実走のものではないので触っていない。
- 環境変数はテストのプロセスの中だけのもので、プロセスが終われば残らない。
