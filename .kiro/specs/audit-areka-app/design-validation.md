# 設計の検証: audit-areka-app

- 対象: `.kiro/specs/audit-areka-app/design.md`（2026-11-22・`a2032eb7a`）
- 入力: `requirements.md`（「要件の討議で確定したこと」は確定として扱った）・`research.md`・`spec.json`（language `ja`）・steering（`structure.md` の設計が名指す行だけ。`product.md`・`tech.md` は通して読んでいない）
- 進め方: `kiro-validate-design` の REVIEW（Analysis → Critical Issues → Strengths → GO/NO-GO）。開発者には問わずに判定した（非対話）

## 設計の検証の要約

確定した 97 件と預かった件 5.1〜5.21 が、指摘ごとの直し・預かった件の表・作業の組 G1〜G22 に漏れなく割り当てられ、要件の追跡表も 1〜9 の全項目を埋めている。設計が名指す型・関数・行（`PersistFailureCause` の 451 行、`surface_stage` の既にある失敗の返し方、`join_bounded`、`carrier_cue`、`PropGetCueSink::new`、`workspace_scan` の口、`build_placement_config` ほか）はコードに実在し、直し方はどれも今のコードで組める。残る問題は、兄弟から受けた件の扱いの記録と、脳を消す段の後片づけの持ち主と、要件 4.1 のテストの抜けの 3 つで、どれも設計の討議で決めて設計の文を直せば足りる。

## 重大な問題（3 件）

🔴 **重大な問題 1**: 兄弟から受けた件 S1〜S6 が、確定を待たずに設計へ組み込まれ、文の間に食い違いがある
**懸念**: S1〜S6 は要件の確定の一覧（97 件）に無いが、G5・G14 に組み込み済み（OPEN QUESTION 1 は「載せる前提」）。さらに「Revalidation Triggers」は `boot_context_named` を「兄弟と `menu/` のテストが呼ぶ」と書く一方、既定の直し（要件 5.9・確定の議題 13）は `menu/ghost_frame_tests.rs` を寄せない。S3 は兄弟が本 spec の `alert_tests.rs` を直す前提（OPEN QUESTION 4）、S4 は直さずに閉じる手もある（OPEN QUESTION 10）。
**影響**: 実装係が既定と違う向きで `menu/` を触るおそれがある。要件 9.5 の差分の確かめ（範囲の外の差分は要件 5 の件と呼び出し元の直しだけ）と、要件 2.1・5.22 の記録が、どの件を直す対象にしたかを示せなくなる。
**提案**: 設計の討議で OPEN QUESTION 1・2・4・10 を先に決め、結果を `requirements.md` の確定の節へ足す（S1〜S6 と、議題 13 を改めるならその改め）。「Revalidation Triggers」と `menu/ghost_frame_tests.rs` の行を、決めた一方に合わせて書き直す。
**追跡**: 要件 2.1・3.2・5.9・7.3・9.5
**根拠**: design.md「Revalidation Triggers」「Modified Files（範囲の外）」の `menu/ghost_frame_tests.rs` の行・「兄弟から受けた件」・「OPEN QUESTIONS」1・2・4・10

🔴 **重大な問題 2**: 脳を消す段の後片づけに持ち主がいない。消してよいかの確かめも狭い
**懸念**: spec 1 本は PR 1 本の squash マージなので、兄弟と「同時に着地」はできず、どちらかが先に入る。こちらが先なら、「設計の段で決めた事項」3 の 3 で `shiori_host.rs`・`shiori_host_case_fold_tests.rs` と dev の `shiori-abi`・`windows-core` が残る。設計はこれを「向こうの着地の後に消す件として申し送る」と書くが、受け取る spec を名指していない。兄弟が後から消すと、C26 の間は本 spec だけが持つ席の本体の `Cargo.toml`（約束 ⑵・要件 7.5）と本 spec のファイルに触ることになる。また、消してよいかの確かめは `git grep "shiori_host" -- crates/areka/src/mcp` だけで、`shiori_abi`・`windows_core` の使い手を見ていない。
**影響**: 要件 5.20 が求める「消す件として申し送る」が宙に浮き、使い手の無い試験の部品と dev の依存が、どの spec の持ち物でもないまま残る。
**提案**: 申し送り先を設計に書く。たとえば、兄弟の brief に件として書き、ウェーブの後に兄弟がテストを直すのと同じコミットで消す、と伝言で申し合わせる。ウェーブの後の spec へ回す手もある（要件 7.11 の実在の確かめつき）。そのうえで G21 に記録の行を足す。消してよいかの確かめは `git grep -n "shiori_host\|shiori_abi\|windows_core" -- crates/areka` にする。
**追跡**: 要件 5.20・7.2・7.5・7.11
**根拠**: design.md「設計の段で決めた事項」3・「並走の兄弟と申し送り」の `mcp/get_property_tests.rs` の行・「Allowed Dependencies」

🔴 **重大な問題 3**: 直したバグと危ない所のうち、壊れたら落ちるテストを持たない件がある（要件 4.1）
**懸念**: 指摘 31（例が片方だけの `balloon.offsetx`／`offsety` を捨てるバグ）の直しは「配置の crate と同じ式」を例に書き写すもので、テストの欄は「例はテストが走らない」になっている。指摘 27（exe のテストが開発者の根を受け継ぐ危ない所）のテストの欄は「既存の exe のテスト」だけで、既定の環境変数を外しても赤にならない。
**影響**: 要件 4.1「1 件ごとに 1 本以上」を満たさない。指摘 31 は、同じ式の写しを 1 つ増やす（要件 3.5・ponytail の「既にあるものを使う」に反する）。
**提案**: 指摘 31 は、例の `read_balloon_offset` の判定を `areka_placement::config::build_placement_config(..).scopes[&0].balloon_offset` を呼ぶ形に替えて写しを消す。既にあるテスト `balloon_offset_axes_are_independent`（`areka-placement/src/config.rs:696`）が落ちるテストになる。指摘 27 は、`offline_dump_exe_test/common/mod.rs` が組む `Command` の `get_envs()` に 5 つの変数があり、呼び手の値で上書きできることを見る小さいテストを 1 本足す。Testing Strategy の表にも、この 2 件の行を足す。
**追跡**: 要件 4.1・4.2・3.5
**根拠**: design.md「指摘ごとの直し」の 27・31 の行・「Testing Strategy」の「壊れたら落ちるテスト」の表

## 設計の良いところ

- **直し方がコードの事実に当たっている**: 下地の上限は `surface_stage` が既に使う失敗の返し方（`DumpTally { written: 0, failed: surfaces.len() }`）をそのまま使い、`PersistFailureCause::Unusable` は作る所を `read_existing` の 451 行の 1 か所に絞って、網羅の呼び手 2 つ（`alert.rs`・`write_failure.rs`）を名指している。脳のテストは、本番の受け口 `PropGetCueSink` の選別（`get`・`property`）どおりに組み直している。どれも新しい本番のモジュールと新しい依存を作らない。
- **消す・名を変えるときの照合と兄弟との境が、先に表になっている**: 消す的ごとに既知の当たり（`roadmap-history.md` のように残す経緯を含む）を挙げ、兄弟の持ち物の差分 0 行と、逆向きに頼む 2 行（OPEN QUESTION 4）まで書いてある。要件 3.4・7.3・9.5 を実装の段で確かめやすい。

## 最後の判定

- **判定**: GO
- **理由**: 層の組み方の食い違いと要件の欠けは無く、97 件と 5.1〜5.21 の直し方はどれもコードで組める。重大な問題 3 件は、どれも設計の討議で決めて設計の文と表を直せば閉じ、作り直しは要らない。
- **次の段**: 設計の討議（`kiro-design-discussion`）で上の 3 件と OPEN QUESTIONS 1〜10 を決め、design.md を直してから `/kiro-spec-tasks audit-areka-app` へ進む。

## 補足: 確かめたこと（抜き取り）

- 実在した: `PersistFailureCause`（`#[non_exhaustive]` ではない・`match` は `alert.rs:220-223` と `write_failure.rs:55-58` の 2 か所）と `Unusable` を作る 451 行。`Canvas { width, height }` と `vec![0u8; cw * ch * 4]`（`render.rs:98`）。`areka_runtime::spine::join_bounded(&str, ActorHandle)`。`consumer_ledger::test_support::carrier_cue`。`frame::test_support::{capture_logs, gpu_on}`。`frame::shared_test_support::wiring`。`source_text_test_support::code_only`。`areka_placement::test_support::{SharedFakeIo, fake_handle}`。`PropGetCueSink::new(reader, AskerId, Sender)`。`ghost_asker_id`。`workspace_scan` の `walk_workspace_sources`・`scan_sources`・`read_source`・`files_that_must_not_spell_the_tokens`。`real_run_kit` の `deadline`。`WintfTaskPool::with_threads`。`restore_and_judge`・`apply_balloon_limit`・`restored_scope_marks`・`prepare_ghost_windows`。
- 当たった: `areka.manifest` と `tests.manifest` の違いは 2 行目のコメントだけ。`areka-session` は `publish = false` なので、`build.rs` が兄弟の crate のファイルを読んでも packaging は壊れない。`areka_emo_present` を `areka-session` で使うのはテストと `cfg(any(test, feature = "test-support"))` の支えだけ。`async_io`・`tracing_subscriber`・`areka_user_data` は本体の本番から使われていない。`shiori_abi`・`windows_core` の使い手は、脳の 8 本と、範囲の中の 1 本と兄弟の 1 本のテストと、`embedded_icon_test.rs` だけ。本体に `#[implement]` は脳の外に無い。本体の `test-support` は、`areka-session` → `areka-runtime` → `areka-placement` の順に連鎖して開く。
- 残る危うさ（重大には数えない）: 確定の議題 7 の「幅か高さ」の上限の下では、16,384×16,384 の下地はまだ組める（面ごとに約 1GiB の確保と PNG の写し）。議題 7 は確定なので、ここでは記録だけにする。新しいテストのファイルに要る `#[path]` 付きの `mod` の行（`ghost_session_handle.rs`・`session_end_tests.rs`）は、タスクの段で書き出せば足りる。
