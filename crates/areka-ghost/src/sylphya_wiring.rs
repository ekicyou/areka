//! `areka-ghost` ⓪ghost の sylphya 結線層（task 8.1〜8.2）。
//!
//! ここでは descript 由来の名前情報（`GhostNames`）から sylphya のフラット静的値
//! （`selfname`／`selfname2`／`keroname`）を導出する純関数 [`derive_flat_statics`] と、
//! boot 系列が sylphya（統一プロパティシステム）を起動・結線するための薄い配線ヘルパ群
//! （アクター spawn・静的構成/baseware publish・prefetch sink クロージャ・provider 生成）を
//! 提供する（task 8.2・design「ghost（結線・provider 差替）」）。
//!
//! # 決定論檻（要件 9.4）
//! [`derive_flat_statics`] は純関数（I/O・時計・乱数を持たない）であり、同一の `GhostNames`
//! からは常に同一順序・同一内容の `Vec` を返す。descript 実値解決の全判断分岐
//! （keroname の 3 分岐・selfname2 の有無・selfname の有無）を x64 純粋単体テストで檻に入れる。
//! 配線ヘルパ（spawn／publish／sink／provider）は判断分岐を持たない薄い結線であり、
//! その振る舞いは boot 統合テスト（task 8.4）が檻に入れる（記憶知見「配線は再テストしない」）。

use std::path::{Path, PathBuf};

use areka_kanade::resources::{ResourceOutcome, ResourceSink};
use areka_parsers::package::GhostNames;
use areka_sakura::contract::SystemVarSnapshot;
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{
    spawn_sylphya, AskerContext, AskerId, ScopeRoots, SylphyaInit, SylphyaParts, SylphyaPublisher,
    SylphyaReader,
};

use crate::runtime::SystemVarSource;

/// baseware 識別名（大域点付き `baseware.name`・R5.1・実値）。
const BASEWARE_NAME: &str = "areka";

/// ログ target（steering: areka-log-first-no-silent-failure）。
const LOG_TARGET: &str = "ghost-boot";

/// descript の名前情報 → sylphya フラット静的値（純関数・決定論檻対象・R9.4）。
///
/// 生成規則（要件 4.3／4.4／4.5・design「ghost（結線・provider 差替）」・
/// `doc/COMPAT_ARCHITECTURE.md` §8 対応表 ②③）:
///
/// - `sakura.name` が `Some(v)` → `("selfname", v)` を積む（R4.3）。未定義なら積まない（素通し縮退）。
/// - `sakura.name2` が `Some(v)` → `("selfname2", v)` を積む（R4.4）。未定義なら**何も積まない**
///   （素通し縮退・フォールバック創作なし・既定値なし）。
/// - keroname（R4.5・SSP 互換）:
///   - `kero.name` が `Some(v)` → `("keroname", v)` を積む。
///   - `kero.name` が `None` かつ `sakura.name` が `Some(v)` → `("keroname", v)` を積む
///     （SSP 互換フォールバック＝本体側の名前）。
///   - 両方 `None` → 何も積まない（素通し縮退）。
///
/// フラットトークン名は sylphya のフラット語彙に合わせ `%` を含まない
/// （`"selfname"`／`"selfname2"`／`"keroname"`）。返す `Vec` は決定論的な安定順
/// （selfname → selfname2 → keroname）で、これらは task 8.2 で ghost が `PublishStatic` する。
pub fn derive_flat_statics(names: &GhostNames) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();

    // %selfname＝sakura.name（R4.3）。未定義→積まない（素通し縮退）。
    if let Some(v) = &names.sakura_name {
        out.push(("selfname".to_string(), v.clone()));
    }

    // %selfname2＝sakura.name2（R4.4）。未定義→積まない（素通し縮退・対応表 ②）。
    if let Some(v) = &names.sakura_name2 {
        out.push(("selfname2".to_string(), v.clone()));
    }

    // %keroname＝kero.name。未定義なら sakura.name へフォールバック（SSP 互換・R4.5・対応表 ③）。
    // 両者未定義なら積まない（素通し縮退）。
    if let Some(v) = &names.kero_name {
        out.push(("keroname".to_string(), v.clone()));
    } else if let Some(v) = &names.sakura_name {
        out.push(("keroname".to_string(), v.clone()));
    }

    out
}

// ============================================================================
// task 8.2: sylphya 起動・結線の薄い配線ヘルパ（判断分岐なし・boot 図の結線正本）
// ============================================================================

/// スコープ root の profile フォルダ（`<base>/profile/areka/`）を組む（R6.5・層別 profile 物理分離）。
///
/// sylphya はパスを解釈しない最下層規律ゆえ、所属実体（ghost/shell）の分離は結線側の
/// per-実体 profile ディレクトリの物理分離が担う。ghost スコープは `<MountModel.shiori.dir>`、
/// shell スコープは `<ShellMount.dir>` を base に取る（design「boot 系列」）。
pub fn profile_areka_root(base: &Path) -> PathBuf {
    base.join("profile").join("areka")
}

/// ghost 自身の `AskerId` を `MountModel.shiori.dir` の正準文字列から構築する（設計「ghost 自身の
/// AskerId は MountModel.shiori.dir の正準文字列から構築」）。
///
/// フラット静的値（selfname 等）・SHIORI 照会値（username）の per-asker 着地先であり、
/// provider／prefetch sink が同一 asker を共有する（同一ゴーストの読み書きが一致する）。
pub fn ghost_asker_id(shiori_dir: &Path) -> AskerId {
    AskerId::new(shiori_dir.to_string_lossy().into_owned())
}

/// boot 系列の sylphya アクターを起動する（本番 IO＝[`FsPersistIo`]・運行 sink 未配線＝M1）。
///
/// `roots` は層別 profile root（ghost/shell/app/balloon）。起動時に全スコープを寛容ロードして
/// 初期鏡像へ投影する（不在スコープは空扱い・emo2 fixture の read-only でも汚染しない——M1 本番
/// 経路に永続書込呼出は無い・design Implementation Notes）。返る [`SylphyaParts`] の reader は
/// 結線直後から無待機で読める。
pub fn spawn_ghost_sylphya(roots: ScopeRoots) -> SylphyaParts {
    spawn_sylphya(SylphyaInit {
        roots,
        io: Box::new(FsPersistIo),
        runtime_sink: None,
    })
}

/// 静的構成層を publish する（フラット＝`derive_flat_statics`・大域点付き＝baseware 2 項・R4.3-4.5/R5.1）。
///
/// フラット（selfname/selfname2/keroname）は `asker` の per-asker 区画へ、baseware（`baseware.name`
/// ＝実値 `"areka"`／`baseware.version`＝`baseware_version`）は大域点付き区画へ着地する
/// （design「boot 系列」・sylphya `PublishStatic` の flat/dotted 区分に対応）。投函のみで反映は
/// アクターが担う——呼び出し側は必要に応じ `barrier()` で反映完了を待つ。
pub fn publish_ghost_statics(
    publisher: &SylphyaPublisher,
    asker: AskerId,
    names: &GhostNames,
    baseware_version: &str,
) {
    let flat = derive_flat_statics(names);
    let dotted = vec![
        ("baseware.name".to_string(), BASEWARE_NAME.to_string()),
        ("baseware.version".to_string(), baseware_version.to_string()),
    ];
    publisher.publish_static(asker, flat, dotted);
}

/// kanade へ注入する実 [`ResourceSink`]（publish_shiori＋barrier のクロージャ・R4.1/R4.2）。
///
/// prefetch 段（kanade boot 系列）が `(id, outcome)` を **同期的に** 呼ぶ。本 sink は:
/// - [`ResourceOutcome::Value(v)`] → `publish_shiori(asker, id, Some(v))`
/// - [`ResourceOutcome::NoContent`]／[`ResourceOutcome::Failed`] → `publish_shiori(asker, id, None)`
///   （不在の観測記録・既定値は書かない＝sakura の唯一定義点に残置・R4.2）
///
/// を投函した **後** `barrier()` で反映完了を待ってから返る。これにより prefetch→初回 talk までの
/// 反映順序が決定論化する（研究 §12-1）。barrier が [`areka_actor::ReplyError`] を返す
/// （sylphya アクター死亡）場合は **warn＋続行**（永久ブロックしない・panic しない・design Risks）。
pub fn make_username_resource_sink(publisher: SylphyaPublisher, asker: AskerId) -> ResourceSink {
    Box::new(move |id, outcome| {
        let value = match outcome {
            ResourceOutcome::Value(v) => Some(v),
            // 204／失敗は不在記録（既定値縮退は消費側 sakura の責務・R4.2）。
            ResourceOutcome::NoContent | ResourceOutcome::Failed(_) => None,
        };
        publisher.publish_shiori(asker.clone(), id.to_string(), value);
        // 反映フェンス: 投函済み publish の反映完了を同期観測してから返る（初回 talk 前の順序保証）。
        if let Err(err) = publisher.barrier() {
            tracing::warn!(
                target: LOG_TARGET,
                id = id,
                error = %err,
                "sylphya barrier failed after publishing shiori resource \
                 (actor stopped); continuing without reflection guarantee"
            );
        }
    })
}

/// `FromSylphya` provider を構築する（reader＋自 asker を捕捉・`talk_snapshot` → [`SystemVarSnapshot`]・R7.1）。
///
/// dispatcher が talk 起動ごとに一度呼び出す（凍結像の刻印点）。呼び出し時点の鏡像から
/// [`SylphyaReader::talk_snapshot`]（値実在フラット名のみの `BTreeMap`）を取り、各名→値を
/// sakura 所有の [`SystemVarSnapshot`] へ insert 写像して返す。sakura の契約（`SystemVarSnapshot`・
/// 値源優先・既定値唯一定義点）は無改変で、変わるのは **スナップショットの源**（sylphya 鏡像）だけ
/// （R7.1/R2.2・provider 差替の核）。
pub fn from_sylphya_provider(reader: SylphyaReader, asker: AskerId) -> SystemVarSource {
    let ctx = AskerContext { asker };
    Box::new(move || {
        let raw = reader.talk_snapshot(&ctx);
        // provider 差替の証跡（design Monitoring 固定ログ・R9.3 サインオフ用）。
        // talk 起動ごとに 1 回、スナップショットの源が sylphya 読み口であることを記録する
        // （固定 target/メッセージは Revalidation Trigger——変更時は design Monitoring を更新）。
        tracing::debug!(
            target: "areka_ghost",
            asker = ctx.asker.as_str(),
            count = raw.len(),
            "talk snapshot from sylphya reader"
        );
        let mut snapshot = SystemVarSnapshot::default();
        for (name, value) in raw {
            snapshot.insert(name, value);
        }
        snapshot
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `GhostNames` は `#[non_exhaustive]` ゆえ他クレートから構造体リテラル構築不可。
    /// `Default` を起点にフィールドをミューテートして組み立てる（決定論檻の入力構築）。
    fn names(
        sakura_name: Option<&str>,
        sakura_name2: Option<&str>,
        kero_name: Option<&str>,
    ) -> GhostNames {
        let mut n = GhostNames::default();
        n.sakura_name = sakura_name.map(|s| s.to_string());
        n.sakura_name2 = sakura_name2.map(|s| s.to_string());
        n.kero_name = kero_name.map(|s| s.to_string());
        n
    }

    // --- selfname（R4.3） ---

    /// sakura.name あり → `selfname` エントリが実値で積まれる（R4.3）。
    #[test]
    fn selfname_present_when_sakura_name_defined() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, None));
        assert!(
            got.contains(&("selfname".to_string(), "むらさき".to_string())),
            "sakura.name 定義時は selfname が実値で積まれるべき: {got:?}"
        );
    }

    /// sakura.name 未定義 → `selfname` エントリを積まない（素通し縮退・R4.3）。
    #[test]
    fn selfname_absent_when_sakura_name_undefined() {
        let got = derive_flat_statics(&names(None, None, None));
        assert!(
            !got.iter().any(|(k, _)| k == "selfname"),
            "sakura.name 未定義時は selfname を積まない（素通し）: {got:?}"
        );
    }

    // --- selfname2（R4.4・対応表 ②） ---

    /// sakura.name2 あり → `selfname2` エントリが実値で積まれる（R4.4）。
    #[test]
    fn selfname2_present_when_name2_defined() {
        let got = derive_flat_statics(&names(Some("むらさき"), Some("紫"), None));
        assert!(
            got.contains(&("selfname2".to_string(), "紫".to_string())),
            "sakura.name2 定義時は selfname2 が実値で積まれるべき: {got:?}"
        );
    }

    /// sakura.name2 未定義 → `selfname2` エントリを積まない（素通し縮退・フォールバック創作なし・R4.4）。
    #[test]
    fn selfname2_absent_when_name2_undefined() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, None));
        assert!(
            !got.iter().any(|(k, _)| k == "selfname2"),
            "sakura.name2 未定義時は selfname2 を積まない（素通し・既定値やフォールバックを創作しない）: {got:?}"
        );
    }

    // --- keroname 3 分岐（R4.5・対応表 ③） ---

    /// (a) kero.name あり → `keroname` = kero.name（R4.5）。
    #[test]
    fn keroname_from_kero_name_when_defined() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, Some("エモ")));
        assert!(
            got.contains(&("keroname".to_string(), "エモ".to_string())),
            "kero.name 定義時は keroname = kero.name: {got:?}"
        );
    }

    /// (b) kero.name 未定義＋sakura.name あり → `keroname` = sakura.name（SSP 互換フォールバック・R4.5）。
    #[test]
    fn keroname_falls_back_to_sakura_name_when_kero_undefined() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, None));
        assert!(
            got.contains(&("keroname".to_string(), "むらさき".to_string())),
            "kero.name 未定義＋sakura.name 定義時は keroname が sakura.name へフォールバック: {got:?}"
        );
    }

    /// (c) kero.name・sakura.name 両方未定義 → `keroname` エントリを積まない（素通し縮退・R4.5）。
    #[test]
    fn keroname_absent_when_both_undefined() {
        let got = derive_flat_statics(&names(None, None, None));
        assert!(
            !got.iter().any(|(k, _)| k == "keroname"),
            "kero.name・sakura.name 両方未定義時は keroname を積まない（素通し）: {got:?}"
        );
    }

    /// kero.name が定義されていれば sakura.name の有無に依らず kero.name が勝つ（フォールバック非適用）。
    #[test]
    fn keroname_prefers_kero_name_over_sakura_fallback() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, Some("エモ")));
        assert!(
            got.contains(&("keroname".to_string(), "エモ".to_string()))
                && !got.contains(&("keroname".to_string(), "むらさき".to_string())),
            "kero.name 定義時は sakura.name へフォールバックしない: {got:?}"
        );
    }

    // --- 全語彙同時・順序・決定論 ---

    /// 全て定義 → selfname／selfname2／keroname がこの安定順で並ぶ（決定論の順序契約）。
    #[test]
    fn full_names_produce_stable_order() {
        let got = derive_flat_statics(&names(Some("むらさき"), Some("紫"), Some("エモ")));
        assert_eq!(
            got,
            vec![
                ("selfname".to_string(), "むらさき".to_string()),
                ("selfname2".to_string(), "紫".to_string()),
                ("keroname".to_string(), "エモ".to_string()),
            ],
            "安定順（selfname → selfname2 → keroname）で並ぶべき"
        );
    }

    /// 同一 `GhostNames` からは常に同一の `Vec`（順序・内容）を返す（決定論・R9.4/R2.5）。
    #[test]
    fn deterministic_same_input_same_output() {
        let n = names(Some("むらさき"), None, None);
        let a = derive_flat_statics(&n);
        let b = derive_flat_statics(&n);
        assert_eq!(a, b, "同一入力は同一出力（決定論）");
        // フォールバック経路でも selfname と keroname の両方が sakura.name 由来で並ぶ。
        assert_eq!(
            a,
            vec![
                ("selfname".to_string(), "むらさき".to_string()),
                ("keroname".to_string(), "むらさき".to_string()),
            ]
        );
    }

    // --- provider 差替の固定ログ（design Monitoring・R9.3 サインオフ証跡・Task 10.1） ---

    /// `from_sylphya_provider` が生成するスナップショットのたびに、design Monitoring の固定ログ
    /// `debug!(target: "areka_ghost", "talk snapshot from sylphya reader")` が**必ず 1 回**発火する
    /// （provider の源が sylphya 読み口であることの R9.3 grep 証跡）。ログが削除・target/メッセージ
    /// 変更・レベル変更されると本檻が落ちる（固定ログの回帰檻）。
    ///
    /// interest-keeper 経由の [`crate::test_log_capture::capture`] で捕捉し、並列 `cargo test` 負荷下
    /// でも `Interest::never` 焼き付きに影響されず決定論的に判定する（bare `with_default` 不使用）。
    #[test]
    fn provider_snapshot_emits_fixed_debug_log() {
        use crate::test_log_capture::{assert_logged, capture};

        // 空 roots（全 None）で sylphya を起動——FsPersistIo だが root 不在で FS を一切触らない
        // （load_scope は root 不在→空縮退・決定論）。selfname を publish して asker 区画へ着地させる。
        let parts = spawn_ghost_sylphya(ScopeRoots::default());
        let asker = AskerId::new("ghost/provider-log-cage");
        parts.publisher.publish_static(
            asker.clone(),
            vec![("selfname".into(), "さくら".into())],
            vec![],
        );
        parts.publisher.barrier().expect("barrier while actor alive");

        let provider = from_sylphya_provider(parts.reader.clone(), asker);

        // provider をテストスレッド上で駆動——debug! は同一スレッドで発火し capture が確実に捕える。
        let mut snapshot = None;
        let events = capture(|| {
            snapshot = Some(provider());
        });

        // 固定ログが規約どおり（target="areka_ghost"・DEBUG・固定メッセージ）で発火している。
        assert_logged(
            &events,
            tracing::Level::DEBUG,
            "areka_ghost",
            "talk snapshot from sylphya reader",
        );
        // スナップショットの源が sylphya 読み口であること（publish した値が provider 像に載る）。
        assert_eq!(
            snapshot.expect("provider produced a snapshot").get("selfname"),
            Some("さくら"),
            "provider 像は sylphya 鏡像由来（publish した selfname が載る）"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins without panic");
    }
}
