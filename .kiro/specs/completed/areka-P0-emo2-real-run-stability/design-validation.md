# 設計の確かめ: areka-P0-emo2-real-run-stability

- 確かめた日: 2026-10-22
- 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・`.kiro/steering/`（`product.md`・`tech.md`・`structure.md`）・`.claude/skills/kiro-validate-design/rules/design-review.md`
- 照らし合わせたコード: `crates/real-run-kit/src/`（`evidence.rs`・`run.rs`・`launch.rs`・`mcp.rs`・`close.rs`）・`crates/real-run-kit/tests/fake_evidence_test.rs`・`crates/areka/tests/emo2_real_run.rs`・`crates/shiori-host32-helper/src/`（`shiori_proxy.rs`・`main.rs`・`main_load_ack_tests.rs`・`shiori_proxy_loadu_tests.rs`）・`crates/shiori-host32-testdll-loadu/`・`crates/shiori-host32-host/src/process_host.rs`・`crates/areka/src/app_exit.rs`・`crates/areka/src/mcp/dump_surface.rs`・`crates/areka-mcp/src/tools/dump_{surface,balloon}.rs`・`crates/wintf/src/ecs/layout/systems/monitor_systems.rs`・`crates/areka-kanade/src/schedule/boot.rs`
- 実機の走行と重いビルドはしていない（読むだけ）。

## まとめ

設計は、窓口 `real-run-kit` に手を入れずに試験の側だけで件 1 と件 2 を書く形で、窓口の今の部品で全部が組めることをコードで確かめた。名指しで確かめるよう頼まれた 3 点（受け止めの中での証拠の残り方・`tests/emo2_real_run/common.rs` の置き方・`GetLastError` を読む時点）は、どれも設計のとおりに成り立つ。直すべき点は、MCP を呼ぶ形の「失敗」の線引きと、遅さの切り分けの規則の土台、保険の自動終了の値を確かめる範囲の 3 つ。どれも実装の前に設計の文を足せば済み、作りを変える必要は無いので **GO** とする。

## 名指しの 3 点の確かめ

1. **受け止め（`catch_unwind`）の中で取っ手を捨てたときに証拠が残るか — 残る。**
   窓口は `RunDirs` の破棄で `self.finish(std::thread::panicking())` を呼ぶ（`evidence.rs` 126〜129 行目）。`std::thread::panicking()` は、panic が始まってから `catch_unwind` が受け止め終えるまで真のままなので、受け止めの内側で巻き戻しの途中に捨てられた取っ手からは真が読める。窓口自身の試験 `fake_evidence_test.rs` の `kept_on_panic_child`（`catch_unwind` の中で取っ手を持ったまま panic する）が、まさにこの形で証拠が残ることを確かめている。`Run::drop` の本体は子を静かに止めるだけで、その後に欄の `dirs` が破棄されるので、子がファイルを掴んだまま複写することも無い。設計の `catch_unwind(AssertUnwindSafe(move || boot_and_close(run, ..)))` は `run` を受け止めの内側へ移すので、そのまま効く。
2. **`tests/emo2_real_run/common.rs` の置き方を cargo が受けるか — 受ける。**
   cargo が試験として建てるのは `tests/` 直下の `*.rs` と `tests/<フォルダ>/main.rs` だけで、`main.rs` の無いフォルダは試験にならない。`tests/emo2_real_run.rs` と同じ名前のフォルダ `tests/emo2_real_run/` が並んでも食い違わない。`#[path = "emo2_real_run/common.rs"] mod common;` は試験のファイルのフォルダ（`tests/`）を基準に解け、`common.rs` の中の `#[cfg(test)] #[path = "common_tests.rs"] mod tests;` は `common.rs` のフォルダを基準に解ける（`structure.md` 145 行目の決まりと同じ）。同じ形の前例が `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs` に在る。
3. **初期化の入口が 0 を返した直後に `GetLastError` を読めるか — 読める。**
   `shiori_proxy.rs` の `encode_alloc_and_load` は、入口名の行（`eprintln!`）を呼ぶ前に出し、`let ret = unsafe { entry.func()(hdir, len) };` の後は `if ret != 0` の比べだけで `Err` を返す（275〜282 行目）。`FreeLibrary` は呼び手の `load` の側（250 行目）で後から呼ばれる。だから `else` の枝の最初で `GetLastError()` を読めば、DLL が返ってからほかの OS の関数を挟まない。`GetLastError` は仲介の `main.rs` 84 行目が既に同じ形（`unsafe { GetLastError() }.0`）で使い、機能 `Win32_Foundation` は仲介にも試験用の DLL にも在る。試験用の DLL の `loadu` は、環境変数を読んだ後に `SetLastError` を置けば、0 を返すまでに値が上書きされない（`std::env::var` の後に置くこと）。記録の行の頭 `[helper] LOAD 失敗（観測・ack[0]）: LoadReturnedFalse` は `LoadReturnedFalse { last_error: 5 }` になっても部分の一致で拾えるので、分けの規則も変わらない。

ほかに確かめたこと: 窓口の `RealRun` は `safety_exit`・`deadline`・`rust_log`・`env`・`before_spawn`（写しの後・子を起こす前に 1 度）を持ち、`Launch::Sample { name, balloon }` で設計の起こし方がそのまま書ける。仲介は `areka.exe` の標準エラーを受け継いで起こされる（`process_host.rs` 262 行目・標準入出力の指定なし）ので、仲介の行は子の `stderr.log` に入る。`event="boot_talk"`（`boot.rs` 292・311 行目）と `[initialize_layout_root] Creating Monitor entity`（`dpi=`・`is_primary=`・`debug`）の行も在る。`dump_surface`・`dump_balloon` の引数はどれも省ける（`{}` で呼べる）。

## 直すべき点（3 つ）

### 1. MCP を呼ぶ形で、何を「失敗」とするかの線引きが無い

- **気がかり**: 設計は「MCP の呼び出しが子の終わり以外で失敗したら panic（ほかの失敗）」とだけ書く。ところが areka の `dump_surface` は、結線の前に呼ばれると「窓が無い」の失敗の答え（`NG:`）をすぐ返す（`dump_surface.rs` 頭の説明）。窓口の `call` は、この種の答えを `Err` でなく `Ok(ToolReply { is_error: true, .. })` で返す。実装係が「答えが失敗なら赤」と読むと、起動の直後に必ず呼ぶ基本の条件では、全部の回が「ほかの失敗」になる。
- **影響**: 要件 4.6 の分け（成功・LOAD の失敗・ほかの失敗）が基本の条件で働かず、くり返しの数が意味を失う。
- **直し方**: 共通の手続きの約束に「`ToolReply` の失敗の答え（`is_error`）は数えずに読み流す。赤にするのは `McpError::ListenFailed`・`McpError::Unreachable` だけ。`McpError::ChildExited` は輪を抜ける」と書く。呼ぶときの引数は `{}` と決めて書く。
- **要件**: 4.4・4.6・4.7
- **設計の場所**: 「System Flows／1 回の走り」の 3 つ目の項・「Error Handling／Error Strategy」の表の「MCP の呼び出しが子の終わり以外で失敗」の行

### 2. 遅さの在りかを決める規則が、起動の順を決め打ちしている

- **気がかり**: 確かめの手順 4 は「areka 自身の時間 ＝ `attach_ms` ＋ `talk_to_wrap_ms`」とし、これが旧い保険の値 3 秒を超えた回が在れば「装着そのもの」と決める。この足し算は「装着 → SHIORI の読み込み待ち → 最初の台詞 → `wrap`」と順に並ぶことを前提にしている。実際には、仲介と SHIORI の読み込みは起動の早い段で始まり、窓の装着と並んで進む。SHIORI が先に読めた回では `boot_talk` が装着より前に出るので、`talk_to_wrap_ms` が装着を待つ時間を含んでしまい、装着の時間を 2 度数える。また `attach_ms` はプロセスの立ち上がりから数えた時間なので、debug 版の 200% の画面では、それだけで 3 秒に近づきうる。
- **影響**: 規則が誤って「装着そのもの」と出すと、要件 2.3 の条件つきの直し（areka の本番のコードの変更と設計の追補）が理由なく始まる。逆に、本当に装着が遅い回を「待ち方」と読み落とすこともある。
- **直し方**: 時間の 1 行に、起動から `boot_talk` が見えるまでの時間（`talk_ms`）を足す（待たずに、記録の時刻か `wait_for` の後の読みで取る）。そのうえで「`wrap` の直前に揃ったのが装着か台詞か（`attach_ms` と `talk_ms` の遅い方）」で律速を決め、装着が律速で、かつ `attach_ms` が 3 秒を超えた回だけを「装着そのもの」とする、と書き直す。
- **要件**: 2.1・2.2・2.3
- **設計の場所**: 「確かめの手順」の 4・「共通の手続き」の `Times` の欄

### 3. 保険の自動終了の値（45 秒）を確かめる範囲が、件 1 の走りだけになっている

- **気がかり**: 確かめの手順 5 は「`wrap_ms` の最大 ＋ `close_ms` の最大 ≦ 35 秒」を件 1 の試験の数字だけで見る。保険の自動終了は子の起動から数えるので、くり返しの基本の条件では、印を待つ間の MCP の呼び出し（画像の組み立てで CPU を使う）が `wrap` を遅らせ、閉じる知らせも呼び出しが返るまで遅れる。この分は件 1 の数字に出ない。
- **影響**: 余りが削れた回で保険の自動終了が別れの台詞の途中に来ると、終わり方が `SafetyExit` になって「ほかの失敗」に数えられ、LOAD の失敗の割合を調べる数に、件 1 と同じ型の理由の分からない赤が混ざる（要件 1.3・1.5 と 4.6 の両方に掛かる）。
- **直し方**: くり返しの成功の回が出す `Times::line` も確かめの手順 5 の対象に入れ、「件 1 とくり返しの両方の最大の和」で 35 秒の線を見る、と書く。`verification.md` の 3 節（保険の自動終了の値の確かめ）にも、くり返しの数字を写すと書き足す。
- **要件**: 1.3・1.5・4.6
- **設計の場所**: 「確かめの手順」の 5・「文書の層／記録」の 3 節・「Revalidation Triggers」の 3 つ目の項

## 良いところ

- **窓口に手を入れずに、証拠の残し方を窓口の決まりに乗せた**。「破棄の時に panic の途中なら残す」決まりを、受け止めの内側で取っ手を捨てる形でそのまま使うので、ほかの spec と触るファイルが重ならず、窓口自身の試験が既にこの形を確かめている。要件 4.8・5.1〜5.3 が新しい仕組み無しに満たされる。
- **調べの道具でも赤と緑の意味をはっきりさせた**。くり返しは全部の回を回してから、落ちた回が 1 回でもあれば赤にし、まとめの行を失敗の文に入れる。LOAD の失敗の分けを `LoadReturnedFalse` まで含めた綴りで決め、`LoadLibraryFailed` などと混ざらないようにしている。仲介の手がかりも「約束のない手がかり」と型の説明に書き、読み手が過信しない形になっている。

## 小さな気づき（直すべき点には数えない）

- `structure.md` は統合テストの入口のファイルを「`mod` の宣言だけ」、共通の部品を `tests/{domain}/common/mod.rs` と書くが、設計は入口に試験の本体を書き、共通の部品を `tests/emo2_real_run/common.rs` に置く。今の `emo2_real_run.rs` も本体を持つので害は無いが、設計の「steering との合い方」の 1 文は「`tests/<領域>/` の下に置き `#[path]` で引く」までに留めておくとよい。
- 調べの記録（`research.md` 12.3 の議題 2）は `event="boot_talk"` を「順に待つ」と書き、設計は「待たない」としている。設計の方が新しいので設計に従えばよいが、直すべき点 2 を直すときに合わせて揃えるとよい。

## 判定

**GO**

- **理由**: 名指しの 3 点を含め、設計の作りは今のコードの上でそのまま成り立つ。直すべき点 3 つはどれも約束の文や確かめの手順の書き足しで済み、部品の分け方・依存の向き・触るファイルは変わらない。
- **次の一歩**: 設計の討議（`kiro-design-discussion`）で上の 3 点を議題にし、決めた文を `design.md` に足してから `/kiro-spec-tasks areka-P0-emo2-real-run-stability` でタスクを作る。
