# 設計の検証: audit-areka-install-update

- 日付: 2026-11-22
- 対象: `design.md`（854 行）・`requirements.md`・`research.md`（4 節・9 節・10 節）・steering
- 作法: kiro-validate-design（Analysis → Critical Issues → Strengths → GO/NO-GO）。開発者には何も尋ねずに書いた（非対話）。
- 確かめ方: 設計の主張のうち重いもの（指摘 13・2・12・5・23）は、コードを読んで確かめた。テストもコンパイルも回していない。

## Design Review Summary

確定した 102 件（91 を除く 1〜103）は、全部が「指摘ごとの直し」の表で直し方と組に当たっている。バグと危ない所には、赤を先に立てるテストか、檻になるテストが割り当てられている。範囲の外への手出しは、申し合わせた `switch_to` の後半と、持ち主の見直しが完了した 2 つの crate の公開の関数 1 つずつと、文書・brief の決まった行に絞られている。`Cargo.toml`・`Cargo.lock`・dola・wintf には触らない。ただし直しで新しく生まれる失敗の道が 1 つ、設計に書かれていない（長く生きる取得のスレッドが倒れたとき）。テストも 2 本、決定論でないか、的を縛れていない。どれも設計の討議で数行足せば閉じる大きさで、全体は実装へ進めてよい。

## 検証の要点（依頼の (a)〜(e)）

### (a) 指摘 13: `switch_to` の分け方と「降ろした後・起こす前」の待ち

- **窓とゴーストが 0 の間も tick が回るか（OPEN QUESTION 3）**: コードで見る限り回る。実装の最初のテストで確かめれば足り、wintf の起こしの口を足す代わりの形は要らない見込み。
  - `crates/areka/src/main.rs:196` は `WinApp::with_exit_policy(ExitPolicy::Explicit)` を使う。そのため、最後の窓が閉じても終了は指示されない（`wintf/src/runtime/mod.rs:209-224` の空遷移 hook は `Explicit` では差さない）。
  - tick は `run_async_tick` が回す（`wintf/src/runtime/tick_bridge.rs:283-305`）。起こすのは `VsyncEventBridge` で、別のスレッドの `DwmFlush` を待つ。アプリの窓の有無には依らない。
  - 今でも、`close_windows_for_restart` の後の新しい窓は投函され、後の Input 段で作られる（`app_exit.rs` の `WindowsEpoch` の doc）。窓が 0 のフレームは、今も短い間だけ在る。新しいのは、ゴーストも居ない（`GhostSlot` が空の）フレームが展開の長さだけ続くことである。読む系はどれも `Option` で受ける（research.md R5）。
- **1 フレーム遅らせる逃げが無いか**: 無い。待ちは、新しい NonSend の `BetweenHold` という状態の持ち方で表している。続きは背景の答え（`DeskAsk::Extracted`）を受けた同じ `drain` の中で走る。フレームの数で合わせる所は無い（要件 6.7 に合う）。
- **段の語と兄弟の行**: `SwitchStage` に腕を足さない。待ちの間は予約が `SendOff` のまま残る。kanade は降ろし終えているので、`on_ghost_stopped`（`ghost_switch.rs:413-470`）が誤って `switch_to` を呼び直す道は無い。`BodySystems::overwrite_between` の形（`fn(&mut World)`）も変えない。`ghost_switch_between_tests.rs` は、crate の中の `NO_BODY` を `..NO_BODY` で上書きした束を据えれば組める。`body_systems.rs` には触らずに済む。
- **待ちの間の終了**: 正規の道に乗っている。
  - 展開は背景のスレッドが `enter_write` の中で行う。`begin_close` は書く段を `EXIT_WAIT_LIMIT` まで待つ（`exit_wait.rs:124-152`）。
  - 窓口の `discard_for_exit` は `BetweenHold` を捨て、起こさない。後から届く `Extracted` と `WorkerPanicked` は、段が合わない腕か閉じた後の捨ての腕で落ちる。
  - OS のセッションの終了（`session_end.rs`）は窓の手続きで受けるので、窓が 0 の間は受けられない。今は UI スレッドが展開の間ずっと止まるので、悪くはならない。設計の 844 行もそう書いている。
  - 設計の「今の写し待ち（`CopyWait`）と同じ扱い」という喩えは正確でない。写し待ちは `take_down` より前の段（`ghost_switch.rs:616-619`）で、前のゴーストも窓も生きている。終了の扱い（捨てて起こさない）が同じだけである。直す要は無いが、読み手は混同しないこと。

### (b) 指摘 2・12: 長く生きる取得のスレッドと門

- 門の意味は正しい。`WorkGate` が持つ名は 1 つである（`exit_wait.rs:46-53`）。取得を同時に 1 本にすれば、取得中の URL は `begin_close` で `exit_wait_abandoned` に 1 件残る。書く段には入らないので待たない。完了した `areka-P0-network-update` の要件 7.5 のまま。
- 「窓口が無いとき、`url` の腕は取りに行かなくなる」という帰結は正しい。
  - 今は、`raw_sender` が窓口の無いときに受信端の無い送出端を返す（`install/desk.rs:142-147`）。それでも `url` の腕は先に `start_fetch` で取りに行き（`install_cue.rs:151-154`）、落とし終えた後の送出で初めて失敗する。
  - 直した後は、URL そのものを窓口へ送るので、その場で `install_cue_send_failed` になって終わる。本番では通らない道で、記録も残る。OPEN QUESTION 2 の仮の答えのまま進めてよい。
- **抜けている所**: 取得のスレッドが倒れたとき・起こせないときの窓口の戻し方。下の重大な指摘 1。

### (c) 指摘 5・23: 倒れの守り手

- 形は正しい。
  - Drop の中で `std::thread::panicking()` を見て、`error!`・`gate.end()`・`WorkerPanicked` の順に行う。
  - `gate.end()` は書く段の旗も下ろす（`exit_wait.rs:72-78`）。終了が上限まで待たされることは無くなる。
  - 素の `OrderDone` を送らず、別の腕にして `worker = None` にする理由は research.md R10 で示されている。死にかけの受信端へ次の依頼が黙って消える穴を、正しく避けている。
- `update` で守り手を `ports` より先に作る順も、Drop の順（逆順）と合っている。
- 実装で気をつける所が 1 つある。窓口の `answer` は、閉じた後の頼みを捨てる腕（`install/desk.rs:300`）より前で `OrderDone` を拾っている。`WorkerPanicked` も同じく、その腕より前で拾うこと。閉じた後に届いても `busy` を下ろせるようにするためである。

### (d) 確定した指摘の全部への当て

- 「指摘ごとの直し」の表は、1〜103 を欠けなく並べている（15〜17・20・21・41・43・51・52・53・55・70・71・82・83・89・90・92 はまとめた行）。91 は「裁定で残す」。
- 赤を先に立てるテスト: 1・2・3・5・12・13・14・23・24・40・50・65・66・67・68。檻になるテスト: 7・22・30・42・64・88・96・97 ほか。扱いは Testing Strategy に分けてある。
- 「—」の行は、doc・名・写しの寄せ・削りに限られている。要件 4.1 の「バグと危ない所」に当たるのに檻が無い行は無い。
- 決定論で無いテスト・的を縛れないテストが 2 本ある。下の重大な指摘 2。

### (e) ウェーブ C26 の持ち物

- `audit-emo2-boot-switch` の持ち物には、申し合わせた所だけを触る。`ghost_switch.rs` の `switch_to`（597〜708 行）、末尾の `mod` の行、新しい子のファイル 2 つである。`take_down`・`boot_into`・`SwitchStage`・`on_ghost_stopped`（向こうの行 40〜44・66〜90・111・250〜333・423・522）には触れない。子のファイルは `#[path]` で `ghost_switch.rs` から引くので、`lib.rs`（`audit-areka-app` の持ち物）にも触らない。
- `audit-areka-app` の持ち物（`body_systems*`・`alert*`・`log_history*`・`reload*`・`exit_wait*`・`main.rs`・`png_image*`）は呼ぶだけである。94 の `alert.rs`・`alert_tests.rs` は、相手が `completed/` へ移った後に限る。31・33・85・103 は相手の着地の後に回す。
- `Cargo.toml`・`Cargo.lock`・`crates/dola/`・`crates/wintf/`・`vendors/` は触らない。差分 0 を G25 で確かめる手順も在る。
- `areka-runtime/src/consumer_ledger.rs:77` の 1 語は、OPEN QUESTION 6 の答えしだいである。持ち主が居ない件なので、討議で決めれば足りる。

## Critical Issues

### 重大な指摘 1: 長く生きる取得のスレッドが倒れたとき・起こせないときの戻し方が無い

- **Concern**: 直しでは、窓口が `fetching: bool` を持ち、取得のスレッドへ URL を 1 件ずつ渡す。ところが次のことが設計に無い。
  - 「取得の終わりの知らせ」を運ぶ型。`RawInstallRequest` は `Path` と `Url` の 2 つの腕だけで、ほかに運ぶ先が無い。
  - 取得に失敗した道（`fetch_and_send` の `error!` で依頼 0）でも、その知らせを送ること。
  - スレッドが倒れたとき（`WinHttpFetch` の中の panic）の守り手。
  - `fetch_tx.send` が失敗したときに、スレッドを起こし直すこと。
  - スレッドを起こせなかったときに、`fetching` を下ろして次の URL へ進むこと。
- **Impact**: 知らせが 1 回でも来なければ、`fetching` は真のまま残る。その後の `\![execute,install,url]` は全部待ち行列に積まれるだけになり、プロセスの終わりまで黙って止まる。5・23 で直すのと同じ種類の穴で、今の 1 件ごとに起こす形には無かったものを、この直しが新しく作る。要件 6.2（記録の無い失敗の道を作らない）と 6.3 に反する。
- **Suggestion**: 次の 3 つを設計に足す。
  - 取得のスレッドの URL ごとの処理に守り手を置く。5・23 と同じ形で、どの道で抜けても（成功・失敗・panic）`gate.end()` と「取得の終わり」を送り、panic なら `error!(install_fetch_panicked)` を残す。
  - 「取得の終わり」は窓口が受ける腕として型に置く。`RawInstallRequest` の第 3 の腕でも、`DeskAsk` の腕でもよい。
  - 窓口は `fetch_tx.send` の Err と起こせないときに `fetch_tx = None`・`fetching = false` にし、次の URL で起こし直す。
  - `fetch_queue_tests.rs` に 2 本足す。偽の取得口が Err を返したときと panic したときに、2 本目の URL が依頼になること。
- **Traceability**: 要件 6.2・6.3・4.1、指摘 2・12（5・23 と同じ種類）
- **Evidence**: design.md「取得の待ち行列（指摘 2・12）」の State Management（498〜509 行）、System Flows「URL からの取得」（363〜385 行）

### 重大な指摘 2: 指摘 28・4 のテストが、決定論でないか、直しの的を縛れていない

- **Concern**:
  - 28 のテストは、「`handle` の戻りの時点で `try_answer` が `None`」を見て、UI の側で読まないことを確かめる。これは走る順の偶然に頼る。名前つきのスレッドが小さな検体を読み終えるのは速いので、負荷の高い全体テストで呼び手のスレッドが待たされると、`Some` になって赤になりうる。既にある前例（`dump_surface_tests.rs:291`・`dump_surface_gpu_tests.rs:422`）は、答えがフレームを回した後でしか作られない形なので決定論である。28 はその形に当たらない。
  - 4 のテストは、「控えた JoinHandle が `apply` の戻りの時点で在る」を見る。これでは、`survey` を UI スレッドで済ませてから記録だけをスレッドへ出す退行でも緑のままで、指摘 4 の的（UI スレッドが根を見に行かない）を縛れない。しかも、テストのためだけに本番の NonSend へ JoinHandle を控える。これは、設計が退けた案（「本番の形を試験のために曲げる」）と同じ種類である。
- **Impact**: 要件 4.3（決定論）に反するテストが全体テストへ入る。4 は、直しが戻っても落ちない檻になり、要件 4.1 を満たさない。
- **Suggestion**:
  - 28: 時点の比べをやめる。決定論の形で「読みが UI スレッドの外で走る」ことを縛る。例えば、読む段の `debug!` を名前つきのスレッドから出し、`capture` の窓は呼び手のスレッドだけを取る性質（`log-capture-kit` の `does_not_capture_events_from_outside_the_window_or_other_threads`）を使う。その語が呼び手のスレッドの記録に 0 件で、答えは `wait_answer` で届くことを見る。どうしても縛れないなら、「縛らない」と理由を討議で決めて記録する。
  - 4: 同じく、`survey` の段の記録の語が `apply` を呼んだスレッドの `capture` に 0 件で、記録のスレッドに dispatcher を持たせた窓では 1 件以上、という形にする。そうすれば JoinHandle を本番に控えずに済む（待ちには `join_thread_bounded` が使える戻り値か、テストだけの口を使う）。
- **Traceability**: 要件 4.1・4.3、指摘 4・28、OPEN QUESTION 7
- **Evidence**: design.md「指摘ごとの直し」の 4・28 の行（557・578 行）、「設計の段で決めた事項」の 2（702 行）、Open Questions 7（848 行）

## Design Strengths

1. **持ち物の境と状態の持ち方**: 13 の直しは、展開を背景へ戻すという大きな変更である。それでも `SwitchStage` の腕も `BodySystems` の口の形も変えず、兄弟の行（423 行の照合など）を避けて、`BetweenHold` の有無だけで待ちを表している。1 フレーム遅らせる逃げも、ベースウェアの判断やモードも足していない。兄弟と申し合わせた行の範囲まで具体的に書かれている。
2. **調べの裏付け**: research.md 10.1 の R2〜R12 は、ほとんどの主張にファイルと行の根拠を持つ。具体例は次のとおり。
   - 素の `OrderDone` を送ると依頼が黙って消える落とし穴（R10）。
   - `update_reload_requested` の範囲の外の読み手が台帳 1 か所だけであること（R6）。
   - `compose_alone` ではアニメーションの段が合わない理由（指摘 3）。
   - 窓口が無いときの `url` の腕の変化（OPEN QUESTION 2）。

   これらはどれも、この検証でコードと突き合わせて正しかった。

## Final Assessment

- **Decision**: GO
- **Rationale**: 範囲の外への手出しは申し合わせのとおりで、確定した指摘は全部が直しに当たっている。いちばん重い 13 の待ちも、tick が窓 0 で回ること・終了の道が正規であることがコードで裏付けられた。残る 2 件は、設計の討議で数行を足せば閉じる。
- **Next Steps**:
  1. 設計の討議（`/kiro-design-discussion audit-areka-install-update`）で、重大な指摘 1・2 を議題にし、設計へ書き足す。
  2. OPEN QUESTION 3 は、上の (a) の根拠（`ExitPolicy::Explicit`・`DwmFlush` で回る tick）を添えて「回る」と閉じる。代わりの形（wintf の起こしの口を叩く）は消してよい。実装の最初のテストで確かめる手順は残す。
  3. 実装で気をつける所（討議の議題でない）:
     - (c) の、`WorkerPanicked` を閉じた後の捨ての腕より前で拾うこと。
     - (a) の、写し待ちとの喩えの言い直し。
  4. 討議の後に `/kiro-spec-tasks audit-areka-install-update` へ進む。

## Interactive Discussion

この検証は、開発者に尋ねずに書いた。上の 2 件は、設計の討議の議題の候補として渡す。どちらも推す答えを Suggestion に書いた。
