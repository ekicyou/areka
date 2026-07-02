# Requirements Document

## Project Description (Input)

M1 `areka-P0-emo2-boot` の「① SHIORI 通信層エンジン host-32」トラックのユニット。当初は「host-32 flat-C load 層の結線」のみを対象としていたが、設計ディスカッションで **IShiori ABI の根幹的欠陥（`load_dir` 欠落）** が判明し、開発者判断により **IShiori / IShioriHost / コンストラクタ関数の ABI 契約是正まで本仕様 1 本に畳む**スコープ拡大が確定した（2026-07-02・/kiro-discovery 再入）。理由は `load_dir` の根幹性・所有権・Drop lifecycle の推論が host-32 flat-C 層と IShiori COM 層を地続きに貫くため、context 継続性を優先したこと。

**二層にまたがる欠落を一体で埋める**:

1. **WS-A: host-32 flat-C load 層** — x64 areka は emo2 の 32bit SHIORI DLL を in-proc ロードできない。上流 `areka-P0-host32-ipc`（完了・凍結）が bytes-over-wire transport を完成させたが、helper は `respond()` echo stub のままで実 DLL を触らず、`MsgTag::Load(2)` は定義済みだが未処理。本仕様はこの seam を埋め、x64 親が実 i686 helper 越しに SHIORI DLL を `LoadLibraryW` → `load(load_dir)` 成功（`true`）まで無クラッシュで駆動できることを観測可能にする。
2. **WS-B: IShiori ABI 是正** — 内部正準 ABI `IShiori`（`crates/shiori-abi`・完了済み `areka-P0-shiori-com` 由来）の `Load(host)` が `load_dir` を欠く。`load_dir` は「ゴースト記述・辞書・シェルの所在」を指す per-instance のリソース根であり、これを個別保持できるからこそ各ゴーストが独立して動く。加えて raw ポインタ露出による不要な `unsafe`、RAII で足りる `Unload` の冗長が同居する。本仕様は factory 融合 create（`IShioriFactory::create(load_dir, shiori_name, host)`）・`Get`/`Notify` 分離・`GetProperty`/`SetProperty` 新設・型付き COM 引数への安全化・module entry `shiori_factory` 化で是正する。

両ワークストリームは **D1（load_dir の per-instance 貫通）と D7（teardown の Drop(RAII) 全層一貫）** の一貫原則で束ねられる。Confirmed Decisions D1–D7（brief.md）は開発者ディスカッションで確定済みであり、本要件はそれを忠実に実現する。

## Introduction

本仕様の到達目標は「**下から上まで `load_dir` が per-instance で貫通し、teardown が Drop(RAII) で一貫する**」ことである。

- **WS-A（host-32 flat-C load 層）**: helper の echo stub を置換して `MsgTag::Load` をロード実行トリガとして結線し、helper 内に `load`/`unload`/`request` 3 エクスポートの fn ポインタを保持する常設プロキシを確立、`load(load_dir)` の同期 bool 結果を凍結 wire 上の 1 byte ack で親へ返す。検証の主役は host-32 トラック所有の最小 SHIORI DLL fixture であり、本物 emo2 `pasta.dll` は env-gated の任意 confidence に留める。
- **WS-B（IShiori ABI 是正）**: `IShioriFactory::create` が生成＋load を融合して per-instance の load_dir を COM 契約面で受け、`IShiori` は `Get`/`Notify` へ痩身、`Unload` は消えて Drop(RAII) が teardown を担い、メソッドは型付き COM 引数で安全化される。ABI 証明は reference/mock backend で行い、host-32 互換 backend の factory 実装は下流 `areka-P0-host32-request` に委ねる。

本仕様は上流 `areka-P0-host32-ipc` の WM_COPYDATA transport（wire/framing/`MsgTag` 定義）を一切改変しない。`request` の呼出・SHIORI/3.0 セマンティクス・常駐 lifecycle は明示的に対象外である。

## Boundary Context

- **In scope（本仕様が担う観測可能な振る舞い）**:
  - **[WS-A]** helper の `MsgTag::Load` トリガ結線（echo stub 置換）／SHIORI DLL の `LoadLibraryW` ロードと 3 エクスポート解決・`load` 呼出（ANSI(CP_ACP) 符号化・HGLOBAL 所有権規約・Drop 時 courtesy unload）／`spawn` 起動パラメーター契約の拡張（load_dir・SHIORI 名の明示 arg＋env fallback・cwd=load_dir 維持）／最小 SHIORI DLL fixture crate 新設／LOAD E2E（成功・失敗・無クラッシュ）観測。
  - **[WS-B]** `IShiori`/`IShioriHost` の ABI 是正（factory 融合 create・`Get`/`Notify` 分離・`GetProperty`/`SetProperty` 新設・raw ポインタ排除の型付き契約面・`Unload` 削除→Drop teardown）／module entry `shiori_factory`＋`IShioriFactory` 新設（旧 `shiori_create` 置換）／ergonomic 層・mock/テスト・consumer（reference brain・`ShioriSession`・host sink・demo）の波及更新。ABI 証明は reference/mock backend で行う。
- **Out of scope（本仕様が所有しないもの）**:
  - `request` の**呼出**・SHIORI/3.0 build/marshal・`Value` parse・request の UTF-8 charset（→ 下流 `areka-P0-host32-request`）。※`request` fn ポインタの**解決**は本仕様、**呼出はしない**。
  - 常駐メッセージループ生存・`OnSecondChange` poll・crash 監視・`unload` の恒常呼出という lifecycle（→ 下流 `areka-P0-host32-lifecycle`）。※teardown の Drop courtesy unload は本仕様。
  - WM_COPYDATA transport（`shiori-host32-ipc` の wire/framing/`ResponseSlot`/HELLO/timeout/`MsgTag` 定義）の改変（上流完了・凍結）。
  - host-32 互換 backend の `IShioriFactory` 実装（`create`=spawn＋LOAD トリガ＋ack、`Get`=request wire の結線）——`Get` 結線が必須ゆえ下流 `areka-P0-host32-request` の領分。本仕様は両半（ABI＝reference/mock・flat-C load＝WS-A E2E）を個別に証明する。
  - native x64 脳の実装本体・里々/YAYA・SAORI・M2 互換面拡大。
- **Adjacent expectations（隣接仕様への期待）**:
  - **上流 `areka-P0-host32-ipc`（完了・凍結）**: `MsgTag{Hello=1,Load=2,Request=3,Response=4,Unload=5}`／WM_COPYDATA framing（`dwData` 低 32bit=タグ・`cbData`=生バイト長・ヘッダ無し）／`send_copydata`・`send_request`・`ResponseSlot`（再入 RESPONSE）／`SMTO_ABORTIFHUNG`+timeout を提供済み。本仕様の `spawn` 起動パラメーター拡張は launch 契約の拡張であって、凍結 wire には及ばない。
  - **SHIORI 名の出所（ukadoc 正典）**: SHIORI DLL 名は `descript.txt` の `shiori,<ファイル名>`（既定 `shiori.dll`）で与えられる。emo2 の `pasta.dll` は一例。**名前解決（descript 参照）は親／`package-mount` の領分**であり、本仕様の各層（helper・factory）は解決済みの名前を受け取るのみで descript を解釈しない。
  - **是正対象の完了済み spec `areka-P0-shiori-com`**: 本仕様が IShiori ABI を是正する（開発者決定により別 revisit せず本仕様 1 本・同一ブランチ内で consumer 追随まで完結）。IID は dev-stage の流動契約であり再採番可（互換維持義務なし）。
  - **参照専用 `pilot-shiori-host-32`（go 済 2026-07-01・コピペ禁止）**: FFI シーケンス／charset 非対称／HGLOBAL 所有権規約の一次記録。production クレートは `crates/pilot` へ inbound 依存しない（葉ノード隔離）。
  - **`vendors/pasta`（flat-C ABI の正確源）と `doc/COMPAT_ARCHITECTURE.md`（互換正本）**: 実装前に `git submodule update --init` で展開し flat-C 署名をバイト正確に確認する（前提制約であり要件ではない）。
  - **プロパティシステムの詳細 semantics**（key 名前空間の網羅・欠落 key の扱い・スレッド安全性・host-32 越しの実現方式）は design フェーズで確定する（brief Open Questions）。

## Requirements

### Requirement 1: 共通原則 — load_dir の per-instance 貫通（D1）

**Objective:** As a areka ベースウェア開発者, I want `load_dir` が flat-C 層から IShiori COM 層まで per-instance の必須入力として貫通する契約, so that 各ゴーストが自分のリソース根（記述・辞書・シェルの所在）を個別保持して独立動作できる

#### Acceptance Criteria

1. The 本仕様の各契約面（helper 起動パラメーター・flat-C `load` 入力・`IShioriFactory::create` 引数） shall `load_dir` を per-instance の必須入力として受け取る（省略・プロセス共有グローバル化・cwd 推測による代替を認めない）。
2. If いずれかの契約面で `load_dir` が供給されない, then the 当該層 shall 生成／ロードを決定的な失敗として観測可能にする（暗黙の既定値で続行しない）。
3. The `IShioriFactory::create` shall 呼出ごとに独立した per-ghost インスタンスを返し、各インスタンスは自身の `load_dir` に束縛される（エンジン実体が内部共有でもインスタンスごとの `load_dir` で独立動作する契約）。

### Requirement 2: 共通原則 — teardown の Drop(RAII) 全層一貫（D7）

**Objective:** As a areka ベースウェア開発者, I want teardown が明示メソッドではなく所有権解放（Drop）で全層一貫する契約, so that リソース解放経路が一本化され「解放し忘れ」や冗長 API が構造的に排除される

#### Acceptance Criteria

1. The 本仕様の SHIORI インスタンス契約（IShiori・helper 内 DLL プロキシ） shall 明示的 teardown メソッド（`Unload` 等）を公開せず、所有権の解放（Drop）を teardown の唯一の経路とする。
2. When helper 内 DLL プロキシが破棄される, the helper shall best-effort の courtesy `unload` 呼出と DLL 解放（`FreeLibrary`）を行う。
3. If courtesy teardown が失敗またはハングする, then the 親／helper shall それを戻り値のエラーとして扱わない（プロセス処分は下流 `areka-P0-host32-lifecycle` のプロセス lifecycle に委ねる）。
4. The 本仕様 shall 同一 helper プロセス内の reload-in-place（再ロード）を要求しない（areka=1 helper=1 ゴースト・再生成で足りる）。

### Requirement 3: [WS-A] helper 起動パラメーター契約の拡張（D2/D2'/D3）

**Objective:** As a x64 親プロセス（host-32 ホスト層）, I want `load_dir` と SHIORI 名を helper へ起動パラメーターとして明示供給する, so that helper が wire にパスを流さず・cwd を推測せず、確実にロード対象を特定できる

#### Acceptance Criteria

1. When 親が helper プロセスを起動する, the host-32 ホスト層 shall `load_dir` と SHIORI 名を明示的コマンドライン引数として helper へ渡す（既存の `parent_hwnd` 供給と同規約）。
2. The host-32 ホスト層 shall 同値を環境変数 fallback（例: `HOST32_LOAD_DIR`／`HOST32_SHIORI_NAME`）としても供給する。
3. When 親が helper プロセスを起動する, the host-32 ホスト層 shall helper の作業ディレクトリ（cwd）を `load_dir` に設定する（伺か慣習: SHIORI は自ディレクトリ cwd 前提で相対 I/O を行う）。
4. The helper shall `load_dir` と SHIORI 名の**値**を明示引数（不在時は env fallback）から取得し、cwd から推測しない（cwd と load 引数は同一 ghost/master を指して整合する）。
5. If 必須起動パラメーター（`load_dir`・SHIORI 名）が引数にも env にも欠落している, then the helper shall ロードを決定的な失敗として観測可能にする（黙って既定値を仮定しない）。
6. The helper shall 受領した SHIORI 名（`descript.txt` の `shiori,<ファイル名>`・既定 `shiori.dll` を親側で解決済み）をそのまま使用し、descript.txt を自ら解釈しない。

### Requirement 4: [WS-A] LOAD トリガ結線と SHIORI DLL ロード実行

**Objective:** As a host-32 helper プロセス, I want `MsgTag::Load` をトリガとして実 SHIORI DLL をロードし flat-C `load` を呼び出す, so that echo stub のままでは決して動かなかった emo2 の脳を駆動する常設足場ができる

#### Acceptance Criteria

1. When helper が `MsgTag::Load` のフレームを受領する, the helper shall それを SHIORI ロード実行の**トリガ**として処理する（現行の「既知だが無視」扱いを置換し、ペイロードにパスを期待しない）。
2. When ロードが実行される, the helper shall `load_dir\<SHIORI名>` の DLL をロードし、flat-C の `load`／`unload`／`request` **3 エクスポートすべて**を解決して fn ポインタとして保持する常設プロキシを確立する。
3. The helper shall 本仕様の範囲では 3 エクスポートのうち `load` のみを呼び出す（`request` の呼出は下流 `areka-P0-host32-request`、`unload` の恒常呼出は下流 `areka-P0-host32-lifecycle`。ただし Requirement 2 の courtesy unload は除く）。
4. When `load` を呼び出す, the helper shall `load_dir` を ANSI(CP_ACP) で符号化し、`GlobalAlloc(GMEM_FIXED)` で確保した HGLOBAL バッファとして長さと共に渡す。
5. The helper shall `load` へ渡した入力 HGLOBAL を自ら解放しない（所有権は DLL 側へ移転・二重解放を発生させない）。
6. The helper が用いる flat-C 契約 shall `vendors/pasta` を正確源とする署名（cdecl・戻り値は 1 byte の Rust `bool`（Win32 BOOL ではない）・`request` の長さ引数は in/out）に一致する。
7. The 親子間の観測契約 shall 「`load` の同期 bool 結果＋無クラッシュ」のみとし、SHIORI DLL の内部スレッド等の内部動作に前提を置かない。

### Requirement 5: [WS-A] load-ack 応答（D5・凍結 wire 上）

**Objective:** As a x64 親プロセス, I want `load` の結果を凍結 wire 上の 1 byte ack として同期受領する, so that 新タグや framing 変更なしにロード成否を確実に観測できる

#### Acceptance Criteria

1. When helper で `load` 呼出（またはロード試行）が完了する, the helper shall `MsgTag::Response` の 1 byte bool ペイロード（`[1]`=成功・`[0]`=失敗）で親へ ack を返す。
2. The ack shall 既存の再入 RESPONSE 経路（親の `send_request(MsgTag::Load, 空ペイロード, timeout)` → `ResponseSlot`）にそのまま乗り、新しい `MsgTag`・framing 変更・wire 改変を伴わない。
3. When 親が ack `[1]` を受領する, the 親 shall SHIORI ロード成功（DLL ロード・3 エクスポート解決・`load`→`true`）を観測できる。
4. If helper が timeout 内に ack を返さない, then the 親 shall 上流凍結 transport の既存 timeout 機構によって失敗を検出する（本仕様は timeout 機構を変更しない）。

### Requirement 6: [WS-A] 失敗パスの決定的観測

**Objective:** As a x64 親プロセス, I want ロード失敗の各態様がクラッシュせず決定的に観測できる, so that fixture による失敗パステストと実運用時の障害切り分けが成立する

#### Acceptance Criteria

1. If DLL ファイルが存在しない、またはロードに失敗する, then the helper shall クラッシュせず失敗 ack（`[0]`）を親へ返す。
2. If 3 エクスポートのいずれかが解決できない, then the helper shall クラッシュせず失敗 ack を返す。
3. If `load` が `false` を返す, then the helper shall 失敗 ack を返す。
4. While ロードが失敗した後も, the helper プロセス shall クラッシュせず生存を維持する（プロセス処分の判断は親／下流 lifecycle の領分）。

### Requirement 7: [WS-A] 最小 SHIORI DLL fixture と E2E 検証（D4）

**Objective:** As a host-32 トラックの開発者, I want トラック所有の最小 SHIORI DLL fixture で決定的な E2E 検証を行う, so that 本物 `pasta.dll` に依存せず成功・失敗・無クラッシュを CI で再現可能に観測できる

#### Acceptance Criteria

1. The host-32 トラック shall 自トラック所有の最小 SHIORI DLL fixture（i686 cdylib・出力名 `shiori.dll`・flat-C の `load`/`unload`/`request` を実装・数 KB 規模）を新設する。
2. Where 環境変数による失敗指示が与えられている, the fixture shall `load` の戻り値を `false` に強制でき、失敗パスを決定的にテスト可能とする。
3. The E2E 検証 shall 実 i686 helper プロセス越しに fixture のロード成功（ack `[1]`）・失敗（ack `[0]`）・無クラッシュを観測する。
4. Where 環境変数 `HOST32_PASTA_DLL` が設定されている, the E2E 検証 shall 本物 emo2 `pasta.dll` による追加の confidence 検証を実行する。
5. If `HOST32_PASTA_DLL` が設定されているのに指定 DLL が見つからない, then the テスト shall 明示的に失敗する（silent skip を認めない）。
6. The E2E 検証 shall 本物 `pasta.dll` を CI 必須ゲートとして要求しない（先進坑 `pilot-shiori-host-32` の go 済み実証を根拠とする）。
7. The fixture crate shall `crates/pilot` へ依存しない（葉ノード隔離の維持）。

### Requirement 8: [WS-B] IShioriFactory 新設と module entry 是正（D6）

**Objective:** As a areka 本体（SHIORI consumer）, I want 生成＋load を融合した factory 契約と是正された module entry, so that `load_dir` を持たない（＝自分の資源の在り処を知らない）SHIORI インスタンスが契約上存在し得なくなる

#### Acceptance Criteria

1. The shiori-abi shall 新 interface `IShioriFactory`（新規 IID）を定義し、`create(load_dir, shiori_name, host)` → 成功時 `IShiori` を返す操作を per-ghost インスタンス生成の唯一の経路とする。
2. The `create` shall 生成と load を**融合**し、成功時には load 完了済みの `IShiori` インスタンスを返す（呼出側に別途の Load 手順を要求しない）。
3. The `create` shall 単一の普遍署名とする: `shiori_name`（呼出側が descript から解決済み）を常時受け取り、互換 backend はそれを使用し、native backend は無視または検証してよい（backend 対応別の複数 create 分割は採らない）。
4. The factory shall backend（native／host-32 互換／mock）単位で提供され、instance は per-ghost とする。
5. The module entry shall C エクスポート `shiori_factory`（`extern "system"`・raw out-param・GetProcAddress 可能）として factory を返し、旧 `shiori_create` エクスポートを置換する（残置しない）。
6. If `create` が失敗する（load 失敗を含む）, then the factory shall エラーを返し、半構築のインスタンスを呼出側へ露出しない。

### Requirement 9: [WS-B] IShiori の痩身 — Get/Notify 分離

**Objective:** As a areka 本体（SHIORI consumer）, I want `IShiori` が応答ありの `Get` と片道の `Notify` の 2 操作に痩身される, so that SHIORI/3.0 の GET/NOTIFY 意味論が契約面で直截に対応し、冗長な Load/Unload が消える

#### Acceptance Criteria

1. The `IShiori` shall `Get(input)` → 応答結果（即時応答、または token を伴う遅延）と `Notify(input)` → 成否のみ（応答なし・片道）の 2 操作で構成される。
2. The `IShiori` shall `Load`／`Unload` メソッドを持たない（load は `IShioriFactory::create` に融合済み・teardown は Drop(RAII)＝Requirement 2）。
3. The `Get` shall GET SHIORI/3.0 の後継（応答あり）、`Notify` shall NOTIFY SHIORI/3.0 の後継（応答なし）として意味対応を保つ。
4. When `Get` が遅延応答となる, the `IShiori` shall token を返し、最終応答は `IShioriHost::Complete(token, response)` 経由で届く契約とする。

### Requirement 10: [WS-B] IShioriHost の役割拡充 — プロパティアクセス新設

**Objective:** As a SHIORI 脳の実装者, I want host sink 経由で自発スクリプト送出・遅延応答完了に加えて共有プロパティの読み書きができる, so that 脳がベースウェア側の共有変数（プロパティシステム）へ正当な経路でアクセスできる

#### Acceptance Criteria

1. The `IShioriHost` shall 既存の `Raise(script)`・`Complete(token, response)` に加え、`GetProperty(key)` → 値 と `SetProperty(key, value)` を提供する。
2. The プロパティ key shall SSP プロパティシステムの dotted パス名前空間（ukadoc プロパティシステム準拠）に従う。
3. While 脳が `Get` 要求を処理している最中でも, when 脳が `GetProperty` を呼び出す, the host sink shall **同期的に**値を応答する（mailbox 投函等の遅延応答で代替しない＝再入前提の契約）。
4. The `create` に渡される host sink shall 共同所有として扱われ、インスタンス生存中は sink への callback（`Raise`/`Complete`/`GetProperty`/`SetProperty`）が可能である。
5. The 本仕様のプロパティ要件 shall 契約面（操作の存在・同期性・key 名前空間）に限定し、key の網羅・欠落 key の応答・スレッド安全性・host-32 越しの実現方式は design フェーズで確定する。

### Requirement 11: [WS-B] 型付き COM 契約面への安全化

**Objective:** As a shiori-abi の利用者（脳実装者・consumer 実装者）, I want interface 契約面から raw ポインタが排除され型付き引数になる, so that ABI 境界の誤用（null・二重ポインタ・寿命違反）が型で防がれ不要な `unsafe` が消える

#### Acceptance Criteria

1. The shiori-abi の interface 契約面（`IShioriFactory`・`IShiori`・`IShioriHost` の各操作） shall 呼出側から見える署名において raw ポインタを排した型付き契約（文字列は HSTRING 参照・interface は型付き参照渡し・出力は型付き out 表現または Result 直返し）とする。
2. The C エクスポート入口（`shiori_factory`）のみ shall `extern "system"`＋raw out-param を維持する（GetProcAddress 互換のための唯一の例外）。
3. Where windows-core の interface 定義機構が特定箇所で非 `unsafe`／型付き表現を許さない, the 実装 shall 最小の `unsafe` を当該箇所に局所化しつつ、契約面（利用側から見える型付き署名）を保つ（zero-unsafe を硬性の受入条件としない。実装可否の確定は design フェーズの検証項目）。
4. The 既存 IID 群 shall dev-stage の流動契約として再採番してよい（旧 IID との互換維持を要求しない）。

### Requirement 12: [WS-B] consumer 波及更新と ABI 証明

**Objective:** As a areka 開発者, I want ABI 是正の波及先（reference brain・セッション層・mock/テスト・ergonomic 層）が同一ブランチ内で新 ABI に追随する, so that ワークスペース全体が整合した状態で 1 PR 完結し、新 ABI が reference/mock backend で証明される

#### Acceptance Criteria

1. The 本仕様 shall shiori-abi の mock／テスト、ergonomic 層、crates/areka の consumer（reference brain・セッション層・host sink・demo）を新 ABI へ追随更新し、ワークスペース全体のビルドとテストが通過する状態で完結する。
2. The reference brain shall 新 `IShiori`（`Get`/`Notify`）の実装・reference の `IShioriFactory` 実装・module entry `shiori_factory` を備える。
3. The セッション層（現行 activate が `load(host)` を呼ぶ経路） shall factory 経由の生成（create に load 融合済み）へ移行する。
4. The host sink 実装 shall `GetProperty` に対して同期応答できる（M1 で必要な最小のプロパティ応答意味論・詳細 semantics は design で確定）。
5. The ergonomic 層（`ShioriExt` 相当） shall 新 ABI の型付き安全メソッド化を踏まえ、存続・改廃を design フェーズで確定する（要件としては「consumer が新 ABI で機能を維持する」ことのみを要求する）。
6. The 新 ABI の証明 shall reference／mock backend で行い、host-32 互換 backend の factory 実装（create=spawn＋LOAD＋ack・Get=request wire）は要求しない（下流 `areka-P0-host32-request` の領分）。

### Requirement 13: 開発制約の遵守（横断）

**Objective:** As a areka 開発者, I want 凍結境界・隔離規律・32bit ビルド規律が本仕様の全作業で守られる, so that 上流資産の安定性と検証の再現性が損なわれない

#### Acceptance Criteria

1. The 本仕様 shall `shiori-host32-ipc` の wire/framing/`MsgTag`/`ResponseSlot`/HELLO/timeout の定義を改変しない（凍結境界）。
2. The i686 成果物（helper・fixture）のビルドおよびテスト shall PowerShell 経由で実行され、`cargo test --target i686-pc-windows-msvc` を検証に含める。
3. The 32bit 側のコード shall `dwData`/ULONG_PTR 由来の演算を u64 幅で評価する（i686 では usize=32bit という可搬性制約）。
4. The production クレート shall `crates/pilot` へ inbound 依存しない（先進坑コードは知見参照のみ・コピペ禁止）。
5. The flat-C 署名 shall 実装前に `vendors/pasta`（`git submodule update --init` で展開）との一致をバイト正確に確認した上で固定される。
