# 設計の検証: audit-emo2-boot-frame

- 検証日: 2026-11-20
- 対象: `design.md`（656 行）・`requirements.md`（末尾の「要件の討議で確定したこと」を含む）・`research.md`（2 節の 114 件・11 節の設計の段の調べ）・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C23 の行）
- 作法: `kiro-validate-design`（分析 → 大事な問題 → 良い所 → GO／NO-GO）。対話なしで判定した。
- 突き合わせたコード: `main` `9bf6e32e` の作業ツリー（`crates/areka-runtime/src/frame/*.rs`・`skin_facts_copy.rs`・`target_map.rs`・`crash_report_test_support.rs`・`frame_balloon_runtime_test_support.rs`、本体の `ghost_session.rs`・`input_events/balloon_{pressed,wheel}_tests.rs`、`crates/log-capture-kit/tests/file_length_guard_test.rs`）

## 設計の検証の要約

114 件はどれも「指摘ごとの直し方」の表に 1 行ずつある（機械で数えて 1〜114 に抜けは 0）。要件 5 の件 1〜14 も「要件の対応」で指摘に結び付いている。C23 の触らない一覧（`crates/areka-placement/`・`Cargo.toml`／`Cargo.lock`・`hit_region.rs`・`input_events/`・`main.rs`・`ghost_session*.rs`・`app_exit.rs`・`menu/`・`event.rs`・§8）は守られていて、T27 に差分で確かめる手順もある。ただし、コードと突き合わせると、新しい形を引く直しのうち 3 つ（4・49・31 番）に、直しが欠ける所か、本番の道や同じウェーブの兄弟のテストを壊す所が見つかった。どれも設計の討議で直せる大きさで、設計の骨組みは変えなくてよい。

## 確かめた主な点

| 点 | 結果 |
|---|---|
| 1〜114 が直し方の行を持つか | 持つ（114 行・抜け 0）。直さずに残す指摘は 0 件で、確定 7 と合う |
| 要件 5（件 1〜14）の対応 | 5.1〜5.17 が指摘の番号と束（T1〜T25）に結ばれている |
| C23 の約束（要件 7.3） | 触らない一覧が要件 7.3 と一致。`close_windows_for_restart` は呼ぶだけ。T27 の `git diff --stat` の手順が揃っている |
| 上限越えの表（要件 7.4） | 今の表は 3 件で、`frame_transition_branch_tests.rs` の 1 行を消すと 2 件。設計の「3→2」と合う |
| 1,000 行の番人（要件 6.1） | 900 行台の 3 本は、足す前に縮める順になっている。分けた後の 2 本の見立ても上限の下 |
| 記録の無い失敗（要件 6.2） | 4 番は `warn!` を残して読み飛ばす。20・32 番は捨てていた失敗を捨てない形になる |
| 1 フレームの遅れ（要件 6.4） | 3 番の繋ぎ直しは同じフレームで起きる（`frame.rs` の並びで、文字の拡大率の相は L348 にあり、装着〔L251〕・取り出し〔L274〕・切り替え〔L278〕より後）。設計の主張どおり |
| ゴーストに見える変化の理由 | 3 番（後で文字が出る）は OPEN QUESTIONS 1 で議題にしてある。ほかの 4 つは記録の水準と文と手間だけ |

## 大事な問題（3 件）

🔴 **大事な問題 1**: 4 番の「8 か所」に、同じ形の 9 か所目（`attach.rs` L458）が抜けている
**問題**: `run_attach_phase` の終わりの `window_scopes.iter().filter_map(|&s| u32::try_from(s).ok())` は、`skin_facts_copy::copy` へ渡ったあと `balloon_target(scope)`（`2*scope+1`）を呼ぶ（`skin_facts_copy.rs` L15-L24）。上限の検査が無いので、scope が `i32::MAX` を越えるとデバッグのビルドではあふれて panic し、リリースのビルドでは桁が切れて別の target を引く。設計が直す `switch.rs` L358（同じ `copy` 系の `swapped` へ渡す所）と同じ形なのに、表に無い。
**影響**: 4 番の直しが半端になる（要件 3.1）。かけ直しの audit（T26）で同じ指摘がまた出る。「窓の側の入口を 1 つの関数で閉じる」（安全の考え）が成り立たない。
**直し方**: 4 番の表に `frame/attach.rs` L458 の `.filter_map(|&s| window_scope(s))` を足し、「8 か所」を「9 か所」にする（`research.md` の 11.1 の件数も合わせる）。T4 の触るファイルは変わらない（`attach.rs` は元から入っている）。実装のときは `u32::try_from`・`as u32` を範囲の中で grep し、0 件を完了の条件にする。
**要件**: 3.1・5.10・6.2
**根拠**: design.md「4 番 窓の側の scope の上限」の場所の表・「失敗の扱い」の 4 番・「安全の考え」

🔴 **大事な問題 2**: 49 番の見張りが、作り直すべき 2 つの場合を見落としている
**問題**: ⑴ 初めて呼ばれたときにモニターが 0 台だと、`Changed` は 0 件で、数も `seen` の初めの値 0 と同じになる。だから「初回は全部が Changed に見えるので必ず作り直しの道を通る」は成り立たず、`warn!` が 1 行も出ない。設計自身の縛るテスト（0 台で 3 回呼んで `warn!` 1 行）と食い違う。⑵ 本番の `prepare_ghost_windows`（本体の `ghost_session.rs` L235・L287-L288）は、窓を開け直すたび（ゴーストの切り替えと再起動の `reopen_ghost_windows`）に、OS から直に数えたモニター（`enumerate_monitors()`）で `MonitorSnapshot`／`MonitorDpiTable` を入れ直す。`MonitorWatch` は World の資源なので切り替えの後も残り、モニターの component が変わらない限り作り直さない。今は毎フレームの比べで wintf の表へ揃い直っていたのに、それが無くなる。設計はこれを「手で置き替えるテスト」の話としか書いていない（「見直しが要る変化」・`research.md` の 11.3）。
**影響**: 設計は「ゴーストに見える振る舞いは変わらない」と書いている。けれども、2 つの数え方が食い違ったときに切り替えの後で作業領域の下端が揃い直らない道ができる（要件 3.1・4.7）。また、⑴ のせいで縛るテストが設計の書いた手順のままでは緑にならない。`ghost_session.rs` は C23 で触れないので、直しは `work_area_sync.rs` の中だけで済ませる必要がある。
**直し方**: 見張りの条件を「初めて呼ばれたとき（`seen: Option<usize>` が `None`）」「モニターの component の `Changed` が 1 件以上」「数が変わった」「資源 `MonitorSnapshot`・`MonitorDpiTable` が入れ直されたか変わった（`SystemState` に `Res<…>` を入れて `is_changed()` で見る）」のどれかにする。縛るテストに「資源を入れ直したフレームは作り直す」の 1 本を足す。設計の本文の「初回は全部が Changed に見える」を直し、`reopen_ghost_windows` の道を「見直しが要る変化」に書く。
**要件**: 3.1・4.1・4.7・7.3
**根拠**: design.md「49 番 作業領域の源の同期を変化のあるフレームだけにする」・「見直しが要る変化」の 3 つ目・「失敗の扱い」の 49 番

🔴 **大事な問題 3**: 31 番で COM の初期化を消すと、同じウェーブの `input-crate-split` が移すテストが壊れうる
**問題**: 設計は「両方のテストのバイナリにプロセスの寿命の MTA がある」ことを理由に、`frame_balloon_runtime_test_support.rs` の `manual_balloon_runtime`（L27-L32）ほかから `CoInitializeEx` を消す。けれども、この支えは test-support の公開の項目で、本体の `input_events/balloon_pressed_tests.rs` L30 と `balloon_wheel_tests.rs` L25 が使っている。C23 の `input-crate-split`（④）は、そのテストを新しい crate `areka-input` へ移し、`areka-runtime` の test-support を dev-dependency で引く（同 brief の L43）。MTA を据える `crash_report_test_support.rs` の `INSTALL` は `#[cfg(test)]` なので、`areka-runtime` 自身のテストのバイナリにしか入らない。本体のものは `runtime_tests/mod.rs` にある。どちらも `areka-input` のテストのバイナリには入らない。wintf の WIC（`com/wic.rs` の `CoCreateInstance`）と WUC（`com/wuc.rs` L148 の暗黙の MTA の前提）は、COM が立っていないと失敗する。`research.md` の 11.1 が調べたのは 2 つのバイナリだけである。
**影響**: どちらの spec が先に着地しても、`areka-input` のテストが `GraphicsCore`／`WucGraphicsResource` の `expect` で赤になりうる。しかも直す場所は兄弟の持ち物（`crates/areka-input/`）で、要件 7.3 によりこの spec は触れない。設計は要件 7.7 の申し送りを 0 件と書いているが、それが事実でなくなる。
**直し方**: `gpu_on`（と test-support の `manual_balloon_runtime`）では COM の初期化を残す。そのうえで失敗を捨てずに確かめる（`S_OK`／`S_FALSE` は通し、それ以外は `expect` で止める）。31 番の「失敗を捨てている」はこれで直る。初期化の行を消す案を取るなら、`input-crate-split` のセッションへ「`areka-input` のテストのバイナリにプロセスの MTA を据える」ことを申し送り、要件 7.7 の記録に書く。どちらの案かは設計の討議で決める。
**要件**: 3.1・3.2・7.3・7.7
**根拠**: design.md「31 番 GPU の World の組み立て」・「要件の対応」の 7.7・「失敗の扱い」の 31 番

## 設計の良い所

1. **114 件と件 1〜14 を落とさずに追える形**: 指摘ごとの表が「直し方・縛るテスト・束」を 1 行ずつ持つ。要件の対応表が 1.1〜9.8 を全部受けているので、タスクの生成とかけ直しの audit で抜けを機械で数えられる。
2. **C23 の約束と依存の向きを、直し方を選ぶ理由として使っている**: 49 番で `same_monitors` の署名を変える案を退け、2 番で上限の案と捨てる案を退けた（§8 と `window-work-area-clamp` の決め）。33・99 番では dev-dependency を引く補助を `cfg(test)` に閉じて `Cargo.toml` を変えない。範囲の外で触るファイルを一覧にし、T27 の差分の手順で確かめる所まで書いてある。

## 最後の判定

- **判定: GO**（大事な問題 3 件を、設計の討議で直すことを条件とする）
- **理由**: 骨組み（相の並び・依存の向き・置き場の決まり・27 の束）は今のアーキテクチャと C23 の約束に合っていて、要件は全部受けられている。3 件はどれも、1 つの直しの中の場所の抜けか見張りの条件か COM の初期化の残し方で、設計の作り直しは要らない。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion audit-emo2-boot-frame`）で、大事な問題 1〜3 を、設計の OPEN QUESTIONS 1〜7 と一緒に議題にする。問題 3 は案（初期化を残して失敗を確かめる／消して `input-crate-split` へ申し送る）を開発者に選んでもらう。
  2. design.md を直す（4 番の表に 9 か所目を足す・49 番の見張りの条件と縛るテスト・31 番の初期化の扱いと要件 7.7 の行）。
  3. `/kiro-spec-tasks audit-emo2-boot-frame` でタスクを作る。

## 対話の材料（設計の討議へ）

- 問題 2 の ⑵ は、今の作りでは 2 つの数え方（OS から直に数えたもの・wintf の component）がふつうは同じ値になるので、実害が出る場面は狭い。それでも、毎フレーム揃い直す今の保証を黙って外すことになるので、資源の変化も見る形にしておくのがいちばん小さい完全な直しである。
- 問題 3 で初期化を消す案を取るなら、`input-crate-split` の設計がもう MTA を据える形になっているかどうかを、そのセッションに先に聞いてから決めるのがよい（持ち場の事実はそのセッションへ相談する決まり）。
