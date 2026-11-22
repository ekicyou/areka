# ギャップ分析（audit）: audit-emo2-boot-switch

> 2026-11-22 `/kiro-validate-gap`。この spec では、ギャップ分析の段で audit をかける（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。指摘には 3 つの組を通した番号 1〜73 を付けた。以後の段はこの番号で指す。この文書は分析と選択肢を並べるだけで、直す対象は要件の討議で確定する。

## 0. 要約

- 受け持つ **68 ファイル・22,708 行**を全部、全行読んだ。内訳は本番 16 本・5,326 行、テスト 52 本・17,382 行。要件の「本番の名前のファイル 5,931 行」は、名前が本番の型でもテストの表である `balloon_visibility_timeout_suppression_cases.rs`（605 行・`#[path]` でテストに取り込まれる）を本番に数えた値（1.1 節）。
- **指摘は 73 件**。組ごとの数は、必ず直す 1・直したほうがよい 32・できれば 40。要件の討議の後に、並走の `audit-areka-app` から受け取った 2 件（74・75・直したほうがよい）を 11 節に足し、設計の段で `audit-areka-install-update` の申し送りから 76（直したほうがよい・2.2 節の 3 番の隣）を足して、計 76 件。
- 必ず直すのは 1 件だけ。
  - 1: バルーンの時間切れの満了の後もポインタがバルーン（か文字の出ている箱）の上に残っていると、相が毎画面更新を予約し続ける（`balloon_visibility_phase.rs:L119-L123`・`L172-L185`）。席を離れたままなら何時間も 60fps 前後で回る。完了した `areka-P0-draw-load-parity` の tasks.md L280 が「候補メモ」として持ち越した所で、決めではない。
- 本番のコードで次に重いのは次の 3 件（直したほうがよい）。
  - 2: シェルの窓の scope を `as u32` で切り捨てて比べる〔件 6〕。
  - 3: シェル・バルーンの切替の入口は「更新中」だけを断り、「インストール中」を断らない。読み直しの入口は両方を断る（同じ規則が 2 か所で違う）。
  - 4〜7: 出し直しと公開の面の絞り〔件 1・件 2・件 3〕。
- brief に預かった件 1〜10・12（要件 5）は、今の `main` で全部確かめ直した（3 節）。件 1〜10 はどれも今も当てはまり、件 12 は済んだまま。
  - 件 9（問い 3）: `Cargo.toml` も依存も変えずに 1 つにする形（呼び手が検体を取って渡す・写す手順は std だけの支えへ）が成り立つ。
  - 件 10（問い 4）: 足場の側で本体の組を据えても、組を要らないほかの 5 本の表明は変わらない。今は組の無い足場で要求のたびに `error!(body_systems_missing)` が出ている。
- 範囲の数え方に 1 つ穴がある。要件の名の型は `balloon_hover.rs`・`shell_box_hover.rs` を名指しで挙げるため、2026-11-22 に `audit-areka-input-menu`（#492）が足したその兄弟のテスト 2 本（144 行）がどの brief の範囲にも入っていない。読んだ結果は指摘 0（議題 9）。
- 開発者の裁定が要る候補は 5 件（1・3・37・40 と件 9 の形・5 節）。ほかに要件 7.5 の読みに関わるもの（10 番と 6 節のア）がある。外への約束（要件 1.6）に響く指摘は 4 節。C26 の約束に触れるのは 7 節。
- 規模は **17〜22 タスク**と見込む（議題 1）。

---

## 1. 測り直しと作法（要件 1.1〜1.4）

### 1.1 取り込んだ main と範囲の大きさ

- 作業のブランチ（`claude/kiro-start-audit-emo2-boot-switch-9ab333`）の HEAD は `9604793a`。main `1b3ede54` の上に、spec の初期化のコミットが 1 つ載っている（`spec.json`・`requirements.md` だけ）。`git fetch` の後に `git merge-base --is-ancestor origin/main HEAD` で、main を取り込み済みと確かめた。コードは main と同じ。
- 範囲は Project Description のとおり。`crates/areka-session/src/` の `ghost_switch*`・`shell_balloon_switch*`（17 本）、`crates/areka-runtime/src/` の `switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs`（41 本）、本体の `runtime_tests/balloon_visibility*`（7 本）・`session_tests/shell_balloon_switch*`（3 本）。

| 区分 | ファイル | 行 |
|---|---:|---:|
| 本番（`areka-session` 2・`areka-runtime` 14） | 16 | 5,326 |
| `areka-session` のテストと支え | 15 | 5,680 |
| `areka-runtime` のテストと支え（`_cases.rs` の表を含む） | 27 | 8,495 |
| 本体のテスト（`runtime_tests/`・`session_tests/`） | 10 | 3,207 |
| **計** | **68** | **22,708** |

- 要件の Project Description の実測（68 ファイル・22,708 行）と合う。本番とテストの分け目だけが違う: 要件は名前で分けて本番 5,931・テスト 16,777 とした。`balloon_visibility_timeout_suppression_cases.rs`（605 行）は `balloon_visibility_timeout_suppression_tests.rs:L374` の `#[path]` で取り込まれるテストの表なので、この文書はテストに数えた。
- 1,000 行を越えるファイルは 0 本。900 行台は `ghost_switch_tests.rs` 982・`balloon_visibility_tests.rs` 960・`ghost_switch.rs` 927。どれも 1,000 行の番人の上限越えの表に載っていない。
- 範囲の名の型から外れる兄弟: `crates/areka-runtime/src/balloon_hover_tests.rs`（31）・`shell_box_hover_tests.rs`（113）。2026-11-22 の `audit-areka-input-menu`（#492 `26a5d573`）が足した。どの brief の「受け持つファイル」にも無い。範囲に入れるかは議題 9。読んだ結果は指摘 0（2 本とも `balloon_hover.rs`・`shell_box_hover.rs` の私有の振る舞いを正しく試す）。

### 1.2 ponytail の版

- 今入っているのは 5.3.3。`~/.claude/plugins/installed_plugins.json` の `ponytail@ponytail`（user scope・`lastUpdated` 2026-11-06・`gitCommitSha` `96608e07`）で確かめた。brief の起票のときは 5.3.2。物差し（`skills/ponytail-audit/SKILL.md`）は 5.3.2 と 5.3.3 で同じ（`audit-emo2-boot-spine`・`audit-emo2-boot-frame` の研究の 1.2 節と同じ版）。

### 1.3 作法

- ponytail-audit の 6 つの物差し（バグ・危ない所・負荷・テストの抜け・遅い所・要らないコード）で見た。どの指摘にも具体的な場合を付けた。「使われていない」と書く前には、ツリー全体（`crates/`・`tools/`・`doc/`・`.claude/skills/`・字面の読み）を grep した。
- 外した作法は 2 つ。
  - 「最大 20 件」を外し、全件を出した（要件 1.3）。
  - 「読まなかった所」を外し、68 ファイルを全部、全行読んだ（要件 1.4）。範囲を 6 つの束に分けて読み手を立てた。どの束からも、全ファイルを全行読んだ旨と、ファイルごとの行数の表を受け取った。合計は 22,708 行で、上の表と合う。
- `shortcut:`・`ponytail:` の注釈は範囲の中に **0 件**。範囲の中の `#[allow(...)]` は 2 件だけ。
  - `ghost_switch_test_support.rs:L212` の `#[allow(dead_code)]`（`RigSample::keep`・「読まず、捨てると木が消える寿命だけを持つ」）。理由は今の事実と合う。欄は `pub mod` の中の `pub` の型の `pub` の欄なので、許可そのものが効いていない見込みがある（cargo を回していないので確かめていない・Research Needed）。
  - `shell_balloon_switch_session_lap_tests.rs:L112` の `#[allow(clippy::type_complexity)]`（戻り値の 3 つ組・doc が中身を説明する）。今の事実と合う。
- わざと置いた先送りの形（指摘にしない）: `move_cue_geometry.rs:L40` の `// TODO: 宣言された point.basepos を差す resolver を足す。` と、実装 1 つの型シーム `BaseposResolver`。追跡先は `roadmap.md:363` の `surfaces-basepos`（据え置き・区分 D）と `doc/COMPAT_ARCHITECTURE.md:155`。`TimedMoveImmediate`（`window-timed-directives` の brief）と `OtherGhostBase` → scope 0（COMPAT:138）も記録つきの縮退。
- プロジェクトの決まりへの違反も指摘に数えた。記録の無い失敗の道（`let _ =`）・1,000 行・決定論のテスト・終了の道・spec 名と要件番号をコードに書かない（`## Comments`）・古くて事実と違う説明。
- 想定した負荷は次のとおり。
  - 利用者 1 人のデスクトップで areka が 1 つ動き、ゴーストの窓が数枚ある。ECS の相が毎フレーム（60fps 前後）回る。何時間も動き続ける。
  - 他人のゴースト・シェル・バルーンのフォルダ（descript・台本）は信用できない入力として扱う。
  - テストは `cargo test -j 4` で GPU のテストと CPU を取り合う状態を、ふだんの負荷とみなした。
- cargo は回していない（読むのと grep だけ）。

### 1.4 読んだ分け方（6 つの束・全部読んだ）

| 束 | 中身 | ファイル | 行 |
|---|---|---:|---:|
| B1 | `areka-session` の本番 2 本・足場 2 本・`ghost_switch_tests` ほか 4 本 | 8 | 3,560 |
| B2 | `areka-session` のテスト 9 本・`areka-runtime` の `skin_switch.rs`・`switch_assets.rs`・`switch_assets_tests.rs` | 12 | 4,098 |
| B3 | 本体の `runtime_tests/balloon_visibility*` 7 本・`session_tests/shell_balloon_switch*` 3 本 | 10 | 3,207 |
| B4 | `balloon_visibility` の本番 4 本・支え 2 本・`balloon_hover.rs`・`shell_box_hover.rs`・`hover_inject.rs`・相のテスト 6 本 | 15 | 3,567 |
| B5 | `balloon_visibility` の判断のテスト 9 本（`_cases.rs` を含む） | 9 | 4,067 |
| B6 | `move_cue*`・`change_cue*` 14 本 | 14 | 4,209 |
| 計 | | **68** | **22,708** |

- ほかに、件 3（要件 5.3）の「開けた公開の使い手」を 1 つの読み手にツリー全体で grep させた（3 節の件 3）。
- 束の数え直し: 6 つの束の一覧を合わせると 68 本で、範囲の一覧と一致し、重なりは 0。
- 主な指摘は、根拠の行まで読み直して確かめた（8 節）。

---

## 2. 指摘の一覧（通し番号・3 つの組を通す）

各指摘の印は次のとおり。

- 〔約束 a/b/c〕: 直すと「外への約束」に触れる。(a) ゴーストと SSP 互換の振る舞い（`doc/COMPAT_ARCHITECTURE.md` の行を含む）、(b) `areka-runtime`・`areka-session` の公開の面（テストの支えの公開の補助を含む）、(c) ログの行の語と target。響く読み手は 4 節。
- 〔裁定 ⑴/⑵/⑶〕: 要件 2.2 の 3 種類（⑴ 開発者の裁定に反する直し・⑵ わざと置いた先送りの印・⑶ SSP 互換か ukadoc の正典に関わる振る舞いの変化）に当たる、またはその候補。
- 〔件 n〕: brief に預かった件（3 節の件 n）と同じ所を指す（同じ直しを 2 回数えない・要件 1.8）。
- 〔範囲の外〕: 直しの本体か一部が、範囲の外のファイルに要る（持ち主を書く）。
- 〔C26〕: 直すと C26 のほかの 2 本の持ち物のファイルに及ぶ（7 節）。
- 規模: S は 1 タスク以内の小さな直し、M は 1〜2 タスク。

### 件数

| 組 | 番号 | 件数 |
|---|---|---:|
| 必ず直す（Must fix） | 1 | **1** |
| 直したほうがよい（Should fix） | 2〜33 | **32** |
| できれば（Nice to have） | 34〜73 | **40** |
| 計 | | **73** |

- 束の読み手が出した番号（B1-1 など）は、まとめるときに付け直した。読み手が出した 80 件（B1 18・B2 15・B3 11・B4 10・B5 13・B6 13）を、同じ所か同じ直しを指すものどうしで 4 つの指摘（4・13・16・24）にまとめ（9 件減）、読み手の「範囲の外」の注のうち実は範囲の中を指した 1 件（3 番）と件 3 の読み手から 1 件（38 番）を足して 73 件になった。対応は付録 A。

### 2.1 必ず直す（1）

#### 1. 時間切れの満了の後も抑止が続くと、毎画面更新を予約し続ける〔裁定 ⑴ の候補〕
- 場所: `crates/areka-runtime/src/balloon_visibility_phase.rs:L119-L123`（`match visibility_wake(..)`）・`L172-L185`（`visibility_wake`）。判断は `balloon_visibility_wait.rs:L231-L242`（抑止中の超過は保留し、満了予定を消さない）。
- 種類: 負荷
- 何をする所か: 時間切れの計測が動いている間、満了の時刻に次の起床を預ける（`tick_wake::arm_deadline`）。期限が過ぎていれば `VisibilityWake::Now` で次の画面更新を予約する（`tick_wake::mark(REARM)`）。
- 問題（具体的な場合）: 台詞の後、利用者がポインタをバルーンの上（または文字の出ている箱の上）に置いたまま席を離れる。30 秒の満了の後は、判断が「抑止中の超過は保留」で満了予定を残すので、`visibility_wake` は毎回 `Now` を返し、毎フレーム `REARM` が立つ。ポインタが離れるまで、アイドルのデスクトップで frame_system が 60fps 前後で回り続ける。`frame/scale_text.rs:L241` は「放置中まで回り続けさせないため」に `REARM` を止めていて、同じ考え方に反する。抑止の解け方（`WM_MOUSELEAVE`・`WM_MOUSEMOVE` の `POINTER`・ドラッグの終わりの入力）はどれも自分で起床を持つ（`wintf/src/ecs/world/tick_wake.rs:L302-L303`）。解けた縁で `decide_timeout` が `now + wait` を立て直す。
- 直し方: `visibility_wake` に `state.prev_suppressed`（か判断の抑止の答え）を渡し、`remaining <= 0.0` かつ抑止中なら `None`、抑止なしなら今のまま `Now`。`balloon_visibility_phase_wake_tests.rs:L44-L57` に「抑止中の超過は預けない」の 1 件を足し、`Now` の doc の「抑止で保留中など」を直す。本体の `tick_gate_config_producers_tests.rs` の字面の検査（`REARM`・`arm_deadline(`）は残るので緑のまま。
- 直さないと: 滞在が続く限り、CPU を回し続ける（`areka-P0-draw-load-parity` の目標「CPU 絶対値 3.0% 未満」を放置中に崩しうる）。
- 〔裁定 ⑴ の候補〕: `visibility_wake` を入れた完了 spec `areka-P0-draw-load-parity` の `tasks.md:L280` は「候補メモ: 期限超過かつ抑止中（ドラッグ／滞在／選択肢）は毎フレーム REARM＝省略しない（安全側・解除はポインタ駆動なので `None` でも足りる可能性＝周回で検討）」と書く。決めでなく持ち越しの候補なので、覆す直しには当たらない見込み。要件の討議で確かめる（議題 2）。
- 規模: S

### 2.2 直したほうがよい（2〜33）

#### 本番のコード

#### 2. シェルの窓の scope を `as u32` で切り捨てて比べる〔件 6〕
- 場所: `crates/areka-runtime/src/balloon_visibility_phase.rs:L459-L465`（L464 `marker.scope as u32`）
- 種類: バグ
- 何をする所か: 箱に文字が出ている scope のシェルの窓がドラッグ中なら、時間切れを止める。
- 問題（具体的な場合）: `CharWindowMarker.scope` は `usize`（`areka-placement/src/spawn.rs:L109`）。scope 4294967296 の窓は 0 へ切り捨てられ、scope 0 の箱に文字が出ていれば、別の窓のドラッグで scope 0 の時間切れが止まる。ほかの呼び手（`frame/` の 9 か所）は `target_map.rs:L48` の `window_scope` で上限（`MAX_SCOPE` = `i32::MAX`）を守っている。今は窓の scope が {0,1} だけなので届かない。
- 直し方の候補（決めない）:
  - ㋐ 広げて比べる: `box_scopes.iter().any(|&s| s as usize == marker.scope)`。`box_scopes` は上限の中なので答えは ㋑ と同じで、記録は出ない。
  - ㋑ 要件 5.6 の字面どおり `window_scope(marker.scope).is_some_and(|s| box_scopes.contains(&s))`。`window_scope` は外れるたびに `warn!` を 1 行出す（抑えが無い）ので、上限越えの窓をドラッグしている間は毎フレーム 1 行出る。
  - どちらも `balloon_visibility_phase_box_tests.rs` の `shell_window_drag_counts_for_scopes_with_box_text` に scope `1<<32` の窓の 1 件を足し、直す前に赤を確かめる（要件 4.2）。
- 直さないと: 他の窓のドラッグで、別の scope のバルーンが消えずに残る。
- 規模: S

#### 3. シェル・バルーンの切替の入口が「インストール中」を断らない〔裁定 ⑴ の候補〕〔約束 c〕
- 場所: `crates/areka-session/src/shell_balloon_switch.rs:L132-L140`（`update_busy` だけを問う）。比べる相手は `reload.rs:L300-L322` の `overlap_or_context`（`update_busy` と `install_busy` の両方）。
- 種類: 危ない所（同じ規則が 2 か所で違う）
- 何をする所か: 切替の要求を、ゴースト切替中・更新中なら断る。
- 問題（具体的な場合）: インストールの作業がバルーンのフォルダを書き換えている最中に、台本の `\![change,balloon,…]` か メニューから同じバルーンへの切替が届くと、入口は受け、背景で資産を読み始める。読み直しの入口は同じ場合を `RefusedReason::Installing` で断る。完了 spec `areka-P0-shell-balloon-switch` の設計の表（`design.md:L966`）は「ゴースト切替中・更新中に要求」だけを挙げ、インストール中を考えた跡が無い。
- 直し方: `install_busy` も問い、`warn!(event = "skin_switch_installing")` で断る（`SkinVerdict` に腕を 1 つ足すか、`Updating` と同じ腕にまとめる）。テストを 1 本足す。
- Research Needed: インストールの完了の後の自動切替（同設計 L39「インストール直後の自動切替」）が、`install_busy` がまだ真の間に入口を呼んでいないか。呼んでいるなら、断ると自動切替が止まる。
- 〔約束 c〕: 新しい warn の行が 1 つ増える（既にある行の語は変わらない）。
- 規模: S
- 設計の段の調べ（12 節の 12.1）: areka にはインストール直後の自動切替が無い（完了 `ghost-install` の裁定）。ただしゴーストが `OnInstallComplete(Ex)` の応えの台本で出す `\![change,shell/balloon,…]` は、`install_busy` が真の間に入口へ届く。

#### 76. ゴースト切替の入口が「更新中」を断らない〔約束 c〕（設計の段で足した・`audit-areka-install-update` の申し送り）
- 場所: `crates/areka-session/src/ghost_switch.rs:L250-L320`（`request_ghost_switch_with` は進行中・文脈なし・該当なしだけを問う）。比べる相手は `shell_balloon_switch.rs:L132`（`update_busy` で断る）と `reload.rs:L311`（`update_busy`・`install_busy` で断る）。
- 種類: 危ない所（同じ規則が 3 か所で違う）
- 何をする所か: ゴーストの切替の要求を受け、降ろして起こす。
- 問題（具体的な場合）: ネットワーク更新がゴーストのフォルダを書き換えている最中に、メニューか台本の `\![change,ghost,<同じゴースト>]` で同じゴーストへの切替が届くと、入口は受け、書きかけのフォルダから SHIORI を起こしうる。
- 区別の要: 更新の手続き自身の読み直し（`crates/areka/src/update/desk.rs:L173-L215` の `reload`）も同じ入口を通る。背景のスレッドは `DeskAsk::Reload` を `DeskAsk::OrderDone` より先に頼む（`update/worker.rs:L243-L251`・`L92`）ので、その時は `Stage::Running`＝`update::desk::is_busy` が真（`desk.rs:L537-L541`）。入口が `update_busy` だけで断ると、更新が自分の読み直しを断られる。
- 見分け方（コードで確かめた）: 入口の呼び手は 4 つ。メニュー（`menu/ghost_frame.rs:L51`・`ChangeOrigin::Manual`・`GhostSpec::Folder`）、台本の取り出し（`ghost_switch.rs:L80`・`Automatic`・`GhostSpec::Name`）、更新の読み直し（`update/desk.rs:L192`・`Automatic`・`Folder`）、上書きのインストール（`install/overwrite.rs:L109`・`Automatic`・`Folder`）。メニューと台本は「`Manual` か `Name`」で、更新とインストールの手続きは「`Automatic` かつ `Folder`」で見分けられる。
- 直し方: 入口で、要求が「`Manual` か `Name`」で `update_busy` が真なら、`warn!(event = "ghost_switch_updating")` を 1 行残して `SwitchVerdict::Updating`（腕を 1 つ足す）で断る。手続きの要求（`Automatic` かつ `Folder`）には当てない。テストを 1 本足す。
- `SwitchVerdict` の腕を足すと、`install/overwrite.rs:L118-L130` の `match` が網羅でなくなる（`audit-areka-install-update` の持ち物）。手続きの要求には `Updating` を返さないので、相手の側は `NotFound | NoContext` の腕に `Updating` を足すだけでよい（申し合わせる）。
- 〔約束 c〕: 新しい warn の行が 1 つ増える。
- 規模: S

#### 公開の面と出し直し

#### 4. シェル・バルーンの入口の出し直しが設計の 5 型より広い〔件 1〕〔約束 b〕〔範囲の外〕〔C26〕
- 場所: `crates/areka-session/src/shell_balloon_switch.rs:L37-L40`。見出しの説明は `crates/areka-runtime/src/skin_switch.rs:L3-L4`。
- 種類: 要らないコード（yagni）・古い説明
- 何をする所か: `areka_runtime::skin_switch` の 10 項目を `pub use` する（自分で使う取り込みも兼ねる）。
- 問題（具体的な場合）: 本番でこの道を通るのは本体の `menu/balloon_frame.rs:17`（`SkinKind`）と `menu/shell_frame.rs:18-20`（`SkinKind`・`SkinOrigin`・`SkinRequest`・`SkinSpec`）だけ。`areka-session` の本番（`hooks.rs:20`・`reload.rs:40`・`runtime_boot.rs:31`）は `areka_runtime::skin_switch` を直に引く。残る `SkinRequestRaw`・`SkinSwitchInFlight`・`SkinSwitchStage`・`skin_ref_name`・`skin_ref_path` をこの道で引くのはテストだけ（3 節の件 1）。`skin_switch.rs:L3-L4` の「入口は呼び手のために出し直す」も使われ方と合わない（`reload_test_support.rs:9/14` と本体 `session_tests/reload_tests.rs:26/37` は同じファイルの中で両方の道を混ぜる）。
- 直し方: `pub use` を 5 型に絞り、残りの 5 項目は私有の `use` にする。テストの取り込みは `areka_runtime::skin_switch::…` へ替える。範囲の中は `shell_balloon_switch_session_lap_test_support.rs:21`・`shell_balloon_switch_session_lap_tests.rs:24-27`・本体 `session_tests/shell_balloon_switch_session_tests.rs:12-15`・`session_tests/shell_balloon_switch_tests.rs:43`（glob に 1 行足す）。範囲の外は `areka-session/src/reload_test_support.rs:14`・本体 `runtime_tests/frame/switch_tests.rs:37`・`runtime_tests/frame/switch_finish_tests.rs:50-52`（どれも `audit-areka-app` の持ち物＝〔C26〕）。`skin_switch.rs:L3-L4` の 2 行を決めに合わせて直す。
- 直さないと: テストのためだけの道が公開の面に残る。
- 規模: S

#### 5. テストのためだけの `SwapFinish` の出し直し〔件 2〕〔約束 b〕〔範囲の外〕〔C26〕
- 場所: `crates/areka-session/src/shell_balloon_switch.rs:L41-L43`
- 種類: 要らないコード（delete）
- 問題（具体的な場合）: 使い手は本体のテスト 2 か所だけ（`runtime_tests/frame/switch_finish_tests.rs:50-52`・`runtime_tests/frame/switch_tests.rs:838`）。どちらも `areka-runtime` の差し替えの相を試すテストで、本体は `areka-runtime` に直に依存している（`crates/areka/Cargo.toml:43,107`）。
- 直し方: L41-L43 を消し、2 か所を `areka_runtime::skin_switch::SwapFinish` から引く形にする（2 か所は `audit-areka-app` の持ち物＝〔C26〕）。
- 規模: S

#### 6. ゴースト切替の入口も、使い手の無い項目を `pub use` する〔約束 b〕〔範囲の外〕〔C26〕
- 場所: `crates/areka-session/src/ghost_switch.rs:L40-L44`
- 種類: 要らないコード（yagni）
- 問題（具体的な場合）: `NameResolution`・`resolve_special_name` の出し直しの使い手は、`ghost_switch.rs` とその子のテストのほかに 0 件。`LastInstalledGhost` を外から引くのは本体のテスト 4 本だけ（`install/desk_overwrite_tests.rs:39`・`desk_record_tests.rs:22`・`startup_flow_tests.rs:292`・`worker_tests.rs:32`）。L40-L41 の `#[cfg(test)] use UnresolvedReason` は `ghost_switch_tests.rs` のためだけ。
- 直し方: `NameResolution`・`resolve_special_name` は私有の `use` にする。`UnresolvedReason` はテストのファイルで取り込む（23 番をすれば自然に消える）。`LastInstalledGhost` は、本体の 4 本を `areka_runtime::special_name::LastInstalledGhost` へ替えてから私有にするか、今のまま置く（4 本は `audit-areka-install-update` の持ち物＝〔C26〕）。行は減るだけ。
- 規模: S

#### 7. `runtime-crate-split` が開けた項目のうち、今すぐ戻せるのは `settle_box_hover` 1 つ〔件 3〕〔約束 b〕
- 場所: `crates/areka-runtime/src/shell_box_hover.rs:L11`（ほかは 3 節の件 3 の表）
- 種類: 要らないコード（公開の面）
- 問題（具体的な場合）: 4.2 ⑴ で「本体のテストが使う」として開けたが、今の使い手は `areka-runtime` の相（`balloon_visibility_phase.rs:L416`）と兄弟のテスト `shell_box_hover_tests.rs` だけ。本体と `areka-input` に 0 件（`audit-areka-input-menu` でテストが `areka-runtime` へ移った後、開ける理由が消えた）。2.5 の 26 件はどれも外に使い手があるか、公開の型の欄・署名に出るので戻せない。4.2 の残りは、24 番・40 番で本体のテストを戻したときに使い手を失う（3 節）。
- 直し方: `pub(crate)` に戻す。24 番・40 番の後の姿で残りを数え直し、使い手を失った渡し口と `test_open` を閉じる（問い 2 の仮定どおり）。
- 規模: S

#### 古くて事実と違う説明（本番と足場）

#### 8. `SwitchVerdict` の説明「呼び手は分岐しない」が事実と違う
- 場所: `crates/areka-session/src/ghost_switch.rs:L111`
- 種類: 古い説明
- 問題（具体的な場合）: 本番の呼び手が判定で分岐する（`install/overwrite.rs:118-130` は Busy なら預かったまま頼み直す・`update/desk.rs:201` は Accepted のときだけ後送りを据える・`reload.rs:378`）。説明を信じて判定の意味を変えると、上書きのインストールや更新の読み直しが壊れる。
- 直し方: 「（記録の語彙・呼び手は分岐しない）」を消すか、「呼び手は Busy を頼み直しに使う」と書き直す。
- 規模: S

#### 9. 足場の「emo2 は今のゴーストにならないので誰も読まない」が事実と違う
- 場所: `crates/areka-session/src/ghost_switch_test_support.rs:L248-L251`
- 種類: 古い説明
- 問題（具体的な場合）: 本体の `session_tests/ghost_session_switch_fallback_tests.rs:64,133`・`main_session_mark_tests.rs:740` は既定のゴースト emo2 へ戻して起こす。そのとき `resolve_balloon_for_ghost`（`boot_config.rs:428-461`）はゴーストの中の `ghost\emo2\balloon\emo2-kakukaku` を読み、根の同名のバルーンを隠す。根の `balloon/emo2-kakukaku` を書き換えても emo2 へ戻した回には効かない。
- 直し方: 最後の 1 文を消すか、「emo2 を起こすとゴーストの中のバルーンが根を隠す」と書き直す。
- 規模: S

#### 10. `COMPAT` の抑止の行が、箱の上の滞在とシェルの窓のドラッグを書いていない〔範囲の外〕〔約束 a〕〔C26〕
- 場所: `doc/COMPAT_ARCHITECTURE.md:177`・`:407`（どちらも §8 の沈黙ルール対応表）。コードは `balloon_visibility_phase.rs:L252-L258`（`observe_shell_dragging`）・`L435-L439`（`combine_hover`）。
- 種類: 古い説明
- 問題（具体的な場合）: 407 行は「止める理由はバルーン窓のドラッグ中と、見えているバルーンの上にポインタがあることの 2 つだけ」と書く。コードは、箱に文字が出ている scope のシェルの窓のドラッグでも、文字の出ている箱の上の滞在でも止める（完了 spec `areka-P0-shell-balloon` の設計 6.11 の決め）。表を読んだ人は、シェルの窓のドラッグでバルーンが消えないのを不具合と読む。
- 直し方: 407 行（か 177 行）に 1 文と定義点（`observe_shell_dragging`・`combine_hover`・`settle_box_hover`）を足す。コードは変えない。
- 要件 7.5（既にある行の「名指しの字面だけ」を直し、行を足さない）は、既にある行へ文を足すことを許していない。議題 6。
- 規模: S

#### 11. `MoveDirective.x` の説明「数値＝物理 px」が型の説明と食い違う〔約束 b〕
- 場所: `crates/areka-runtime/src/move_cue.rs:L65-L68`
- 種類: 古い説明
- 問題（具体的な場合）: 同じファイルの `AxisSpec::Px`（L86-L91）は「作者基準 px」で、物理 px への換算（k 倍）は `compute_move_target` がする。COMPAT:174 も「物理 px」と書いていたのは是正済みの契約違反だと書く。欄の説明だけが古く、拡大率 200% のとき、読んだ人は k を掛けない値を物理 px と取り違える。
- 直し方: L65・L67 の「数値＝物理 px」を「数値＝作者基準 px（[`AxisSpec::Px`]）」にする。
- 規模: S

#### 12. `MoveCueSink` の説明と emit の注釈が「move と resetwindowpos だけ」のまま〔約束 b〕
- 場所: `crates/areka-runtime/src/move_cue.rs:L183-L191`（`///`）・`L254-L259`
- 種類: 古い説明
- 問題（具体的な場合）: 説明は「`"move"`、または `"execute"` で第 1 引数が `resetwindowpos` を運ぶものだけ」「5 組」と書く。実際の腕（L260-L298）は move のほかに 8 組（execute,resetwindowpos／execute,resetballoonpos／set・reset,sticky-window／set,alignmenttodesktop／set,alignmentondesktop／lock・unlock,balloonmove）を選ぶ。説明を読んだ人は `\![lock,balloonmove]` がこの受け口に来ないと思う。
- 直し方: 1 行目を「窓の位置の指令（`\![move]`・並べ直し・窓の組・吸着・バルーン位置・バルーンを動かさない指定）」にし、「だけ」「のみ」「5 組」を消す。
- 規模: S

#### 決まりの違反

#### 13. spec の名・要件番号・設計の番号がコードと文字列に残る〔約束 c〕
- 場所（7 つの束の指摘をまとめた）:
  - ログの文: `move_cue.rs:L240`・`L244`・`L248`・`L302`・`L334`（「D8④」「R8.5」「R4.5/R8.5」「R5.5」）・`hover_inject.rs:L140`（「・8.2」）・`L376`（「・8.6」）
  - 表明の文: `balloon_visibility_timeout_config_tests.rs:L32`・`L180`・`L211`（Requirement 4.2・9.5）・本体 `runtime_tests/balloon_visibility_lifecycle_e2e_tests.rs:L175`（Requirement 4.1）・本体 `runtime_tests/balloon_visibility_phase_tests.rs:L368`（D10）・`L671`（Requirement 6.7）・`move_cue_apply_move_tests.rs:L160`・`L189`・`L360`・`L370`・`move_cue_move_severity_log_tests.rs:L25`・`L50`・`L55`・`L102`・`L124`・`L129`・`move_cue_move_sink_tests.rs:L47`
  - テストの名: `ghost_switch_balloon_tests.rs:L248` `a_memory_written_before_this_spec_finds_the_ghost_inner_balloon`
  - 見出し・注釈: `balloon_visibility_user_break_tests.rs:L70-L71`・`L120-L121`（「判断分岐 ⑵／⑹」＝`areka-P0-balloon-break/design.md` の番号で、`decide` の説明は ⑴〜⑷ しか持たない）・`move_cue_place.rs:L187`（「MoveMeasure」＝`window-work-area-clamp/design.md:616` の部品名）
- 種類: 決まりの違反（`CLAUDE.md` の `## Comments`）
- 問題（具体的な場合）: spec が completed へ移り番号がずれた今、どれも辿れない（「⑹」はコードのどこも指さない）。
- 直し方: 括弧の番号だけを消す。テストの名は `a_folder_name_only_memory_finds_the_ghost_inner_balloon` などに替える（名指しは 0 件）。見出しは「隠す側」「隠した後の出し直し」の語だけにする。`MoveMeasure` は「拡大率の取り違えを後で測るための欄」など理由だけの文にする。
- 〔約束 c〕: ログの文が変わる（語と target と欄は変えない）。`crates/areka/tests`・`tools/`・`doc/`・`.claude/skills/` を「良性スキップ」「非数値 scope」「MoveCueSink: 」・hover_inject の文で grep し、字面で読む所は 0 件。
- 規模: S

#### 14. 準備の失敗を `let _ =` で捨てる
- 場所: `crates/areka-session/src/ghost_switch_fallback_tests.rs:L206`（`let _ = crate::boot_resolve::write_session_mark(&profile, "A");`）
- 種類: 決まりの違反（記録の無い失敗の道）
- 問題（具体的な場合）: 書けなかったとき、argv の回の期待 `Some("A")` が `None` と食い違って赤になり、「切替が印を消した」ように見える。ふつうの回は書けなくても緑のまま通る。
- 直し方: `.expect("前から在る印を書く")` にする。
- 規模: S

#### 15. 記憶のアクターの後始末の join が失敗も止まりも捨てる〔件 7〕
- 場所: `crates/areka-runtime/src/move_cue_apply_move_sticky_tests.rs:L254-L255`・`move_cue_place_tests.rs:L385-L386`（どちらも `let _ = parts.handle.join();`）
- 種類: 決まりの違反
- 問題（具体的な場合）: sylphya の中で panic が起きても `Err` を捨てるので緑のまま。アクターが終わらないとテストが打ち切りなしに固まる。
- 直し方: 2 か所とも `crate::spine::join_bounded("sylphya", parts.handle).expect("sylphya が終わる");`。`join_bounded(what: &str, handle: ActorHandle) -> Result<(), ActorError>` は `spine_wait.rs:L428` にあり `spine.rs:L321` で出し直されている。前例は本体 `runtime_tests/frame/switch_finish_tests.rs:300`。
- 規模: S

#### テストの抜け（落ちない・空振りする檻）

#### 16. 「0 件」の檻が、ログの文面だけに結び付いていて空振りしうる〔約束 c〕
- 場所: `crates/areka-session/src/ghost_switch_reload_tests.rs:L234-L240`（`ticker_closed`）・`L269`。`shell_balloon_switch_session_balloon_tests.rs:L157-L163`（`external_transitions`）・`L324`。
- 種類: テストの抜け
- 何をする所か: 前者は「shiori の読み直しで単位（loop ticker）が降ろされない」を、`ghost_session.rs:600` の文面「loop ticker を Close」の info が 0 件であることで見る。後者は「バルーンの差し替えで古い装着の消滅を外からの変化と記録しない」を、`balloon_visibility_phase.rs:L494` の文面「本制御が発行していない可視状態の変化」が 0 件であることで見る。
- 問題（具体的な場合）: どちらの文面も、木の中で 1 件以上出ることを確かめるテストが無い（読み手はこの 2 か所だけ）。文面を手直しすると、単位を降ろすように壊れても・`forget_scope` を外しても、件数は 0 のままで緑。doc の「非空虚性」は文面が今のままのときにしか成り立たない。
- 直し方の候補（決めない）: ⑴ 陽性の対を同じファイルに置く（前者は単位を降ろす ghost の読み直しのテスト L88 に `assert!(ticker_closed(&events) >= 1)`・後者は文面を `areka-runtime` の `pub const` 1 つにして本番とテストから引く）。⑵ 本番の行に `event=` の欄を足して欄で照らす（〔約束 c〕・読み手は `crates/` のこの 2 本だけ）。
- 規模: S

#### 17. 消去の回数（`clear_count`）で表示を判定する分岐にテストが 1 本も無い
- 場所: 本体の判断 `crates/areka-runtime/src/balloon_visibility_decision.rs:L256-L262`。支え `balloon_visibility_test_support.rs:L19-L31` の `seen` は `clear_count: 0` 固定。
- 種類: テストの抜け
- 問題（具体的な場合）: 時間切れで隠れた scope 0 に 3 文字が残り、次のトークの `\c` と新しい 3 文字が同じフレームに届くと、文字の数は 3→3 で変わらない。`clear_count` で基準を 0 に戻す分岐を消すと表示が出なくなるが、0 以外の `clear_count` を組むテストが無いので全部緑のまま（`GlyphObservation {` を組むのは本番 2 か所と支えの 0 固定だけ）。
- 直し方: `balloon_visibility_content_tests.rs`（355 行）に 1 本足す（約 30 行・対照として `clear_count` 0 なら何も起きないことも見る）。
- 規模: S

#### 18. 「待ち時間は 1 回だけ読む」を確かめていない〔約束 a〕
- 場所: `crates/areka-runtime/src/balloon_visibility_timeout_config_tests.rs:L264-L277`（`configured_timeout_secs_reads_env_once_and_is_stable`）
- 種類: テストの抜け
- 問題（具体的な場合）: 環境変数を変えないので、`OnceLock` を外しても 2 回の呼び出しは同じ値を返し緑のまま。外すと毎フレーム `std::env::var` と `info!` が走りログが毎秒 60 行で埋まるが、このテストは落ちない。
- 直し方: 先に 1 回呼んで値を確定させ、`log_capture_kit::capture(|| (configured_timeout_secs(), configured_timeout_secs()))` の記録が 0 件であることを表明する（捕捉は呼んだスレッドの窓で、ほかのテストと混ざらない）。
- 〔約束 a〕: `AREKA_BALLOON_TIMEOUT_MS`・`configured_timeout_secs` に触れる（振る舞いは変えない）。
- 規模: S

#### 19. 自分の番号の scope を基準にした `\![move]` が「相対」になる枝にテストが無い
- 場所: `crates/areka-runtime/src/move_cue_place.rs:L214-L217`
- 種類: テストの抜け
- 問題（具体的な場合）: 檻は `--base=me` だけ（`move_cue_place_tests.rs:L530` の `relative_move_keeps_the_pattern_shift`）。`|| directive.base == MoveBase::Scope(scope)` を消すと `\1\![move,--base=1,--X=5,--Y=5]` が面の `move` のずれを 2 度数えるが、どのテストも赤にしない（`"--base=1"` の grep は 0 件）。
- 直し方: 同じテストを `--base=me` と `--base=1` の 2 通りで回す。
- 規模: S

#### 20. `rig_ab` の説明が事実と違い、作業プールを入れる行が呼び手 2 か所にある
- 場所: `crates/areka-session/src/shell_balloon_switch_session_abort_tests.rs:L365-L373`（doc「切替先の窓の準備の閉包の投函先つき」）・`L391-L393`・`L478-L480`
- 種類: 古い説明・要らないコード
- 問題（具体的な場合）: `rig_ab` も `lap_rig_on` も `WintfTaskPool` を入れない（入れるのは呼び手 2 か所）。doc を信じて 3 本目の呼び手が入れ忘れると、切替先の窓の準備が `task_pool_missing` で落ちる。
- 直し方: `rig_ab` の中で `WintfTaskPool::with_threads(1)` を 1 回入れ、呼び手の 3 行ずつを消す。
- 規模: S

#### 21. 「中断も起点は占有終端・起点は 1 つしかない」の説明が今の事実と違う
- 場所: `crates/areka-runtime/src/balloon_visibility_tests.rs:L389-L392`
- 種類: 古い説明
- 問題（具体的な場合）: 今は、止まった時刻が占有終端より早ければ止まった時刻が起点になる（`balloon_visibility_wait.rs:L105-L107`・`L208-L209`・`MeasurementOrigin` は 5 種）。中断で終わったトークで起点は占有終端にならない。
- 直し方: その段落を消す（止まった時刻の件は `balloon_visibility_talk_end_tests.rs` が持つ）。
- 規模: S

#### 22. 「同じ観測の列なら同じ判断」の 2 本が、ほかのテストより先に落ちる場合が無い
- 場所: `crates/areka-runtime/src/balloon_visibility_tests.rs:L260-L294`（時刻なし）・`L891-L960`（時刻あり）
- 種類: 要らないコード（落ちない表明）
- 問題（具体的な場合）: `decide` は状態・観測・引数だけで決まる関数で、毎回 `Default` の状態から始まるので 2 回の結果は常に一致する。並びの崩れは `output_order_is_scope_ascending_regardless_of_input_order`（L244-L258）が先に捕まえる。後ろの較正（L942-L959）は `suppression_holds_…`（L500-L561）の写し。
- 直し方: 時刻ありの 1 本（約 −70 行）を消す。時刻なしの 1 本（約 −35 行）も消してよい。消すなら `balloon_visibility_content_tests.rs:L22` の名指しも直す。
- 直さないと: 何も守らない約 100 行が、960 行のファイルを 1,000 行へ押す。
- 規模: S

#### 置き場と写し（痩せ）

#### 23. `areka-runtime` の純関数のテスト約 180 行が、1,000 行に近い `areka-session` のファイルに置かれている〔範囲の外〕
- 場所: `crates/areka-session/src/ghost_switch_tests.rs:L237-L418`（「特別な名前の解決（純粋）」の補助 4 つとテスト 8 本）。関連して `ghost_switch_roots_tests.rs:L65-L93`。
- 種類: 置き場（split）
- 問題（具体的な場合）: `areka_runtime::special_name::resolve_special_name` と `UnresolvedReason` を fs なしで試すテストだが、`areka-runtime/src/special_name.rs` には自前のテストが 0 本で、別の crate のこのファイルにだけある。ファイルは 982 行で、19 行を越えるテストを 1 本足すと番人が赤になる。
- 直し方: 純粋な段を `crates/areka-runtime/src/special_name_tests.rs`（新しいファイル）へ移す。`no_pick` は L883・911・951・978 の入口のテストでも使うので両方に置く。`ghost_switch_tests.rs` は約 800 行になる。新しいファイルは範囲の外（`areka-runtime` の範囲外は完了した spec の範囲）なので、要件 3.2 の「直しに要る変更」として扱うか議題にする（議題 8）。
- 規模: S

#### 24. 本体の可視性のテスト 2 本と `world_at` の写しを `areka-runtime` へ戻す〔件 4〕〔約束 b〕
- 場所: 本体 `crates/areka/src/runtime_tests/balloon_visibility_phase_tests.rs:L711-L783`（`decision_events_are_written_with_the_agreed_level_and_fields`）・`L786-L817`（`transition_of_a_show_that_did_not_take_is_not_written`）・`L145-L150`（`world_at`）と L44 の `use wintf::ecs::FrameTime;`。関連して `areka-runtime/src/balloon_visibility_phase_log_tests.rs:L5-L7`（本体の関数を同じ名前の `areka-runtime` のファイルの中にあるかのように指す）。
- 種類: 要らないコード（merge・reuse）
- 問題（具体的な場合）: 2 本が使うのは `areka_runtime::balloon_visibility` の型 5 つ・渡し口 `emit_visibility_logs`・`log_capture_kit`・`tracing::Level`・同じファイルの `visibility_lines` だけで、本体の項目は 0。`runtime-crate-split` は「番人の補助 `visibility_lines`」を理由に本体へ残した（その verification.md L330）が、字面の同じ `visibility_lines` は既に `balloon_visibility_phase_log_tests.rs:L16` にある。`world_at` の中身 3 行は支え `balloon_visibility_phase_test_support.rs:L37-L42` と 1 字ずつ同じ（違うのは doc と `pub` だけ）。
- 直し方: 2 本を `areka-runtime/src/balloon_visibility_phase_log_tests.rs` へ移し、`use super::super::{MeasurementDiscardReason, SuppressionKinds};` を 1 行足す（`emit_visibility_logs` ほかは `super::*` で私有のまま届く・dev-dependency の `log-capture-kit` と依存の `tracing` で足りる）。本体の `world_at` を消して `use super::test_support::{attach_headless, world_at};` にする。移した後、渡し口 `emit_visibility_logs`（支え L105-L109）は使い手が無くなるので消す。`log_tests.rs:L5-L7` の指し先が正しくなる。依存は足さない。
- 規模: S

#### 25. 可視性の相が読まない `BalloonWiring` を World に据えている（17 か所）
- 場所: 本体 `runtime_tests/balloon_visibility_phase_tests.rs:L38`・`L77-L82`・`L104-L108`、据える行 L214・L284・L337・L392・L421・L502・L571・L606・L691・L825。`balloon_visibility_phase_box_tests.rs:L28`・`L139`・`L196`、`balloon_visibility_phase_reappear_tests.rs:L19`・`L59`、`balloon_visibility_lifecycle_e2e_tests.rs:L37`・`L112`。
- 種類: 要らないコード（delete）・古い説明
- 問題（具体的な場合）: 相が World から読むのは `FrameTime`・`BalloonHover`・`ShellBoxHover`・窓の印・`SessionHooks` だけ。そもそも `areka-runtime` は `areka-input` に依存しない（`areka-runtime/Cargo.toml` に 0 件）ので、相は `BalloonWiring` を型の名でも引けない。`Harness::_choice_rx` は常に `None`（L107 だけ）で、説明「送信端を生かしておくためだけに保持する」は事実と違う。それなのに、この 2 つが 24 本のテストを本体（`areka-input`）に縛る唯一のつながりになっている。
- 直し方: 据える行と `use areka_input::balloon::…` を消す。`_choice_rx` の欄・説明・初期化と `ChoiceSelection` の取り込みも消す（約 −21 行）。
- 規模: S

#### 26. 空の `BootAssets` の手書きの写し 3 つ
- 場所: 本体 `runtime_tests/balloon_visibility_phase_tests.rs:L54-L71`・`balloon_visibility_lifecycle_e2e_tests.rs:L50-L67`・`session_tests/shell_balloon_switch_tests.rs:L89-L103`（`headless_wiring` の中）
- 種類: 要らないコード（reuse）
- 問題（具体的な場合）: `areka-runtime/src/frame_test_support.rs:L55-L96` の `synth_assets(&[])`（`test-support` で開いている）と欄が 1 つずつ同じ値。`BootAssets` に欄が 1 つ増えると、手書きの 3 か所を全部直すことになる。
- 直し方: 3 か所を `synth_assets(&[])` にする（`shell_balloon_switch_tests` は `headless_wiring_with(mpsc::channel().1, zero_clock())` に替えてもよい・入口は `RuntimeWiring` の有る無ししか見ない）。約 −55 行。
- 規模: S

#### 27. 移動の cue のテストの支えの写し 4 つ〔件 5〕
- 場所: `crates/areka-runtime/src/move_cue_apply_move_tests.rs:L16-L21`（`fake_handle`）・`L43-L55`（`attach_fake_handles`）、`move_cue_place_tests.rs:L82-L90`（同じループの書き込み）、`move_cue_apply_move_sticky_tests.rs:L200-L208`・`move_cue_place_tests.rs:L321-L329`（`SharedFakeIo`）
- 種類: 要らないコード（reuse）
- 問題（具体的な場合）: `crates/areka-placement/src/test_support.rs` の同じ働きの支え（`fake_handle`・`give_fake_handles`・`SharedFakeIo`）と別に保守されていて、片方を直しても他方は追わない。違いの事実は 3 節の件 5。
- 直し方: `areka_placement::test_support::{give_fake_handles, SharedFakeIo}` を直に使う。新しい支えのファイルは要らない（dev-dependency `areka-placement/test-support` は `crates/areka-runtime/Cargo.toml:62` に既にあり、`frame_test_support.rs:10` などが使う）。`SharedFakeIo(shared.clone())` は `SharedFakeIo::new(shared.clone())` にする。約 −39 行。
- 規模: S

#### 28. `boot_context` の写し〔件 8〕
- 場所: `crates/areka-session/src/ghost_switch_tests.rs:L84-L108`
- 種類: 要らないコード（merge）
- 問題（具体的な場合）: 本体 `menu/ghost_frame_tests.rs:43` に字面まで同じもの（`roots` の道の書き方だけ違う）、`install/desk_tests.rs:730` にバルーン名だけ違うものがある。ほかにも `BootContext` の組み立ては範囲の中外に 7 か所以上（3 節の件 8 の表）。片方だけ欄を足すと、もう片方は黙って古いままになる。
- 直し方の候補（決めない）: (a) `ghost_switch_test_support.rs` に `pub fn boot_context(root: &BasewareRoot, current: &str) -> BootContext` を置き、`ghost_switch_tests.rs` から引く（982→約 958 行・支えは 753→約 778 行）。(b) 既にある `reload::test_support::boot_context(temp, root)`（`reload_test_support.rs:17`・`audit-areka-app` の持ち物）に `current` を足して使う（呼び手 2 か所も直す＝〔範囲の外〕〔C26〕）。入口のテストは文脈のうち `roots` と `current.ghost.folder` しか読まないので、経路やバルーン名の違いで結果は変わらない。
- 規模: S

#### 29. `add_rpost_shell` の写し 2 つ〔件 9〕〔約束 b〕
- 場所: `crates/areka-session/src/shell_balloon_switch_session_lap_tests.rs:L57-L68`・本体 `session_tests/shell_balloon_switch_session_update_tests.rs:L53-L66`
- 種類: 要らないコード（merge）
- 何をする所か: 検体 `R_POST_and_KOMAINU` の `shell/master` を `add_shell_copy` で `<folder>` に写し、A の `shell/<folder>` へ複製する。どちらも `lap_rig_on` に `fn(&Path, &str)` として渡す。
- 問題（具体的な場合）: 9 行の同じ形が 2 つあり、検体の名・元のシェル・写し方を変えるときは両方を直す。写しを残す理由の文は本体の L55 にだけあり、`areka-session` の側に無い。
- 直し方の候補（決めない・事実は 3 節の件 9）: ⓐ 呼び手が検体を取って渡す形（`lap_rig_on` が「写す元のシェルの道」を受け、支えの側は std だけの `copy_tree` で写す）。`Cargo.toml` も依存も変えない。呼び手 2 か所には acquire＋`add_shell_copy` の約 4 行が残る。前例は `RigSample`（`ghost_switch_test_support.rs:208`）と本体 `switch_rig`（`sample_test_support.rs:43-55`）。ⓑ `sample-ghost-kit` に「シェルを別のゴーストへ名前を替えて写す」窓口を 1 つ足す（範囲の外・依存は増えない・`test-support` には入れない）。ⓒ crate をまたぐ `#[path]`（`crates/*/src` に前例 0・推さない）。ⓓ 1 つにしない場合、理由の文を `areka-session` の側にも足す。
- 〔約束 b〕: ⓐ は `test-support` で開けた `pub fn lap_rig_on` の形を変える（使い手は範囲の中の 2 本）。
- 規模: S

#### 30. `world.insert_resource(crate::BODY_SYSTEMS);` を手で足す 3 本〔件 10〕
- 場所: 本体 `crates/areka/src/session_tests/shell_balloon_switch_tests.rs:L288`・`L407`・`L463`（足場は同じファイルの `rig` L121-L164）
- 種類: 要らないコード（merge）・足場が本番と違う
- 問題（具体的な場合）: 足場 `rig` は本体の組を据えないので、組を据えないほかの 5 本では、要求のたびに `areka-session` の `error!(body_systems_missing)`（「結線の誤り」）が出ている（`body_systems.rs:L47-L57` の `#[cfg(not(test))]` は、本体のテストから見ると真＝`areka-session` は test でなく `test-support` で組まれる）。本番では組が必ず据わっている。新しいテストが ERROR を数えると、1 行足し忘れで赤になり、理由が見えない。
- 直し方: `rig` の `world.init_resource::<Schedules>();` の後ろに 1 行足し、3 本の 1 行を消す。表明は変わらない（3 節の件 10）。
- 規模: S

#### 31. 記録の件数を数える関数が範囲の中に 6 つ
- 場所: `ghost_switch_balloon_tests.rs:L179-L184` `count`・`ghost_switch_boot_event_tests.rs:L44-L49` `named`・`ghost_switch_reload_tests.rs:L32-L37` `named`・`ghost_switch_fallback_tests.rs:L87-L92` `count_event`（`pub(super)`）・`ghost_switch_tests.rs:L53`・`shell_balloon_switch_session_abort_tests.rs:L42-L47` `count`
- 種類: 要らないコード（reuse）
- 問題（具体的な場合）: 中身はどれも「`event` の欄が名と一致する記録の数」。数え方（level の条件など）を変えると 6 か所を直す。`ghost_switch_notice_tests` は既に `super::fallback_tests::count_event` を使う。
- 直し方: `ghost_switch` の兄弟 4 つを `use super::fallback_tests::count_event;` に替える（約 −24 行）。abort は `ghost_session` の木の下で `fallback_tests` が私有なので、残すか共通の支えへ移す。
- 規模: S

#### 32. 呼出列を文字列にする `ids` が 2 つあり、支えの `kinds` と同じ仕事をする
- 場所: `ghost_switch_boot_event_tests.rs:L25-L34`・`ghost_switch_reload_tests.rs:L21-L30`（支え `shell_balloon_switch_session_lap_test_support.rs:L311-L320` の `kinds`）
- 種類: 要らないコード（reuse）
- 問題（具体的な場合）: 3 つとも同じ写し方。違いは Unload の綴り（`Unload` と `UNLOAD`）だけで、`ids` を使う表明はどれも Unload の綴りと照らさない（全件を読んで確かめた）。
- 直し方: `kinds` を引き、`ids` の 2 つを消す（約 −18 行）。
- 規模: S

#### 33. `run_with_signals` の写し 2 つと手組みの観測が `Frame` と重なり、doc も誤り
- 場所: `crates/areka-runtime/src/balloon_visibility_user_break_tests.rs:L15-L37`（`run_with_signals`・`break_round`）・`L232-L235`・`balloon_visibility_box_tests.rs:L24-L34`
- 種類: 要らないコード（reuse）・古い説明
- 問題（具体的な場合）: doc（L17）は「`Frame` は会話の開始と占有終端しか組み立てられない」と書くが、今の `Frame` には `signal()`・`dragging()`・`ended_at()` がある（`balloon_visibility_test_support.rs:L142-L151`）。同じ組み立てが 3 通り残る。
- 直し方: 補助を消し、`Frame::new(e).signal(TalkLifecycleSignal::UserBreak)[.at(t)][.dragging()].run(&mut state)` に置き換える（約 −25 行）。
- 規模: S

### 2.3 できれば（34〜73）

| 番号 | 中身 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 34 | 解けない切替要求（該当なしの名）が 1 件ごとに目録を全部読み直す。`\![change,ghost,Nobody]` が 200 個並び、ゴーストが 40 体なら 1 回の Input の段で約 8,000 回 descript を読み、UI が一瞬固まる。シェル・バルーンも同じ（`shell_candidates`・`balloon_candidates`） | `ghost_switch.rs:L69-L90`・`L271`、`shell_balloon_switch.rs:L74-L95`・`L145-L152` | (a) 取り出しの系が目録を 1 回だけ読んで入口へ渡す（ghost_switch.rs は約 +8 行で 1,000 行の内側）。(b) 同じ取り出しの中で該当なしだった同じ名を読み飛ばす（warn の件数が変わる） | (b) は〔約束 c〕 |
| 35 | 観測の収集で、起きた画面更新ごとに小さい割り当てを重ねる（`scopes` の Vec と並べ替え・`box_scopes`・scope ごとに `ActorKey::from(scope.to_string())` を 2 回・`boxes` の Vec・`world.query_filtered` の QueryState を毎回作る・`visible` の Vec）。今の負荷では目立たない | `balloon_visibility_phase.rs:L87-L88`・`L252-L257`・`L286`・`L390`・`L382-L398`・`L446-L452`・`L459-L465`、`balloon_visibility_wait.rs:L138-L153` | `ActorKey` を scope ごとに 1 回だけ作って共用する（−2 行前後）。残りは計測で上位に出たときだけ | — |
| 36 | `hover_inject` が `pub mod` で、`pub` 6 項目とも crate の外で使われない（外の読み手は `crate_source!` の字面読み 2 か所だけ） | `crates/areka-runtime/src/lib.rs:L21`・`hover_inject.rs:L36・L52・L94・L116・L150・L183` | `pub(crate) mod hover_inject`（0 行） | 〔約束 b〕〔範囲の外〕（`lib.rs`） |
| 37 | `pub use geometry::{BaseposResolver, CanonDefaultBasepos}` の外の使い手が無い（`CanonDefaultBasepos` は本体 `spine_move_cue_tests.rs` の注釈に出るだけ） | `move_cue.rs:L28` | `pub use` を `use` にする。型シームそのものは `surfaces-basepos` のための先送りなので残す | 〔約束 b〕〔裁定 ⑵〕 |
| 38 | `pub(crate)` を私有か `pub(super)` に狭められる項目。`areka-runtime`: `DEFAULT_BALLOON_TIMEOUT_SECS`・`TimeoutSource`・`VisibilityTrigger::as_str`・`MeasurementDiscardReason::as_str`・`MeasurementOrigin::as_str`・`TalkEnd`・`VisibilityWake`・`visibility_wake` は私有、`parse_timeout_ms` は `pub(super)`。`areka-session`: `end_switch`・`request_ghost_switch_with`・`switch_to_default`・`drain_switch_requests` は私有。使い手 0 で消せるものは 0 | 3 節の件 3 の「pub(crate) の表」 | 可視性だけ狭める（0 行） | — |
| 39 | 足場の `pub` のうち範囲の外の使い手が 0 のもの（`add_balloon_copy`・`plant_last_balloon`） | `ghost_switch_test_support.rs:L383`・`L397` | `pub(crate)` にする。`Stall`・`FakeShiori::ScriptedStalled` は pub の enum の変種が名指すので今のまま | 〔約束 b〕〔件 2〕 |
| 40 | 本体に置いた可視性の相のテスト 27 本は、25 番の後なら `areka-runtime` へ戻せる（`phase_tests` の残り 12・`box_tests` 4・`reappear_tests` 10・`lifecycle_e2e` 1）。戻すと渡し口 6 つ（`collect_observations`・`observe_scope_boxes`・`per_scope`・`display_end`・`observation_failure_logged_runtime`・`phase::test_support`）と `test_open` の 3 つと 4.2 ⑴ の多くが使い手を失い閉じられる。`notice_tests`（5 本・`areka_session` を使う）と代わりの親 2 つは本体に残る | 本体 `runtime_tests/balloon_visibility_phase_tests.rs`・`_box_tests.rs`・`_reappear_tests.rs`・`_lifecycle_e2e_tests.rs` | 25 番の後、関数を `areka-runtime` の同じ名のテストのファイルへ戻し、`box_tests` L29 の `areka_input::shell_box::ShellBoxHover` を `areka_runtime::shell_box_hover::ShellBoxHover` に替える。戻し先の `balloon_visibility_phase_tests.rs` は 43→約 860 行 | 〔約束 b〕〔裁定 ⑴〕（`runtime-crate-split` の割り方の表と違う置き場。規則 D8「本体を引かない関数は戻す」には合う）・M |
| 41 | `.timeout(TIMEOUT)` 54 か所が何もしない（`Frame::new` の既定も `TIMEOUT` も `DEFAULT_BALLOON_TIMEOUT_SECS`）。しかも `areka-runtime` のテストは既定値しか渡さないので、`decide` が引数 `timeout_secs` を無視しても緑（既定でない値は本体 `balloon_visibility_phase_notice_tests.rs:25` の `WAIT = 2.0` だけ） | `balloon_visibility_tests.rs` 43 か所（L362〜L932）・`balloon_visibility_box_tests.rs` 9 か所・`balloon_visibility_user_break_tests.rs:L191・L216`・支え `balloon_visibility_timeout_suppression_tests.rs:L309`・doc `balloon_visibility_tests.rs:L351`・`_timeout_suppression_tests.rs:L78-L79` | ⑴ 何もしない呼び出しと doc を消す（約 −54 行）。⑵ 表の `TIMEOUT` を既定でない値（例 20.0・`LATE + 3.0` より大）にして引数を守る。⑴⑵ は一緒にできる | 〔約束 a〕（振る舞いは変えない） |
| 42 | テストの補助の写し（`boxed` 2・`hide_timed_out` 3・`show`／`shown_log` 2） | `balloon_visibility_box_tests.rs:L16-L22`・`_choice_wait_tests.rs:L39-L45`・`_tests.rs:L26-L46`・`L57-L63`・`_talk_end_tests.rs:L12-L18`・`_choice_wait_tests.rs:L83-L88`・`_user_break_tests.rs:L39-L51` | `balloon_visibility_test_support.rs` の `hovered` の隣へ寄せる（約 −30 行）。公開の面を増やしたくなければ `#[cfg(test)]` だけの私有の mod に置く | 〔約束 b〕（支えへ足す場合） |
| 43 | 「選択肢を出した」の印のテストが、選択肢のテストの 2 点と重なる | `balloon_visibility_talk_end_tests.rs:L437-L458`（重なる相手 `_choice_wait_tests.rs:L56-L59`・`L204`） | talk_end 側の 1 本を消す（−22 行） | — |
| 44 | `apply_move_directive` の同じ確かめを 2 重に持つテスト 4 本（同じファイルの pipeline の 2 本が同じ検体・同じ表明を全部持つ） | `move_cue_apply_move_tests.rs:L94-L163`・`L165-L196`（相手 L260・L327） | 単体の 4 本を消す（約 −95 行）か、pipeline の 2 本を消す（決めない） | — |
| 45 | 正準のキャリア cue を手で組む所が 4 か所（隣のテストは `crate::consumer_ledger::test_support::actor_carrier_cue` を使う） | `move_cue_move_severity_log_tests.rs:L113-L118`・`L145-L153`・`L186-L191`・`move_cue_apply_move_tests.rs:L243-L251` | `actor_carrier_cue(actor, name, tokens)` に置き換える（約 −25 行） | — |
| 46 | `MoveCueSink` の送出が失敗する枝（受信端が閉じている）にテストが無い。`ChangeCueSink` には `dropped_receiver_is_logged_and_does_not_panic` がある | `move_cue.rs:L342-L346` | `rx` を drop してから move を 1 件流し、panic せず `warn!` が 1 件のテストを `move_cue_move_sink_tests.rs` に足す | — |
| 47 | 「どのイベントで起きるか」を見る 3 本が、待ちの条件 `exit_requested() || …` で終了した回を見逃す | `ghost_switch_boot_event_tests.rs:L141-L164`・`ghost_switch_reload_tests.rs:L144-L161`・`L166-L196` | shutdown の前に `assert!(!rig.exit_requested())` を 1 行ずつ | — |
| 48 | 背景のスレッドが COM を初期化することの檻が走る順に左右される（同じバイナリのテストが MTA を初期化したまま終わると暗黙の MTA で通る・doc が自分で書く） | `switch_assets_tests.rs:L34-L40`（初期化したまま終わるのは L122・L232・L270） | `spawn_build_worker` へ `CoGetApartmentType` が暗黙の MTA なら panic する閉包を注入するテストを 1 本足す | — |
| 49 | 読み直しのテストで同じ組み立てが繰り返される（`running_a` が `running_a_with` と同じ・ghost の `ReloadRequest` 4 回・作り口を差し替えて根を壊す 12〜13 行が 3 回・`reload_failed` の理由を集める 5 行が 4 回） | `ghost_switch_reload_tests.rs:L49-L57`・`L93-L99`・`L537-L543`・`L601-L607`・`L667-L673`・`L371-L383`・`L428-L438`・`L586-L598`・`L190-L195`・`L400-L404`・`L456-L460`・`L633-L637` | `reload_ghost(rig)`・`break_boot(rig, ..)`・`reload_reasons(events)` を足して置き換える（約 −55 行） | — |
| 50 | descript に `balloon,` を足す処理がテストの中にもう 1 度ある | `ghost_switch_balloon_tests.rs:L108-L116`（`name_balloon_in_b` L167-L177 と同じ） | `name_balloon_in_b` を呼ぶ（−8 行） | — |
| 51 | 記憶の書き手に柵を掛けて読む関数が 2 つ（7 行が同じ） | `shell_balloon_switch_session_lap_tests.rs:L144-L153`・`_balloon_tests.rs:L166-L175` | `fence(rig)` を 1 つにする（約 −7 行） | — |
| 52 | `copy_tree` の写し（足場の私有・skip つきと lap の pub・skip なし） | `ghost_switch_test_support.rs:L720-L736`・`shell_balloon_switch_session_lap_test_support.rs:L173-L187` | 足場の方を `pub` にし lap の方を消す（呼び手は `None` を渡す・約 −15 行）。29 番の ⓐ と一緒に | 〔約束 b〕 |
| 53 | Tick を送る処理と `TICK_EVERY` の写し。合成の時計も `SwitchRig.talk_clock_ms` と `LapRig.clock_ms` の 2 つ。今は 1 つのテストで混ぜていないが、混ぜると dispatcher へ後戻りの時刻が届く | `shell_balloon_switch_session_lap_test_support.rs:L39`・`L256-L270`、`ghost_switch_test_support.rs:L182`・`L591-L603` | lap は `crate::ghost_switch_test_support::TICK_EVERY` を引き、`SwitchRig` の「時刻を渡して 1 つ送る」私有の口を使う（約 −10 行） | — |
| 54 | `plant_last_balloon` が `plant_boot_record` と同じ道を組み直し、書いたファイルを上書きする | `ghost_switch_test_support.rs:L352-L367`・`L397-L412` | 道を組む補助を 1 つにし、1 回だけ書く（約 −6 行） | — |
| 55 | 文脈なしの warn の写し（同じ event・reason・文）。`accept_switch` は `reload.rs` からも直に呼ばれるので確かめそのものは要る | `ghost_switch.rs:L263-L270`・`L326-L333` | 私有の関数 1 つにまとめる（約 −6 行） | — |
| 56 | `folder_of` が `resolved_folder` の写し | `ghost_switch_roots_tests.rs:L34-L36`・`ghost_switch_tests.rs:L41-L43` | 片方を `pub(super)` にして引く（23 番で両方を移すなら不要） | — |
| 57 | メニューのテスト 2 本で、受理の記録と印の取り出しが同じ 16 行（2 つ目の `descript.txt` を書く所も 3 回） | 本体 `session_tests/shell_balloon_switch_session_tests.rs:L169-L184`・`L246-L261`・`L58-L65`・`L150-L156`・`L225-L231` | `requested(events)` と descript を書く補助を 1 つずつ（約 −15 行） | — |
| 58 | 「隣の `shell_balloon_resolve.rs`」が古い（crate 分けの後は `areka_runtime::shell_balloon_resolve`） | `shell_balloon_switch.rs:L8-L9` | crate の道で書き直す | — |
| 59 | 取り出しの系の説明に、先頭の `switch_copy::poll`（写しが終わると `switch_to` でゴーストを降ろして起こす）が書かれていない | `ghost_switch.rs:L66-L70`（登録の説明 L56-L59 も） | 説明に 1 行足す（927→928 行） | — |
| 60 | `// SAFETY:` が守る条件でなく目的を書く（`CoInitializeEx` を今のスレッドで呼ぶ・`CoUninitialize` と対にしない理由が無い） | `ghost_switch_test_support.rs:L243-L246` | 守る条件を 1 行で書く | — |
| 61 | モジュールの名を言い直すだけのコメント（「足場の待ちの檻。」「時計の先行の檻。」） | `ghost_switch_test_support.rs:L745`・`L750` | 2 行消す | — |
| 62 | `SwitchBuildError` の説明「（`BootWiringError`／`PlacementError` の写し）」に 3 つ目の枝 `ShellUndecodable` が入らない | `switch_assets.rs:L90`（枝は L99-L107） | 「資産づくり・配置の値の読み・シェルの絵の復号の失敗」にする | — |
| 63 | 見出しの `[`super::boot_ghost`]` が無い関数を指し（本物は `areka_session::ghost_session::boot_ghost`）、「期限（20 秒）で赤」も違う（足場の待ちは進みの目印つき・`SPIN_WAIT` 30 秒か `WAIT_CAP` 300 秒） | 本体 `session_tests/shell_balloon_switch_session_tests.rs:L3-L4`・`L17-L19` | 道を直し、「（20 秒）」を消す | — |
| 64 | 「判断中核の私有状態を読むため親の内側に置く」は `areka-runtime` にいた頃の理由（今は渡し口 `display_end` で読む）。「本番 `wire_runtime` 手順 4/5 と同型」も `lib.rs:L382-L403` の手順の番号と合わない | 本体 `runtime_tests/balloon_visibility.rs:L9-L10`・`balloon_visibility_lifecycle_e2e_tests.rs:L85` | L10 の後ろの文を消し、L85 は手順の番号を消す | — |
| 65 | 本体の `box_tests` の見出しが、このファイルに無い「発行」を書く（発行の 4 本は `areka-runtime` 側にある） | 本体 `runtime_tests/balloon_visibility_phase_box_tests.rs:L1-L12`・`balloon_visibility_phase.rs:L33` | 観測だけに直す（40 番をすればファイルごと消える） | — |
| 66 | hover_inject のモジュール doc が呼び手を「frame.rs の text phase」と書く（本物は `frame/scale_text.rs` の `run_text_phase` L252） | `hover_inject.rs:L11-L12`・`L19`・`L178` | 呼び手の場所を直す | — |
| 67 | `balloon_visibility.rs` の説明 3 か所が今の事実と違う: ㋐「非公開フィールド」だが `prev_visible` は `pub`、㋑「`text_scale_warned`（`frame/wiring.rs:94`）」は今 L102、㋒「判断中核だけで既に 700 行を超えており」だがファイルは 549 行（判断は 2 ファイルへ分かれた） | `balloon_visibility.rs:L398-L400`・`L407-L408`・`L498` | ㋐「非公開」を消す ㋑ 行番号を消す ㋒ 文を消すか「3 ファイルと同居させると 1,000 行を越える」にする | — |
| 68 | 支えの見出し「…組み立てだけを持つ」だが渡し口 4 つがある。ログのテストの見出し「起点の採り方（3 つの値）」だが `MeasurementOrigin` は 5 種で、ファイルも 5 種を見る | `balloon_visibility_phase_test_support.rs:L1-L6`・`balloon_visibility_phase_log_tests.rs:L4` | 「だけ」「3 つの値」を消す | — |
| 69 | 外からの非表示のテストを名指す行が、同じ crate の同じ名の別のファイル（43 行）を指してしまう（本物は本体 `runtime_tests/balloon_visibility_phase_tests.rs:333`） | `balloon_visibility_content_tests.rs:L30-L32` | `crates/areka/src/runtime_tests/` から始まる道で書く（40 番をすれば指し先が戻る） | — |
| 70 | 説明は「不可視のまま」だが、最初の観測は `seen(4, true)`（見えている） | `balloon_visibility_tests.rs:L199-L200` | 観測に合わせて書き直す | — |
| 71 | 変更履歴の言い方（「既存のテスト群」「箱を足す前と同じく」「既存の理由」「足しも消しもしない」）と、外れた名指し（「明示の指示の分岐は `balloon_visibility_tests.rs` 群が持つ」が、`Explicit` の検出は本体の phase のテストにしかない）。支えの doc「回数の変わらない観測の列は今と同じ答えを返す」も同じ型 | `balloon_visibility_box_tests.rs:L7`・`L80`・`balloon_visibility_user_break_tests.rs:L6-L7`・`L181-L183`・`balloon_visibility_test_support.rs:L20` | 履歴の言い方を外し、名指しを直す | — |
| 72 | 手順を語る注釈（「1)〜4)」の番号つきの手順）と変更履歴の見出し（「最初の実消費者」）。L231-L235 の「execute 宛も debug——warn は InstallCueSink が持つ」はコードから読めない理由なので残す | `move_cue.rs:L179-L181`・`L230-L235`・`L254-L259`・`L307`・`L312` | 番号つきの語りと「最初の実消費者」を消す | — |
| 73 | 「骨格は `readme_cue.rs` と同型」の場所が古い（今は `crates/areka-session/src/readme/readme_cue.rs`） | `change_cue.rs:L4` | 括弧を消すか crate の道を書く（`//!` の 1 行目は残す） | — |

### 判定と減らせる量

- 判定: 範囲は総じて健全。本番のコードの必ず直すは 1 件（1 番・負荷）。直す順の頭は、1 番（毎画面更新の予約）・2 番（scope の切り捨て・件 6）・3 番（インストール中の断り）・16〜19 番（空振りする檻と抜け）。
- 減らせる量の見込み: テストの写しと重なり（22・25・26・27・28・31・32・33・41〜45・49〜57 番ほか）で、**約 −650〜−750 行**。40 番（27 本の移し）は差し引き約 0 で、`areka-runtime` の渡し口と公開を約 −45 行。依存は増えも減りもしない（−0）。

---

## 3. 申し送りの件（要件 5・1.7）

行番号は今の木（HEAD `9604793a`＝main `1b3ede54`）。

| 件 | 要件 | 出どころ | 今も当てはまるか・今の場所 | 指摘 |
|---|---|---|---|---|
| 1 | 5.1 | `runtime-module-untangle` | 当てはまる。`shell_balloon_switch.rs:L37-L40`（brief の 37〜39 は今 L37-L40 の 4 行）。本番の呼び手で 5 項目を出し直しの道で引くものは 0 | 4 |
| 2 | 5.2 | `session-crate-split` | 当てはまる。cfg は L42、出し直しは L43。`SwapFinish` の使い手は本体のテスト 2 か所だけ。`ghost_switch_test_support` は範囲の外の約 30 本（本体）と 9 本（`areka-session`）が使う＝モジュールの開け方は要る。中で範囲の外の使い手 0 の `pub` は 2 つ | 5・39 |
| 3 | 5.3 | `runtime-crate-split` ⑶ | 下の「件 3 の数」。今すぐ戻せるのは `settle_box_hover` 1 つ | 7・36・38 |
| 4 | 5.4 | `runtime-crate-split` ⑶・`audit-emo2-boot-spine` ウ | 当てはまる。2 本は今 L711-L783・L786-L817。`world_at` は L145-L150（支えは `balloon_visibility_phase_test_support.rs:L37-L42`・中身 3 行が 1 字ずつ同じ）。2 本は本体の項目を 0 個使い、依存を足さずに戻せる | 24 |
| 5 | 5.5 | `audit-emo2-boot-frame` キ・棚卸㊸ 件 10 | 当てはまる。下の「件 5 の突き合わせ」 | 27 |
| 6 | 5.6 | `audit-emo2-boot-frame` ク | 当てはまる。`balloon_visibility_phase.rs:L464`。`window_scope` は `target_map.rs:L48`（`pub fn window_scope(scope: usize) -> Option<u32>`・`u32::try_from` の後 `<= MAX_SCOPE`・外れるたび `warn!` 1 行・抑えなし）。呼び手は `frame/` の 9 か所 | 2 |
| 7 | 5.7 | `audit-emo2-boot-frame` コ | 当てはまる。`move_cue_apply_move_sticky_tests.rs:L255`・`move_cue_place_tests.rs:L386`（brief の行のまま）。`move_cue_apply_move_tests.rs` に同じ型は 0（`audit-emo2-boot-frame` の 6 節コが挙げた `L256` は今は無い）。範囲の外に同じ型が 1 つ（6 節） | 15 |
| 8 | 5.8 | 棚卸㊶ シ | 当てはまる。`ghost_switch_tests.rs:L84-L108`。下の「件 8 の表」 | 28 |
| 9 | 5.9 | `session-crate-split` | 当てはまる。`shell_balloon_switch_session_lap_tests.rs:L57-L68`・本体 `session_tests/shell_balloon_switch_session_update_tests.rs:L53-L66`。下の「件 9 の事実」 | 29・52 |
| 10 | 5.10 | `session-crate-split` D13 | 当てはまる。L288・L407（for の中）・L463。下の「件 10 の事実」 | 30 |
| 12 | 5.12 | `audit-emo2-boot-spine` オ | 済んだまま。`balloon_visibility_phase_zorder_chain_tests.rs:L38` が `use crate::source_text_test_support::code_only;`（L262 で呼ぶ）。ファイルに写しは無い | — |

### 件 3 の数（要件 5.3・問い 2）

数え方は問い 2 の仮定（本体のテストだけが使う項目は使い手に数える）に従った。`crate_source!`・`include_str!` で字面を読むだけの所は使い手に数えていない。

- **2.5 の 26 件**（`pub(crate)` → `pub`）: どれも開いたままが要る。① `areka-runtime` の外の本番に使い手があるもの 23（`BalloonHover` の 3・`ChangeRequestRaw`・`ChangeCueSink`・`::new`・`ShellBoxHover` の 5・`SkinKind`・`SkinRequestRaw`・`SkinSpec`・`SkinOrigin`・`SkinRequest`・`SkinVerdict`・`SkinSwitchInFlight`・`SkinSwitchStage`・`skin_ref_name`・`skin_ref_path`・`SwitchBuildRequest`・`spawn_switch_build`）、② 本体のテストだけだが公開の型の欄か署名に出るので要るもの 3（`SwapFinish`＝`SkinSwitchStage::Committed` の欄・`SwapBuilt`・`SwitchBuildError`＝`spawn_switch_build` の戻りと `SkinSwitchStage::Waiting`）。戻せる 0、消せる 0。
  - 事実のずれ: brief の「`switch_assets.rs` 4」は `runtime-crate-split/verification.md:L187-L190` の 4 件。分ける前の `crates/areka/src/runtime/switch_assets.rs`（`7f41b53e6^`）では `SwapPayload`・`SwapSlot` も `pub(crate)` で、今は 6 件が `pub`。`SwapPayload` は `RuntimeWiring` の `pub swap_slot`（`frame/wiring.rs:124`）の型なので要る。`SwapSlot` は本体のテストだけが使う別名（`pub(crate)` にすると本体のテストが `Arc<Mutex<Option<SwapPayload>>>` と綴る）。
- **4.2 ⑴〜⑷**: `areka-runtime` の外の本番の使い手は全項目で 0。
  - 今すぐ戻せる: `settle_box_hover`（7 番）。
  - 24 番（2 本の戻し）の後に使い手を失う: 渡し口 `emit_visibility_logs`（消せる）。型の `MeasurementDiscardReason`・`SuppressionKinds`・`MeasurementOrigin` は `VisibilityLogEvent` の腕の署名に出るので開いたまま。
  - 40 番（27 本の戻し）の後に使い手を失う: `decide`・`VisibilityTrigger`・`VisibilityAction`・`VisibilityDecision::{actions, logs}`・`ScopeObservation` の欄 4 つ・`ScopeVisibility`・`GlyphObservation`・`VisibilityObservations`・`forget_scope`・支えの `per_scope`・`display_end`・`observation_failure_logged_runtime`・`phase::test_support`（`attach_headless`・`world_at`・`runtime`）・⑵ の `IssueOutcome`・`issue_actions`・`clear_hover_residency`・⑷ の `collect_observations`・`observe_scope_boxes`。署名の鎖で残るものは設計で数え直す。
  - 40 番の後も残る: `notice_tests`（本体）が使う `BalloonVisibilityState`・`VisibilityDecision` の型と `timeout_notice`・`ScopeObservation` の型・`Frame`・`TALK_ID`・`seen`・`notify_timeout`・⑶ の印。本体 `frame_*` の 2 本が使う `configured_timeout_secs`。`areka-input` のテストが使う `run_balloon_visibility_phase`・`is_balloon_hovered`。`BalloonVisibilityState` は `RuntimeWiring::balloon_visibility`（`frame/wiring.rs:83`・4.2 の「RuntimeWiring の欄 11」＝この spec の持ち物の外）が `pub` のあいだ戻せない。
- **範囲の本番の `pub(crate)`**（11 件）: 私有にできる 8・`pub(super)` で足りる 1（`parse_timeout_ms`）・要る 1（`BalloonVisibilityState::talk_id`＝`frame/wiring.rs:226`）・本番は同じモジュールだけで別のモジュールのテスト（`placement_tests/reset_balloon_pos_tests.rs:25,383`）のために要る 1（`build_swap`）。消せる 0（38 番）。
- **`areka-session` の 2 ファイル**: `session-crate-split` が開けた分は、その verification の 4.2 節が指す一覧 `4.2-surface.txt`（P＝本体の本番が使う・T＝本体のテストだけ）で特定した（この worktree には無く、`C:\home\maz\git\areka\.claude\worktrees\kiro-start-input-crate-split-16b18a\target\session-crate-split\4.2-surface.txt` にあった）。T の `pub`（`LastInstalledGhost`・`SwitchEnd`・`on_notice`・`switch_to`・`SwitchRx`・`wire_switch_rx`・`register_switch_drain`・`discard_for_exit`・`SwapFinish`）はどれも本体のテストに使い手があり、`SwapFinish` の出し直し（5 番）と `NameResolution`・`resolve_special_name` の出し直し（6 番）のほかは閉じられない。私有にできる `pub(crate)` は 4 つ（38 番）。

### 件 5 の突き合わせ

- `attach_fake_handles`（`move_cue_apply_move_tests.rs:L44-L55`）と `give_fake_handles`（`areka-placement/src/test_support.rs:L88-L96`）: 回る順は同じ（`scopes()` ごとに char → balloon）。HWND の値が違う（give は `0x100 + i`、attach は 0x100 から 0x10 刻み）。窓が無いときは give が `expect(..)`、attach が `unwrap()`。偽の HWND の作りは同じ。`move_cue_place_tests.rs:L82-L90` のループは 0x110 から 0x10 刻み。束の中に HWND の数値を読む表明は 0 件で、窓ごとに値が別なら足りる＝どれに寄せても動きは同じ。
- `SharedFakeIo` の 2 つの写しは字面まで同一（`SharedFakeIo(Arc<FakePersistIo>)`・`read`／`commit` を渡すだけ）。placement 版（`test_support.rs:L46-L70`）は欄 `store` と `commits: Arc<AtomicUsize>`・`new(store)`・`commit_count()` を持ち、`commit` で数を足す。束のテストは数を使わないので `SharedFakeIo::new(..)` で同じに動く。
- 置き場: 新しい支えは要らない。placement の支えを直に使える（dev-dependency は `areka-runtime/Cargo.toml:62`）。

### 件 8 の表（`BootContext` を組む所）

| 場所 | 置き場・helper | 今のゴースト | ゴーストの経路 | バルーン | argv |
|---|---|---|---|---|---|
| `areka-session/src/ghost_switch_tests.rs:84`（範囲の中） | root/profile・root/helper.exe | 引数 | Default | StayseeBalloon | false |
| `ghost_switch_test_support.rs:454`（範囲の中・`boot_with_balloon` の中で直に組む） | `app_dir()`・作り口の helper_exe | 引数 | Memory | 引数（既定は emo2-kakukaku・Companion） | false |
| `ghost_switch_fallback_tests.rs:41`（範囲の中・`switching_world` の中で直に組む） | root/profile・root/helper.exe | A 固定 | Memory | StayseeBalloon | false |
| `areka-session/src/reload_test_support.rs:17`（`reload::test_support`・pub・test-support） | temp/profile・temp/helper.exe | A 固定 | Default | kaku | false |
| 本体 `menu/ghost_frame_tests.rs:43`（`pub(crate)`・`balloon_frame_tests`・`shell_frame_tests` も引く） | `ghost_switch_tests` と字面まで同じ | | | | |
| 本体 `install/desk_tests.rs:730`（`desk_exit_tests` も `super::*` で引く） | 同じ | 引数 | Default | B | false |
| `areka-session/src/readme/opener_submit_tests.rs:31` | 同じ | g 固定 | Memory | b | false |
| 本体 `runtime_tests/frame/switch_finish_tests.rs:147` | 相対の skins | A | Default | kaku（Argv・folder なし） | true |
| 本体 `runtime_tests/frame/window_state_tests.rs:31` | 相対の skins | pasta | Default | kaku | false |
| 本体 `session_tests/reload_tests.rs:52` | `reload::test_support::boot_context` を引く（写しではない） | | | | |

- ほかに `insert_resource(BootContext {` と直に組む所が本体に約 15 か所ある。brief（棚卸㊶ シ）の「5 か所」は今は 9 か所の補助と約 15 か所の直の組み立て。
- 範囲をまたぐまとめは Out of scope（問い 5）。28 番の (a) の形なら、ほかの範囲の写し（`menu/ghost_frame_tests.rs:43` は字面まで同じ）は置き換えるだけで使える＝持ち主（`audit-areka-app`）へ渡す材料。

### 件 9 の事実（問い 3）

- 1 字ずつの突き合わせ（session L57-L68／本体 L53-L66）: doc の 2 行は同じ。本体だけ L55 に `//` の 1 行（「集めた層の `shell_balloon_switch_session_lap_tests.rs` にも同じ形がある: 検体の窓口は `test-support` に入れられない。」＝今の事実に合う）。session は `use sample_ghost_kit::SampleRoot;`（L17・使うのはここだけ）で 1 行、本体は完全な道で 2 行に折れる。それ以外（`add_shell_copy("master", folder, folder)`・`.expect("2 つ目のシェルを写す")`・`copy_tree(..)` の引数）は字面まで同じ。`copy_tree` はどちらも `lap_test_support.rs:L173` の同じ関数。

| 使うもの | 定義 | `areka-session` のテスト | 本体のテスト | `areka-session` の `test-support` の中 |
|---|---|---|---|---|
| `SampleRoot::acquire`・`add_shell_copy`・`folder` | `sample-ghost-kit` | ○（dev-dependency） | ○（dev-dependency） | ×（`[features] test-support` に入らない＝決め D8 ⑵） |
| `add_shell_copy` の中の名の書き換え（`named_descript`・バイト単位・Shift_JIS を保つ） | `sample-ghost-kit` の私有 | 窓口経由だけ | 窓口経由だけ | ×（検体 R_POST は Shift_JIS＝`areka-nar/src/manifest_tests.rs:107`。支えで `read_to_string` して書き換える形は使えない） |
| `copy_tree`・`lap_rig_on` | `areka-session` の `shell_balloon_switch_session_lap_test_support`（`cfg(any(test, feature = "test-support"))`・std だけ） | ○ | ○ | ○ |

- 結論: `Cargo.toml` を変えず依存も足さずに成り立つ形は ⓐ（呼び手が検体を取り、写す元のシェルの道を `lap_rig_on` へ渡す）。写す手順は std だけの支えに 1 つになる。acquire＋`add_shell_copy` の約 4 行は呼び手 2 か所に残る（検体の窓口を `test-support` に入れない決めの帰結）。関数を完全に 1 つにするには ⓑ（`sample-ghost-kit` に窓口を足す・範囲の外・依存は増えない・`test-support` に入れない＝D8 ⑵ に反しない）か ⓒ（crate をまたぐ `#[path]`・前例 0）しかない。

### 件 10 の事実（問い 4）

- 表明と本体の組から要るもの:
  - L288 `refusals_in_order_leave_one_warning_each_and_send_nothing`: 3 段目の「更新中」は、本物の `update_busy`＝`update::desk::is_busy` が `insert_running_desk_for_test` の `UpdateDesk` を見て真を返すことを要る（`NO_BODY` の `update_busy` は常に偽）。
  - L407 `acceptance_starts_one_wait_marked_only_for_menu_and_raise_event_shells`: warn＋error が 0 件。組は `body_systems_missing` の `error!` を出さないためだけに要る。
  - L463 `send_failure_leaves_one_error_and_no_marker`: error が 1 件だけ。同じく `body_systems_missing` を出さないためだけ。
- 入口 `request_skin_switch` が問うのは `update_busy` だけ（`shell_balloon_switch.rs:L132`）。`install_busy` を問うのは `reload.rs:L314` だけ。`session-crate-split` の design.md L705 の「`update_busy`・`install_busy` の照会が真のときの断りを見る」は、このファイルについては `update_busy` だけが当たる（3 番の指摘の元）。
- 足場: このファイルは自分の `rig`（L121-L164）を使い、本体の `switch_rig`（`sample_test_support.rs:L43-L55`→`SwitchRig::with`→`register_systems` が組を据える）は使わない。
- `rig` の側で組を据えたとき、組を要らない 5 本（`missing_context_refuses_with_one_warning`・`balloon_switch_targets_the_ghost_inner_balloon`・`drain_hands_every_script_request_to_the_entry`・`exit_cleanup_drops_the_marker_with_one_info`・World を使わない `reference_name_…`）: `UpdateDesk` が無ければ `is_busy` は記録を出さず偽を返すので判定は変わらない。変わるのは、今それぞれの要求で出ている `error!(body_systems_missing)` が出なくなることだけ。5 本とも ERROR の数も行の総数も数えていない＝表明は変わらない。
- 範囲の外で同じ 1 行を手で足す形: 5 本（`install/desk_tests.rs:L385`・`L554`、`update/desk_resolve_tests.rs:L558`、`session_tests/reload_tests.rs:L142`・`L199`）。範囲の中の 3 本と合わせて木全体で 8 本（ロードマップの C26 の約束 ⑴ の「8 本」と合う）。直さない。

---

## 4. 外への約束に響く指摘（要件 1.6）

| 指摘 | 約束 | 響く読み手 | 扱いの案 |
|---|---|---|---|
| 1 | (a) は変わらない（時間切れの判断は同じ・起き方だけ）。字面の検査（`tick_gate_config_producers_tests.rs` の `REARM`・`arm_deadline(`）は残る | 本体の字面の検査 | 直す |
| 3 | (a) areka の中の断りの集合が 1 つ増える（ukadoc の正典の外・SSP 互換の振る舞いではない）。(c) warn の行が 1 つ増える | 字面で読む所は 0（新しい語） | 議題 3（Research Needed の結果しだい） |
| 4・5・6・7・36・37・38・39 | (b) 公開の面が減る | 本体の `menu/`・`session_tests/`・`runtime_tests/frame/switch*`・`install/`（〔C26〕）・`areka-session/src/reload_test_support.rs` | 呼び手を同じ変更で直す（要件 7.6）・相手と申し合わせる（要件 7.1） |
| 10 | (a) 文書だけ（`doc/COMPAT_ARCHITECTURE.md` §8 の 177・407 行に 1 文足す）。コードは変えない | 文書の読み手 | 議題 6（要件 7.5 は文を足すことを許さない） |
| 11・12 | (b) 公開の `///` の文だけ | — | 直す |
| 13 | (c) ログの文が変わる（`move_cue.rs` の 5 行・`hover_inject.rs` の 2 行）。語・target・欄は変えない | 字面で読む所は 0（grep） | 直す |
| 16 | (c) 案 ⑵ なら本番の 2 行に `event=` が増える。案 ⑴ は行を変えない | 読み手はこの 2 本だけ | 直す（案は設計で） |
| 18・41 | (a) `AREKA_BALLOON_TIMEOUT_MS`・`DEFAULT_BALLOON_TIMEOUT_SECS`・`configured_timeout_secs` に触れる（振る舞いは変えない） | COMPAT:179 の行は字面を変えない | 直す |
| 24・40 | (b) `test-support` の渡し口と `test_open` が減る | 本体の `runtime_tests/balloon_visibility*`（範囲の中） | 戻すと同じ変更で閉じる |
| 29 | (b) `test-support` の `pub fn lap_rig_on` の形が変わる（ⓐ） | 範囲の中の 2 本だけ | 直す |
| 34 | (c) 案 (b) なら warn の件数が変わる | 字面で読む所は 0 | 案 (a) なら触れない |

- ゴーストと SSP 互換の振る舞い（(a)）が変わる指摘は 0 件。3 番は areka の中の断りで、ukadoc の正典（`\![change,shell/balloon,…]`）は「インストール中」を語らない。1 番は起き方だけで判断は変わらない。
- ukadoc で確かめた正典（束の読み手が引いた）: `\![set,balloontimeout,時間]` の「0または負数でタイムアウトなし」は `talk_lifecycle.rs:L116-L117` が `Never` へ写す（https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-balloontimeout）。`OnBalloonTimeout` は「選択肢以外でバルーンがタイムアウトした際に発生」で、この範囲の送り方は合う（https://ukatech.shillest.net/ukadoc-spec/list_shiori_event.html#OnBalloonTimeout）。`\![move]`（https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-moveasync.note）の既定値・キーの大小・名前付きの形はコードと合い、違う 3 点（`point.basepos`・time>0 の補間・他のゴーストの基準）はどれも COMPAT の表か roadmap に記録されたわざとの差。
- `doc/COMPAT_ARCHITECTURE.md` の名指しの行（133・135〜138・154・155・174・175・179・180・407 の選択肢の起点）は今のコードと合う。合わないのは 134 行（6 節）と、177・407 行の抑止の理由（10 番）。

---

## 5. 直さずに残す候補（要件 2.2）

- **⑴ 開発者の裁定に反する直し**（候補 3 件）
  - 1: 完了した `areka-P0-draw-load-parity` の tasks.md L280 の「安全側」の候補メモ。決めではなく「周回で検討」の持ち越しと読める（覆す直しには当たらない見込み）。
  - 3: 完了した `areka-P0-shell-balloon-switch` の設計 L966 は断りを「ゴースト切替中・更新中」と決めた。インストール中を外したのがわざとかは書かれていない。
  - 40: `runtime-crate-split` の割り方の表（4.2 の案 C）は本体に残すテストを名指した。規則 D8「本体を引かない関数は戻す」には合い、名指しの理由（当時は `ShellBoxHover` が本体にあった・`BalloonWiring` を据えていた）は今は消えている。
  - 件 9 の ⓑ は D8 ⑵（番人の crate を `test-support` に入れない）には反しないが、範囲の外の `sample-ghost-kit` に触る。ⓐ は D8 ⑵ に触れない。
- **⑵ わざと置いた先送りの印**（候補 1 件）
  - 37: `BaseposResolver`（実装 1 つの型シーム）は COMPAT:154-155 に登記された先送り。直すのは `pub use` の可視性だけで、シームは残す。
  - `shortcut:`・`ponytail:` の注釈は範囲の中に 0 件。doc つきの `#[allow(dead_code)]` は 1 件（`RigSample::keep`・指摘にしていない）。
- **⑶ SSP 互換か ukadoc の正典に関わる振る舞いの変化**（候補 0 件）
  - 範囲の中に、SSP 互換の振る舞いを変える直しは無い（4 節）。41 番の ⑵ は表の中の値だけで、本番の既定値は変えない。

---

## 6. 範囲の外で見つけたこと（持ち主へ渡すもの）

| 件 | 場所 | 持ち主の候補 | 中身 |
|---|---|---|---|
| ア | `doc/COMPAT_ARCHITECTURE.md:134` | `doc-compat-architecture-rename`（この spec の着地を待つ）か、この spec（議題 6） | 「`ignore-sticky-window` を受けても動きは変わらない（窓どうしの連携がまだ無い）」が古い。今は組の相手を連れ動かさず `sticky_follow_ignored` を記録する（`move_cue_place.rs:L208-L212`・テスト `move_cue_apply_move_sticky_tests.rs:150`） |
| イ | `crates/areka-runtime/src/frame_persist_test_support.rs:154` | 完了した `audit-emo2-boot-frame` の範囲（今は持ち主なし） | `PersistStore::drop` の `let _ = handle.join();`（15 番と同じ型） |
| ウ | `SharedFakeIo` のタプルの写し 3 つ: 本体 `runtime_tests/frame/switch_finish_tests.rs:60`・`areka-session/src/boot_resolve_tests.rs:421`・`boot_seam_persist_wiring_tests.rs:13` | `audit-areka-app`〔C26〕 | 27 番と同じ形（placement の支えで足りる） |
| エ | `crates/areka-placement/src/windowposition.rs:L130-L134` | 完了した `audit-placement-*` | `scale_signed(i32::MIN, k)` が `-i32::MAX` を返す（1 ずれる）。台本の `--X=-2147483648` だけで起き、実害はほぼ無い |
| オ | `crates/areka-runtime/src/shell_balloon_resolve.rs:3` | 完了した `audit-emo2-boot-spine`／`frame` の範囲 | 「入口が長くなるので…こちらへ分けた」は経緯のコメントで、別の crate の入口を前提にする |
| カ | 本体 `session_tests/ghost_session_switch_tests.rs:31`・`update/worker_path_tests.rs:101` の `kinds`、`refs_of` の 3 つ（`install/desk_overwrite_tests.rs:302`・`update/desk_reload_tests.rs:119`・`update/worker_path_tests.rs:113`） | `audit-areka-app`・`audit-areka-install-update`〔C26〕 | `shell_balloon_switch_session_lap_test_support::kinds`（pub）と同じ中身 |
| キ | `crates/areka-session/src/ghost_session_runtime.rs:109-121` | `audit-areka-app`〔C26〕 | `ghost_switch.rs` の `take_down`（L767-L777）と同じ計時と記録（`ghost_switch_down_failed`／`ghost_switch_down_ms`）の写し |
| ク | `crates/areka-session/src/runtime_boot.rs:22` | `audit-areka-app`〔C26〕 | `ChangeRequestRaw` を ghost_switch の出し直しから、対の `SkinRequestRaw`（L31）は runtime から直に引く。片方にそろえれば `ghost_switch.rs:44` の `pub(crate) use` も消せる |
| ケ | `crates/areka-session/src/reload_test_support.rs:9,14`・本体 `session_tests/reload_tests.rs:26,37` | `audit-areka-app`〔C26〕 | 同じ型を 2 つの道で引く（4 番の直しで 1 つにそろう＝4 番の範囲の外の呼び手） |
| コ | 本体 `mcp/dump_surface_gpu_test_support.rs:63,145` | `audit-areka-install-update`〔C26〕 | `TICK_EVERY` と Tick を送る処理の 3 つ目の写し（53 番と同じ） |
| サ | `crates/areka-session/src/shell_balloon_switch_session_lap_test_support.rs:204-212`（範囲の中）と `areka_placement::test_support::give_fake_handles` | この spec（直し方には出さない） | 同じ形だが、`areka-session` の `test-support` は `areka-placement/test-support` を連鎖させないので、寄せるには `Cargo.toml` を直す（要件 6.6 で不可）。事実として記録だけ |
| シ | 件数を数える関数の写し 5 つ（`boot_resolve_readonly_tests.rs:15`・`boot_resolve_tests.rs:38`・`boot_shell_tests.rs:109`・`switch_copy_tests.rs:126`・`write_failure_late_tests.rs:33`） | `audit-areka-app`〔C26〕 | 31 番と同じ |
| ス | 本体 `runtime_tests/frame_visibility_integration_tests.rs` | `audit-areka-app`〔C26〕 | 相が読まない `BalloonWiring::new` の据え付け 1 か所（25 番と同じ）と `visibility_lines` の 3 つ目の写し（L97） |
| セ | `crates/areka-session/src/reload_skin_tests.rs:52` の `headless_wiring` | `audit-areka-app`〔C26〕 | 空の `BootAssets` の手書き（26 番と同じ・`synth_assets(&[])` で足りる） |
| ソ | 本体 `tick_gate_config_producers_tests.rs:L68-L73` | `audit-areka-app`〔C26〕 | 自分用の `code_only` の写し（`areka_placement::test_support::code_only` と同じ中身） |
| タ | `BODY_SYSTEMS` を手で足す範囲の外の 5 本（3 節の件 10） | `install`・`update` の 3 本は `audit-areka-install-update`、`reload_tests` の 2 本は `audit-areka-app`〔C26〕 | 30 番と同じ型。ロードマップの約束 ⑴ どおり持ち主が直す |
| チ | 本体 `sample_test_support.rs:42-55` の `switch_rig` | — | `RigSample::of`（`cfg(test)`）の写し。`sample-ghost-kit` を `test-support` に入れない決めの帰結で、指摘ではなく事実 |

- 申し送りの書き方（brief に書き足すか、走っているセッションへ伝えるか）は要件 7.1・7.8・7.9 に従う。〔C26〕の件（ウ・カ〜ケ・コ・シ〜タ）は C26 のあいだ持ち主のセッションへ伝える（約束 ⑴）。完了した範囲の件（イ・エ・オ）は持ち主の spec が無い＝要件 7.9 の「受け持つ spec が実在する」を満たさないので、扱いは要件の討議で決める（議題 10）。

---

## 7. 並走の約束（要件 7・C26 の約束 ⑴〜⑷）に触れる指摘

- **ほかの 2 本の持ち物のファイル（約束 ⑴・要件 7.1・7.3）**: 直しの本体はどれも範囲の中にある。直しに要る範囲の外の呼び手の書き換えは次のとおり（どれも `use` の道を替えるだけ）。
  - 4 番: `areka-session/src/reload_test_support.rs:14`・本体 `runtime_tests/frame/switch_tests.rs:37`・`runtime_tests/frame/switch_finish_tests.rs:50-52`（`audit-areka-app`）。
  - 5 番: 本体 `runtime_tests/frame/switch_finish_tests.rs:50-52`・`switch_tests.rs:838`（`audit-areka-app`）。
  - 6 番: `LastInstalledGhost` を私有にするなら本体 `install/` の 4 本（`audit-areka-install-update`）。今のまま置けば触らない。
  - 28 番の (b): `reload_test_support.rs:17` と呼び手 2 か所（`audit-areka-app`）。(a) なら触らない。
  - どれも相手のセッションと申し合わせてから触る（要件 7.1）。触れなければ申し送る（要件 7.8）。
- **`Cargo.toml`・`Cargo.lock`（約束 ⑵・要件 6.6・7.4）**: 依存を足す指摘は 0 件。27 番・24 番・40 番は既にある dev-dependency で足りる。29 番は ⓐ なら `Cargo.toml` に触らない。6 節のサの寄せは `Cargo.toml` を要るので直し方に出さない。
- **`doc/COMPAT_ARCHITECTURE.md` §8（約束 ⑶・要件 7.5）**: 10 番（177・407 行に 1 文足す）と 6 節のア（134 行の文の直し）は、どちらも既にある行の中身の直しで、要件 7.5 の「名指しの字面だけ」を越える。ほかの 2 本が §8 のどの行に触るかは、着手のときに確かめる。名指しの字面が変わる直し（関数や定数の名の変更）は 0 件。
- **`crates/dola/`・`wintf`（約束 ⑷）**: 触る指摘は 0 件（1 番は `tick_wake` を読むだけ）。
- **`crates/areka-runtime/src/lib.rs`**: 36 番だけが触る（`pub mod` → `pub(crate) mod`）。範囲の外だが、ほかの 2 本の持ち物ではない（完了した spec の範囲）。
- **1,000 行の番人（要件 6.1）**: 上限越えの表に入るファイルを作る指摘は 0 件。`ghost_switch_tests.rs` 982 は 23・28 番で約 800 行、`balloon_visibility_tests.rs` 960 は 22・41 番で約 810〜845 行に下がる。`ghost_switch.rs` 927 への直しは減らすか 1 行足すだけ（34 番の (a) は約 +8 行）。40 番の戻し先は約 860 行。
- **後の spec の先回り**: `balloon-script-offset-align`（`move_cue*` の働き）・`dola-interrupt-ticks`（時刻の型と合図）・`doc-compat-architecture-rename`（文書の名）・`ghost-switch-balloon-fallback`（切替のバルーンの決め方）の仕事に当たる指摘は 0 件。`move_cue_geometry.rs:36` の文書の名指しには触れない。11・12・19・27・44〜46・72 番は `move_cue*` を触るが、働きは変えない（説明・テスト・支えだけ）。

---

## 8. 確かめた指摘

読み手の報告のうち、次の指摘はこの文書を書く前に根拠の行を読み直した。

- 1 番: `balloon_visibility_phase.rs:L115-L123`・`L150-L185`（`VisibilityWake::Now` の doc に「抑止で保留中など」）と `balloon_visibility_wait.rs:L225-L242`（抑止中の超過は満了予定を残す）。`areka-P0-draw-load-parity/tasks.md:L280` の候補メモ。`visibility_wake` を入れたのは `327e7fd3`（#118 `areka-P0-draw-load-parity`）。
- 2 番: `balloon_visibility_phase.rs:L455-L465`（`marker.scope as u32`）。
- 3 番: `shell_balloon_switch.rs:L120-L140`（`update_busy` だけ）と `reload.rs:L300-L322`（`update_busy` と `install_busy`）。`areka-P0-shell-balloon-switch/design.md:L966`。
- 30 番: `areka-session/src/body_systems.rs:L36-L57`（`#[cfg(not(test))]` の `error!(body_systems_missing)`）。
- 23 番: `areka-runtime/src/special_name.rs` に兄弟のテストが無く、`resolve_special_name` の使い手は `areka-session` の 3 ファイルだけ（grep）。
- 11 番: `move_cue.rs:L63-L69`。
- 25 番: `areka-runtime/Cargo.toml` に `areka-input` の依存が無い（L34 は「本体と areka-input のテストが入口を引くため」の説明だけ）。件 3 の読み手と B3 の読み手が別々に同じ結論を出した。
- 件 3 の `switch_assets` の 6 件: 分ける前の `7f41b53e6^:crates/areka/src/runtime/switch_assets.rs` の `pub(crate)` を数えた。
- 範囲の名の型から外れる 2 本（`balloon_hover_tests.rs`・`shell_box_hover_tests.rs`）: 全行読み、指摘 0。足したのは `26a5d573`（#492）。

それ以外の指摘は、読み手が報告に書いた根拠の行による。走らせての確かめ（1 番の毎フレームの予約・2 番のあふれの赤・3 番の自動切替の経路・16〜19 番の赤・24 番と 40 番の移しが組めるか・`RigSample::keep` の許可が効いているか）は実装の段で行う。

---

## 9. 要件と今の姿の突き合わせ

| 要件 | 要るもの | 今の姿 | 差（Missing / Unknown / Constraint） |
|---|---|---|---|
| 1 | 範囲の測り直しと全件の audit | この文書 | 無し。範囲の名の型から外れる 2 本は議題 9 |
| 2 | 直す対象の確定と、直さずに残すものの裁定 | 候補 4 件（5 節） | Constraint: 開発者の裁定待ち |
| 3 | 73 件の直し・呼び手の直し・消す前の grep | 指摘ごとに場所と直し方がある | Missing: 直しそのもの。4〜6・28 番は範囲の外の呼び手に及ぶ |
| 4 | バグと危ない所ごとのテスト | 1・2・3・14・15・16〜19 番に足すテストを書いた | Missing: 1 番の「抑止中は預けない」・2 番のあふれの赤・3 番の断りの檻・16 番の陽性の対・17〜19 番の抜け |
| 5 | 預かった件 1〜10・12 | 3 節 | Missing: 件 1〜10 の直し。件 12 は確かめ済み |
| 6 | 決まり（1,000 行・記録の無い失敗・依存を足さない ほか） | 写しの直しは行を減らす。900 行台の 3 本は先に減らせる | Constraint: 23 番は範囲の外の新しいファイルを作る（議題 8） |
| 7 | 範囲の外への直しと C26 の約束 | 7 節 | Constraint: 4・5・6・28 番の範囲の外の呼び手・10 番と 6 節アの §8 の文 |
| 8 | 重い操作を回さない | 決定論のテストだけで確かめられる | 無し（SSP の測りが要る指摘は 0） |
| 9 | かけ直しと全体テスト | — | Missing: 実装の後 |

### 直し方の選び方

- **案 A（ファイルのまとまりごとに切る）**: 束（1.4 節の表）に沿ってタスクを切る。同じファイルを 2 つのタスクが触らないので、レビューと衝突が少ない。重さの違う指摘が 1 つのタスクに混ざる。
- **案 B（組ごとに切る）**: 必ず直す → 直したほうがよい → できれば、の順に切る。大事なものが先に着地するが、同じファイル（`balloon_visibility_tests.rs`・本体の `balloon_visibility_phase_tests.rs`・足場）を何度も触るので、行番号のずれと衝突が増える。
- **案 C（混ぜる）**: 1・2・3 番を先に独立のタスクにする（それぞれ赤のテストから始める・要件 4.2）。次に公開の面の絞り（4〜7・36〜39 番・件 1〜3）を `areka-session` と `areka-runtime` の 2 本にまとめ、範囲の外の呼び手の申し合わせを 1 度で済ませる。その後に本体のテストの戻し（24・25・40 番＝件 3 の数え直しの前提）、支えの寄せ（26〜33・49〜57 番）、檻の抜け（16〜22・41〜48 番）、説明と決まり（8〜13・58〜73 番）をファイルのまとまりで切る。戻しを公開の絞りの前に置くと、件 3 の数え直しが 1 回で済む。

### 手間と危うさ

- 手間: **M〜L**（1〜2 週間）。73 件のほとんどは S だが数が多い。40 番は 27 本のテストの移し（M）、13 番は 7 束にまたがる文の直し。
- 危うさ: **低〜中**。
  - 本番の振る舞いが変わるのは 1・2・3 番だけ（1 番は起き方・2 番は届かない入力・3 番は断りの集合）。
  - 公開の面の絞り（4〜7 番）は範囲の外の呼び手（C26 のほかの 2 本の持ち物）に及び、申し合わせが要る。
  - 40 番の移しは、`areka-runtime` の dev-dependency だけで組めるかを組んで確かめるまで確かでない（依存の向きと grep からの推定）。
  - どれも既にある型の延長で、新しい技術は要らない。

### タスクの数の見立て（要件 2.6）

| まとまり | 指摘 | タスク |
|---|---|---:|
| 毎画面更新の予約 | 1 | 1 |
| scope の切り捨て（件 6） | 2 | 1 |
| インストール中の断り | 3 | 1 |
| `areka-session` の公開の面の絞り（件 1・件 2） | 4・5・6・38（session 分）・39 | 1 |
| 本体のテストの戻し（件 4） | 24・25・40・64・65・69 | 1〜2 |
| `areka-runtime` の公開の面の数え直し（件 3） | 7・36・37・38（runtime 分） | 1 |
| 移動の cue の支えと檻（件 5・件 7） | 15・19・27・44〜46 | 1 |
| `areka-session` の足場の寄せ（件 8・件 9） | 23・28・29・31・32・52〜56 | 2 |
| 本体の `session_tests` の足場（件 10） | 26・30・57・63 | 1 |
| 可視性のテストの抜けと重なり | 16・17・18・21・22・33・41〜43・70・71 | 1〜2 |
| `areka-session` のテストの小さな直し | 14・20・47〜51 | 1 |
| spec の名と番号の名指し | 13 | 1 |
| 本番の説明（session・runtime） | 8〜12・58〜62・66〜68・72・73 | 1〜2 |
| 負荷・割り当ての小さな直し | 34・35 | 1 |
| かけ直しの audit と全体テスト（要件 9） | — | 1〜2 |
| **計** | | **17〜22** |

- 20 を越えうる。要件 2.6 と問い 1 の仮定（後の決め 2026-11-16 と前例 `audit-emo2-boot-spine` の 23 タスク・`audit-emo2-boot-frame` の 24〜28 の見込みで 1 本）が要る（議題 1）。

### 設計へ持ち越す調べ（Research Needed）

- 3 番: インストールの完了の後の自動切替が、`install_busy` が真の間に `request_skin_switch` を呼んでいないか。
- 1 番: 抑止の解け方（ポインタの離脱・ドラッグの終わり・選択肢の終わり）がどれも自前の起床を持つかの網羅（`tick_wake.rs` と `areka-input` の入力の口）。選択肢の待ちで満了予定が立つ場合があるか。
- 40 番: 27 本が `areka-runtime` の dev-dependency だけで組めるか、戻した後に署名の鎖で開いたままが要る項目（`private_interfaces`）の数え直し。
- 16 番: 案 ⑴ の陽性の対（`ticker_closed(&events) >= 1`）が UI スレッドの捕捉の窓に出るか。
- 48 番: 注入のテストが `CoGetApartmentType` で暗黙の MTA を見分けられるか。
- `RigSample::keep` の `#[allow(dead_code)]` が効いているか（`pub` の型の `pub` の欄）。
- 41 番の ⑵: 表の `TIMEOUT` を既定でない値にしたとき、表の全部の行が成り立つか。

---

## 10. 要件の討議へ渡す議題（答えで作業が変わるもの）

1. **規模（問い 1・要件 2.6）**: 17〜22 タスクの見込み（9 節）。推す＝仮定どおり 1 本のまま進め、見立てを記録だけする（2026-11-16 の決め・前例 `audit-emo2-boot-spine` 23 タスク）。
2. **1 番の毎画面更新の予約**: `areka-P0-draw-load-parity` の tasks.md L280 が「安全側」と書いて持ち越した候補。推す＝直す（抑止中の超過は預けない・抑止の解けはポインタ駆動の起床に任せる）。決めを覆すかどうかの確認が要る。
3. **3 番のインストール中の断り**: 推す＝Research Needed（自動切替の経路）を設計で確かめ、自動切替を止めないなら `install_busy` も断る。自動切替が当たるなら、断りの位置（取り出しの系の側か入口か）を設計で決める。
4. **問い 2（使い手の数え方）**: 推す＝仮定どおり。件 3 の結果は「今すぐ戻せるのは `settle_box_hover` 1 つ・ほかは本体のテストを戻した後の姿で数える」（3 節）。brief の「`switch_assets.rs` 4」は 6 に読み替える（`SwapPayload`・`SwapSlot` も開いた・どちらも本体のテストか公開の欄が使う）。
5. **問い 3（`add_rpost_shell`）**: 1 つにする形 ⓐ が `Cargo.toml` も依存も変えずに成り立つ（3 節の件 9）。推す＝ⓐ（`lap_rig_on` が写す元のシェルの道を受け、写す手順は std だけの支えへ 1 つ・52 番の `copy_tree` の寄せと一緒に）。検体を取る約 4 行は呼び手に残り、その理由の文を両方に置く。関数の定義まで 1 つにしたいなら ⓑ（範囲の外の `sample-ghost-kit` に窓口を足す）を開発者に問う。
6. **`doc/COMPAT_ARCHITECTURE.md` §8 の文の直し（10 番・6 節ア）**: 要件 7.5 は「名指しの字面だけ・行を足さない」。177・407 行に抑止の理由を 1 文足す直しと、134 行の古い文の直しは、どちらも既にある行の中身の直し。推す＝要件 7.5 を「既にある行の、この範囲の振る舞いを書く文の直しまでは許す（行は足さない）」と読み替えて直す。C26 の約束 ⑶（別々の既にある行だけ）には触れない見込み。だめなら `doc-compat-architecture-rename` の brief へ申し送る。
7. **問い 4（`BODY_SYSTEMS` の 1 行）**: 足場の側でまとめても表明は変わらない（3 節の件 10）。推す＝`rig` の中で 1 回据え、3 本の 1 行を消す（組の無い足場が出していた `error!(body_systems_missing)` も消える）。
8. **23 番の置き場**: `special_name` の純関数のテストを `areka-runtime/src/special_name_tests.rs`（範囲の外の新しいファイル）へ移す。推す＝要件 3.2 の「直しに要る変更」として行う（`ghost_switch_tests.rs` を 1,000 行から離し、関数とテストを同じ crate に置く）。範囲の外のファイルは完了した spec の範囲で、C26 のほかの 2 本の持ち物ではない。
9. **範囲の名の型から外れる 2 本**: `balloon_hover_tests.rs`・`shell_box_hover_tests.rs`（144 行・#492 が足した）はどの brief の範囲にも無い。推す＝この spec の範囲に入れる（受け持つ本番 2 本の兄弟のテストで、7 番の直しで `settle_box_hover` の兄弟のテストを読む）。読んだ結果は指摘 0 なので、作業は増えない。要件の Project Description の「41 ファイル」を 43 に、計を 70 ファイル・22,852 行に改める。
10. **範囲の外の持ち主の無い件（6 節のイ・エ・オ）**: 完了した範囲の件で、要件 7.9 の「受け持つ spec が実在する」を満たさない。推す＝イ（`frame_persist_test_support.rs:154` の join）とオ（経緯のコメント 1 行）は要件 3.2 の「直しに要る変更」ではないので触らず、事実として記録だけ残す。エ（1 ずれ）は実害が無いので記録だけ。直すなら開発者の裁定で。
11. **問い 5（`boot_context`）**: 推す＝仮定どおり、自分の範囲の 1 か所だけを寄せる。28 番の (a)（`ghost_switch_test_support.rs` に `pub fn boot_context(root, current)`）なら範囲の外に触らず、`menu/ghost_frame_tests.rs:43`（字面まで同じ）の持ち主がそのまま使える＝材料として渡す。(b) は C26 の相手のファイルに触る。
12. **問い 6（`SwapFinish` と `ghost_switch_test_support`）**: 推す＝仮定どおり。`SwapFinish` の出し直しは閉じる（5 番・本体のテスト 2 か所の道を替える＝`audit-areka-app` と申し合わせる）。モジュールの開け方は残し（本体の約 30 本・`areka-session` の 9 本が使う）、範囲の外の使い手 0 の `add_balloon_copy`・`plant_last_balloon` を `pub(crate)` にする（39 番）。`Stall` は pub の enum の変種が名指すので残す。
13. **40 番（27 本の戻し）**: `runtime-crate-split` の割り方の表と違う置き場になる。推す＝この spec で行う（名指しの理由は今は消え、規則 D8 には合う・`areka-runtime` の渡し口と `test_open` を閉じられる）。
14. **5 節の裁定の候補**: 1・3・40 番と件 9 の形（議題 2・3・5・13）。37 番は可視性だけを直しシームは残す（⑵ の印には触れない）。

### 設計の段で決めること（要件の討議の分類 B の候補）

- 2 番: 案 ㋐（広げて比べる・記録なし）か ㋑（`window_scope`・毎フレーム warn）か。要件 5.6 は「`warn!` を残して読み飛ばす（`window_scope` を使う形にそろえる）」と書くので、㋑ を取るなら warn の抑え（1 回だけ）の形を決める。
- 16 番: 案 ⑴（陽性の対）か ⑵（`event=` の欄）か。
- 22 番・44 番: どちらの重なりを消すか。
- 28 番: (a) か (b) か。
- 41 番: ⑴ だけか ⑵ も足すか。
- タスクの切り方: 9 節の案 A・B・C。
- 9 節「設計へ持ち越す調べ」。

---

## 11. 並走の spec から受け取った申し送り（2026-11-22・要件の討議の確定の後）

`audit-areka-app` のギャップ分析が、範囲の外（＝この spec の範囲）に見つけた件。4 件のうち 2 件は既にある指摘と同じ所で、2 件を新しい指摘 74・75 として足した（どちらも直したほうがよいの組・要件 2.1 のとおり直す対象）。

| 受け取った件 | 扱い |
|---|---|
| `move_cue_apply_move_sticky_tests.rs:200`・`:255`、`move_cue_place_tests.rs:321`・`:386` の `SharedFakeIo` の写しと `let _ = …handle.join()` | 27 番・15 番と同じ（件 5・件 7） |
| 本体 `session_tests/shell_balloon_switch_tests.rs:78` の `headless_wiring` が `areka-session/src/reload_skin_tests.rs:52-79` と 1 字違わない | 26 番と同じ所。寄せ先は `synth_assets(&[])` のほかに、相手が自分の側で使う `areka_runtime::frame::shared_test_support::wiring().1` もある（設計で選ぶ・相手の側は相手が直す） |
| `ghost_switch.rs:522` の記録の文の「今日どおり」 | 新しい指摘 75 |
| `shell_balloon_switch.rs` の `accept_skin` が送れたかを確かめる前に組み立てを起こす | 新しい指摘 74 |

#### 74. `accept_skin` が、kanade へ頼めたかを確かめる前に資産の組み立てを起こす
- 場所: `crates/areka-session/src/shell_balloon_switch.rs:L210-L240`（L217 `spawn_switch_build(..)` の後に L228 の `send(KanadeMsg::AwaitTalkGap ..)`）
- 種類: 負荷・危ない所
- 問題（具体的な場合）: kanade が止まっていて送りが失敗すると、`NoContext` で断るのに、背景では資産の組み立て（descript と画像の読み）が既に走っていて、結果は誰も受け取らずに捨てられる。
- 直し方: 送りを先にし、送れたときだけ `spawn_switch_build` を起こす。送りの失敗の回に組み立てが起きないことをテストで縛る。
- 規模: S

#### 75. 記録の文に経緯の語「今日どおり」が残る〔約束 c〕
- 場所: `crates/areka-session/src/ghost_switch.rs:L423`（「切替の予約が無い停止通知——今日どおり終了する」）・`L522`（「…終了要求が勝っていれば今日どおり終わる」）
- 種類: 決まりの違反（経緯の語）
- 問題（具体的な場合）: 「今日どおり」は、変える前と比べた言い方で、読む人には何と同じなのかが分からない。
- 直し方: 「そのまま終了する」「通常どおり終わる」の形にする。語・target・欄は変えない。`crates/`・`tools/`・`doc/`・`.claude/skills/` の grep で、この 2 つの文を字面で読む所は 0 件（同じ語は範囲の外にもあるが、そちらは持ち主の範囲）。
- 規模: S

---

## 12. 設計の段の調べと決め（2026-11-22・`/kiro-spec-design -y`）

- 発見の型: 拡張（既にある系の直し）なので軽い発見（`design-discovery-light.md`）。外の調べ（Web）は要らない（新しい依存・新しい技術は 0）。読んだのはコードと grep だけで、cargo は回していない。
- 使った物差し: `kiro-spec-design` の `design-principles.md`・`design-synthesis.md`・`design-review-gate.md`。前例の設計は `completed/audit-emo2-boot-frame/design.md`・`completed/audit-emo2-boot-spine/design.md`（指摘ごとの表・束・1,000 行の表・要件の対応の形）。

### 12.1 Research Needed の答え（9 節の持ち越し）

| 項目 | 答え | 根拠 |
|---|---|---|
| 3 番: インストール直後の自動切替が `install_busy` の真の間に入口を呼ぶか | **ベースウェアの自動切替は 0**（完了 `ghost-install` の裁定「インストールと切り替えは別イベント・areka は入れた後の切替を主導しない」・完了 `areka-P0-shell-balloon-switch` の requirements.md L59・design.md L39 の Non-Goals）。入口の呼び手はメニュー（`menu/shell_frame.rs:L70`）と台本の取り出し（`shell_balloon_switch.rs:L84`）の 2 つだけ。**ただしゴースト側の「入れた後の切替」は届く**: 背景のスレッドは `OnInstallComplete(Ex)` を依頼の中で送り（`install/procedure.rs:L271-L285`）、kanade は最初の往復の結果で返事をし（`areka-kanade/src/change.rs:L120-L135`）、台本の再生はその後に UI で進む。`busy` が偽になるのは `DeskAsk::OrderDone`（`install/desk.rs:L297-L299`）で、`run_order` が返った後（飛ばした同梱の警告の後）。待っている依頼（`queue`）か上書き（`overwrite`）があれば `is_busy` はその間ずっと真（`desk.rs:L151-L155`）。だから応えの台本の頭の `\![change,balloon,…]`・複数の書庫の投入・警告の出ている間は、`install_busy` が真のまま入口に届く | 左の各行 |
| 1 番: 抑止の解け方がどれも自前の起床を持つか | 持つ。抑止は「ドラッグ（バルーン窓・箱に文字の出ている scope のシェルの窓）」と「滞在（可視バルーン・文字の出ている箱）」の 2 つだけ（`balloon_visibility.rs:L169-L181`・`balloon_visibility_wait.rs:L155-L156`）。ポインタの離脱・移動・ボタンは `POINTER`、移動とサイズの終わりは `WM_GEOMETRY`（`wintf/src/ecs/world/tick_wake.rs:L282-L320`）。バルーンが隠れる・箱の文字が消えるのは cue の配信か隠す発行で、そのフレームは cue が起こす。解けた縁は同じフレームの `decide` が `now + wait` を立て直し（`balloon_visibility_wait.rs:L189-L198`）、同じフレームの `visibility_wake` が `After(wait)` を預ける。選択肢は抑止の種類ではない（起点の採り方だけ・`effective_display_end`） | 左の各行 |
| 1 番: 相の中で抑止の答えを読めるか | 読める。相は `balloon_visibility` の子のモジュール（`balloon_visibility.rs:L502-L507`）なので私有の欄 `prev_suppressed`（L450）が見える。`decide` の後なら今のフレームの抑止の答えに等しい（`balloon_visibility_wait.rs:L190`） | — |
| 1 番: 決定論の赤 | `tick_wake` はプロセス共有の 1 つ（`tick_wake.rs:L256` の `static GLOBAL`）で、相の後に `take` で見るテストは同じバイナリのほかのテストと混ざる。だから赤は純判断 `visibility_wake` と、`decide` の後の状態から答えを引く形で立てる（設計の 1 番） | — |
| 40 番: 27 本が `areka-runtime` の dev-dependencies だけで組めるか | 組める見込み（字面で確かめた）。27 本の `use` は `areka_emo_atlas`・`areka_emo_compose`・`areka_emo_present`・`areka_emo_text`・`areka_sakura`・`areka_seriko`・`areka_parsers`・`areka_placement`・`bevy_ecs`・`dola`・`wintf`・`tracing`・`log_capture_kit` と `areka_runtime` の項目で、どれも `areka-runtime` の `[dependencies]`/`[dev-dependencies]` にある。外の crate は `areka_input::balloon::{BalloonWiring, ChoiceSelection}`（25 番で消す）と `areka_input::shell_box::ShellBoxHover`（`areka_runtime::shell_box_hover::ShellBoxHover` の出し直し＝`areka-input/src/shell_box.rs:L17`）だけ。本体の項目（`crate::…`）を引く行は 0。組めるかは実装で `cargo test -p areka-runtime --no-run` で確かめる | grep |
| 40 番: 戻した後も開いたままが要る項目 | 本体に残る `notice_tests`（5 本）が `notify_timeout`（渡し口）・`Frame`・`TALK_ID`・`seen`・`BalloonVisibilityState` を使う。本体の `frame_*` の 2 本が `configured_timeout_secs`、`areka-input` の支えが `phase::run_balloon_visibility_phase` と `balloon_hover::BalloonHover` を使う（grep）。ほかの「40 番の後に使い手を失う」（3 節の件 3）は、移した後にツリー全体 grep で 0 件を確かめて狭める。公開の型の欄・署名に出るもの（`private_interfaces`）は残して名と理由を記録する | grep |
| 16 番: 陽性の対 | 可視性の行は欄 `trigger = "explicit"`（`VisibilityTrigger::Explicit.as_str()`・`balloon_visibility_phase.rs:L490-L495`）を持つので、文面でなく欄で数えられる（本番は変えない）。単位の行（`ghost_session.rs:L600`・`audit-areka-app` の持ち物）には `event` の欄が無いので、文面のまま陽性の対を同じファイルに置く。UI のスレッドの捕捉の窓に出るかは実装で確かめる（仮定） | — |
| 48 番: 暗黙の MTA を見分けられるか | 見分けられる。`CoGetApartmentType` は暗黙の MTA で `APTTYPE_MTA` と `APTTYPEQUALIFIER_IMPLICIT_MTA` を返す（Win32 の文書どおり）。`spawn_build_worker`（`switch_assets.rs:L135`・私有）は閉包を受けるので、子のテストから注入できる。`windows` の機能は `Win32_System_Com` で足りる見込み（`CoInitializeEx` を既に使う）。足りなければ機能を足さずに議題（要件 6.7） | — |
| `RigSample::keep` の `#[allow(dead_code)]` | 指摘ではない（1.3 節）。かけ直しの audit のときに、許可を外して `cargo check -p areka-session --tests` と `--features test-support` の警告で効いているかを見る（効いていなければ消す・効いていれば今の理由のまま） | — |
| 41 番 ⑵: 表の `TIMEOUT` を既定でない値にしたとき | 表の各行が `TIMEOUT` からの足し引きで書かれているかは実装で 1 行ずつ確かめる（仮定: 20.0 で全行が成り立つ。成り立たない行は期待を `TIMEOUT` から出す形へ直す・要件 4.7） | — |

### 12.2 設計の段で見つけたこと

- **5.15 ㋑（`scale_signed`）は、完了 spec がわざと固定した例外を変える**: `crates/areka-placement/src/balloon_offset_supply_tests.rs:L82-L108` の `identity_scale_saturates_only_at_i32_min`（「拡大率 1 では生値が素通しになる、という原則の唯一の例外」「製品欠陥ではない」「本檻が固定する」）と `follow_offset_space_tests.rs:L195` の `supply_identity_saturates_only_at_i32_min` が `-i32::MAX` と `saturated == true` を表明する。入れたのは `areka-P0-balloon-offset-dpi`（#125 `9ac408757`・design.md L739「負値・`i32::MIN` 近傍・飽和の各腕を逐語で固定」）。要件ではなく設計とテストの決めなので要件 2.4 の「裁定」には当たらないと読むが、直すと檻 2 本の期待と `follow/offset_space.rs:L251-L256` の飽和の判定が変わる。設計の議題にする。
- **`SwitchVerdict` の腕を足す（76 番）と、範囲の外の網羅の `match` が 1 つ組めなくなる**: `install/overwrite.rs:L118-L130`。手続きの要求には `Updating` を返さないので、相手の腕に名を 1 つ足すだけ。`SkinVerdict` の腕を足す（3 番）ほうは、範囲の外の網羅の `match` が 0（grep・`reload.rs:L576` は `!= Accepted`）。

### 12.3 並走の spec からの申し送り（2026-11-22・設計の段・`audit-areka-install-update` `43031a5fc`）

| 件 | 中身 | 扱い |
|---|---|---|
| ⑴ | 起動中のゴーストへの上書きのインストールの展開を UI のスレッドの外へ出す。`ghost_switch.rs` の `switch_to` を「降ろした」と「起こす」の間で展開の答えを待つ段に分ける要がある | この spec の直しは `switch_to`（`ghost_switch.rs:L606-L710`）に触らない（直す所は L40-L44・L66-L90・L111・L250-L333・L423・L522）。継ぎ目として設計に書き、どちらが分けるか・着地の順を議題にする |
| ⑵ | `shell_balloon_switch_session_lap_test_support.rs` の `spawn_windows` に `dpi: u16` を受けさせ、それと `pump_messages` を `pub` にしてほしい（本体の `mcp/dump_surface_gpu_test_support.rs:L323`・`L357` の写しを消すため） | 写しの寄せ（6 節のコ・53 番と同じ型）。受けるのを推す（議題） |
| ⑶ | 更新の背景のスレッドの記録の語 `update_reload_requested` を `update_reload_asked` に替える | 範囲の中の読み手 `crates/areka/src/session_tests/shell_balloon_switch_session_update_tests.rs:L243`（`count(&events, "update_reload_requested")` が 1）。相手が確かめて知らせる。替わったら同じ変更で追う（要件 7.7） |
| ⑷ | ゴースト切替の入口が `update_busy` を問わない | 新しい指摘 76（2.2 節）。コードで確かめ、更新の自分の読み直しを断らない形を設計した |

### 12.4 合成（`design-synthesis.md`）

- **まとめ（一般化）**: 3 番と 76 番は同じ問題（「入口の断りの集合が 3 つの入口でそろっていない」）の 2 つの場合。断りは入口の判定の並び（進行中 → ゴースト切替中 → 更新中 → インストール中 → 文脈）に 1 段ずつ足す形にそろえ、どちらも「手続き自身の要求を断らない」ことを出どころで見分ける。新しい仕組み（待たせる列・モード）は足さない。
- **既にあるものを使う（自作しない）**: scope の上限は `target_map::window_scope`（`audit-emo2-boot-frame` の正本）、アクターの待ちは `spine::join_bounded`、偽の HWND と偽の記憶の書き手は `areka_placement::test_support::{give_fake_handles, SharedFakeIo}`、空の資産は `frame::test_support::synth_assets(&[])` か `frame::shared_test_support::wiring()`、観測の組み立ては `balloon_visibility::test_support::Frame`、記録の件数は `fallback_tests::count_event`、呼出列は `lap_test_support::kinds`。
- **減らす**: 足す継ぎ目は 2 つだけ（74 番の `accept_skin_with` の作り口の注入・34 番の目録を 1 回だけ読む引数）。どちらも私有で、公開の面は増やさない。`SkinVerdict::Installing`・`SwitchVerdict::Updating` の腕は、断りの理由を呼び手へ返すための最小の足し。

---

## 13. 実装の段の記録（預かった件の片づけと申し送り・2026-11-23）

### 13.1 預かった件 1〜10・12 の片づけ（要件 5.1〜5.12）

| 件 | 要件 | 出どころ | 指摘 | タスク | コミット | 片づいた形 |
|---|---|---|---|---|---|---|
| 1 | 5.1 | `runtime-module-untangle` | 4 | 3.1 | `70bb10c2a` | シェル・バルーンの切り替えの出し直しを 5 型だけに絞り、残り 5 項目は私有の取り込みへ |
| 2 | 5.2 | `session-crate-split` | 5・39 | 3.1 | `70bb10c2a` | `SwapFinish` の出し直しを消し、範囲の外で使い手の無い足場の 2 関数を `pub(crate)` へ |
| 3 | 5.3 | `runtime-crate-split` ⑶ | 7・36・38 | 3.3（38 は 3.1 も） | `be4259528`・`70bb10c2a` | テストを戻した後の使い手で数え直し、公開を狭めた（箱の上の滞在の確定・ホバーの注入の mod・移動の cue の出し直しほか） |
| 4 | 5.4 | `runtime-crate-split` ⑶・`audit-emo2-boot-spine` ウ | 24 ほか（25・26・40・64・65・68・69） | 3.2 | `ff4a59133` | 本体の可視性の相のテスト 29 本を `areka-runtime` へ戻した |
| 5 | 5.5 | `audit-emo2-boot-frame` キ・棚卸㊸ 件 10 | 27 | 4.1 | `df16079cc` | 偽の HWND の輪と偽の記憶の書き手の写しを `areka_placement::test_support` の 2 つに寄せた |
| 6 | 5.6 | `audit-emo2-boot-frame` ク | 2 | 1.2 | `c723cea95` | ドラッグ中のシェルの窓の scope を `window_scope` の上限で比べる形にした |
| 7 | 5.7 | `audit-emo2-boot-frame` コ | 15 | 4.1 | `df16079cc` | 記憶の書き手の後始末の 2 か所を `join_bounded` で待つ形にした |
| 8 | 5.8 | 棚卸㊶ シ | 28 | 4.2 | `f1a631316` | `boot_context(root, current)` を足場に置き、`ghost_switch_tests.rs` の写しを消した |
| 9 | 5.9 | `session-crate-split` | 29・52 | 4.2 | `f1a631316` | `add_rpost_shell` の写し 2 つを、元のシェルの道を受ける形と std だけの `copy_tree` の 1 か所にまとめた |
| 10 | 5.10 | `session-crate-split` D13 | 30 | 4.3 | `a8902f5bf` | 足場 `rig` で本体の組を 1 回据え、`error!(body_systems_missing)` が出なくなった |
| 12 | 5.12 | `audit-emo2-boot-spine` オ | — | （直しなし） | — | 済んだまま。`balloon_visibility_phase_zorder_chain_tests.rs` は `source_text_test_support::code_only` を使い、ファイルに写しは無い |

- 件 11 は要件 5.11 の扱い: 申し送りの記録（13.3・13.4）。
- 範囲の外の 6 節 イ・エ・オ（持ち主の無い 3 件・要件の確定 10）とア（要件の確定 6）は、この spec で直した（ア＝`COMPAT_ARCHITECTURE.md` の 134 行・5.3、イ＝記憶の書き手の後始末・5.3 ㋐、エ＝`scale_signed` の飽和・5.3 ㋑、オ＝コメントの削除・5.3 ㋒。コミット `fba7c4f7e`）。サ（`Cargo.toml` を要るので寄せない）とチ（`sample-ghost-kit` を `test-support` に入れない決めの帰結）は事実の記録だけで、申し送りは要らない。

### 13.2 規模と、直さずに残す指摘

- 規模: 見立ての 17〜22 タスクのうち、20 タスク（T1〜T20）で組んだ。
- 直さずに残す番号つきの指摘は 0 件（要件 2.5・3 番の続きの `reload.rs` の件は 13.4）。5 節の候補 ⑴〜⑶（候補 1・3・37・40 と件 9 の形）は、要件の討議の確定（2＝候補 1・要件 5.13、3＝候補 3・要件 5.16、5＝件 9 の ⓐ・要件 5.9）と要件 2.1・5.3・5.4（37・40・タスク 3.3・3.2）で直す側に回り、設計の確定 1 で候補 3 をメニューの要求だけに絞った。⑶ は候補 0 件。76 件すべてを直す対象とした。

### 13.3 並走の spec への申し送り（要件 7.3・7.8・7.9）

送った日: 2026-11-23。書く前に、相手の spec が `.kiro/specs/` にあり `completed/` に無いこと（origin/main）を確かめた。

| 相手 | 申し送った件（6 節） | 中身 |
|---|---|---|
| `audit-areka-app` | ウ・カ（本体 `ghost_session_switch_tests.rs` の `kinds`）・キ・ク・ケ・シ・ス・セ・ソ・タ（`reload_tests` の 2 本） | 持ち物の写し・出し直しのそろえ・据え付けの削り。加えて、この spec が相手の持ち物に触ったのは申し合わせた use の道の 4 か所だけという知らせ。実際にはほかに `areka-session/src/body_systems_tests.rs`（相手の brief の `src/body_systems*`）を 2 つ直していて、この知らせから漏れていた。仕上げの確かめで拾い、2026-11-23 に別に知らせた（15.1・15.2） |
| `audit-areka-install-update` | カ（`refs_of` の 3 つ・`worker_path_tests.rs` の `kinds`）・コ・タ（`install`・`update` の 3 本） | 持ち物の写しと `BODY_SYSTEMS` を手で足す所。加えて着地の知らせ: `SwitchVerdict::Updating` と腕の名 1 つを `install/overwrite.rs:129` へ（`68c8e1069`・相手の開発者の確定 `537109983`・相手の要件 7.13 (a)）、`spawn_windows`・`pump_messages` を `pub` に（`f1a631316`） |

### 13.4 並走の spec の答えと決まりごと

- 3 番の続き（`reload.rs` の読み直しの入口が台本の `\![reload,…]` も断る件）: `audit-areka-app` が設計で「MCP からの読み直しだけを断る」と決めて片づいた。この spec は記録だけで、`reload.rs` には触れない。
- `update_reload_requested` の語の替えは窓口の側（`update/desk.rs:206` の記録）に及ばない。範囲の中の読み手 `shell_balloon_switch_session_update_tests.rs:243` は窓口の側の記録を数えるので、直しは要らない（`audit-areka-install-update` が確かめた）。
- 実装の段の例外: `doc/COMPAT_ARCHITECTURE.md` の 405 行は、4.6 のテストの名の替えに合わせて名指しを替えた（134・177・407 行のほかの 1 行・行は足していない）。75 番は 5.2 で research の 2 か所だけを直し、`ghost_switch.rs` の別の 2 つの文（「今日どおり終了する」）と `COMPAT_ARCHITECTURE.md:303` は 75 番の場所に無いので残した。かけ直しの audit で拾われたらそこで扱う。`ghost_switch.rs` の 2 つの文は、かけ直し 1 回目（`3ad5647f6`）で「そのまま終了する」の形へ直した（14 節・15.3）。

---

## 14. かけ直しの結果（要件 9・2026-11-23）

- 作法は 1 節と同じ。`main` は `1b3ede54` から進んでおらず取り込みは不要だった。範囲は 70 ファイル（`special_name_tests.rs` を移した先で数え、`special_name.rs` は足した `catalog_entry` だけ）・約 22,700 行。7 つの束に分けて読み手を立て、回ごとに全ファイルを全行読んだ。上限 20 件は外した。
- 直しは束を組み合わせた 3 組（移動の cue・可視性・切り替え）に分け、組ごとに実装係と独立のレビュー係を通してからコミットした。前の回の直しが呼んだ連鎖（2 回目の可視性）は Fable でまとめて直した。

| 回 | 指摘 | 必ず直す | 本番の振る舞いに関わる直し | 直しのコミット |
|---|---:|---:|---|---|
| 1 | 54 | 0 | 組の相手の移動量をずらしの当て直しの後の位置から取る・バルーンの切り替えで測り直した期限を消さない・選択肢のホバーの注入の記録を番号が変わったときだけ残す・切替の予約が在る間の受理を断る・予約の無い停止は `error!` の上で終了する | `210d3ce14`・`03230f27e`・`3ad5647f6` |
| 2 | 36 | 0 | 観測の欠けたフレームは計測に触れない・抑止の解けと計測の成立が同じフレームなら測り始めとして記録する・壊れた `lock`／`unlock` の荷物を warn に残す | `19c415fba`・`f6b5c90b4`・`df89eeb4f` |
| 3 | 34 | 0 | 止める前に基準の置き換えを記録する・動かない軸の置き換えは記録しない・MoveCueSink の 3 つの warn に `event` を足す | `543e54e32`・`6a5684a22`・`abaceeb9e` |
| 4 | 13 | 0 | 中断が箱へ届かないとき、窓の隠れた scope へ重ねて隠す指示と遷移の記録を出さない・置き換えの記録の文を「置き換えた」にする・どの軸も動かないときは基準の窓が無い記録を残さない | `7aaa4ff65`・`78c2d89cc` |
| 5 | 9 | 0 | なし（説明・テストの名・テストの補助だけ） | `41f75b9cc`（8 件） |

- 5 回目は「必ず直す」0 件・本番の振る舞いに関わる指摘 0 件なので、要件 9.6 のとおりここで打ち切った。残りの 9 件のうち 8 件は直した。直さずに残すのは 1 件:
  - `crates/areka-runtime/src/balloon_visibility_phase_zorder_chain_tests.rs` の `placement`・`spawn_scopes`・`set_directive` は `crates/areka-runtime/src/frame/zorder_drain_test_support.rs` の写し。寄せるには範囲の外の `frame/zorder_drain.rs` の支えの mod を開く必要があり、範囲の外なので触らない。
- 範囲の外で見つけて直さなかったもの: `crates/areka-runtime/src/talk_lifecycle.rs:211` のコメントが送出の後に印を立てる理由を `PresentBridge::send` と名指すが、理由は `PresentBridge::deliver` にある（範囲の中の `move_cue.rs` の同じ名指しは直した）。`take_down` の中の記録の並び（失敗の `error!` の後に「降ろした」の `info!`）は `audit-areka-install-update` へ申し送った（2026-11-23）。
- かけ直しで決めたこと（推奨案で確定・audit 系の扱いのとおり）:
  - `DisplayEndAt` が `TalkEnded` の後に届いたら計測を捨てる枝（`DisplayEndAdvanced`）は、送り手の約束の外の順への守りとして残し、説明をその理由に直した。
  - `LapRig.windows` は `pub` のまま（`pub(crate)` にすると `test-support` の組み立てで未使用の警告が出る。読み手は `cfg(test)` の中だけ）。
  - `ghost_switch.rs` の `switch_to`・`take_down`・`boot_into` の中身の中のコメントには触れなかった（範囲の外の持ち物）。

## 15. 仕上げの確かめ（全体テストと差分・2026-11-23）

### 15.1 差分の確かめ（要件 9.5・設計「仕上げの確かめ」4）

比べる先は `origin/main`（`1b3ede541`）。作業ブランチの分かれ目も同じ `1b3ede541` で、main は進んでいない。

| 確かめ | コマンド | 結果 |
|---|---|---|
| 触らないファイルの差分が空 | `git diff --stat origin/main...HEAD -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml' crates/dola/ crates/wintf/ vendors/ crates/log-capture-kit/tests/file_length_guard_test.rs crates/sample-ghost-kit/ crates/areka-session/src/body_systems.rs crates/areka/src/main.rs crates/areka-session/src/reload.rs crates/areka/src/update/` | 出力なし（差分 0） |
| 依存・フォーク・`[patch]`・`vendors/` | `git diff --stat origin/main...HEAD -- '*.toml' Cargo.lock vendors .cargo` | 出力なし（差分 0） |
| `COMPAT_ARCHITECTURE.md` | `git diff -U0 origin/main...HEAD -- doc/COMPAT_ARCHITECTURE.md` | 134・177・405・407 行の 1 行ずつの置き換えだけ（4 行追加・4 行削除・行数は変わらない）。405 行は 4.6 のテストの名の替えに合わせた記録済みの例外（13.4） |
| C26 のほかの 2 本の持ち物 | `git diff -U0 origin/main...HEAD -- crates/areka/src/install/overwrite.rs crates/areka-session/src/reload_test_support.rs crates/areka/src/runtime_tests/frame/` | `overwrite.rs` は腕の名 `SwitchVerdict::Updating` を 1 つ足しただけ（`audit-areka-install-update` の開発者確定・13.3）。`reload_test_support.rs`・`frame/switch_tests.rs`・`frame/switch_finish_tests.rs` は use の道（`areka_runtime::skin_switch::…`）と、`SwapFinish::Shell` の道の 1 か所だけ（`audit-areka-app` と申し合わせ済み） |
| 申し合わせの外で触った持ち物 | `git diff origin/main...HEAD -- crates/areka-session/src/body_systems_tests.rs` | `audit-areka-app` の持ち物（相手の brief の `src/body_systems*`）。`4cb2d09e5`（口の束の順のテストの期待の並びに `"update_busy"` を 1 つ足し、説明に「メニューからの切替要求は `update_busy` を 1 回読み」を足す）と `7ddc891d2`（`request_skin_switch_with(…, \|_\| 0)` を `request_skin_switch(…)` へ呼び替え、取り込みも替える）の 2 つ。理由は要件 7.7（形を変えた所の読み手を同じ変更で直す）: 前者は 2.2 で入口が `update_busy` を読むようになった並び、後者は 5.1 で `request_skin_switch_with` を私有にした形に追う。申し合わせの 4 か所に入っていなかったので、2026-11-23 に `audit-areka-app` のセッションへこの 2 つの直しを知らせた（13.3） |
| 範囲の外の差分 | `git diff --name-status -M origin/main...HEAD` を範囲の名の型と突き合わせ | 下の 15.2 |
| 外への約束の変化 | 本番コードの追加行の `warn!`・`error!`・`info!` と公開の面の差分 | 下の 15.3 |
| 端末へ出す文 | 追加行の `println!`・`eprintln!`・`print!`・`dbg!` を grep | 0 件 |
| テストの根 | 追加行の `C:\`・`temp_dir`・`/tmp` を grep | 1 か所だけ: `special_name.rs` の `catalog_entry` の偽の目録の道 `C:\ghost-switch-tests\…`。移す前の `ghost_switch_tests.rs` にあった同じ文字列の移し替えで、fs を見ない（開かない・書かない）。テストの根は今までどおり `temp-path-kit`・`sample-ghost-kit` の `target\` の下 |

### 15.2 範囲の外のファイル（設計「範囲の外（持たないもの）」との突き合わせ）

範囲の外で差分のあるファイルは次のとおり。

- 一覧どおり: `areka-runtime/src/special_name.rs`（`catalog_entry` と兄弟のテストの宣言）・`special_name_tests.rs`（新）・`lib.rs`・`frame_persist_test_support.rs`・`shell_balloon_resolve.rs`、`areka-placement/src/` の 4 本（`windowposition.rs`・`follow/offset_space.rs`・`balloon_offset_supply_tests.rs`・`follow_offset_space_tests.rs`）と `lib.rs`（`i32::MIN` の素通しに合わせた doc の書き換えだけ）、`doc/COMPAT_ARCHITECTURE.md`、C26 の 2 本の持ち物（15.1）。
- 記録済みの判断による: `install/overwrite.rs`（腕の名・13.3）。
- 本体の `runtime_tests/` の 3 本（`frame_balloon_timeout_notice_e2e_tests.rs`・`frame_shell_box_integration_tests.rs`・`frame_visibility_integration_tests.rs`）は、可視性の相のテストを `areka-runtime` へ移したことで古くなった**コメントのファイル名の指し先**の書き換えだけ（コードの行は 0）。
- `areka-session/src/body_systems_tests.rs` は `audit-areka-app` の持ち物（相手の brief の `src/body_systems*`）。`4cb2d09e5` で期待の並びに `"update_busy"` と説明の 1 句を足し、`7ddc891d2` で `request_skin_switch_with` から `request_skin_switch` へ呼び替えた。どちらも要件 7.7 の読み手の直し（2.2 の入口の並びと 5.1 の私有化に追う）。申し合わせた use の道の 4 か所の外なので、2026-11-23 に `audit-areka-app` のセッションへこの 2 つを知らせた（15.1・13.3）。`body_systems.rs` 本体は触っていない。
- 範囲の中: `balloon_hover.rs`・`shell_box_hover.rs`・`shell_box_hover_tests.rs`・`hover_inject.rs`・`change_cue.rs`・`skin_switch.rs`・`switch_assets*`・`move_cue*`・`balloon_visibility*`・`ghost_switch*`・`shell_balloon_switch*` と、本体の `session_tests/shell_balloon_switch*`・`runtime_tests/balloon_visibility*`（移して消した 2 本を含む）。
- `.kiro/specs/audit-emo2-boot-switch/` の 6 ファイルは spec の文書。
- 説明のつかない差分: なし。

### 15.3 外への約束の変化（要件 7.6・7.7・設計「見直しが要る変化」）

比べたのは `git diff origin/main...HEAD` の本番コード（`*tests*`・`*test_support*` を除く `crates/**/*.rs`）。出どころは、設計・タスクで決めたもの（タスク番号）と、かけ直しの回で直したもの（14 節の回と直しのコミット）に分けた。読み手は `git grep -F <語> -- crates tools doc .claude/skills` で数えた。

**ログの水準と文の変化**

| 変化 | 出どころ | 読み手 |
|---|---|---|
| `ghost_switch_no_reservation` を `on_ghost_stopped` で `debug!` → `error!`、`set_stage` で `warn!` → `error!`。文は「切替の予約が無い停止通知——今日どおり終了する」→「切替の予約が無いのに停止通知を切替の段で捌く指示が来た——そのまま終了する」、「切替の予約が無い——段は記録しない」→「切替の予約が無いのに段を進める指示が来た——段は記録しない」 | かけ直し 1 回目（`3ad5647f6`） | 語は `ghost_switch.rs` 4・`ghost_switch_notice_tests.rs` 2（水準を `error` と読む表明・同じ変更で直した）。古い 2 つの文はどこにも残っていない（0 件） |
| `ghost_switch_not_accepted` の文の「今日どおり終了する」→「そのまま終了する」 | かけ直し 1 回目（`3ad5647f6`） | 文の読み手 0 件（語は変えていない） |
| `move_frame_substituted` の文「`\![move]` の基準を置き換えて動かす」→「`\![move]` の基準を置き換えた」。止める（`skipped`）前にも出す。動かない軸（`Fix`）の置き換えは記録しない | 文はかけ直し 4 回目（`78c2d89cc`）、止める前の記録と動かない軸はかけ直し 3 回目（`6a5684a22`）と 4 回目（`78c2d89cc`・どの軸も動かないときの基準の窓が無い記録） | 古い文 0 件・新しい文は `move_cue_place.rs` 2 だけ。語は `move_cue_geometry.rs` 1・`move_cue_place.rs` 2・`move_cue_place_tests.rs` 3・`doc/COMPAT_ARCHITECTURE.md` 1・`doc/ukadoc-coverage/ledger/sakura-script.toml` 1（どちらも語と振る舞いを書くだけで文を読まない） |
| `MoveCueSink` の 3 つの `warn!` に `event` を足した: `move_cue_unopenable`（自分宛の非正準の荷物・`lock`／`unlock` の壊れた荷物の warn を新しく足した）・`move_cue_bad_actor`・`move_cue_send_failed`。文から要件の番号（「D8④」「R5.5」）を消した | `event` はかけ直し 3 回目（`6a5684a22`）、`lock`／`unlock` の warn はかけ直し 2 回目（`19c415fba`） | `move_cue_unopenable` は `move_cue.rs` 1・`move_cue_move_severity_log_tests.rs` 1、`move_cue_bad_actor` は `move_cue.rs` 1・`move_cue_balloon_lock_tests.rs` 1、`move_cue_send_failed` は `move_cue.rs` 1・`move_cue_move_sink_tests.rs` 1（範囲の中のテストだけ）。古い文「MoveCueSink: 自分宛（move）」は 0 件 |
| 新しい `warn!`: `ghost_switch_updating`（更新中のメニューの切替要求）と `skin_switch_installing`（インストール中のメニューのシェル・バルーンの切替要求） | 2.2（`68c8e1069`）と 1.3（`e67a74b40`） | `ghost_switch.rs` 2・`ghost_switch_tests.rs` 1、`shell_balloon_switch.rs` 1・`shell_balloon_switch_tests.rs` 2 |
| `ghost_switch_busy`・`ghost_switch_no_context` は判定の場所を寄せた（予約の判定の `reserved`・文脈の無い記録の `no_boot_context`）だけで、語と文は変わらない。予約が在る間は読み直しの窓口から直に受理を呼ばれても断る | かけ直し 1 回目（`3ad5647f6`） | 語は変わらないので読み手の直しは要らない |

**可視性の振る舞いの変化**

- 満了の後の抑止の間は起床を預けない（1.1・`9750cd79d`）。
- `ScopeVisibility::forget_scope` はバルーンの差し替えで期限（`deadline`）を消さない（かけ直し 1 回目・`03230f27e`）。`pub(crate)` に狭めた（3.3・`be4259528`）。
- 文字の観測が取れなかったフレーム（どれかの scope の `visible_glyphs` が `None`）は計測に触れない（かけ直し 2 回目・`f6b5c90b4`）。
- 抑止が解けたときに期限がまだ無ければ測り直さず、初めの成立を下の確立の枝に任せる（同じフレームなら測り始めとして記録する・かけ直し 2 回目・`f6b5c90b4`）。
- 利用者の中断が箱へ届かない巡では、窓の隠れた・箱だけに文字が出ている scope を隠す対象にしない（かけ直し 4 回目・`78c2d89cc`）。
- `hover_inject` の注入の `info!` は actor ごとに番号が変わったフレームだけ（かけ直し 1 回目・`03230f27e`）。
- 可視性の相のテスト 29 本が本体から `areka-runtime` へ移った（3.2・`ff4a59133`）。

**移動の振る舞いの変化**

- 組の相手の移動量と `move_applied` の `to` を、面の `move` のずれを当て直した後の窓の位置から取る（かけ直し 1 回目・`210d3ce14`）。

**配置の変化**

- `scale_signed(i32::MIN, k)` は `i32` に収まる答えを素通しで返す（恒等比なら `i32::MIN`）。収まらない答えの飽和（`±i32::MAX`）は変えない（5.3・`fba7c4f7e`）。

**狭めた公開の面**（どれも 3.1・3.3・5.1 の設計どおり。外の使い手は下の grep で 0 件）

- `areka-runtime`（3.3・`be4259528`）: `balloon_visibility` の `decide` の出し直しを消し `decide` を `pub(super)` に。`VisibilityTrigger`・`SuppressionKinds`・`MeasurementDiscardReason`・`VisibilityLogEvent`・`MeasurementOrigin`・`VisibilityAction`・`GlyphObservation`・`VisibilityObservations`・`ScopeVisibility` と、`ScopeObservation`・`VisibilityDecision` の欄を私有・`pub(crate)` に。`balloon_visibility::phase` の `test_open` の出し直し（`IssueOutcome`・`issue_actions`・`clear_hover_residency`）を消した。`move_cue` の `BaseposResolver`・`CanonDefaultBasepos` の出し直しを私有の取り込みに（型シームは残す）。`hover_inject` を `pub(crate) mod` に、`shell_box_hover::settle_box_hover` を `pub(crate)` に。
- `areka-session`（3.1・`70bb10c2a`）: `ghost_switch` の `NameResolution`・`resolve_special_name` の出し直しを私有に（`LastInstalledGhost` は残す）。`shell_balloon_switch` の `areka_runtime::skin_switch` の出し直しを 5 型に、`SwapFinish` の出し直しを消した。`request_ghost_switch_with`・`end_switch`・`switch_to_default`・`drain_switch_requests` を `pub(crate)` から私有に。
- `areka-session`（5.1・`7ddc891d2`）: `request_skin_switch_with` を `pub(crate)` から私有に（読み手の `body_systems_tests.rs` は 15.2）。
- 外の使い手の grep（定義の crate の外の `crates`・`tools`）: 上の型と関数はどれも 0 件。名の一致は `CanonDefaultBasepos` の本体 `spine_move_cue_tests.rs` のコメント 3・`areka-update` の別の型 `FetchError::NameResolution`・`reload_tests.rs:507` のコメントの `end_switch`・本体 `switch_finish_tests.rs:46` の `areka_runtime::skin_switch::SwapFinish`（申し合わせた use の道）だけで、どれも狭めた項目を使っていない。`hover_inject` を `areka_runtime::hover_inject` か `crate::hover_inject` で引く所は `areka-runtime` の外に 0 件。

**足した項目**

- `SkinVerdict::Installing`（1.3・`e67a74b40`・`areka-runtime/src/skin_switch.rs`）と `SwitchVerdict::Updating`（2.2・`68c8e1069`・腕の名を `install/overwrite.rs` に 1 つ・13.3）。入口の断りの並びは 進行中 → ゴースト切替中 → 更新中 → インストール中 → 文脈。
- `special_name::catalog_entry`（`#[cfg(any(test, feature = "test-support"))]` の `pub fn`・fs を見ない偽の目録の 1 項目）。かけ直し 1 回目（`3ad5647f6`）で `special_name_tests.rs` と `ghost_switch_tests.rs` の写しを寄せた先。読み手はこの 2 本だけ（`ukadoc-survey` の同じ名は別の関数）。

**設計が確かめを求めた 2 つ**

- `balloon_visibility::phase::test_support`（`balloon_visibility_phase_test_support.rs`）で `pub` なのは `notify_timeout` だけ。ほかは `attach_headless`・`runtime` が `pub(in crate::balloon_visibility)`、`world_at`・`visibility_lines` が `pub(super)`。`areka-runtime` の外の使い手は本体 `runtime_tests/balloon_visibility_phase.rs:4` の `notify_timeout` の 1 か所だけ。
- 配置の後の段は `i32::MIN` を受けても回り込まない（`git grep saturating_` で確かめた・数は本番のファイルの中の出現）: `areka-placement/src/follow/move_shift.rs` 8（ずれの差し引き）・`windowposition.rs` の `apply_windowposition`（`saturating_add`）・`resolver.rs` 27（P5 の加算と `keyword_balloon_pos` の `saturating_add`）・`persist.rs` 10（`char_pos + balloon_offset`）・`areka-runtime/src/move_cue_geometry.rs` 11。`areka-placement/src/lib.rs` の `apply_author_balloon_offset_scale` は換算した値を `balloon_offset` に入れるだけで加算をしない（飽和の記録は `warn_if_saturated`）。

**ほか**

- 本番コードの追加行に、上のどれにも当たらない新しい公開の面・環境変数・ファイル・端末への出力はない。

### 15.4 全体テスト（要件 9.4・4.4・4.5）

- コマンド: `pwsh -NoProfile -File tools/test-all.ps1`（コミット `3d10ec039`・開始時の未コミットの変更は `research.md` の 1 件だけ）。1 回だけ回し、終了コード 0・`all steps green`。赤の段は 0 なので、回し直しはしていない。
- 21 段すべて OK（秒）: sample fetch 3・add i686 target 0・build i686 artifacts 1・fmt --check 15・x64 workspace tests 684・dola toml feature tests 34・i686 tests (host-32) 42・crates.io pre-publish check (package only) 4・encoding check 25・release notes check 9・roadmap overview check 11・i686 helper sync check 14・sample fetch check 3・package check 14・exe install check 5・load flake check 9・mouse input check 2・app icon check 6・cursor art check 44・perf loop selftest 130・i686 helper beside areka.exe 1。
- 件数: x64 workspace tests と i686 tests の出力にある `test result:` の行を合算した。`test result: ok` が 180 行、失敗（failed）は全行で 0、合算は passed 15,061・ignored 82。
- emo2・pasta の実走は全体テストの中だけで回った。SSP の測りと画面の操作は回していない。

## 付録 A: 束の読み手の番号との対応

| 番号 | 束の番号 |
|---|---|
| 1 | B4-1 |
| 2 | B4-2 |
| 3 | B3 の 5 節の注 4（範囲の中を指していた） |
| 4 | B1-1・B2-14 |
| 5 | B1-2 |
| 6 | B1-3 |
| 7 | B4-5（件 3 の読み手の表） |
| 8 | B1-6 |
| 9 | B1-7 |
| 10 | B4-3 |
| 11 | B6-3 |
| 12 | B6-4 |
| 13 | B5-3・B3-6・B6-5・B4-4・B2-4・B5-4・B6-9 |
| 14 | B2-5 |
| 15 | B6-1 |
| 16 | B2-1・B2-2 |
| 17 | B5-1 |
| 18 | B5-2 |
| 19 | B6-6 |
| 20 | B2-6 |
| 21 | B5-5 |
| 22 | B5-7 |
| 23 | B1-5 |
| 24 | B3-2・B3-4（B3 の 5 節の注 2） |
| 25 | B3-1 |
| 26 | B3-5 |
| 27 | B6-2 |
| 28 | B1-4 |
| 29 | B2-3 |
| 30 | B3-10 |
| 31 | B2-7（B1 の 5 節の `ghost_switch_tests.rs:53`） |
| 32 | B2-8 |
| 33 | B5-6 |
| 34 | B1-8 |
| 35 | B4-10 |
| 36 | B4-6 |
| 37 | B6-12 |
| 38 | 件 3 の読み手の 4 節・5 節 |
| 39 | B1-18 |
| 40 | B3-3 |
| 41 | B5-8 |
| 42 | B5-9 |
| 43 | B5-10 |
| 44 | B6-7 |
| 45 | B6-8 |
| 46 | B6-13 |
| 47 | B2-12 |
| 48 | B2-13 |
| 49 | B2-9 |
| 50 | B2-10 |
| 51 | B2-11 |
| 52 | B1-9（B2・B3 の 5 節の注にも） |
| 53 | B1-10 |
| 54 | B1-11 |
| 55 | B1-12 |
| 56 | B1-17 |
| 57 | B3-11 |
| 58 | B1-13 |
| 59 | B1-14 |
| 60 | B1-15 |
| 61 | B1-16 |
| 62 | B2-15 |
| 63 | B3-7 |
| 64 | B3-8 |
| 65 | B3-9 |
| 66 | B4-7 |
| 67 | B4-8 |
| 68 | B4-9 |
| 69 | B5-11 |
| 70 | B5-12 |
| 71 | B5-13（B5 の 5 節の注） |
| 72 | B6-10 |
| 73 | B6-11 |
