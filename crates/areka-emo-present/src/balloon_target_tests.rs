use super::*;

use super::test_support::{TempDir, emo2_balloon_root};
use areka_emo_atlas::{ElementId, MemoryDecoder};
use areka_parsers::shell::parse;

/// 不透明 1×1 PBGRA スペック（bake が placement を必ず産む＝非退化）。
fn opaque_1x1() -> (u32, u32, u32, Vec<u8>, bool) {
    (1, 1, 4, vec![10u8, 20, 30, 255], true)
}

/// R5.1/R5.3 転記一致（観測完了基準）: synthetic surfaces.txt → `shell::parse` の往復で、
/// 各面の surface id（`{ID}`）と element path（採用面の**実ファイル名**）が転記一致する。
///
/// 系列が scope 別になったため、転記される element path は接頭辞固定ではない——ここでは
/// 相方側系列の面（`balloonk0.png`）が採用面としてそのまま転記されることを併せて固定する。
/// これはファイルシステム/デコードを一切要さない純粋な転記層の檻。
#[test]
fn synthetic_text_transcribes_face_id_and_path() {
    let faces = vec![
        ResolvedFace {
            surface_id: 0,
            prefix: "balloonk".to_string(),
            tier: ChainTier::Own,
            file_name: "balloonk0.png".to_string(),
        },
        ResolvedFace {
            surface_id: 1,
            prefix: "balloons".to_string(),
            tier: ChainTier::Default,
            file_name: "balloons1.png".to_string(),
        },
    ];
    let text = synthetic_surfaces_txt(&faces);
    let shell = parse(&text);

    assert_eq!(shell.surfaces.len(), 2, "2 面 → 2 surface");
    for face in &faces {
        let surface = shell
            .surfaces
            .iter()
            .find(|s| s.id == face.surface_id)
            .unwrap_or_else(|| panic!("surface id {} が転記されていない", face.surface_id));
        assert_eq!(
            surface.elements.len(),
            1,
            "各面は単一 overlay element へ転記される"
        );
        assert_eq!(
            surface.elements[0].path.as_str(),
            face.file_name,
            "element path が採用面の実ファイル名へ転記一致しない"
        );
        assert_eq!(surface.elements[0].layer, 0, "layer 0（element0）へ転記");
    }
}

/// R5.1/R5.2/R5.3 full build: `build_balloon_target` が当該 scope の連鎖に属する面のみを
/// 列挙し、シェルと同一の parse→bake→World 経路で `(EmoWorld, AtlasTable)` を返す。
/// 非面ファイルは列挙されずアトラスにも World にも現れない。
/// MemoryDecoder ゆえ実 PNG 不要で決定論。
#[test]
fn build_balloon_target_end_to_end_frames_only() {
    let dir = TempDir::new();
    // 面 2 枚 ＋ 非面 3 種を同ディレクトリへ配置。
    dir.touch("balloons0.png");
    dir.touch("balloons1.png");
    dir.touch("balloonc0.png"); // 入力ボックス（非面）
    dir.touch("arrow0.png"); // スクロール矢印（非面）
    dir.touch("marker.png"); // マーカー（非面）

    // 枠のみデコーダへ登録（非枠は登録しない＝もし列挙されれば decode 失敗で露見する）。
    let mut dec = MemoryDecoder::new();
    let (w, h, stride, bytes, has_alpha) = opaque_1x1();
    dec.insert(
        dir.path().join("balloons0.png"),
        w,
        h,
        stride,
        bytes.clone(),
        has_alpha,
    );
    dec.insert(
        dir.path().join("balloons1.png"),
        w,
        h,
        stride,
        bytes,
        has_alpha,
    );

    let (world, table) = build_balloon_target(dir.path(), &dec, 0).expect("枠 2 枚から Ok が返る");

    // アトラスに枠 2 枚のエントリがあり placement を持つ（PNG α 尊重で焼かれる・R5.2）。
    for rel in ["balloons0.png", "balloons1.png"] {
        let id = table
            .resolve(SetId(0), rel)
            .unwrap_or_else(|| panic!("{rel} がアトラスに解決されない"));
        assert!(
            table.entry(id).placement.is_some(),
            "{rel} は不透明ゆえ placement を持つ"
        );
    }
    // 非枠は列挙対象外ゆえアトラスに存在しない（R5.3）。
    assert_eq!(table.resolve(SetId(0), "balloonc0.png"), None);
    assert_eq!(table.resolve(SetId(0), "arrow0.png"), None);
    assert_eq!(table.resolve(SetId(0), "marker.png"), None);
    assert_eq!(table.len(), 2, "生存エントリは枠 2 枚のみ");

    // World は surface id = N（balloons{N} の N）を常駐させる。
    assert!(
        world.surface(0).is_some(),
        "surface id 0（balloons0）が World にある"
    );
    assert!(
        world.surface(1).is_some(),
        "surface id 1（balloons1）が World にある"
    );
    assert!(world.surface(2).is_none(), "存在しない id は None");
}

/// R5.5 多面バルーン fixture（偶数 id・正典準拠）: TempDir へ `balloons0.png`＋`balloons2.png`
/// の 2 枚（偶数 id 0/2）を置き、`build_balloon_target` が **surface 0 と surface 2 の両面**を
/// 列挙・構築した world を返すことを固定する。既存 `..._frames_only`（id 0/1）と対をなし、
/// 面 id が飛び番（1 を欠く 0/2）でも各面が id=N でそのまま常駐することを実演する。
/// MemoryDecoder ゆえ実 PNG 不要で決定論（既存流儀踏襲）。
#[test]
fn build_balloon_target_enumerates_multiple_even_id_faces() {
    let dir = TempDir::new();
    // 偶数 id の面 2 枚（1 を欠く飛び番）を test-local fixture として自前用意（R5.5）。
    dir.touch("balloons0.png");
    dir.touch("balloons2.png");

    // 両面をデコーダへ登録（MemoryDecoder 経路・実 PNG 不要）。
    let mut dec = MemoryDecoder::new();
    let (w, h, stride, bytes, has_alpha) = opaque_1x1();
    dec.insert(
        dir.path().join("balloons0.png"),
        w,
        h,
        stride,
        bytes.clone(),
        has_alpha,
    );
    dec.insert(
        dir.path().join("balloons2.png"),
        w,
        h,
        stride,
        bytes,
        has_alpha,
    );

    let (world, table) =
        build_balloon_target(dir.path(), &dec, 0).expect("偶数 id 2 面から Ok が返る");

    // アトラスに 2 面が解決され placement を持つ（PNG α 尊重で焼かれる）。
    for rel in ["balloons0.png", "balloons2.png"] {
        let id = table
            .resolve(SetId(0), rel)
            .unwrap_or_else(|| panic!("{rel} がアトラスに解決されない"));
        assert!(
            table.entry(id).placement.is_some(),
            "{rel} は不透明ゆえ placement を持つ"
        );
    }
    assert_eq!(table.len(), 2, "生存エントリは偶数 id 面 2 枚のみ");

    // 多面列挙の要（本タスクの主張）: surface 0 と surface 2 の **両面** が world に常駐する。
    assert!(
        world.surface(0).is_some(),
        "surface id 0（balloons0）が World にある"
    );
    assert!(
        world.surface(2).is_some(),
        "surface id 2（balloons2）が World にある"
    );
    // 飛び番の欠番 id 1 は列挙対象に無いゆえ常駐しない（面 id=N の同一性を固定）。
    assert!(world.surface(1).is_none(), "欠番 id 1 は World に無い");
}

/// R1.1/R1.2/R7.2（本タスクの観測可能な完了状態）: 実 fixture `emo2-kakukaku` は本体側
/// `balloons0.png` と相方側 `balloonk0.png` を**両方**持つ。`build_balloon_target` に
/// scope を渡すと、**scope 1 の構築 World は相方側系列の面から成り**・scope 0 の構築 World は
/// 本体側系列の面から成る。
///
/// 全 scope が同一枠へ畳み込まれていれば scope 1 でも `balloons0.png` がアトラスへ載るため、
/// この檻は「単一接頭辞固定」の残存をそのまま検出する。デコードは MemoryDecoder（実 PNG
/// デコード不要・決定論）だが**列挙は実ディレクトリを走る**ゆえ、どの scope がどのファイルを
/// 採るかという主張そのものが檻になる。
#[test]
fn build_balloon_target_composes_scope_series_on_emo2_fixture() {
    let dir = emo2_balloon_root();

    // 実 fixture の面画像 2 枚をデコーダへ登録する（列挙が採った面だけがアトラスへ載る）。
    let mut dec = MemoryDecoder::new();
    let (w, h, stride, bytes, has_alpha) = opaque_1x1();
    for name in ["balloons0.png", "balloonk0.png"] {
        dec.insert(dir.join(name), w, h, stride, bytes.clone(), has_alpha);
    }

    // --- scope 1（相方側）: World は相方側系列の面から成る ---
    let (kero_world, kero_table) =
        build_balloon_target(&dir, &dec, 1).expect("scope 1 の面 0 は balloonk0 で解決する");
    assert!(
        kero_table.resolve(SetId(0), "balloonk0.png").is_some(),
        "scope 1 のアトラスは相方側系列の面 balloonk0.png から成る"
    );
    assert_eq!(
        kero_table.resolve(SetId(0), "balloons0.png"),
        None,
        "scope 1 は面 0 を相方側で解決済みゆえ本体側系列の面を採らない"
    );
    assert_eq!(kero_table.len(), 1, "解決された面は 1 枚（面 0）のみ");
    assert!(
        kero_world.surface(0).is_some(),
        "面 0 が World に常駐する（初期表示面・R2.6）"
    );

    // --- scope 0（本体側）: 同一ディレクトリでも本体側系列の面から成る ---
    let (sakura_world, sakura_table) =
        build_balloon_target(&dir, &dec, 0).expect("scope 0 の面 0 は balloons0 で解決する");
    assert!(
        sakura_table.resolve(SetId(0), "balloons0.png").is_some(),
        "scope 0 のアトラスは本体側系列の面 balloons0.png から成る"
    );
    assert_eq!(
        sakura_table.resolve(SetId(0), "balloonk0.png"),
        None,
        "scope 0 の連鎖に balloonk は無く相方側の面を採らない"
    );
    assert_eq!(sakura_table.len(), 1, "解決された面は 1 枚（面 0）のみ");
    assert!(
        sakura_world.surface(0).is_some(),
        "面 0 が World に常駐する"
    );
}

/// 面が 1 枚も解決できなければ log-first で `EmptyComposition`（Hide 縮退許容）を返す
/// （施行点は [`resolve_balloon_faces`] の面 0 必在契約＝R1.7）。
#[test]
fn no_frames_returns_empty_composition() {
    let dir = TempDir::new();
    dir.touch("balloonc0.png"); // 非バルーン面のみ配置。
    let dec = MemoryDecoder::new();

    // `(EmoWorld, AtlasTable)` は Debug 非実装ゆえ expect_err を使わず match で判定する。
    match build_balloon_target(dir.path(), &dec, 0) {
        Ok(_) => panic!("枠 0 枚なら Err のはず"),
        Err(err) => assert!(
            matches!(
                err,
                PresentError::Compose(ComposeError::EmptyComposition(0))
            ),
            "枠不在は EmptyComposition(0) へ畳む: {err:?}"
        ),
    }
}

// ── 後方互換の非回帰（R1.4/R5.4/R5.5・tasks 5.1）──────────────────────────────────
//
// 「本仕様適用前」とは merge-base（969a9b3）の実装を指す——面の列挙は**固定接頭辞
// `balloons`** で `{接頭辞}{数字}.png`（大小無視）だけを採り、surface id 昇順で返していた
// （`FRAME_PREFIX` / `frame_id` / `enumerate_frames`）。適用前のバイナリは手元に残らないが、
// 規則そのものは数行で再現できるため test-local な**神託**として持ち、適用後の解決結果を
// 直接突き合わせる。これにより「同一に保つ」という主張が、手書き期待値ではなく
// **適用前の規則そのもの**に対して固定される。

/// 本仕様適用前の固定接頭辞列挙を再現する神託（`(面 ID, 実ファイル名)` の surface id 昇順）。
///
/// 適用前の判定は `strip_prefix("balloons")` → `strip_suffix(".png")` →
/// `parse::<u32>()` の 3 段であった（現行の全数字明示検査より 1 段緩く、`balloons+0.png` の
/// ような符号付き病的名を受理し得た点だけが異なる。正典の面 ID 表記は符号を持たず、
/// 実資産にそのような名は現れないため面集合の同一性には影響しない）。
fn pre_spec_faces(dir: &Path) -> Vec<(u32, String)> {
    let mut faces: Vec<(u32, String)> = std::fs::read_dir(dir)
        .expect("神託: バルーンディレクトリは走査できる")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .filter_map(|name| {
            let lower = name.to_ascii_lowercase();
            let stem = lower.strip_prefix("balloons")?.strip_suffix(FRAME_SUFFIX)?;
            stem.parse::<u32>().ok().map(|id| (id, name))
        })
        .collect();
    faces.sort_unstable_by_key(|(id, _)| *id);
    faces
}

/// 指定寸の**不透明** PBGRA スペック（bake が trim せず原寸のまま placement を産む）。
/// 面ごとに別寸を与えると、アトラス要約 [`scope_digest`] が「どの画像が載ったか」を
/// ファイル名だけでなく**中身の寸**でも判別できるようになる。
fn opaque(w: u32, h: u32) -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = w * 4;
    let mut bytes = Vec::with_capacity((stride * h) as usize);
    for _ in 0..(w * h) {
        bytes.extend_from_slice(&[10u8, 20, 30, 255]);
    }
    (w, h, stride, bytes, true)
}

/// 当該 scope の解決＋構築を行い、**scope 間で比較可能な要約**へ畳む。
///
/// 要約は 3 点——(1) 面集合 `(面 ID, 実ファイル名)`、(2) World に常駐する surface id、
/// (3) アトラスの中身 `(相対パス, 原寸, placement 有無)`。tier は連鎖上の**地位**ゆえ
/// scope で異なるのが正しく（scope 0 は Own・scope≧1 は Default）、同一性の主張には
/// 含めない——R1.4/R5.4 の主張は「解決される面集合と表示物が同じ」ことである。
fn scope_digest(
    dir: &Path,
    dec: &MemoryDecoder,
    scope: u32,
) -> (
    Vec<(u32, String)>,
    Vec<u32>,
    Vec<(String, (u32, u32), bool)>,
) {
    let faces = resolve_balloon_faces(dir, scope).expect("面 0 は解決できる");
    let face_set: Vec<(u32, String)> = faces
        .iter()
        .map(|f| (f.surface_id, f.file_name.clone()))
        .collect();
    let (world, table) =
        build_balloon_target_from_faces(dir, dec, &faces).expect("採用面列からの構築は成功する");
    let resident: Vec<u32> = (0..8u32)
        .filter(|id| world.surface(*id).is_some())
        .collect();
    let mut atlas: Vec<(String, (u32, u32), bool)> = (0..table.len())
        .map(|i| {
            let id = ElementId(i as u32);
            let entry = table.entry(id);
            (
                table.key(id).rel_path.clone(),
                (entry.original.w, entry.original.h),
                entry.placement.is_some(),
            )
        })
        .collect();
    atlas.sort();
    (face_set, resident, atlas)
}

/// R1.4/R5.4（tasks 5.1）: 相方側系列（`balloonk*`）を 1 枚も持たないバルーンでは、
/// **解決される面集合も構築される World／アトラスの中身も** scope に依らず同一であり、
/// かつ本仕様適用前の固定接頭辞列挙（神託 [`pre_spec_faces`]）と一致する。
///
/// 既存檻 `select_faces_converges_to_legacy_series_for_all_scopes` は純核（ファイル名リスト）
/// までを固定していた。本檻は同じ主張を**実ディレクトリ走査＋実 bake／World 構築**の全経路で
/// 固定する——R5.4 が要求する非回帰は「面集合」だけでなく「表示」にも及ぶため。
///
/// 面ごとに別寸（面 0=12×7・面 1=5×3・面 2=9×4）の画像を与えているので、
/// もしある scope が別の面を採ればアトラス要約の寸で露見する。
#[test]
fn balloonk_absent_converges_to_pre_spec_faces_and_target_for_all_scopes() {
    let dir = TempDir::new();
    for name in ["balloons0.png", "balloons1.png", "balloons2.png"] {
        dir.touch(name);
    }
    // 非バルーン面の妨害物（どの連鎖でも採用されない＝デコーダにも登録しない）。
    for name in ["balloonc0.png", "arrow0.png", "marker.png", "balloonk.png"] {
        dir.touch(name);
    }

    let mut dec = MemoryDecoder::new();
    for (name, (w, h)) in [
        ("balloons0.png", (12u32, 7u32)),
        ("balloons1.png", (5, 3)),
        ("balloons2.png", (9, 4)),
    ] {
        let (w, h, stride, bytes, has_alpha) = opaque(w, h);
        dec.insert(dir.path().join(name), w, h, stride, bytes, has_alpha);
    }

    // 神託（適用前の固定接頭辞列挙）。
    let oracle = pre_spec_faces(dir.path());
    assert_eq!(
        oracle,
        vec![
            (0, "balloons0.png".to_string()),
            (1, "balloons1.png".to_string()),
            (2, "balloons2.png".to_string()),
        ],
        "神託の前提: 適用前は本体側 3 枚のみを面として採る"
    );

    let baseline = scope_digest(dir.path(), &dec, 0);
    assert_eq!(baseline.0, oracle, "scope 0 の面集合は適用前と同一（R1.4）");
    assert_eq!(baseline.1, vec![0, 1, 2], "面 0/1/2 が World に常駐する");
    assert_eq!(
        baseline.2,
        vec![
            ("balloons0.png".to_string(), (12, 7), true),
            ("balloons1.png".to_string(), (5, 3), true),
            ("balloons2.png".to_string(), (9, 4), true),
        ],
        "アトラスは本体側 3 枚のみ（非バルーン面は載らない）"
    );

    for scope in [1u32, 2, 5] {
        assert_eq!(
            scope_digest(dir.path(), &dec, scope),
            baseline,
            "scope {scope} の面集合・World・アトラスが scope 0 と一致しない（後方互換の破れ・R5.4）"
        );
    }
}

/// R5.5（tasks 5.1）: 相方側系列を**持つ**実 fixture でも、本体側 scope（scope 0）の
/// 面集合と表示物は本仕様適用前と同一である。
///
/// 適用後の scope 0 の連鎖は `balloonp0def` → `balloons` であり、emo2-kakukaku は
/// `balloonp0def*` を 1 枚も持たないため、実質的に適用前の固定接頭辞 `balloons` と
/// 同じ探索になる——この等価性を神託との突合で固定する（手書き期待値ではなく規則同士の突合）。
///
/// 判別力: 同一ディレクトリの scope 1 は相方側系列を採るため要約が異なる。もし scope 0 の
/// 連鎖に `balloonk` が紛れ込めば（あるいは全 scope が 1 本へ畳み戻れば）本檻が落ちる。
#[test]
fn scope0_faces_and_target_match_pre_spec_enumeration_on_emo2_fixture() {
    let dir = emo2_balloon_root();

    // 実 fixture の面画像 2 枚を**別寸**で登録する（列挙が採った面だけがアトラスへ載る）。
    let mut dec = MemoryDecoder::new();
    for (name, (w, h)) in [
        ("balloons0.png", (40u32, 22u32)),
        ("balloonk0.png", (28, 20)),
    ] {
        let (w, h, stride, bytes, has_alpha) = opaque(w, h);
        dec.insert(dir.join(name), w, h, stride, bytes, has_alpha);
    }

    let oracle = pre_spec_faces(&dir);
    assert_eq!(
        oracle,
        vec![(0, "balloons0.png".to_string())],
        "神託の前提: emo2-kakukaku の本体側系列は面 0 の 1 枚のみ"
    );

    let (faces0, resident0, atlas0) = scope_digest(&dir, &dec, 0);
    assert_eq!(
        faces0, oracle,
        "scope 0 の面集合は適用前の固定接頭辞列挙と一致する（R5.5）"
    );
    assert_eq!(resident0, vec![0], "World に常駐するのは面 0 のみ");
    assert_eq!(
        atlas0,
        vec![("balloons0.png".to_string(), (40, 22), true)],
        "scope 0 のアトラスは本体側系列の面のみ＝適用前と同一の表示物（R5.5）"
    );

    // 判別力（この等式が空虚でないことの自己検証）: 同一ディレクトリの scope 1 は別物になる。
    let (faces1, _, atlas1) = scope_digest(&dir, &dec, 1);
    assert_ne!(
        faces1, oracle,
        "scope 1 まで神託と一致するなら本 fixture は判別力を持たない"
    );
    assert_eq!(
        atlas1,
        vec![("balloonk0.png".to_string(), (28, 20), true)],
        "scope 1 のアトラスは相方側系列の面（本体側の 40×22 ではない）"
    );
}
