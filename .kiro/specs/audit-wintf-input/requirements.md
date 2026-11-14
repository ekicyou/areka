# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（起票のときは 5.3.2・着手のときは 5.3.3・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **wintf のドラッグ・ポインタ・クリック透過**（`crates/wintf/src/ecs/` の `drag/`・`pointer/`・`clickthrough/`）を受け持つ。ツールチップは 2026-11-01 の開発者の決め（「境界領域です。リアクター描画にした方が良い可能性がある。WinUIで描画できる要素ですからね」）で範囲から外した。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。

## Introduction

この文書は、wintf のドラッグ・ポインタ・クリック透過のコードに `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直すための要件を定める。要件の段では「全部直す」という形だけを書き、指摘の中身は書かない。指摘の一覧はギャップ分析の報告に残し、要件の討議でこの文書に落として直す対象を確定する。

受け持つ範囲の大きさ（着手のときの測り直し・`main` `30a343eb`）:

| 測った時点 | 範囲 | ファイル | コード | テスト |
|---|---|---|---|---|
| 起票のとき（`e3c2cbe4`・brief の値） | `drag*`・`pointer*`・`clickthrough*`・`tooltip*` | 37 | 8,848 行 | 5,058 行 |
| 着手のとき（`30a343eb`） | `drag/`・`pointer/`・`clickthrough/`（ツールチップを外した） | 24 | 6,681 行 | 1,212 行 |

テストに数えたのは、名前が `_tests.rs` で終わるファイル・`tests/` の下のファイル・名前が `_test_support.rs` で終わるファイル（起票のときと同じ数え方）。この数え方では、名前がちょうど `tests.rs` のテストのファイル 3 本（`drag/state/tests.rs` 848 行・`pointer/dispatch/tests.rs` 560 行・`pointer/types/tests.rs` 326 行・計 1,734 行）はコードの側に入る（起票のときの 8,848 行も、同じ 3 本の当時の 1,695 行をコードに入れた値＝`e3c2cbe4` で測って一致を確かめた）。3 本をテストに数え直すと、コード 4,947 行・テスト 2,946 行。コードのファイルのうち 15 本は、中に `#[cfg(test)]` のテストを持つ（行はコードに数えた）。

受け持つ 24 ファイル:

- `crates/wintf/src/ecs/drag/`（11 本）: `mod.rs`・`accumulator.rs`・`accumulator_tests.rs`・`capture_guard.rs`・`context.rs`・`dispatch.rs`・`reentry.rs`・`reentry_tests.rs`・`systems.rs`・`state/mod.rs`・`state/tests.rs`
- `crates/wintf/src/ecs/pointer/`（8 本）: `mod.rs`・`buffers.rs`・`nchittest_cache.rs`・`systems.rs`・`dispatch/mod.rs`・`dispatch/tests.rs`・`types/mod.rs`・`types/tests.rs`
- `crates/wintf/src/ecs/clickthrough/`（5 本）: `mod.rs`・`controller.rs`・`controller_tests.rs`・`monitor.rs`・`registry.rs`

（`drag/` 11 本＋`pointer/` 8 本＋`clickthrough/` 5 本＝24 本）

1,000 行に近いのは `drag/state/tests.rs`（848 行）・`clickthrough/controller_tests.rs`（702 行）・`pointer/buffers.rs`（682 行）・`drag/state/mod.rs`（650 行）。範囲の中に、1 ファイル 1,000 行の番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`・範囲の外）に載るファイルは 0 本。

## 確定した指摘の一覧（要件 1.5・2026-11-14 要件の討議）

指摘の中身はギャップ分析（`research.md` 3 節）が正本。番号は audit の番号。議題 1〜10 は推奨案で確定した（2026-11-04 開発者「audit系は推奨案でよい」の常の承認による）。議題 11 は `audit-wintf-window` との申し合わせで、あちらの要件の討議で開発者が承認した。**残すと裁定した指摘は 0 件、ほかの spec へ回した指摘も 0 件**＝23 件とも、この spec で直す。

| 組 | 指摘 | 扱い |
|---|---|---|
| 必ず直す | 1（1 回の配りにドラッグの終わりと次の始まりが重なると取り違える） | 直す（議題 5） |
| 直したほうがよい | 2〜8・12・13 | 直す |
| 直したほうがよい | 9（使われないコード） | 直す。⑸ ⑹ は消す（議題 2）。⑴ `check_threshold` は消さずに残す（議題 11） |
| 直したほうがよい | 10（速さと時刻を誰も読まない・座標の f32 往復） | 直す＝消す（議題 3） |
| 直したほうがよい | 11（`local_point` が窓の中の座標） | 直す＝本当のローカル座標を入れる（議題 4） |
| できれば | 14・16・17・20・22・23 | 直す |
| できれば | 15（`initial_inset` の説明と名前） | 直す＝説明を直し、名前も改める（議題 7） |
| できれば | 18（`DragStateSnapshot` の写し） | 直す（議題 8） |
| できれば | 19（`OnPointerMoved` を毎 tick 配る） | 直す（議題 9） |
| できれば | 21（`Window` を外した窓が対象に残る） | 直す＝汎用の対象をやめる（議題 10） |

議題ごとの決め:

1. **切り方（要件 8）**: 切らない（見込み 14〜20 タスク・どの指摘も同じ 3 つのフォルダの同じファイルに触る）。タスクを作って 20 を越えたら、そのとき 2 本目へ切ることを議題にし直す。
2. **右・中ボタンのドラッグの設定と `is_primary`（指摘 9 ⑸ ⑹）**: 消す。完了した `event-drag-system` の要件 1.2・1.5・1.6（左・右・中のどのボタンでもドラッグ・ボタンの識別・ボタンごとの設定）を閉じる裁定（要件 3.1 の 1 種類目）。理由＝今も左しか作られておらず、`true` にしても何も起きない設定は使う人を迷わせる。右・中のドラッグが要るときは新しい spec で作る。`window_proc/` のテスト 1 行と `areka-placement` のテスト 5 か所に触る＝要件 5.2 の申し合わせ（2026-11-14 `audit-placement-follow`・`audit-placement-core` と「後から着地する側が読み手を直す」で合意）。消すのはドラッグの知らせ 3 つ（`DragStartEvent`・`DragEvent`・`DragEndEvent`）の `is_primary` だけで、`MonitorRecord`・`Monitor` の `is_primary`（`diag.rs:382` ほか・C20 の約束 ⑶）は別物なので消さない。
3. **ポインタの速さと時刻（指摘 10）**: 消す。完了した `event-mouse-basic` の要件 3（速さと 5 件の履歴）を閉じる裁定（1 種類目）。理由＝その要件の目的（撫でる操作の見分け）は、後の決め（`roadmap-history.md:255`「撫で＝OnMouseMove 連打の解釈は SHIORI 側の領分」）で本体の仕事でなくなった。速さが要るようになったら、ドラッグの知らせの位置から求める。
4. **ローカル座標（指摘 11）**: `local_point` に、当たった entity の左上を引いた本当のローカル座標を入れる（完了した `event-mouse-basic` の要件 4 どおり・公開の形は変わらない）。窓の手続きの作る所を揃えるので `audit-wintf-window` と申し合わせる。
5. **指摘 1 の後始末**: `cleanup_drag_state` を消し、配りの終わりの腕で外す今の形を正本にする（`OnDragEnd` の受け手が `DraggingState` を読めることが下流の約束＝`drag_follow.rs:114-116`）。
6. **wintf の公開の形を減らす直し（9 ⑶ ⑸ ⑹・10・22）**: 減らしてよい（wintf は 0.x の版で v0.2.0 の作り直しの途中・使い手はツリーの中だけで grep で確かめられる）。C20 の約束 ⑶ の名には触らない。
7. **`initial_inset` の改名（指摘 15）**: この spec で `initial_window_pos` へ改め、範囲の外の使い手（`areka-placement`・`areka` の `runtime/frame/dpi.rs`・窓の手続き）まで直し切る。走っている `audit-placement-follow`・`audit-wintf-window` とは要件 5.2 で申し合わせる（ギャップ分析の推しは「説明だけ直して改名は後へ」だったが、後へ回す持ち主の spec が無く要件 2.2 に反するため、並走の都合で回避せず調整する形にした）。
8. **`DragStateSnapshot` の写し（指摘 18）**: この spec で直す（捕捉の守りを列挙の外へ出し、写しの列挙と写す関数を消す）。写しの読み手のうち窓の手続き（`mouse_move.rs` ほか）は `audit-wintf-window` と申し合わせる（議題 7 と同じ理由で後へ回さない）。
9. **毎 tick の `OnPointerMoved`（指摘 19）**: 直す＝その tick に動いた entity にだけ配る。要件 3.1 の 3 種類目には当たらない: 本体の間引き `plan_mouse_move`（`crates/areka/src/input_events/throttle.rs:54-66`）は位置が同じ呼び出しで送らないので、SHIORI へ送る `OnMouseMove` の数は変わらない（ukadoc の原文「マウスが移動した際に発生。」<https://ukatech.shillest.net/ukadoc-spec/list_shiori_event.html#OnMouseMove> にも近づく向き）。
10. **`Window` を持たない汎用の対象（指摘 21）**: やめる。登録するのは wintf の窓だけにし、`WindowHandle` の HWND と一致しない対象は刈る。逆を固定するテスト（`controller_tests.rs:513-525`）は向きを直す。
11. **`audit-wintf-window` との分け方（2026-11-14 あちらの要件の討議で開発者が承認・正本はあちらの `requirements.md` の「確定した指摘の一覧」）**:
    - 指摘 9 ⑴: `check_threshold` は消さずに残す。あちらの指摘 62 が、窓の手続きの中の閾値の計算の写し（`mouse_move.rs:228-233`）をこの関数の呼び出しに替えて写しを無くす。こちらは関数とその 5 本のテストを残し（指摘 7 の「消すテスト」から外す）、行き先の無い参照「P62」などの古い説明だけを直す（指摘 16）。
    - 窓の手続き（`ecs/window_proc/mouse_*`）の作りの直しはあちらが持つ（あちらの 45・46・61〜63）。こちらがあちらのファイルに入れるのは、型の形が変わったことへの機械的な追従（欄の名・引数の型・構築の式の 1 欄）だけ。そのため議題 4 のローカル座標は、窓の手続きの作る所を変えずに、範囲の中（ポインタの状態を配る前）で引いて入れる形にする。
    - 着地は先に着いた方がそのまま入れ、後の方が rebase して追従する。
    - `WindowPointerTracking`（`pointer/types/mod.rs:206`・`types/tests.rs:122-125`）は、あちらの指摘 6 の直しで使い手が無くなるので、あちらが型ごと消す（範囲の中のファイルだが、実装の前にもう一度申し合わせる）。

## Boundary Context

- **In scope**:
  - 上の 24 ファイルに audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 5 と要件 6 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。名前に drag・pointer・clickthrough を含んでも、次のファイルは範囲の外:
    - `crates/wintf/src/ecs/tooltip/` の 13 ファイル（2026-11-01 の開発者の決めで外した・リアクターの描画に替える候補で、どの見直しの spec も受け持たない）。
    - `crates/wintf/src/ecs/window_proc/mouse_move_drag_origin_tests.rs`（`ecs/window*` を受け持つ `audit-wintf-window` の持ち物・同じウェーブ C20 で走っている）。
    - `crates/wintf/src/runtime/wndproc_bridge_drag_cancel_tests.rs`・`wndproc_bridge_drag_release_tests.rs`・`wndproc_bridge_drag_test_support.rs`、`crates/wintf/tests/drag.rs`・`crates/wintf/tests/drag/`、`crates/wintf/examples/taffy_flex_demo/drag.rs`（`ecs/` の外を受け持った `audit-wintf-runtime` の持ち物・完了済み）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 3）。
- **Adjacent expectations**:
  - 範囲の型や関数を使う範囲の外のコードは、着手のときの grep で `crates/areka/`（約 66 ファイル・入力の受け口 `input_events/` ほか）・`crates/areka-placement/`（約 35 ファイル）・`crates/areka-emo-text/`（約 3 ファイル）・`crates/pilot/`（約 2 ファイル）・wintf の範囲の外（約 28 ファイル・`ecs/window*`・`runtime/` ほか）。
  - 同じウェーブ C20 で走っている spec のうち、直しが及びうるのは `audit-wintf-window`（wintf の `ecs/window*`）・`audit-placement-core` と `audit-placement-follow`（`crates/areka-placement/`）・`wintf-visual-damage-redraw`（wintf の `ecs/graphics/visual/draw/`・`ecs/graphics/composite.rs`・`ecs/graphics/systems/render.rs`・`numerics/matrix3x2.rs`）。これらのファイルに直しが及ぶときは要件 5.2 に従う。
  - ギャップ分析で範囲の外に見つけた 2 つ（窓の手続き `mouse_move.rs:228-233` の閾値の計算の写し・`mouse_click.rs:150-156` が押すたびに出す `info!`）は `audit-wintf-window` の持ち物として、そのセッションへ伝えた（2026-11-14）。この spec では直さない（要件 5.4）。
  - v0.2.0 の `instant-to-interrupt-ticks` は、この見直しと触るファイルが重なり、この見直しの着地を待つ（roadmap の spec 台帳）。この spec の側に約束は無い。

## Requirements

### Requirement 1: 指摘の洗い出し

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になっていること, so that 直す対象を要件の討議で確定できる

#### Acceptance Criteria

1. When ギャップ分析を行う, the 本見直し shall 上の 24 ファイルだけを対象にして `/ponytail:ponytail-audit` をかける
2. The 本見直し shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
3. The 本見直し shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない
4. The 本見直し shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたまま、ギャップ分析の報告に残す
5. When 要件の討議で直す対象が確定した, the 本見直し shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか）をこの文書に書き加える

### Requirement 2: 指摘を全部直す

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that ドラッグ・ポインタ・クリック透過のコードからバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本見直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本見直し shall 要件 3 の 3 種類のどれかに当たって開発者が残すと裁定した指摘のほかは、直さずに残さない
3. When 指摘の直しで消す・まとめる・形を変える箇所が出た, the 本見直し shall その箇所を使うテスト・検体・呼び出し元まで直し切り、直しの途中の状態を残さない

### Requirement 3: 直さずに残す指摘は開発者の裁定で決める

**Objective:** As a 開発者, I want 直さずに残す指摘を自分で裁定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. If 指摘の直しが次の 3 種類のどれかに当たる, the 本見直し shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
2. When 3 種類目に当たるかを判断する, the 本見直し shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする
3. If 指摘の直しが 3 種類のどれにも当たらない, the 本見直し shall その指摘を残すことを議題にせず直す
4. When 開発者が指摘を残すと裁定した, the 本見直し shall 裁定とその理由をこの文書に残し、要件 4 の「新しい指摘 0 件」の数から外す

### Requirement 4: 直した後の確かめ

**Objective:** As a 開発者, I want 直しが新しい問題を生んでいないことを確かめられること, so that 見直しが終わったと安心して言える

#### Acceptance Criteria

1. When 確定した指摘を全部直し終えた, the 本見直し shall 同じ範囲に、要件 1 と同じ作法（件数の上限なし・全部読む）で `/ponytail:ponytail-audit` をもう一度かける
2. If もう一度かけた audit が新しい指摘（直しが生んだ指摘を含む）を出した, the 本見直し shall その指摘も直し、新しい指摘が 0 件になるまで audit をかけ直す
3. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無い, the 本見直し shall 開発者に一言知らせたうえで、5 回目の指摘を直したところでかけ直しを打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書く（2026-11-06 `audit-seriko` で開発者が承認した形）
4. The 本見直し shall かけ直した audit の結果（回ごとの組ごとの件数と、0 件または打ち切りの判定）を検証の記録に残す
5. When 見直しを終える, the 本見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる
6. The 本見直し shall 直したバグと危ない所の 1 件ずつに、その直しが壊れたら落ちるテストを 1 本以上置く
7. The 本見直し shall 足したテストと直したテストを、毎回同じ結果になる（決定論の）テストにする

### Requirement 5: 範囲の外へ直しが及ぶとき

**Objective:** As a 開発者, I want 範囲の外への直しが安全に、並んで走るほかの spec とぶつからずに入ること, so that ほかのクレートが壊れず、並行作業が止まらない

#### Acceptance Criteria

1. When 直しで関数や型の形を変えた, the 本見直し shall ツリー全体で呼び出し元を探して、全部直す
2. If 直しが及ぶ呼び出し元の範囲を受け持つ spec のセッションが走っている, the 本見直し shall そのセッションと申し合わせてから、その呼び出し元に触る
3. When コード・テスト・検体を消す, the 本見直し shall 消す前に、ツリー全体（テスト・検体・設定・文字列での参照を含む）で使われていないことを確かめる
4. The 本見直し shall 範囲の外のファイルの、直しに要らない箇所を直さない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）
5. The 本見直し shall `crates/wintf/src/ecs/tooltip/` を、範囲の型や関数の形を変えたことで要る呼び出し元の直しのほかは変えない

### Requirement 6: C20 の並走の約束

**Objective:** As a 開発者, I want 見直しの直しが同じウェーブ C20 のほかの spec の邪魔をしないこと, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. The 本見直し shall `areka-placement` が使う wintf の型と記録の綴り（`MonitorRecord`・`Monitor`・`DESPAWNED_SKIP_TAG`・記録の的 `areka_placement::diag`）を、足すことのほかは変えず、消さない（C20 の約束 ⑶）
2. The 本見直し shall 依存を足さず、`Cargo.lock` を変えない（C20 の約束 ⑸＝`Cargo.lock` の席は `wintf-winmsg-executor-import`）
3. If 直しに新しい依存が要ると分かった, the 本見直し shall 依存を足さずに開発者へ議題として出す（足すなら `tech.md` への登記と開発者の承認が要る）
4. The 本見直し shall `vendors/` の下に手を入れない

### Requirement 7: 守る決まり

**Objective:** As a 開発者, I want 見直しの直しがプロジェクトの決まりを守ること, so that 直しが別の決まりを破らない

#### Acceptance Criteria

1. The 本見直し shall 1 ファイル 1,000 行の番人を赤にせず、直しで行が上限を越えるときは新しいファイルへ分け、番人の例外表へ新しいファイルを足さない
2. The 本見直し shall 記録の無い失敗の道を作らず、失敗は記録（`error!`）を残したうえで呼び出し元へ失敗（`Err`）として返す
3. The 本見直し shall 終了の道を正規の実装にし、終了を早めるための小細工を入れない
4. The 本見直し shall 実機の根と一時フォルダを、ワークツリーの `target` の下に置く
5. The 本見直し shall GPU のテストを実 GPU で回す
6. If 文字化けが起きた, the 本見直し shall コンソールの文字コードを変えて逃げず、読む側を直す

### Requirement 8: spec の大きさ

**Objective:** As a 開発者, I want 1 本の spec が大きくなりすぎないこと, so that 1 本ずつ確かめて着地できる

#### Acceptance Criteria

1. If 確定した指摘の直しが 20 タスクを越えそうになった, the 本見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切ることを議題にする
