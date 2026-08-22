//! task 2.3: `map_get_outcome` 写像表の全行（純関数・COM 不要）＋実 DLL 越しの `InProcBackend`
//! 往復（get/notify/status/unload）＋`inproc_connect` の `file: None` 即失敗
//! （design.md §InProcBackend・「`IShiori::Get` 応答の写像表」・要件 3.2/3.3/3.4/3.5/7.1）。

use super::*;
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShioriError};

/// `S_OK`（即時応答）の HRESULT。
const S_OK: HRESULT = HRESULT(0);
/// 任意の error HRESULT（`E_FAIL` 相当）——解釈不能→Parse へ集約されること（写像表末行）。
const E_FAIL: HRESULT = HRESULT(0x8000_4005u32 as i32);

/// 正準 SHIORI/3.0 応答テキストを組む小ヘルパ（status line＋任意ヘッダ＋空行終端）。
fn resp(status_line: &str, headers: &[&str]) -> Vec<u8> {
    let mut s = String::from(status_line);
    s.push_str("\r\n");
    for h in headers {
        s.push_str(h);
        s.push_str("\r\n");
    }
    s.push_str("\r\n");
    s.into_bytes()
}

// --- map_get_outcome 写像表（全 6 行・純関数・要件 3.2）--------------------

/// `S_OK`＋200＋`Value` → `Ok(Some(value))`（写像表 1 行目・要件 3.2）。
#[test]
fn map_s_ok_200_with_value_is_some() {
    let bytes = resp(
        "SHIORI/3.0 200 OK",
        &["Charset: UTF-8", r"Value: \s[0]hi\e"],
    );
    let out = map_get_outcome(S_OK, &bytes).expect("200+Value は Ok");
    assert_eq!(out, Some(r"\s[0]hi\e".to_string()));
}

/// `S_OK`＋200（`Value` 欠落）→ `Ok(None)`（写像表 1 行目・欠落枝・要件 3.2）。
#[test]
fn map_s_ok_200_without_value_is_none() {
    let bytes = resp("SHIORI/3.0 200 OK", &["Charset: UTF-8"]);
    let out = map_get_outcome(S_OK, &bytes).expect("200(Value 欠落) は Ok");
    assert_eq!(out, None);
}

/// `S_OK`＋204 → `Ok(None)`（写像表 2 行目・要件 3.2）。
#[test]
fn map_s_ok_204_is_none() {
    let bytes = resp("SHIORI/3.0 204 No Content", &[]);
    let out = map_get_outcome(S_OK, &bytes).expect("204 は Ok");
    assert_eq!(out, None);
}

/// `S_OK`＋400 → `Err(Shiori(Status{400}))`（写像表 3 行目・テスト DLL の malformed 400 がここへ届く）。
#[test]
fn map_s_ok_400_is_shiori_status_error() {
    let bytes = resp("SHIORI/3.0 400 Bad Request", &[]);
    let err = map_get_outcome(S_OK, &bytes).expect_err("400 は SHIORI エラー");
    assert!(
        matches!(
            err,
            RequestError::Shiori(ShioriError::Status { status: 400, .. })
        ),
        "400 は Shiori(Status{{400}}): got {err:?}"
    );
}

/// `S_OK`＋500 → `Err(Shiori(Status{500}))`（写像表 3 行目・要件 3.2）。
#[test]
fn map_s_ok_500_is_shiori_status_error() {
    let bytes = resp("SHIORI/3.0 500 Internal Server Error", &[]);
    let err = map_get_outcome(S_OK, &bytes).expect_err("500 は SHIORI エラー");
    assert!(
        matches!(
            err,
            RequestError::Shiori(ShioriError::Status { status: 500, .. })
        ),
        "500 は Shiori(Status{{500}}): got {err:?}"
    );
}

/// `S_OK`＋200＋`ErrorLevel` → `Err(Shiori(Status{error_level}))`（写像表 3 行目・error_level 先読み）。
#[test]
fn map_s_ok_200_with_error_level_is_error() {
    let bytes = resp(
        "SHIORI/3.0 200 OK",
        &["ErrorLevel: warning", "Value: ignored"],
    );
    let err = map_get_outcome(S_OK, &bytes).expect_err("ErrorLevel 付きはエラー");
    assert!(
        matches!(
            err,
            RequestError::Shiori(ShioriError::Status {
                status: 200,
                error_level: Some(_),
                ..
            })
        ),
        "ErrorLevel 付きは status に関わらず Shiori エラー: got {err:?}"
    );
}

/// `S_OK`＋解釈不能バイト列 → `Err(Shiori(Parse))`（写像表 4 行目・codec 契約違反）。
#[test]
fn map_s_ok_unparseable_is_parse_error() {
    // status 行に数値コードが無い＝malformed（parse_response が Err→Parse へ集約）。
    let bytes = b"GARBAGE WITHOUT STATUS\r\n\r\n".to_vec();
    let err = map_get_outcome(S_OK, &bytes).expect_err("解釈不能は Parse");
    assert!(
        matches!(err, RequestError::Shiori(ShioriError::Parse)),
        "解釈不能は Shiori(Parse): got {err:?}"
    );
}

/// `SHIORI_S_PENDING` → `Err(Shiori(Parse))`（写像表 5 行目・防衛線・M1 InProc 非対応・要件 7.4）。
#[test]
fn map_pending_is_parse_error_defensive() {
    // SHIORI_S_PENDING は成功コードだが M1 InProc 非対応＝防衛線で Parse（要件 7.4）。
    let err = map_get_outcome(SHIORI_S_PENDING, &[]).expect_err("PENDING は防衛線で Err");
    assert!(
        matches!(err, RequestError::Shiori(ShioriError::Parse)),
        "PENDING は Shiori(Parse): got {err:?}"
    );
}

/// 任意の error HRESULT → `Err(Shiori(Parse))`（写像表 6 行目・解釈不能を Parse へ集約・D-7）。
#[test]
fn map_error_hresult_is_parse_error() {
    let err = map_get_outcome(E_FAIL, &[]).expect_err("error HRESULT は Err");
    assert!(
        matches!(err, RequestError::Shiori(ShioriError::Parse)),
        "error HRESULT は Shiori(Parse) へ集約: got {err:?}"
    );
}

// --- 実 DLL 越しの InProcBackend 往復（design.md D-1・要件 3.2/3.3/3.4）------

/// deps ディレクトリのビルド済み cdylib を実ロードして [`InProcBackend`] を組む
/// （D-1・不在は明示 panic・happy_path 檻と同一 locate 導出）。
fn build_real_backend() -> InProcBackend {
    let test_exe = std::env::current_exe().expect("test executable path is available");
    let deps_dir = test_exe
        .parent()
        .expect("test executable resides in a deps directory");
    let dll_path = deps_dir.join(shiori4_testdll::DLL_FILE_NAME);
    assert!(
        dll_path.exists(),
        "built test DLL が正準位置に不在: {}\n\
         `cargo test --workspace`（または `cargo build -p shiori4-testdll`）を先に実行すること\
         （フォールバックなし・design.md D-1）。",
        dll_path.display()
    );

    let (library, factory) = InProcLibrary::load(&dll_path)
        .expect("built cdylib は正常ロードされ factory を解決すること");
    let host: IShioriHost = InProcHost::new().into();
    let load_dir_h = HSTRING::from(deps_dir.as_os_str());
    let name_h = HSTRING::from(shiori4_testdll::DLL_FILE_NAME);
    let mut out: Option<IShiori> = None;
    // SAFETY: factory は shiori_factory が move-out した有効な IShioriFactory。host は Ref 借用、
    // out は OutRef 書込先（成功時 move-out）。
    unsafe {
        factory
            .CreateInstance(&load_dir_h, &name_h, (&host).into(), (&mut out).into())
            .expect("CreateInstance は load 完了済み IShiori を move-out すること");
    }
    // factory を FreeLibrary より先に Release（設計・順序不変条件）。
    drop(factory);
    let shiori_iface = out.expect("out に IShiori が move-out されていること");
    InProcBackend {
        shiori: Some(shiori_iface),
        host: Some(host),
        library: Some(library),
        unloaded: false,
    }
}

/// 実 `IShiori` 境界を横断して get/notify/status/unload が正しく往復すること
/// （build_request→Get→parse→map の全経路・要件 3.2/3.3/3.4）。
#[test]
fn adapter_roundtrips_through_real_ishiori() {
    let mut backend = build_real_backend();

    // GET OnFirstBoot（収載）→ Ok(Some(value))＝凍結スナップショットの Value 行 payload
    // （task 6.2 実採取の起動挨拶さくらスクリプト）。envelope 全文ではなく Value 行 payload のみが
    // 返ること（build_request→Get→parse→map の実証）。期待値は凍結スナップショットの Value 行から
    // 導出し、実採取差し替え時に自動追随させる（ハードコード giant literal を避ける）。
    let snapshot =
        shiori4_testdll::snapshot_for("OnFirstBoot").expect("OnFirstBoot は収載されていること");
    let expected_value = snapshot
        .lines()
        .find_map(|l| l.strip_prefix("Value: "))
        .expect("凍結 OnFirstBoot は Value 行を持つこと")
        .to_string();
    let onfirstboot = backend
        .get("OnFirstBoot", &[], None)
        .expect("OnFirstBoot GET は Ok");
    assert_eq!(
        onfirstboot,
        Some(expected_value),
        "OnFirstBoot は凍結スナップショットの Value 行 payload を返すこと"
    );

    // GET OnBoot（実採取後は非収載）→ 204 → Ok(None)（kanade フォールスルーで GET スキップ）。
    assert_eq!(
        backend.get("OnBoot", &[], None).expect("OnBoot GET は Ok"),
        None,
        "OnBoot は実採取後に非収載＝204→Ok(None)"
    );

    // GET 未収載 → 204 → Ok(None)。
    assert_eq!(
        backend
            .get("SomethingUnknown", &[], None)
            .expect("未収載 GET は Ok"),
        None,
        "未収載 ID は 204→Ok(None)"
    );

    // NOTIFY → Ok(())。
    backend
        .notify("OnInitialize", &[], None)
        .expect("NOTIFY は Ok");

    // status: ロード中は Running（別プロセスがなく死活監視対象なし・要件 3.3）。
    assert_eq!(
        backend.status(),
        HelperStatus::Running,
        "ロード中は Running"
    );

    // unload: 常に Ok(Clean)（構造的に不能失敗・D-7・要件 3.4）。
    assert!(
        matches!(backend.unload(), Ok(ExitKind::Clean)),
        "unload は常に Ok(ExitKind::Clean)"
    );

    // unload 後の status: Exited(Clean)（正直な語彙・D-7）。
    assert_eq!(
        backend.status(),
        HelperStatus::Exited(ExitKind::Clean),
        "unload 後は Exited(Clean)"
    );
}

// --- inproc_connect の file:None 即失敗（決定論・DLL 不要・要件 3.5）---------

/// `shiori.file` が `None`（DLL 名未解決）なら DLL ロードを試みず即座に `Err`（推測しない・要件 3.5）。
/// `shiori_wiring.rs` の `build_shiori_mount(None)` と同型に、実フィクスチャの `resolve` で得る。
#[test]
fn inproc_connect_missing_file_fails_immediately() {
    use areka_parsers::charset::DefaultEncoding;
    use areka_parsers::package;

    let unique = format!(
        "areka-ghost-inproc-connect-test-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    );
    let root = std::env::temp_dir().join(unique);
    let master_dir = root.join("ghost").join("master");
    std::fs::create_dir_all(&master_dir).expect("fixture master dir 作成");
    std::fs::create_dir_all(root.join("shell").join("master")).expect("fixture shell dir 作成");
    // shiori 行なし＝file: None（resolve が推測しない）。
    std::fs::write(master_dir.join("descript.txt"), "").expect("空 descript 書き込み");

    let mount = package::resolve(&root, DefaultEncoding::Ansi).expect("fixture の resolve");
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(
        mount.shiori.file, None,
        "fixture は shiori 行なし＝file:None のはず"
    );

    let connect = inproc_connect(mount.shiori);
    let result = connect();
    match result {
        Err(err) => assert!(
            err.contains("ファイル名"),
            "エラーはファイル名未解決が理由であることを示すこと: {err}"
        ),
        Ok(_) => panic!("file:None は接続失敗になるはず（推測しない）"),
    }
}
