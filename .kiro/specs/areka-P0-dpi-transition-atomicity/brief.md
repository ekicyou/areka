# Brief: areka-P0-dpi-transition-atomicity

> **起票 2026-08-01**（`/kiro-discovery` 再入・`areka-P0-dpi-window-vanish` の task 4.7 実機セッション②-b 中に開発者が「スケール切り替え時に一瞬ガクッとする」と観測）。
> 本 brief は**実測証跡・踏査結果・所有権判断を全て内包**する。別セッションはこの brief 単体で再開できる（会話ログは不要）。
> **機序は未確定である。** 本 brief は仮説を仮説として書く。断定している箇所は実測引用を必ず添えてある。

> **📌 2026-08-06 追記(60)再実測（棚卸⑥・col=collision-dpi-hittest PR#100 マージ後）**: 本 spec の全アンカーは現物一致を再確認（dpi 相 :796-950 帯・`run_dpi_phase` :976・`resnap_shell_targets` :1305/:1488・`flush_window_pos_commands` command.rs :210）。干渉先の presenter.rs `apply_show` は col で +17 シフト＝現在 **:360-614**（budget 檻点 :394-409・cage④ :527-531）。requirements 着手（W6 文書同居）の条件に変更なし。
>
> **📌 2026-08-01 追記(58)陳腐化補正（棚卸⑤・W5 3本マージ後の実測・本ブロックが以下の本文より優先）**:
> - **着手ゲート開放**: van（`dpi-window-vanish`）マージ済（PR#98・main `ec9687c`）＝「van 5.1/5.2 着地後に再観測」の待ち条件が**充足**。**第1段再観測は本 spec の requirements フェーズの research として実施する**（spec 外の作業経路は本プロジェクトに存在しない）。`/kiro-start` は要件討議まででコードに触れないため **W6 の 5 本と文書フェーズで同居可能**——**design 以降は W6.75 まで進めないこと**（presenter.rs `apply_show`・frame.rs dpi 相で col/vis と衝突）。
> - **S1 是正着地で実測①②の前提が動いた**: window_pos.rs の WM_DPICHANGED は `dpi_suggested_position_decision`＋`ExternalAuthority`（:288-290/:388-389）で **OS 提案位置を書かなくなった**＝実測①の 8 回中 4 回（flags=21 の S1 書込 ②④⑥⑧）は現行コードで発生しない見込み。「859ms・SetWindowPos 8 回」は**全面再採取が必要**。実測②「applied=true 素通し」は是正済みの過去形として読むこと。
> - **+36px（work area 非追随）スコープは有効なまま**: `resnap_shell_targets` が新設された（frame.rs:1305・:1488 で毎フレーム駆動）が、これは**シェルサーフェス寸変化駆動**であり work area 変化駆動の再スナップは依然存在しない。S5 担当＝本 spec は追記(57)⑴で確定済み（`MonitorSnapshot`＝main.rs:703 の 1 箇所・dpi=144 で 24px 浮き実測・**要件はこの実測から書き起こせる＝S5 側の再観測は不要**）。
> - アンカードリフト: frame.rs 経路 B :801-859 → **:796-950 帯**（`run_dpi_phase` :976）・`emo2_frame_system` :1369-1399 → **:1466-1496**・window_pos.rs :285-385 → **:284-419**（決定関数化で中身も書き換わった）・`flush_window_pos_commands` は command.rs **:210**。exact 一致: command.rs `enqueue` :155-167・tick_bridge.rs :200・chain.rs ResizeBuffers :177-188・presenter.rs `take_pending_resize` :841・world/mod.rs tick_order :587。
> - **縮退・分割規定（過積載対策）**: 再観測で「859ms・多段適用」が消えていれば本 spec は **「+36px work-area 追随＋檻」へ縮退し `balloon-offset-dpi` と統合**（着手ゲート同一・follow.rs 共有＝roadmap 追記(58) の既定路線）。残存した場合は (a) フレーム単位観測基盤＋機序確定／(b) 原子性是正／(c) work-area resnap の 3 分割を要件フェーズで検討すること（brief 194 行・3 関心は単一 spec の上限）。
> - 干渉: presenter.rs `apply_show` 域で `recompose-budget`・`test-cage-determinism`④と**同ハンク級**＝budget 着地後に 859ms を再測すると合成コスト（1 合成 ≒143ms）の帰着切り分けが最良。

> **📌 2026-08-13 追記(63)（`areka-P0-scope-chain-gap` からの申し送り・スコープ追加）**: **拡大率遷移で二体の隙間が再発する経路が残っている。** scg が要件 7 で「初期配置は実表示サーフェス寸が確定するまで暫定」とし、確定時に連鎖（`scope_n.L = scope_{n-1}.L − 自幅`）を**一度だけ**解き直す機構を入れた（`crates/areka/src/placement/chain_finalize.rs`＝判定の純関数＋標識 `ChainFinalized`／`crates/areka/src/emo2_boot/frame/drain_resnap.rs` の `finalize_chain_once_with`＝結線／`frame.rs` の resnap 直後で駆動）。
> - **穴**: `ChainFinalized` は**恒久フラグ**で解除されない。確定後にモニタ間移動や OS 表示スケール変更で k が変わると、各窓は `resize_window_to` の下端中央固定で再アンカーされ**幅だけが k 倍に変わる**が、連鎖は二度と解き直されない。結果、scg が消したのと**同じ機序の隙間**（幅変化の半分ぶん左端がずれる）が DPI 遷移後に戻る。実測の下敷き: 200% で scope0 の面が 868→764 に変わったとき左端が `(868−764)÷2 = 52` 右へ寄り、連鎖非再解決ゆえ 52px の隙間が残っていた（これが要件 7 の起票理由そのもの）。
> - **なぜ scg で直さなかったか**: 要件 7.4 が「確定後のサーフェス切替では再解決しない」を**明示的に要求**している（会話中の表情差替で相方が横滑りしないため）。一度きりにしたことで追従（follow）の領分へ踏み込まずに済み、wpl・atom との干渉も回避できた。**DPI 遷移は要件 7.4 が想定した「サーフェス切替」ではなく、scg の要件にも設計にも記述が無い**。本 spec の宣言領域（「拡大率切替でキャラが跳ねる」）と真正面から重なるため申し送る。
> - **本 spec で決めること**: ⑴ DPI 遷移時に連鎖を解き直すか（解き直すなら `ChainFinalized` の解除条件＝どの遷移で暫定へ戻すか）⑵ 解き直さないなら、隙間が残ることを正典として要件へ明記するか。いずれにせよ **`ChainFinalized` の寿命は本 spec の設計判断**とする。⑶ 遷移中の中間フレームで解き直すと「跳ね」を増やしうるため、本 spec の原子性設計と同時に決めること（scg の一度きり確定は、まさに中間状態で解かないための形である）。
> - **構造的依存（併せて確認されたい）**: `finalize_chain_once_with` の駆動条件は「全スコープで `WindowPos.size` が実表示寸と一致する（＝resnap が landing 済み）」である（`drain_resnap.rs` の見送りガード）。これは完了済み `areka-P0-surface-resize-resnap` の内部挙動（再アンカー規則・フレーム内順序・`WindowPos` 更新タイミング）への**観測依存**であり、そこが変わると確定が静かに永久に見送られる（隙間が戻る）か早すぎるフレームで走る。完了済み spec ゆえ追跡先が無く、frame 相の順序を扱う本 spec が実質の見張り役になる。**ただし「静かに」ではなくなった**——scg のタスク 6.5 で、確定が有界の待ち（600 フレーム＝60Hz で約 10 秒・`CHAIN_FINALIZE_STALL_FRAMES`）を超えても起きない場合に、どのスコープがどの条件で見送られたかを添えた `warn!` が**一度だけ**出る。停滞の切り分けはログから可能になっている。
> - **6.5 が本 spec へ及ぼす点**: `ChainFinalized` を解除して確定をやり直す設計を採るなら、見送り計数 `ChainFinalizeStall`（`reported` は一発フラグ）も同時に初期化しないと、2 度目の待ちでは診断が出ない。`ChainFinalized` の寿命を決める際に併せて裁定されたい。
> - scg 側の証跡: `.kiro/specs/completed/areka-P0-scope-chain-gap/real-run-signoff-2026-08-13.log` §5.5（拡大率 200% で定常時 gap 0 を実測）・同 `tasks.md` の要件 7 節。

## Problem

**誰の問題か**: エンドユーザー（ゴーストを常駐させる利用者）。

**症状（開発者の実機目視・2026-08-01）**:

> スケールに追従してサイズは変わる。やはりスケール切り替え時に一瞬ガクッとしますね。
> **サイズについては即時に反映される。そのとき、一瞬表示位置（Y）が拡大時は浮くし、縮小時は多分下にめり込む。**
> 実際にはログを埋め込まないと判断できないのでは？ **目視では無理です。**

**痛み**:
1. デスクトップマスコットは**見た目が製品そのもの**である。拡大率変更のたびにキャラが跳ねるのは体感品質を直接損なう。
2. **目視では機序が判定できない**（開発者の明言）。フレーム単位の観測が無いと、直したつもりで直っていないことを検出できない。

**開発者の裁定（2026-08-01）**: 当初は「頻繁な操作ではないのでこのままでも良い」との所見だったが、その後**「スコープを広めにとらないとダメでは？ とにかく、作った spec で必ず解決すること」**へ改められた。**本 spec は「観測して終わり」では完了しない。**

## Current State

### 実測①: 1 回の遷移に **859ms** かかり、`SetWindowPos` が **8 回**に分かれて段階適用される

生ログ `%LOCALAPPDATA%\areka-diag\20260801-s2-crash\out.log`（commit `f8bcfd0`・release・拡大率 125%→200% の 1 回目）:

```
23:25:34.281 [UPD] Updating Monitor entity          old_work_area=…2100 → new_work_area=…2064  old_dpi=120 new_dpi=192
23:25:34.282 [RED] Redriving window DPI  entity=3v0 （以下 5v0/4v0/6v0 と 3ms 以内に連続）
23:25:34.362 [MV ] route=KeepPositionResize entity=3v0 balloon x=2797 y=1241 w=800 h=448
23:25:34.407 [MV ] route=KeepPositionResize entity=5v0 balloon x=3174 y=1600 w=800 h=448
23:25:34.474 [MV ] route=DpiReproject       entity=4v0 char    x=3186 y=1006 w=764 h=1094
23:25:34.526 [MV ] route=DpiReproject       entity=6v0 char    x=2628 y=1300 w=672 h=800
23:25:34.681 [GSP] SetWindowPos hwnd=0x5202E8  x=2797 y=1241 cx=800  cy=448  flags=20   ← ①バルーン0
23:25:34.753 [GSP] SetWindowPos hwnd=0x5202E8  x=2797 y=1241 cx=0    cy=0    flags=21   ← ②S1 の位置のみ書込
23:25:34.810 [GSP] SetWindowPos hwnd=0x26E0E6A x=3174 y=1600 cx=800  cy=448  flags=20   ← ③バルーン1
23:25:34.877 [GSP] SetWindowPos hwnd=0x26E0E6A x=3174 y=1600 cx=0    cy=0    flags=21   ← ④S1
23:25:34.937 [GSP] SetWindowPos hwnd=0x6F0A92  x=3186 y=1006 cx=764  cy=1094 flags=20   ← ⑤キャラ0（★656ms 遅れ）
23:25:35.027 [GSP] SetWindowPos hwnd=0x6F0A92  x=3186 y=1006 cx=0    cy=0    flags=21   ← ⑥S1
23:25:35.076 [GSP] SetWindowPos hwnd=0x2A0D8A  x=2628 y=1300 cx=672  cy=800  flags=20   ← ⑦キャラ1
23:25:35.113 [GSP] SetWindowPos hwnd=0x2A0D8A  x=2628 y=1300 cx=0    cy=0    flags=21   ← ⑧S1
23:25:35.140 [GSP] SetWindowPos hwnd=0x26E0E6A x=3174 y=1600 cx=0    cy=0    flags=21
```

**モニタ表更新から最後の書込まで 859ms。** 4 窓が 60〜90ms 間隔で**1 枚ずつ**動く。とくに**キャラ 0 の実書込は 656ms 遅れ**であり、それまで旧寸・旧位置のまま残っている。

**これは 1 vblank の問題ではない。** 踏査時の第一仮説（「サーフェスは即リサイズ・`SetWindowPos` は tick 末尾で flush＝1 vblank ズレ」）は**規模が 2 桁違う**。仮説として棄却はしないが、859ms の主因は別にある。

### 実測②: S1（OS 提案位置の書込）は本セッションでは**値として無害**だった

```
[WM_DPICHANGED] suggested position write decision entity=4v0 applied=true suggested_left=3186 suggested_top=1006
                                                  ↑ areka が .474 に書いた値と完全一致
```

24 件すべて `applied=true` だが、**OS の提案位置は areka が直前に書いた値と一致している**（areka が先に書いたので、OS は更新後の窓矩形から提案を導いた）。したがって `flags=21`（`SWP_NOSIZE`）の書込は同じ座標を書き戻すだけで、**位置の飛びを生んでいない**。

> **重要な帰結**: `areka-P0-dpi-window-vanish` のタスク 5.1（S1 是正）が着地しても、**本症状が消えるとは限らない**。5.1 は「OS 提案位置を最終位置として残さない」を保証するが、本セッションではそもそも提案位置＝areka の確定値だった。**5.1 着地後の再観測が本 spec の最初の仕事**である。

### 実測③: 接地点が work area の変化に追随しない（**+36px**）

`diagnosis-report.md` §2.5.3 に登記済み。キャラ窓 13 件の書込すべてで `ground_y = 2100` に固定:

| DPI | work area 下端 | 実際の接地点 | |
| --- | --- | --- | --- |
| 120 | 2100 | 2100 | 一致 |
| **192** | **2064** | 2100 | **+36px はみ出す**（下端がタスクバー領域へ潜る） |

発生は 6 件／13 件、すべて低→高方向、すべて `route=DpiReproject`。`+36` は 2 つの拡大率でのタスクバー高の差そのもの。**`Resnap` の書込が 1 件も無い**＝work area 変化を契機に再スナップする経路が存在しない。

**Req 4.1（接地点を変化の前後で保つ）の違反ではない**——2100 の保存はまさにそれを満たしている。要件が要求していないのは「work area 自体が変わったときの再スナップ」のほう。**開発者裁定により本 spec が持つ**（下記 Scope）。

### 踏査で確定した構造（`file:line` つき・2026-08-01）

| # | 事実 | 出所 |
| --- | --- | --- |
| 1 | DPI 変化の書込経路は **2 本**。経路 A＝wndproc 即時（`WM_DPICHANGED` ハンドラが `guarded_set_window_pos` を**同期呼出**・位置のみ `SWP_NOSIZE`）／経路 B＝areka frame 相 | `crates/wintf/src/ecs/window_proc/window_pos.rs:285-385`／`crates/areka/src/emo2_boot/frame.rs:801-859` |
| 2 | 経路 B の `SetWindowPos` は**キューへ push するだけ**で、実発火は **13 スケジュール全完了後**（World 借用解放後）の `flush_window_pos_commands()` | `crates/wintf/src/ecs/window/command.rs:155-167`／`crates/wintf/src/runtime/tick_bridge.rs:199-200` |
| 3 | 描画側（`presenter.apply(ShowSurface)`）と窓リサイズ判定（経路 B）は**同一関数・同一 World 借用で同期直列**（`FrameFinalize` 内） | `crates/areka/src/emo2_boot/frame.rs:1369-1399`（`emo2_frame_system`） |
| 4 | swap chain の `ResizeBuffers` は `presenter.apply_show` 内で**即時実行**＝`FrameFinalize` 中盤 | `crates/areka-emo-present/src/chain.rs:172-194` |
| 5 | ゆえに**サーフェス寸は tick 中盤で、窓寸は tick 末尾で**変わる（同一 tick 内だが別ポイント） | 上記 2・4 の合成 |
| 6 | **窓寸とサーフェス寸を同一コミットへ揃えるバリア・2 相コミットは存在しない**（`pending_resize`／`reconcile_reported_sizes` は取りこぼし防止であって同時性保証ではない） | `presenter.rs:841`／`frame.rs:872-877` の doc |
| 7 | **順序や同一フレーム性を固定するテストも存在しない**（`tick_order_tests` はスケジュール名の順序のみ） | `crates/wintf/src/ecs/world/mod.rs:586-623` |
| 8 | `try_tick_on_vsync` は現行アーキテクチャで**常に false**＝wndproc 内のネスト tick は起きない | `crates/wintf/src/ecs/world/vsync.rs:17-22, 106-155` |

### 未解明（**仮説段階・断定しないこと**）

- **859ms の内訳が不明。** 窓 1 枚あたり 60〜90ms が何に消えているのか（合成 143ms/回という `recompose-budget` の実測と近い桁である点は示唆的だが、因果は未確認）
- **開発者が見た「Y の浮き／めり込み」がどの中間状態か未特定。** 上の 8 回の書込のどこかで一時的に不整合な矩形が可視化されているはずだが、**フレーム単位の可視コミット記録が無い**ため確定できない
- **経路 A と経路 B の相互作用**が別の環境（OS 提案位置が areka の値と一致しない場合＝セッション①で観測された最大 861px 乖離）でどう見えるかは未観測

## Desired Outcome

- 拡大率を切り替えたとき、**キャラ・バルーンが跳ねずに新しい寸法・位置へ移る**
- 遷移中の中間状態が**観測可能**である（目視に頼らずログとテストで機序を判定できる）
- 遷移後の接地点が**新しい work area の下端**に一致する（+36px のはみ出しが解消する）
- 遷移の所要時間が**決定論的な量で檻に入る**（実時間ではなく `SetWindowPos` 回数・フレーム数など）

## Approach

**採用: 観測を先に建ててから直す（`dpi-window-vanish` と同じ流儀）。**

開発者が「目視では無理・ログを埋め込まないと判断できない」と明言しており、実際に踏査でも**フレーム単位の可視コミット記録が無い**ことが確定している。機序未確定のまま是正を当てると、`dpi-window-vanish` が 2026-07-18 に踏んだ偽陰性を再生産する。

1. **第 1 段（再観測・Phase C 着地後）**: `dpi-window-vanish` の 5.1／5.2 着地後に同じ手順で採取し直し、症状が残るか・どう変わるかを確定する。**S1 是正で消える可能性と、消えない可能性の両方がある**（上記 実測②）。
2. **第 2 段（フレーム単位の観測増設）**: 1 回の遷移に含まれる全 `SetWindowPos` と全サーフェス更新を、**フレーム番号つき**で 1 本の時系列に並べられる観測を建てる。判定語は `dpi-window-vanish` の流儀（専用 target・既定 OFF・grep 判定語を純関数化して檻で固定）に倣う。
3. **第 3 段（機序の確定）**: 859ms の内訳と、Y が不整合になる中間状態を実測で名指しする。
4. **第 4 段（是正）**: 確定した機序のみを直す。候補は「遷移を 1 コミットへ束ねる（2 相コミット／バリア）」「窓ごとの逐次適用をやめて一括適用する」「work area 変化を契機とする再スナップ経路の新設（+36px）」。**選択は第 3 段の結果を見てから。**

**なぜこの順か**: 第 4 段の候補はいずれも既存の tick 構造に手を入れる大きな変更であり、機序が確定しないまま選ぶと外す。第 2 段の観測は第 4 段の回帰檻としてそのまま使える。

**棄却した案**:
- **いきなり 2 相コミットを実装する**: 859ms の主因が合成コストなら、コミットを束ねても跳ねは消えない（順番が揃うだけで所要時間は変わらない）。機序未確定での大改造は [[analyze-ideal-form-not-minimal]] の「解決/未解決を明示」に反する。
- **`dpi-window-vanish` へ相乗り**: 同 spec は 3 フェーズ承認済み・実装中（Phase C）で、承認済み成果物を実装中に膨らませることになる。加えてドメインが違う（あちらは**位置権威の正しさ**、本 spec は**遷移の見え方と所要時間**）。台帳 §5.2 に「境界が違う」と登記済み。
- **`recompose-budget` へ相乗り**: あちらは**定常状態のアロケーション予算**、本 spec は**遷移時の原子性**。関心が違う。ただし `presenter.rs` を両者が触る見込みゆえ干渉台帳へ登記する（下記）。
- **何もしない**: 開発者裁定「作った spec で必ず解決すること」により不可。

## Scope

- **In**:
  - DPI／拡大率の遷移中に発生する**窓ジオメトリと描画内容の不整合**（開発者観測の「Y の浮き／めり込み」）
  - 遷移の**所要時間と逐次性**（859ms・8 回の段階適用）
  - **遷移後の接地点が新しい work area 下端へ追随しないこと（+36px）**——開発者裁定「スコープを広めに・必ず解決すること」により本 spec が持つ
  - 上記を判定するための**フレーム単位の観測**と、決定論的な回帰檻
- **Out**:
  - **位置権威の正しさそのもの**（S1＝OS 提案位置の素通し／S2＝再射影のゲート／S3＝可視性の不変条件）＝`areka-P0-dpi-window-vanish` の所有。本 spec は**その是正が着地した後**の見え方を扱う
  - **定常状態の合成コスト・アロケーション予算**＝`areka-P0-recompose-budget` の所有。ただし 859ms の内訳が合成コストに帰着した場合は**そちらへ差し戻す**（本 spec で最適化しない）
  - モニタ着脱・解像度変更・配置変更への全面追随（`dpi-window-vanish` の Boundary Context が「対象外」と明記済み・本 spec も踏襲）
  - SERIKO のアニメ発火頻度・正典解釈（`completed/areka-P0-seriko-loop` の領分）
  - GPU ドライバ差・実機 GPU 性能

## Boundary Candidates

- **遷移のコミット単位**（`SetWindowPos` キューの flush 粒度・`tick_bridge.rs` の flush 点）＝本 spec の中核候補
- **サーフェス寸と窓寸の同時性**（`chain.rs` の `ResizeBuffers` と経路 B の flush の関係）
- **work area 変化を契機とする再スナップ**（+36px・`Resnap` route の発火条件）＝`dpi-window-vanish` 6.1 と**近接するので着手前に実測再突合が要る**
- **フレーム単位の観測基盤**＝本 spec が新設し、以後の表示系 spec が再利用する資産

## Out of Boundary

- 位置の**値**が正しいかどうか（`dpi-window-vanish` が所有）
- 合成アルゴリズム本体（`build_plan`／`blit::execute`）
- ドラッグ中の追従（セッション①で比 1.000・暴走なしと確定済み）

## Upstream / Downstream

- **Upstream**:
  - **`areka-P0-dpi-window-vanish`（W5・実装中）** — S1／S2／S3 是正が着地しないと本 spec の再観測が意味を持たない。**5.1／5.2 着地が本 spec 着手の前提**
  - `completed/areka-P0-emo-dpi-scaling`（W4）— DPI 追従フェーズの提供元（**completed ＝消化不能**）
  - `completed/areka-P0-emo-present` — `presenter.rs`／`chain.rs` の所有（**completed ＝消化不能**）
  - `completed/areka-P0-app-shell` — tick 構造・スケジュール順の所有（**completed ＝消化不能**）
- **Downstream**:
  - `areka-P0-emo2-conformance-e2e`（W7）— 適合一周走行での見え方
  - 以後の表示系 spec がフレーム単位観測を再利用する

## Existing Spec Touchpoints

- **Extends**: なし。**所有者候補（`emo-dpi-scaling`／`emo-present`／`app-shell`）はいずれも `completed/` にあり消化不能**（[[deferral-requires-verified-owner]]「completed は消化不能」）。ゆえに新規 spec が要る。
- **Adjacent（干渉台帳へ登記すること）**:
  - **`areka-P0-dpi-window-vanish`（W5）— 最重要**。本 spec の +36px は同 spec の **6.1（遷移ガード配線）と同じ `follow.rs` を触る可能性がある**。6.1 のガードは「提案矩形が work area と**交差するか**」を見るため、はみ出し（交差はしている）を検出しない見込みだが、**6.1 着地後に実測再突合すること**（`dpi-window-vanish` の tasks.md 4.7 に「6.1 着手時に要確認」と登記済み）
  - **`areka-P0-recompose-budget`（W6.5）— 同一ファイル `crates/areka-emo-present/src/presenter.rs`・ハンク未確定**。あちらは `:369-400`（compose/cache/resample）、本 spec は `apply_show`／`chain.upload` 近傍の見込み。**先着後 rebase**。859ms の内訳が合成コストに帰着したら**そちらへ差し戻す**関係でもある
  - `areka-P0-scale-exact-rational`（W6.5）— 同じく `presenter.rs`（`:659-666`）。異ハンクの見込み
  - `areka-P0-ghost-window-zorder`（W6）— 窓の `SetWindowPos` を触る点で近接。z オーダーは flags の別ビットゆえ素の見込みだが着手時に確認
- **合流しない相手（判断の記録）**:
  - `areka-P0-dpi-window-vanish` とは**合流しない**。①ドメインが別（位置権威の正しさ vs 遷移の見え方）②3 フェーズ承認済みで Phase C 実装中＝承認済み成果物を実装中に膨らませることになる ③本 spec の第 1 段は同 spec の**着地を前提**とするので順序が逆
  - `areka-P0-recompose-budget` とも**合流しない**。関心が別（定常コスト vs 遷移の原子性）で、要件も檻も別物になる

## Constraints

- **機序未確定のまま原因を書かない**（`dpi-window-vanish` Req 1.5 の流儀を踏襲）。観測されていないことを「起きていない」の根拠に使わない
- 檻は**実時間ではなく決定論的な量**で表現する（`SetWindowPos` 回数・フレーム数・順序）。実時間はマシン差で非決定（[[deterministic-test-coverage-mandate]]）
- 実機でしか確定できない残余は**有界自動終了＋ログ grep** でサインオフする（[[areka-real-machine-signoff-bounded-auto-exit]]）
- 常時テストは x86 を避け純 x64 決定論で（[[prefer-x64-fake-boundary-tests-not-x86]]）
- `cargo test -p areka` に `--bins` を付けない（examples が `#[path]` include）
- `cargo clippy -p wintf` は `com/d2d/command_sink.rs` の既存不良で失敗＝DoD に使わない
- wintf 側で `Schedule` を回すログ檻は **`ExecutorKind::SingleThreaded` を明示**すること（`dpi-window-vanish` 4.6 の教訓＝多スレッド実行器では `capture_under_filter` が 1 行も捕捉できず否定 assert が空虚に緑になる）

## Open Questions（要件フェーズで裁定・本節が正本）

1. **859ms の内訳は何か。** 合成コストなら `recompose-budget` へ差し戻す。tick 構造なら本 spec が持つ。**第 1 段の再観測で最初に切り分ける。**
2. **「同時性」をどこまで保証するか。** 全窓を 1 フレームで揃えるのか、キャラとその随伴バルーンだけを揃えれば十分か。前者は tick 構造への大きな介入になる。
3. **+36px の是正主体。** `Resnap` の発火条件へ work area 変化を足すのか、`DpiReproject` 側で新 work area を引き直すのか。**`dpi-window-vanish` 6.1 の着地内容を見てから決める。**
4. **フレーム単位観測の恒久性。** `dpi-window-vanish` の diag と同じく「恒久・既定 OFF」で建てるか、遷移時のみの限定観測にするか。

## Wave 提案（開発者裁定要）

**開発者選択（2026-08-01）: 「5.1／5.2 着地後に再観測してから決める」。**

したがって本 spec は**ウェーブ未確定**として起票する。roadmap のウェーブ表には**保留行**として載せ、`dpi-window-vanish` の Phase C（5.1／5.2）着地時に再観測してから配置を確定する。

- **再観測で症状が消えていた場合**: 本 spec は「+36px の是正」＋「再発を捕まえる檻」へ縮退し得る。ただし**縮退の判断は実測に基づいて記録すること**（[[defer-canon-with-full-vocabulary-and-tracking-spec]]）
- **残っていた場合**: 第 2 段（観測増設）から通しで実施。W6.5（`recompose-budget` と同居して `presenter.rs` をまとめて触る）が有力
