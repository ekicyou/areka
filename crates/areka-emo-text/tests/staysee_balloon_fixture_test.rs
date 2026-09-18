//! # staysee_balloon_fixture_test — 既定バルーン `StayseeBalloon` を検体に値を固定する
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **2.2**／**3.1**／**3.2**／**3.3**／
//! **5.4**／**7.1**・設計 **C2**）。
//!
//! ## このテストが塞ぐ穴
//!
//! `vendors/sample_ghost/StayseeBalloon/` は areka の**既定バルーン**（第三者がバルーンを
//! 同梱しないゴーストを入れたときに使われる資産）として保管した上流無改変のファイル群である。
//! 保管したファイルが欠けても増えても、あるいは定義の読み取りが後退しても、それを赤にする
//! テストが無ければ「既定バルーンが表示できる」保証は毎回の目視に頼ることになる。
//!
//! ## 検体パスは 1 定数だけが持つ（要件 3.2）
//!
//! 下流の `areka-P0-nar-install` が検体を `.nar` へ畳み、共有ヘルパへ寄せるとき、
//! 付け替えるのは [`STAYSEE_BALLOON_DIR`] の 1 行だけで済む形にしてある。パス文字列を
//! 本ファイルの他の場所へ散らしてはならない。
//!
//! ## 本番コードは 1 行も変えない（要件 8.1）
//!
//! 本ファイルは公開 API（`areka_parsers::{charset, kv, balloon}`・
//! `areka_emo_text::writing`）を**外から読むだけ**である。検体側に不足があっても
//! バルーンを改変して期待値に合わせることはしない（要件 3.9）。
//!
//! ## 決定論
//!
//! ファイル読み込みと純粋層の解決のみ。実 GPU・実窓・COM・DirectWrite を要さず、
//! 同一入力に対して常に同一の結果を返す。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_emo_present::balloon::{
    ChainTier, ResolvedFace, load_scope_balloon_model, resolve_balloon_faces,
};
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::kv::parse_kv;
use log_capture_kit::{CapturedEvent, LineFormat, capture, format_line};

// ── 検体の所在（要件 3.2: パスを持つのはこの 1 定数だけ）───────────────────

/// 保管フォルダの所在（`crates/areka-emo-text` から見た相対パス）。
///
/// **本ファイル内で検体パスを綴るのはここだけ**である。`nar-install` が共有ヘルパを
/// 導入したら、この 1 行をヘルパ呼び出しへ付け替える（要件 3.2・設計 C2）。
const STAYSEE_BALLOON_DIR: &str = "../../vendors/sample_ghost/StayseeBalloon";

/// [`STAYSEE_BALLOON_DIR`] を実体化する。
fn staysee_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(STAYSEE_BALLOON_DIR)
}

// ── 保管フォルダの期待値（要件 2.2・5.4）──────────────────────────────────────

/// 上流 `ponapalt/StayseeBalloon` `fe1b02f3` の配布物に含まれる全 29 ファイル。
///
/// areka が読まないファイル（`thumbnail.pnr`・`online*`・`marker.png`・`sstp.png`・
/// `balloonc*`・`arrow*`・`install.txt`）も削らず、告知ファイルを足しもしない
/// （要件 2.2・5.4）。並びは `verification/provenance.md` §3 のハッシュ一覧と同じ昇順。
const EXPECTED_FILE_NAMES: [&str; 29] = [
    "LICENSE",
    "arrow0.png",
    "arrow1.png",
    "balloonc0.png",
    "balloonc1.png",
    "balloonc2.png",
    "balloonc3.png",
    "balloonc4.png",
    "balloonk0.png",
    "balloonk1.png",
    "balloons0.png",
    "balloons1.png",
    "balloons2.png",
    "balloons3.png",
    "descript.txt",
    "install.txt",
    "marker.png",
    "online0.png",
    "online1.png",
    "online2.png",
    "online3.png",
    "online4.png",
    "online5.png",
    "online6.png",
    "online7.png",
    "online8.png",
    "readme.txt",
    "sstp.png",
    "thumbnail.pnr",
];

/// areka が枠として焼き込む 6 枚の原寸（IHDR 由来・image px）。
///
/// `balloons*` が本体側（scope 0）・`balloonk*` が相方側（scope 1）の系列で、
/// 面 0／1 と面 2／3 で高さが違う。この値は文字描画範囲（`validrect` の負値解決）の
/// 基準そのものなので、差し替わったら領域の期待値の意味が変わる。
const EXPECTED_FRAME_SIZES: [(&str, u32, u32); 6] = [
    ("balloons0.png", 335, 205),
    ("balloons1.png", 335, 205),
    ("balloons2.png", 335, 395),
    ("balloons3.png", 335, 395),
    ("balloonk0.png", 335, 135),
    ("balloonk1.png", 335, 135),
];

// ── 定義の読み取りの期待値（要件 3.3・7.1）──────────────────────────────────────

/// 既定バルーンの id。`descript.txt` の `id` と `install.txt` の `directory` の両方（要件 7.1）。
const EXPECTED_ID: &str = "StayseeBalloon";
/// `descript.txt`／`install.txt` の `name`。
const EXPECTED_NAME: &str = "Balloon for Staysee Syncfield";
/// `descript.txt` が宣言する文字コード（復号の規約が効いていることの確認点）。
const EXPECTED_CHARSET: &str = "Shift_JIS";
/// `descript.txt`／`install.txt` の `type`。
const EXPECTED_TYPE: &str = "balloon";

/// `validrect.left`／`top`／`right`／`bottom` の**宣言された生値**（負値は反対辺基準）。
const EXPECTED_VALIDRECT: (i32, i32, i32, i32) = (22, 20, -26, -47);
/// `font.height`。`font.name` が無いので書体は ukadoc 既定へ縮退する（要件 1.3）。
const EXPECTED_FONT_HEIGHT: u32 = 12;
/// `font.color.r`／`g`／`b`。
const EXPECTED_FONT_COLOR: (u8, u8, u8) = (0, 40, 100);

// ── 小ヘルパ（`tests/*.rs` は各々独立クレートなので本ファイル内に持つ）──────

/// 検体のテキストファイルを本番と同じ規約で復号する。
///
/// 本番経路 `areka_emo_present::balloon::load_scope_balloon_model` は
/// `decode(&bytes, DefaultEncoding::Ansi)`（**既定 Ansi・ファイル内の `charset` 宣言優先**）で
/// 読む。StayseeBalloon は `charset,Shift_JIS` を宣言しているのでその宣言が効く。
///
/// **読み込み失敗は明示的に panic する**。読めなかったときに「対象 0 件だから緑」になる形を
/// 作ってはならない。
fn read_decoded(name: &str) -> String {
    let path = staysee_root().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "既定バルーンの定義 {} の読取に失敗した（本テストはこのファイルの実在が前提）: {e}",
            path.display()
        )
    });
    assert!(
        !bytes.is_empty(),
        "既定バルーンの定義 {} が空である（空ファイルでは読み取り結果が主張と無関係になる）",
        path.display()
    );
    decode(&bytes, DefaultEncoding::Ansi)
}

/// PNG の IHDR から原寸 `(width, height)` を読む（署名 8B ＋ 長さ 4B ＋ `IHDR` 4B の直後）。
///
/// 画像デコーダ（WIC）を持ち込まないのは本ファイルを純粋層・非 COM に保つため。
/// IHDR がストリーム先頭に在ることは PNG 仕様が定めているので固定オフセットで足りる。
fn png_native_size(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("バルーン枠画像 {} の読取に失敗した: {e}", path.display()));
    assert!(
        bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR",
        "{} が PNG（先頭 IHDR チャンク付き）として読めない",
        path.display()
    );
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    (w, h)
}

/// KV から鍵を引く。無ければどのファイルのどの鍵が無いかを名指しで落とす。
fn kv_get<'a>(map: &'a BTreeMap<String, String>, file: &str, key: &str) -> &'a str {
    map.get(key).map(String::as_str).unwrap_or_else(|| {
        panic!(
            "{file} に鍵 `{key}` が無い（実在する鍵は {} 個: {:?}）",
            map.len(),
            map.keys().collect::<Vec<_>>()
        )
    })
}

// ── 保管フォルダの中身（要件 2.2・5.4）────────────────────────────

/// 保管フォルダ直下のエントリ名が 29 本の固定集合と**過不足なく**一致し、サブフォルダが 0 である。
///
/// 母数（数えた本数）を先に固定してから集合を突合する。上流を取り直してファイルが増減したら
/// ここが赤になる＝意図した検出であり、期待値を緩めるのではなく `provenance.md` を採り直す。
#[test]
fn stored_folder_holds_exactly_the_upstream_29_files() {
    let root = staysee_root();
    let entries = std::fs::read_dir(&root).unwrap_or_else(|e| {
        panic!(
            "既定バルーンの保管フォルダ {} を読めない（本テストはこのフォルダの実在が前提）: {e}",
            root.display()
        )
    });

    let mut names: Vec<String> = Vec::new();
    let mut dir_names: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            panic!("{} の列挙中に失敗した: {e}", root.display());
        });
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|e| panic!("{} の種別を取れない: {e}", root.display()));
        if file_type.is_dir() {
            dir_names.push(name.clone());
        }
        names.push(name);
    }
    names.sort();

    assert_eq!(
        dir_names.len(),
        0,
        "{}: サブフォルダは 0 であること（上流の配布物は平坦・`.git` も置かない）。実在: {:?}",
        root.display(),
        dir_names
    );
    assert_eq!(
        names.len(),
        EXPECTED_FILE_NAMES.len(),
        "{}: 直下のエントリ数が期待 {} 本に対し実測 {} 本（出所: 上流 fe1b02f3 の配布物・要件 2.2）",
        root.display(),
        EXPECTED_FILE_NAMES.len(),
        names.len()
    );

    let expected: Vec<String> = EXPECTED_FILE_NAMES.iter().map(|s| s.to_string()).collect();
    let extra: Vec<&String> = names.iter().filter(|n| !expected.contains(n)).collect();
    let missing: Vec<&String> = expected.iter().filter(|n| !names.contains(n)).collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "{}: 保管ファイルの集合が上流 29 本と食い違う。余分 {} 件 {:?}／欠落 {} 件 {:?}",
        root.display(),
        extra.len(),
        extra,
        missing.len(),
        missing
    );
}

/// 枠として焼き込む 6 枚の原寸が実 PNG の IHDR と一致する。
///
/// 原寸をハードコードしつつ実ファイルと突合するのは、PNG が差し替わったときに期待値が
/// 黙って追従して「テストが自分で期待値を作る」形になるのを防ぐためである。
#[test]
fn frame_png_native_sizes_are_pinned() {
    let root = staysee_root();
    assert_eq!(
        EXPECTED_FRAME_SIZES.len(),
        6,
        "枠画像の母数は 6 枚（`balloons0〜3` と `balloonk0〜1`）であること"
    );
    for (name, width, height) in EXPECTED_FRAME_SIZES {
        assert_eq!(
            png_native_size(&root.join(name)),
            (width, height),
            "{}: 原寸が期待 {width}×{height} と違う（出所: 上流 fe1b02f3 の IHDR 実測）",
            root.join(name).display()
        );
    }
}

// ── 定義の読み取り（要件 3.3・7.1）────────────────────────────────

/// `descript.txt` を宣言された文字コードで復号し、素性の鍵を宣言どおりに読み取る。
///
/// `charset,Shift_JIS` を宣言しているので復号の規約（既定 Ansi・宣言優先）が効いていなければ
/// `craftmanw` の日本語が化け、`name` 等の突合も意味を失う。
#[test]
fn descript_identity_keys_are_read_as_declared() {
    let map = parse_kv(&read_decoded("descript.txt"));
    assert!(
        map.len() >= 5,
        "descript.txt から読めた鍵が {} 個しかない（復号か KV 解析が壊れている合図）",
        map.len()
    );
    for (key, expected) in [
        ("charset", EXPECTED_CHARSET),
        ("type", EXPECTED_TYPE),
        ("id", EXPECTED_ID),
        ("name", EXPECTED_NAME),
    ] {
        assert_eq!(
            kv_get(&map, "descript.txt", key),
            expected,
            "descript.txt の `{key}` が宣言どおりに読めない（出所: 上流 fe1b02f3 の descript.txt）"
        );
    }
}

/// `install.txt` の `directory` が `descript.txt` の `id` と同綴りである（要件 7.1）。
///
/// 下流（`baseware-root-layout` の既定 id 定数・`nar-install` の畳み込み先・
/// `alpha-release-signoff` の zip の中の綴り）はこの事実を写す。片方だけ変わったら赤になる。
#[test]
fn install_directory_matches_descript_id() {
    let descript = parse_kv(&read_decoded("descript.txt"));
    let install = parse_kv(&read_decoded("install.txt"));

    let id = kv_get(&descript, "descript.txt", "id");
    let directory = kv_get(&install, "install.txt", "directory");
    assert_eq!(
        directory, id,
        "install.txt の `directory` と descript.txt の `id` が食い違う（既定バルーン id は 1 綴りであること・要件 7.1）"
    );
    assert_eq!(
        id, EXPECTED_ID,
        "既定バルーン id が期待 `{EXPECTED_ID}` と違う（下流 3 spec の申し送り先が写す綴り）"
    );
    for (key, expected) in [
        ("charset", EXPECTED_CHARSET),
        ("type", EXPECTED_TYPE),
        ("name", EXPECTED_NAME),
    ] {
        assert_eq!(
            kv_get(&install, "install.txt", key),
            expected,
            "install.txt の `{key}` が宣言どおりに読めない"
        );
    }
}

/// `BalloonModel` が宣言された描画範囲・文字の高さと色を写し取る。
///
/// `validrect` は**宣言された生値**で固定する（負値の反対辺解決は画像原寸に依るので
/// 後続の領域解決テストの担当）。ここで見るのは「読み手が宣言を落としていないか」である。
#[test]
fn balloon_model_reads_declared_geometry_and_font() {
    let model = staysee_model();

    let validrect = model.validrect();
    assert_eq!(
        (
            validrect.left(),
            validrect.top(),
            validrect.right(),
            validrect.bottom()
        ),
        (
            Some(EXPECTED_VALIDRECT.0),
            Some(EXPECTED_VALIDRECT.1),
            Some(EXPECTED_VALIDRECT.2),
            Some(EXPECTED_VALIDRECT.3)
        ),
        "validrect の宣言値（left/top/right/bottom）が期待 {EXPECTED_VALIDRECT:?} と違う（出所: 上流 descript.txt の `validrect.*` 4 行）"
    );

    let font = model.font();
    assert_eq!(
        font.height(),
        Some(EXPECTED_FONT_HEIGHT),
        "font.height が期待 {EXPECTED_FONT_HEIGHT} と違う（出所: descript.txt `font.height,12`）"
    );
    let color = font.color();
    assert_eq!(
        (color.r(), color.g(), color.b()),
        (
            Some(EXPECTED_FONT_COLOR.0),
            Some(EXPECTED_FONT_COLOR.1),
            Some(EXPECTED_FONT_COLOR.2)
        ),
        "font.color が期待 {EXPECTED_FONT_COLOR:?} と違う（出所: descript.txt `font.color.r/g/b`）"
    );
}

/// 宣言の無い鍵が「未指定」として扱われ、書字方向が横書きへ解決される。
///
/// StayseeBalloon は `vertical`・`writing_mode`・`wordwrappoint.*`・`font.name` を宣言しない。
/// これらが `Some` で返るようになったら、読み手が既定値を宣言と取り違えた合図である
/// （縮退の行き先＝横書き・`validrect` の遠辺・ukadoc 既定書体は下流の解決層が決める）。
#[test]
fn undeclared_keys_stay_unspecified_and_resolve_to_horizontal() {
    let model = staysee_model();

    assert_eq!(
        model.vertical_raw(),
        None,
        "`vertical` は宣言が無いので未指定であること（出所: 上流 descript.txt に当該行なし）"
    );
    assert_eq!(
        model.writing_mode(),
        None,
        "`writing_mode` は宣言が無いので未指定であること"
    );
    let wrap = model.wordwrappoint();
    assert_eq!(
        (wrap.x(), wrap.y()),
        (None, None),
        "`wordwrappoint.x/y` は宣言が無いので未指定であること（折返し基準は validrect の遠辺へ縮退する）"
    );
    assert_eq!(
        model.font().name(),
        None,
        "`font.name` は宣言が無いので未指定であること（ukadoc 既定書体へ縮退する＝要件 1.3 のとおり areka 側の既定は変えない）"
    );
    assert_eq!(
        WritingMode::resolve(&model),
        WritingMode::HorizontalTb,
        "縦書きの宣言が無いので横書きへ解決されること"
    );
}

/// `descript.txt` 単層（面別上書き層なし）から本番と同じ経路でモデルを組む。
///
/// StayseeBalloon は面別上書き層（`balloons0s.txt` 等）を持たないので第 2 引数は `None`。
fn staysee_model() -> BalloonModel {
    parse_str(&read_decoded("descript.txt"), None)
}

// ── 面の系列解決と記録（要件 3.4・3.8）────────────────────────────

/// 解決の記録の宛先（`resolve_balloon_faces`／`load_scope_balloon_model` の発行元モジュール）。
///
/// 捕捉した記録がこの宛先から出たことまで確かめるのは、別の層が出した無関係な記録を
/// 数え込んで「件数が合っている」ように見えるのを防ぐためである。
const BALLOON_LOG_TARGET: &str = "areka_emo_present::balloon";

/// どちらの scope でも解決される面の本数（面 0〜3）。
const EXPECTED_FACE_COUNT: usize = 4;

/// 本体側（scope 0）で解決される面の並び `(面 id, 採用接頭辞, 実ファイル名, 採用段)`。
///
/// 本体側は連鎖の最終受け皿そのものなので、どの面も縮退ではなく自身の候補
/// （[`ChainTier::Own`]）から採られる。
const EXPECTED_SCOPE0_FACES: [(u32, &str, &str, ChainTier); EXPECTED_FACE_COUNT] = [
    (0, "balloons", "balloons0.png", ChainTier::Own),
    (1, "balloons", "balloons1.png", ChainTier::Own),
    (2, "balloons", "balloons2.png", ChainTier::Own),
    (3, "balloons", "balloons3.png", ChainTier::Own),
];

/// 相方側（scope 1）で解決される面の並び。
///
/// StayseeBalloon は相方側の枠を 2 枚（`balloonk0`／`balloonk1`）しか持たないので、
/// 面 2・3 は面 id 単位で本体側の系列（[`ChainTier::Default`]）へ落ちる。系列全体が
/// 本体側へ切り替わるのではなく、欠けた面だけが落ちることをこの並びが固定する。
const EXPECTED_SCOPE1_FACES: [(u32, &str, &str, ChainTier); EXPECTED_FACE_COUNT] = [
    (0, "balloonk", "balloonk0.png", ChainTier::Own),
    (1, "balloonk", "balloonk1.png", ChainTier::Own),
    (2, "balloons", "balloons2.png", ChainTier::Default),
    (3, "balloons", "balloons3.png", ChainTier::Default),
];

/// 相方側で本体側の系列へ縮退する面（`warn!` の欄 `surface_id`・`prefix`）。
///
/// 本体側は縮退の概念を持たないので空、相方側はちょうどこの 2 件である。件数で固定する
/// のは、保管フォルダの面が増減したらそれ自体が上流の取り直しの合図（要件 2.2）だからである。
const EXPECTED_SCOPE1_FALLBACK: [(u32, &str); 2] = [(2, "balloons"), (3, "balloons")];

/// 保管フォルダに在って areka が面として読まない資産の内訳（名前に含まれる字面 → 本数）。
///
/// 要件 3.8 が名指しする 6 種で、合計 19 本。残る 4 本（`LICENSE` と 3 つの `.txt`）を
/// 足した 23 本が「面ではない資産」の全数＝保管 29 本 − 枠 6 枚である。
const NON_FACE_ASSET_GROUPS: [(&str, usize); 6] = [
    ("balloonc", 5),
    ("arrow", 2),
    ("online", 9),
    ("marker.png", 1),
    ("sstp.png", 1),
    ("thumbnail.pnr", 1),
];

/// 面ではない資産の全数（保管 29 本 − 枠 6 枚）。
const NON_FACE_ASSET_COUNT: usize = EXPECTED_FILE_NAMES.len() - EXPECTED_FRAME_SIZES.len();

/// 解決の 2 関数を**同じ捕捉窓**で走らせ、採用面列とその間の記録を返す。
///
/// 窓に入れるのは `resolve_balloon_faces` と `load_scope_balloon_model` の 2 つだけである
/// （焼き込み側の記録は本テストの主張の対象外）。
fn resolve_faces_with_records(scope: u32) -> (Vec<ResolvedFace>, Vec<CapturedEvent>) {
    let root = staysee_root();
    capture(|| {
        let faces = resolve_balloon_faces(&root, scope).unwrap_or_else(|e| {
            panic!(
                "scope {scope} の面の系列解決が失敗した（面 0 が解決できたことが本テストの前提）: {e}"
            )
        });
        let face0 = faces
            .iter()
            .find(|f| f.surface_id == 0)
            .unwrap_or_else(|| {
                panic!(
                    "scope {scope}: 解決結果に面 0 が無い（実測 {} 面）",
                    faces.len()
                )
            })
            .clone();
        let _model = load_scope_balloon_model(&root, scope, &face0);
        faces
    })
}

/// 記録を 1 件 1 行の読める形へ落とす（assert の失敗文言に実測を添えるため）。
fn record_lines(events: &[CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| format_line(e, LineFormat::LevelTargetFields))
        .collect()
}

/// 採用面の並び（面 id・接頭辞・ファイル名・採用段）を期待と突き合わせる。
fn assert_face_series(
    scope: u32,
    faces: &[ResolvedFace],
    expected: &[(u32, &str, &str, ChainTier)],
) {
    let actual: Vec<(u32, &str, &str, ChainTier)> = faces
        .iter()
        .map(|f| {
            (
                f.surface_id,
                f.prefix.as_str(),
                f.file_name.as_str(),
                f.tier,
            )
        })
        .collect();
    assert_eq!(
        actual.len(),
        expected.len(),
        "scope {scope}: 解決された面の本数が期待 {} に対し実測 {}（実測の並び: {:?}）",
        expected.len(),
        actual.len(),
        actual
    );
    assert_eq!(
        actual,
        expected.to_vec(),
        "scope {scope}: 面の並び（面 id・採用接頭辞・実ファイル名・採用段）が期待と違う"
    );
}

/// areka が面として読まない 23 本の資産が解決の列挙に 1 本も載らない（要件 3.8）。
///
/// 「載らない」は列挙が空でも真になるので、**先に列挙の母数を固定**してから主張する。
/// 除外側の 23 本も、要件 3.8 が名指しする 6 種の内訳で数を固定してから使う（保管フォルダ側
/// から資産が消えたら除外集合が黙って縮む形にしない）。
fn assert_non_face_assets_are_absent(scope: u32, faces: &[ResolvedFace]) {
    let frames: Vec<&str> = EXPECTED_FRAME_SIZES.iter().map(|(n, _, _)| *n).collect();
    let non_face: Vec<&str> = EXPECTED_FILE_NAMES
        .iter()
        .copied()
        .filter(|n| !frames.contains(n))
        .collect();
    assert_eq!(
        non_face.len(),
        NON_FACE_ASSET_COUNT,
        "面ではない資産の本数が期待 {NON_FACE_ASSET_COUNT} に対し実測 {}（保管 {} 本 − 枠 {} 枚）",
        non_face.len(),
        EXPECTED_FILE_NAMES.len(),
        EXPECTED_FRAME_SIZES.len()
    );
    for (needle, count) in NON_FACE_ASSET_GROUPS {
        let hit = non_face.iter().filter(|n| n.contains(needle)).count();
        assert_eq!(
            hit,
            count,
            "要件 3.8 が名指しする `{needle}` の本数が期待 {count} に対し実測 {hit}（除外対象 {}: {non_face:?}）",
            non_face.len()
        );
    }

    let listed: Vec<&str> = faces.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(
        listed.len(),
        EXPECTED_FACE_COUNT,
        "scope {scope}: 列挙の母数が期待 {EXPECTED_FACE_COUNT} 本に対し実測 {} 本（母数が 0 だと「載らない」の主張は何も確かめない）: {listed:?}",
        listed.len()
    );
    let leaked: Vec<&&str> = non_face.iter().filter(|n| listed.contains(n)).collect();
    assert!(
        leaked.is_empty(),
        "scope {scope}: areka が面として読まない資産が {} 件 {:?} 解決の列挙に載った（列挙 {} 本: {:?}）",
        leaked.len(),
        leaked,
        listed.len(),
        listed
    );
}

/// 解決の窓で捕捉した記録を突き合わせる（失敗の記録 0 件・縮退の記録は期待どおりの欄で）。
///
/// 冒頭の 2 件の情報の記録が**対照**である。「失敗の記録が 0 件」は捕捉が空振りしていても
/// 真になるので、同じ窓・同じ宛先で必ず出る記録を先に数え、発行点が生きていたことを示す。
fn assert_resolution_records(
    scope: u32,
    events: &[CapturedEvent],
    expected_fallback: &[(u32, &str)],
) {
    let lines = record_lines(events);

    // 対照 ⑴: 解決の 2 関数の発行点が窓の中で本当に動いた（系列解決の完了＋2 層マージの確定）。
    let infos: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::INFO && e.target == BALLOON_LOG_TARGET)
        .collect();
    assert_eq!(
        infos.len(),
        2,
        "scope {scope}: 宛先 {BALLOON_LOG_TARGET} の情報の記録が期待 2 件に対し実測 {} 件。0 件なら捕捉が空振りしており、以下の「0 件であること」の主張は何も確かめない。捕捉した全 {} 件: {lines:?}",
        infos.len(),
        lines.len()
    );
    // 対照 ⑵: 2 件の内訳が別々の発行点である（欄で見分ける——本文の文字列には依存しない）。
    assert_eq!(
        infos.iter().filter(|e| e.field("chain").is_some()).count(),
        1,
        "scope {scope}: 系列解決の完了の記録（欄 `chain` を持つもの）が 1 件でない: {lines:?}"
    );
    assert_eq!(
        infos
            .iter()
            .filter(|e| e.field("validrect_left").is_some())
            .count(),
        1,
        "scope {scope}: 2 層マージの確定の記録（欄 `validrect_left` を持つもの）が 1 件でない: {lines:?}"
    );

    // 失敗の記録は両 scope とも 0 件（要件 3.8）。
    let errors: Vec<&String> = events
        .iter()
        .zip(&lines)
        .filter(|(e, _)| e.level == tracing::Level::ERROR)
        .map(|(_, l)| l)
        .collect();
    assert!(
        errors.is_empty(),
        "scope {scope}: 失敗の記録は 0 件であること。実測 {} 件: {errors:?}",
        errors.len()
    );

    // 縮退の記録は本体側 0 件・相方側ちょうど 2 件。欄（宛先・scope・面 id・接頭辞）まで固定する。
    let warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(
        warns.len(),
        expected_fallback.len(),
        "scope {scope}: 縮退の記録が期待 {} 件に対し実測 {} 件。捕捉した全 {} 件: {lines:?}",
        expected_fallback.len(),
        warns.len(),
        lines.len()
    );
    let actual: Vec<(String, String, String, String)> = warns
        .iter()
        .map(|e| {
            let field = |name: &str| e.field(name).unwrap_or("<欄なし>").to_string();
            (
                e.target.clone(),
                field("scope"),
                field("surface_id"),
                field("prefix"),
            )
        })
        .collect();
    let expected: Vec<(String, String, String, String)> = expected_fallback
        .iter()
        .map(|(surface_id, prefix)| {
            (
                BALLOON_LOG_TARGET.to_string(),
                scope.to_string(),
                surface_id.to_string(),
                (*prefix).to_string(),
            )
        })
        .collect();
    assert_eq!(
        actual, expected,
        "scope {scope}: 縮退の記録の欄（宛先・scope・surface_id・prefix）が期待と違う"
    );
}

/// 本体側と相方側で面の系列が解決され、その間の記録が期待どおりである（要件 3.4・3.8）。
///
/// 相方側は枠を 2 枚しか持たないので面 2・3 が本体側の系列へ落ち、その 2 面についてだけ
/// 縮退の記録が出る。本体側は落ちる先を持たないので記録は出ない。この非対称が、記録の
/// 件数の主張が恒真でないこと（＝捕捉が本当に働いていること）の証拠を兼ねている。
#[test]
fn face_series_and_records_are_pinned_for_both_scopes() {
    let (faces0, events0) = resolve_faces_with_records(0);
    assert_face_series(0, &faces0, &EXPECTED_SCOPE0_FACES);
    assert_non_face_assets_are_absent(0, &faces0);
    assert_resolution_records(0, &events0, &[]);

    let (faces1, events1) = resolve_faces_with_records(1);
    assert_face_series(1, &faces1, &EXPECTED_SCOPE1_FACES);
    assert_non_face_assets_are_absent(1, &faces1);
    assert_resolution_records(1, &events1, &EXPECTED_SCOPE1_FALLBACK);
}
