# ギャップ分析（audit）: audit-emo2-boot-spine

> 2026-11-17 `/kiro-validate-gap`。この spec では、ギャップ分析の段で audit をかける（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。指摘には 3 つの組を通した番号 1〜102 を付けた。以後の段はこの番号で指す。この文書は分析と選択肢を並べるだけで、直す対象は要件の討議で確定する。

## 0. 要約

- 受け持つ **125 ファイル・32,204 行**を全部、全行読んだ（`.rs` 124 本・32,140 行と `Cargo.toml` 64 行）。内訳は本番の名前のファイル 42 本・9,966 行（examples の 8 本・1,092 行を含む）、テスト 82 本・22,174 行。
- **指摘は 102 件**。組ごとの数は、必ず直す 3・直したほうがよい 33・できれば 66。
- 必ず直すのは次の 3 件。
  - 1: 台本の `\p[2147483648]` で、表示対象の番号の計算（`2*scope`）があふれる。デバッグのビルドでは seriko のスレッドが panic する。リリースのビルドではスコープ 0 の窓の絵が変わる。
  - 2: 揺れるテスト `spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits` の根の形が分かった（brief の件 2）。終わりの条件が「終わりの知らせ」だけを待ち、別のスレッドから届く目印の面を待っていない。
  - 3: spine の足場の後片付けが、ゴーストの降ろしの失敗（アクターのスレッドの panic）を `let _ =` で捨てている。そのため、壊れていてもテストが緑のまま通る。
- brief に預かった 8 件（要件 5）は、今の `main` で全部確かめ直した（3 節）。
  - 件 1（audit #16）: 呼び手は要件 5.1 が挙げる 3 か所より多い。**23 ファイル**（4 つの crate・examples 1 本・本体の `input_events` のテスト 1 本）ある（議題 2）。
  - 件 6: 17 本のどれも、新しい依存なしで戻せる（`areka-runtime` の dev-dependencies に番人の 3 つの crate は前から入っている）。ただし 3 本はほかの spec の持ち物のファイルにある。
- 開発者の裁定が要る候補は 6 件（25・35・81・82 と、約束を変える 48・62）。外への約束（要件 1.6）に響く指摘は 8 件（4 節）。C22 の約束に触れるのは 2 件（7 節）。
- 規模は **20〜24 タスク**と見込む。20 を越えうるので、要件 2.6 の議題（問い 1）が要る。

---

## 1. 測り直しと作法（要件 1.1〜1.4）

### 1.1 取り込んだ main と範囲の大きさ

- 作業のブランチ（`claude/kiro-start-audit-emo2-boot-spine-30ddb6`）の HEAD は `b2cb58f9`。main `5749a4c7` の上に、spec の初期化のコミットが 1 つ載っている（`spec.json`・`requirements.md` だけ）。`git merge-base --is-ancestor origin/main HEAD` で、main を取り込み済みと確かめた。コードは main と同じ。
- 範囲の決め方は Project Description のとおり。次の (1)〜(3) を合わせて 125 ファイル。
  - (1) `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` の全部（`git ls-files`）から、ほかの spec の持ち物を除いた分。除いたのは、`frame/`・`frame.rs`・`frame_*`（`audit-emo2-boot-frame`）、`switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs`（`audit-emo2-boot-switch`）、`placement_tests/transition_*`（`audit-placement-follow-residue`）。
  - (2) `crates/areka/src/session/` のうち `hooks.rs`・`reload*`・`runtime_boot*`・`switch_copy*`・`same_folder_pin*`。
  - (3) e2e の 2 本（`script_security_e2e_tests.rs`・`readme_script_e2e_tests.rs`）。
- 範囲の外のファイル（`Cargo.toml` 以外の `.rs` でないもの）は 0 本。

| 区分 | ファイル | 行 |
|---|---:|---:|
| `areka-runtime` の本番の名前のファイル（`src/`） | 29 | 7,606 |
| うち `#[cfg(any(test, feature = "test-support"))]` の下の足場（`spine.rs`・`spine_wait.rs`・`spine_hold_support.rs`） | (3) | (1,158) |
| `areka-runtime` の examples（`collision-probe*`） | 8 | 1,092 |
| `areka-runtime` のテスト（`_tests.rs`・`_test_support.rs`・`placement_tests/`） | 41 | 10,083 |
| `areka-runtime/Cargo.toml` | 1 | 64 |
| 本体の `runtime_tests/`（全部テスト） | 34 | 9,530 |
| 本体の `session/` の本番（`hooks.rs`・`reload.rs`・`runtime_boot.rs`・`same_folder_pin.rs`・`switch_copy.rs`） | 5 | 1,268 |
| 本体の `session/` のテスト（e2e の 2 本を含む） | 7 | 2,561 |
| **計** | **125** | **32,204** |

- 本番の名前のファイル（42 本・9,966 行）は、brief の棚卸㊴の実測「`areka-runtime` の本番 42 本 9,966 行」と合う。`.rs` 124 本・32,140 行も同じ実測と合う。
- 要件の Project Description の「124 ファイル・計 32,049 行」とは数が違う。違いは、`Cargo.toml` を 1 本と数えたかどうかと、行の数え方の端。今の実測は上の表で、行は `wc -l`。
- 1,000 行に近いファイルは brief の値と同じ（どれも変わっていない）。`spine_conformance_lap_tests.rs` 913・`spine_conformance_script.rs` 894・`session/reload_tests.rs` 857・`assets_tests.rs` 810・`session/reload.rs` 774・`lib.rs` 758。

### 1.2 ponytail の版

- 今入っているのは 5.3.3。`~/.claude/plugins/installed_plugins.json` の `ponytail@ponytail`（user scope・`lastUpdated` 2026-11-06）で確かめた。brief の起票のときは 5.3.2。
- `skills/ponytail-audit/SKILL.md` の中身は 5.3.2 と 5.3.3 で同じ（`diff` の差が 0）。物差しは変わっていない。

### 1.3 作法

- ponytail-audit の 6 つの物差し（バグ・危ない所・負荷・テストの抜け・遅い所・要らないコード）で見た。どの指摘にも具体的な場合を付けた。「使われていない」と書く前には、ツリー全体を grep した。
- 外した作法は 2 つ。
  - 「最大 20 件」を外し、全件を出した（要件 1.3）。
  - 「読まなかった所」を外し、125 ファイルを全部、全行読んだ（要件 1.4）。範囲を 7 つの束に分けて読み手を立てた。どの束からも、全ファイルを全行読んだ旨と、ファイルごとの行数の表を受け取った。合計は 32,204 行で、上の表と合う。
- `shortcut:`・`ponytail:` の注釈は範囲の中に 2 件あった。どちらも、書いた限界を今の負荷は越えていない。そのため指摘にしていない（5 節の ⑵）。
- プロジェクトの決まりへの違反も指摘に数えた。記録の無い失敗の道・1,000 行・決定論のテスト・一時フォルダは `target\` の下・終了の道・1 フレーム遅らせない・古くて事実と違う説明、の 7 つ。
- 想定した負荷は次のとおり。
  - 利用者 1 人のデスクトップで areka が 1 つ動き、ゴーストの窓が数枚ある。
  - ECS の相が毎フレーム（60fps 前後）回る。SHIORI の要求は 1 秒に数件まで。何時間も何日も動き続ける。
  - 他人のゴースト・シェル・バルーンのフォルダ（descript・台本・絵）は信用できない入力として扱う。
  - テストについては、`cargo test -p areka -j 4` で GPU のテストと CPU を取り合う状態を、ふだんの負荷とみなした。
- cargo のビルドとテストは回していない（読むのと grep だけ）。

### 1.4 読んだ分け方（7 つ・全部読んだ）

| 束 | 中身 | ファイル | 行 |
|---|---|---:|---:|
| A | `Cargo.toml`・`examples/collision-probe*`・`lib.rs`・`adapter*`・`assets*`・`hit_region.rs`・`target_map.rs`・`balloon_background*`・`sample_test_support.rs`・`seriko_clock_tests.rs`・`wire_tests.rs` | 25 | 5,300 |
| B | `spine.rs`・`spine_wait*`・`spine_hold_*`・`talk_clock.rs`・`talk_lifecycle*`・`crash_report_test_support.rs`・`source_text_test_support.rs`・`session_hooks.rs`・`scope_scale.rs`・`special_name.rs` | 15 | 4,744 |
| C | `consumer_ledger*`・`shell_balloon_resolve*`・`shell_box_assets*`・`shell_carry*`・`shell_cursor_assets*`・`shell_cursor_table*`・`skin_facts_copy*`・`strict_judges*` | 20 | 4,372 |
| D | `reload_args*`・`reload_cue*`・`switch_cue.rs`・`user_break_cue*`・`window_state_cue*`・`zorder_cue*`・`zorder_wiring_tests.rs`・`script_restriction*`・`placement_tests/`（`transition_*` でないもの） | 19 | 4,429 |
| E | 本体の `runtime_tests/` の `mod.rs`・`spine.rs`（`SpineHarness`）・`spine_conformance_*`（6 本）・`spine_test_support.rs` | 9 | 4,574 |
| F | 本体の `runtime_tests/` の残り（`assets*`・`consumer_ledger*`・`film_playback_e2e_tests.rs`・`shell_*`・`spine_*_tests.rs` 9 本・`strict_judges*`・`switch_cue*`・`wire_tests.rs`・`zorder_wiring_tests.rs`） | 25 | 4,956 |
| G | 本体の `session/` の 12 本 | 12 | 3,829 |
| 計 | | **125** | **32,204** |

- 束の数え直し: 7 つの束の一覧を合わせて `sort` し、範囲の一覧と `diff` をとった。差は 0。重なりも 0。
- 主な指摘は、根拠の行まで読み直して確かめた（8 節）。

---

## 2. 指摘の一覧（通し番号・3 つの組を通す）

各指摘の印は次のとおり。

- 〔約束〕: 直すと「外への約束」に触れる。(a) ゴーストと SSP 互換の振る舞い、(b) `areka-runtime` の公開の面、(c) ログの行の語と target。響く読み手も書く（要件 1.6）。
- 〔裁定〕: 要件 2.2 の 3 種類（⑴ 開発者の裁定に反する直し・⑵ わざと置いた先送りの印・⑶ SSP 互換か ukadoc の正典に関わる振る舞いの変化）に当たる、またはその候補。
- 〔件 n〕: brief に預かった件（要件 5 の n）と同じ所を指す（同じ直しを 2 回数えない）。
- 〔範囲の外〕: 直しの本体が、範囲の外のファイルにある（持ち主を書く）。
- 〔C22〕: 直すと C22 の約束（要件 7.3〜7.5）に触れる。
- 規模: S は 1 タスク以内、M は 1〜2 タスク。

### 件数

| 組 | 番号 | 件数 |
|---|---|---:|
| 必ず直す（Must fix） | 1〜3 | **3** |
| 直したほうがよい（Should fix） | 4〜36 | **33** |
| できれば（Nice to have） | 37〜102 | **66** |
| 計 | | **102** |

- 束ごとの読み手が出した番号（A1・B1 など）は、まとめるときに付け直した。読み手どうしで同じ所を指した指摘は 1 件にまとめた（11・19・29 ほか）。

### 2.1 必ず直す（1〜3）

#### 1. 台本のスコープの番号が大きいと、表示対象の番号の計算があふれる
- 場所: `crates/areka-runtime/src/target_map.rs:L19-L29`（`shell_target`＝`2*scope`・`balloon_target`＝`2*scope+1`）・`L36-L38`（`scope_of`）。呼び手は `adapter.rs:L54,L62,L73,L81,L89,L94,L99`。
- 種類: バグ・危ない所（信用できない入力で落ちる）
- 何をする所か: 話し手の番号（スコープ）を、表示対象の番号（`TargetId`）に直す。シェルは偶数、バルーンは奇数。`scope_of` は話し手の名前（`ActorKey`）を `u32` として読む。
- 問題（具体的な場合）: ゴーストの台本に `\p[2147483648]\s[0]` とあると、次の順に進む。
  - `speaker_scope_n`（`crates/areka-parsers/src/sakura/decode.rs:L454-L459`）が `u32` として受け入れる。
  - seriko が、知らないスコープへの Show を足して送る（`areka-seriko/src/state.rs:L214-L220`）。
  - `shell_target(2147483648)` の `2*scope` があふれる。
  - デバッグのビルドでは seriko のスレッドが panic し、絵の切り替えもアニメも全部止まる。リリースのビルドでは `TargetId(0)` に回り込み、スコープ 0 の窓の絵が勝手に変わる。
  - 同じ心配への手当ては、`frame/dpi.rs:L404`・`mcp/dump_surface_judge.rs:L71`・`window-placement.rs:L367` には既にある。ここだけ抜けている。
- 直し方: `scope_of` を `key.as_str().parse::<u32>().ok().filter(|&s| s <= i32::MAX as u32)` の 1 行にする。これで adapter の既にある「`warn!` を出して捨てる」道に乗る。`target_map` のテストに `scope_of("2147483648") == None` を 1 行足す。
- 直さないと: 他人のゴーストの台本 1 行で、seriko が落ちるか（デバッグ）、別の窓の絵が変わる（リリース）。
- 〔範囲の外〕〔C22〕窓の側から来る `u32` の scope の写しは `frame/dpi.rs:L404-L413` にある。usize から u32 への変換は確かめているが、`2*scope+1` のあふれは確かめていない。持ち主は `audit-emo2-boot-frame` で、C22 では触れない＝申し送る（6 節）。
- 〔裁定〕の候補: 上限を `i32::MAX` に置くのはこの直しの仮置き。SSP が大きな `\p[n]` をどう扱うかは測っていない。今の「数でない key は warn して捨てる」と同じ扱いにするだけなので、⑶ には当たらないと見る（議題 8）。
- 規模: S

#### 2. 揺れるテストの終わりの条件が、確かめたい中身の到着を待っていない〔件 2〕
- 場所: `crates/areka/src/runtime_tests/spine_third_scope_talk_tests.rs:L115-L143`（`drive_talk` の輪）・`L211-L214`（起動の会話の終わりの条件）・`L226-L230`（終了の会話）・`L151-L169`（確かめ）
- 種類: バグ（テストが負荷で揺れる＝決定論のテストの決まりに反する）
- 何をする所か: 起動の会話と終了の会話を Tick で進め、`done` が真になったら止める。その後で「scope 2 の面」と「scope 0 の目印の面 2100」が届いたかを確かめる。
- 問題（具体的な場合）:
  - `done` は「`TalkEnded` を見た」だけで決まる。
  - `TalkEnded` は、台詞のスレッドが talk の写しの受け口を落とすときに、表示の寿命の線（`lifecycle_rx`）へ直に送る（`areka-runtime/src/talk_lifecycle.rs:L251-L265`・`runtime_tests/spine.rs:L488-L489`）。
  - 目印の面 `\s[2100]` は別の道を通る。台詞のスレッドが seriko の受け箱に積むだけで（`areka-seriko/src/actor.rs:L243-L246`）、別のスレッドの seriko アクターが `PresentBridge`（`adapter.rs:L141-L150`）を通して別の線 `rx` へ出す。
  - この 2 つの間に、先か後かの保証は無い。しかも 1 回の反復の中で、`rx` を先に、`lifecycle_rx` を後に汲む（L123-L136）ので、すき間が広がる。
  - `\s[2100]` の後ろには「おわり。」の 4 文字しか無い（注入時刻でおよそ 200ms＝100ms の Tick で 2〜3 回ぶん）。`-j 4` で seriko のスレッドが数 ms 走れないと、`TalkEnded` だけを見て輪を抜ける。そして L159-L163 で赤になる。単独なら間に合って緑になる。この形は brief の症状と合う。
  - 起動の側で取りこぼした目印は、終了の側の列の先頭に入る。そのため終了の側の `marker > third` も落ちうる（二次の赤）。
- 直し方: 確かめる中身を全部 `done` の条件に入れる。起動の会話は `ended && seen.position(shell_target(0), MARKER_SURFACE).is_some()`。終了の会話はそれに `Unload` と、scope 2 の後の目印を足す。表明（L151-L169）と同じ式を使えばよい。目印が本当に来なければ進みの目印が増えないので、本物の故障は今どおり 30 秒の `［止まった］` で赤になる。
- 決定論で赤にするやり方（要件 4.2・5.2）: 足場の `boot_with`（`runtime_tests/spine.rs:L445-L451`）で `spawn_seriko` に渡す出力を、テスト専用の門つきの `SurfaceOutput` で包む。門は `Show { scope: "0", surface_id: 2100 }` だけを止め、テストは `drive_talk` が戻ってから門を開ける。今のコードは毎回 L159-L163 で赤になり、直した後は緑になる見込み（作って確かめるのは実装の段）。
- 足場の中の待ち（`spine_wait.rs` の芯・GPU の許可・固まる台本）は、条件を錠の中で見ている。そのため、確かめと待ちの間の競り合いも、知らせの取りこぼしも無い。壁時計の上限（30 秒・300 秒・500 秒）も、ふだんの道より桁違いに長い。揺れの元は、テスト自身の終わりの条件だけだった（束 B と束 F が別々に同じ結論を出した）。
- 直さないと: 全体テストがときどき赤になり、原因を探す手間が繰り返される。
- 規模: S

#### 3. spine の足場の後片付けが、ゴーストの降ろしの失敗を捨てている
- 場所: `crates/areka/src/runtime_tests/spine.rs:L594-L597`（`SpineHarness::shutdown_bounded`）
- 種類: テストの抜け（本物の欠陥が緑で通る）
- 何をする所か: テストの最後にゴーストを正しく降ろす。spine 系のテスト 12 ファイルと `ghost_session_restart_tests.rs` が使う（呼び出しは 13 か所）。
- 問題（具体的な場合）:
  - `let _ = ghost.shutdown(CloseReason::User { scope: 0 });` が戻り値を捨てている。
  - `GhostRuntime::shutdown` が `Err` を返すのは、アクター（kanade・dispatcher・shiori）のスレッドが panic したときだけ（`crates/areka-ghost/src/runtime.rs:L327-L332`）。
  - 偽の SHIORI は、台本に無い呼び出しを受けると shiori のスレッドで panic する（`areka-runtime/src/spine.rs:L244`・`L267`・`L278`）。たとえば降ろしの途中で `OnClose` が 2 回送られる退行があると、shiori のスレッドが落ちて `Err` が返る。それでも捨てるので、テストは緑のまま。
- 直し方: `.expect("spine ghost shutdown")` にする。このクロージャは `run_bounded_watching` の別のスレッドで動くので、panic は待つ側に `［相手が居ない］` の赤として届く（`spine_wait.rs:L331-L343`）。降ろしは何度呼んでも同じ結果になる作りなので、一周のテストの終了の段の後に呼んでも壊れない。13 か所の呼び出しが全部緑になることを確かめてから入れる。
- 直さないと: アクターの panic という本物の欠陥が、`error!` を 1 行残すだけでテストを通り抜ける。
- 規模: S

### 2.2 直したほうがよい（4〜36）

#### 本番のコード（`areka-runtime` の `lib.rs`・`assets.rs`・`hit_region.rs`）

#### 4. `BootWiringError::SpawnUi` は一度も作られず、その説明も事実と違う〔件 1〕〔範囲の外〕〔約束〕
- 場所: `crates/areka-runtime/src/lib.rs:L246-L254`（変種と説明）・`L190`・`L196-L198`・`L383`・`L395`・`L434-L435`・`L507-L519`
- 何をする所か: バルーンの文字の層の UI アクターを起こすのに失敗したときの枝。
- 問題: 説明は「UI スレッドの外からの呼び出しを見つけると `NotUiThread` を返す」と書く。けれども `areka_actor::spawn_ui` はいつも `Ok` を返す（`areka-actor/src/ui.rs:L66-L72` 自身が「現状この `Err` を返さない」と書く）。だから `lib.rs:L515-L518` の `Err` の腕には一度も入らない。読む人は、この枝が守ってくれていると思い込む。
- 直し方: 要件 5.1 のとおり、`spawn_ui` と `spawn_emo_text` の戻り値から `Result` を外す（`(UiSender, JoinHandle)` にする）。そのうえで、lib.rs の上の行を全部直す。変種 `SpawnUi` とその説明を消し、`L513-L519` の `match` を `let (emo_text_sink, _emo_text_join) = spawn_emo_text(Rc::clone(&runtime));` の 1 行にする。`RuntimeOutcome.seriko: Option<areka_actor::ActorHandle>`（L297）が残るので、`areka-actor` への依存は今のまま要る。
- 直さないと: 死んだ型と事実と違う説明が残り、呼び手ごとに使わない `match`・`expect` を書き続ける。
- 響く読み手: 呼び手は 23 ファイルある（3 節の件 1 の表）。要件 5.1 が挙げる 3 か所より多い。`BootWiringError` は `pub` だが、crate の外で名指す所は 0 件。
- 規模: M（4 つの crate にまたがる機械的な直し）

#### 5. `hit_region.rs` の頭の「`crate::` フリー規律」は全部古い〔件 4〕
- 場所: `crates/areka-runtime/src/hit_region.rs:L45-L57`
- 問題:
  - `#[path]` で `hit_region.rs`・`target_map.rs` を取り込む example は 0 件。probe は crate の公開面を `use` している（`examples/collision-probe.rs:L218-L219`）。examples の `#[path]` は、どれも子のファイルのためのもの（`collision-probe.rs:L229-L241` ほか）。
  - `hit_region.rs` の本番のコードに `crate::` は 0 件（使うのは `super::target_map::shell_target` と `areka_emo_present` だけ）。
  - `lib.rs` が `is_benign_boot_error` を呼ぶ所も無い（名前は説明の中にだけ出る: `lib.rs:L384-L385`・`L444`）。定義は `crates/areka/src/boot_config.rs:L623` で、呼び手は `ghost_session.rs:L783`・`session/runtime_boot.rs:L137`・テスト。
  - 「破るとコンパイルできなくなる」という警告は、事実と違う。
- 直し方: 節を消す。残すなら「probe は `areka_runtime::hit_region`・`target_map` を crate の公開面で `use` する（`#[path]` では取り込まない）。だからこのファイルの書き方に、置き場所の制約は無い」の 1 文にする。
- 直さないと: 次に触る人が、要らない制約を守ろうとする。
- 規模: S

#### 6. `build_boot_assets` の `#[allow(dead_code)]` は何もしていない〔件 3〕
- 場所: `crates/areka-runtime/src/assets.rs:L215`
- 問題: `assets` は `pub mod`（`lib.rs:L16`）で、関数も `pub fn`。lib の公開の関数には `dead_code` の警告がもともと出ないので、この許可は効いていない。理由の文「本番の起動はシェル名つきの兄弟を通る（呼び手はテスト）」は今も事実。
  - 呼び手は全部テスト。`areka-runtime` の `assets_tests.rs`・`assets_shell_tests.rs`・`assets_markers_tests.rs`・`frame_attach_tests.rs`、本体の `runtime_tests/frame.rs`・`frame_attach_carry_tests.rs`・`frame_scale_notice_integration_tests.rs`・`frame_visibility_integration_tests.rs`・`spine.rs:L87,L380`。
  - 本番は `lib.rs:L358-L375 build_boot_assets_for` から `build_boot_assets_with_shell` へ通る。
- 直し方: 行を消す。「呼び手はテストだけ」を型で示したいなら、`#[cfg(any(test, feature = "test-support"))]` に替える。
- 直さないと: 警告を消すための印に見えて、読む人を迷わせる。
- 規模: S

#### 7. lib.rs の頭と `wire_runtime` の説明が、今のコードと違う
- 場所: `crates/areka-runtime/src/lib.rs:L8-L11`・`L384`・`L436`・`L571`
- 問題:
  - L8-L11 は「`target_map`→`adapter`→`talk_clock`→`assets`→`frame` の順で、左は右を import しない」と書く。けれども `adapter.rs:L21` は `crate::switch_assets` を import し、`switch_assets.rs:L30-L31` は `assets` と lib を import する。L11 の `ghost_session.rs`／`main.rs` も、今は別の crate（`crates/areka`）にある。
  - L436 の手順 3 は「ここで `ClockedTextSink::new`」と書く。実際に作るのは `crates/areka/src/session/runtime_boot.rs:L80`。
  - L384 の「main の `is_benign_boot_error`」は、定義が `boot_config.rs:L623` にある。
  - L571 の「`build_boot_assets` の構築ループ」は、今は `build_balloon_assets_with_limits`。
- 直し方: 文を事実に合わせる。依存の順の節は消してもよい。
- 規模: S

#### 8. 「選びの種は抽選の種と別」のテストが、本番の結線を見ていない
- 場所: `crates/areka-runtime/src/wire_tests.rs:L5-L10`・`lib.rs:L575-L576`
- 問題: テストは `entropy_seed() != entropy_seed()` を見ているだけ。これは std の `RandomState` の性質で、`lib.rs:L576` を `let pick_seed = seed;` に変えても緑のまま。要件 3.6 の守りになっていない。
- 直し方: 2 つの種を返す `fn entropy_seeds() -> (u64, u64)` を置き、`wire_runtime` がそれを使う。テストはその `.0 != .1` を見る（数行）。
- 直さないと: 種をうっかり 1 つにまとめても、誰も気づかない。
- 規模: S

#### spine の足場と時計（`areka-runtime` の `spine*`・`talk_*`）

#### 9. 待ちの説明が、今の打ち切り方と食い違っている
- 場所: `crates/areka-runtime/src/spine.rs:L13-L15`・`spine_wait.rs:L4-L5`・本体の `runtime_tests/spine.rs:L19-L22`
- 問題:
  - spine.rs は「待機はすべて壁時計の `SPIN_WAIT` で有界化」と書く。けれども今の打ち切り方は 3 通りある。目印のある待ちは「30 秒進まなければ打ち切り・総時間は 300 秒まで」（`spine_wait.rs:L213-L282`）。GPU の許可は 500 秒（L441）。`run_bounded` は総時間の `recv_timeout`（L359-L372）。
  - spine_wait.rs は「`run_bounded` は芯の上に載せた」と書くが、`run_bounded` は芯（`wait_until_with`）を通らない。芯から借りるのは失敗の文言だけ。
  - 本体の `runtime_tests/spine.rs:L19-L22` は「ハイブリッドの輪は `wait_until` を流用しない」と書く。けれども `drive_talk` と S2 Phase 1 は使っている。
- 直し方: 3 か所の文を今の形に直す（コードは変えない）。
- 直さないと: 2 番のような揺れを調べる人が、打ち切りの形を読み違える。
- 規模: S

#### 10. `talk_clock.rs` の頭が「骨格だけ」と言っている
- 場所: `crates/areka-runtime/src/talk_clock.rs:L7-L8`
- 問題: 「骨格のみ。…実装は task 2.2 が担う」とある。けれども実装（L28-L135）もテスト（L137-L502）も揃っている。
- 直し方: 2 行を消す。
- 規模: S

#### 台帳・シェルの資産（`consumer_ledger*`・`shell_*`）と、分けたテストの説明

#### 11. 分けたテストのファイルの説明が、相手のファイルにあるテストのことを書いている
- 場所:
  - `areka-runtime` の側: `consumer_ledger_agreement_tests.rs:L1-L20`・`shell_balloon_resolve_tests.rs:L1-L6`・`shell_carry_tests.rs:L1-L12`・`zorder_wiring_tests.rs:L10`・`L25-L29`・`consumer_ledger.rs:L58-L60`
  - 本体の側: `runtime_tests/consumer_ledger_agreement_tests.rs:L20`・`shell_balloon_resolve_tests.rs:L4-L6`・`zorder_wiring_tests.rs:L1-L29`
- 何をする所か: `runtime-crate-split` はテストを関数の単位で 2 つの crate に割った。その両方の半分の頭の説明。
- 問題:
  - 両方の半分が、割る前の説明をそのまま持っている。たとえば `areka-runtime` の `consumer_ledger_agreement_tests.rs` は「11 の受け口を本物の送信端つきで組み、cue を全員へ配る」と書く。けれどもそのテストは本体の側にあり、こちらにあるのは見本の組の一致の 1 本だけ。
  - `consumer_ledger.rs:L58-L60` が名指す `consumer_ledger_agreement_tests.rs` は 2 つある。
  - `zorder_wiring_tests.rs` の「据える側（`mod.rs`）」は、今は `lib.rs`。
- 直し方: どの頭の説明も、そのファイルに本当にあるテストだけに縮める。相手の分は「`<相手の crate>` の同じ名前のファイルにある」の 1 行にする。
- 直さないと: 読む人が無いテストを探して迷う。あると信じて、相手の側のテストを消しかねない。
- 規模: S

#### 12. `replay_without_a_ghost_does_not_panic` が何も確かめていない
- 場所: `crates/areka-runtime/src/shell_carry_tests.rs:L127-L133`（確かめる相手は `shell_carry.rs:L322-L328`）
- 問題: 説明は「`warn!` で残す」と書くが、`assert` が 1 つも無い。`replay` から `warn!` を消しても、何かを送るように変えても緑のまま。
- 直し方: `log_capture_kit::capture` で囲み、`event == "reload_shell_carry_unavailable"` の warn が 1 件出ることを確かめる（`log-capture-kit` は既に dev-dependencies にある）。
- 規模: S

#### 13. 正準台帳の同じ確かめが重なっている
- 場所: `crates/areka-runtime/src/consumer_ledger_tests.rs:L8-L21`・`L26-L39`・`L569-L575`
- 問題: `canonical_has_move_and_bind_registered` と `canonical_registers_both_zorder_pairs` の assert は、`canonical_builds_without_duplicate`（L314-L321・L331-L338）と `selector_bearing_registration_owns_only_listed_selectors`（L84-L87）が既に確かめている。bind の行は 3 回確かめている。
- 直し方: 前の 2 本を消す（約 -30 行）。
- 規模: S

#### 台本の合図の受け口（cue）

#### 14. 相順の檻 2 本（t_zwi03・t_zwi04）が、本番ではもう使っていない仕組みを試している
- 場所: `crates/areka-runtime/src/zorder_wiring_tests.rs:L199-L261`（`L203-L221`・`L16-L19`・`L291-L296`）
- 問題: 本番の相は `Update` の段へ移った。同じファイルの t_zwi06（L286-L289）は「`.before(apply_zorder_chain)` が残っていないこと」を主張している。ツリー全体で、本番のコードが `.before/.after(apply_zorder_chain)` を使う所は 0 件。2 本が試しているのは、テスト専用の `raise_the_dirty_mark` に対する bevy の順序の指定だけで、どちらを壊しても本番の振る舞いは変わらない。
- 直し方: t_zwi03・t_zwi04・`raise_the_dirty_mark`・`wired_finalize_world` と、t_zwi06 の `contains(FrameFinalize)`（L291-L296）を消す。頭の説明の ⑵ と L16-L19 も消す（約 -70 行）。
- 直さないと: 何も守らない檻が、守っているように読まれ続ける（古くなったテストは外す方針）。
- 規模: S

#### 15. `\v` を運ぶ cue の荷物が壊れていると、debug を 1 行残すだけで捨てる
- 場所: `crates/areka-runtime/src/window_state_cue.rs:L107-L114`・`window_state_cue_tests.rs:L290`（t_wsc10）
- 問題: 運ぶ名前 `\v` を受け持つのは、名簿（`consumer_ledger.rs`）で WindowStateSink だけ。それなのに、パーサの退行などで `Custom{command:"\\v", params: Null}` が届くと debug を 1 行残すだけで、`\v` が効かない。既定のログ水準では誰にも見えないので、実質は黙った失敗になる。設計（`window_state_cue` の design の L358）は「ZOrderCueSink と同じ宛名の規律」とし、debug にとどめる例外を `set` だけに限っている。
- 直し方: 開けない荷物の枝で、`command == RAISE_TAG_CARRIER` のときだけ `warn!` にする。t_wsc10 の `\v` の分を「warn 1 件」に移す。
- 参考: `user_break_cue.rs:L167-L173` も同じ形だが、L164-L166 に理由（第 1 引数で担当が分かれる名前）を書いて決めている。こちらは直さない。
- 響く読み手: その文面を字面で読む実機の確かめ・`tools/perf`・`doc/` は 0 件（grep で確かめた）。ログが 1 行増えるだけ。
- 規模: S

#### 本体の `session/`

#### 16. 読み直しの記録のゴースト名の作り方を、自前で写している
- 場所: `crates/areka/src/session/reload.rs:L732-L757`（`ghost_name`）
- 問題: 同じ規則（descript の `name`、無ければ根の絶対パスから末尾の区切りを落とす）の正本 `areka_ghost::config::listed_name`（`crates/areka-ghost/src/config.rs:L41`）が既にある。mcp の `resolve.rs:L82` と `readme/opener.rs:L184` はそれを使っている。正本の規則が変わると、ここの記録の綴りだけがずれ、`get_log` を `ghost_name` で絞ったときに `reload_failed` の行が拾われなくなる。
- 直し方: 本体を `listed_name(session.names().and_then(|n| n.name.as_deref()), session.ghost_dir())` の 1 式にする（約 -15 行。今の出力は 1 文字も変わらない）。
- 規模: S

#### 一周の適合のテスト（本体の `runtime_tests/spine_conformance_*`・`spine.rs`）

#### 17. 「`%username` が展開されること（R3.10）」の確かめが、言っているほど効いていない
- 場所: `crates/areka/src/runtime_tests/spine_conformance_script.rs:L373-L385`・`L591-L617`
- 問題:
  - L383 は「記法が表示へ漏れれば、表示指令の列で捕まる」と書く。けれども表示指令の写し（`DisplayProjection`）は窓と面の番号しか持たず、文字を運ばない（`spine_conformance_support.rs:L711-L734`）。
  - 確かめられる場所は `OnTranslate` の Ref0 だけ。ところがその期待値は、製品の `areka_sakura::expand_system_vars` と `areka_kanade::events::on_translate` を呼んで作っている。`%username` を展開しなくなる退行が起きると、期待値も実測も同じく生の `%username` のままになり、緑で通る。
- 直し方: 起動の挨拶の `OnTranslate` の Ref0 を、逐語の定数 `r"\0\s[0]アヒル、おはよう。\e"` で書く。L383 の文は消す。
- 規模: S

#### 18. 「1 件も増えない」という主張を、届くのを待たずにすぐ読んでいる
- 場所: `crates/areka/src/runtime_tests/spine_conformance_support_tests.rs:L532-L561`・`spine_conformance_support_stage_tests.rs:L109-L133`
- 問題: 再生の側の Tick が SHIORI への呼び出しを起こさないことを、待ちが打ち切られた直後に件数を 1 回だけ読んで確かめている。dispatcher が Tick を誤って kanade へ流す退行が起きると、`OnSecondChange` は別のスレッドを通って非同期に記録される。負荷の下では、記録が届く前に `== 1` を読んで緑になる。
- 直し方: assert の前に、足場の回収の部品 `settle_bounded`（`runtime_tests/spine.rs:L133`）を 1 行挟む（足場の「負の検証は settle で回収する」の決まりと同じ形）。
- 規模: S

#### 19. 判定の説明「3 つの取り出し口はいずれも濾さない」が、例外を落としている
- 場所: `crates/areka/src/runtime_tests/spine_conformance_judge.rs:L16-L28`・偽の SHIORI の `crates/areka-runtime/src/spine.rs:L222-L228`
- 問題: 偽の SHIORI は、台本に無い `OnBalloonTimeout` を記録せずに 204 で答える。そのため一周の途中で製品が誤って `OnBalloonTimeout` を送っても、列に現れず緑になる。説明の一覧（L23-L28）は、この例外を書いていない。
- 直し方: 判定の説明と spine.rs の doc に「`OnBalloonTimeout`（実時間で決まるので、台本に無ければ記録しない）だけは列の照合の外」と 1 行ずつ足す。濾すこと自体は、実時間に頼らないための決めごとなので残す。
- 規模: S

#### 20. 2 スコープの合成の配置と窓のタイトルが、既にある支えの写し
- 場所: `crates/areka/src/runtime_tests/spine.rs:L184-L224`
- 問題: `two_scope_placements()` と `titles()` は、`areka_placement::spawn::test_support::{two_scope_placements, titles}`（`crates/areka-placement/src/spawn_test_support.rs:L17-L50`・`pub`）と値まで同じ。`placement_tests/spawn_assembly_tests.rs:L17` は向こうを使っている。
- 直し方: 2 つを消して向こうを `use` する（約 -38 行。`areka-placement` には触らない）。
- 規模: S

#### 21. 偽の窓の印と ERROR の数え方が、既にある支えの写し
- 場所: `crates/areka/src/runtime_tests/spine_conformance_lap_tests.rs:L306-L325`・`spine.rs:L260-L264`・`spine_move_cue_tests.rs:L20-L49`
- 問題: `fake_handle`・`attach_fake_window_handles` は `areka_placement::test_support::give_fake_handles`（`test_support.rs:L87-L95`）と同じ仕事で、写しが 2 つある。`count_level` は `areka_runtime::frame::test_support::count_level`（`frame_test_support.rs:L130-L133`）と同じ。
- 直し方: 既にある部品を呼び、写しを消す（約 -20 行。913 行のファイルの余白が増える）。
- 規模: S

#### 22. 出どころとして書いた `file:line` の参照の多くがずれている
- 場所: `spine_conformance_script.rs` の 117 か所・`spine_conformance_support_stage_tests.rs` 26・`spine_conformance_support.rs` 9・`spine_conformance_lap_tests.rs` 8・`spine_conformance_judge.rs` 3・`spine_conformance_support_tests.rs` 2
- 問題: 台本は「期待値の出どころは実装で、各定数の上に file:line を書いてある」（script L11-L13）を根拠にしている。抜き取りで約 25 か所を見ると、多くがずれていた。例: `events.rs:39` は今は 97 行、`msg.rs:307` は 593 行。`spine.rs:333-341` の待ちの部品は `spine_wait.rs` へ移った。
- 直し方: 行番号を外し、関数名・定数名（grep で引ける名前）だけを残す。名前はほぼ全部の注記に既に付いている。
- 規模: M（165 か所の機械的な直し。全数の確かめは実装の段）

#### 23. 古くて事実と違う説明と、それに隠れた死んだ定数
- 場所: `crates/areka/src/runtime_tests/spine_conformance_script.rs:L34-L44`・`L354`ほか
- 問題:
  - 頭は「消費側（一周のテスト）はまだ居ないので `#![allow(dead_code)]`」と書く。一周のテストは既にあり、この許可が今黙らせているのは使い手の無い `CHOICE_MENU_CLOSE`（L354・ツリー全体の grep で 0 件）だけ。
  - 数が古い記述もある。judge の L35「33 行」は実際は 53 行。L120「15〜16 要素」は実際は 26・25。script の L222 の `version = "0.0.1"` は今は 0.1.0。stage の L37 の `SpineHarness::new` は存在しない（`boot_with`）。support_tests の L6「spine.rs の余白は 44 行」。spine.rs の L17 の `[DENSE_SPIN](wait::DENSE_SPIN)` は、ツリーに無い道を指している。
- 直し方: 許可と L34-L44 の段落と `CHOICE_MENU_CLOSE` を消し（約 -14 行）、数と古い記述を今の値に直す。
- 規模: S

#### 24. 「本番の受け口の構成を spine でも忠実に再現する」は事実と違う
- 場所: `crates/areka/src/runtime_tests/spine.rs:L476-L480`（渡しているのは L501-L506）
- 問題: 本番の `runtime_sinks` は 13 本ある（`session/runtime_boot.rs:L59-L101`）。足場が渡すのは先頭の 4 本（面・文字・移動・表示の寿命）だけ。Z 順・readme・切替・読み直し・窓の状態の cue を含む台本を spine で流しても、届け先が無く何も起きない（束 E と束 G が別々に見つけた）。
- 直し方: 「本番 13 本のうち、先頭 4 本だけを同じ順で渡す」と書き直す。
- 規模: S

#### 25. 余韻の「300 反復」は、スレッドの順番の運に頼った待ち〔裁定〕
- 場所: `crates/areka/src/runtime_tests/spine_conformance_lap_tests.rs:L85-L92`・`spine_conformance_support.rs:L215-L232`
- 問題: 完了の条件が成り立った後、注入をやめて 300 反復（1 反復 200µs 眠る）観測を続け、kanade が「選択待ち」へ進むのを待っている。けれども待っている状態はテストから観測できない。22 本のスレッドが CPU を取り合う負荷で kanade が 300 反復のあいだ走れないと、製品が正しくても次の段の選択確定が棄却され、30 秒の `［止まった］` で赤になる。同じ形の赤が過去に出たことを、doc 自身が書いている（L160-L163・L680-L683）。
- 直し方: kanade の選択待ちを観測できる口が製品に要る。これは完了した spec の決め R2.9（製品のコードへ口を新しく作らない）とぶつかる。口を作らないなら、今の形と根拠を残すしかない。
- 〔裁定〕⑴ の候補（議題 3）。
- 規模: M

#### 本体の `runtime_tests/` の残り

#### 26. `SwitchInFlight` が無いことを確かめる行は、構造上落ちない〔件 6〕
- 場所: `crates/areka/src/runtime_tests/shell_balloon_resolve_tests.rs:L17`・`L135-L138`
- 問題: 試している関数（`record_installed_shell`・`installed_for`・`resolve_skin_target`）は `areka-runtime` にある。`SwitchInFlight` は本体の `pub(crate)` の型（`session/ghost_switch.rs:L160`）で、`areka-runtime` は本体に依存しないので、この型を入れることがそもそもできない。だからこの assert は何があっても緑になる。しかもこの 1 行のせいで、ファイルが丸ごと本体に残っている。
- 直し方: use と assert の 2 行を消す。4 本のテストは `areka-runtime/src/shell_balloon_resolve_tests.rs` へ移せる（`temp-path-kit` は `areka-runtime` の dev-dependencies に既にある）。代わりの親の `runtime_tests/shell_balloon_resolve.rs` と `mod.rs` の 1 行も消せる。
- 規模: S

#### 27. switch_cue のテスト 10 本が本体に残っている理由は、出し直しの import 1 行だけ〔件 6〕
- 場所: `crates/areka/src/runtime_tests/switch_cue_tests.rs:L9`・`runtime_tests/switch_cue.rs`
- 問題: 本体の項目は `crate::session::shell_balloon_switch::SkinKind` だけを使う。これは `areka_runtime::skin_switch::SkinKind` の出し直しにすぎない（`session/shell_balloon_switch.rs:L37-L40`）。`log-capture-kit` は `areka-runtime` も dev-dependencies に持っている。`areka-runtime` の側に `switch_cue_tests.rs` は無い。
- 直し方: ファイルを `crates/areka-runtime/src/switch_cue_tests.rs` へ移し、import を `crate::skin_switch::SkinKind` にする。`switch_cue.rs` に `#[cfg(test)] #[path] mod tests;` を足す（前例は `reload_cue_tests.rs`・`change_cue_tests.rs`）。代わりの親（8 行）と `mod.rs:L36` を消す。
- 直さないと: `areka-runtime` の部品のテストが、本体のビルドでしか回らない。
- 規模: S

#### 28. 文字の面の読み戻しに失敗しても 0 と数えるので、Clear の檻が空振りする
- 場所: `crates/areka/src/runtime_tests/spine_talk_close_tests.rs:L24-L33`・`L239-L242`・`L268`
- 問題: `text_surface_opaque` は、面が無いときも `read_back` が `Err` のときも 0 を返す（`unwrap_or(0)`）。文字の相が面を壊したり外したりしても、Phase 2 は「Clear が届いた」で緑になる。`counts[0] == 0` も同じ理由で空振りしうる。
- 直し方: `rt.surface(actor).expect("装着済み").read_back().expect("読み戻せる")` にする（呼び手は装着の後でしか使わない）。
- 規模: S

#### 29. S4「バルーン宛ての指令は 0」が、指令の 3 種類を見ていない
- 場所: `crates/areka/src/runtime_tests/spine_display_tests.rs:L8-L15`・`L428-L438`・`spine_test_support.rs:L20-L27`（`variant_name`）
- 問題: `present_command_target` が見るのは `ShowSurface`・`Hide`・`InvalidateCache` だけ。`ReplaceTarget`・`StageAck`・`MoveShift`（どれも `target` を持つ・`areka-emo-present/src/command.rs:L78-L110`）は `None` になって素通りする。たとえばバルーンへ `MoveShift` が漏れても緑。`variant_name` も同じ 3 種類を `<unknown>` にする。
- 直し方: 2 か所の match に、その 3 つの腕を足す。
- 規模: S

#### 30. 「起動の系列 5 呼び出し」の数が古く、待ちが basewareversion の前で抜ける
- 場所: `crates/areka/src/runtime_tests/spine_close_wiring_tests.rs:L88-L101`・`spine_talk_close_tests.rs:L310-L328`（同じ `>= 5` は `spine_conformance_lap_tests.rs:L580`・`spine_conformance_support_stage_tests.rs:L100`・`spine_conformance_support_tests.rs:L494` にもある）
- 問題: `spine_boot_smoke_tests.rs:L55-L66` が逐語で固定しているとおり、Status でない起動の系列は 6 呼び出し（OnInitialize・username・OnFirstBoot・OnBoot・OnTranslate・basewareversion）。`>= 5` は OnTranslate で抜ける。close_wiring の説明（L88-L89）は「boot の途中で差し込まない」ための待ちだと書いているが、その目的は果たせていない（kanade が起動中の close を保留するので、今は緑で済んでいる）。S5 の失敗の文言（L327）が挙げる 5 つの名前も、実物と違う。
- 直し方: `>= 6` にして、説明と文言を 6 呼び出しに直す。または待ちと説明を消す。
- 調べが要る（Research Needed）: 一周のテストの段の台帳は 5 番目を basewareversion としている。2 つの台帳で数がずれていないかを、設計の段で確かめる。
- 規模: S

#### 31. 3 人目のスコープのテストの「30 秒の期限」の説明は事実と違う〔件 2 の周り〕
- 場所: `crates/areka/src/runtime_tests/spine_third_scope_talk_tests.rs:L16-L18`・`L49-L51`・`L180`
- 問題: 足場は `TickerMode::Disabled` で、kanade へ `Tick` を送るのは `areka-ghost/src/ticker.rs:L239` だけ。だから終了の挨拶の期限は最後まで決まらない（`areka-kanade/src/schedule/close.rs:L168-L174` の `None`）。「期限で畳まれれば `DeadlineExceeded` で落ちる」（L180）は起こりえず、実際は `［止まった］` で落ちる。注入時刻の頭打ちが効いているのは、dispatcher の起点の件（L116-L117）だけ。
- 直し方: 3 か所の説明を事実に合わせる（期限の話を消し、起点の話にする）。2 番と同じタスクで直せる。
- 規模: S

#### 32. まばたきのスモークは、kero の 1 周の golden テストに丸ごと含まれる重なり
- 場所: `crates/areka/src/runtime_tests/spine_seriko_loop_tests.rs:L17-L116`
- 問題: スモークは `boot_live(\s[2100]\e, 常時発火, seeded(0))` で表示を待ち、Tick を入れて「pattern を載せた ShowSurface が出る」を見る。`spine_e2e_kero_blink_one_cycle_golden`（L264-L307）は同じ足場・同じ台本・同じ乱数で、指令のコマが 2106 であることまで確かめる。スモークが落ちる壊れ方なら、golden も必ず落ちる。
- 直し方: スモークを消す（`always_fire_rng` は残す・約 -100 行）。GPU を使う重い spine の起動が 1 回減る。
- 規模: S

#### 33. shell_carry の純粋な 6 本は、本体の部品を何も使っていない〔件 6〕
- 場所: `crates/areka/src/runtime_tests/shell_carry_tests.rs:L60-L182`
- 問題: `decide_binds_*` の 6 本が使うのは `areka-runtime` の `decide_binds`・`areka_parsers`・`sample_ghost_kit` だけで、どれも `areka-runtime` から使える。本体に依存するのは残りの 2 本（`nothing_is_carried_without_a_ghost_reload`・`replay_shows_…`）だけ。
- 直し方: 6 本を `areka-runtime/src/shell_carry_tests.rs`（133 行）へ移す。補助の `names` は本体に残る `replay_shows_…` も使うので、分け方の決めが要る（3 節の件 6・議題 7）。
- 規模: S

#### 34. wire のテストは、呼ばれない起動の関数のためだけに本体に居る。道も `C:\` 直下に書いている
- 場所: `crates/areka/src/runtime_tests/wire_tests.rs:L14-L15`・`L32`
- 問題: 無い ghost_root を渡すと `wired=false` になり、ゴーストも seriko も起こさないことを確かめるテスト。`crate::session::runtime_boot::boot_runtime` を渡しているが、主張は「そこまで届かない」こと。型は fn ポインタ（`areka-runtime/src/lib.rs:L180`）なので、捕まえの無いクロージャで代えられる。道は `C:/areka-nonexistent-…` で、一時の根は `target\` の下の決まりから外れている。
- 直し方: `|_, _, _, _, _| unreachable!("壊れた根で起動まで行った")` を渡し、道は `TempPath::under_target(..)` の作らない子の道にする。そのうえで `areka-runtime` の `wire_tests.rs`（47 行）へ移す。
- 規模: S

#### 35. 手書きの「30 秒の総時間で切る」待ちが 10 か所ある〔裁定〕
- 場所: `spine_display_tests.rs:L28-L42`・`spine_move_cue_tests.rs:L115-L132`・`spine_seriko_loop_tests.rs:L57-L74`・`L85-L109`・`L190-L222`・`L459-L483`・`spine_close_wiring_tests.rs:L114-L127`・`L169-L177`・`spine_third_scope_talk_tests.rs:L245-L253`・`spine_talk_close_tests.rs:L260-L276`（どれも本体の `runtime_tests/`）
- 問題: Tick を入れて汲み、200µs 眠る、を `Instant::now() + SPIN_WAIT` まで繰り返す。同じ束の S2 Phase 1（`spine_talk_close_tests.rs:L135-L139`）が書くとおり、この「自前の締め切り」の形は、前に負荷の下で赤になった形。相手が進んでいても 30 秒で切る。同じ 8〜12 行が 10 回写されている。
- 直し方: `wait_until(what, Progress::Count(&受けた数), || { tick; drain; sleep; 条件 })`（`drive_talk` と同じ形）にそろえる。または足場に `drive_until(harness, |cmd| …)` を 1 本置く。
- 〔裁定〕⑴ の候補: 本体の `runtime_tests/spine.rs` の doc（R7.9）は「ハイブリッドは各呼び出し点の自前の輪」と書く。完了した spec の決めに触れるので、開発者の確かめが要る（議題 3）。
- 規模: M

#### `runtime-crate-split` が開けた公開（件 5）

#### 36. 開けた公開のうち 5 項目は、`areka-runtime` の外に使い手が無い〔件 5〕
- 場所:
  - `crates/areka-runtime/src/spine.rs:L66`（`record_status`）・`L77`（`snapshot_status_calls`）
  - `spine_wait.rs:L380`（`wait_thread_end`。`pub` だが出し直されておらず、使い手は同じファイルの L409・L416 だけ）
  - `spine_hold_support.rs:L51`（`HoldGate`。私有の欄の型にしか出てこない）
  - `shell_carry.rs` の `decide`（外の使い手は 0。中では `frame/attach.rs:L296` が使う）
- 問題: 外から使えるように見えて、使い手がいない。公開の面が要らず広い。
- 直し方: 5 つを `pub(crate)` か私有へ戻す（`decide` は `frame/attach.rs` が使うので `pub(crate)`）。数えた全体（142 項目）は 3 節の件 5。
- 〔約束〕(b) 公開の名前が減る。ただし外の読み手は 0 件。
- 規模: S

### 2.3 できれば（37〜102）

#### 本番のコードと examples（`areka-runtime`）

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 37 | 当たり判定のテスト 2 本が同じ確かめで、取り違えを捕まえない（`surface_point: (x, y)` に変えても両方緑） | `hit_region.rs:L151-L167` | 消す（-17 行） | — |
| 38 | 受け持ち先の検査と重なり、取り違えも見分けられないテスト（96/96 なので入れ替えても通る。192/144 のテスト L651-L668 が受け持つ） | `assets_tests.rs:L670-L716`（`areka-placement/src/source_tests.rs:L236-L240`・`L262-L264` と同じ） | 消す（-47 行・emo2 の焼き 1 回ぶん減る） | — |
| 39 | 同じ等式の確かめが 2 回ある | `assets_tests.rs:L234-L240`（L156-L161 と同じ） | 消す | — |
| 40 | テストの説明が古い（`mod.rs`→`lib.rs`・`placement_shared_test_support.rs`→`areka-placement/src/test_support.rs`・assets.rs はもう surfaces.txt を自分で読まない） | `assets_tests.rs:L246`・`L261`・`L769`・`L774-L779` | 文を直す | — |
| 41 | 写像の数が古い（「4 写像」だが 7 個ある）。テストの名前 `all_variants` は 4 種しか見ない | `adapter.rs:L26`・`L400-L402` | 「4」を消し、テストの名前を合わせる | — |
| 42 | assets の説明が古い（`frame.rs`→`frame/attach.rs`・「本タスクは起動時資産への保持のみ」は結線済み） | `assets.rs:L121`・`L151` | 文を直す | — |
| 43 | probe の説明が古い（ダブルクリックでの終わり方は外され、今はメニューの「終了」・本番は `_with_shell`・`frame.rs`→`frame/`） | `examples/collision-probe/pointer.rs:L28`・`setup.rs:L64`・`L74-L79`・`L117`・`collision-probe.rs:L71-L79`・`L192` | 文を直す | — |
| 44 | probe の自動終了の値を書き損じると、黙って自動終了が切れる（例 `3m`・この probe にはほかの終わり方が無い） | `examples/collision-probe/smoke.rs:L9-L16` | 読めないときに `warn!` を 1 行出す | — |
| 45 | 送れなかったことを黙って捨てる | `examples/collision-probe/setup.rs:L15`（`let _ = tx.send(..)`） | 失敗したら `error!` を出す | — |
| 46 | 使い手の無い `pub(crate)` と、それを前提にした説明（この crate にゴーストを起動するテストは 0 件） | `sample_test_support.rs:L6-L8`・`L25` | `acquire_emo2` を私有にし、説明の 1 つ目の箇条を消す | — |
| 47 | Cargo の注釈が事実と違う（`tracing-subscriber` を使うのは example の 1 か所だけ） | `Cargo.toml:L61` | 注釈を「examples の購読者」にする | — |
| 48 | 腕が 1 つだけの誤りの型を本番が読まない（`wire_runtime` が `let-else` で捨てる。読むのは本体の `runtime_boot_tests.rs:L221` だけ） | `lib.rs:L170-L176`・`L180-L186`・`L653` | `BootRuntimeFn` の戻りを `Option<GhostRuntime>` か `Result<_, GhostBootError>` にする | 〔約束〕(b)（本体の `session/runtime_boot.rs:L38,L115,L148` と同 tests:L221）・M |
| 49 | 公開の関数の型が crate の中だけの別名（`pub(crate) type BootRuntimeFn` が `pub fn wire_runtime` の引数に出る） | `lib.rs:L180`（説明 `L170`・`L442`） | `pub type` にする（1 語） | — |

#### spine の足場と時計（`areka-runtime`）

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 50 | 「本体が 1,000 行に近い」はもう違う（spine.rs は 364 行） | `spine_hold_support.rs:L19` | 1 行消す | — |
| 51 | 行番号の参照が古い（`dispatcher.rs:290-294` は今 335-344 ほか） | `talk_lifecycle_tests.rs:L217`・`L404`・`L531` | 行番号を外し、関数名・定数名だけにする | — |
| 52 | `HOLDER_ROWS` の説明「檻が 2 つまで取る」は違う（2 つずつ取る檻が 2 本で、同時に 4 つ。16 行に収まる） | `crash_report_test_support.rs:L173` | 文を直す | — |
| 53 | `HOLDER_ROWS`・`NO_ROW` が `pub` だが、使い手はこのファイルの中だけ | `crash_report_test_support.rs:L174`・`L198` | `pub` を外す | — |
| 54 | `Status` の記録は書くだけで誰も読まない（読む口は毎回 `Status` を除く。kanade の手空きのたびに 1 件ずつ増える）。欄 `status` もいつも `Running` | `spine.rs:L178`・`L200`・`L282-L288`・`L313`・`L324` | `status()` は `HelperStatus::Running` を返すだけにし、push・除く filter・欄を消す。`RecordedCall::Status` の枝は残す | 枝まで消すなら〔約束〕(b)（テスト 5 ファイルの match） |
| 55 | 子のテストのためだけに、親が `#[cfg(test)] use std::time::{Duration, Instant}` を持つ | `spine.rs:L35-L37`・`spine_wait_tests.rs:L9-L12` | 子が自分で use し、親の 3 行を消す | — |
| 56 | 許可を 2 つ取る 3 行が確かめに効いていない（`wait_failure` は `held=2` を引数でもらう） | `spine_wait_tests.rs:L461-L463` | 3 行を消す | — |

#### 台帳・シェルの資産（`areka-runtime`）

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 57 | 正準台帳の 43 行が、ほぼ同じ `try_register(...).expect(...)` の 43 回。同じ一覧を説明に手で 3 回写している（行を足すたびに 4 か所を直す） | `consumer_ledger.rs:L407-L551`（説明 `L41-L50`・`L379-L398`・テストの文 `consumer_ledger_tests.rs:L299-L307`） | `[(名前, 選別子, 担当); 43]` の表 1 つと `for` にし、説明の一覧は消す（約 -120 行） | 響く読み手: `crates/ukadoc-survey/src/evidence/candidates.rs:L177-L190` が `register("…")` の字面で拾う当て推量の出力が減る（常時テストでは守らない数・同 L32）。〔約束〕には当たらない |
| 58 | `entry_count()==43` と長い一覧の文は、`samples_cover_exactly_the_ledger_rows` の集合の比べより弱い重ね | `consumer_ledger_tests.rs:L292-L308`・`consumer_ledger.rs:L373-L377` | 件数の assert と `entry_count`（テスト専用）を消す | — |
| 59 | `pub const ABSOLUTE` は同じファイルの `samples()` でしか使わない | `consumer_ledger_test_support.rs:L12` | `const` にする | — |
| 60 | `pub sized` は `areka-runtime` の中（テスト）でしか触らない | `shell_cursor_assets.rs:L31` | `pub(crate)` にする | — |
| 61 | `PathReason` は形が `OutsideFolder` の 1 つだけの enum（`Option` で足りる） | `shell_cursor_table.rs:L98-L101`・`L289` | `Option<Vec<PathBuf>>` にし、呼び手 1 か所（`shell_cursor_assets.rs:L145`）とテスト 1 本を直す | — |
| 62 | `replay` は、seriko の送り手があるかを確かめる前に「出し直す」の info を出す。同じ event 名 `reload_shell_carry_unavailable` を、別々の失敗 3 つ（L230・L260・L323）に使う | `shell_carry.rs:L271-L332` | 送り手の確かめを info の前へ移す。送りの失敗だけ別の event 名（例 `reload_shell_replay_unavailable`）にする | 〔約束〕(c)（読み手は `crates/areka/src/session/ghost_switch_reload_tests.rs:L525` の件数 1 本だけ・期待の 1 は変わらない。実機の確かめと `tools/perf` は読んでいない＝grep で確かめた） |

#### 台本の合図の受け口（`areka-runtime` の cue・`placement_tests/`）

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 63 | 古い行番号の参照（`move_cue.rs:489-505`。今の move_cue.rs は 373 行で、宛名の規律は L231-L245） | `zorder_cue.rs:L27` | 「`MoveCueSink::emit` の宛名の規律」と名前で指す | — |
| 64 | 「今この 2 つの名前（set/reset）を担当しているのは本 sink だけ」は誤り（名簿では choicetimeout・sticky-window・alignment*・windowstate も持つ） | `zorder_cue_tests.rs:L308-L309` | 「開けない形の set/reset を警告する持ち主が本 sink だけ」に直す | — |
| 65 | 「本体の片割れ」などの「本体」が、今は `areka-runtime` を指している（このリポジトリで「本体」は `crates/areka`） | `placement_tests/spawn_zorder_pair_deferred_tests.rs:L1`・`L12`・`L29-L32`・`L39`・`L52`・`L108` | 「areka-runtime」に言い換える | — |
| 66 | テストの道具の写し（`code_only` は `source_text_test_support::code_only` の写し、`count_level` は `frame_test_support::count_level` の写しが 3 か所、`index_of` が 2 か所） | `window_state_cue_tests.rs:L56-L72`・`zorder_cue_tests.rs:L100-L120`・`user_break_cue_tests.rs:L49-L52`・`zorder_wiring_tests.rs:L105-L109` | 既にある支えを `use` して消す（約 -30 行） | 範囲の外の写しは 6 節 |
| 67 | 同じことを 2 度確かめるテスト 2 本（`script_and_mcp_descript_agree`・`clone_without_begin_talk_is_ghost`） | `reload_args_tests.rs:L124-L137`・`user_break_cue_origin_tests.rs:L125-L135` | 2 本を消す（約 -25 行） | — |
| 68 | `parse_window_state`・`ParsedWindowState` が `pub` だが、外の使い手は 0 | `window_state_cue.rs:L46`・`L67` | `pub` を外す | 〔約束〕(b) は名目だけ（読み手 0） |
| 69 | `pub(crate) mod transition_judge_offset` を使うのは `placement_tests` の中だけ | `placement_tests/mod.rs:L8` | `mod` にする（宣言だけ。`transition_*` の中身には触らない） | — |
| 70 | 送れたときにも毎回 `format!("{:?}", reload.target)` を作る（使うのは失敗の warn だけ） | `reload_cue.rs:L77-L83` | `if let Err(SendError(reload)) = self.tx.send(reload)` で、失敗のときだけ書く | — |
| 71 | `DescriptNotes.empty` は「`DescriptParts` が全部 false」の写しで、正本が 2 つある（`Descript(全部 false)` と `notes: default()` を組めば「語が無い」の断りをすり抜ける・今の経路では起きない） | `reload_args.rs:L108-L109`・`L141`（読み手 `crates/areka/src/session/reload.rs:L267`） | 欄を消し、読み手で `ReloadTarget::Descript(p)` から判る | 〔約束〕(b)（本体の `reload.rs:L267`・`reload_tests.rs:L312`・`L357` ほか） |
| 72 | 表の欄 `note`・`source`・`enforced` を本番のコードが読まない（lib のビルドで「field is never read」が出るはず・ビルドで確かめていない） | `script_restriction.rs:L39-L43` | `#[cfg_attr(not(test), expect(dead_code, reason = "表の照合テストの欄"))]` を付ける | 調べが要る |
| 73 | テストの後片付けが、アクターのスレッドの panic を捨てる（`let _ =`）。L392 は失敗の中身を落とす | `placement_tests/reset_balloon_pos_tests.rs:L145`・`L392` | `.join().expect(..)`・`unwrap_or_else(\|e\| panic!("…: {e:?}"))` にする | — |

#### 本体の `session/`

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 74 | 説明の受け口の本数が古い（「12 本」だが今は 13 本） | `script_security_e2e_tests.rs:L4` | 「13 本」に直す | — |
| 75 | 起動の通しのテスト 3 本が、ほぼ同じ組み立て（`RuntimeInputs` の 15 行・降ろしの塊・Tick の輪）を写している | `runtime_boot_tests.rs:L233-L245`・`L252-L259`・`script_security_e2e_tests.rs:L88-L130`・`readme_script_e2e_tests.rs:L53-L106` | 親の `runtime_boot_tests.rs` に `emo2_inputs`・`shut_down` を置き、子の 2 本はそれを呼ぶ（約 -40 行） | — |
| 76 | 読み直しのテストの組み立てが、2 本でほぼ同じ（`BootContext` の 25 行・よその切替の予約を置く関数） | `reload_tests.rs:L48-L70`・`L127-L143`・`reload_skin_tests.rs:L88-L110`・`L450-L466` | 片方に置き、もう片方から呼ぶ（約 -35 行） | — |
| 77 | ファイルの中でしか使わない物の公開が広すぎる | `hooks.rs:L23 HOOKS`・`reload.rs:L118 SkinStep`・`L145 drain`・`L719 discard_for_exit`・`runtime_boot.rs:L41 RuntimeLines`・`L48`・`L59 runtime_sinks` | `pub(crate)`・`pub(super)` を外す（子のテストは非公開でも見える） | — |
| 78 | kanade が落ちた後の受け付けの失敗に、それを固定するテストが無い（印 `ReloadInFlight` が残ると、その後の読み直しが全部 `reload_in_flight` で断られる） | `reload.rs:L383-L386`・`L405-L407`・`L572-L573` | 送る相手の受信端を落としてから shiori・ghost・shell を 1 回ずつ要求し、「`reload_failed` が 1 件・印が残らない」を 1 本で確かめる | — |
| 79 | 記録の文と説明が、場合を取り違えている（`TalkGap::Left` の理由には `NotSteady`・`Closing` もあるのに「利用者の取りやめ・ほかの切替」と言い切る） | `reload.rs:L171`・`L215` | 文を「台詞の切れ目に達しなかった（理由は left 欄）」にする（`event` と `reason` の欄は変えない） | 〔約束〕(c) は名目だけ（文面だけで欄は同じ。字面で読む所 0） |

#### 一周の適合のテスト（本体の `runtime_tests/`）

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 80 | 注入時刻の上限 180,000 を手で写している | `spine_conformance_script.rs:L166` | `LAP_STAGES[LAP_STAGES.len() - 1].limit_ms` から const で導く | — |
| 81 | `OnSecondChange` の応答を 189 本×2 積んでいるが、`KanadeTick` は `once` にしか置けず一周で 2 本 | `spine_conformance_script.rs:L172-L198` | 本数を「`once` の KanadeTick の数＋余裕」にする | 〔裁定〕⑴（本数の規則は完了した spec の task 2.2 の決め） |
| 82 | 判定の側の採取時刻の自己検査は、駆動の側の `CollectedOutsideInterval` と同じ検査で、doc 自身が「必ず真」と言う。中の Vec は `is_empty` にしか使わない | `spine_conformance_judge.rs:L165-L186` | 残すなら Vec をやめて `any` 1 回にする | 〔裁定〕⑴（design D3 の決め） |
| 83 | `runtime` の欄の `#[allow(dead_code)]` は要らない（一周のテスト L763 と `spine_talk_close_tests.rs:L25` が読む） | `runtime_tests/spine.rs:L296` | 許可を消す（コンパイルで確かめる） | — |
| 84 | 何もしない文（`let _ = shiori_handle;`・`let _ = reached;`・`\|\| String::new()`） | `runtime_tests/spine.rs:L611`・`spine_conformance_lap_tests.rs:L456`・`L379` | 前の 2 つは消す。3 つ目は `String::new` | — |
| 85 | 2 本目の檻だけ、再生の側の Tick の `Err` を捨てていて、「dispatcher が閉じた」の手がかりが出ない | `spine_conformance_support_stage_tests.rs:L290` | 1 本目（L163-L171）と同じく印を立て、assert の文に出す | — |
| 86 | 窓の entity が 1 つ欲しいだけなのに `GhostWindows` を丸ごと clone する | `spine_test_support.rs:L45-L49`・`L62-L66` | `resource::<GhostWindows>().balloon_window(scope)` で Entity だけ取る | — |
| 87 | `#[cfg(test)]` の木の中で、子の mod にも `#[cfg(test)]` を重ねている（`main.rs:L52` が既に `#[cfg(test)]`） | `runtime_tests/mod.rs:L41-L57`・`spine.rs:L621-L659`・`spine_conformance_lap_tests.rs:L911`・`spine_conformance_support.rs:L755-L760` | 重ねた印を消す | — |

#### 本体の `runtime_tests/` の残り

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 88 | 消した import の説明だけが残っている | `runtime_tests/assets_tests.rs:L50-L52` | 3 行を消す | — |
| 89 | 「`assets_tests.rs` は上限に近い」は、本体の側では違う（86 行）。どちらも 1 本ずつで検体も同じ | `runtime_tests/assets_shell_tests.rs:L4`・`assets.rs:L6-L8` | 1 ファイルにまとめ、代わりの親の宣言を 1 つ消す | — |
| 90 | 「二重に当てていない」は検出できない（move は絶対位置なので、2 回当てても同じ値） | `spine_move_cue_tests.rs:L146-L152` | 消す。または move の受け口が空なことを数で見る | — |
| 91 | S3 の「ちょうど 2 件」は、2 件で汲むのをやめるので 3 件目を見ない | `spine_display_tests.rs:L275-L282` | 集めた後に `settle_bounded` で残りを汲む | — |
| 92 | `seriko_tick_expect_one` の「ちょうど 1 指令」も、最初の空でない汲みで止まる | `spine_seriko_loop_tests.rs:L230-L253` | 同上（短い settle） | — |
| 93 | S2 Phase 1 は `now` を 1 反復で 2 回進める（説明は 5ms だが実際は 10ms）。Phase 2 は締め切りの確かめが 2 回ある。L261 の「Phase 1 と同じく時刻期限」は誤り | `spine_talk_close_tests.rs:L150-L153`・`L170-L173`・`L261-L274` | 進めるのを 1 か所にし、重なった if と説明を直す | — |
| 94 | 「有界スピン（sleep なし・yield_now のみ）」は古い（`wait_until` は 60ms を過ぎると眠る） | `spine_boot_smoke_tests.rs:L29-L30` | 文を直す | — |
| 95 | 時計を差した settle の檻でも、反復ごとに本物の 1ms を眠る（3 本で約 280 回） | `spine_settle_tests.rs:L29-L41`（本体は `runtime_tests/spine.rs:L142-L160`） | `settle_bounded_with` に `pause` の引数を足す。この関数は本体の部品を使わないので、`spine_wait.rs` へ移せば 4 本とも `areka-runtime` へ移せる | — |
| 96 | プロセスの寿命の MTA が据わっているので、テストの `CoInitializeEx` は要らない | `runtime_tests/assets_shell_tests.rs:L16-L19`・`assets_tests.rs:L62-L65`・`film_playback_e2e_tests.rs:L59-L65`（据える所は `mod.rs:L16-L25`） | 3 か所を消す | — |
| 97 | `carrier_cue` の写しがワークスペースに 17 個ある（範囲の中に 2 個） | `switch_cue_tests.rs:L18-L25`・`consumer_ledger_agreement_tests.rs:L69-L76` | test-support に 1 本置いて使う（範囲の中の 2 個から） | 範囲の外の写しは 6 節 |
| 98 | `pump_until_idle` の 2 行の写し（ほかに `spine.rs:L231`・`areka-runtime` の `strict_judges_tests.rs:L128` など 5 個） | `runtime_tests/strict_judges_tests.rs:L149-L154` | 本体の中では spine.rs のものを `pub(super)` で使う | — |
| 99 | 装着の前提の 3 行（`capture_logs(run_attach_phase)`・`planned=2 attached=2`・ERROR 0）が spine に 11 回ある | display・boot_smoke・seriko_loop・talk_close・text_scale・third_scope の各テスト | 足場に `attach_all()` を 1 本置く | — |
| 100 | 「結線の窓はえみりの形と同じ」は、足場が直書きした [0,1] と比べているだけ（足場はえみりの形を使わない） | `spine_third_scope_talk_tests.rs:L5-L8`・`L192-L196` | assert を消すか、doc に「足場は emo2 の 2 scope」と書く | — |
| 101 | t_zwi05 の対照の 3 行は、`areka-runtime` の `frame_schedule_tests.rs:L114-L124` と逐語で同じ | `runtime_tests/zorder_wiring_tests.rs:L71-L83` | 本体の側を消す | — |
| 102 | film の E2E 3 本が使う本体の項目は `crate_path`（`CARGO_MANIFEST_DIR` を join するだけ）だけ | `film_playback_e2e_tests.rs:L51-L57` | `areka-runtime` の側に同じ読み口を置けば移せる（「読み口は 1 つ」の決まりとの兼ね合いは設計で） | — |

### 判定と減らせる量

- 判定: 本番の振る舞いで最初に直すのは 1 番（台本 1 行で落ちる）。テストで最初に直すのは 2 番（揺れ）と 3 番（降ろしの失敗が緑で通る）。範囲の残りの本番のコードは、どれも記録つきの失敗の道を持っていて健全。指摘の多くは、`runtime-crate-split` でテストを 2 つの crate に割った後の、説明の古さ・写し・置き場の不自然さ。
- 減らせる量: 約 -700 行（束ごとに A 約 110・B 約 30・C 約 150・D 約 140・E 約 80・F 約 170・G 約 90）。依存は -0（足す依存も 0）。

---

## 3. 申し送りの件（要件 5・1.7）

今の `main`（`5749a4c7`）で確かめ直した結果。

| 件 | 出どころ | 場所（今） | 今も当てはまるか | 指摘 |
|---|---|---|---|---|
| 1 | `audit-sylphya-actor` 議題 3（audit #16） | `crates/areka-actor/src/ui.rs:L84-L87`（`spawn_ui` の戻り値）・`L174-L176`（`UiSpawnError`）・`lib.rs:L99`（出し直し）・`crates/areka-runtime/src/lib.rs:L254` | 当てはまる。`spawn_ui` はいつも `Ok` | 4 番 |
| 2 | `runtime-module-untangle` の申し送り | `crates/areka/src/runtime_tests/spine_third_scope_talk_tests.rs`（関数は L182 から） | 当てはまる。根の形が分かった | 2 番（31 番も同じタスクで） |
| 3 | `runtime-crate-split` ⑵ | `crates/areka-runtime/src/assets.rs:L215` | 当てはまる。許可は効いていない。理由の文は今も事実 | 6 番 |
| 4 | `runtime-crate-split`（`hit_region.rs` の説明） | `crates/areka-runtime/src/hit_region.rs:L45-L57` | 当てはまる。`#[path]` の取り込み 0・`is_benign_boot_error` の呼び出し 0 | 5 番 |
| 5 | `runtime-crate-split` ⑶ 2.5・3.2・4.2 | 下の表 | 外に使い手の無いものが 5 項目。死んだ `pub(crate)` は 0 | 36 番（ほかに 46・53・59・60・68・69・77 が同じ型の狭め） |
| 6 | `runtime-crate-split` ⑶ 4.6 | 下の表 | 17 本とも新しい依存なしで戻せる。3 本はほかの spec の持ち物 | 26・27・33 番 |
| 7 | `runtime-crate-split` ⑵ | `crates/areka-runtime/src/placement_tests/transition_judge.rs:L47-L50`（今も同じ行） | `audit-placement-follow-residue/brief.md:L109` に書かれている（「47〜50 行の「areka は lib ターゲットを持たない bin crate ゆえ…」」）。申し送りは要らない | 指摘にしない |
| 8 | `runtime-crate-split` ⑶ | `crates/areka/src/runtime_tests/balloon_visibility_phase_tests.rs:L146-L150`（支えの側は `crates/areka-runtime/src/balloon_visibility_phase_test_support.rs:L41`） | `audit-emo2-boot-switch/brief.md:L98` に書かれている（「`world_at`（146 行）は…前からの写し」）。両方のファイルとも向こうの持ち物 | 指摘にしない |

### 件 1 の呼び手（23 ファイル）

`spawn_ui(`・`spawn_emo_text(`・`UiSpawnError` を grep した結果（説明の文だけの当たりは除いた）。要件 5.1 が挙げるのは `areka-runtime` の `lib.rs`・`areka-emo-text` の `actor.rs`・`actor_test_support.rs`・`areka-actor` のテストだけ。ほかにも直す所がある。

| crate | ファイル |
|---|---|
| `areka-actor` | `src/ui.rs`・`src/lib.rs`・`tests/toy_ui_pump_test.rs`・`tests/ui_span_test.rs` |
| `areka-emo-text` | `src/actor.rs`・`src/actor_test_support.rs`・`src/sink.rs`（テストの部）・`src/sink_probe_tests.rs`・`src/sink_talk_end_tests.rs`・`src/actor_tests.rs`・`src/actor_lookahead_tests.rs`・`src/actor_runtime_frame_tests.rs`・`tests/attach_wiring_test.rs`・`tests/draw_readback_test.rs`・`tests/pipeline_test.rs`・`tests/viewbox_scroll_test.rs`・`examples/emo-text-layer/drive.rs` |
| `areka-runtime` | `src/lib.rs`・`src/strict_judges_tests.rs` |
| `areka`（本体） | `src/runtime_tests/spine.rs`・`src/runtime_tests/strict_judges_tests.rs`・`src/session/runtime_boot_tests.rs`（ここまで範囲の中）・`src/input_events/balloon_choice_settle_tests.rs`（範囲の外＝`audit-areka-input-menu` の持ち物・C22 では走っていない） |

- C22 の約束との突き合わせ: どれも `frame/`・`frame*.rs`・`areka-placement` ではない（`areka-placement/src/follow.rs:L30` の当たりは説明の文だけ）。emo-text は直すだけで、新しいソースファイルは足さない。C22 のほかの 5 本は、`areka-actor`・`areka-emo-text` に触らない（`collisionex-regions` が触るのは emo-compose・emo-present・parsers）。
- 説明の文で `spawn_ui` の `Err` に触れる所（`areka-emo-text/src/sink.rs:L180` ほか）も、同じ変更の中で grep して直す。

### 件 5 の数（142 項目）

| 出どころ | 数 | (a) 本体の本番が使う | (b) 本体のテストだけが使う | (c) `areka-runtime` の中だけ | (c*) 公開の項目が名指すので要る | (d) 使い手 0 |
|---|---:|---:|---:|---:|---:|---:|
| 2.5（`pub(crate)`→`pub`） | 76 | 73 | 2 | 0 | 1 | 0 |
| 3.2（spine の 3 ファイル） | 39 | 0 | 34 | 4 | 1 | 0 |
| 4.2 ⑴（この spec の分） | 27 | 0 | 26 | 1 | 0 | 0 |
| 計 | 142 | 73 | 62 | **5** | 2 | 0 |

- (c) の 5 項目が 36 番の指摘。
  - `record_status`・`snapshot_status_calls`（`spine.rs:L66`・`L77`）
  - `wait_thread_end`（`spine_wait.rs:L380`）
  - `HoldGate`（`spine_hold_support.rs:L51`）
  - `shell_carry::decide`
- (b) の 62 項目は、要件 5.5 の仮置き（「使い手が本体のテストだけでも使い手に数える」）では戻さない。ただし外の使い手が、ほかの spec の持ち物のファイルだけにあるものが多い。
  - `NO_BODY`・`ShellCursorAssets::hand_to`・`SwapPayload`・`SwapSlot` は `runtime_tests/frame/switch*`。
  - `snapshot` は `frame_attach_carry_tests.rs`。
  - `LastInstalledBalloon` は `install/*_tests`。
  - `ScopeScaleTable::set`・`percent_text` は `ghost_session_scope_scale*_tests`。
  - `CursorFile` は `input_events/cursor_answer_tests`。
  - `build_*_with_limits` は本体の `placement_tests`。
  - spine の (b) の多くは、ほかの spec のテストの支え（`session/ghost_switch_test_support`・`session_end_*`・`update/worker_path_tests` ほか）が使う。
- (c*) の 2 項目は、公開の項目が名指しているので戻せない。
  - `TalkTimeout`: `TalkLifecycleSignal::BalloonTimeout` が名指す。
  - `StatusLedger`: `test_support::status_calls` が名指す。
- 本番の 1 項目 `ShellBoxAssets` は、公開の欄 `assets.rs:L168,L301` の `boxes` が名指すので、(b) でも戻せない。
- 調べ方: `git grep -P` で `crates/` の全体を調べた（説明の行は除いた）。`get`・`build` のような一般の名前のメソッドは、当たった所を目で確かめて分けた。型の推論までは追っていない。

### 件 6 の 17 本

`crates/areka-runtime/Cargo.toml` の `[dev-dependencies]` には、番人の 3 つ（`log-capture-kit` L44・`sample-ghost-kit` L48・`temp-path-kit` L52）が前から入っている。**17 本とも、新しい依存なしで戻せる**。本体に残した理由は「補助を 2 つの crate で使うと写しになる（設計 D4）」だった。

| 本数 | 関数（今の場所） | 持ち物 | 判定 |
|---:|---|---|---|
| 5 | `switch_cue_tests.rs` の `missing_name_warns_and_sends_nothing`（L107）ほか 4 本 | この spec | ファイルごと（10 本と補助）戻せる＝27 番 |
| 3 | `shell_balloon_resolve_tests.rs` の `hidden_shell_by_name_and_the_current_one_are_candidates`（L43）・`balloon_lastinstalled_resolves_only_when_in_the_catalog`（L151）・`balloon_candidates_put_the_ghost_inner_first`（L181） | この spec | 26 番で空振りの assert を消せば、4 本目も本体を引かなくなり、補助 `plant_ghost` ごと全部戻せる |
| 6 | `shell_carry_tests.rs` の `decide_binds_*` 6 本（L62・L70・L78・L127・L143・L168） | この spec | 戻せる。ただし補助 `names` を、本体に残る `replay_shows_…`（L192）も使う＝分け方の決めが要る（33 番・議題 7） |
| 2 | `balloon_visibility_phase_tests.rs` の `decision_events_are_written_with_the_agreed_level_and_fields`（L714）・`transition_of_a_show_that_did_not_take_is_not_written`（L789） | `audit-emo2-boot-switch` | 補助ごと戻せる（同じ中身の `visibility_lines` が `areka-runtime/src/balloon_visibility_phase_log_tests.rs:L17` にある）。向こうの持ち物なので、この spec では触らず申し送る |
| 1 | `runtime_tests/frame/window_state_tests.rs` の `log_ghost_falls_back_to_dash_and_raise_reports_normal_band`（L144） | `audit-emo2-boot-frame`〔C22〕 | 依存なしで戻せるが、補助 `window_state_events` が残る 4 本と分かれる。`verification.md` の 4.6 の表の書き方があいまいで、この 1 本を「12 本目」と見たのは推定。向こうの持ち物なので申し送る |

---

## 4. 外への約束に響く指摘（要件 1.6）

| 指摘 | 約束 | 響く読み手 | 仮置き（問い 7）での扱い |
|---|---|---|---|
| 1 | (a) は変わらない見込み（大きすぎる scope の指令を warn して捨てる）。ログに warn が 1 行増える | 字面で読む所は 0 | 直す。上限の値は議題 8 |
| 4 | (b) `areka_actor::spawn_ui`・`areka_emo_text::actor::spawn_emo_text` の戻り値・`BootWiringError::SpawnUi` が消える | 3 節の件 1 の 23 ファイル | 同じ変更の中で全部直す |
| 36 | (b) 公開の名前が 5 つ減る | 外の読み手 0 | 戻す |
| 48 | (b) `BootRuntimeFn` の戻り値の形が変わる | 本体の `session/runtime_boot.rs:L38,L115,L148`・`runtime_boot_tests.rs:L221` | 同じ変更の中で全部直す |
| 54 | (b)（`RecordedCall::Status` の枝まで消すときだけ） | テスト 5 ファイルの match | 枝は残す形なら響かない |
| 62 | (c) event 名を 1 つ足す（送りの失敗だけを分ける） | 本体の `ghost_switch_reload_tests.rs:L525`（期待の数は変わらない） | 読み手を同じ変更で直す |
| 68 | (b) 名目だけ（外の読み手 0） | — | 戻す |
| 71 | (b) `DescriptNotes` の欄が 1 つ消える | 本体の `session/reload.rs:L267`・`reload_tests.rs:L312`・`L357` ほか | 同じ変更の中で全部直す |

- ゴーストと SSP 互換の振る舞い（(a)）が変わる指摘は 0 件。
  - 束 D と束 G は、cue の名前と読み直しの順を ukadoc と照らした。`\![reload,*]`・`\![reloadsurface]`・`\![change,shell|balloon]`・`\![enter|leave,nouserbreakmode]`・`\![set,windowstate,…]`・`OnDestroy` の Reference0 の `reload`・`\![reload,ghost]` が SHIORI のイベントを出さないこと・`OnShellChanged` の Reference1。どれも ukadoc か、開発者の裁定と一致した。
  - ukadoc の字面と違う所（`\v` を SSP 2.9.26 で測った働きに合わせる・外部扱いの `\![enter,nouserbreakmode]` を止める・読み直しで `OnShellChanged` を出さない）は、どれも完了した spec の裁定どおり。そのため指摘にしていない。

---

## 5. 直さずに残す候補（要件 2.2）

- **⑴ 開発者の裁定に反する直し**（候補 4 件）
  - 25: 製品に観測の口を作らない R2.9。
  - 35: ハイブリッドは各呼び出し点の自前の輪 R7.9。
  - 81: `OnSecondChange` の本数の規則（task 2.2）。
  - 82: 採取時刻の自己検査（design D3）。
  - どれも完了した spec の決めに触れる。直すか残すかは、要件の討議で開発者が決める（議題 3・4）。
- **⑵ わざと置いた先送りの印**（指摘にしていない 2 件）
  - `crates/areka-runtime/src/crash_report_test_support.rs:L436`: `GetThreadDescription` の文字列を `LocalFree` しない。限界は「落ちていくプロセスの数十バイト」で、走るのは落ちたときに 1 度だけ。
  - `crates/areka/src/runtime_tests/spine_conformance_support.rs:L446-L448`（`TAKEN_PRESENTS`）: 同じスレッドの足場どうしで数を共有する。限界は「足場が自分の数を要るようになったら欄へ移す」で、今の使い方は「増えること」しか見ない。
  - `#[allow(dead_code)]` に実在の理由の doc を付けた語彙の口は、範囲の中に 0 件。範囲の中の `#[allow(dead_code)]` は 3 つ（6・23・83 番）あるが、どれも効いていないか、理由が古い。
- **⑶ SSP 互換か ukadoc の正典に関わる振る舞いの変化**: 当たる指摘は 0 件（4 節）。1 番の上限の値だけ、念のため議題 8 に挙げる。

---

## 6. 範囲の外で見つけたこと（持ち主へ渡すもの）

| 件 | 場所 | 持ち主 | 中身 |
|---|---|---|---|
| ア | `crates/areka-runtime/src/frame/dpi.rs:L404-L413` | `audit-emo2-boot-frame`〔C22〕 | 窓の側から来る scope の `2*scope+1` のあふれ（1 番の片割れ） |
| イ | `crates/areka/src/runtime_tests/frame/window_state_tests.rs:L144` | `audit-emo2-boot-frame`〔C22〕 | 件 6 の 1 本を `areka-runtime` へ戻せる（補助の分け方つき） |
| ウ | `crates/areka/src/runtime_tests/balloon_visibility_phase_tests.rs:L714`・`L789` | `audit-emo2-boot-switch` | 件 6 の 2 本を補助ごと `areka-runtime` へ戻せる（`world_at` の写しは向こうの brief に既にある） |
| エ | `crates/wintf/src/test_crash_report.rs`・`crates/areka-placement/src/test_crash_report.rs`・範囲の中の `crash_report_test_support.rs` | wintf と placement の見直しは完了済み〔C22: placement〕 | 落ちの報告がほぼ同じ写しで 3 本ある（約 500 行ずつ）。違いは持ち手の表があるかだけ |
| オ | `code_only` の写し: `crates/areka/src/tick_gate_config_producers_tests.rs:L70`・`crates/areka-runtime/src/balloon_visibility_phase_zorder_chain_tests.rs:L257`・`crates/areka-runtime/src/frame/zorder_drain_tests.rs:L465` | 本体の見直し（`audit-areka-app` の見込み）・`audit-emo2-boot-switch`・`audit-emo2-boot-frame`〔C22〕 | `source_text_test_support::code_only` と 1 字も違わない |
| カ | `carrier_cue` の写し 15 個（範囲の外） | 各持ち主 | 97 番で test-support に 1 本置いた後に、各持ち主が寄せる |
| キ | `crates/areka/src/boot_config.rs:L617`・`L621-L622` | 本体の起動の設定（`audit-areka-app` の見込み） | `is_benign_boot_error` の説明が「`wire_runtime` が再利用する」と書くが、呼び手は `ghost_session.rs:L783`・`session/runtime_boot.rs:L137` だけ |
| ク | `crates/areka/examples/emo-present/setup.rs:L88` | 本体の examples の持ち主 | 「本番（`build_boot_assets`）」という、43 番と同じ古い文 |
| ケ | `crates/ukadoc-survey/src/evidence/candidates.rs:L26-L30`・`L42-L46` | `audit-ukadoc-survey`（C22 で並走中） | `consumer_ledger.rs` の `register(` の数の実測が古い。57 番を直すと、さらに数が変わる |

- 申し送りの書き方（brief に書き足すか、走っているセッションへ伝えるか）は、要件 7.1・7.7・7.8 に従う。ア・イ・ウ・ケは今のウェーブで決める（議題 9）。
- 2026-11-17 の要件の討議（議題 5・推奨案で確定）で済ませた: ア・イ・オの 1 つは `audit-emo2-boot-frame`、ウ・オの 1 つは `audit-emo2-boot-switch`、オの 1 つ・キ・クは `audit-areka-app` の brief へ書き足した。ケは並走中の `audit-ukadoc-survey` のセッションへ伝えた。エ・カは `/kiro-complete` の未解決の棚卸で扱う。

---

## 7. 並走の約束（要件 7）に触れる指摘

- 〔C22〕に当たるのは 1 番の片割れ（6 節のア・`frame/dpi.rs`）と、6 節のエ（`areka-placement` の写し）だけ。どちらも範囲の外の持ち主へ申し送る形にすれば、約束 ⑷ に触れない。
- 依存を足す指摘は 0 件（約束 ⑸・要件 6.6）。件 6 の戻しも、既にある dev-dependencies で足りる。
- emo-text に新しいソースファイルを足す指摘は 0 件。件 1 は、既にあるファイルを直すだけ。
- `log-capture-kit/tests/file_length_guard_test.rs` の表に載るファイルを割る指摘は 0 件（約束 ⑵）。範囲の中の 1,000 行に近い 6 本は、どの直しでも行が減るか、数行しか増えない。
- `placement_tests/mod.rs` で触るのは 69 番の宣言の印だけ。`transition_*` の中身には触らない（約束 ⑶）。
- C22 のほかの 5 本が触るファイルの一覧（ロードマップの C22 の行）と、この spec の直しが触るファイルとの重なりは 0。

---

## 8. 確かめた指摘

読み手の報告のうち、次の指摘はこの文書を書く前に根拠の行を読み直した。

- 1 番: `target_map.rs:L19-L38` と `decode.rs:L454-L459`。
- 2 番: `spine_third_scope_talk_tests.rs:L205-L232`。束 B と束 F が別々に同じ根の形を出した。
- 3 番: `runtime_tests/spine.rs:L590-L600`。
- 4 番: `spawn_ui`・`spawn_emo_text`・`UiSpawnError` の grep をし直し、呼び手が 23 ファイルあると分かった。
- 6 番: `assets.rs:L213-L217`。
- 16 番: `listed_name` が `areka-ghost/src/config.rs:L41` に実在する。
- 30 番: `>= 5` が 5 ファイルにある（`grep`）。

それ以外の指摘は、読み手が報告に書いた根拠の行による。走らせての確かめ（1 番の台本を実際に流す・2 番の門つきの赤・72 番の警告・83 番の許可）は実装の段で行う。

---

## 9. 要件と今の姿の突き合わせ

| 要件 | 要るもの | 今の姿 | 差（Missing / Unknown / Constraint） |
|---|---|---|---|
| 1 | 範囲の測り直しと全件の audit | この文書 | 無し |
| 2 | 直す対象の確定と、直さずに残すものの裁定 | 候補 4 件（5 節 ⑴） | Constraint: 開発者の裁定待ち |
| 3 | 102 件の直し・呼び手の直し・消す前の grep | 指摘ごとに場所と直し方がある | Missing: 直しそのもの。4 番は 23 ファイルに及ぶ |
| 4 | バグと危ない所ごとのテスト | 1・2・3・15・78 番に足すテストを書いた | Missing: 1 番の境のテスト・2 番の決定論の赤（門つきの出力）・15 番の warn の檻・78 番の印の檻 |
| 5 | 預かった 8 件 | 3 節 | Missing: 件 1〜6 の直し。件 7・8 は確かめ済み。件 6 の 3 本は申し送り |
| 6 | 決まり（1,000 行・記録の無い失敗・依存を足さない ほか） | 直しはどれも行を減らすか数行で、依存は足さない | Constraint: 22 番（165 か所）・57 番（表への書き換え）は行の数を確かめながら |
| 7 | 範囲の外への直しと C22 の約束 | 7 節 | Constraint: 4 番の本体の `input_events` の 1 本（持ち主は走っていない）・6 節の申し送り |
| 8 | 重い操作を回さない | 決定論のテストだけで確かめられる | 無し（SSP の測りは要らない見込み） |
| 9 | かけ直しと全体テスト | — | Missing: 実装の後 |

### 直し方の選び方

- **案 A（ファイルのまとまりごとに切る）**: 束（1.4 節の表）に沿ってタスクを切る。同じファイルを 2 つのタスクが触らないので、レビューと衝突が少ない。2・31・35 番のように同じファイルで組をまたぐものも 1 か所で直せる。重さの違う指摘が 1 つのタスクに混ざる。
- **案 B（組ごとに切る）**: 必ず直す → 直したほうがよい → できれば、の順に切る。大事なものが先に着地する。けれども同じファイルを何度も触るので、行番号のずれと衝突が増える。
- **案 C（混ぜる・推す）**: 必ず直す 3 件と、範囲の外に及ぶ 4 番（件 1）を、先に独立のタスクにする。残りを案 A のまとまりで切る。4 番は 4 つの crate に及ぶので、1 本のタスクに閉じたほうが、壊れる呼び手を一度に直し切れる。2 番は決定論の赤から始める（要件 4.2）。

### 手間と危うさ

- 手間: **L**（1〜2 週間）。102 件のほとんどは S の直しだが、数が多い。4 番は 4 つの crate・23 ファイル、22 番は 165 か所、件 6 は 3 つのファイルの移し替え。
- 危うさ: **中**。
  - 足場（`runtime_tests/spine.rs`・`areka-runtime/src/spine*.rs`）の直し（3・9・35・54・95・99 番）は、spine 系のテストの多くに響く。
  - 4 番は、公開の形を 4 つの crate で同時に変える。
  - 2 番の決定論の赤は、組み立てを作ってみるまで確かでない。
  - どれも既にある型の延長で、新しい技術は要らない。

### タスクの数の見立て（要件 2.6）

| まとまり | 指摘 | タスク |
|---|---|---:|
| 1 番（境のテスト・あふれ） | 1 | 1 |
| 揺れるテスト（決定論の赤→直し） | 2・31 | 1 |
| 足場の降ろし | 3 | 1 |
| 件 1（`spawn_ui` の戻り値・4 crate） | 4 | 1〜2 |
| lib.rs・assets・hit_region・wire | 5〜8・41・42・46〜49 | 1 |
| examples | 43〜45 | 1 |
| assets のテスト・当たり判定のテスト | 37〜40 | 1 |
| spine の足場（`areka-runtime`）と公開の戻し | 9・10・36・50〜56 | 1 |
| 台帳 | 13・57〜59 | 1 |
| シェルの資産と分けたテストの説明 | 11・12・60〜62 | 1 |
| cue と `placement_tests/` | 14・15・63〜73 | 1〜2 |
| `session/` | 16・74〜79 | 1 |
| 一周の適合のテスト | 17〜25・80〜87 | 2 |
| 本体の `runtime_tests/` の移し替え（件 6） | 26・27・33・34・95・102 | 1 |
| 本体の `runtime_tests/` の残り | 28〜30・32・35・88〜94・96〜101 | 2 |
| 申し送り（件 7・8 の記録・6 節） | — | 1 |
| かけ直しの audit（最大 5 回） | — | 1〜2 |
| 全体テストと検証の記録 | — | 1 |
| **計** | | **20〜24** |

- 20 を越えうるので、要件 2.6 の議題が要る（問い 1）。記憶の決めごと（2026-11-16）は「一度 spec に切り出した後は、規模を理由にした分割を後出しで提案しない」。仮置きは 1 本のまま。

### 設計へ持ち越す調べ（Research Needed）

- 2 番: 門つきの `SurfaceOutput` で、今のコードが毎回赤になるか（作って確かめる）。
- 30 番: 起動の系列の数（6 呼び出し）と、一周のテストの段の台帳の数（5 番目を basewareversion とする）が食い違っていないか。
- 72 番: lib のビルドで `field is never read` の警告が出るか（`cargo build -p areka-runtime`）。
- 83 番: `runtime` の欄の `#[allow(dead_code)]` を消しても警告が出ないか。
- 22 番: 165 か所の参照の全数（抜き取りは約 25 か所）。
- 1 番: 台本 `\p[2147483648]` が実際に seriko まで届くか（偽の SHIORI の台本で決定論のテストを立てる）。dispatcher の側にスコープの上限が無いことは grep でしか見ていない。

---

## 10. 要件の討議へ渡す議題（答えで作業が変わるもの）

1. **分け方（要件 2.6・問い 1）**: 見込みは 20〜24 タスク。1 本のままか、組ごとに 2 本目へ切るか。仮置きは 1 本のまま。
2. **件 1 の呼び手の範囲（要件 5.1）**: 要件 5.1 が挙げるのは 3 か所だが、実際は 23 ファイル（3 節）ある。うち 1 本（`crates/areka/src/input_events/balloon_choice_settle_tests.rs`）は `audit-areka-input-menu` の持ち物（C22 では走っていない）。要件 5.1 の文を「呼び手を全部」に広げてよいか。範囲の外のそのファイルを、申し合わせなしで直してよいか（問い 9 と同じ形）。
3. **完了した spec の決めに触れる待ちの形（25・35 番・要件 2.2 ⑴）**: 35 番は、10 か所の自前の 30 秒の輪を `wait_until` の形へそろえるか（R7.9「ハイブリッドは各呼び出し点の自前の輪」）。25 番は、余韻の 300 反復を残すか、製品に観測の口を作るか（R2.9「製品のコードへ口を作らない」）。推すのは、35 番はそろえる（R7.9 の後の load-flake の spec で `drive_talk` は既にこの形）、25 番は残す。
4. **一周の台本の決め（81・82 番・要件 2.2 ⑴）**: `OnSecondChange` の応答の本数の規則（task 2.2）と、採取時刻の自己検査（design D3）を直すか残すか。
5. **公開の面を変える直し（36・48・71 番・問い 4・7）**: 読み手を同じ変更で全部直す仮置きでよいか。件 5 の (b) 62 項目（本体のテストだけが使う）は戻さない仮置きでよいか。
6. **ログの event 名を足す直し（62 番・要件 7.6）**: 送りの失敗だけ `reload_shell_replay_unavailable` に分けるか。読み手は本体のテスト 1 本だけ。
7. **件 6 の分け方（要件 5.6・問い 5）**: 17 本とも依存は要らない。shell_carry の 6 本は、補助 `names` を本体に残る 1 本と分け合う。写しを作らない設計 D4 の扱い（補助を `areka-runtime` の test-support に置いて本体から引くか、残る 1 本の書き方を直すか）。ほかの spec の持ち物の 3 本（balloon_visibility 2・window_state 1）は、持ち主の brief へ申し送る形でよいか。
8. **1 番の上限の値**: `scope_of` の上限を `i32::MAX`（`2*scope+1` が u32 に収まる上限）に置く。大きすぎる `\p[n]` を warn して捨てるのは、今の「数でない key」と同じ扱いで、SSP の互換には関わらない見込み。測りは要らないと見るが、要るなら SSP の MCP で測る（要件 8.2 の許しが要る）。
9. **範囲の外への申し送り（6 節・要件 7.1・7.7・7.8）**: ア・イ（`audit-emo2-boot-frame`）・ウ（`audit-emo2-boot-switch`）・ケ（`audit-ukadoc-survey`・C22 で並走中）を、各 brief に書き足すか、走っているセッションへ伝えるか。エ・オ・カ・キ・クを、どの spec の件として残すか。
10. **揺れるテストの受け取り方（件 2・問い 3）**: 根の形が分かり、門つきの出力で決定論の赤を立てられる見込み。仮置き（決定論の赤→直し→全体テスト 1 回の緑）のままでよいか。赤が立たなかったときに、そのテストだけを何回回すか。

## 11. 要件の討議で設計へ回した事項（2026-11-17）

要件の討議で、答えが how に関わるものを設計の段（`/kiro-design`）へ回した。

1. 直し方の切り方（9 節の案 A・B・C）。ギャップ分析の推しは案 C（必ず直す 3 件と 4 番を独立のタスクにし、残りをファイルのまとまりで切る）。
2. 1 番の上限の値（`i32::MAX`）。今の振る舞い（デバッグで panic・リリースでスコープ 0 の窓の絵が変わる）は互換の振る舞いでなく欠陥なので、「数でない key」と同じく warn して捨てる形は ⑶ に当たらないと見る。SSP の測りは要らない。
3. 48 番の `BootRuntimeFn` の戻り値の形（`Option<GhostRuntime>` か `Result<_, GhostBootError>`）。
4. 33 番・件 6 の補助 `names` の分け方（`areka-runtime` の test-support に置いて本体から引くか、本体に残る 1 本の書き方を直すか・設計 D4 の「写しを作らない」を守る形）。
5. 9 節の「設計へ持ち越す調べ」の 6 件（2・30・72・83・22・1 番）。

## 12. 設計の段の調べと決め（2026-11-17・`/kiro-spec-design`）

### 12.1 調べの範囲と作法

- 種類: 既にある仕組みの直し（Extension）なので、軽い調べ（`design-discovery-light.md`）で行った。外の crate・新しい技術は 0 件で、WebSearch は使っていない。
- 使ったスキルと手引き: `kiro-spec-design`（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）。設計の書き方は完了した `audit-sylphya-actor` の design.md の型に合わせた。
- 読んだ所: `target_map.rs`・`adapter.rs`（1 番）、本体の `runtime_tests/spine.rs` の `boot_with`・`shutdown_bounded` と `spine_third_scope_talk_tests.rs`（2・3・31 番）、`lib.rs` の `BootRuntimeFn`・`wire_runtime` の手順 5 と本体の `session/runtime_boot{,_tests}.rs`（48 番）、本体と `areka-runtime` の `shell_carry_tests.rs`・`shell_carry_test_support.rs`・`areka-parsers` の `BindGroupDefaults`（33 番）、起動の系列を待つ 5 か所と一周の台帳（30 番）、`script_restriction.rs`（72 番）、`areka-runtime/src/spine.rs` の偽の SHIORI（54 番）、`film_playback_e2e_tests.rs`・本体の `crate_source_test_support.rs`・`crash_report_test_support.rs` の据え付け（96・102 番）、`shell_carry.rs` の 3 つの `reload_shell_carry_unavailable`（62 番）、`areka-actor/src/ui.rs`・`areka-emo-text/src/actor.rs`（4 番）。
- cargo のビルドとテストは回していない（このワークツリーの `target\` は空で、冷えたビルドは重い・要件 8）。コンパイルで確かめる 72・83 番は実装のタスクの初めに確かめる。

### 12.2 持ち越した調べ（11 節の 5）の結果

| 番号 | 結果 |
|---|---|
| 2 | 門は組み立てられる見込み。`SpineHarness::boot_with` が `spawn_seriko(.., PresentBridge)` を呼ぶので、`PresentBridge` を包む `SurfaceOutput` を渡す口（`boot_with_gate`）を 1 つ足せばよい（呼び手 8 か所は変えない）。**ただし 2 番の「`drive_talk` が戻ってから門を開ける」形は、直しの後は `drive_talk` が目印を待ち続けて門が開かず、30 秒の `［止まった］` で赤になる＝直しの後に緑にならない。** 設計では開ける時点を「テストが `TalkEnded` を見た時」にした（直しの前はその反復で輪を抜けて赤・直しの後は次の反復以降で目印が届いて緑）。前提は「`TalkEnded` は seriko を待たずに届く」（2 番の読み）。作って確かめるのは実装のタスク |
| 30 | 食い違いは無い。一周の台帳（`spine_conformance_script.rs` の `expected_calls`）は「起動 5」を basewareversion と番号付けしているが、`OnTranslate` を「起動 4」の後ろに `translated(..)` で挟んでいて、呼び出しの列は 6 で `spine_boot_smoke_tests.rs` の逐語の 6 と同じ。**ただし 30 番の直し方「`>= 6` にする」は使えない**: `spine_conformance_support_stage_tests.rs` の 2 本は `OnBoot` が台詞を返さず（`Ok(None)`）、`OnTranslate` が出ないので 5 呼び出しで終わる＝`>= 6` は永久に来ない。設計では「最後の basewareversion が記録された」を見る `boot_finished` に寄せた |
| 72 | 構造体は `pub(crate)`・欄は `pub` で、`note`・`source`・`enforced` を読むのは `script_restriction_tests.rs` だけ（本番のコードの読みは 0）。`Debug`・`Clone` の derive は dead_code の数えに入らないので、lib のビルドで「never read」が出る見込み。`tools/test-all.ps1` に clippy の `-D warnings` は無いので、警告が出ていても今の全体テストは緑のまま。実装で `cargo check -p areka-runtime --lib` の警告を見て決める（出なければ前提が崩れたとして記録して閉じる） |
| 83 | `runtime` の欄は `spine_conformance_lap_tests.rs` と `spine_talk_close_tests.rs` が読む（同じ crate の `#[cfg(test)]` の木の中）。許可を消しても警告は出ない見込み。コンパイルで確かめる |
| 22 | 6 ファイルの `\.rs:\d+` の当たりは 88 か所（script 55・stage 15・support 9・lap 6・judge 2・support_tests 1）。ギャップ分析の 165 か所とは数え方が違う（行の頭の「出所:」の数などを数えた可能性）。どちらでも直し方は同じで、完了の条件は「6 ファイルで `\.rs:\d+` が 0 件」とした |
| 1 | `scope_of` の呼び手は `adapter.rs` の 7 か所と `send_rebased`（L221）だけで、入口の 1 か所で上限を見れば足りる。`shell_target`・`balloon_target` を直に呼ぶ所（`frame/`・`skin_facts_copy.rs`・`shell_carry.rs`・本体の `mcp/`）は窓の一覧や表から来る小さな数。台本の `\p[2147483648]` が seriko まで届くかは、決定論のテストを `target_map`・`adapter` の純関数で立てる（`map_display_command` が `None`・直しの前はデバッグのビルドで panic）ので、偽の SHIORI の台本までは要らない。写像不能の `warn!` の文「非数値 scope の…」を字面で読む所は 0 件 |

### 12.3 設計の決め

- **D1 切り方**: 案 C（必ず直す 3 件と 4 番を先に独立のタスクに・残りをファイルのまとまりで）。20 タスク。本体の `runtime_tests/spine.rs` を触る束（T2・T3・T4・T14〜T17）は直列。
- **D2 1 番の上限**: `i32::MAX`（`balloon_target` の `2*scope+1` が `u32` に収まる最大）。入口の `scope_of` 1 か所で見る。
- **D3 48 番の形**: `Result<GhostRuntime, GhostBootError>`（`RuntimeBootError` を消す）。`Option` にしないのは、失敗を記録と `Err` の両方で表す決まり（要件 6.2）と、テストが失敗の種類（`Mount`）まで確かめられるため。
- **D4 33 番の分け方**: 6 本と補助 `names`・`names_of`・`char2_names`・`wear` を、そのまま `areka-runtime` のテストへ移す（`names_of` は本番と同じシェルの読みのまま・`sample-ghost-kit` は dev-dependencies にある）。本体に残る 1 本は、名前表を 1 式の `BindResolver::from_groups(&BindGroupDefaults::from([..]))` で組む（確かめたいのは送る順で、名前の読みではない）。補助をテストの支えへ置く形は取らない（`names_of` が番人の crate を使い、`test-support` の下では dev-dependencies がリンクされない＝依存を足さないと置けない）。
- **D5 30 番の待ち**: 数でなく「basewareversion が記録された」で見る（12.2 の 30）。
- **D6 2 番の門**: 足場だけに置く。開ける時点は `TalkEnded` を見た時（12.2 の 2）。鍵を捨てても開く。
- **D7 35 番の形**: 10 か所を `drive_talk` と同じ「進みの目印つきの `wait_until`」にそろえる。足場に 1 本の部品は置かない（輪の中身が 10 か所で違い、共通は目印の 2 行だけ）。
- **D8 62 番**: 出し直しの送り手の確かめを記録の前へ移し、送れないときだけ `reload_shell_replay_unavailable`。控えと渡しの 2 つは今の名前のまま（読み手の `ghost_switch_reload_tests.rs:L525` は控えの側の数を見る）。
- **D9 54 番**: 偽の SHIORI は `Status` を記録しない。`RecordedCall::Status` の枝は残す仮置き（枝を消すと範囲の外の 4 ファイルの match に及び、議題 4 の約束の変化の一覧に無い）。設計の討議の未決 2。
- **D10 102 番**: 読み口 `crate_path` を `areka-runtime` の `source_text_test_support.rs` へ下ろし、本体はその出し直しにして、film の E2E を移す仮置き（読み口は 1 つのまま・`CARGO_MANIFEST_DIR` の `..` の先は同じ `crates/`）。`areka-runtime` のテストの実行ファイルにもプロセスの寿命の MTA の据え付けがあるので、96 番の `CoInitializeEx` の削除と両立する。設計の討議の未決 1（移さずに ⑴ として残す案と並べる）。

### 12.4 総括の 3 つの目（`design-synthesis.md`）

- **まとめ（Generalization）**: 「2 つの crate で使う補助は `areka-runtime` の `*_test_support.rs` に 1 つ」の型を、97 番（`carrier_cue`）・95 番（`settle_bounded`）・102 番（`crate_path`）に同じ形で当てた。待ちは全部「進みの目印つきの `wait_until`」に寄せた（35 番・30 番の `boot_finished` も同じ芯の上）。
- **作るか使うか（Build vs. Adopt）**: 新しく作る部品は 2 番の門だけ（std の `Mutex`・`Condvar`）。ほかは既にある支え（`listed_name`・`give_fake_handles`・`count_level`・`code_only`・`two_scope_placements`・`settle_bounded`・`wait_until`）を使う。外の crate は使わない。
- **削る（Simplification）**: 足場の部品 `drive_until`（35 番）・名前表のテストの支え（33 番）・`RuntimeBootError` の包み（48 番）は置かない／消す。約 −700 行の見込みは変わらない。

### 12.5 危うさ

- 2 番の門の前提（`TalkEnded` が seriko を待たない）が崩れると、直しの前の赤か直しの後の緑のどちらかが立たない。そのときは設計の討議へ戻す（要件 5.2）。
- 3 番の縛るテストは spine の起動を 1 回増やす（実 GPU）。赤が出たら赤のテストだけに絞る（要件 8.4）。
- 4 番は 4 つの crate・23 ファイルを 1 つのタスクで直す。コンパイルが通れば呼び手の漏れは無い（戻りの型が変わるので、直し忘れはビルドで止まる）。
- 移し替え（T15）は、本体から消えたテストの数と `areka-runtime` に増えた数が合うことを確かめる。

### 12.6 設計の review gate

- 1 回目: 機械の確かめ（要件の 62 の番号が全部設計に在る・境界の 4 節が埋まっている・ファイルの構成に具体の道・境界とファイルの構成が合う・部品は全部ファイルの道を持つ）は通った。判断の確かめで、2 番の赤の確かめの順（門を先に入れて赤を見てから終わりの条件を直す）を書き直した（その場の直し）。
- 2 回目: 指摘 0。要件の穴・食い違いは見つからなかった（examples の 44・45 番の縛るテストの扱いは要件 4.1 の読みの問題として未決 3 にした）。

### 12.7 設計の討議（2026-11-17）

- 確かめの報告（`design-validation.md`・GO）の重大な問題 2 件（3 番の使い手の数＝11 ファイル・22 か所、範囲の外の差分の条件つきの一覧）と気付き 2 件（35 番の目印・依存の一覧の書き漏れ）は、自明な直しとして design.md に入れた。
- 議題 3 件を開発者が推奨案で承認した: 102 番は移す・54 番は `RecordedCall::Status` の枝ごと消す（型は `test-support` の下だけ＝本番の約束でない・`crates/areka-ghost/tests/ghost/` の同名の型は別物）・縛るテストは 6 件で examples の 44・45 番には置かない。

## 13. 預かった件・討議の裁定・申し送りの記録（2026-11-17・タスク 8.1／要件 5.9・2.5・5.7・5.8・7.7・7.8・8.5）

この節は、実装を終えた時点（`git log main..HEAD` のコミットの題名の末尾がタスクの番号）の事実を、出どころと一緒に残す。3 節は「ギャップ分析の時点の確かめ」、この節は「片づけた結果」。

### 13.1 件 1〜8 の片づけ

| 件 | 出どころ | 片づけ方 | どのタスクで（コミット） |
|---|---|---|---|
| 1 | `audit-sylphya-actor` 議題 3（audit #16） | `spawn_ui`・`spawn_emo_text` の戻りから使われない `Result`（`UiSpawnError`・`BootWiringError::SpawnUi`）を外し、呼び手 23 ファイル（範囲の外の `input_events/balloon_choice_settle_tests.rs` を含む。持ち主 `audit-areka-input-menu` は走っておらず、要件 5.1 のとおり申し合わせなしで直した）を同じ変更で直した | 2（`bf97c9f76`・指摘 4） |
| 2 | `runtime-module-untangle` の申し送り | 揺れの根（終わりの条件が `TalkEnded` だけを待ち、seriko のスレッドから届く目印の面 2100 を待たない）を突き止め、門（`ShowGate`）で毎回赤を 5/5 で確かめてから、終わりの条件を表明と同じ式にして 5/5 緑 | 1.2（`aec1fe951`・指摘 2・31） |
| 3 | `runtime-crate-split` 申し送り ⑵ | `build_boot_assets` の効いていない `#[allow(dead_code)]` を、テストの支えの cfg に替えた | 3.1（`a8856bc46` ほか・指摘 6） |
| 4 | `runtime-crate-split` の申し送り（ほかに一緒に伝えること・`hit_region.rs` の説明） | `hit_region.rs` の頭の「`crate::` フリー規律」の節を、今の事実（`#[path]` の取り込み 0・`is_benign_boot_error` の呼び出し 0）の 1 文にした | 3.1（指摘 5） |
| 5 | `runtime-crate-split` 申し送り ⑶ の 2.5・3.2・4.2 | 外に使い手の無い 5 項目（`record_status`・`snapshot_status_calls`・`wait_thread_end`・`HoldGate`・`shell_carry::decide`）を私有へ戻した。本体のテストだけが使う 62 項目は使い手に数えて戻さない（要件の討議の 4）。件 5 に数えるほかの狭め（3 節の件 5・design.md「要件の対応」5.5）も直した | 3.6（`c31ccb9f0`・指摘 36・53）・3.1（`a8856bc46`・指摘 46）・3.3（`b1af556bd`・指摘 59）・3.7（`9c235f4d7`・指摘 60）・3.4（`53a011934`・指摘 68・69）・4（`85f752e86`・指摘 77） |
| 6 | `runtime-crate-split` 申し送り ⑶ 4.6 | この spec の持ち物の 14 本（`switch_cue_tests` 5・`shell_balloon_resolve_tests` 3・`shell_carry_tests` 6）を `areka-runtime` へ戻した。依存は足していない。残りの 3 本は 13.4 | 6.1（`641ca25f3`・指摘 26・27・11）・6.2（`12d69add7`・指摘 33） |
| 7 | `runtime-crate-split` 申し送り ⑵ | 直さない。持ち主の `audit-placement-follow-residue` の `brief.md` 109 行に実在することを確かめた（`transition_judge.rs` の古い説明）。申し送りは要らない | 要件 5.7（確かめのみ） |
| 8 | `runtime-crate-split` 申し送り ⑶ | 直さない。持ち主の `audit-emo2-boot-switch` の `brief.md` 98 行に実在することを確かめた（`world_at` の写し・両方のファイルとも向こうの持ち物）。申し送りは要らない | 要件 5.8（確かめのみ） |

- 件 7・件 8 の確かめは、`completed/` でなく `.kiro/specs/<持ち主>/brief.md` を読んで行った（出どころ: 要件 5.7・5.8）。

### 13.2 直さずに残した指摘・規模・⑶（要件 2.2・2.5・2.6）

| 番号 | 種類 | 開発者の裁定（2026-11-17・要件の討議・推奨案で承認） | 出どころ |
|---|---|---|---|
| 25 | ⑴ 開発者の裁定に反する直し | 直さずに残す。余韻の 300 反復と根拠の文を今の形のまま置く | 完了した spec の決め R2.9「製品のコードへ観測の口を作らない」（要件の討議の 2） |
| 81 | ⑴ 開発者の裁定に反する直し | 直さずに残す。毎秒の変化通知の応答本数の規則は、余った応答が記録に残らず害が無い | 完了した spec の task 2.2「段の上限だけから導き、駆動器から借りない」（要件の討議の 3） |

- 82 番は直す側（検査を残したまま `Vec` をやめて `any` 1 回）。出どころ: 5 節の ⑴ の候補・`requirements.md`「要件の討議で確定したこと」の 3（design D3）。
- 35 番は直す側（`wait_until` の形へそろえる・R7.9 は承認で改めた）。出どころ: 5 節の ⑴ の候補・`requirements.md`「要件の討議で確定したこと」の 1。
- 規模: ギャップ分析の見込み 20〜24 タスクのうち、設計で 20 束（T1〜T20）にして、1 本のまま進めた。分けず・削らず、事実として記録するだけ（要件 2.6・2026-11-16 開発者の決め）。
- ⑶（SSP 互換か ukadoc の正典に関わる振る舞いの変化）に当たった指摘は 0 件。1 番の上限（`i32::MAX`）は、今の「数でない key」と同じ扱いで ⑶ に当たらないと見て、SSP の測りは要らないと確定した（11 節の 2）。⑵（先送りの印）に当たる指摘にしたものも 0 件（5 節）。

### 13.3 実装で分かった前提の崩れと設計の調整（`tasks.md` の Implementation Notes から）

- **72 番の前提が崩れた**: `cargo check -p areka-runtime --lib` に「never read」の警告が出ない（`derive(PartialEq)` が全部の欄を読むため）。`cfg_attr(expect(dead_code))` は付けずに、変えないまま閉じた（タスク 3.4）。
- **78 番の縛るテストの置き場**: `reload_tests.rs` の組み立てでは、shell の回が kanade と無関係に `no_context` で止まり、`accept_skin` の送り損ねの道を縛れなかった。`reload_skin_tests.rs` の結線つきの組み立てへ移し、3 回目をバルーンにした（design.md の 78 番の節と行数の表を合わせた・タスク 4）。
- **33 番の名前表**: 本体に残る出し直しのテスト 1 本は、名前表を `BindResolver::from_groups(&BindGroupDefaults::from([..]))` の 1 式で組み、服の名前は 2 件（服 A の 100 と 服 B の 101）だけにした（確かめたいのは送る順で、名前の読みではない・設計 D4・タスク 6.2）。
- **58 番の正準台帳の表**: 設計の `const CANONICAL: [...; 43]` でなく `CANONICAL_ROWS: &[...]` にした（手で持つ件数を除く趣旨・タスク 3.3）。
- **95 番の `pause`**: 足したのは `settle_bounded_with` だけ（設計の 95 番どおり。tasks.md の「2 つの関数に」は字面の違い・タスク 6.4）。
- **35 番の 10 か所は 8 か所になった**: 2 か所は 7.1 でまばたきのスモークと一緒に消えた（`da72765f9`）。残る 8 か所を `wait_until` にした（タスク 7.2）。`frame/switch_tests.rs` の 5 秒の輪は 35 番の対象外で残る。

### 13.4 申し送りの行き先と件 6 の 3 本

| 件 | 行き先 | 確かめた事実 | 出どころ |
|---|---|---|---|
| 件 6 の 2 本（`balloon_visibility_phase_tests.rs` L714・L789） | `audit-emo2-boot-switch` の `brief.md`「audit-emo2-boot-spine からの申し送り」のウ | 同 brief に書き足し済み | `runtime-crate-split` 申し送り ⑶ 4.6（要件 5.6・7.8） |
| 件 6 の 1 本（`frame/window_state_tests.rs` L144） | `audit-emo2-boot-frame` の `brief.md` の同名の節のイ | 同 brief に書き足し済み（C22・走っている相手へも申し送り済み） | `runtime-crate-split` 申し送り ⑶ 4.6（要件 5.6・7.8） |
| ア（1 番の片割れ・`frame/dpi.rs`） | `audit-emo2-boot-frame` の `brief.md` の同名の節のア | 書き足し済み | 6 節 |
| イ | 上に同じ | 書き足し済み | 6 節 |
| ウ | `audit-emo2-boot-switch` の `brief.md` のウ | 書き足し済み | 6 節 |
| オ | `audit-emo2-boot-frame`・`audit-emo2-boot-switch`・`audit-areka-app` の各 brief | 書き足し済み | 6 節・要件の討議の 5 |
| キ・ク | `audit-areka-app` の `brief.md` | 書き足し済み | 6 節・要件の討議の 5 |
| ケ | 並走中の `audit-ukadoc-survey` のセッション | 伝え済み | 6 節・要件の討議の 5 |
| **エ**（落ちの報告の写し 3 本） | **`/kiro-complete` の未解決の棚卸の件** | 持ち主の spec が無い（wintf と placement の見直しは完了済み）。完了のときに起票か扱いを決める | 6 節 |
| **カ**（`carrier_cue` の写し） | **`/kiro-complete` の未解決の棚卸の件** | 97 番で `areka-runtime` の test-support に 1 本置いた（タスク 3.3）。かけ直しの 3 回目で `areka-runtime` の中の写しは寄せ終えた。残りは本体の `install`・`readme`・`update` の cue のテスト 3 本と、依存の向きで共有できない `areka-ghost/src/prop_sink.rs`・`areka-seriko/src/actor_test_support.rs`（14.3） | 6 節 |

- 要件 7.7（C22 のほかの 5 本の触るファイル）に及ぶ直しは 0 件で、相手への申し送りが要るものはア・イ・ウ・オ・ケの記録のみ（出どころ: 6 節・`requirements.md`「要件の討議で確定したこと」の 5）。

### 13.5 新しい申し送り（要件 7.8）

- **`areka-test-wait-wording`**: 出どころはタスク 6.5。film の E2E を `crates/areka/src/runtime_tests/film_playback_e2e_tests.rs` から `crates/areka-runtime/src/film_playback_e2e_tests.rs` へ移したため、同 spec の brief の `recv_timeout` の数え方（`crates/areka/src` の下だけ）から 1 か所が外れる。
  - 受け持つ spec の実在を確かめた: `.kiro/specs/areka-test-wait-wording/`（`completed/` でない・`brief.md` あり）。
  - 走っているセッションは無い: 台帳（`roadmap.md`）に載る起票済みの spec で、C22 のウェーブ 6 本（`collisionex-regions`・`audit-placement-follow-residue`・`audit-emo2-boot-spine`・`audit-test-kits`・`audit-ukadoc-survey`・`audit-pilot`）に入っていない。
  - 同 brief の末尾に「audit-emo2-boot-spine からの申し送り」の節を 1 つ足した（数えの範囲に `crates/areka-runtime/src` を加える）。
- **`areka-test-wait-wording`（spine 系 6 本の `Instant::now() +`）**: 出どころはタスク 7.1（`da72765f9`・2 か所）と 7.2（`036c7ad44`・8 か所）。`crates/areka/src/runtime_tests/spine_{close_wiring,display,move_cue,seriko_loop,talk_close,third_scope_talk}_tests.rs` の `Instant::now() +` の締切 10 か所を消した（`git grep -c "Instant::now() +"` で `main` 10 → HEAD 0）。同 spec の brief の 2026-11-16 の節の「数え直し」（`Instant::now() +` 14）と「触るファイル」（spine 系 6 本）が今の事実と合わない。
  - 上と同じ節に 1 項目を足した（数え直しの命令は `git grep -n "Instant::now() +" -- crates/areka/src crates/areka-runtime/src`）。持ち主の元の文は直していない。

### 13.6 8.1 の操作でしか確かめられない直し（要件 8.5）

- **0 件**。どの直しも、決定論のテスト（偽の SHIORI・偽の時計・門・`target` の下の写し）で筋まで縛った。静かな机の計測・画面の操作・本物の置き場への設置は回していない。効き目は、次の実機の確かめ（全体テストの段の中の emo2・pasta の実走）で見る（出どころ: 要件 8.5・9 節の要件 8 の行「決定論のテストだけで確かめられる」・`tasks.md` の各タスクの記録）。

## 14. かけ直しの結果（2026-11-17・タスク 8.2／要件 9.1〜9.3・9.6）

範囲は 1 回目の audit と同じ 119 ファイル（移したファイルは移した先で数えた）を 7 つの束に分け、束ごとに Opus の読み手 1 人が、上限なし・全部のファイルを全行読む作法でかけた。直さずに残すと裁定した 25 番・81 番と、触らない約束のファイルは数えていない。

| 回 | 指摘 | 必ず直す | 本番の振る舞い | 直しのコミット | 直した者 |
|---|---:|---:|---:|---|---|
| 1 | 79 | 0 | 0 | `34c9ba681` | 束ごとの Opus 7 人 |
| 2 | 47 | 0 | 0（小さな振る舞いの直しは下に記す） | `856b34282` | 束ごとの Opus 7 人 |
| 3 | 27 | 0 | 0 | `cabc3c874` | Fable 1 人（直しが呼んだ組）＋ Opus 4 人 |
| 4 | 27 | 0 | 0（記録の文から番号を外した 1 件は文言だけ） | `1a47ecd6a` | Fable 2 人（crate ごと・参照の決まりを全ファイルへ） |
| 5 | 15 | 0 | 0 | `6367daf42` | Opus 2 人（crate ごと） |

**5 回目で打ち切った・残りは直し済み**（要件 9.6）。5 回目の 15 件はどれも「できれば」の説明・テストの文で、「必ず直す」と本番の振る舞い（ゴーストに見える振る舞い・SHIORI へ送るもの・ログの行・公開の面）に関わる物は 0 件だった。15 件はすべて直した。同じ所から「必ず直す」が続けて出たことは一度も無いので、要件 9.7 の総括には当たらない。

### 14.1 回ごとの主な直し

- **1 回目:** 行番号の注記・手で写した数・指摘の番号をコードの注釈から外した。テストの中の不要な `CoInitializeEx` を外した（テストのプロセスは `crash_report_test_support::install` でプロセスの寿命の MTA を持つ）。二重の眠りを 1 か所で外した。古い doc の文を今の姿へ直した。
- **2 回目:** 写しの集約（`pump_until_idle` など）、手で写した sink の本数の除去、`HOLDER_ROWS` の説明の数の訂正。小さな振る舞いの直しとして、`collision-probe` の窓 0 枚での自動終了、`balloon_background` の `page_missing`／`pixel_out_of_range` を `warn!` へ、`assets.rs` の descript の確かめを焼く前へ移した。
- **3 回目:** 前の回の直しが呼んだ指摘を Fable にまとめて渡した（2 回目に書いた新しい手書きの数・1 回目に作った `TICK_STEP_MS`／`TICK_EVERY` の写し・回ごとに継ぎ足された待ちの説明）。待ちの芯に `wait_until_paced`／`INJECT_PACE` を足し、Tick を入れる待ちの休みを関数が持つ形にそろえた（`areka-P0-kero-balloon` 要件 7.9 の「`yield_now` でなく短い sleep」を守るため。休みは「200µs、窓の後は芯の 1ms」から「常に 200µs」になり、観測の内容は変わらない）。`carrier_cue`／`broken_custom_cue` の写しを `consumer_ledger_test_support.rs` へ集めた。`GhostBootOptions` の組み立てを `runtime_boot_options` の 1 か所にした。
- **4 回目:** 参照の書き方が回をまたいで振れた（3 回目にファイルの頭へ spec 名を書いたら、本文の別の spec の番号がその spec の物に読めるようになり、読み手は「頭に持ち主を書け」と同じ形を勧めた）。そこで決まりを 1 つに決め、Fable 2 人が範囲の全ファイルへ一律に当てた。決まり: タスクの番号はコメント・記録の文から全部消す。要件・設計の参照は同じ文に持ち主の spec を名乗るときだけ残す（確かめられれば名乗らせ、確かめられなければ番号を消して事実を残す）。ファイルの頭の spec 名は本文の番号の持ち主の代わりにしない。
- **5 回目:** 4 回目の決まりの取りこぼし（「task」の語の付かないタスクの番号、`load-repro.md` の節の持ち主）と、古くなった説明の文（置き場・理由の並び・Live のテストの数）を直した。

### 14.2 回をまたいだ矛盾の見張り

開発者の指示（2026-11-17）どおり、回ごとに前の回の直しを打ち消す組・振り子になる組を先に探した。見つけたのは 3 回目（直しが生んだ数・写し・待ちの説明）と 4 回目（参照の書き方）の 2 つで、どちらも Fable に関連する指摘をまとめて渡し、決まりを 1 つにして直した。5 回目には振り子になる組は無かった。

### 14.3 範囲の外で見つけたこと（`/kiro-complete` の棚卸しへ）

- `crates/areka/src/install/install_cue_tests.rs`・`readme/readme_cue_tests.rs`・`update/update_cue_tests.rs` に、演者を取らない `carrier_cue` の写しが残る（本体は test-support から `areka_runtime::consumer_ledger::test_support::carrier_cue` を使える）。
- `crates/areka-runtime/src/adapter.rs`・`move_cue_move_severity_log_tests.rs`・`frame_chain_finalize_tests.rs` に `capture_logs`／`count_level` の写しが残る（`frame*` は触らない約束の中）。
- `crates/areka-runtime/src/frame/drain_resnap_tests.rs` の「`load-repro.md` の ⑹」に持ち主の spec が無い（`frame/` は触らない約束の中）。
- 範囲の外のファイル（`balloon_visibility.rs` など）に、持ち主の spec を名乗らない「Requirement X.Y」が残る。

## 15. 仕上げの確かめ（2026-11-17・タスク 8.3／要件 9.4・9.5）

### 15.1 全体テスト

- `pwsh -NoProfile -File tools/test-all.ps1` を排他テストの机を持って 1 回回した（コミット `621012439`・始めの未コミットの変更 0）。21 段のうち 20 段が緑、赤は「x64 workspace tests」の 1 段だけだった。赤のテストは `ukadoc-survey` の `--test consistency` の 16 本だけで、ほかのテストの実行ファイルは全部緑（`--no-fail-fast`）。
- 赤の原因はこの spec の直しにあった。かけ直しの 4 回目で、rustdoc の `bare_urls` の警告を消すため `consumer_ledger.rs` の `ukadoc: https://…` 27 行を `<…>` で囲んだ。`ukadoc-survey` はこの形の行を実装の証拠として読むので、証拠が読めなくなった（カタログに無い URL 27 件・証拠の無い実装 23 件）。27 行を元の裸の形へ戻した（`22627fc49`）。rustdoc の `bare_urls` の警告は直しの前からあったもので、字面の読み手（`ukadoc-survey`）があるので残す。
- 赤のテストだけに絞って回し直した（要件 8.4）: `cargo test -p ukadoc-survey` で 601・6・113・5 本が緑、失敗 0。

### 15.2 既定の枝との差分（比べた先は `origin/main` との分かれ目 `2a9fa0a0f`）

- 約束のファイル（`crates/areka-runtime/src/frame/`・`frame*.rs`・`crates/areka-placement/`・`crates/log-capture-kit/tests/file_length_guard_test.rs`・`vendors/`・`Cargo.lock`）の差分: 0 件。
- `crates/areka-emo-text/` に足したファイル: 0 件。
- `placement_tests/transition_*` の差分: 0 件。`placement_tests/mod.rs` の差分は 69 番の宣言の 1 行だけ。同じフォルダの `reset_balloon_pos_tests.rs`・`spawn_zorder_pair_deferred_tests.rs`・`measure_bake_log_tests.rs` は範囲の中のファイルで、差分は説明の文と 69 番の印（`reset_balloon_pos_tests.rs` の 570 行の差分のほとんどは空白で、空白を見ない差分は 12 行の追加・20 行の削除）。
- 依存: 足した依存・フォーク・`[patch]` は 0 件。`crates/areka-runtime/Cargo.toml` の差分は説明の 2 行だけ（依存の行は変わらない）。
- 端末へ出す文: 足した `println!`／`print!`／`eprintln!`／`eprint!` は 0 件（消したのは 1 件）。
- 範囲の外のファイルの差分と、それを入れたタスク:
  - `crates/areka-actor/`（4 ファイル）・`crates/areka-emo-text/`（13 ファイル・足したファイルは無い）・`crates/areka/src/input_events/balloon_choice_settle_tests.rs`: タスク 2（`spawn_ui` の戻りから使われない失敗を外した・`bf97c9f76`）。持ち主は C22 で走っていない（設計の「並走の約束」7.1）。
  - `crates/areka/src/crate_source_test_support.rs`・`doc/COMPAT_ARCHITECTURE.md`（パスの 2 行）: タスク 6.5（102 番・設計の討議の議題 1）。`doc/` の 2 行は `collisionex-regions` のセッションへ知らせ済み。
  - `crates/areka/src/ghost_session_switch_tests.rs`・`shell_balloon_switch_session_lap_tests.rs`・`update/worker_path_tests.rs`・`session/ghost_switch_talk_clock_tests.rs`: タスク 3.6（54 番の `RecordedCall::Status` の読み手・`c31ccb9f0`）。
  - `crates/areka/src/boot_config.rs`: タスク 8.2 のかけ直しの 3 回目で、`ghost_boot_options` を `session/runtime_boot.rs` の `runtime_boot_options` を 1 回呼ぶ形にした（`GhostBootOptions` の組み立ての写しを 1 か所へ・返す値は同じ）。同じ時に、説明の task 番号と持ち主の無い要件の番号も外した。設計の範囲の外の一覧に無かった追加の範囲外で、写しを 1 か所へ寄せるのに両方のファイルへ触る必要があった（返す値は `main_ghost_wiring_tests.rs`・`main_module_policy_tests.rs` が守る）。持ち主の `audit-areka-app` は C22 で走っていない。
  - かけ直しの 3〜5 回目で触った、設計の一覧に無かった追加の範囲外（持ち主は `audit-emo2-boot-switch` などで、どれも C22 で走っていない）:
    - `crates/areka-runtime/src/balloon_hover.rs`・`shell_box_hover.rs`: 説明からタスクの番号を外しただけ（5 回目・`6367daf42`）。直しに要る変更ではない。
    - `crates/areka-runtime/src/balloon_visibility_phase_zorder_chain_tests.rs`: `code_only` の写しを `source_text_test_support` へ寄せた（3 回目・`cabc3c874`）。
    - `crates/areka-runtime/src/change_cue_tests.rs`・`move_cue_balloon_lock_tests.rs`・`move_cue_move_severity_log_tests.rs`・`move_cue_move_sink_tests.rs`: cue を組む補助の写しを `consumer_ledger_test_support.rs` へ寄せた（3・4 回目）。
    - `crates/areka/src/session/ghost_switch_test_support.rs`: `TICK_STEP_MS`・`TICK_EVERY` を `pub(crate)` にした。範囲の中の `runtime_boot_tests.rs` が写しをやめて借りるため（3 回目）。
- 外への約束の変化: かけ直しで足した公開の名前は `areka_runtime::spine::{wait_until_paced, INJECT_PACE}`（テストの足場・test-support の中）。狭めた名前は `settle_bounded_with`（`pub` から `pub(super)` へ・`spine.rs` の出し直しも外した）で、読み手は `spine_wait.rs` の子のテストだけ。ログの文の変化は、`warn!`／`error!` の本文から要件の番号を外したもの（`runtime_boot.rs` の「・R7.4」、`lib.rs` の「・R7.3」、`consumer_ledger.rs` の `LedgerError` の Display の「・要件 11.3」、`zorder_cue.rs` の debug の「・要件 11.2」）と、`reload.rs` の `reload_failed` の文（`gap_cancelled` のときにも合う言い方へ）だけで、event 名は変えていない。どれも、その字面を読む物（テスト・道具）が 0 件であることを `git grep` で確かめた。
