# Brief: areka-P0-ghost-window-zorder

> **起票 2026-07-31**（`/kiro-discovery` 再入・`areka-P0-dpi-window-vanish` の task 4.5 実機セッション中に開発者が発見）。
> 本 brief は**実測証拠と正典語彙を全て内包**する。別セッションはこの brief 単体で再開できる（会話ログは不要）。

> **📌 2026-08-01 追記(58)陳腐化補正＋配置確定（棚卸⑤・本ブロックが以下の本文より優先）**:
> - **ウェーブ配置確定＝W6 編入**（col ∥ vis ∥ bind ∥ 本 spec ∥ scg の 5 本並走・実測全ペア素）。vis は本 spec の「各バルーンが自分のシェルより手前」保証を前提にする（相互登記）＝本 spec が W6 より後ろへ落ちることは不可。
> - **「van 着地後に rebase」条項は消化済み**: van マージ済（PR#98）で spawn.rs は `ExternalAuthority` 付与（:353-354・バンドル :245/:273）＋despawn hook まで着地した現物＝本 spec は現物に owner／z-order を積むだけ。
> - **追い風**: `SetWindowPosCommand` は `hwnd_insert_after: Option<HWND>` を搬送可能（wintf `ecs/window/command.rs` :124/:141）＝案 B（`ZOrder` 明示維持）の配線コストが起票時想定より低い。案 A（Win32 owner）の WUC/クリックスルー/`NOREDIRECTIONBITMAP` 共存実機検証が最初のタスクである点は不変。
> - **混同予防**: `zorder_raw`（`placement/config.rs:72`）は descript `seriko.zorder`（SERIKO レイヤ順）の生文字列で**本 spec の窓 z-order とは別概念**。grep で引っかかっても編集対象ではない。
> - アンカーは全一致を再確認（`ZOrder` :25-38・ビルダー :119/:131/:156・keyboard.rs WM_ACTIVATE :119・window_factory.rs :152・`NoChange` 以外の本番指定ゼロ）。

## Problem

**誰の問題か**: エンドユーザー（ゴーストを常駐させる利用者）。

**症状（開発者の実機観測・2026-07-31）**: キャラ窓をドラッグして別ディスプレイへ移動すると、**バルーンが他アプリの窓の背後に埋もれて見えなくなる**。開発者の診断（そのまま採用）:

> 移動時にシェルの Z オーダーは前に移動するが、**バルーンの Z オーダーが調整されない**ため、別ウィンドウより後ろに埋もれてしまう。

観測上は「左ディスプレイへ動かすと消える・右に戻すと復旧」と見えたが、これは**左ディスプレイに手前へ来る窓が在っただけ**で、モニタ依存の現象ではない。

**痛み**: 会話が読めなくなる。しかもユーザーには「バグで消えた」としか見えず、**キャラを掴み直す以外に復帰手段が無い**。デスクトップマスコットの中核体験の毀損。

## Current State

### 実測（2026-07-31・release ビルド・実機 2 モニタ混在 DPI 192/144・fixture emo2）

**消失は位置の問題ではない**——4 系統すべてが「バルーンは正しい場所に、隠されずに在る」ことを示す:

| 検証 | 結果 |
|---|---|
| バルーン矩形が全 work area 非交差になったか | **0 件 / 1185 レコード**（scope 0・幾何的には常に可視領域内） |
| `apply(Hide)` 指令 | **0 件** |
| `EmptyComposition` 縮退 | **0 件** |
| `ShowWindow`／`SW_HIDE`／`SWP_HIDEWINDOW` | **0 件** |

→ 要件 2.2（`dpi-window-vanish`）の判別語でいう「**可視領域内に存在した（見落とし）**」。真の不可視ではない。

**Z オーダーは誰も管理していない**:

| 検証 | 結果 |
|---|---|
| `SetWindowPos` の flags | **4242 件すべて `SWP_NOZORDER`**（`flags=21`＝`SWP_NOSIZE\|SWP_NOZORDER\|SWP_NOACTIVATE`／`flags=20`＝寸変更時） |
| `crates/areka/src/` の z-order 設定 | **一箇所も無い** |
| ゴースト窓の owner／parent | **無し**——`crates/wintf/src/runtime/window_factory.rs:152` が「`parent: None`（**現行 areka／全 example**）」と明記 |

→ **バルーンとキャラは互いに何の関係も持たない独立トップレベル窓**。キャラをクリックすると Windows がそれを活性化して前面へ上げるが、バルーンは Z オーダー上に置き去りにされる。

**構造上、対称の不具合が在るはず**: バルーンをドラッグすればキャラが埋もれる（未実測・要件フェーズで確認すること）。

### 語彙は完備、配線ゼロ（**本 spec 特有の好条件**）

`crates/wintf/src/ecs/window/window_pos/mod.rs:25-38` に `ZOrder` 列挙が既に在る:

```rust
pub enum ZOrder {
    #[default] NoChange,     // Z-order を変更しない
    TopMost,                 // 常に最前面
    NoTopMost,               // 通常のウィンドウ
    Top,                     // 最前面に配置
    Bottom,                  // 最背面に配置
    InsertAfter(HWND),       // 指定ウィンドウの後ろに配置
}
```

ビルダー（`with_zorder`／`zorder_topmost`／`zorder_top`／`zorder_insert_after` ほか `:119-156`）と `get_hwnd_insert_after`／`build_flags_for_system` の変換も実装済みで、**単体テストも完備**（`window_pos/tests.rs:84-214`）。

**しかし `NoChange` 以外を指定する本番コードは 1 箇所も無い**（grep のヒットは全て `tests.rs`）。**`InsertAfter(HWND)` は開発者が求める「シェルの一つ手前」をそのまま表現できる語彙**であり、実装ではなく**配線が欠けている**。

> 本セッションで見つかった「語彙は完備・配線ゼロ」の**3 例目**（他: `Composer::compose_into`＝`areka-P0-recompose-budget`／`PlacementRoute::SpawnInitial`・`Restore`＝`dpi-window-vanish` 5.1 で配線予定）。[[defer-canon-with-full-vocabulary-and-tracking-spec]] の運用が語彙側では効いている一方、**配線の追跡が弱い**ことを示す。

### 活性化のフックは既に在る

`crates/wintf/src/ecs/window_proc/keyboard.rs:119` が `WM_ACTIVATE` を処理している（現状の用途は**ドラッグキャンセルのみ**・`:114-161`）。`window_proc/mod.rs:70` で結線済み。**活性化を観測する席は空いていない＝ここへ相乗りできる**。

`SetForegroundWindow`／`BringWindowToTop`／`SetActiveWindow` は wintf に無く、前面化は Windows の既定動作に委ねている。

### 正典（ukadoc）の規定と、areka の未実装分

| 語彙 | 正典の意味 | areka |
|---|---|---|
| `\v` ／ `\![set,windowstate,stayontop]` | 手前に表示。以降**常に最前面**。ゴースト終了まで有効 | **未実装** |
| `\![set,windowstate,!stayontop]` | 手前に表示しない。他窓と同様に重なりを処理 | **未実装** |
| `\![set,windowstate,minimize]` | 最小化する | **未実装** |
| `OnWindowStateMinimize` ／ `OnWindowStateRestore` | 最小化／復帰時に発生（Ref0＝理由: system／script／sakuraapi／user） | **未実装** |
| `OnFullScreenAppMinimize` ／ `OnFullScreenAppRestore` | 全画面アプリ起動／終了に伴う強制最小化・復帰 | **未実装** |

**正典の重要な明文**（`\v` の項）:
> ただし**スコープごとの重なり（本体側と相方側どちらが上にくるか）はユーザの操作次第**。

→ **sakura（scope 0）と kero（scope 1）の上下関係を baseware が強制してはならない。** 本 spec が保証するのは「**各バルーンが自分のシェルより手前**」だけ。

**emo2 は `windowstate`／`\v` を使わない**（fixture 全 grep で 0 件）＝M1 の実物スコープ外。ゆえに上表の未実装分は**語彙・シームのみ予約して先送り**が妥当（[[defer-canon-with-full-vocabulary-and-tracking-spec]] の 4 点セット）。

## Desired Outcome

**開発者の指定（そのまま採用）**:
> シェルがアクティブになったら、該当のバルーンはシェルの**一つ手前**くらいに Z オーダーを調整されるべき。

具体的に:
- キャラ窓が活性化・前面化したとき、**同一 scope のバルーンがそのキャラ窓の直前**へ来る
- ドラッグ・クリック・DPI 変化・再配置のいずれの経路でも、この関係が**破れない**
- **scope 間の上下は強制しない**（正典の明文）
- 他アプリを活性化したときは、ゴースト一式が**一緒に**背面へ回る（バルーンだけ前に残らない）

## Approach

### 案 A: Win32 の owner 関係を張る（**推奨**）

バルーン窓を対応するキャラ窓の **owner window** として生成する（`CreateWindowExW` の `hWndParent` に非子窓として渡す）。Windows が「owned window は常に owner より手前」を**OS 保証**する。

- **Pros**: 経路の網羅漏れが原理的に起きない（活性化・ドラッグ・DPI 変化・将来の新経路すべてに自動適用）。維持コードがゼロ。他アプリ活性化時にゴースト一式が一緒に沈むのも OS 任せで正しくなる。[[canonical-not-minimal-lifecycle]] の流儀
- **Cons**: owner 関係は Z オーダー以外の副作用を伴う（タスクバー非表示・owner の最小化/破棄への追随・活性化の伝播）。areka の窓は `WS_EX_NOREDIRECTIONBITMAP`＋WUC 合成＋クリックスルー（`WS_EX_TRANSPARENT` トグル）という特殊構成ゆえ、**owner 化がそれらを壊さないかの実機検証が必須**
- **要注意**: `window_factory.rs:150-163` の現行 `parent` は `SetParent` を呼ぶ＝**子窓**化であり owner ではない。子窓は owner と全く別物（親矩形にクリップされる）。**owner は別概念として新設**する必要がある
- **規模**: 小〜中（生成経路の 1 引数＋実機検証）

### 案 B: 活性化フックで明示的に Z を維持

`WM_ACTIVATE`（`keyboard.rs:119` の既設ハンドラ）でキャラ窓の活性化を捕らえ、同 scope のバルーンを `ZOrder::InsertAfter` で直前へ再配置する。

- **Pros**: 既存の `ZOrder` 語彙とフックをそのまま使える＝新規概念ゼロ。ヘッドレスで檻に入れやすい（`WM_ACTIVATE` ディスパッチ檻の前例が `dpi-window-vanish` 4.3 に在る）。副作用が読み切れる
- **Cons**: **Z オーダーが動く経路を人手で網羅する必要**があり、漏れが再発の穴になる（活性化以外にも、表示開始・DPI 変化・他アプリからの復帰などがあり得る）。命令的維持は [[areka-collision-overlap-painter-algorithm]] 型の「後から入った経路が規約を知らない」問題を招く
- **規模**: 小

### 案 C: A を基礎に B を補助として併用

owner で構造保証しつつ、活性化時に明示調整も行う。

- **Pros**: 保険
- **Cons**: A が効いていれば B は恒真＝**空虚な保険**になり、A が壊れたときに B が症状を隠す。[[real-machine-signoff-catches-what-cages-hide]] の逆をやることになる
- **規模**: 中

**推奨は A**。理由: 本 spec の症状は「経路が網羅されていない」ことそのものであり、**命令的維持（案 B）は同じ穴を別の形で残す**。構造保証で解けるならそちらが正。

ただし **A の実現可能性検証を要件／設計フェーズの最初のタスクとする**——owner 化が WUC 合成・クリックスルー・`WS_EX_NOREDIRECTIONBITMAP` と共存できるかは**実機でしか判らない**（[[areka-transparency-requires-layered-window]]／[[areka-clickthrough-hittest-config]]）。壊れるなら案 B へ落とす。この分岐は brief に記録済みゆえ、どちらへ倒れても判断の履歴が残る。

## Scope

- **In**:
  - 同一 scope の「バルーンはシェルの直前」関係の確立と維持（案 A または B）
  - `ZOrder` 語彙の**本番配線**（現状 `NoChange` 以外の呼出ゼロ）
  - 他アプリ活性化時にゴースト一式が一緒に背面へ回ること
  - バルーン側をドラッグした場合の対称ケース（キャラが埋もれない）
  - 決定論テスト（`WM_ACTIVATE` ディスパッチ檻・owner 関係の構造検証）＋**実機サインオフ**（重なりは実機でしか確定しない残余）
- **Out**:
  - **scope 間（sakura ⇄ kero）の上下関係の強制**——正典が「ユーザの操作次第」と明文で規定
  - `\v`／`\![set,windowstate,*]`／`OnWindowState*`／`OnFullScreenApp*` の**実装**（emo2 消費者ゼロ＝M1 実物スコープ外。**語彙は本 brief に完全収録済み**・縮退シームと追跡は下記）
  - バルーンの show/hide ライフサイクル（`areka-P0-balloon-visibility` の所有）
  - 窓の**位置**の正しさ（`areka-P0-dpi-window-vanish` の所有）
  - 最小化・タスクバー表示・Alt+Tab の扱い（`windowstate` 先送りに含む）

## Boundary Candidates

- **窓の生成時関係**（owner の有無）＝`wintf` の `window_factory` 層
- **活性化イベントの消費**＝`wintf` の `window_proc/keyboard.rs`（既設 `WM_ACTIVATE`）
- **scope ↔ 窓ペアの対応**＝`areka` の `placement/spawn.rs`（`GhostWindows` レジストリが既に scope→(char, balloon) を持つ）
- **正典 `windowstate` の受理口**＝将来 `sakura` パーサ／`kanade` 側（本 spec では**シームのみ**）

## Out of Boundary

- SERIKO・talk・SHIORI 側の駆動
- 描画・合成（`emo-*` 一式）
- 窓の位置・寸法の決定（`dpi-window-vanish`／`kero-balloon`）

## Upstream / Downstream

- **Upstream**:
  - `completed/areka-P0-window-placement`・`completed/wintf-P0-click-through`・`completed/areka-P0-emo2-boot`（窓生成と特殊スタイルの所有元。いずれも **completed ＝消化不能**）
  - `areka-P0-dpi-window-vanish`（W5・実装中）— `placement/spawn.rs` の spawn バンドルを **task 5.1 で触る予定**（`DpiSuggestedRectPolicy::ExternalAuthority` 付与）。本 spec も同ファイルへ owner／z-order の配線を足すなら**同一ハンク近接**——干渉台帳へ登記済み
- **Downstream**:
  - `areka-P0-balloon-visibility`（W6）— show/hide の再表示時に「手前に出る」ことが本 spec の保証に乗る
  - `areka-P0-emo2-conformance-e2e`（W7）— 適合一周でバルーンが見えることが前提
  - 将来の `windowstate` 実装（本 brief が語彙の正本）

## Existing Spec Touchpoints

- **Extends**: なし。所有者候補（`window-placement`・`click-through`・`emo2-boot`）は**全て `completed/` で消化不能**（[[deferral-requires-verified-owner]]）。
- **Adjacent（干渉台帳へ登記すること）**:
  - **`areka-P0-dpi-window-vanish`（W5・実装中）— `crates/areka/src/placement/spawn.rs` で近接**。van は task 5.1 で char/balloon 両 spawn バンドルへ component を追加予定。本 spec が同バンドルへ owner／z-order を足すなら**同一ハンク**になり得る＝**van 着地後に本 spec が rebase**
  - **`areka-P0-balloon-visibility`（W6）— 概念的に隣接**（どちらも「バルーンが見える」）が責務は別（本 spec＝重なり／vis＝show/hide ライフサイクル）。vis の brief 現行 Scope に **Z オーダーの言及はゼロ**＝穴が空いており、本 spec がそれを埋める。**vis の要件フェーズで本 spec の保証を前提にしてよい**
  - `areka-P0-recompose-budget`（W6.5+）— ファイル素（present 層 vs window 層）
- **合流しない相手（判断の記録）**:
  - **`dpi-window-vanish` とは合流しない**。同 spec の要件 3.1 は「いずれのモニタ work area とも**交差しない**状態を防ぐ」＝**幾何限定**であり、Z オーダーによる遮蔽は別軸。承認済み要件（3 フェーズ承認済み・実装中）への追加は逸脱。**ただし要件 2.2 は「可視領域内の見落とし」の判別記録を要求しており、本症状の観測結果は task 4.5 の成果として `diagnosis-report.md` に記録される**（是正は本 spec）
  - **`balloon-visibility` へ畳まない**（当初検討したが不採用）。vis は show/hide の**状態機械**を所有し、本 spec は**窓の重なり関係**という別レイヤ。混ぜると vis の要件が 2 軸になり、どちらの失敗かが切り分けられなくなる

## Constraints

- **正典の明文を破らない**: scope 間の上下は強制しない
- **特殊窓構成と共存すること**: `WS_EX_NOREDIRECTIONBITMAP`＋WUC 合成＋クリックスルー（`WS_EX_TRANSPARENT` トグル）。owner 化がこれらを壊さないかは**実機検証必須**（[[areka-transparency-requires-layered-window]]・[[areka-clickthrough-hittest-config]]・[[areka-gpu-window-screenshot-readback]]＝GPU 合成窓はスクショ不可ゆえ重なりの自動判定は困難）
- **重なりは決定論檻に入れにくい残余**＝実機サインオフを有界 auto-exit ＋ログ grep で行う（[[areka-real-machine-signoff-bounded-auto-exit]]）。ヘッドレスで固定できるのは「`SetWindowPos` が期待する `hWndInsertAfter` で呼ばれた」までで、**実際に手前に見えたかは実機**
- `cargo test -p areka` に `--bins` を付けない／`cargo clippy -p wintf` は既存不良で失敗＝DoD に使わない

## 先送り正典の 4 点セット（[[defer-canon-with-full-vocabulary-and-tracking-spec]]）

1. **完全語彙**: 上表のとおり `\v`／`\![set,windowstate,stayontop|!stayontop|minimize]`／`OnWindowStateMinimize`／`OnWindowStateRestore`／`OnFullScreenAppMinimize`／`OnFullScreenAppRestore`（Ref0＝`system`／`script`／`sakuraapi`／`user`／`fullscreen`）
2. **縮退シーム**: 本 spec が確立する「バルーンはシェルの直前」を**既定状態**とし、`stayontop` は「ゴースト一式を最前面へ」の 1 ビットとして後付けできる形にする（`ZOrder::TopMost` が既に語彙として在る）
3. **追跡 spec**: 本 brief が語彙の正本。実装は M2 以降または `windowstate` 専用 spec
4. **roadmap 明記**: 追記(54) に記録

## Open Questions（要件フェーズで裁定・本節が正本）

1. **案 A（owner）か案 B（明示維持）か。** A の実機実現可能性検証が最初のタスク。WUC 合成・クリックスルー・`NOREDIRECTIONBITMAP` と共存できるか
2. **バルーンをドラッグしたとき**、キャラも一緒に前へ出るべきか（ペアとして浮上）、バルーンだけでよいか。未実測の対称ケース
3. **他アプリ活性化時**の挙動——ゴースト一式が一緒に沈むのが正しいか、それとも常に手前が正しいか（後者は `stayontop` 相当＝先送り語彙に触れる）
4. **ウェーブ配置**（下記）

## Wave 提案（開発者裁定要）

**推奨: W6**（`balloon-visibility` ∥ `bindoption-exclusivity` へ 3 本目として同居）。理由:
- vis が「再表示時に手前に出る」を前提にできる＝**vis の要件フェーズより前か同時に居るのが自然**
- ファイル集合は vis（frame.rs＋emo2_boot 新 module）・bind（parsers／seriko／assets.rs）と**素**（本 spec＝wintf の window_factory／window_proc／window_pos ＋ areka placement/spawn.rs）
- **van（W5）着地後に rebase**（`placement/spawn.rs` の spawn バンドル近接）

**対抗案: W5 直後の単独割込**。理由: 症状が**日常的に見える挙動バグ**で、しかも `ZOrder` 語彙が既に完備＝**配線だけで解ける可能性が高い**（案 B なら小規模）。W6 の 2 本より先に片付く見込み。
