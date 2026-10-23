# 試験の手引き

areka の試験の書き方と回し方の手引き。

## 実機の試験

本物の areka を子のプロセスとして立て、MCP のツールを呼び、閉じて確かめる試験は、窓口のクレート `real-run-kit`（`crates/real-run-kit/`）を使って書く。子の起こし方・空いた番号の選び方・32bit の仲介の揃え方・MCP の呼び出し・閉じ方・落ちた回の証拠は窓口が持つので、試験の側には書かない。窓口は `[dev-dependencies]` からだけ引く（`[dependencies]` へ置くと `log-capture-kit/tests/sample_path_guard_test.rs` が赤にする）。

```toml
[dev-dependencies]
real-run-kit = { path = "../real-run-kit" }
```

流れは「立てる → 呼ぶ → 閉じる」の 3 つ。例はクレートの説明（`crates/real-run-kit/src/lib.rs`）にもある。

```rust
use real_run_kit::{EndedBy, Launch, RealRun};

let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
    .launch(Launch::Sample { name: "emo2".into(), balloon: Some("emo2-kakukaku".into()) })
    .spawn();
let reply = run
    .call_until("get_log", r#"{"log_type":"status"}"#, "本物のゴースト窓を開きました")
    .unwrap_or_else(|e| panic!("get_log を呼べませんでした: {e}\n{}", run.output()));
assert!(!reply.is_error, "{}", reply.text);
let ended = run.close();
assert_eq!(ended.by, EndedBy::UserClose, "{}", run.output());
```

### 立て方

`RealRun::new(env!("CARGO_BIN_EXE_areka"))` に、起動するものを `.launch(..)` で 1 つ選び、`.spawn()` で子を起こす。立て方は次の 5 通り。

| 指定 | 起動するもの |
|---|---|
| `Launch::Sample { name, balloon }` | 検体のゴースト（`sample-ghost-kit` の名前）。その回だけの写しを取り、ゴーストのフォルダ（と同梱バルーンのフォルダ）を位置の引数で渡す |
| `Launch::Ghost { folder, balloon }` | 任意のゴーストのフォルダ（とバルーンのフォルダ）を位置の引数で渡す |
| `Launch::SampleRoot { name }` | 検体の写しの根を根として渡す（どのゴーストを起こすかは areka が根から決める） |
| `Launch::Root(path)` | 任意の根を根として渡す |
| `Launch::Empty`（既定） | その回だけの空の根 |

どの立て方でも、子には回の置き場の根・記憶の置き場（`<回>\profile`）・空いた MCP の番号・`RUST_LOG=info`・保険の自動終了（45 秒）を渡す。開発者の本物の根や記憶の置き場は受け継がない。既定は次で上書きできる。

- `.profile_dir(path)`（記憶の置き場）・`.mcp_port(n)`（MCP の番号）・`.rust_log(filter)`（記録の段）・`.safety_exit(d)`（保険の自動終了）・`.deadline(d)`（どの待ちにも使う締切・既定 60 秒）
- `.env(名, 値)`（任意の環境変数。窓口の既定より後に置くので何でも上書きできる）・`.arg(引数)`（位置の引数の後ろに足す）
- `.before_spawn(|staged| ..)`（起こす直前に 1 度だけ呼ぶ。写しの書き換えや、`staged.ensure_i686(..)` で試験用の 32bit の DLL を揃える）

`.rust_log(..)` で窓口が頼る行の出所（`areka_mcp::server`・`areka::app_exit`）が info より絞られるときは、窓口が `,<出所>=info` を足す。名指しの判定は `EnvFilter` と同じ素の前方一致なので、`areka=debug` はそのまま、`areka_mcp=debug` には `,areka::app_exit=info` が足される。

### ツールの呼び方と失敗の見分け

- `run.call(ツール, 引数の JSON)` は 1 回だけ呼ぶ。引数は JSON の object の文（`"{}"` でよい）。
- `run.call_until(ツール, 引数の JSON, 文字列)` は、答えの文にその文字列が現れるまで呼び直す。締切までに現れなければ、最後の答えを添えて panic する。
- `run.wait_for(文字列)` は、子の出力（標準出力と標準エラー）にその文字列が現れるまで待つ。

答えは `Ok(ToolReply)` で、`text` が答えの文、`is_error` が真ならツールが失敗を答えた（成功の答えと見分ける）。呼べなかったときは `Err(McpError)` で、訳は 3 つに分かれる。

| 訳 | 意味 |
|---|---|
| `McpError::ListenFailed { port, line }` | 子の記録が待受を始められなかったことを示した（`line` はその行） |
| `McpError::Unreachable { target, reason }` | つながらない・答えが読めない・締切を過ぎた |
| `McpError::ChildExited(exited)` | 呼び出しの前か途中で子が終わった（`exited.status` が終わり方） |

赤の文には `run.output()`（標準出力と標準エラーの全文）を添えると、読むときに置き場を開かずに済む。

### 閉じ方と、自分で終わるのを待つ形

- `run.close()` は利用者と同じ閉じ方（閉じる知らせ）で閉じる。答えを得たら保険の自動終了を待たずにこれで閉じる。
- `run.wait_exit()` は閉じる知らせを送らず、子が自分で終わるのを待つ。保険の自動終了や、ゴーストの台本からの終了を確かめる試験で使う。

どちらも締切までに終わらなければ、子を止めて回収してから panic する。返す `Ended` の `status` が終了コード、`by` が終わり方（`EndedBy::UserClose`＝利用者と同じ閉じ方・`SafetyExit`＝保険の自動終了・`Other(..)`＝ほかの出所・`Unlogged`＝終わり方の行が無い）。2 度目からは同じ答えを返す。どちらも呼ばずに取っ手を捨てると、窓口が静かに閉じる（締切で止めても panic しない）。

### 残した証拠の場所と読み方

回の置き場は `target\test-roots\areka-rr-<プロセスの番号>-<連番>\` に作る。成功した回は消え、落ちた回だけが残り、標準エラーに「落ちた回の証拠を残しました: <場所>」の 1 行が出る。中身は次のとおり。

| 名前 | 中身 |
|---|---|
| `stdout.log` | 子の標準出力の全文 |
| `stderr.log` | 子の標準エラーの全文 |
| `sample\` | 検体の写し（検体で立てた回だけ）。子が書き換えた後の姿 |
| `root\`・`profile\` | 子へ渡した根と記憶の置き場 |
| `.copy-work\` | 写しを複写した作業の空の棚。気にしなくてよい |

残った置き場は自動では消えない。読み終えたら自分で消す。

### 試験は panic で落とす

落ちたかどうかは、取っ手を捨てるときに panic の途中かどうかで決める。試験は `assert!`・`panic!`・`unwrap` で落とし、`Result` を返す形（`fn t() -> Result<..>`）で書かない。`Err` を返して落ちた試験は panic を通らないので、証拠が残らない。

### 32bit の仲介の建て方

32bit の SHIORI を読むゴーストには、32bit の仲介 `shiori-host32-helper.exe` が要る。窓口は立てるたびに `areka.exe` の隣へ 32bit の物を揃える（環境変数 `HOST32_HELPER_EXE` → `target\i686-pc-windows-msvc\debug` → `release` の順に探す）。どこにも無ければ、探した場所と建て方を添えて panic するので、先に建てておく。

```powershell
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
```

### 負荷の走りと同じ時間に回さない

実機の試験は本物の窓を出し、CPU と GPU を使う。負荷の走り（`areka-test-binary-stability` の走行）と同じ時間に回すと、両方の結果が乱れるので重ねない。実機の試験を 1 本ずつ回す机のことは、机の貸し出しの手順書 `doc/impl-watch.md` を見る。

### emo2 の実機の試験とくり返し

emo2 の検体を本物の areka で起こす試験は 2 本ある。どちらも環境変数で明示したときだけ子を起こし、明示しなければ素通りする（常時の `cargo test -p areka` では回らない）。回す前に排他テストの机を借りる（`doc/impl-watch.md`）。1 本ずつ回し、ほかの実機の試験や負荷の走りと重ねない。回す前に、回す回数と所要時間の見込みを示しておく。

#### 回し方

件 1 の試験（`crates/areka/tests/emo2_real_run.rs`）は、新しい写しから emo2 を 1 回起こし、印（wire 成立・装着・`wrap=BudouxWordWrap`）を待ってから利用者と同じ閉じ方で閉じ、終了コード 0 を確かめる。1 回では揺れを見落とすので、3 回続けて回す。

```powershell
$env:AREKA_EMO2_REAL_RUN="1"; cargo test -p areka --test emo2_real_run -- --nocapture
```

100% 相当で回すときは、子にだけ `__COMPAT_LAYER` を渡す `AREKA_EMO2_COMPAT_LAYER` に互換モードの値を置く（まず `DPIUNAWARE`、効かなければ `GDIDPISCALING DPIUNAWARE`）。時間の 1 行の `primary_dpi=96` で効いたことを見る（200% なら `192`）。

```powershell
$env:AREKA_EMO2_REAL_RUN="1"; $env:AREKA_EMO2_COMPAT_LAYER="DPIUNAWARE"; cargo test -p areka --test emo2_real_run -- --nocapture
```

くり返しの試験（`crates/areka/tests/emo2_first_load_repeat.rs`）は、新しい写しからの最初の起動を `AREKA_EMO2_REPEAT` の回数だけ回し、成功・LOAD の失敗・ほかの失敗を数える。基本の条件は 20 回以上回す。

```powershell
$env:AREKA_EMO2_REPEAT="20"; cargo test -p areka --test emo2_first_load_repeat -- --nocapture
```

LOAD の失敗が 1 回でも出たら、`AREKA_EMO2_REPEAT_DROP` で取り合いの相手を 1 つずつ外した 3 つの条件を、基本と同じ回数ずつ回して失敗の数を比べる。

| `AREKA_EMO2_REPEAT_DROP` | 条件 |
|---|---|
| 無い・空 | 基本。写した直後に起こし、前の回の直後に次を起こし、待つ間に MCP で面とバルーンを読む |
| `gap` | 前の回の終わりから次の起動まで 10 秒あける |
| `copy-wait` | 写しを取った後 10 秒待ってから起こす |
| `no-mcp` | 待つ間の MCP の呼び出しを止める |

```powershell
$env:AREKA_EMO2_REPEAT="20"; $env:AREKA_EMO2_REPEAT_DROP="gap"; cargo test -p areka --test emo2_first_load_repeat -- --nocapture
```

`AREKA_EMO2_REPEAT` が数でない・0 のときと、`AREKA_EMO2_REPEAT_DROP` が上の 3 つ以外のときは、子を起こさずに理由を書いて赤になる。同じ PowerShell で続けて回すときは、前に置いた環境変数が残るので、要らないものは `Remove-Item Env:AREKA_EMO2_REPEAT_DROP` などで消す。

どちらも `-- --nocapture` を付ける。付けないと、次の行が見えない。

```text
emo2_real_run: wire_ms=764 attach_ms=959 wrap_ms=2294 close_ms=19521 talk_to_wrap_ms=72 attach_to_talk_ms=1277 primary_dpi=192
emo2_first_load_repeat run=1/20 outcome=ok: wire_ms=812 attach_ms=1540 ...
emo2_first_load_repeat run=2/20 outcome=load_failed: kept=C:\...\target\test-roots\areka-rr-1234-2
emo2_first_load_repeat: condition=basic runs=20 ok=19 load_failed=1 other_failed=0
```

時間はミリ秒の整数で、見つからない値は `none`。各値の意味は `emo2_real_run.rs` の冒頭の説明にある。

所要時間の見込み（200%・debug 版で測った値）:

- 件 1 の試験: 1 回約 25 秒（22〜26 秒だった）。3 回続けて約 1 分半
- くり返しの基本の条件: 20 回で約 9 分（510 秒だった）。1 回あたり約 25 秒で見積もる
- `gap`・`copy-wait`: 1 回あたり 10 秒ずつ増える（20 回で約 12 分）。`no-mcp` は基本と同じに見込む

#### 落ちた回の証拠と読む順

落ちた回の置き場は消えずに残る（中身は上の「残した証拠の場所と読み方」）。次の順に読む。

1. 置き場を見つける。くり返しなら落ちた回の行の `kept=` に、件 1 の試験なら標準エラーの「落ちた回の証拠を残しました: <場所>」の行に場所が出る。くり返しの赤の文の終わりにも、落ちた回の一覧が並ぶ
2. `stderr.log` の `[helper]` の行を読む。`[helper] SHIORI 初期化の入口: ` の行で仲介が SHIORI のどの入口を呼んだかが分かり、`[helper] LOAD 失敗（観測・ack[0]）: ` の行で失敗の訳が分かる。訳が `LoadReturnedFalse { last_error: N }` なら、`N` は SHIORI の LOAD が 0 を返した直後に読んだ OS の最後のエラーの値。DLL がこの値を残す約束は無いので、手がかりとしてだけ読む（5＝アクセス拒否・32＝共有違反なら、ファイルの取り合いを疑う。0 なら手がかり無し）
3. `stdout.log` の `event="connect_failed"` の行と、その前後を読む（areka が SHIORI とつなげなかった時刻と、その前に起きていたこと）
4. 写しの中の SHIORI 自身の記録 `sample\ghost\emo2\ghost\master\profile\pasta\logs\` を読む。標準エラーに「落ちた回の検体の写しを複写できませんでした」の行が出ていれば、写しは途中までしか残っておらず、この記録の最後が欠けている

#### 件 2（最初の LOAD の失敗）の結論

件 2 は、200% と 100% 相当で合わせて 32 回起こして LOAD の失敗が 0 回だったので、再現しないとして記録した。根拠と各回の値は `.kiro/specs/completed/areka-P0-emo2-real-run-stability/verification.md` にある。次に同じ赤が出たら、上の順で証拠を読み、比べの 3 条件を回す。
