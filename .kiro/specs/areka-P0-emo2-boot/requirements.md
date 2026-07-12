# Requirements Document

## Project Description (Input)

M-boot（「emo2 が起動して喋る」最初の可視結果）を構成する 5 トラックのエンジンは全て単体完成しているが、**それらを束ねて「動くアプリ」にする最後の一結線が誰の所有でもない**。現状の `areka` main は起動時に表示なしの記録専用 sink を 2 本挿しており、本物のゴースト窓は生成されるものの surface 未装着で不可視、シェルアニメーション側の表示終端は mock 止まり、バルーン文字層の毎フレーム駆動を回す者もいない。

本仕様（areka-P0-emo2-boot）は M-boot マイルストーンと同名の**統合ユニット**であり、`areka.exe <emo2 path>` の一発起動で「実サーフェスが既定位置に表示され、OnBoot 応答スクリプトがバルーンに typewriter 進行で流れ、close で OnClose 握手を経て全エンジンが正常終了する」ことを実現する。新規機構は作らず、**シェルアニメーション側の表示指令を表示層の指令へ変換するアダプタ 1 個＋各エンジンの結線＋二段の観測**に徹する。全上流依存（window-placement / emo-text-layer / balloon-face-cue を含む）は完了済みで、順序ゲートは解消されている。とりわけ balloon-face-cue の完了により、バルーン面切替 cue（`\b`）は parser（`Instruction::BalloonSurface`）→ dola（`CueCommand::BalloonSurface`）→ sakura → seriko を貫いて第一級の表示指令（`DisplayCommand::ShowBalloon`／`HideBalloon`）として統合層へ到達するようになった。本仕様のアダプタはこれをバルーン表示対象へのサーフェス表示指令（`binds` 既定）／非表示へ配送する責務を負う（cue 語彙自体は再定義せず消費する）。

## Introduction

本仕様は areka M1 の M-boot マイルストーンを充足する最終統合ユニットである。対象は「起動〜発話〜終了」の一本道（boot → talk → close）を実物のエンジン群で成立させる結線と、その正しさを二段（決定論 spine と env-gate 実走）で観測する仕組みである。各エンジンの内部実装・窓の生成配置・文字描画・撫で/メニュー/選択肢・一周適合証明は本仕様の対象外であり、それぞれ既存の完了仕様および後続マイルストーンが所有する。

本仕様が新たに正本として確立するのは **scope から表示対象（target）への写像**のみであり、それ以外の契約（talk 契約・再生出力契約・表示指令・バルーン面切替 cue 語彙・文字層装着 API・窓写像・死活語彙・sink 注入契約）は上流の完了仕様が定めた正本をそのまま消費する。この写像はシェル表示対象とバルーン表示対象の双方を含み、balloon-face-cue が第一級化したバルーン面切替指令（`ShowBalloon`／`HideBalloon`）のバルーン表示対象への配送も本仕様が担う。

## Boundary Context

- **In scope**: 一発起動で実 emo2 サーフェスを既定位置に表示する結線／シェルアニメーション側の表示指令（Show/Hide）を表示層のサーフェス表示指令へ変換し配送するアダプタ（scope→表示対象の写像を含む・薄い変換＋配送のみで状態を持たない）／実 sink（サーフェス sink・テキスト sink）への差し替えと、それを可能にする構築順序の再編（boot を UI 基盤初期化の後へ）／生成済み窓写像（キャラクター窓・バルーン窓）の表示層への装着（初回サーフェス表示 → 文字層スロット取得 → 文字層接続の順序遵守）／バルーン文字層のアクター起動と毎フレーム駆動の結線／バルーン面切替 cue（`\b`）の配送（balloon-face-cue が第一級化した `DisplayCommand::ShowBalloon`／`HideBalloon` をバルーン表示対象へのサーフェス表示指令〔`binds` 既定〕／非表示へ変換・配送）／窓 close から OnClose 応答の再生完了待ちを経た全エンジンの正常終了／決定論 spine 統合テスト（`cargo test --workspace` 常設・外部 CI なし）と実 pasta env-gate 実走（人間サインオフ）。
- **Out of scope**: 窓の生成・配置・ドラッグ（`areka-P0-window-placement` の責務）／バルーン文字の描画そのもの（`areka-P0-emo-text-layer` の責務）／撫で・メニュー・選択肢（M-life / M-dialogue）／boot→talk→touch→menu→close の一周適合証明（`areka-P0-emo2-conformance-e2e`）／二人立ちの表示対象割当の本格化（M-dual・写像シームまで）／各エンジンの内部改変／バルーン面切替 cue 語彙そのものの定義（`areka-P0-balloon-face-cue` が確立済み＝消費のみ）／バルーン面キーの name 形・alias 解決（将来増分）。
- **Adjacent expectations**: 上流完了仕様の正本を再定義せず消費する（talk 契約 = ghost-setup / kanade / sakura、再生出力契約 = sakura、表示指令 = seriko、バルーン面切替 cue 語彙〔parser `Instruction::BalloonSurface`・dola `CueCommand::BalloonSurface`・seriko `DisplayCommand::ShowBalloon`／`HideBalloon` の正本〕= balloon-face-cue、表示層指令・表示対象・文字層スロット = emo-present、文字層装着 API・テキスト sink = emo-text-layer、窓写像 = window-placement、死活語彙 = host32-lifecycle、sink 注入契約 = ghost-setup）。窓生成の準備失敗時に開かれるダミー窓フォールバック（window-placement の良性失敗設計）は意図的残置であり本仕様では触らない。

## Requirements

### Requirement 1: 一発起動と実サーフェスの可視化
**Objective:** ユーザーとして、emo2 のパスを 1 つ渡して areka を起動するだけで実サーフェスが既定位置に表示されてほしい。そうすれば M1 の「最初の可視結果」を目視で得られる。

#### Acceptance Criteria
1. When ユーザーが `areka.exe` に emo2 のパスを与えて起動する, the 起動統合層 shall emo2 の実サーフェスを既定位置に表示する。
2. When 生成済みの窓写像（キャラクター窓・バルーン窓）が利用可能である, the 起動統合層 shall 各窓を表示層へ装着し当該サーフェスを描画する。
3. While 表示層への装着が完了していない間, the 起動統合層 shall 窓を不可視のまま保つ。
4. The 起動統合層 shall キャラクター窓とバルーン窓の両方を表示対象として結線する。

### Requirement 2: OnBoot トーク配送とバルーンの typewriter 表示
**Objective:** ユーザーとして、起動直後に OnBoot 応答スクリプトがバルーンへ流れてほしい。そうすればゴーストが「喋る」ことを観測できる。

#### Acceptance Criteria
1. When ゴースト起動が成功しゴーストが OnBoot に応答する, the 起動統合層 shall 応答スクリプトのトークをバルーンへ配送する。
2. When トークのテキスト cue がバルーン文字層へ到着する, the バルーン文字層 shall 文字を typewriter 進行で表示する。
3. While トークが再生中である, the 起動統合層 shall バルーン文字層のフレーム駆動を毎 UI フレーム実行する。
4. When サーフェス切替を伴うトーク指令が発行される, the 起動統合層 shall 当該指令を表示層のサーフェス表示指令へ変換して配送する。

### Requirement 3: 表示指令の変換と配送（scope→表示対象 写像の正本）
**Objective:** 統合を行う開発者として、シェルアニメーション側の表示指令が正しい表示対象へ届いてほしい。そうすれば各 scope のサーフェスが正しい窓に表示される。

#### Acceptance Criteria
1. When シェルアニメーション側が表示指令 Show（scope・surface id・bind 集合）を発行する, the 起動統合層 shall 当該 scope を対応するシェル表示対象へ写像し、表示層へサーフェス表示を指示する。
2. When シェルアニメーション側が表示指令 Hide（scope）を発行する, the 起動統合層 shall 当該 scope のシェル表示対象を非表示にする。
3. When シェルアニメーション側がバルーン面表示指令 ShowBalloon（scope・surface id・bind なし）を発行する, the 起動統合層 shall 当該 scope をバルーン表示対象へ写像し、既定 bind 集合でサーフェス表示を指示する。
4. When シェルアニメーション側がバルーン面非表示指令 HideBalloon（scope）を発行する, the 起動統合層 shall 当該 scope のバルーン表示対象を非表示にする。
5. The 起動統合層 shall scope から表示対象への写像（シェル表示対象・バルーン表示対象の双方）を本仕様の正本として確立し、他の正本を再定義しない。
6. The 変換アダプタ shall 変換と配送のみを行い、状態を保持しない。
7. The 変換アダプタ shall 発行された指令を UI スレッドの表示層へ UI 配送経路で届ける。

### Requirement 4: バルーン文字層の装着順序
**Objective:** 統合を行う開発者として、バルーン文字層が確実に接続されてほしい。そうすればトークのテキストがバルーンに表示される。

#### Acceptance Criteria
1. When バルーン表示対象を装着し当該対象への初回サーフェス表示（バルーン枠表示）が完了する, the 起動統合層 shall 文字層スロットを取得して文字層を接続する。
2. If 初回サーフェス表示の完了前に文字層スロットの取得を試みる, then the 起動統合層 shall 文字層をまだ接続せず、スロット未生成の状態を尊重する。
3. When 文字層の接続が完了する, the 起動統合層 shall 以降のテキスト cue をバルーン文字層へ反映できる状態にする。

### Requirement 5: バルーン面切替 cue（`\b`）の配送
**Objective:** 統合を行う開発者として、第一級の表示指令として届くようになったバルーン面切替 cue が正しくバルーン表示対象へ配送されてほしい。そうすれば `\b` を含むスクリプトでバルーン面が切り替わり、`\b` を使わない OnBoot デモも従来どおり完走できる。

#### Acceptance Criteria
1. When バルーン面表示指令 `DisplayCommand::ShowBalloon`（scope・surface id）が統合層へ到達する, the 起動統合層 shall 当該 scope をバルーン表示対象へ写像し、既定 bind 集合の `PresentCommand::ShowSurface` として配送する。
2. When バルーン面非表示指令 `DisplayCommand::HideBalloon`（scope・`\b[-1]` 相当）が統合層へ到達する, the 起動統合層 shall 当該 scope のバルーン表示対象を非表示にする。
3. The 起動統合層 shall バルーン面キーを seriko が解決した数値 id のまま消費し、alias を再適用しない。
4. Where 決定論 spine 統合テストが `\b` を含むスクリプトで駆動される, the 起動統合層 shall バルーン面切替の配送経路（到達 → 写像 → 配送）が働くことを headless の記録で観測可能にする。
5. The OnBoot デモ shall `\b` を使用せず（emo2 fixture は balloons0.png のみ）、バルーン面切替なしで完走する。

### Requirement 6: 終了握手と全エンジンの正常終了
**Objective:** ユーザーとして、窓を閉じるとゴーストが OnClose に応答してから静かに終了してほしい。そうすれば会話が途中で切れず、後片付けが行われる。

#### Acceptance Criteria
1. When ユーザーが窓を閉じる, the 起動統合層 shall 終了理由を付与して shutdown を開始する。
2. When shutdown が開始される, the 起動統合層 shall OnClose 応答スクリプトの再生完了を待ってから終了する。
3. When 全エンジンの終了が完了する, the 起動統合層 shall プロセスを正常終了（exit 0）させる。
4. Where smoke ゲート（`AREKA_APP_SMOKE_EXIT_MS`）が有効化される, the 起動統合層 shall 本物の起動経路上で自動 close→正常終了を成立させる。

### Requirement 7: 構築順序の再編と非致命 boot
**Objective:** 統合を行う開発者として、実 sink が UI 基盤の後に結線されてほしい。そうすれば表示付きの起動が成立し、boot 失敗でもアプリが即死しない。

#### Acceptance Criteria
1. When アプリが起動する, the 起動統合層 shall UI 基盤を初期化した後に実 sink（サーフェス sink・テキスト sink）を構築し、その後で boot を結線する。
2. The 起動統合層 shall 現行の記録のみの sink（表示なし）を、表示を伴う実サーフェス sink とテキスト sink へ差し替える。
3. If boot が非致命エラーで失敗する, then the 起動統合層 shall エラーをログに記録し、アプリの実行を継続する。
4. The 起動統合層 shall 既存の非致命 boot の意味論（致命/非致命の区別）を維持する。

### Requirement 8: 決定論 spine（`cargo test --workspace` 常設観測）
**Objective:** 開発者として、起動〜発話〜終了の全経路を sleep 不使用で決定論的にテストしたい。そうすれば回帰を `cargo test --workspace`（DoD ゲート・ローカル常設。本プロジェクトは外部 CI を持たない）の実行テストの檻に入れられる。可能な限りテスト観測を目指し、実描画（GPU）まで通す。

#### Acceptance Criteria
1. Where 決定論 spine 統合テストが実行される, the 起動統合層 shall スクリプト化された SHIORI バックエンドと実 sink 経路で boot→talk 配送→close 握手の全経路を実行する。
2. Where 決定論 spine 統合テストが実行される, the 起動統合層 shall 実 sink 経路の末端（アダプタ → `EmoPresenter::apply` ／ `present_frame` の実描画 → readback）まで通し、観測境界をアダプタ出力記録に留めない。
3. The 決定論 spine 統合テスト shall sleep を使用せず、注入した Tick（`talk_time`）のみで時間を進める。
4. The 決定論 spine 統合テスト shall 実描画を headless GPU（`GraphicsCore::new()`・WARP 可・MTA COM 初期化）とオフスクリーン readback で観測し、実画面提示には依存しない（既存 `draw_readback_test`／`attach_wiring_test` と同一方針）。
5. The 決定論 spine 統合テスト shall readback したピクセル述語（例: 可視グリフ増加に伴う非透明ピクセル単調増加・validrect 外に非透明なし・Clear 後全域透明）で実描画結果を観測する。
6. The 決定論 spine 統合テスト shall x64 で完結し（WARP は x64）、i686 成果物へ依存しない。

### Requirement 9: 実 pasta 実走（env-gate＋人間サインオフ）
**Objective:** 開発者として、実 pasta.dll・実 DPI での起動を任意で追験したい。そうすればマイルストーンの最終サインオフを人間判断で行える。

#### Acceptance Criteria
1. Where 実走テストが env-gate で有効化される, the 起動統合層 shall 実 pasta.dll・実表示・実 DPI（≠96）で boot→talk→ドラッグ→close を実行する。
2. The 実走テスト shall DoD（受け入れ緑化）の前提にせず、env-gate の opt-in 追験として扱う。
3. The マイルストーン完了 shall 実走の人間サインオフを経てのみ宣言され、AI 単独では宣言されない。

### Requirement 10: 変更境界と非所有・非改変の遵守
**Objective:** 統合を行う開発者として、本仕様がアダプタ・結線・観測に限定され既存エンジンを侵さないでほしい。そうすれば完成済みエンジンの安定性を保ったまま統合できる。

#### Acceptance Criteria
1. The 起動統合層 shall 窓の生成・配置・ドラッグを所有せず、それらは window-placement の責務として尊重する。
2. The 起動統合層 shall バルーン文字の描画そのものを所有せず、それは emo-text-layer の責務として尊重する。
3. If 既存エンジンの内部改変が必要と判明する, then the 起動統合層 shall 本仕様では改変せず、該当エンジンへ増分 issue として申し送る。
4. The 起動統合層 shall 新規機構を作らず、変換アダプタ 1 個・結線・観測に範囲を限定する。
5. The 起動統合層 shall 新規の外部（crates.io）依存を追加せず、tokio を使用せず、Rust 2024 で実装する。
6. The 起動統合層 shall 表示層を UI スレッドに固定し、worker 側の sink／アダプタからは UI 配送経路を介して指令を送る。
7. The 起動統合層 shall 窓生成の準備失敗時に開かれるダミー窓フォールバックを改変せず存置する。
8. Where 結線に既存 workspace crate（`areka-seriko`／`areka-emo-present`／`areka-emo-text`／`areka-sakura`／`areka-actor` 等）の path 依存が必要となる, the 起動統合層 shall それらを `areka` バイナリ crate の path 依存へ昇格してよく、これは R10.5 の「新規依存」（外部 crates.io crate の追加）に該当しない統合結線として in-scope とする。
