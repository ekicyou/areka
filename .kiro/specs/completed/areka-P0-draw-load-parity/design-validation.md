# Design Validation Report: areka-P0-draw-load-parity

> 実施 2026-08-22（`/kiro-validate-design`・非対話）。対象＝`design.md`（908 行・C1〜C21）／`requirements.md`（要件 1〜8・受入基準 69 件）／`research.md` §6〜§7／steering（product・tech・structure・logging・workflow・roadmap の該当行）。
> 本レポートは設計を**変更しない**。指摘は設計ディスカッションの議題として渡す。

## Review Summary

設計は 69 件の受入基準すべてを追跡表で C1〜C21 へ写像し、ループ駆動層・計測の道具・実行体の観測・フレーム駆動の門という 4 つの層に境界を切って、依存方向（areka → wintf）・既定 OFF の前置ガード・既存の判定式と行の語彙の不変性を守っている。file:line の主張は 30 箇所以上を現行ツリーで突き合わせ、**別ファイルを指す誤り・実在しない行への参照は 1 件も無かった**（ずれは 5 件・いずれも ±2 行以内か関数先頭の取り違え 1 件）。一方で、無人で回すという本 spec の中核に対して、⑴ 終端判定の字面が文書側に露出していること、⑵ 計測 4 段のうち 2 段（スレッド・関数）が現行の環境・ビルド前提では成立しないことが確認でき、この 3 点を是正すれば実装へ進める。

## Critical Issues

### 🔴 Critical Issue 1: 終端の字面が文書に露出しており、ループが偽の「達成」で止まり得る

**Concern**: `/goal` の判定役は**会話に現れた文字列だけ**を見る（research §7.1）。設計は達成／不可能の判定を `PERF-LOOP FINAL: GOAL_MET` ／ `PERF-LOOP FINAL: STOPPED reason=` の**行頭一致**に置いている（design C1 条件文テンプレート・C11 STATUS 行の文法）。ところが同じ字面は、毎ターン会話へ読み込まれるプロジェクトスキル `perf-loop-iteration/SKILL.md`（C2）、`draw-load-parity.goal.md`、`perf-ledger.py` のソース、そして本 design.md 自身にも書式見本として現れる。判定役はテンプレートと実出力を区別する手段を持たない。

**Impact**: 周 0 の 1 ターン目でスキル本文が文脈へ入った時点で「達成」と判定され、目標が解除されてループが**成果ゼロのまま静かに終わる**可能性がある。これは「未達が spec の内側から見えない」形の再演であり、しかも今回は**成功と誤認される**ぶん検出が遅れる。逆側（`STOPPED reason=` の露出で「不可能」判定）も同じ経路で起きる。

**Suggestion**: ⒜ 実出力にだけ現れる**走行固有トークン**を終端行へ入れる（例 `PERF-LOOP FINAL: GOAL_MET run=<周 0 で生成した 8 桁の乱数>`）。条件文はそのトークン込みの字面を要求する。トークンは `perf-ledger.py goal-check` が生成して台帳の `状態` ブロックへ書き、条件文はループ起動時に生成物として貼る。⒝ スキル・README・design のすべての書式見本は、実出力と**一致しない書き方**（`PERF-LOOP FINAL: <GOAL_MET|STOPPED …>` のような山括弧プレースホルダ）に統一する。⒞ 「見本と実出力が一致しないこと」を `perf-ledger.py --selftest` の 1 ケースとして固定する（`fixtures-loop/ledger/`）。

**Traceability**: 要件 1.4／1.6／1.9／5.7
**Evidence**: design.md「C1 `/goal` 条件文テンプレート（要旨）」・「C11 STATUS 行の文法」・「Flow 1」／research.md §7.1「判定役は会話に現れたものしか見ない」・§7.2 DD-2

### 🔴 Critical Issue 2: 段②（スレッド別 CPU）の採り方が `Cargo.toml` 非接触と両立しない

**Concern**: C14 は `CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD)` でプロセス内スレッドを列挙する設計だが、この API は `windows` crate の **`Win32_System_Diagnostics_ToolHelp` feature** に属する。現行ツリーの `Cargo.toml:63-93` の feature 一覧に同 feature は**無く**（`Win32_System_Threading` はある＝`GetThreadTimes`／`GetThreadDescription`／`OpenThread` は使える）、全 crate が `windows = { workspace = true }`（`crates/wintf/Cargo.toml:32`・`crates/areka/Cargo.toml:35`）でワークスペース定義を共有しているため、**「既に feature を持つ crate（wintf `api.rs`）へ置く」という設計の逃げ道は現に存在しない**（どの crate も同じ feature 集合しか持たない）。ツリー全体に `Toolhelp32` の使用例は 0 件。

**Impact**: 要件 2.3（スレッド別 CPU＋役割名）を設計どおり実装しようとすると、要件 8.6 が禁じる `Cargo.toml` の feature 追加が**初手で必要**になる。ここで止まるか、禁止を破って改訂欄送りにするかの二択が実装 1 日目に来る。順位表の段②が欠けると、「ticker 3 系統・カーソル監視・タスクプールのどれが重いか」という本 spec の最も価値のある切り分け（更新前の計測に内訳が無かった当の部分）が最初から出せない。

**Suggestion**: 列挙を Win32 のスレッド列挙に頼らず、**自前のスレッド登記簿**へ替える。areka／wintf／areka-actor のスレッド生成点は既に `thread::Builder::name` で名付けている（`tick_bridge.rs:65-66`・`clickthrough/monitor.rs:87-88`・`ticker.rs:179,289`・`areka-actor/src/spawn.rs`）ので、生成時に「役割名＋`GetCurrentThread` の複製ハンドル（または TID）」をプロセス共有の登記簿へ登録し、報告器は登記簿を舐めて `GetThreadTimes` を呼ぶ。これなら `Win32_System_Threading` だけで足り、`Cargo.toml` に触れず、役割名の写像（C14 の純関数）も推測でなく**登録時の宣言**になって決定論テストが素直に書ける。bevy のタスクプール（`TaskPool (N)`）だけは自前登記に載らないので、「登記されていない残余」を `perf(process)` との差分として 1 行で出す。設計時に `cargo tree -e features` で足りるかを確かめる、という Technology Stack 表下の補足は**確認済みで足りない**ことが判明したので、補足を結論へ書き換える。

**Traceability**: 要件 2.3／2.6／8.6
**Evidence**: design.md「Technology Stack」表の補足・「C14 `perf_thread_report` — 採り方」／research.md §7.4 段② DD-4／現行 `Cargo.toml:63-93`

### 🔴 Critical Issue 3: 段③（関数別）が管理者権限に依存し、失敗が即「ループ停止」へ落ちる

**Concern**: C8 は `xperf -on PROC_THREAD+LOADER+PROFILE -stackwalk Profile` を第一候補とし、「管理者権限が要る。無ければ exit 4」と書く。exit 4 は `MEASURE_FAILED`＝`TOOLFIX` へ 1 回だけ入り、直らなければ `STOPPED reason=measure_failed`（design C5 終了コード・C2 遷移表）。しかし権限不足は道具の不具合ではないので `TOOLFIX` では直らず、代替の `wpaexporter` も同じ ETW カーネルセッションを要するため同じ壁に当たる。さらに `invoke-cpu-sample.ps1 -SelfTest` は**同梱の dump 断片を解析するだけ**で実採取を試さないので、`perf-loop.ps1 selftest` は緑のまま実採取だけが毎回落ちる。

**Impact**: 昇格していないセッションで起動すると、ループは周 0 の RANK 相で `STOPPED reason=measure_failed` に入り、**1 度も是正を試さずに終わる**。要件 1.5（開発者の関与は開始と受領の 2 点）を満たしたまま、実質「開始のしかたが違うと何も起きない」仕組みになる。段③が使えないこと自体は致命ではない（段①②④でも順位付けはできる）のに、設計はその降格運転を持っていない。

**Suggestion**: ⒜ 周 0 の先頭に**能力確認の相**（`PREFLIGHT`）を置き、`perf-loop.ps1 preflight` が「昇格の有無・`xperf.exe` の実在・PDB 生成の可否・`judge_version` 一致・Python／PowerShell 版」を 1 回で判定して台帳と STATUS 行へ書く。⒝ 段③が使えないときは `MEASURE_FAILED` にせず、`rank.txt` の `[3] 関数` を `UNAVAILABLE reason=not_elevated` として**段①②④で続行**する（順位表の欠落は台帳へ明記＝要件 2.11 の「黙って続けない」は満たす）。⒞ 昇格が必要な旨と昇格した起動手順を README §13／§14 と goal 条件文の前提欄に書き、⒟ `-SelfTest` に「実採取を 5 秒だけ試して停止する」既知ケースを足して、道具の較正が実採取の可否を含むようにする。

**Traceability**: 要件 2.4／2.11／1.4／1.5
**Evidence**: design.md「C8 `invoke-cpu-sample.ps1` Batch / Job Contract」・「C5 終了コード」・「C2 相の遷移表（TOOLFIX 行）」・「Error Categories and Responses」

## Design Strengths

1. **`/goal` の性質から設計を逆算している**——「判定役は会話しか見ない」「背景作業があるターンは判定を飛ばす」「終了は新しいターンとして届く」という 3 つの制約から、**1 ターン＝1 相の状態機械＋台帳が正本＋毎ターン STATUS 行**という形を導いており（research §7.1〜7.2・DD-1／DD-2）、要約・再開・モデル差に耐える。相の遷移表を `perf-ledger.py next-phase` の純関数として fixture 固定する（C2 Validation）まで降りているので、「自走」が願望でなく検査対象になっている。

2. **既存資産を壊さずに足す規律が具体的**——判定式⑴〜⑷b と較正値は不変、`RUST_LOG_VALUE` は不変で点灯は `-RustLogExtra` の連結のみ、新設行は新しい文言と新しいフィールド名で 1 行内重複なし、`J_REQUIRED_LOG_KINDS` は不変で `[tick]`／`perf(thread)` は任意種（C7・C12・Data Contracts）。合否の走行は素の走行・順位付けの走行だけ点灯する分離（Flow 2）も、観測費用を合否に混ぜない先行 spec の作法を正しく継いでいる。**file:line の裏取りも実効している**（30 箇所超を突き合わせて誤参照ゼロ）。

## 検証記録（file:line の突合・30 箇所超）

**一致**: `tick_bridge.rs:65-68/114-134/187-210/218-236`・`world/mod.rs:488,490,493,517-524,548-560,563,657,707` と単スレッド固定 6 本 `:117,135,141,146,151,156`・`Schedule::new(Update)` の字面 `:109`・`Cargo.toml:48-56`（`multi_threaded`）・`visual_sync.rs:25`・`command.rs:49,76-79,86-88,129-155,657-679,723`・`clickthrough/monitor.rs:34,87-88`・`ticker.rs:57-65,179,262,289` と catch-up 3 系統の発行点・`transition_diag.rs:54,622,633-635`・`emo2_boot/frame.rs:158-233`・`scale_text.rs:255`・`balloon_visibility_phase.rs:64`・`controller.rs:212,416`・`runtime/mod.rs:230-236` と vblank 中継・`judge-perf.py:106,364,380,396,451-452,466,588`・`invoke-perf-run.ps1:101,105`・`main.rs:126-128`・`window_proc/window_pos.rs:290`・`frame_test_support.rs:710-716`・`frame_harness_tests.rs:397`・実行器の字面検査 2 本・`kiro-impl` の Agent 派遣 3 箇所（`:93,111,142`）・`kiro-validate-impl:72-84`・`.claude/agents/` 未作成・fixture 17 件（＋`generate.py`）。

**軽微なずれ（実装時に直せば足りる）**: ⒜ `areka-emo-text/src/actor.rs:744-805` を `present_actor` の範囲として書いているが、同関数の先頭は **639** 行（744-805 は関数内の「行レイアウト〜`render`」の区間）。C18 の対象は正しいが、関数の所在としては誤読を招く。⒝ `main.rs:794`→実際 793。⒞ `pointer/systems.rs:17-31`→実際 17-33。⒟ `command.rs:99-114`→`SetWindowPosGuard::new` は 97 起点。⒠ 実行器の字面検査 `:362-369`／`:774-781`→実際の `assert!` は `:367-371`／`:778-782`。

## 軽微な所見（Critical ではないが議題に値する）

1. **C16 を「無条件に実装」する方針が、順位表駆動の選択規則と噛み合っていない。** design C16 は「旗・純関数・門の分岐・決定論テストは候補の採否に関わらず入れる／既定値だけを A/B で決める」と書くが、要件 1.2・3.1 は「順位表の上位から 1 つ選ぶ」を 1 周の骨格に据えている。門の結線は wintf・areka 合わせて 12 ファイル規模に及ぶため、ベースラインの順位表が tick を最上位に出さなかった場合、ループは自分の規則の外で最大の変更を払い終えていることになる。「周 0 のベースライン順位表で相の全走費用が上位 N に入ったときに限り C16 を実装する」と条件付けるか、逆に「C16 は仕組みの一部（計測の A/B 弁）であって候補ではない」と位置づけを言い切るか、どちらかへ寄せたい。

2. **メッセージ種別→旗の写像表そのものに決定論テストが無い。** 要件 6.1 は入力の全組合せ（表示 1 コマ適用・入力・アニメ境界・窓ジオメトリ・**DPI**・Z 順・ドラッグ中）に対する結果の固定を求めるが、設計の全組合せテストは `should_run`（ビット列を入力とする純関数）に閉じており、DPI は `WM_GEOMETRY` へ畳まれて独立の入力ではない。写像表側は `include_str!` による「`tick_wake::mark(` が在ること」の字面検査しか掛かっておらず、写像の**中身が誤っていても全テストが緑**になる（`WM_DPICHANGED` の登録漏れは要件 4.4 の実走チェックまで露見しない）。写像表を `fn wake_bits_for_message(msg: u32) -> WakeBits` の純関数へ切り出し、既知メッセージの表を決定論テストで固定することを勧める（未知→`FORCE` の既定も同じ表で固定できる）。

3. **`CLAUDE_CODE_GOAL_CHECKIN_MINUTES=60` は文書上の約束にとどまる。** 目標定義 TOML の `[goal_runtime] checkin_minutes` は道具が読むだけで、実際の値はセッション起動側の環境変数で決まる。加えて `measure-baseline` は 25＋25＋7 分＝約 60 分を 1 コマンドで走らせる設計なので、要件 1.11「計測の所要が check-in 間隔を超えない」を字義どおりには満たさない（設計自身がリスク欄で認めている）。`perf-loop.ps1 preflight`（Critical 3 の提案）で実効値を読んで台帳へ記録し、超えるなら baseline を release／dev／順位付けの 3 コマンドへ割る、が素直。

4. **catch-up の系統識別**: C12 は「loop は文言で識別」とするが、実コードは `tracing::info!(target = "loop_ticker", …)` で**フィールドとしての `target=` を 3 系統とも名乗っている**（`ticker.rs:203-206,223-226,305-308`）。3 系統とも `parse_fields` の `target=` 1 本で引ける旨に直しておくと、判定側の分岐が 1 つ減る。

## Final Assessment

**Decision: GO**

**Rationale**: 要件 69 件の追跡・境界の宣言・既存資産の不変条件・依存方向・テスト戦略はいずれも実装に足る具体度で書かれており、file:line の裏取りも実効している。上の 3 件はいずれも**設計の骨格ではなく前提の詰め**（終端字面の一意化・段②の採取方式の差し替え・段③の降格運転）であり、design.md の構造を保ったまま該当節の差し替えで解ける。ただし Critical 1〜3 は**タスク生成の前に**設計へ反映すること——1 と 3 はループの起動直後に効き、2 は実装 1 日目に要件 8.6 との衝突として現れるため、タスク分解の順序（周 0 の道具作り）に直接影響する。

**Next Steps**:
1. `/kiro-design-discussion areka-P0-draw-load-parity` で Critical 1〜3 と軽微所見 1〜4 を議題化し、design.md の C1／C8／C11／C14／C16 と Technology Stack 表の補足を改訂する。
2. 改訂後 `/kiro-spec-tasks areka-P0-draw-load-parity` へ。タスク 1 は要件 1.13 のとおり `kiro-impl` 改修（C4）、タスク 2 に `PREFLIGHT`／能力確認を含む道具の骨格（C5・C6・C11）を置き、ループを回し始める前に「道具が本当に動く」ことを既知ケースで確かめる順にする。
3. `actor.rs:744-805` ほか軽微なずれ 5 件は改訂の同じ機会に直す。
