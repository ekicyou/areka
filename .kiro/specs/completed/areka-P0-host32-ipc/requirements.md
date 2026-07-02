# Requirements Document

## Project Description (Input)

x64 の areka が emo2 の 32bit `pasta.dll` を駆動するには、x64↔i686 間の IPC・ハンドシェイク・プロセス生存管理を担う **transport 層（bytes-over-wire）** が必要である。先進坑 `pilot-shiori-host-32`（✅ go 済み 2026-07-01）で feasibility は実証済みであり、本ユニット `areka-P0-host32-ipc` は M1 `areka-P0-emo2-boot` の「① SHIORI 通信層エンジン host-32」トラックの先頭ユニットとして、その transport をクリーンに（pilot コードのコピペを禁じ、README/REPORT の検証結果を参照して一から）実装する。

本ユニットの責務は **「request bytes を送り、response bytes を受ける」seam まで**であり、SHIORI/3.0 の build/parse、`LoadLibraryW pasta.dll`、常駐 lifecycle は下流ユニットの領分として明示的に除外する。別プロセス境界＝天然のアクター境界ゆえ、parser/wintf トラックとは非衝突で安全に並走する。

## Introduction

本要件は、x64 親プロセス（host-32 x64 side）と i686 ヘルパープロセス（host-32 helper）の間で、生バイト列を往復させる transport 層のユーザー／オペレーター観測可能な振る舞いを定義する。成功の観測可能な指標は、x64 親が i686 helper を spawn し、HELLO ハンドシェイクで互いのウィンドウハンドルを交換し、WM_COPYDATA で request bytes を送出して response bytes を受領する「往復 echo」を **クラッシュ無し・デッドロック無し**で観測できることである。

本ユニットは bytes-over-wire 層に限定し、その上に載る SHIORI セマンティクスや DLL ロードには関与しない。

## Boundary Context

- **In scope（本ユニットが担う振る舞い）**:
  - i686 helper プロセスの spawn と、ブロックせずに生死を確認できる生存監視（正常終了 / 異常終了の分類）
  - WM_COPYDATA によるメッセージ framing（メッセージ種別タグ・u32 リトルエンディアンで表現した HWND・payload 長・生バイト payload）
  - HELLO ハンドシェイク（両プロセスがウィンドウハンドルを交換して相互到達可能になる）
  - 再入 RESPONSE 受信（親がブロック送信中でも helper 側が応答を受信スロットへ配送し、クロスプロセスのデッドロックを起こさない）
  - timeout / wedge（ハング）検出
  - 生バイト列の往復 echo（request bytes → response bytes）の観測
  - i686 helper のビルド（PowerShell 前提・32bit 可搬性の維持）
- **Out of scope（下流ユニット／pilot の領分・本ユニットは所有しない）**:
  - `LoadLibraryW pasta.dll` + `GetProcAddress` + load/unload/request 解決（下流 `areka-P0-host32-shiori-load`）
  - SHIORI/3.0 request の build + marshal + Value parse + charset 処理（下流 `areka-P0-host32-request`）
  - 常駐メッセージループ + `OnSecondChange` ポーリング + unload + crash 監視の lifecycle（下流 `areka-P0-host32-lifecycle`）
  - x64 `IShiori` ABI 実装本体（下流と design フェーズで結線）
  - pilot コード（使い捨て検証・仮 selftest）の再利用・コピペ
- **Adjacent expectations（隣接システム／spec への期待）**:
  - 上流 `pilot-shiori-host-32`（go 済・参照専用）: 検証結果は README/REPORT 経由でのみ参照し、コードは隔離する
  - 上流 `crates/shiori-abi`（x64 `IShiori`/`IShioriHost` COM）: 本ユニットは bytes transport を提供するのみで、ABI 実装本体は下流で結線する
  - 上流 `wintf-winmsg-executor` 0.0.5: helper 側メッセージループの基盤として利用（i686 実証済）
  - 下流 `areka-P0-host32-shiori-load` / `-request` / `-lifecycle`: 本ユニットが提供する「bytes 往復」seam の上に SHIORI セマンティクスを構築する
  - 制約変更が生じる場合の正本は `doc/COMPAT_ARCHITECTURE.md`

## Requirements

### Requirement 1: ヘルパープロセスの spawn と生存監視

**Objective:** As a host-32 x64 側の管理者コンポーネント, I want i686 helper プロセスを起動しその生死をブロックせずに確認できること, so that helper の異常終了を検知して呼び出し側が適切に対処できる

#### Acceptance Criteria

1. When x64 親が helper の起動を要求する, the host-32 x64 side shall i686 helper プロセスを spawn し、その helper への参照（プロセスハンドル）を保持する。
2. When x64 親が helper の生存を問い合わせる, the host-32 x64 side shall 呼び出し側スレッドをブロックせずに、helper が稼働中か終了済みかを返す。
3. If helper プロセスが正常終了した, then the host-32 x64 side shall 終了種別を「正常終了（Clean）」として分類し観測可能にする。
4. If helper プロセスが異常終了（クラッシュ等）した, then the host-32 x64 side shall 終了種別を「異常終了（Abnormal）」として分類し観測可能にする。
5. When helper の spawn が失敗した, the host-32 x64 side shall エラーを呼び出し側へ報告し、稼働中の helper が存在しない状態を保つ。

### Requirement 2: WM_COPYDATA メッセージ framing

**Objective:** As a host-32 の x64 側および i686 helper, I want メッセージを種別タグ・ハンドル・長さ・生バイト payload で構造化して授受できること, so that x64 と i686 の異なるビット幅を跨いでも解釈の齟齬なくデータを往復できる

#### Acceptance Criteria

1. When いずれかのプロセスが相手へメッセージを送出する, the host-32 transport shall メッセージ種別タグ・payload 長・生バイト payload を含む WM_COPYDATA として送出する。
2. Where メッセージにウィンドウハンドルを載せる, the host-32 transport shall ハンドルを u32 リトルエンディアン表現で符号化する。
3. When 受信側がメッセージを受け取る, the host-32 transport shall 種別タグと payload 長に基づき生バイト payload を取り出す。
4. The host-32 transport shall プロセス境界を跨ぐ payload を生バイト列のみとして扱い、ビット幅依存のローカル資源（32bit ローカルの HGLOBAL・x64 ローカルの HSTRING）を境界を越えて共有しない。
5. If 受信した payload 長が宣言された framing と整合しない, then the host-32 transport shall そのメッセージを不正として扱い、破損したまま上位へ渡さない。

### Requirement 3: HELLO ハンドシェイク

**Objective:** As a host-32 の x64 側と i686 helper, I want 起動直後に互いのウィンドウハンドルを交換して相互到達可能になること, so that 以降の request/response を確実に相手へ届けられる

#### Acceptance Criteria

1. When helper プロセスが起動しメッセージ受信の準備が整う, the host-32 helper shall 自身のウィンドウハンドルを u32 リトルエンディアンで HELLO として x64 親へ通知する。
2. When x64 親が helper の HELLO を受領する, the host-32 x64 side shall helper のウィンドウハンドルを記録し、ハンドシェイク完了を観測可能にする。
3. While HELLO ハンドシェイクが未完了である, the host-32 x64 side shall request/response の往復を開始しない。
4. If 所定の待機時間内に HELLO が完了しない, then the host-32 x64 side shall ハンドシェイク失敗として扱い、呼び出し側へ報告する。

### Requirement 4: 再入 RESPONSE 受信によるデッドロック回避

**Objective:** As a host-32 の x64 側, I want ブロック送信中でも helper からの応答を確実に受け取れること, so that クロスプロセスの相互待機によるデッドロックを起こさず request/response を成立させられる

#### Acceptance Criteria

1. When x64 親が request を送出する, the host-32 x64 side shall 相手への到達確認を伴うブロック送信を用いて 1 件ずつ（single-in-flight）送る。
2. When helper が request を受信する, the host-32 helper shall 応答を受信スロットへ配送したうえで受信処理から速やかに戻り、クロスプロセスの再送信を開始しない。
3. While ひとつの request が処理中である, the host-32 transport shall 次の request を新たに送出せず、応答は厳密にネストした順序で受領する。
4. The host-32 transport shall request/response の一往復を、クロスプロセスの相互ブロック（デッドロック）無しで完了する。

### Requirement 5: timeout / wedge（ハング）検出

**Objective:** As a host-32 の x64 側, I want 応答しない helper を検出して送信を打ち切れること, so that helper のハングが x64 親の無限待機に波及しない

#### Acceptance Criteria

1. When x64 親が request を送出する, the host-32 x64 side shall 応答待機に上限時間を設ける。
2. If 上限時間内に response が返らない, then the host-32 x64 side shall 送信を timeout として打ち切り、その結果を呼び出し側へ報告する。
3. If 相手プロセスがハング（応答不能）状態にある, then the host-32 x64 side shall ハングした送信を中断して呼び出し側の無限待機を防ぐ。

### Requirement 6: 往復 echo の観測

**Objective:** As a 本ユニットの検証者, I want request bytes を送出し同一内容の response bytes を受領する往復を無クラッシュで観測できること, so that transport 層が M1 のゲート指標を満たしていることを確認できる

#### Acceptance Criteria

1. When x64 親が任意の request bytes を送出する, the host-32 transport shall helper 経由で対応する response bytes を x64 親へ返す。
2. When echo 往復が完了する, the host-32 transport shall 受領した response bytes を送出した request bytes と照合可能な形で観測可能にする。
3. While 往復 echo テストを実行している, the host-32 transport shall いずれのプロセスもクラッシュさせず、デッドロックに陥らない。

### Requirement 7: i686 helper のビルドと 32bit 可搬性

**Objective:** As a 本ユニットのビルド担当, I want helper を 32bit 環境で正しくビルド・動作させられること, so that x64 親が 32bit の pasta.dll 世界へ橋渡しできる

#### Acceptance Criteria

1. The host-32 helper shall i686（32bit）ターゲットとしてビルド可能である。
2. Where helper が 32bit ポインタ幅（`usize` = 32bit）で動作する, the host-32 helper shall ポインタ／ハンドルのビット幅を跨ぐ算術でオーバーフローを起こさない。
3. The host-32 transport shall x64 側と i686 側の双方向で境界を跨ぐデータを生バイト列のみとして表現し、32bit 可搬性を維持する。

### Requirement 8: 責務境界（本ユニットが所有しないもの）

**Objective:** As a 下流ユニットおよびレビュアー, I want 本ユニットが transport（bytes 往復）の seam までに限定されることを明示的に確認できること, so that SHIORI セマンティクスや DLL ロードが誤って本ユニットへ混入しない

#### Acceptance Criteria

1. The host-32 IPC unit shall request/response を意味を持たない生バイト列として扱い、SHIORI/3.0 の build・parse・charset 変換を行わない。
2. The host-32 IPC unit shall `pasta.dll` の `LoadLibraryW` / `GetProcAddress` / load・unload・request 解決を行わない。
3. The host-32 IPC unit shall 常駐メッセージループ・`OnSecondChange` ポーリング・unload・crash 監視から成る lifecycle を所有しない。
4. The host-32 IPC unit shall 上流 `pilot-shiori-host-32` のコードをコピー・再利用せず、README/REPORT の検証結果を参照した実装に限る。
