# shiori2-protocol 検証の記録（下書き）

5.x の実機の結果はこのファイルへ足していく。

## 4.3 3.0 の回帰（要件 6.1・6.2・6.3・7.5）

### 決定論のテスト（2026-10-23・机なし）

前準備: `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`・`cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc`（通信の層の結合テストが要する i686 の成果物）。

| 命令 | 結果 |
|---|---|
| `cargo test -p shiori-host32-host` | 緑・7 実行ファイル・202 通過・0 失敗・1 除外（`console_window_probe_child`＝別のテストが子として起こす観測の本体で、既定で除外の印つき） |
| `cargo test -p areka-kanade` | 緑・3 実行ファイル・773 通過（単体 688・結合 85）・0 失敗・0 除外 |
| `cargo test -p areka-ghost` | 緑・4 実行ファイル・253 通過（単体 207・結合 40・`real_pasta_online` 6）・0 失敗・0 除外 |

この回では `HOST32_PASTA_DLL` を与えていないので、条件つきの `real_pasta_online::emo2_does_not_start_chatting_while_online`・`snapshot_capture_test::capture_real_pasta_snapshots`（と `real_pasta_test`）は門で抜けて「通過」に数えられている。実物の pasta で回した結果は次の節。

### 確かめの 1 本と記録する要求の並び（静的）

確かめの往復は `crates/areka-ghost/src/shiori_wiring.rs` の `real_connect` の閉包の中で、LOAD の ack の直後・`ShioriConnection` を返す前に 1 回だけ行う。条件つきの 2 本は交信の記録（`Recorder`）で `real_connect` の返した接続を包むので、確かめの 1 本は記録の外で済み、記録する要求の並びには入らない。

### 実物の pasta と中継で回る条件つきの 2 本（机が要る）

置き場はすべてこのワークツリーの `target\` の下。

#### 1 回目（2026-10-23）: 2 本とも赤

`HOST32_PASTA_DLL` には、`vendors/sample_ghost/emo2.nar` の `ghost/master/pasta.dll` を `target\s2p-4-3\pasta.dll` へ単体で取り出したものを渡した。`TMP`・`TEMP` は `target\s2p-4-3\tmp` に向けた。
- `real_pasta_online`: 失敗 1・通過 5。helper の LOAD が `LoadReturnedFalse { last_error: 203 }` で失敗し、記録は空のまま挨拶を 120 秒待って panic した（`real_pasta_online.rs:192`）。記録が空なので、要求の並びは比べられていない。
- `capture_real_pasta_snapshots`: テストのスレッドがスタックを使い切った（`STATUS_STACK_OVERFLOW` `0xc00000fd`）。書き出しは 0 件。
- ログ: `target\s2p-4-3\online.log`・`target\s2p-4-3\snapshot.log`

調べた結果（机なし）:
- スタックの溢れは、こちらの命令の誤り。`write_real_pasta_ghost_fixture` は DLL の親のフォルダを丸ごと再帰で写す。そのフォルダ（`target\s2p-4-3`）の中に一時フォルダ（`TMP`）を置いたので、写し先が写し元の中にあり、`copy_dir_contents_recursive` が終わらずに再帰を重ねた（`snapshot_capture_test.rs` の 230〜263 行目）。本 spec の差分は関わらない。また、このテストは DLL の親が本物の emo2 の `ghost/master`（`dic/`・`scripts/`・`pasta.toml` 込み）であることを前提にしている。
- LOAD の失敗は、確かめの往復より前の段で起きている（`real_connect` の手順 5。確かめは手順 6）。本 spec の差分（`git diff main...HEAD`）は 32 ビットの仲介（`shiori-host32-helper`）にも、LOAD を送る手順 1〜5 にも触れていない。完了 spec `areka-P0-emo2-real-run-stability` が扱った「新しい写しからの最初の LOAD が `LoadReturnedFalse` で断られる」件と同じ形で、あの spec は 32 回くり返して再現せずに閉じている。今回の `last_error: 203`（ERROR_ENVVAR_NOT_FOUND）は、あの spec が「次に出たときの手がかり」として仲介に足した値の、初めての観測になる。失敗した回の `profile\pasta\logs\pasta.log` は、写しを消すので残っていない。

#### 2 回目に回す命令

DLL は emo2 の `ghost/master` を丸ごと `target\s2p-4-3\emo2\ghost\master` へ取り出した中のものを渡す。一時フォルダはその外（`target\s2p-4-3\tmp`）に置き、snapshot の 1 本のときだけ向ける。

```powershell
$wt = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-shiori2-protocol-22a93e'
$env:HOST32_PASTA_DLL = "$wt\target\s2p-4-3\emo2\ghost\master\pasta.dll"
cargo test --manifest-path "$wt\Cargo.toml" -p areka-ghost --test real_pasta_online -- --nocapture
$env:AREKA_SNAPSHOT_OUT = "$wt\target\s2p-4-3\snapshots"
New-Item -ItemType Directory -Force "$wt\target\s2p-4-3\tmp" | Out-Null
$env:TMP = "$wt\target\s2p-4-3\tmp"; $env:TEMP = $env:TMP
cargo test --manifest-path "$wt\Cargo.toml" -p areka-ghost --test ghost snapshot_capture_test::capture_real_pasta_snapshots -- --nocapture
```

結果（2026-10-23・机を持って 1 回）: 2 本とも緑。
- `real_pasta_online`: 通過 6・失敗 0（79.16 秒）。LOAD は通り、`emo2_does_not_start_chatting_while_online` は記録の並びに置いた主張（起動の挨拶・`online` の間の雑談 0 件・守り手を落とした後の応答）をすべて満たした。
- `capture_real_pasta_snapshots`: 通過 1・失敗 0（1.37 秒）。書き出しは GET の初出の 3 件（`OnFirstBoot`・`OnTranslate`・`username`）で、確かめの 1 本（`GET Version`）は記録に入っていない。
- ログ: `target\s2p-4-3\online2.log`・`target\s2p-4-3\snapshot2.log`

確かめの 1 本が先に届いても、記録する要求の並びは変わらなかった（要件 6.1・6.3）。1 回目の LOAD の失敗は 2 回目では出なかった。

### 4.1 で変えた kanade のテスト（`ShioriConnection` の書き下しに `version: ShioriVersion::Shiori3` の 1 行を足しただけ）

- `crates/areka-kanade/tests/kanade/common/common_window_actor.rs`
- `crates/areka-kanade/tests/kanade/idle_pump_test.rs`
- `crates/areka-kanade/tests/kanade/real_helper_test.rs`

4.3 で変えたテスト: なし。
