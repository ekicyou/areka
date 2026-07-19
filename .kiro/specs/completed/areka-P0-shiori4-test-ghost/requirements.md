# Requirements Document

## Introduction

areka の実 boot 経路（descript.txt 起点マウント → SHIORI DLL ロード → wire → talk → close）を貫くテストは、現状「決定論だが境界を踏まない（`ShioriWiring::Custom`＋`ScriptedShioriBackend`）」か「境界を踏むが非決定論かつ i686 前提（実ゴースト emo2＝32bit `pasta.dll`）」の二極しか持たない。中間＝**実 SHIORI 境界を踏む決定論テストゴースト**が不在である。

本 spec の**最優先目的は、areka 自身のテスト内でこの決定論テストゴーストを「使いやすく」立ち上げられること**である。テスト作者が少ない手数で、pasta 非依存・i686 非依存・別プロセス非依存・sleep 不使用のまま、実 boot 経路を決定論的に一周・照合できることを組織原理に据える（要件 1）。それを実現する支持要件群として、x64 SHIORI4 テスト DLL（決定論脳・要件 2）、正規 in-proc ロード経路 `ShioriWiring::InProc`＋`IShiori`→`ShioriBackend` アダプタ（要件 3）、マウント可能テストゴースト fixture（要件 4）、boot→talk→close 決定論 e2e（常設ゲート・要件 5）、既存資産との共存・置換規律（要件 6）、スコープ境界と M2 前方整合（要件 7）を定義する。

M1 の正典イベント最小集合は既存運行表（`OnInitialize` / `OnFirstBoot` / `OnBoot` / `basewareversion` / `OnSecondChange` / `OnClose`）に一致させる。`InProc` シームは M2 の native x64 SHIORI4 が乗る正規シームでもあるが、これは第一級だが**副次**の関心として扱い、テストエルゴノミクスの焦点を薄めない。

テスト脳の応答台本は**実 emo2 `pasta.dll` の出力から採取したゴールデンスナップショット**（正典イベントごとの実応答を凍結した静的 fixture）を replay する方式を採り、shell/balloon 資産は emo2 実物をそのまま流用する（非決定論は脳＝SHIORI に局在するため、そこだけを x64 決定論 DLL へ差し替える）。常設ゲートは cue sink 受領レベルで境界の決定論を保証し、実 shell/balloon を実描画まで貫くエミュレーションは流用資産上の opt-in 追加確認に留める（常設ゲート必須ではない）。

## Boundary Context

- **In scope**: x64 SHIORI4 テスト DLL（cdylib・決定論固定台本）／正規 in-proc ロード経路 `ShioriWiring::InProc`＋`IShiori`→`ShioriBackend` アダプタ／マウント可能テストゴースト fixture（descript＋最小 shell、必要なら最小 balloon）／boot→talk→close の決定論 e2e（常設・`cargo test --workspace`）／既存 emo2 依存 smoke の乗り換え可否の仕分け。
- **Out of scope**: 本番 main の結線変更（M1 本番ゴーストは emo2＝`Helper` 経路のまま）／descript 駆動の bitness・種別自動判別（M2 シーム予約）／`IShioriHost::Raise` 起点の自発イベントや deferred（`SHIORI_S_PENDING`）の網羅（ReferenceBrain 既存檻を超えない）／`areka-P0-emo2-conformance-e2e` の spine 改稿／SAORI・里々・YAYA。
- **Adjacent expectations**: 既存 IShiori COM ABI（`shiori-abi`）は変更せず消費する／`ShioriBackend` トレイト（`areka-kanade`）は変更せず実装する／既存 `ShioriWiring::Custom`／`ScriptedShioriBackend` シームは併存維持する／env-gate 実 pasta 追験（`HOST32_PASTA_DLL`／`AREKA_EMO2_REAL_RUN`）は残置する／SHIORI/3.0 テキストの content 契約（既存 codec）を不透明搬送で再利用する／正典はゴースト仕様として ukadoc、内部 SHIORI4/IShiori 規約は `shiori-abi` rustdoc＋completed shiori 系 spec。

## Requirements

### Requirement 1: areka テスト内での決定論テストの使いやすさ（最優先）

**Objective:** As an areka test author, I want to stand up a boundary-crossing deterministic SHIORI4 test ghost with minimal ceremony, so that I can write reproducible regression tests over the real boot path without pasta, i686, separate-process, or sleep dependencies.

#### Acceptance Criteria

1. When a test author sets up a deterministic boundary-crossing SHIORI4 test, the テストハーネス shall boot it through the existing `GhostBootOptions`／`ShioriWiring` seam by supplying only the テストゴースト fixture path と in-proc 結線の選択（専用の別 boot 入口や独自オーケストレーションを新設しない）。
2. The 決定論 e2e shall run as part of `cargo test --workspace`（常設ゲート）without any env-gate や opt-in、i686 成果物、32bit helper プロセス、`pasta.dll`、別プロセス spawn、**または手動 cdylib プリビルド段**。x64 テストDLL は同一ネイティブ target のワークスペースメンバとして `cargo test --workspace` が自動ビルドする（i686 成果物を手動プリビルドする既存慣行とは非対称——本 DLL はクロスビルドでなくネイティブ同一 target ゆえ自動ビルドされる）。
3. While driving the deterministic test, the テストハーネス shall advance time only through injected Tick input と shall not use sleep や実時計待機。
4. When the test exercises the boot→talk→close 経路, the テストハーネス shall let the author assert both the SHIORI 交信列（送出されたイベント id・NOTIFY/GET の別・発火順序）and the resulting cue sink 出力（内容・順序）。交信列の観測は `ShioriBackend` seam に噛ませる記録デコレータ（`Recorder<B: ShioriBackend>` 相当）を通じて行い、InProc 実DLL backend と既存 `Custom`／`ScriptedShioriBackend` fake の**双方で同一手口**とする（cue sink 記録装置と対をなす二記録装置）。
5. The テストゴースト応答 shall be deterministic——同一入力に対し常に同一応答を返す（乱数・実時計に依存しない）。
6. Where a test needs custom per-test scripting without crossing the DLL boundary, the テストハーネス shall keep the existing `ShioriWiring::Custom`（closure 注入・`ScriptedShioriBackend`）seam available and usable in parallel。

### Requirement 2: x64 SHIORI4 テスト DLL（決定論脳・台本）

**Objective:** As the boundary-crossing deterministic test seam, I want an x64 SHIORI4 dynamic library that answers a canonical event set with fixed deterministic responses, so that the real IShiori DLL boundary can be crossed reproducibly.

#### Acceptance Criteria

1. The SHIORI4 テスト DLL shall be an x64 dynamic library that exposes the areka 内部 SHIORI4 生成入口（`shiori_factory`・`extern "system"`）conforming to the existing IShiori COM ABI（`IShioriFactory`／`IShiori`・生成＋load 融合）、so that it loads through the same entry a future native x64 SHIORI4 would。
2. When the DLL receives a request for a canonical M1 イベント（`OnInitialize`／`OnFirstBoot`／`OnBoot`／`basewareversion`／`OnSecondChange`／`OnClose`）, the DLL shall return the固定の決定論応答（さくらスクリプト／ステータス）を、**実 emo2 `pasta.dll` の出力から採取したゴールデンスナップショット**（正典イベントごとの実応答を凍結した静的 fixture データ）から返す。
3. When the DLL receives a request for an未知または未台本の ID, the DLL shall return a 204 相当（No Content）応答（silent success ではなく明示的な No Content）。
4. If the DLL receives a malformed または構造不整合の request, then the DLL shall return a検出可能な失敗応答（fail-visible）and shall not panic。
5. The DLL shall carry request/response content as opaque SHIORI/3.0 テキスト（不透明搬送）and shall not invent an独自 content スキーマや意味づけ分岐（採取したスナップショットをそのまま replay する＝最も忠実な不透明搬送）。
6. The 台本 スナップショット shall be採取される——実 emo2 `pasta.dll` を env-gate 実 pasta 経路（要件 6.1）で走らせて正典イベントの実応答を観測し、静的 fixture として commit する。常設決定論テストはこの静的スナップショットを replay し、実行時に `pasta.dll` を呼ばない。スナップショットは pasta の乱数応答のうち**1つの代表応答を凍結したもの**であり（pasta が常に同一応答を返すことの主張ではない）、その凍結応答上で boot→talk→close が決定論的に振る舞うことを保証する。

### Requirement 3: 正規 in-proc ロード経路（`ShioriWiring::InProc`）＋ `IShiori`→`ShioriBackend` アダプタ

**Objective:** As the boot path, I want a canonical in-process x64 SHIORI4 load seam that loads a DLL and adapts `IShiori` to the `ShioriBackend` the shiori actor drives, so that the real mount→load→exchange boundary is exercised on the same code path as production.

#### Acceptance Criteria

1. The boot 経路 shall provide a 第3の SHIORI 結線方式（`ShioriWiring::InProc`）that、given a DLL パス、loads the x64 DLL、resolves the SHIORI4 生成入口、creates a load 完了済みの `IShiori`、and adapts it to a `ShioriBackend`。
2. When the shiori アクターが GET／NOTIFY を発行する, the InProc アダプタ shall id／references／status を SHIORI/3.0 リクエストへ組み立て、`IShiori` を呼び、応答を `ShioriBackend` の戻り値（`Ok(Some)`＝Value／`Ok(None)`＝204／`Err`＝失敗）へ機械的に写像する。
3. While the in-process brain is loaded, the InProc アダプタ shall report a live status（別プロセス helper が存在しないため死活監視の対象がない——失敗は get／notify／load のエラー経路で顕在化する）。
4. When shutdown または unload が発生する, the InProc アダプタ shall正規に teardown する（`IShiori`・ロード済みライブラリの解放）。
5. If DLL ロードに失敗する（欠落 DLL・生成入口 未解決・create 失敗）, then the InProc ロード経路 shall log-first（`error!`＋失敗戻り）で接続失敗として扱い、silent に成功を偽装しない。
6. The InProc ロード経路 shall be a正規実装（テスト専用ハックではない）であり、M2 の native x64 SHIORI4 が同一シームに本番消費者として乗れる形とする。

### Requirement 4: マウント可能テストゴースト fixture

**Objective:** As the deterministic e2e, I want a complete lightweight test-ghost folder that mounts via the real descript-driven path, so that emo2（32bit pasta 脳）is not required to exercise boot.

#### Acceptance Criteria

1. The テストゴースト fixture shall provide a完全なゴーストフォルダ（`ghost/master/descript.txt`＝charset UTF-8＋`shiori,<testdll>` 行）that the実 boot 経路の descript 駆動マウント解決が成功裏に解決できる。shell（`surfaces.txt`＋PNG）と balloon 資産は emo2 の**実物をそのまま流用**する（静的データ＝決定論を損なわない）——最小 shell を自作しない。
2. When boot が the fixture を起点にマウント解決する, the fixture shall supply、過不足なく、the boot→talk→close 経路が実際に消費する要素（descript・emo2 流用 shell、必要であれば emo2 流用 balloon）。
3. The 決定論 boot→talk→close 経路 shall not load `pasta.dll` や 32bit 成果物（脳＝x64 テストDLL）。fixture の shell/balloon は emo2 実物の流用でよく——非決定論性は脳＝SHIORI に局在し、それを x64 決定論 DLL へ差し替えることが本 fixture の本質。

### Requirement 5: boot→talk→close 決定論 e2e（常設ゲート）

**Objective:** As areka's standing regression net, I want an e2e that drives the real boot path through the real DLL boundary deterministically, so that the mount→load→exchange→talk→close 経路 stays green reproducibly.

#### Acceptance Criteria

1. When the 常設 e2e が走る, the e2e shall boot the テストゴースト fixture through `ShioriWiring::InProc` and drive 実 mount 解決 → x64 SHIORI4 DLL 実ロード → SHIORI 交信 → talk 再生 → close 握手 を一周する。
2. When the boot 系列が発火する, the e2e shall observe that the canonical `OnBoot` talk（台本どおりの cue 列）が cue sink に届く。
3. When close が発生する, the e2e shall observe a clean close 握手（正規終了）。
4. The 決定論 e2e shall pass as part of `cargo test --workspace`、sleep 不使用・注入時刻（Tick）のみで駆動して、追加の手動 cdylib プリビルド段を要さない（x64 テストDLL は自動ビルドされ、e2e が build 済み DLL を locate する）。
5. The 常設決定論ゲート shall observe at the cue sink 受領レベル（さくらスクリプト→cue 配送の決定論）and shall not require 実描画（seriko/emo 合成・pixel readback）。Where deeper fidelity is wanted, 流用した emo2 実 shell/balloon 資産の上で実描画エミュレーションを opt-in の追加テストとして駆動してよい（実描画は SERIKO random blink 等の別途種固定を要し・描画の正しさ自体は emo 系既存檻が正本）。

### Requirement 6: 既存テスト資産との共存・置換規律

**Objective:** As a steward of existing coverage, I want the new deterministic gate to coexist with existing seams without regressing sign-off assets, so that boundary determinism is added without losing real-machine coverage.

#### Acceptance Criteria

1. The 本 spec shall not廃止 the既存 env-gate 実 pasta 追験（`HOST32_PASTA_DLL`／`AREKA_EMO2_REAL_RUN`）——実機サインオフの正本として残置する。
2. The 本 spec shall not改稿 the `areka-P0-emo2-conformance-e2e` の決定論 spine（`ScriptedShioriBackend` 拡張のまま・不侵）。
3. When 既存の emo2 依存 smoke（`smoke_boot_loop_exit`／`emo2_real_run`）を評価する, the 本 spec shall テストゴースト駆動への乗り換え可否／陳腐化を明示的に仕分ける（乗り換え・併存・残置のいずれかの判断を残す）。
4. The 既存 `ScriptedShioriBackend`／`ShioriWiring::Custom` seam shall併存し続ける（境界を踏まない決定論テストの経路として維持）。

### Requirement 7: スコープ境界と M2 前方整合

**Objective:** As the future M2 native x64 SHIORI4 owner and as this spec's boundary keeper, I want the InProc seam positioned as the canonical forward-scaffolding load path while keeping production wiring and out-of-scope concerns explicit, so that this test infrastructure does not overreach.

#### Acceptance Criteria

1. The `ShioriWiring::InProc` シーム shall be positioned as the正規シーム that a future M2 native x64 SHIORI4 will reuse（前方整合・第一級の布石）。
2. The 本 spec shall leave 本番 main の結線 unchanged（M1 本番ゴースト＝emo2・`Helper` 経路のまま）。
3. The descript 駆動の bitness／種別自動判別 shall remain M2 シーム予約であり、本 spec の範囲外とする（判別鍵は将来 charset 規則）。
4. The テスト DLL shall limit 自発イベント（`IShioriHost::Raise` 起点）と deferred 応答（`SHIORI_S_PENDING`）を、決定論 e2e（要件 5）が要求する範囲のみに留め、それを超える網羅を実装しない（即時応答中心の最小脳・実装基盤の選択には依存しない要件）。
5. The 本 spec shall not対象とする SAORI・里々・YAYA を。
