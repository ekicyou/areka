# Brief: areka-P0-emo2-real-run-stability

> 2026-10-11 `/kiro-discovery` で起票（開発者の指示「バグの起票は粒度が小さい。修正範囲がかぶらない範囲でいくつかに集約」）。emo2 を実機で起こす走行のバグの起票 2 本を 1 本にまとめた。元の brief は下の「件 1〜2」に全文を写してある（見出しを 1 段下げただけ・中身は変えていない）。場所と行数は元の起票（10-08・10-10）と棚卸㉓（main `ee3af616`）のときのもの＝着手時に引き直す。

## Problem

areka.exe を emo2 で実機に起こす走行が、2 つの形で赤になる。どちらも「実機の窓を出して emo2 を起こし、記録を読む」という同じ手順で確かめる件で、負荷を掛ける検査と同じ時間には回せない。別々の spec にすると、同じ足場を 2 回組むことになる。

- **件 1（旧 `emo2-real-run-wrap-timeout`）**: 明示したときだけ走る `emo2_real_run`（`AREKA_EMO2_REAL_RUN=1`）が、`wrap=BudouxWordWrap` を待つ所で赤になる。
- **件 2（旧 `shiori-load-first-run-flake`）**: emo2 を新しい写しから起こした走行が、1 回だけ、SHIORI（pasta.dll）の LOAD の失敗で終わった。

## Current State

- 件 1 は手元で赤を見た。原因の見立て（200% の画面の debug ビルドで、起動から文字の取り付けまで約 2.5 秒＝3 秒の自動終了に間に合わない）はまだ測っていない。
- 件 2 は 1 回見ただけで再現していない。原因は確かめていない。失敗した回の pasta.dll の側の記録は残っていない。
- 触るファイル: 件 1 は `crates/areka/tests/emo2_real_run.rs`（247 行）だけで済ませるのが第一。件 2 は再現しなければ 0、防げると分かれば仲介（`crates/shiori-host32-helper/src/shiori_proxy.rs`）か起こし直しの順。
- 細部は各件の節。

## Desired Outcome

- 件 1: `emo2_real_run` が、自動終了の残り時間に頼らず、印が出たことの観測で待って、200% と 100% 相当の両方で緑。
- 件 2: 再現の割合が数えてあり、失敗した回の証拠が残る手順になっている。areka の側で防げるなら直してあり、防げないなら記録して閉じてある。

## Approach

**件 1 → 件 2 の順**。件 1 で `emo2_real_run` が安定して通るようになれば、件 2 の「新しい写しからの最初の起動を数十回くり返す」足場にそのまま使える（くり返しの各回で、失敗したらゴーストの写しを別の名前で残す）。

1. 件 1: 起動から取り付けまでの時間を測り、遅いのが取り付けそのものか待ち方かを切り分ける。印が出るまで待つ有界の待ちにする（自動終了を延ばすだけの直しは採らない）。
2. 件 2: 上の足場でくり返して割合を数え、取り合いの相手を 1 つずつ外して比べ、仲介が LOAD の失敗で残せる情報を読む。結果で閉じ方を決める。

## Scope

- **In**: 件 1 の赤の再現と時間の測り・テストの待ち方の直し（または取り付けの遅さの原因の直し）・200% と 100% 相当での確かめ／件 2 の再現の試み・失敗した回の証拠の残し方・原因の切り分け・（areka の側で防げると分かったとき）その直し・（防げないとき）記録。
- **Out**: 折り返しそのものの振る舞い（完了 `budoux-reveal-reflow` 系）・常時のテストへの組み込み（要るなら議題）・pasta.dll の中の直し（別のリポジトリ）・SHIORI の失敗の伝え方そのもの（完了 `shiori-fault-notice`）・書けない場所に在るときの伝え方（`write-failure-notice`）・置き場を入れ先の外へ出すこと（`user-data-root`）。

## Boundary Candidates

- テストの待ち方（`emo2_real_run.rs`）／起動から文字の取り付けまでの時間（`emo2_boot` の装着の段）。
- 実機の走行の手順（写しの作り方・前の走行との間隔）と、失敗した回の証拠の残し方。
- 仲介が LOAD の失敗で残す情報／前の SHIORI のプロセスの終わりと次の LOAD の順。

## Out of Boundary

- emo2 の台本・pasta の SHIORI の振る舞い。
- 負荷の下のテストの揺れ（`areka-test-binary-stability`）。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: emo2 で通しを確かめる実機の確認すべて（`mcp-kanade-tools` ほか、MCP で実機を回す spec）。

## Existing Spec Touchpoints

- **Extends**: 完了 `shiori4-test-ghost`（emo2 の検体と `emo2_real_run.rs`）。
- **Adjacent**: `areka-test-binary-stability`（実機の窓が出る検査＝あちらの負荷の走行と同じ時間に回さない）・`emily-ghost-verification`・`shiori4-api`（SHIORI の `load` へ渡すパスの形を疑い先に持つ＝再現の調べでパスの形が効いていたら渡す）・`test-roots-under-target`・`install-live-target-hazards`・`real-run-log-noise`（実機の走行の記録の雑音）・`main.rs`（950 行）を触る spec（件 1 でアプリの側に「印が出てから閉じる」を作る形を選んだときだけ）。

## Constraints

- 段は**バグ**・区分 B。件 2 は確かめが本体。
- 規模の見立て: 4〜13 タスク（件 1＝3〜6・件 2＝確かめと記録だけ 1〜3／直すなら 4〜7）。
- 検査の側だけで直すなら、ほかの未完了の spec と触るファイルの重なりは 0。
- 実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ。emo2 の実走は絶対パスかつ短いパス。
- 止めてよいプロセスは、自分が起こしたと確かめられるものだけ。くり返しの走行は、回す前に所要時間を伝える。
- 要件定義のモデル: Opus。

---

## 件 1: 旧 `areka-P0-emo2-real-run-wrap-timeout`（元の brief の全文）

> 起票: 2026-10-08（`areka-P0-wintf-tooltip` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。実機で赤を見た。原因の見立ては記録の時刻から立てたもので、まだ確かめていない。

### Problem

明示したときだけ走る結合テスト `crates/areka/tests/emo2_real_run.rs`（`AREKA_EMO2_REAL_RUN=1`）が、折り返しの解決の証跡 `wrap=BudouxWordWrap` を待つ所で赤になる。areka.exe を emo2 で実際に起動して確かめる唯一の自動の通しが、手元で使えない。

### Current State

2026-10-06〜08（`wintf-tooltip` のブランチ・タスク 1.1 の確かめ）:

- テストは areka.exe を `AREKA_APP_SMOKE_EXIT_MS=3000`（3 秒で自動終了）で起動し、ログに wire の成立・取り付けの完了・`wrap=BudouxWordWrap` が出ることを確かめる。
- 192 DPI（200%）の画面で debug ビルドを走らせると、文字の取り付けまでに起動から約 2.5 秒かかり、3 秒の自動終了に間に合わず `wrap=BudouxWordWrap` が出る前に終わる見込み。
- `wintf-tooltip` が areka.exe に足したマニフェスト（comctl32 の版 6）を外しても同じく赤＝この spec より前からの問題。
- 常時のテスト（`tools/test-all.ps1`）には入っていないので、全体テストは緑のまま。

### Desired Outcome

`AREKA_EMO2_REAL_RUN=1` の通しが、手元の 200% の画面の debug ビルドでも安定して緑になる。待つ時間を「自動終了までの残り」に頼らず、観測（ログに印が出たこと）で待つ。

### Approach

まず起動から取り付けまでの時間を測り、遅いのが取り付けそのものか、テストの待ち方かを切り分ける。自動終了を延ばすだけで済ませず、印が出てから閉じる形（または印が出るまで待つ有界の待ち）にする。

### Scope

- **In**: 赤の再現と時間の測り・テストの待ち方の直し（または取り付けの遅さの原因の直し）・直した後に 200% と 100% 相当で緑の確かめ。
- **Out**: 折り返しそのものの振る舞い（`budoux-reveal-reflow` 系）・常時のテストへの組み込み（要るなら議題）。

### Boundary Candidates

- テストの待ち方（`emo2_real_run.rs`）／起動から文字の取り付けまでの時間（`emo2_boot` の装着の段）。

### Out of Boundary

- emo2 の台本・pasta の SHIORI の振る舞い。

### Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: emo2 で通しを確かめる実機の確認すべて。

### Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-shiori4-test-ghost`（emo2 の検体と `emo2_real_run.rs`）。
- **Adjacent**: `ghost-session-test-load-flake`（壁時計の締切を観測の待ちへ置き換える方針が同じ）。

### Constraints

- 実機の一時フォルダはワークツリーの `target\` の下だけ。emo2 の実走は絶対パスかつ短いパス。

### 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 起票（10-08）の後、関係のファイルは変わっていない。検査 `crates/areka/tests/emo2_real_run.rs`（247 行）は自動終了 3,000 ミリ秒・見張りの締切 120 秒のまま。待っている証跡 `wrap=BudouxWordWrap` は、文字の面を初めて装着したときの記録（`crates/areka-emo-text/src/actor_present.rs` の「テキスト供給面を予約スロットへ装着した」）に今も出る。`budoux-reveal-reflow` はこの記録を変えていない。
- **触るファイル**:
  - `crates/areka/tests/emo2_real_run.rs` だけで済ませるのが第一。
  - アプリの側で「印が出てから閉じる」を作るなら、自動終了の仕掛け（`crates/areka/src/main.rs`・950 行で上限の近く）に触る。`main.rs` は `mcp-kanade-tools`・`mcp-stdio-bridge` なども触る。
  - 遅さの原因が装着の側にあった場合だけ `crates/areka/src/emo2_boot/` の装着の段。
- **規模**: S（3〜6 タスク）。
- **先に要るもの**: 無い。
- **優先度の区分**: B（バグ。areka.exe を emo2 で起動して確かめる唯一の自動の通しが、手元で赤）。
- **要件定義のモデル**: Opus。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - 検査の側だけで直すなら、ほかの未完了の spec と触るファイルの重なりは 0（このファイルに触ると書く brief は本 spec だけ）＝どのウェーブにも置ける。`anchor-tag-canon` とも並べられる。
  - 実機の窓が出る検査なので、負荷を掛ける検査（`areka-test-threads-av`・`test-wait-marker-gaps`）と同じ時間には回さない。
  - 原因の見立て（起動から装着まで約 2.5 秒）はまだ測っていない＝最初のタスクで測る。自動終了を延ばすだけの直しは採らない。

---

## 件 2: 旧 `areka-P0-shiori-load-first-run-flake`（元の brief の全文）

> 起票: 2026-10-10（`areka-P0-seriko-trigger-intervals` の実機の確かめ〔タスク 9.2〕で 1 回だけ見た件。範囲の外の問題として、完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 9.2 の ⑦ と、その回の記録（下の Current State に要る行を写した。記録の置き場 `target\` は git に残らない）。**見たのは 1 回だけで、再現していない。原因は確かめていない。** 区分 B（バグ・観察）。

### Problem

emo2 を新しい写しから起こした実機の走行が、1 回だけ、SHIORI（pasta.dll）の LOAD の失敗で終わった。同じ手順でゴーストの写しを作り直して走らせたら通り、その後は起きていない。

- **利用者**: もし利用者の機械でも起きるなら、「入れたばかりのゴーストが、最初の 1 回だけ起動に失敗する」形になる（起きるかどうかは分かっていない）。
- **開発者**: areka の側の記録には「LOAD が成功の返事を返さなかった」までしか残らず、pasta.dll がなぜ断ったのかが分からない。実機の確かめを自動で回すと、理由の分からない赤が 1 回混ざる。

### Current State

2026-10-10 に見た 1 回（`seriko-trigger-intervals` のブランチ・debug 版の `areka.exe`・開発機 1 台）。

- **手順**: 同梱の emo2 をワークツリーの `target\` の下へ写し（毎回、前の写しを消してから写し直す）、`surfaces.txt` の末尾に検体の面を足し、`boot.lua` を差し替えて、`areka.exe` をそのフォルダの絶対パスで起こす。有界の自動終了つき。MCP の `dump_surface`・`dump_balloon` を別のプロセスから繰り返し呼んでいた。同じ日に、同じ手順の走行を前後で 5 本通している（どれも exit 0）。
- **失敗した回の記録**（areka の標準出力と、32bit の仲介 `shiori-host32-helper` の標準エラー）:
  - 起動は 11:49:30.53（UTC）。11:49:31.2〜31.9 に絵の焼き込み・seriko の表・文字の層の装着まで進んだ。
  - 仲介の標準エラー: `[helper] SHIORI 初期化の入口: loadu` → `[helper] LOAD 失敗（観測・ack[0]）: LoadReturnedFalse` → `Error: Error { code: HRESULT(0x80004005), message: "エラーを特定できません" }`。
  - 11:49:32.733 `ERROR shiori-actor: SHIORI 接続確立に失敗 … event="connect_failed" reason=SHIORI の LOAD が成功 ack [1] を返さなかった（proxy 未確立）: [0]`。
  - 11:49:32.734 `ERROR kanade: SHIORI 呼出失敗——終了系列（Fault）へ event="shiori_failed"`。
  - 11:49:32.775 `ERROR areka::alert: [alert] 利用者へ告げます event="alert" scene=ShioriFault { … kind: ConnectFailed … }`。
  - 11:49:32.82 に MCP の待受を閉じて終了。走らせた側の測りでは、起動から 3.6 秒で終了コード 1。
- **残っていないもの**:
  - pasta.dll の側の記録。pasta.dll は、ゴーストのフォルダの `ghost\master\profile\pasta\` の下に自分のスクリプトの展開先（`pasta_scripts`）・`logs`・`cache`・`save` を作る。失敗した回のそのフォルダは、次の走行の前に写しごと消して作り直したので残っていない。
  - 走らせた側（Claude のセッション）は、失敗の直後に「アクセスが拒否されました (os error 5)」という文言を、pasta.dll が `profile\pasta\pasta_scripts` を展開する所の記録として見た、と書き残している。その文言を含むファイルは残っていない＝**手がかりであって、確かめた事実ではない**。
  - areka の記憶の置き場（その回は `AREKA_PROFILE_DIR` で走行ごとのフォルダへ向けていた）は空だった（書く所まで進んでいない）。
- **areka の側の伝え方は働いている**: 接続の失敗は記録に `ERROR` 3 行で残り、利用者へ告げる経路（`alert`）も通っている。黙って失敗した経路ではない。分からないのは、DLL の中で何が起きたか。
- **疑っているが確かめていない所**:
  - 新しい写しでは、pasta.dll が最初の LOAD でスクリプトを展開する。その書き込みが、何かと取り合った（直前の走行の `areka.exe` か仲介のプロセスがまだ終わりきっていなかった／写しを作る Python がまだファイルを掴んでいた／ウイルス対策の走査が、写したばかりのファイルを掴んでいた）。
  - 通った 5 本と失敗した 1 本の手順の違いは見つけていない。
- 近い過去の件: 完了 `areka-P0-ghost-session-test-load-flake` は、負荷の下で名前替え（`areka-nar` の確定）が OS に断られる揺れを「試し直す」形で直した。あちらは areka 自身の書き込みで、こちらは SHIORI の DLL の中の書き込みなので、同じ直しは当てられない。

### Desired Outcome

次のどれかで閉じる。

- 再現の条件が分かり、areka の側で防げるなら防ぐ（例: 前のゴーストのプロセスが終わりきるのを待ってから LOAD する、など）。
- areka の側では防げない（DLL の中・OS・ウイルス対策の側）と分かったら、その根拠を記録して閉じる。そのとき、利用者に見える形（最初の 1 回だけ失敗しうる）が残るなら、既定ゴーストの台詞で伝わることを確かめる。
- 再現しないなら、試した回数と条件を記録して閉じる。
- どの場合も、**次に同じ赤が出たら原因が読める**ようにしておく（失敗した回のゴーストの写しを消さずに残す手順・仲介が拾える情報が在れば記録に足す）。

### Approach

確かめが本体。

1. 新しい写しからの最初の起動を、同じ手順で数十回くり返して、失敗の割合を数える（1 回は数秒。窓が出るので、負荷を掛ける検査と同じ時間には回さない）。失敗した回は、ゴーストの写しを消さずに別の名前で残し、`ghost\master\profile\pasta\logs` を読む。
2. 取り合いの相手を 1 つずつ外して比べる: 前の走行の終わりから次の起動までを空ける／写した直後に待つ／MCP の呼び出しを止める。
3. 仲介（`crates/shiori-host32-helper`）が LOAD の失敗のときに残せる情報が在るかを読む（今は `LoadReturnedFalse` と HRESULT だけ。SHIORI の `load` は真偽しか返さないので、増やせるのは OS の最後のエラーくらいの見込み）。
4. 結果で、上の Desired Outcome のどれで閉じるかを決める。

### Scope

- **In**: 再現の試み・失敗した回の証拠の残し方・原因の切り分け・（areka の側で防げると分かったとき）その直し・（防げないとき）記録。
- **Out**: pasta.dll の中の直し（別のリポジトリ）。SHIORI の失敗の伝え方そのもの（完了 `areka-P0-shiori-fault-notice` で確定。変えない）。書けない場所に在るときの伝え方（`write-failure-notice`）。置き場を入れ先の外へ出すこと（`user-data-root`）。

### Boundary Candidates

- 実機の走行の手順（写しの作り方・前の走行との間隔）と、失敗した回の証拠の残し方。
- 仲介が LOAD の失敗で残す情報（`crates/shiori-host32-helper/src/shiori_proxy.rs`）。
- ゴーストの起こし直し・切替のとき、前の SHIORI のプロセスの終わりと次の LOAD の順（areka の側で防げると分かったときだけ）。

### Out of Boundary

- emo2 の台本・pasta の振る舞い。
- 負荷の下のテストの揺れ（`areka-test-threads-av`・`test-wait-marker-gaps`）。

### Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: emo2 を新しい写しから起こす実機の確かめすべて（`mcp-kanade-tools` ほか、MCP で実機を回す spec）。

### Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**（読んで、重なりが無いことを確かめた）:
  - `emily-ghost-verification`・`shiori4-api`: どちらも「SHIORI の `load` へ渡すフォルダのパスの形（区切りの混在・末尾の区切りなし）」を疑い先・議題の候補に持つ。今回は同じ形のパスで 5 本通っているので、別の件と見ている。再現の調べでパスの形が効いていたら、そちらへ渡す。
  - `test-roots-under-target`: テストの一時フォルダを `target\` の下へ寄せる件。今回の走行の根は初めから `target\` の下で、決まりどおり。
  - `install-live-target-hazards`: 使っている最中のフォルダへの上書きを実測する件。「掴まれているファイルへ書く」という形は近いが、あちらはインストールの名前替え、こちらは DLL の中の書き込み。
  - `write-failure-notice`: areka 自身が書けないときの伝え方。SHIORI の DLL が書けないときは含まない。
  - `emo2-real-run-wrap-timeout`: emo2 の実走の別の赤（待ちの時間切れ）。原因が違う。

### Constraints

- 段は**バグ**・区分 B。見たのは 1 回＝まず再現を確かめる。
- 実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ。emo2 の実走は絶対パスかつ短いパス。
- 止めてよいプロセスは、自分が起こしたと確かめられるものだけ。
- 実機の窓が出る検査なので、負荷を掛ける検査と同じ時間には回さない。くり返しの走行は、回す前に所要時間を伝える。
- 規模の見込み: XS〜S（確かめと記録だけ 1〜3・areka の側の直しが要ると分かれば 4〜7 タスク）。
- 要件定義のモデル: Opus。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）

- 前提の変化: 版の段で **v0.1.0** のバグ。10-11 から関係のファイルは変わっていない（`crates/areka/tests/emo2_real_run.rs` 247 行・自動終了 3,000 ミリ秒〔60 行目〕・見張りの締切 120 秒・仲介 `crates/shiori-host32-helper/src/shiori_proxy.rs` 814 行）。
- 検査の側だけで直せるか: 直せる見込み。今の検査は子の出力を、子が終わった後にまとめて読む（3 秒の自動終了に頼る）。候補＝子の標準出力を走りながら読み、`wrap=BudouxWordWrap` を見てから、自分が起こした子の窓へ閉じる知らせを送る（Alt+F4 と同じ道＝`crates/areka/src/app_exit.rs` の `on_ghost_os_close` → `OnClose` → 終了コード 0）。自動終了は保険として長めに残す。これなら `main.rs` に触らない。アプリの側に「印が出てから閉じる」を作るときだけ `main.rs`（956 行）に触る。
- 本物の窓を出すか: **出す**。本物の `areka.exe` を emo2 で起こし、本物の GPU と窓・32bit の仲介と pasta.dll を使う。`AREKA_EMO2_REAL_RUN=1` のときだけ走る（ふだんの `cargo test` では素通り）＝負荷の走行（`areka-test-binary-stability` など）と同じ時間に回さない。件 2 のくり返し（数十回）も同じ。回す前に所要時間を伝える。
- 規模: 4〜13（件 1＝3〜6・件 2＝確かめと記録 1〜3／直すなら 4〜7）。
- 先に要るもの: なし。
- 同じウェーブに置けない相手: 検査の側だけなら 24 本のどれとも 0。アプリの側を選んだら `main.rs` を触る `baseware-root-list`・`user-data-root`・`write-failure-notice`・`multi-ghost-runtime` と直列。件 2 で仲介を直すなら `shiori_proxy.rs` を疑い先に挙げる `emily-ghost-verification` と照合。
- 優先度の区分: B のまま。要件定義のモデル: Opus のまま。分割の案: なし。
- 見つけた穴・古くなった記述: 件 1 の棚卸㉓の節の「`main.rs` は `mcp-kanade-tools`・`mcp-stdio-bridge` なども触る」は誤り（`mcp-kanade-tools` の brief は `main.rs` に触らないと書く・`mcp-stdio-bridge` は着地済み）。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 版: **v0.1.0（棚卸㉗でゴールを組み直した後も残る）**。areka を本物の窓と SHIORI で起こす唯一の自動の通しが赤では、「SSP並みの自動試験」の土台が立たない。件 2（新しい写しの最初の LOAD の失敗）も、自動の試験をくり返すほど理由の分からない赤として混ざる。
- 前提の変化: 10-16 から関係のファイルは変わっていない（`crates/areka/tests/emo2_real_run.rs` 249 行・自動終了 3,000 ミリ秒〔60 行目〕・`crates/shiori-host32-helper/src/shiori_proxy.rs` 814 行・閉じる道 `crates/areka/src/app_exit.rs` 310 行の `on_ghost_os_close`）。`emo2_real_run.rs` は子の出力をパイプで受け、`AREKA_ROOT`・`AREKA_PROFILE_DIR` を外していない（ほかの実機の試験 2 本と組が違う）。
- **`real-run-mcp-kit`（棚卸㉗の新しい 1 本）との関わり**: 件 1 の直しの候補（子の出力を走りながら読み、印を見てから利用者と同じ道〔OS の閉じる知らせ → `OnClose`〕で閉じる・自動終了は保険）は、`real-run-mcp-kit` の窓口の「出力を探して待つ」「閉じる」と**同じ仕組み**。2 本で別々に作らない。推し: `real-run-mcp-kit` が窓口の中に作り、本 spec の件 1 はその窓口へ `emo2_real_run.rs` が移った後に待ち方だけを直す（同じウェーブに置かない・窓口が先）。逆に本 spec を先にするなら、件 1 の閉じ方を窓口へ渡せる形（試験のファイルの中に閉じ込めない）で作る。件 2 の数十回のくり返しも窓口の上で回すと、落ちた回の根と出力を残す手順を書き直さずに済む。
- 触るファイル: 件 1＝`crates/areka/tests/emo2_real_run.rs`（窓口が先なら移った後の形）。件 2＝再現しなければ 0、すれば `shiori_proxy.rs` か起こし直しの順。
- 規模: 4〜13 のまま（窓口が先なら件 1 は 2〜4 に縮む）。
- 先に要るもの: なし（窓口を待つかは上の推し・要件の段で決める）。
- 同じウェーブに置けない相手: `real-run-mcp-kit`（`emo2_real_run.rs`・閉じ方）。検査の側だけならほかは 0。
- 優先度の区分: B のまま。要件定義のモデル: Opus のまま。分割の案: なし。
- 見つけた穴・古くなった記述: `emo2_real_run.rs` の行数は 247→249。
