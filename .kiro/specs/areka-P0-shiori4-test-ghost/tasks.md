# Implementation Plan

- [ ] 1. shiori4-testdll: 決定論replay脳（自給cdylib）
- [x] 1.1 (P) crate雛形を組み、cargo test --workspace 時のビルド成果物の所在を実証する
  - x64 cdylib+rlib の最小crateを立て、スタブの生成入口を持たせる
  - 契約定数（出力DLLファイル名）を公開する
  - cargo test --workspace 実行後、成果物が単一の正準位置に現れることを確認するテストで実証し、その位置をコード内の定数・コメントとして確定記録する（フォールバックは設けない・不在時は明示panic文言を用意）
  - 観測可能な完了: 新設した実証テストがgreenで、正準位置が定数として固定されている
  - _Requirements: 1.2, 5.4_
  - _Boundary: shiori4-testdll crate scaffold（lib.rs スタブ）_

- [x] 1.2 (P) SHIORI要求からイベントIDを抽出し、収載/未知/不整合を判定する純粋ロジックを実装する
  - GET/NOTIFYの別とID行を読み取る
  - 収載ID→固定応答選択、未知ID→204相当、構造不整合→400相当（fail-visible・panicしない）の全分岐を単体テストで網羅する
  - 観測可能な完了: 全分岐（正常・未知・不整合）の単体テストがgreen
  - _Requirements: 2.2, 2.3, 2.4_
  - _Boundary: shiori4-testdll/request.rs_
  - _Depends: 1.1_

- [x] 1.3 (P) 正典イベントごとの凍結応答を保持する静的テーブルを実装する（暫定データで先行）
  - 応答データはファイルから埋め込み、実行時I/Oを持たない
  - 埋め込み時に行末表現を正規化し、gitの改行変換に依存しない
  - 暫定応答は正準SHIORI/3.0形式で明示的にPROVISIONALと分かる形にする
  - 観測可能な完了: 正規化の単体テスト（冪等性含む）がgreenで、ID→応答の参照が決定論的（同一入力に同一参照）
  - _Requirements: 2.2, 2.5, 1.5_
  - _Boundary: shiori4-testdll/snapshot.rs_
  - _Depends: 1.1_

- [x] 1.4 IShiori COM境界を横断する決定論replay脳と生成入口を実装する
  - 生成入口は本番SHIORI4が使うのと同じ規約に準拠させる（1.1のスタブ生成入口を実体へ差し替える）
  - Get呼び出しは1.2のID判定・1.3の凍結応答参照を用いて常に即時応答とし、pending（遅延）を返さない
  - Notifyは受信のみ・内容不問で決定論的に成功を返す
  - host参照欠落時は未書込のまま明示的な失敗を返す（半構築を露出しない）
  - 観測可能な完了: 生成→Get/Notify呼び出しの単体テストがgreen（即時応答・host欠落時の失敗を含む）
  - _Requirements: 2.1, 2.3, 2.4, 2.5, 7.4, 1.5_
  - _Depends: 1.1, 1.2, 1.3_

- [ ] 2. areka-ghost: 正規in-processロード経路
- [x] 2.1 (P) x64 DLLをロードし生成入口を解決するRAII機構を実装する
  - ロード失敗（DLL欠落・不正イメージ・シンボル未解決）はいずれもログ記録済みの失敗として返す
  - 取得済みリソースはいかなる失敗経路でも確実に解放される
  - 正常ロードの単体テストは1.1で確立したスタブDLLを対象に行う
  - 観測可能な完了: 正常ロード・各失敗態様（DLL欠落/不正イメージ/シンボル未解決）の単体テストがgreen
  - _Requirements: 3.1, 3.5_
  - _Boundary: areka-ghost/src/shiori_inproc.rs（InProcLibrary）_
  - _Depends: 1.1_

- [x] 2.2 生成に必要な最小限のホスト側受け口を実装する
  - プロパティの設定・取得を単純に往復できるようにする
  - 本経路が消費しない機能（自発通知の配送・遅延応答の完了）は最小限の応答のみ返し、実配線を持たない
  - 観測可能な完了: 4メソッドそれぞれの単体テストがgreen（プロパティ往復・欠落key・自発通知/遅延応答の最小応答含む）
  - _Requirements: 3.1, 7.4_
  - _Depends: 2.1_

- [x] 2.3 実DLL境界とテスト駆動契約を橋渡しするアダプタを実装する
  - 要求の組み立てから応答の解釈までを一貫させ、正常系・204相当・エラー相当・解釈不能・遅延応答受領のそれぞれを既存の結果語彙へ機械的に写像する
  - 生存中は常時「稼働中」を報告する（別プロセスの死活監視対象がないため）
  - 終了時は保持リソースを宣言順で確実に解放し、常に正常終了として報告する
  - 観測可能な完了: 写像表の全行（正常・204相当・エラー相当・解釈不能・遅延応答受領）と終了経路の単体テストがgreen
  - _Requirements: 3.2, 3.3, 3.4, 3.5, 7.1_
  - _Depends: 2.1, 2.2_

- [x] 2.4 ロード経路をブート結線の正規な第3の選択肢として組み込む
  - 呼び出し側は他の2方式と同列にこの方式を選べる
  - 本番メイン結線・機種自動判別ロジックには一切手を入れない
  - 観測可能な完了: 新方式を指定してブートし、生成〜駆動〜終了までが一気通貫で成功する統合テストがgreen
  - _Requirements: 1.1, 3.1, 3.6, 7.1, 7.2, 7.3_
  - _Depends: 2.3_

- [x] 3. (P) 駆動対象非依存の交信記録機構を実装する
  - 発行イベントの種別・ID・順序・結果を、実装の種類を問わず同一の手口で記録できるようにする
  - 記録は呼び出し順を保った追記のみとする
  - 観測可能な完了: 記録機構を任意の駆動対象に被せ、順序どおりの記録が取得できる単体テストがgreen
  - _Requirements: 1.4, 2.6_
  - _Boundary: areka-ghost/tests/ghost/recorder.rs_

- [ ] 4. テストゴーストfixtureの組立
- [x] 4.1 (P) ビルド済みテストDLLの所在を決定論的に特定する
  - 1.1で確定した正準位置のみを参照する（フォールバック段は設けない）
  - 不在時は次の一手を示す明示的な失敗とする
  - 観測可能な完了: ビルド後に実行した特定処理が実ファイルパスを返す単体テストがgreen
  - _Requirements: 1.2, 5.4_
  - _Boundary: areka-ghost/tests/ghost/inproc_fixture.rs（locate 部分）_
  - _Depends: 1.1_

- [x] 4.2 実在ゴーストの外観資産を流用した最小テストゴーストを組み立てる
  - 実在ゴーストのシェル資産をそのまま複製し、独自の最小シェルは作らない
  - 設定ファイルは新設のテストDLLを指すようにする
  - 特定済みのビルド済みDLL（4.1）を、1.4で完成した実体DLLとしてゴースト構成へ配置する
  - バルーン資産は同梱しない（起動〜終話の経路が消費しない判断を反映）
  - 観測可能な完了: 組み立てた一時ゴーストが実マウント解決を成功裏に通過する単体テストがgreen
  - _Requirements: 4.1, 4.2, 4.3_
  - _Depends: 1.4, 4.1_

- [ ] 5. 決定論e2e（常設ゲート）
- [x] 5.0 spine プローブループを壁時計 deadline で硬化する（承認済み §6.2 例外・2026-07-19 開発者承認）
  - spine_e2e_test.rs の反復回数境界プローブループ 8 箇所を、runtime.rs task2.4 先例（`Instant` deadline 10s ＋ 既存ループ本体そのまま）へ機械変換する。対象＝boot-probe（`for _ in 0..10_000u32`）: S1@709 / S3@1135 / S4@1420 / S5@1703、spin-settle（`for _ in 0..1_000_000u32`）: S1@781 / S4@1462 / S5@1750 / S5@1794（行番号は変換で前後する・関数名で同定せよ）
  - ループ本体・poll 条件・panic 文言・後続 assert・S1〜S6 台本・ScriptedShioriBackend・RecordingSink は一切無改変（§6.2 の意図＝シナリオ意味論の保護は維持・変更は「諦めるタイミングの測り方」＝待機形状のみ）。S2/S6 はプローブループ非保有ゆえ無改変。sim 時刻は従来どおり Tick 注入のみで進む（壁時計はスレッド起動待ちのハングガードにのみ使用）
  - 背景: 現行の回数境界ループは壁時計の底がなく、CPU 競合下で `yield_now()` が空回りして 1万反復が数 ms で尽き、製品コードは正常なのにテストが早合点で赤になる（偽陽性）。5.1 追加**前の** baseline 単独でも ~3%（1/30）の低頻度 flake が実測され、失敗は S1/S3/S4/S5 を巡回。同ファイルは既に `run_bounded`/`join_bounded`（`recv_timeout` 10s）を 6 箇所で常用しており、deadline 化は既存イディオムへの整合
  - 観測可能な完了: `cargo test -p areka-ghost --test ghost` を **30 回連続**実行して **0 失敗**（baseline ~3% flake が消滅した有意な証拠）
  - _Requirements: 6.2（承認済み狭い例外・プラン承認による授権）, 1.3_
  - _Boundary: areka-ghost/tests/ghost/spine_e2e_test.rs（待機ループ形状のみ）_
- [x] 5.1 起動から終話までの一周を注入時刻のみで駆動し照合する
  - 実時間待機を用いず、時刻前進の注入のみで一連の交信を進行させる
  - 起動時挨拶の演出列が凍結台本由来の期待列と全順序で一致することを照合する
  - 凍結応答が差し替えられた際に検出漏れが起きないよう、期待列とスナップショットの整合も併せて確認する
  - 正常終了の握手が観測できることを確認する
  - 常設ゲートの観測は演出配送の受領レベルに留め、実描画（サーフェス合成・画素読戻し）は要求しない
  - 観測可能な完了: 一周テストがgreenで、期待列とスナップショットの不一致を意図的に注入すると確実にfailする
  - 駆動ループは sleep 不使用（`yield_now()` ＋壁時計 deadline ハングガードのみ・task 2.4 先例と同形）。5.0 の spine 硬化が前提で成立するため、`thread::sleep` ポールバックオフは再導入しない
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 1.1, 1.3_
  - _Depends: 5.0, 2.4, 4.2_

- [x] 5.2 交信列と演出出力の双方を同一手口で照合する
  - 実DLLを駆動しながら3の交信記録機構を被せ、発行イベントの種別・ID・順序が期待どおりであることを確認する
  - 同時に演出出力側の記録も期待どおりであることを確認する
  - 観測可能な完了: 交信列assertと演出出力assertの双方を含むテストがgreen
  - _Requirements: 1.4_
  - _Depends: 2.4, 4.2, 3_

- [x] 5.3 ロード失敗の主要態様を決定論的に検証する
  - 参照先未指定・DLL欠落・不正イメージの3態がいずれも明示的な失敗として顕在化することを確認する
  - 観測可能な完了: 3態それぞれのテストがgreen（いずれもpanicせず失敗として報告される）
  - _Requirements: 3.5_
  - _Depends: 2.4_

- [x] 5.4 既存テスト資産が無改変で共存することを確認する
  - 既存の決定論fake駆動テスト・実機追験・窓/wire smokeのいずれも変更なく通ることを確認する
  - 実機追験・窓smokeは仕分けのうえ残置（乗り換えなし）とし、その判断が実際に反映されていることを確認する
  - 観測可能な完了: cargo test --workspace 全体がgreen（既存スイート・新設スイートとも）
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 1.6_
  - _Depends: 5.1, 5.2, 5.3_

- [ ] 6. スナップショットの実採取と確定
- [x] 6.1 (P) 実機からの代表応答を交信記録機構経由で採取するハーネスを実装する
  - 実機環境変数が両方そろった場合のみ動作し、いずれか欠落時は静かにスキップする
  - 実機経路を一周させ、記録からGET交信の初出応答のみを正準形式へ再構成してファイル出力する
  - 観測可能な完了: ハーネス自体の組み立て・出力形式生成ロジックが単体テストでgreen（実機なしでも構造は検証可能な範囲で）
  - _Requirements: 2.6, 6.1_
  - _Boundary: areka-ghost/tests/ghost/snapshot_capture_test.rs_
  - _Depends: 3_

- [x] 6.2 実機から代表応答を採取して凍結コミットし、期待列を更新する（DoD直前）
  - 実機環境（実SHIORI・実ブリッジ経由）で採取ハーネスを実行し、正典イベントの代表応答を得る
  - 得られた応答をレビューのうえ凍結データとしてコミットし、暫定（PROVISIONAL）応答を置き換える
  - 一周テスト（5.1）の期待列を、凍結後の実データに合わせて更新する
  - 観測可能な完了: 暫定データがすべて実採取データへ差し替わり、一周テストが実データ前提でgreen
  - **[実スコープ確定・2026-07-19 実採取成功]**:
    - (a) 採取ハーネス fixture をフルゴースト化（実 pasta は `dic/`・`scripts/`・`pasta.toml` を load_dir から読む・DLL 単体では応答不能＝60s timeout。DLL 親ディレクトリの再帰フルコピーで pasta が 0.96s 応答＝実証済み）→ コミットA
    - (b) 凍結セット＝**採取結果 {OnFirstBoot} のみ**・**OnBoot.txt 削除**（設計「同梱集合は採取結果＝kanade が GET する ID に一致させる」）。理由: 実 emo2 は挨拶を **OnFirstBoot** で返し、kanade 正典フォールスルー（`boot.rs:72-76` Value→StartTalk 直行）で **OnBoot GET は発行されない**。OnBoot 照会は未収載→決定論 204 で充足
    - (c) 影響テスト全更新（testdll snapshot.rs 6本・lib.rs 1本・areka-ghost adapter 1本・I1/I2 期待列）→ 中間赤ゆえアトミック 1 コミット（コミットB）
    - (d) `all_provisional_snapshots_carry_marker` 檻を **マーカ不在 assert へ反転**＝暫定データ全置換の機械的証明
    - (e) golden cue 列は sakura コンパイラ規則（`\![bind/move]`・`\1` は cue 非生成で無視・`\p[N]` のみ actor 切替・`\_w`=Wait cue・`\n[N]`=NewLine・Text=run 単位）から机上導出→観測突合→`(ActorKey, CueCommand)` 順序列でハードコード（float at/duration 厳密一致は除外）。ドリフト検出は `snapshot_for("OnFirstBoot")` へ
  - _Requirements: 2.2, 2.6_
  - _Depends: 6.1, 5.1_

> **要件7.5について**: SAORI・里々・YAYAは意図的にいかなるタスクにもマップしない。「やらないことの証明」という性質上、実装タスクを持たず、境界宣言（design.md の Out of Boundary 節）で充足される。

## Implementation Notes

- **[1.1 spike 実測] cdylib の正準位置は `target/<profile>/deps/shiori4_testdll.dll`**（設計 D-1 の記述「deps を pop して `target/<profile>/`」は誤り）。`cargo test`／`cargo test --workspace` は cdylib を top-level へ uplift せず deps のみ（`cargo build` のみが top-level へ uplift）。実測: test-only ビルド後 top-level 不在・deps 存在。よって **task 3／4.1 の `locate_built_test_dll()` は `current_exe().parent().join(DLL_FILE_NAME)`（deps ディレクトリ・deps-pop しない）で解決すること**。areka-ghost の e2e test binary も同じ deps/ に居るため、この deps-dir 解決が単一正準・フォールバックなしで機能する。不在時は `cargo test --workspace` を促す明示 panic。
- **[1.3] snapshot 表 API は `pub(crate)` で先行**（設計 Service Interface は `pub`）。task 1.4 で `snapshot_for`/`snapshots`/`PROVISIONAL_MARKER` を **`pub` へ広げる**（areka-ghost の rlib 面が task 4.1/5.1 で `DLL_FILE_NAME`＋スナップショット表を参照するため）。`DLL_FILE_NAME` は task 1.1 時点で既に `pub`。
- **[1.3 提供データ] 暫定 OnBoot Value スクリプト = `\0\s[0]おはようございますわ（暫定）\e`**（`X-Areka-Snapshot: PROVISIONAL` header 付き・parse_response が未知 header を無視するので wire 上 inert）。OnFirstBoot = 204。task 5.1 の期待 cue 列は `snapshot_for("OnBoot")` から導出＋ドリフト検出 assert。task 6.2 が実採取で PROVISIONAL を置換する際は 5.1 の期待定数も更新すること。
- **[2.1] clippy `-D warnings` 全体ゲートは使用不可**（既存 toolchain ドリフト＝rust-1.97.0 の `collapsible_if` 等が areka-kanade/runtime/ticker/sink/dola に既在・本 spec 境界外）。真のゲートは `cargo test`（DoD は `cargo test --workspace`・memory areka-no-ci-gpu-tests-in-cargo-test）。各タスクの clippy 検証は「変更ファイルに警告が帰属しないこと」で足る。areka-ghost の `windows` は workspace 継承で `Win32_System_LibraryLoader` を得ており feature 追加不要。areka-ghost に `shiori-abi`（prod）＋`shiori4-testdll`（dev-dep）を追加済み。
- **[2.3→5.1 申し送り] snapshot API が外部到達不可**: task 1.4 で `snapshot_for`/`snapshots`/`PROVISIONAL_MARKER` を `pub` にしたが `shiori4-testdll/src/lib.rs` の宣言が `mod snapshot;`（**非公開モジュール**）ゆえ `shiori4_testdll::snapshot::snapshot_for` は crate 外から到達不可。task 5.1 の**ドリフト検出 assert**（e2e が `snapshot_for("OnBoot")` の凍結 Value と期待定数の一致を確認）には外部到達が必須ゆえ、5.1 で `pub mod snapshot;` へ変更するか crate root へ `pub use snapshot::{snapshot_for, snapshots};` を追加すること（boundary に `shiori4-testdll/src/lib.rs` を含める）。2.3 は暫定リテラル `\0\s[0]おはようございますわ（暫定）\e` 比較で回避済み。
- **[4.2 重大・5.1/5.2必読] ghostテストバイナリの飢餓回避**: 新規fixtureテストが**並列**で重いFS I/O（emo2 shell PNG木の再帰コピー・実行DLLコピー・remove_dir_all）を行うと、**Windows Defenderの新規ファイル再スキャン**がコアを占有し、`spine_e2e_test.rs`の**壁時計なし協調ループ**（`for _ in 0..10_000 { Tick; yield_now() }`）を飢餓させ、ghostスイートが15/15緑→~10/15にflaky化する（実測A/B）。spineは§6.2ゆえ改変不可。**根治策=inproc_fixture.rsの`pub fn shared_test_ghost() -> &'static Path`**（OnceLock・assemble一度・**hardlink優先**materialize＝新規バイトなしでDefender再スキャン回避・直列化mutex・鮮度ガード付きクロスラン再利用・意図的leak）。**task 5.1/5.2 は自前assembleせず`shared_test_ghost()`を再利用し、駆動ループは反復回数境界でなく壁時計deadline境界（task 2.4方式）にすること**。disposableが要る場合のみ`assemble_test_ghost(tag)->TempGhost`（Drop=remove_dir_all・hardlinkゆえ原本無傷）。新e2e追加後は`cargo test -p areka-ghost --test ghost`を15回以上回し0失敗を確認せよ。
- **[5.0 承認済み §6.2 例外・2026-07-19]** spine_e2e_test.rs のプローブループ 8 箇所を回数境界→壁時計 deadline へ硬化することを開発者が承認（プラン `dynamic-snuggling-wall.md` 承認による授権）。**§6.2「spine 不侵」の意図＝シナリオ意味論・S1〜S6 台本・ScriptedShioriBackend 拡張方式の保護**は完全に維持し、変更は「待機ループが諦めるタイミングの測り方」（回数→時計）のみに限定する。根拠: (1) 現行ループは壁時計の底なしで baseline 単独 ~3% flake（実測）＝**既存の潜在欠陥**であり本 spec の新テストが持ち込んだものではない、(2) 同ファイルが既に `run_bounded`/`join_bounded`（recv_timeout 10s）を 6 箇所常用＝deadline 化は既存イディオムへの整合、(3) 開発者恒久方針「決定論テスト必達・小細工禁止・壊れたが意味あるテストは更新」に合致（[[deterministic-test-coverage-mandate]] / [[canonical-not-minimal-lifecycle]] / [[obsolete-vs-broken-test-policy]]）。5.1 の 2ms sleep ポールバックオフ（spine 飢餓回避の応急措置）は 5.0 完了により不要となり撤去（yield_now＋deadline のみ）。
- **[5.0/5.1 実測結果と残存]** 5.0 硬化で `ghost` スイートの偽陽性は baseline ~15%（回数境界の高速空振り）→ **boot-probe ループは完治**。5.0 単独 30 回連続 0 失敗を実測。**残る ~3% 低頻度残存**は spine の純 `yield_now` **settle ループ**（Tick 注入も channel op もしない S4/S5×2）が、**この並列マルチエージェント build セッション特有の兄弟 rustc/cargo burst で全コア飽和した瞬間**にのみ飢餓するもの（14 claude プロセス / 22 論理CPU）。5.1 自体のテスト（`i1_inproc_...`）は全実測で無敗＝残存は spine 側。この burst 競合は**開発者の合流/DoD ゲート（並走 worktree が片付いた専用セッション・[[portfolio-convergence-decided-in-separate-session]]）では消える**性質ゆえ現状で確定。もし将来「あらゆる負荷下で決定論的 0」を要する場合の追跡手当＝settle ループの `yield_now()` を短い poll-backoff（コア解放）へ変える案があるが、これは「no sleep」の明示的例外承認を要し本 spec の範囲外。5.4 の `cargo test --workspace` ゲートは可能なら競合の低い時点で実行、または残存を再確認すること。
- **[5.4 統合ギャップ・cross-crate]** `cargo test --workspace` でのみ顕在化した統合破断: task 2.4 が pub enum `ShioriWiring` へ `InProc` variant を追加したことで、`areka` bin の `#[cfg(test)] ghost_wiring_tests` の**網羅 match**（`options.shiori` を Helper/Custom のみで match）が `non-exhaustive patterns` でコンパイル不能に。`cargo test -p areka-ghost` は areka bin のテストをビルドしないため各タスクレビューで漏れ、5.4 の workspace ゲートが捕捉。修正＝`crates/areka/src/main.rs` の当該 match に `ShioriWiring::InProc => panic!(...)` arm を追加（網羅性回復のみ）。**本番結線 main.rs:178 の `ShioriWiring::Helper` は無改変**ゆえ要件 7.2 は保持。教訓: pub enum への variant 追加は下流 crate（bin 含む）の網羅 match を破りうる＝variant 追加タスクの検証は `cargo test --workspace` まで通すか、当該 enum を `#[non_exhaustive]` 化するか要検討。**workspace ゲート実測: EXIT 0 全 green（実装者＋独立レビュアーの2回とも初回クリーン・spine 残存は非発火）**。i686 host-32 成果物（shiori-host32-testdll/-helper）を `--target i686-pc-windows-msvc` で事前ビルド済・vendors/pasta submodule populated が前提。
- **[6.2 実採取レシピ＋構造発見・2026-07-19]** 実 pasta 採取が成功。**(1) 採取手順**: i686 helper ビルド済＋`HOST32_PASTA_DLL`=実 emo2 `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/pasta.dll`＋`AREKA_SNAPSHOT_OUT`=出力dir を設定し `cargo test -p areka-ghost --test ghost capture_real_pasta` を実行。**(2) fixture フルゴースト必須**: `write_real_pasta_ghost_fixture` は当初 pasta.dll 単体コピーだったため pasta が辞書を持てず OnBoot 一周が 60s timeout（既存 `real_pasta_test` も同欠陥で fail）。DLL 親ディレクトリ（＝実 ghost/master・dic/scripts/pasta.toml 含む）を `copy_dir_contents_recursive` で丸ごと持ち込む修正で pasta が 0.96s 応答。**(3) 構造発見**: 実 emo2 は挨拶を **OnFirstBoot の Value** で返す（むらさき＆エモ二人掛け合い自己紹介・1818B・`\p[0]/\p[1]`・`\![bind]`多数・`\_w`待ち・`\e`終端）。kanade フォールスルー（`schedule/boot.rs:66-77`）で OnFirstBoot が Value を返すと **OnBoot GET はスキップ**されるため、正典 GET は OnFirstBoot のみ＝凍結セットは {OnFirstBoot}。**(4) sakura コンパイラ規則**（`areka-sakura/src/compile.rs`）: `\![bind/move,...]`・裸`\1` は **cue を生成せず無視**（Custom にもならない）／actor 切替は `\p[N]` のみ（`\1` は無視）／`\s[key]`→`Emote{key}`（文字列キーそのまま）／`\_w[ms]`→第一級 `Wait` cue／`\n[N]`→`NewLine{ratio}`／Text は run 単位 1 cue（文字数×50ms duration）／非空台本は先頭 `ClearAll` 前置。大股 Tick で全 due cue を at 昇順一括排出（`dola/src/cue/schedule.rs`）ゆえ駆動は +500ms 級で高速消化可。**(5) follow-up（spec 範囲外）**: `real_pasta_test.rs` の bare-DLL fixture 欠陥（実 pasta では応答不能）は §6.3「無改変残置」検証済ゆえ本 spec で触らず、別タスクへ切り出し提案。
