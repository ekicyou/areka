//! `shell_target`: シェルのフォルダから面の絵を組み立てる**読み込みの権威**。
//!
//! シェルのフォルダを読んで「焼いた絵（[`AtlasTable`]）＋面の表を必要な数だけ組める値」
//! （[`ShellTarget`]）を作る唯一の入口である。fs を触るのは [`load_shell_target`] だけで、
//! 触らない核は [`build_shell_target`] に分かれている（テストはメモリ上の復号器で核を通せる）。
//!
//! 本モジュールはまず「シェルのフォルダ直下のファイル名から、どの番号の面の画像がどれか」を
//! 決める純粋な判定 [`select_surface_images`] を持つ（R1.1〜R1.6・R1.8）。正典は
//! `surface<数字>.png` というファイル名の慣習だけで置かれた絵を、その番号の面の画像として
//! 認める——`surfaces.txt` に `element` 行が 1 本も無いシェル（里々・YAYA の標準テンプレート）は
//! この慣習にすべてを委ねている。
//!
//! 名前の判定そのものはバルーン側と**同じ 1 つの実装**（[`crate::balloon::face_digits_of`]・
//! 接頭辞を大小無視で外す → `.png` を外す → 残りが空でなく全部 ASCII 数字）を接頭辞
//! `surface` で呼ぶ。ゆえに「バルーンの `face_id_of` と同じ扱いにそろえる」（R1.4）は申し合わせ
//! ではなく構造で成り立つ。
//!
//! # 記録を出す場所（要件 6）
//!
//! [`select_surface_images`] は fs にも記録にも触れない純粋な関数であり、重複（R1.5 の `warn!`）
//! と桁溢れ（R1.6 の `debug!`）は**事実として戻り値に載せるだけ**である。同じく
//! [`build_shell_target`] も記録を出さない（結果を [`ShellTarget`] に載せるだけ）。
//!
//! **記録を出すのは fs を触る入口 [`load_shell_target`] だけ**で、読み込み 1 回につき
//! それぞれ 1 度だけ出る——一覧の結果の `info!`（R6.1）・使わなかった画像の `debug!`（R6.2）・
//! 同じ番号の重複の `warn!`（R1.5）・桁溢れの `debug!`（R1.6）・相手の無いコマの `warn!`（R3.5）・
//! 焼く段で落ちた絵の `warn!`・3 つの失敗の `error!`（R1.7・R6.4）、および箱の報告の各件の
//! `warn!` と箱の数の `info!`（spec: areka-P0-shell-balloon 要件 10.1）、入れ子の報告（無い番号・
//! 循環）の各件の `warn!`（spec: areka-P0-surface-element-nesting 要件 3.1・3.2）、descript.txt が
//! 読めない `warn!` と添えてあった `.pna` の数の `warn!`（spec: areka-P0-self-alpha-declaration
//! 要件 5.7・7.4・透過の扱いの `info!` は `self_alpha` の宛先で 1 行）、画像だけで面を組んだ `info!`
//! （同 要件 7.3）である。一覧の 1 件だけが
//! 取れないときは `warn!` を出してその 1 件を飛ばす（[`list_file_names`]）。
//! [`ShellTarget::build_world`] は新しい記録を 1 本も出さない。
//!
//! 宛先は既定の target（`areka_emo_present::shell_target`）、文言はスコープの接頭辞 `shell:`
//! で始め、値は構造化フィールドで渡す（steering `logging.md`・design「Monitoring」）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AlphaParams, AtlasTable, BakeError, ElementDecoder, PackConfig, SetId, SurfaceSet,
    UseSelfAlpha, bake,
};
use areka_emo_compose::{
    BaseImageReport, BoxIssue, BoxLayout, BoxReport, ElementKind, EmoWorld, NestIssue, NestReport,
    element_kind, fold_boxes,
};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::shell::{
    AppendTarget, BoxDefinition, Element, ElementPath, Shell, ShellBoxes, Surface, parse_boxes,
};

use crate::balloon::face_digits_of;

/// シェルの面定義ファイル名（読むのは [`load_shell_target`] の 1 回だけ）。
const SURFACES_TXT: &str = "surfaces.txt";

/// シェルの設定ファイル名（透過の宣言 `seriko.use_self_alpha` を [`load_shell_target`] が読む）。
const DESCRIPT_TXT: &str = "descript.txt";

/// シェルの面画像の接頭辞（大小無視で比較される）。
const SURFACE_PREFIX: &str = "surface";

/// 名前の一覧から決まった「番号 → 採ったファイル名」と、その過程で捨てたもの。
///
/// 3 つの欄はどれも**入力の順に依存しない**（R1.8）。番号は数値の昇順、名前は辞書順である。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SurfaceImageSelection {
    /// 番号 → 採ったファイル名（**元の綴りのまま**——焼く側が実パスを開くため）。
    pub images: BTreeMap<u32, String>,
    /// 同じ番号に複数あったもの: `(番号, 採った名前, 捨てた名前の一覧)`。番号の昇順（R1.5）。
    pub duplicates: Vec<(u32, String, Vec<String>)>,
    /// 形は面の画像だが、数字が `u32` に収まらなかった名前。辞書順（R1.6）。
    pub overflow: Vec<String>,
}

/// 名前の一覧から「番号 → 採ったファイル名」を決める（fs に触らない純粋な関数）。
///
/// 判定と選択は次のとおり——
///
/// 1. **名前の判定**: [`face_digits_of`] に接頭辞 `surface` を与え、`surface{数字列}.png`
///    （接頭辞・拡張子は大小無視）の数字列を得る。`surfaces.txt`・`surface.png`・
///    `surface+0.png`・`surface0.pna` のように 3 段のどれかを満たさない名前は 0 件である
///    （R1.3）。同名の `.pna` を読まない（R2.6）のは、この拡張子の判定の帰結である。
/// 2. **番号の読み**: 数字列を 10 進数として読む。先頭の 0 は無視され、`surface0.png`・
///    `surface00.png`・`surface000.png`・`surface0000.png` はすべて面 0、`surface0010.png` は
///    面 10 になる（R1.2）。`u32` に収まらない数字列は画像と認めず
///    [`SurfaceImageSelection::overflow`] へ送る（R1.6）。
/// 3. **重複の裁き**: 同じ番号に複数の名前が解決したら**ファイル名の辞書順で最小**を採り、
///    残りを [`SurfaceImageSelection::duplicates`] に積む（R1.5・バルーンの `select_faces` と
///    同じ規則）。フォルダの走査順に結果が左右されないためである。
///
/// 入力はファイル名だけ——フォルダそのものやサブフォルダの中身を除くのは一覧を採る側の
/// 責務であり（R1.3）、本関数は渡された名前の列だけを見る。
pub fn select_surface_images<S: AsRef<str>>(names: &[S]) -> SurfaceImageSelection {
    // 番号 → 候補名の集合。集合を辞書順に保つことで、採用（最小）と不採用の並びが
    // 入力の順に依存しなくなる（R1.8）。
    let mut candidates: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut overflow: BTreeSet<String> = BTreeSet::new();

    for name in names {
        let name = name.as_ref();
        let Some(digits) = face_digits_of(SURFACE_PREFIX, name) else {
            continue;
        };
        // 数字列であることは判定済みゆえ、失敗は桁溢れだけである（R1.6）。
        match digits.parse::<u32>() {
            Ok(id) => {
                candidates.entry(id).or_default().insert(name.to_string());
            }
            Err(_) => {
                overflow.insert(name.to_string());
            }
        }
    }

    let mut images: BTreeMap<u32, String> = BTreeMap::new();
    let mut duplicates: Vec<(u32, String, Vec<String>)> = Vec::new();
    for (id, names) in candidates {
        let mut names = names.into_iter();
        // 候補の無い番号は作られないため、最初の 1 件が辞書順最小＝採用名である。
        let Some(adopted) = names.next() else {
            continue;
        };
        let dropped: Vec<String> = names.collect();
        if !dropped.is_empty() {
            duplicates.push((id, adopted.clone(), dropped));
        }
        images.insert(id, adopted);
    }

    SurfaceImageSelection {
        images,
        duplicates,
        overflow: overflow.into_iter().collect(),
    }
}

/// シェルの読み込みの失敗（R1.7・既存の 2 つの失敗）。
///
/// 呼び手はこの 3 つを既存の失敗へ写すだけでよい（新しい枝は要らない）——起動は
/// `BootWiringError::ShellRead`／`ShellEmpty`、採寸は `PlacementError::Measure` である
/// （写し替えはタスク 5.1）。
#[derive(Debug, thiserror::Error)]
pub enum ShellLoadError {
    /// シェルのフォルダの一覧が取れなかった（R1.7）。「画像 0 件」として先へ進まない。
    #[error("shell フォルダの一覧に失敗: {path}")]
    List {
        /// 一覧しようとしたシェルのフォルダ。
        path: PathBuf,
        /// OS が返した理由。
        #[source]
        source: std::io::Error,
    },
    /// `surfaces.txt` が在るのに読めなかった（無いときは失敗にせず画像だけで組む・
    /// spec: areka-P0-self-alpha-declaration 要件 6.1・6.6）。
    #[error("surfaces.txt の読み取りに失敗: {path}")]
    Read {
        /// 読もうとした `surfaces.txt` の絶対パス。
        path: PathBuf,
        /// OS が返した理由。
        #[source]
        source: std::io::Error,
    },
    /// シェルに面が 1 つも無い——`surfaces.txt` が無いか面を定義せず、面の画像も無い
    /// （spec: areka-P0-self-alpha-declaration 要件 6.5）。
    #[error("シェルに面が 1 つも無い: {path}")]
    Empty {
        /// シェルのフォルダ（`surfaces.txt` が無い場合があるため、そのファイルではない）。
        path: PathBuf,
    },
}

/// 読み込み済みのシェル——焼いた絵と、面の表を必要な数だけ組むのに要るものの一式。
///
/// 面の表（[`EmoWorld`]）は複製できず装着で消費されるため、値としては持たず
/// [`ShellTarget::build_world`] が呼ばれるたびに組み直す。入力（`surfaces.txt` の解析結果と
/// 「番号 → 面の画像」の対応）は読み込みの間ずっと変わらないので、何度組んでも結果は同じである。
#[derive(Debug)]
pub struct ShellTarget {
    /// `surfaces.txt` の解析結果（面の表を組むたびに読み直さない）。
    shell: Shell,
    /// 番号 → 面の画像のファイル名（採った綴りのまま）。使わなかった番号も含む全体。
    images: BTreeMap<u32, String>,
    /// 焼いた絵の索引表（`element` が名指しする絵＋土台に**使った**面の画像）。
    atlas: AtlasTable,
    /// 焼く段で落ちた絵（記録を出すのは [`load_shell_target`]）。
    bake_errors: Vec<BakeError>,
    /// 土台の絵の決定の結果（R6.1 の数と R6.2 の一覧の出どころ）。
    ///
    /// 決めるのに要る面の表は [`build_shell_target`] が既に 1 つ組んでいるので、その結果を
    /// ここへ写して持つ。入口が記録のために面の表を組み直さないためである（design
    /// 「頻度の単位」の回数が増えない）。
    base_images: BaseImageReport,
    /// 相手の面が存在しないコマの「(コマを持つ面, 相手の番号)」（R3.5・出どころは同上）。
    dangling: BTreeSet<(u32, u32)>,
    /// 箱の定義の表とサーフェス番号ごとの置き場所の表（spec: areka-P0-shell-balloon）。
    boxes: BoxLayout,
    /// 箱の畳み込みで読み捨てた事実（記録を出すのは [`load_shell_target`]）。
    box_report: BoxReport,
    /// 箱の置き場所を持つサーフェスの数（[`load_shell_target`] の `info!` の出どころ）。
    box_surfaces: usize,
    /// 入れ子の無い番号と循環の報告（記録を出すのは [`load_shell_target`]・要件 3.1・3.2）。
    nest_report: NestReport,
    /// 焼いた絵のうち同じ名前の `.pna` が添えてあったものの数（`BakeResult::ignored_pna` の写し・
    /// `.pna` は使っていない・記録を出すのは [`load_shell_target`]・要件 5.7）。
    pub ignored_pna: usize,
}

impl ShellTarget {
    /// 焼いた絵の索引表（面の表へ装着する正本）。
    pub fn atlas(&self) -> &AtlasTable {
        &self.atlas
    }

    /// 焼く段で落ちた絵の一覧（`emo2` では 0 件）。
    pub fn bake_errors(&self) -> &[BakeError] {
        &self.bake_errors
    }

    /// 箱の定義の表とサーフェス番号ごとの置き場所の表（箱の無いシェルでは空）。
    pub fn boxes(&self) -> &BoxLayout {
        &self.boxes
    }

    /// 面の表を 1 つ組む（[`EmoWorld::build_with_images`] → `bind_atlas(SetId(0))`）。
    ///
    /// スコープの数だけ呼んでよい。同じ入力から組むので、返る面の表の内容は毎回同じである。
    /// 本仕様が足す記録はここでは出さない（0 本・要件 6 の記録はすべて [`load_shell_target`]
    /// が読み込み 1 回につき 1 度だけ出す）——畳み込みと装着の既存の `warn!` は今日と同じく
    /// 組むたびに出る。
    pub fn build_world(&self) -> EmoWorld {
        let mut world = EmoWorld::build_with_images(&self.shell, &self.images);
        world.bind_atlas(&self.atlas, SetId(0));
        world
    }
}

/// シェルのフォルダを読んで [`ShellTarget`] を作る（fs を触る唯一の入口）。
///
/// 順序は「一覧 → `surfaces.txt` の読取と解析 → 面の表 → 使う画像を聞く → 焼く」である。
/// 焼くのを最後に回すのは、どの面の画像を土台に**使う**かが面の表を組んでみないと決まらない
/// ためで、[`EmoWorld`] の構築は焼いた結果を要しないので入れ替えられる。
///
/// 文字コードの扱いは今までと同じ——`surfaces.txt` 自身の `charset,<名前>` 宣言に従い、
/// 未宣言は既定（Ansi）へ後退する。
///
/// # Errors
///
/// フォルダの一覧が取れない（[`ShellLoadError::List`]・R1.7）・`surfaces.txt` が在るのに読めない
/// （[`ShellLoadError::Read`]）・面が 1 つも無い（[`ShellLoadError::Empty`]）。
/// 一覧の中の 1 件だけが取れない場合は、その 1 件を飛ばして続行する（失敗にしない）。
///
/// `surfaces.txt` が無い、または面を 1 つも定義しないときは、面の画像が 1 つ以上在れば画像だけで
/// 面を組み、そのことと面の数を `info!` で 1 行残す（spec: areka-P0-self-alpha-declaration
/// 要件 6.1・6.2・7.3）。
///
/// # 記録
///
/// 要件 6 の記録はすべて本関数が出す（読み込み 1 回につきそれぞれ 1 度だけ・モジュール冒頭
/// 「記録を出す場所」）。3 つの失敗はいずれも `error!` を伴う（記録の無い失敗経路を持たない・
/// R6.4）。
pub fn load_shell_target(
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
) -> Result<ShellTarget, ShellLoadError> {
    let names = list_file_names(shell_dir)?;
    let selection = select_surface_images(&names);

    // 重複と桁溢れは [`build_shell_target`] へ渡すと消えるので、渡す前に記録する。
    for (id, adopted, dropped) in &selection.duplicates {
        tracing::warn!(
            surface_id = *id,
            adopted = adopted.as_str(),
            dropped = ?dropped,
            "shell: 同じ番号へ解決する面の画像が複数在る（ファイル名の辞書順で最小を採用・R1.5）"
        );
    }
    for file in &selection.overflow {
        tracing::debug!(
            file = file.as_str(),
            "shell: 数字が面の番号として大きすぎるので面の画像と認めない（R1.6）"
        );
    }

    // `surfaces.txt` が無い（NotFound）ときは空の定義として続け、面の画像だけで組む（要件 6.1）。
    // 在るのに読めないときだけ今までどおりの失敗である（要件 6.6）。
    let surfaces_path = shell_dir.join(SURFACES_TXT);
    let (content, surfaces_txt) = match std::fs::read(&surfaces_path) {
        Ok(bytes) => (decode(&bytes, DefaultEncoding::Ansi), "empty"),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => (String::new(), "missing"),
        Err(source) => {
            tracing::error!(
                path = %surfaces_path.display(),
                error = %source,
                "shell: surfaces.txt の読み取りに失敗"
            );
            return Err(ShellLoadError::Read {
                path: surfaces_path,
                source,
            });
        }
    };
    let shell = areka_parsers::shell::parse(&content);
    let boxes = parse_boxes(&content);
    // 面を 1 つも定義しないときは、今の「書かれていない番号を画像から認める」枝
    // （`EmoWorld::build_with_images`）が面の画像から面を組む（要件 6.2・6.3）。画像も無ければ失敗。
    let image_only = shell.surfaces.is_empty();
    if image_only && selection.images.is_empty() {
        tracing::error!(
            shell_dir = %shell_dir.display(),
            surfaces_txt,
            "shell: シェルに面が 1 つも無い（surfaces.txt が面を定義せず、面の画像も無い）"
        );
        return Err(ShellLoadError::Empty {
            path: shell_dir.to_path_buf(),
        });
    }

    // 透過の宣言は呼ばれるたびに読む（状態を持たないので切り替えで前の宣言を持ち越さない・要件 1.7）。
    // 昔からのシェルの descript.txt は Shift_JIS が普通なので、UTF-8 として読まず ANSI を既定にする。
    let descript_path = shell_dir.join(DESCRIPT_TXT);
    let descript = match std::fs::read(&descript_path) {
        Ok(bytes) => Some(decode(&bytes, DefaultEncoding::Ansi)),
        Err(error) => {
            tracing::warn!(
                path = %descript_path.display(),
                error = %error,
                "shell: descript.txt が読めないので透過の宣言なしとして続ける"
            );
            None
        }
    };
    let use_self_alpha = crate::self_alpha::read_use_self_alpha(
        "shell",
        "seriko.use_self_alpha",
        shell_dir,
        descript.as_deref(),
    );

    let target =
        build_shell_target_with_boxes(shell, &boxes, selection, shell_dir, decoder, use_self_alpha);

    if target.ignored_pna != 0 {
        tracing::warn!(
            shell_dir = %shell_dir.display(),
            ignored_pna = target.ignored_pna,
            "shell: 絵に添えてある .pna を使わずに表示する"
        );
    }

    for (&surface_id, file) in &target.base_images.shadowed {
        tracing::debug!(
            surface_id,
            file = file.as_str(),
            "shell: element0 が在るため面の画像を土台に使わなかった（R6.2）"
        );
    }
    for &(surface_id, dangling_target) in &target.dangling {
        tracing::warn!(
            surface_id,
            target = dangling_target,
            "shell: コマの相手の面が宣言も画像も無い（描かずに続行・R3.5）"
        );
    }
    for error in &target.bake_errors {
        tracing::warn!(error = %error, "shell: shell bake で脱落した element");
    }
    for issue in &target.nest_report.issues {
        log_nest_issue(issue);
    }
    for issue in &target.box_report.issues {
        log_box_issue(issue);
    }
    if !target.boxes.is_empty() {
        tracing::info!(
            braces = boxes
                .definitions
                .iter()
                .filter(|d| matches!(d, BoxDefinition::Brace(_)))
                .count(),
            surfaces = target.box_surfaces,
            "shell: 箱を読んだ（書かれた balloon.*ブレスの数・箱の置き場所を持つサーフェスの数）"
        );
    }
    if image_only {
        tracing::info!(
            shell_dir = %shell_dir.display(),
            surfaces_txt,
            surfaces = target.images.len(),
            "shell: surfaces.txt が面を定義しないので、面の画像だけで面を組んだ（要件 7.3）"
        );
    }
    tracing::info!(
        shell_dir = %shell_dir.display(),
        recognized = target.images.len(),
        used = target.base_images.used.len(),
        shadowed = target.base_images.shadowed.len(),
        "shell: シェルの面の画像の一覧が終わった（R6.1）"
    );

    Ok(target)
}

/// fs を触らない核（復号器を除く）: 面の表を組んで使う画像を決め、焼いて [`ShellTarget`] にする。
///
/// 焼く絵の一覧は「`shell.surfaces` の複製＋**使う**画像 1 枚につき `element0` だけを持つ
/// 面 1 個」である（[`base_image_surface`]）。マニフェスト導出は渡された全 `Surface` の
/// `elements` を無条件に集めるので、`SurfaceSet`・`ManifestDeriver`・`bake` の変更は要らない。
/// 使わなかった画像は焼かない（索引表に載らない）。
///
/// 透過の扱いは引数 `use_self_alpha`（入口がシェルの descript.txt から読んだ宣言）で決まる。
pub fn build_shell_target(
    shell: Shell,
    selection: SurfaceImageSelection,
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    use_self_alpha: UseSelfAlpha,
) -> ShellTarget {
    build_shell_target_with_boxes(
        shell,
        &ShellBoxes::default(),
        selection,
        shell_dir,
        decoder,
        use_self_alpha,
    )
}

/// [`build_shell_target`] に箱の転記（[`parse_boxes`] の結果）を足した fs を触らない核。
///
/// 箱の表は [`fold_boxes`] が作り、読み捨てた事実は報告として [`ShellTarget`] に載せるだけで
/// 記録を出さない（記録は [`load_shell_target`] が読み込み 1 回につき 1 度だけ出す）。入れ子の
/// 報告（[`EmoWorld::nest_report`]）も同じく載せるだけである。焼く一覧は `shell.surfaces` の
/// 複製から画像でない element（数字だけの欄）を外したもので、その名前の絵を読みに行かない。
pub fn build_shell_target_with_boxes(
    shell: Shell,
    boxes: &ShellBoxes,
    selection: SurfaceImageSelection,
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    use_self_alpha: UseSelfAlpha,
) -> ShellTarget {
    let images = selection.images;

    // どの画像を土台に使うかは面の表を組んで初めて決まる（`element0` が在る面では使わない）。
    // ここで組む面の表は「使う画像と、相手の無いコマを聞く」ためだけのもので、装着せずに捨てる。
    // 入口が記録に使う事実もここで採る（記録のために組み直さないため）。
    let probe = EmoWorld::build_with_images(&shell, &images);
    let base_images = probe.base_images().clone();
    let dangling = probe.dangling_pattern_targets();
    let (box_layout, box_report) = fold_boxes(boxes, &images, &probe);
    let box_surfaces = probe
        .surface_ids()
        .filter(|&id| !box_layout.placements(id).is_empty())
        .count();
    let nest_report = probe.nest_report();

    // 数字だけの element定義はサーフェスを置くもので画像ではない。焼く一覧に入れると、その名前の
    // 絵を読みに行き（在れば使い・無ければ脱落の warn!）になるので、複製から外して渡す（要件 1.5・3.5）。
    let mut surfaces = shell.surfaces.clone();
    for surface in &mut surfaces {
        surface
            .elements
            .retain(|e| element_kind(&e.path) == ElementKind::Image);
    }
    surfaces.extend(
        base_images
            .used
            .iter()
            .map(|(&id, file)| base_image_surface(id, file)),
    );

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: shell_dir,
        alpha_params: AlphaParams { use_self_alpha },
    };
    let baked = bake(std::slice::from_ref(&set), decoder, PackConfig::default());

    ShellTarget {
        shell,
        images,
        atlas: baked.table,
        bake_errors: baked.errors,
        base_images,
        dangling,
        boxes: box_layout,
        box_report,
        box_surfaces,
        nest_report,
        ignored_pna: baked.ignored_pna,
    }
}

/// 入れ子の報告の 1 件を `warn!` 1 行にする（親・element・指した番号・要件 3.1・3.2・8.1）。
fn log_nest_issue(issue: &NestIssue) {
    match issue {
        NestIssue::MissingTarget {
            surface,
            element,
            target,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            target = target.as_str(),
            "shell: element定義が指したサーフェスが無いので置かない"
        ),
        NestIssue::Cycle {
            surface,
            element,
            target,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            target = *target,
            "shell: element定義の参照が循環するので、先祖へ戻る参照を置かない"
        ),
    }
}

/// 箱の報告の 1 件を `warn!` 1 行にする（欄は報告の対象の欄そのまま・要件 10.1）。
fn log_box_issue(issue: &BoxIssue) {
    match issue {
        BoxIssue::BraceEmptyName { heading } => tracing::warn!(
            heading = heading.as_str(),
            "shell: 名前が空の balloon.*ブレスを採らない"
        ),
        BoxIssue::BraceMissingSize { name } => tracing::warn!(
            name = name.as_str(),
            "shell: size の無い balloon.*ブレスを採らない"
        ),
        BoxIssue::BraceBadSize { name, value } => tracing::warn!(
            name = name.as_str(),
            value = value.as_str(),
            "shell: size が正の整数 2 つとして読めない balloon.*ブレスを採らない"
        ),
        BoxIssue::BraceNumericName { name } => tracing::warn!(
            name = name.as_str(),
            "shell: 名前が整数として読める balloon.*ブレスを採らない"
        ),
        BoxIssue::BraceReplaced { name } => tracing::warn!(
            name = name.as_str(),
            "shell: 同じ名前の balloon.*ブレスを後のもので置き換えた"
        ),
        BoxIssue::BraceKeyIgnored { name, key } => tracing::warn!(
            name = name.as_str(),
            key = key.as_str(),
            "shell: 箱に当てはまらないキーを読み捨てた"
        ),
        BoxIssue::BraceBadFollow { name, value } => tracing::warn!(
            name = name.as_str(),
            value = value.as_str(),
            "shell: font.follow の値が不正（scope として扱う）"
        ),
        BoxIssue::ElementBadNumber { surface, element } => tracing::warn!(
            surface = *surface,
            element = element.as_str(),
            "shell: element番号が読めない箱の element定義を読み捨てた"
        ),
        BoxIssue::ElementUnknownBrace {
            surface,
            element,
            name,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            name = name.as_str(),
            "shell: 名前の balloon.*ブレスが無い箱の element定義を読み捨てた"
        ),
        BoxIssue::ElementBadPosition {
            surface,
            element,
            name,
            x,
            y,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            name = name.as_str(),
            x = x.as_str(),
            y = y.as_str(),
            "shell: X・Y が整数として読めない箱の element定義を読み捨てた"
        ),
        BoxIssue::ElementDuplicateName {
            surface,
            element,
            name,
            kept,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            name = name.as_str(),
            kept = *kept,
            "shell: 同じ名前の箱の element定義のうち element番号が最小でないものを読み捨てた"
        ),
        BoxIssue::ElementBelowImage {
            surface,
            element,
            name,
            image_element,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            name = name.as_str(),
            image_element = *image_element,
            "shell: 箱の element番号が画像の element番号より小さい（箱は画像より手前に描く）"
        ),
        BoxIssue::AppendTargetMissing { surface, name } => tracing::warn!(
            surface = *surface,
            name = name.as_str(),
            "shell: surface.append*ブレスの追記先のサーフェスが無いので箱の element定義を読み捨てた"
        ),
        BoxIssue::InChildSurface {
            parent,
            child,
            name,
        } => tracing::warn!(
            parent = *parent,
            child = *child,
            name = name.as_str(),
            "shell: 子として置かれたサーフェスの箱は親の中に置かない"
        ),
    }
}

/// シェルのフォルダ**直下**の**ファイル**の名前を 1 回の走査で集める（R1.3）。
///
/// フォルダそのものとサブフォルダの中身は 0 件である（`file_type().is_file()`）。非 UTF-8 の
/// 名前は面の画像の名前の規約（ASCII の接頭辞＋数字＋拡張子）を満たし得ないのでここで落とす。
/// 1 件ごとの取得失敗（種別が読めない場合を含む）は致命ではなく、その 1 件を飛ばして続ける。
///
/// バルーンの `enumerate_file_names` を流用しないのは、失敗の型がバルーン専用
/// （`PresentError`）で、フォルダを除かないためである。
fn list_file_names(shell_dir: &Path) -> Result<Vec<String>, ShellLoadError> {
    let read_dir = std::fs::read_dir(shell_dir).map_err(|source| {
        tracing::error!(
            shell_dir = %shell_dir.display(),
            error = %source,
            "shell: シェルのフォルダの一覧に失敗（画像 0 件として先へ進まない・R1.7）"
        );
        ShellLoadError::List {
            path: shell_dir.to_path_buf(),
            source,
        }
    })?;

    let mut names: Vec<String> = Vec::new();
    for entry in read_dir {
        // 1 件の取得失敗・種別の読み取り失敗は致命ではない（その 1 件を飛ばして続行・
        // バルーンの `enumerate_file_names` と同じ扱い）。
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(
                    shell_dir = %shell_dir.display(),
                    error = %error,
                    "shell: フォルダの 1 件の取得に失敗（その 1 件を飛ばして続行）"
                );
                continue;
            }
        };
        match entry.file_type() {
            Ok(file_type) if file_type.is_file() => {}
            // フォルダ（とその他の種別）は面の画像と認めない＝失敗ではないので記録しない。
            Ok(_) => continue,
            Err(error) => {
                tracing::warn!(
                    file = %entry.file_name().to_string_lossy(),
                    error = %error,
                    "shell: フォルダの 1 件の種別の読み取りに失敗（その 1 件を飛ばして続行）"
                );
                continue;
            }
        }
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// 面の画像 1 枚だけを持つ合成 `Surface`（焼く絵の一覧へ足す入力・R2.2）。
///
/// パスの綴りは採ったファイル名そのままで、面の表の層 0 の [`ElementPath`] と完全一致する。
/// `AtlasTable::resolve` は文字列の完全一致で引くので、ここがずれると絵は記録も無く描かれない。
fn base_image_surface(id: u32, file: &str) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new(file.to_string()),
            x: 0,
            y: 0,
        }],
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

#[cfg(test)]
#[path = "shell_target_test_support.rs"]
pub(crate) mod test_support;

#[cfg(test)]
#[path = "shell_target_names_tests.rs"]
mod names_tests;

#[cfg(test)]
#[path = "shell_target_load_tests.rs"]
mod load_tests;

#[cfg(test)]
#[path = "shell_target_base_image_tests.rs"]
mod base_image_tests;

#[cfg(test)]
#[path = "shell_target_template_tests.rs"]
mod template_tests;

#[cfg(test)]
#[path = "shell_target_emo2_tests.rs"]
mod emo2_tests;

#[cfg(test)]
#[path = "shell_target_boxes_tests.rs"]
mod boxes_tests;

#[cfg(test)]
#[path = "shell_target_nesting_tests.rs"]
mod nesting_tests;

#[cfg(test)]
#[path = "shell_target_image_only_tests.rs"]
mod image_only_tests;
