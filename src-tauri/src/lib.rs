use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};
use zip::ZipArchive;

const PATCH_EVENT_NAME: &str = "patch://event";
const DEFAULT_PATCH_TARGET: &str = "INT_STEAM";
const INT_STEAM_TARGET: &str = "INT_STEAM";
const CN_BILIBILI_TARGET: &str = "CN_BILIBILI";
const PATCH_AUTO_EXIT_DELAY_SECS: u64 = 10;
const RESUME_INSTALL_ARG: &str = "--resume-install";
const RESUME_PATCH_ARG: &str = "--resume-patch";
const SELF_UPDATE_RELEASE_API: &str =
    "https://api.github.com/repos/maynut02/AstralAutoPatcher/releases/latest";
const PATCH_BUNDLE_RELEASE_API: &str =
    "https://api.github.com/repos/maynut02/astralparty-korean-patch/releases/latest";
const INT_STEAM_RULES_API: &str =
    "https://raw.githubusercontent.com/maynut02/astralparty-korean-patch/refs/heads/main/src/astral_patch/patch/rules/int_steam.json";
const CN_BILIBILI_RULES_API: &str =
    "https://raw.githubusercontent.com/maynut02/astralparty-korean-patch/refs/heads/main/src/astral_patch/patch/rules/cn_bilibili.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchContext {
    mode: String,
    target: String,
    protocol_arg: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchEvent {
    step_index: usize,
    title: String,
    state: String,
    message: String,
    detail: Option<String>,
    progress: u8,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct PatchRuleDocument {
    tasks: Vec<PatchRuleTask>,
}

#[derive(Debug, Deserialize)]
struct PatchRuleTask {
    source: Option<String>,
    root_id: Option<String>,
}

struct PreparedPatch {
    release_tag: String,
    asset_name: String,
    extract_dir: PathBuf,
}

struct SelfUpdatePlan {
    latest_version: String,
    exe_path: PathBuf,
}

fn resolve_log_file_path() -> PathBuf {
    if let Ok(install_dir) = resolve_install_dir() {
        return install_dir.join("logs").join("runtime.log");
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            return parent.join("logs").join("runtime.log");
        }
    }

    resolve_install_dir()
        .unwrap_or_else(|_| env::temp_dir().join("AstralAutoPatcher"))
        .join("logs")
        .join("runtime.log")
}

fn append_log_line(level: &str, message: &str) -> Result<()> {
    let log_file_path = resolve_log_file_path();
    if let Some(parent) = log_file_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "로그 디렉터리 생성에 실패했습니다.\n{}",
                parent.display()
            )
        })?;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp = format!("{}.{:03}", now.as_secs(), now.subsec_millis());

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .with_context(|| {
            format!(
                "로그 파일 열기에 실패했습니다.\n{}",
                log_file_path.display()
            )
        })?;

    writeln!(file, "[{timestamp}] [{level}] {message}")
        .context("로그 파일 기록에 실패했습니다.")?;

    Ok(())
}

fn log_info(message: impl AsRef<str>) {
    if let Err(error) = append_log_line("INFO", message.as_ref()) {
        eprintln!("[INFO] 파일 로그 기록 오류: {error:#}");
    }
}

fn log_error(message: impl AsRef<str>) {
    if let Err(error) = append_log_line("ERROR", message.as_ref()) {
        eprintln!("[ERROR] 파일 로그 기록 오류: {error:#}");
    }
}

#[tauri::command]
fn get_launch_context() -> Result<LaunchContext, String> {
    parse_launch_context().map_err(|error| format!("{error:#}"))
}

#[tauri::command]
fn start_patch_workflow(app: AppHandle, target: Option<String>) -> Result<(), String> {
    let resolved_target =
        resolve_patch_target(target.as_deref()).map_err(|error| format!("{error:#}"))?;

    log_info(format!(
        "패치 워크플로우 시작 요청: {resolved_target}"
    ));

    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = run_patch_workflow(&app, &resolved_target) {
            eprintln!("패치 워크플로우 오류: {error:#}");
            log_error(format!("패치 워크플로우 오류: {error:#}"));
        }
    });

    Ok(())
}

#[tauri::command]
fn start_remove_workflow(app: AppHandle, target: Option<String>) -> Result<(), String> {
    let resolved_target =
        resolve_patch_target(target.as_deref()).map_err(|error| format!("{error:#}"))?;

    log_info(format!(
        "제거 워크플로우 시작 요청: {resolved_target}"
    ));

    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = run_remove_workflow(&app, &resolved_target) {
            eprintln!("제거 워크플로우 오류: {error:#}");
            log_error(format!("제거 워크플로우 오류: {error:#}"));
        }
    });

    Ok(())
}

#[tauri::command]
fn start_install_workflow(app: AppHandle) -> Result<(), String> {
    log_info("설치 워크플로우 시작 요청.");

    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = run_install_workflow(&app) {
            eprintln!("설치 워크플로우 오류: {error:#}");
            log_error(format!("설치 워크플로우 오류: {error:#}"));
        }
    });

    Ok(())
}

fn run_install_workflow(app: &AppHandle) -> Result<()> {
    emit_patch_event(
        app,
        1,
        "프로그램 버전 확인",
        "current",
        "프로그램 버전을 확인하는 중...",
        None,
        0,
    )?;

    if let Some(update_plan) = prepare_self_update_if_needed()? {
        emit_patch_event(
            app,
            1,
            "프로그램 버전 확인",
            "done",
            "신규 버전 확인. 업데이트를 시작합니다.",
            Some(format!(
                "v{} -> v{}\n프로그램을 신규 버전으로 재시작합니다.",
                env!("CARGO_PKG_VERSION"),
                update_plan.latest_version
            )),
            20,
        )?;
        relaunch_for_resume(app, &update_plan.exe_path, None)?;
        return Ok(());
    }

    emit_patch_event(
        app,
        1,
        "프로그램 버전 확인",
        "done",
        "프로그램 버전 확인 완료",
        Some(format!("v{}", env!("CARGO_PKG_VERSION"))),
        20,
    )?;

    emit_patch_event(
        app,
        2,
        "레거시 데이터 정리",
        "current",
        "구버전 프로그램을 정리하는 중...",
        None,
        20,
    )?;

    let legacy_cleanup_detail = cleanup_legacy_v1_artifacts();

    emit_patch_event(
        app,
        2,
        "레거시 데이터 정리",
        "done",
        "구버전 프로그램 정리 완료",
        Some(legacy_cleanup_detail),
        35,
    )?;

    emit_patch_event(
        app,
        3,
        "프로그램 설치",
        "current",
        "프로그램을 설치하는 중...",
        None,
        35,
    )?;

    let safe_exe_path = match relocate_runtime_to_safe_path() {
        Ok((exe_path, detail)) => {
            emit_patch_event(
                app,
                3,
                "프로그램 설치",
                "done",
                "프로그램 설치 완료",
                Some(detail),
                70,
            )?;
            exe_path
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                3,
                "프로그램 설치",
                "error",
                "프로그램 설치 실패",
                Some(message.clone()),
                35,
            )?;
            return Err(anyhow!(message));
        }
    };

    emit_patch_event(
        app,
        4,
        "연결 프로토콜 등록",
        "current",
        "연결 프로토콜을 등록하는 중...",
        None,
        70,
    )?;

    match register_astral_protocol(&safe_exe_path) {
        Ok(detail) => {
            emit_patch_event(
                app,
                4,
                "연결 프로토콜 등록",
                "done",
                "연결 프로토콜 등록 완료",
                Some(detail),
                90,
            )?;
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                4,
                "연결 프로토콜 등록",
                "error",
                "연결 프로토콜 등록 실패",
                Some(message.clone()),
                70,
            )?;
            return Err(anyhow!(message));
        }
    }

    emit_patch_event(
        app,
        5,
        "결과",
        "current",
        "마무리 작업을 진행하는 중...",
        None,
        90,
    )?;
    emit_patch_event(
        app,
        5,
        "결과",
        "done",
        "프로그램 실행 준비 완료",
        Some("astral.maynutlab.com에 접속해 자동 패치를 진행할 수 있습니다.".to_string()),
        100,
    )?;

    Ok(())
}

fn run_patch_workflow(app: &AppHandle, target: &str) -> Result<()> {
    emit_patch_event(
        app,
        1,
        "프로그램 버전 확인",
        "current",
        "프로그램 버전을 확인하는 중...",
        None,
        0,
    )?;

    if let Some(update_plan) = prepare_self_update_if_needed()? {
        emit_patch_event(
            app,
            1,
            "프로그램 버전 확인",
            "done",
            "신규 버전을 확인해 업데이트를 시작합니다.",
            Some(format!(
                "v{} -> v{}\n새 버전 실행 후 패치 작업({})을 이어갑니다.",
                env!("CARGO_PKG_VERSION"),
                update_plan.latest_version,
                target
            )),
            10,
        )?;
        relaunch_for_resume(app, &update_plan.exe_path, Some(target))?;
        return Ok(());
    }

    emit_patch_event(
        app,
        1,
        "프로그램 버전 확인",
        "done",
        "프로그램 버전 확인 완료",
        Some(format!("v{}", env!("CARGO_PKG_VERSION"))),
        10,
    )?;

    emit_patch_event(
        app,
        2,
        "게임 설치 경로 확인",
        "current",
        "게임 설치 경로를 확인하는 중...",
        None,
        10,
    )?;

    let game_dir = match find_game_install_dir(target) {
        Ok(found) => {
            emit_patch_event(
                app,
                2,
                "게임 설치 경로 확인",
                "done",
                "게임 설치 경로 확인 완료",
                Some(found.display().to_string()),
                30,
            )?;
            found
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                2,
                "게임 설치 경로 확인",
                "error",
                "게임 설치 경로 확인 실패",
                Some(message.clone()),
                10,
            )?;
            return Err(anyhow!(message));
        }
    };

    emit_patch_event(
        app,
        3,
        "로컬 다운로드 경로 확인",
        "current",
        "로컬 다운로드 경로를 확인하는 중...",
        None,
        30,
    )?;
    let local_feimo_dir = match find_local_feimo_path(target) {
        Ok(found) => {
            emit_patch_event(
                app,
                3,
                "로컬 다운로드 경로 확인",
                "done",
                "로컬 다운로드 경로 확인 완료",
                Some(found.display().to_string()),
                45,
            )?;
            found
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                3,
                "로컬 다운로드 경로 확인",
                "error",
                "로컬 다운로드 경로 확인 실패",
                Some(message.clone()),
                30,
            )?;
            return Err(anyhow!(message));
        }
    };

    emit_patch_event(
        app,
        4,
        "한글 패치 파일 준비",
        "current",
        "한글 패치 파일 다운로드 중...",
        None,
        45,
    )?;
    let prepared_patch = match prepare_patch_bundle(target) {
        Ok(prepared) => {
            let mut detail = format!("[버전] {}\n[파일] {}", prepared.release_tag, prepared.asset_name);
            if prepared.release_tag.ends_with("-pre") {
                detail.push_str(
                    "\n⚠ Pre-release 버전에서는 일부 번역되지 않은 요소가 존재할 수 있습니다.",
                );
            }
            emit_patch_event(
                app,
                4,
                "한글 패치 파일 준비",
                "done",
                "한글 패치 파일 준비 완료",
                Some(detail),
                70,
            )?;
            prepared
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                4,
                "한글 패치 파일 준비",
                "error",
                "한글 패치 파일 준비 실패",
                Some(message.clone()),
                45,
            )?;
            return Err(anyhow!(message));
        }
    };

    emit_patch_event(
        app,
        5,
        "한글 패치 적용",
        "current",
        "한글 패치를 적용하는 중...",
        None,
        70,
    )?;
    match apply_patch_bundle(target, &prepared_patch.extract_dir, &game_dir, &local_feimo_dir) {
        Ok(detail) => {
            emit_patch_event(
                app,
                5,
                "한글 패치 적용",
                "done",
                "한글 패치 적용 완료",
                Some(detail),
                95,
            )?;
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                5,
                "한글 패치 적용",
                "error",
                "한글 패치 적용 실패",
                Some(message.clone()),
                70,
            )?;
            return Err(anyhow!(message));
        }
    }

    emit_patch_event(
        app,
        6,
        "결과",
        "current",
        "마무리 작업을 진행하는 중...",
        None,
        95,
    )?;

    for remaining_secs in (1..=PATCH_AUTO_EXIT_DELAY_SECS).rev() {
        emit_patch_event(
            app,
            6,
            "결과",
            "done",
            "작업 완료",
            Some(format!(
                "{}초 후 프로그램이 자동으로 종료됩니다.",
                remaining_secs
            )),
            100,
        )?;

        std::thread::sleep(Duration::from_secs(1));
    }

    log_info(format!(
        "패치 작업이 완료되어 {}초 후 프로그램을 자동 종료합니다.",
        PATCH_AUTO_EXIT_DELAY_SECS
    ));
    app.exit(0);

    Ok(())
}

fn run_remove_workflow(app: &AppHandle, target: &str) -> Result<()> {
    emit_patch_event(
        app,
        1,
        "로컬 다운로드 경로 확인",
        "current",
        "로컬 다운로드 경로를 확인하는 중...",
        None,
        0,
    )?;

    let local_feimo_dir = match find_local_feimo_path(target) {
        Ok(found) => {
            emit_patch_event(
                app,
                1,
                "로컬 다운로드 경로 확인",
                "done",
                "로컬 다운로드 경로 확인 완료",
                Some(found.display().to_string()),
                30,
            )?;
            found
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                1,
                "로컬 다운로드 경로 확인",
                "error",
                "로컬 다운로드 경로 확인 실패",
                Some(message.clone()),
                0,
            )?;
            return Err(anyhow!(message));
        }
    };

    emit_patch_event(
        app,
        2,
        "제거 규칙 조회",
        "current",
        "제거 대상 규칙을 조회하는 중...",
        None,
        30,
    )?;

    let root_ids = match fetch_remove_root_ids(target) {
        Ok(ids) => {
            emit_patch_event(
                app,
                2,
                "제거 규칙 조회",
                "done",
                "제거 규칙 조회 완료",
                Some(format!("AssetBundles 대상 root_id {}개", ids.len())),
                55,
            )?;
            ids
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                2,
                "제거 규칙 조회",
                "error",
                "제거 규칙 조회 실패",
                Some(message.clone()),
                30,
            )?;
            return Err(anyhow!(message));
        }
    };

    emit_patch_event(
        app,
        3,
        "한글 패치 제거",
        "current",
        "설치된 한글 패치 데이터를 제거하는 중...",
        None,
        55,
    )?;

    let removal_detail = match remove_assetbundle_roots(&local_feimo_dir, &root_ids) {
        Ok(detail) => {
            emit_patch_event(
                app,
                3,
                "한글 패치 제거",
                "done",
                "한글 패치 제거 완료",
                Some(detail.clone()),
                90,
            )?;
            detail
        }
        Err(error) => {
            let message = format!("{error:#}");
            emit_patch_event(
                app,
                3,
                "한글 패치 제거",
                "error",
                "한글 패치 제거 실패",
                Some(message.clone()),
                55,
            )?;
            return Err(anyhow!(message));
        }
    };

    emit_patch_event(
        app,
        4,
        "결과",
        "current",
        "마무리 작업을 진행하는 중...",
        None,
        90,
    )?;

    for remaining_secs in (1..=PATCH_AUTO_EXIT_DELAY_SECS).rev() {
        emit_patch_event(
            app,
            4,
            "결과",
            "done",
            "제거 모드 완료",
            Some(format!(
                "{}\n\n{}초 후 프로그램이 자동으로 종료됩니다.",
                removal_detail, remaining_secs
            )),
            100,
        )?;

        std::thread::sleep(Duration::from_secs(1));
    }

    log_info(format!(
        "제거 작업이 완료되어 {}초 후 프로그램을 자동 종료합니다.",
        PATCH_AUTO_EXIT_DELAY_SECS
    ));
    app.exit(0);

    Ok(())
}

fn emit_patch_event(
    app: &AppHandle,
    step_index: usize,
    title: &str,
    state: &str,
    message: &str,
    detail: Option<String>,
    progress: u8,
) -> Result<()> {
    let detail_for_log = detail.as_deref().unwrap_or("-");
    log_info(format!(
        "이벤트 전송: step={step_index}, state={state}, progress={progress}, title={title}, message={message}, detail={detail_for_log}"
    ));

    if let Err(error) = app.emit(
        PATCH_EVENT_NAME,
        PatchEvent {
            step_index,
            title: title.to_string(),
            state: state.to_string(),
            message: message.to_string(),
            detail,
            progress,
        },
    ) {
        log_error(format!(
            "패치 이벤트 전송 실패: step={step_index}, state={state}, title={title}, 오류={error:#}"
        ));
        return Err(error).context("패치 이벤트 전송에 실패했습니다.");
    }

    Ok(())
}

fn is_supported_patch_target(target: &str) -> bool {
    matches!(target, INT_STEAM_TARGET | CN_BILIBILI_TARGET)
}

fn resolve_patch_target(raw_target: Option<&str>) -> Result<String> {
    let target = raw_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_uppercase())
        .ok_or_else(|| {
            anyhow!(
                "패치 실행 대상이 비어 있습니다.\n지원 형식: astral://patch/<TARGET>\nTARGET: INT_STEAM, CN_BILIBILI"
            )
        })?;

    if !is_supported_patch_target(&target) {
        bail!(
            "지원하지 않는 패치 대상입니다.\n{}\n지원 대상: {}, {}",
            target,
            INT_STEAM_TARGET,
            CN_BILIBILI_TARGET
        );
    }

    Ok(target)
}

fn resolve_rules_api(target: &str) -> Result<&'static str> {
    match target {
        INT_STEAM_TARGET => Ok(INT_STEAM_RULES_API),
        CN_BILIBILI_TARGET => Ok(CN_BILIBILI_RULES_API),
        other => bail!("현재 지원하지 않는 대상입니다.\n{other}"),
    }
}

fn fetch_remove_root_ids(target: &str) -> Result<Vec<String>> {
    let api_url = resolve_rules_api(target)?;
    let client = reqwest::blocking::Client::new();

    let rules = client
        .get(api_url)
        .header("User-Agent", "AstralAutoPatcherV2")
        .header("Accept", "application/json")
        .send()
        .with_context(|| format!("제거 규칙 API 요청에 실패했습니다.\n{}", api_url))?
        .error_for_status()
        .with_context(|| format!("제거 규칙 API 응답이 실패 상태입니다.\n{}", api_url))?
        .json::<PatchRuleDocument>()
        .with_context(|| format!("제거 규칙 JSON 파싱에 실패했습니다.\n{}", api_url))?;

    let mut root_ids = Vec::new();
    let mut seen = HashSet::new();

    for task in rules.tasks {
        let Some(source) = task.source.as_deref() else {
            continue;
        };

        if !source.eq_ignore_ascii_case("AssetBundles") {
            continue;
        }

        let Some(root_id) = task
            .root_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };

        let dedupe_key = root_id.to_ascii_lowercase();
        if seen.insert(dedupe_key) {
            root_ids.push(root_id);
        }
    }

    if root_ids.is_empty() {
        bail!(
            "제거 규칙에서 AssetBundles 대상 root_id를 찾지 못했습니다.\n{}",
            api_url
        );
    }

    Ok(root_ids)
}

fn format_root_ids(root_ids: &[String]) -> String {
    if root_ids.is_empty() {
        return "-".to_string();
    }

    root_ids
        .iter()
        .map(|value| format!("- {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_assetbundle_roots(local_feimo_dir: &Path, root_ids: &[String]) -> Result<String> {
    let mut removed = Vec::new();
    let mut not_found = Vec::new();

    for root_id in root_ids {
        let entry_path = local_feimo_dir.join(root_id);
        if !entry_path.exists() {
            not_found.push(root_id.clone());
            continue;
        }

        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path).with_context(|| {
                format!(
                    "패치 데이터 폴더 삭제에 실패했습니다.\n{}",
                    entry_path.display()
                )
            })?;
        } else {
            fs::remove_file(&entry_path).with_context(|| {
                format!(
                    "패치 데이터 파일 삭제에 실패했습니다.\n{}",
                    entry_path.display()
                )
            })?;
        }

        removed.push(root_id.clone());
    }

    Ok(format!(
        "[대상]\n{}\n[삭제 완료] {}개\n{}\n[미존재] {}개\n{}",
        local_feimo_dir.display(),
        removed.len(),
        format_root_ids(&removed),
        not_found.len(),
        format_root_ids(&not_found)
    ))
}

fn parse_launch_context() -> Result<LaunchContext> {
    let mut args = env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        if arg == RESUME_INSTALL_ARG {
            return Ok(LaunchContext {
                mode: "install".to_string(),
                target: DEFAULT_PATCH_TARGET.to_string(),
                protocol_arg: None,
            });
        }

        if arg == RESUME_PATCH_ARG {
            let raw_target = args.next().ok_or_else(|| {
                anyhow!(
                    "재개 패치 실행 인자에 대상(route)이 없습니다.\n지원 형식: --resume-patch <TARGET>"
                )
            })?;
            let target = resolve_patch_target(Some(raw_target.as_str()))?;

            return Ok(LaunchContext {
                mode: "patch".to_string(),
                target,
                protocol_arg: None,
            });
        }
    }

    let protocol_arg = env::args().find(|arg| arg.starts_with("astral://"));

    if let Some(arg) = protocol_arg.clone() {
        if let Some(rest) = arg.strip_prefix("astral://patch/") {
            let route = rest.split(['/', '?', '#']).next().unwrap_or_default();
            let target = resolve_patch_target(Some(route))?;

            return Ok(LaunchContext {
                mode: "patch".to_string(),
                target,
                protocol_arg,
            });
        }

        if let Some(rest) = arg.strip_prefix("astral://remove/") {
            let route = rest.split(['/', '?', '#']).next().unwrap_or_default();
            let target = resolve_patch_target(Some(route))?;

            return Ok(LaunchContext {
                mode: "remove".to_string(),
                target,
                protocol_arg,
            });
        }

        bail!(
            "잘못된 프로토콜 실행 경로입니다.\n{}\n지원 형식: astral://patch/<TARGET> 또는 astral://remove/<TARGET>\nTARGET: INT_STEAM, CN_BILIBILI",
            arg
        );
    }

    Ok(LaunchContext {
        mode: "install".to_string(),
        target: DEFAULT_PATCH_TARGET.to_string(),
        protocol_arg,
    })
}

fn find_game_install_dir(target: &str) -> Result<PathBuf> {
    match target {
        INT_STEAM_TARGET => find_int_steam_game_install_dir(),
        CN_BILIBILI_TARGET => find_cn_bilibili_game_install_dir(),
        other => bail!("현재 지원하지 않는 대상입니다.\n{other}"),
    }
}

fn find_int_steam_game_install_dir() -> Result<PathBuf> {
    let mut checked_paths = Vec::new();

    for steam_library_root in find_steam_library_roots()? {
        let game_dir = steam_library_root
            .join("steamapps")
            .join("common")
            .join("Astral Party")
            .join("8vJXnINT")
            .join("AstralParty_INT_Data")
            .join("StreamingAssets")
            .join("aa")
            .join("StandaloneWindows64");

        if game_dir.exists() {
            return Ok(game_dir);
        }

        checked_paths.push(game_dir);
    }

    bail!(
        "Astral Party 설치 경로를 찾지 못했습니다.\n[확인 대상]\n{}",
        format_path_list(&checked_paths)
    )
}

fn find_steam_library_roots() -> Result<Vec<PathBuf>> {
    let steam_root = find_steam_install_root()?;
    let mut roots = Vec::new();
    let mut seen = HashSet::new();

    push_unique_library_root(&mut roots, &mut seen, steam_root.clone());

    let libraryfolders_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    match fs::read_to_string(&libraryfolders_path) {
        Ok(content) => {
            for line in content.lines() {
                let values = extract_vdf_quoted_values(line);
                if values.len() < 2 || !values[0].eq_ignore_ascii_case("path") {
                    continue;
                }

                let normalized_path = values[1].replace("\\\\", "\\");
                if normalized_path.trim().is_empty() {
                    continue;
                }

                push_unique_library_root(&mut roots, &mut seen, PathBuf::from(normalized_path));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            log_info(format!(
                "libraryfolders.vdf가 없어 기본 Steam 라이브러리만 사용합니다. {}",
                libraryfolders_path.display()
            ));
        }
        Err(error) => {
            log_error(format!(
                "libraryfolders.vdf 읽기에 실패했습니다. 기본 Steam 라이브러리만 사용합니다. {} ({error:#})",
                libraryfolders_path.display()
            ));
        }
    }

    Ok(roots)
}

fn push_unique_library_root(roots: &mut Vec<PathBuf>, seen: &mut HashSet<String>, path: PathBuf) {
    let normalized_key = path
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase();

    if normalized_key.is_empty() {
        return;
    }

    if seen.insert(normalized_key) {
        roots.push(path);
    }
}

fn extract_vdf_quoted_values(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        if ch == '"' {
            if in_quotes {
                values.push(current.clone());
                current.clear();
                in_quotes = false;
            } else {
                in_quotes = true;
            }

            continue;
        }

        if in_quotes {
            current.push(ch);
        }
    }

    values
}

fn format_path_list(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return "-".to_string();
    }

    paths
        .iter()
        .map(|path| format!("- {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(target_os = "windows")]
fn find_cn_bilibili_game_install_dir() -> Result<PathBuf> {
    let game_root = find_bilibili_game_install_root()?;
    let game_dir = game_root
        .join("吉星派对_Data")
        .join("StreamingAssets")
        .join("aa")
        .join("StandaloneWindows64");

    if !game_dir.exists() {
        bail!(
            "bilibili Astral Party 설치 경로를 찾지 못했습니다.\n[확인] {}",
            game_dir.display()
        );
    }

    Ok(game_dir)
}

#[cfg(not(target_os = "windows"))]
fn find_cn_bilibili_game_install_dir() -> Result<PathBuf> {
    bail!("CN_BILIBILI 경로 확인은 현재 Windows에서만 지원됩니다.")
}

fn resolve_install_dir() -> Result<PathBuf> {
    if let Ok(program_files) = env::var("ProgramFiles") {
        return Ok(PathBuf::from(program_files).join("AstralAutoPatcher"));
    }

    if let Ok(program_w6432) = env::var("ProgramW6432") {
        return Ok(PathBuf::from(program_w6432).join("AstralAutoPatcher"));
    }

    bail!("Program Files 경로를 확인할 수 없습니다.")
}

fn relocate_runtime_to_safe_path() -> Result<(PathBuf, String)> {
    let current_exe = env::current_exe().context("프로그램 경로를 확인할 수 없습니다.")?;
    let exe_name = current_exe
        .file_name()
        .ok_or_else(|| anyhow!("프로그램 이름을 확인할 수 없습니다."))?
        .to_owned();
    let current_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow!("프로그램의 부모 경로를 확인할 수 없습니다."))?
        .to_path_buf();

    let safe_root = resolve_install_dir()?;
    let safe_exe = safe_root.join(&exe_name);

    if current_exe == safe_exe {
        return Ok((
            safe_exe,
            format!("프로그램이 이미 설치 경로에서 실행 중입니다.\n{}", safe_root.display()),
        ));
    }

    fs::create_dir_all(&safe_root)
        .with_context(|| format!("설치 경로 생성에 실패했습니다.\n{}", safe_root.display()))?;

    if safe_exe.exists() {
        fs::remove_file(&safe_exe)
            .with_context(|| format!("기존 실행 파일 제거에 실패했습니다.\n{}", safe_exe.display()))?;
    }

    fs::copy(&current_exe, &safe_exe).with_context(|| {
        format!(
            "실행 파일 복사에 실패했습니다.\n{} -> {}",
            current_exe.display(),
            safe_exe.display()
        )
    })?;

    if !safe_exe.exists() {
        bail!(
            "설치 경로에 실행 파일이 없습니다.\n{}",
            safe_exe.display()
        );
    }

    let cleanup_note = cleanup_source_exe_if_needed(&current_exe, &safe_exe);

    let detail = if current_dir == safe_root {
        format!(
            "프로그램을 최신 버전으로 업데이트했습니다.\n{}\n{}",
            safe_exe.display(),
            cleanup_note
        )
    } else {
        format!("{}\n{}", safe_exe.display(), cleanup_note)
    };

    Ok((safe_exe, detail))
}

fn cleanup_source_exe_if_needed(source_exe: &Path, installed_exe: &Path) -> String {
    if source_exe == installed_exe {
        return "원본 실행 파일 정리 단계는 건너뛰었습니다.".to_string();
    }

    match fs::remove_file(source_exe) {
        Ok(_) => "다운로드 폴더의 원본 실행 파일을 삭제했습니다.".to_string(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            "원본 실행 파일이 이미 없어 정리할 파일이 없습니다.".to_string()
        }
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            match schedule_deferred_file_delete(source_exe) {
                Ok(_) => {
                    "".to_string()
                }
                Err(schedule_error) => {
                    log_error(format!(
                        "원본 실행 파일 자동 삭제 예약 실패: {schedule_error:#}"
                    ));
                    "원본 실행 파일 삭제 권한이 없어 자동 정리를 예약하지 못했습니다.".to_string()
                }
            }
        }
        Err(error) => {
            log_error(format!("원본 실행 파일 삭제 실패: {error:#}"));
            "원본 실행 파일 삭제에 실패했습니다. 수동으로 삭제해 주세요.".to_string()
        }
    }
}

#[cfg(target_os = "windows")]
fn schedule_deferred_file_delete(path: &Path) -> Result<()> {
    let escaped_path = path.display().to_string().replace('"', "`\"");
    let script = format!(
        "$p=\"{}\"; for($i=0; $i -lt 600; $i++) {{ try {{ Remove-Item -LiteralPath $p -Force -ErrorAction Stop; break }} catch {{ Start-Sleep -Milliseconds 500 }} }}",
        escaped_path
    );

    Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .spawn()
        .with_context(|| {
            format!(
                "원본 실행 파일 자동 삭제 프로세스를 시작하지 못했습니다.\n{}",
                path.display()
            )
        })?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn schedule_deferred_file_delete(_path: &Path) -> Result<()> {
    bail!("자동 삭제 예약은 현재 Windows에서만 지원됩니다.")
}

fn prepare_self_update_if_needed() -> Result<Option<SelfUpdatePlan>> {
    let release = match fetch_latest_release_from(SELF_UPDATE_RELEASE_API) {
        Ok(release) => release,
        Err(error) => {
            log_error(format!("자체 업데이트 확인 실패: {error:#}"));
            return Ok(None);
        }
    };

    let latest_version = normalize_semver(&release.tag_name);
    let current_version = normalize_semver(env!("CARGO_PKG_VERSION"));

    if !is_semver_newer(&latest_version, &current_version) {
        return Ok(None);
    }

    let current_exe_name = env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().to_string()));

    let selected_asset = match release
        .assets
        .iter()
        .find(|asset| {
            let asset_upper = asset.name.to_ascii_uppercase();
            if !asset_upper.ends_with(".EXE") {
                return false;
            }

            if asset_upper.contains("SETUP") {
                return false;
            }

            if let Some(current_name) = &current_exe_name {
                return asset.name.eq_ignore_ascii_case(current_name);
            }

            asset_upper.contains("ASTRALAUTOPATCHER")
        })
    {
        Some(asset) => asset,
        None => {
            log_error("자체 업데이트 실행 파일 에셋을 찾지 못했습니다. 현재 버전으로 계속 진행합니다.");
            return Ok(None);
        }
    };

    let temp_dir = env::temp_dir().join("AstralAutoPatcherV2").join("self-update");
    fs::create_dir_all(&temp_dir)
        .with_context(|| format!("업데이트 임시 폴더 생성에 실패했습니다.\n{}", temp_dir.display()))?;

    let downloaded_exe = temp_dir.join(&selected_asset.name);
    if downloaded_exe.exists() {
        fs::remove_file(&downloaded_exe).with_context(|| {
            format!(
                "기존 업데이트 임시 파일 정리에 실패했습니다.\n{}",
                downloaded_exe.display()
            )
        })?;
    }

    if let Err(error) = download_file(&selected_asset.browser_download_url, &downloaded_exe) {
        log_error(format!(
            "자체 업데이트 파일 다운로드 실패: {error:#}. 현재 버전으로 계속 진행합니다."
        ));
        return Ok(None);
    }

    Ok(Some(SelfUpdatePlan {
        latest_version,
        exe_path: downloaded_exe,
    }))
}

fn relaunch_for_resume(app: &AppHandle, exe_path: &Path, patch_target: Option<&str>) -> Result<()> {
    let mut command = Command::new(exe_path);

    match patch_target {
        Some(target) => {
            command.arg(RESUME_PATCH_ARG).arg(target);
        }
        None => {
            command.arg(RESUME_INSTALL_ARG);
        }
    }

    command
        .spawn()
        .with_context(|| format!("업데이트된 실행 파일 실행에 실패했습니다.\n{}", exe_path.display()))?;

    log_info(format!(
        "업데이트된 실행 파일을 실행하고 현재 프로세스를 종료합니다. {}",
        exe_path.display()
    ));
    app.exit(0);
    Ok(())
}

fn normalize_semver(version: &str) -> String {
    version
        .trim_start_matches('v')
        .trim_start_matches('V')
        .split('-')
        .next()
        .unwrap_or(version)
        .to_string()
}

fn parse_semver_tuple(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch))
}

fn is_semver_newer(latest: &str, current: &str) -> bool {
    match (parse_semver_tuple(latest), parse_semver_tuple(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

#[cfg(target_os = "windows")]
fn cleanup_legacy_v1_artifacts() -> String {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut details = Vec::new();

    match find_steam_library_roots() {
        Ok(library_roots) => {
            let mut removed = Vec::new();
            let mut not_found = Vec::new();
            let mut failed = Vec::new();

            for steam_library_root in library_roots {
                let legacy_exe_path = steam_library_root
                    .join("steamapps")
                    .join("common")
                    .join("Astral Party")
                    .join("AstralAutoPatcher.exe");

                if !legacy_exe_path.exists() {
                    not_found.push(legacy_exe_path);
                    continue;
                }

                match fs::remove_file(&legacy_exe_path) {
                    Ok(_) => {
                        removed.push(legacy_exe_path);
                    }
                    Err(error) => {
                        log_error(format!(
                            "v1 실행 파일 삭제 실패: {} ({error:#})",
                            legacy_exe_path.display()
                        ));
                        failed.push(format!("- {} ({})", legacy_exe_path.display(), error));
                    }
                }
            }

            details.push(format!(
                "[v1 실행 파일]\n삭제 완료 {}개\n{}\n미존재 {}개\n{}\n삭제 실패 {}개\n{}",
                removed.len(),
                format_path_list(&removed),
                not_found.len(),
                format_path_list(&not_found),
                failed.len(),
                if failed.is_empty() {
                    "-".to_string()
                } else {
                    failed.join("\n")
                }
            ));
        }
        Err(error) => {
            log_error(format!("Steam 설치 경로 확인 실패(레거시 정리): {error:#}"));
            details.push(format!(
                "[v1 실행 파일]\nSteam 경로를 찾지 못해 건너뜀\n{}",
                error
            ));
        }
    }

    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    match hkcr.open_subkey_with_flags("astral", KEY_READ) {
        Ok(_) => match hkcr.delete_subkey_all("astral") {
            Ok(_) => {
                details.push("[HKEY_CLASSES_ROOT\\astral]\n삭제 완료".to_string());
            }
            Err(error) => {
                log_error(format!("HKEY_CLASSES_ROOT\\astral 삭제 실패: {error:#}"));
                details.push(format!(
                    "[HKEY_CLASSES_ROOT\\astral]\n삭제 실패\n{}",
                    error
                ));
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            details.push("[HKEY_CLASSES_ROOT\\astral]\n대상이 없어 건너뜀".to_string());
        }
        Err(error) => {
            log_error(format!(
                "HKEY_CLASSES_ROOT\\astral 확인 실패(레거시 정리): {error:#}"
            ));
            details.push(format!(
                "[HKEY_CLASSES_ROOT\\astral]\n확인 실패\n{}",
                error
            ));
        }
    }

    details.join("\n\n")
}

#[cfg(not(target_os = "windows"))]
fn cleanup_legacy_v1_artifacts() -> String {
    "레거시 데이터 정리는 현재 Windows에서만 지원됩니다.".to_string()
}

#[cfg(target_os = "windows")]
fn register_astral_protocol(exe_path: &Path) -> Result<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let command = format!("\"{}\" \"%1\"", exe_path.display());
    let icon = format!("\"{}\",1", exe_path.display());

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (astral_key, _) = hkcu
        .create_subkey("Software\\Classes\\astral")
        .context("astral 레지스트리 키 생성에 실패했습니다.")?;
    astral_key
        .set_value("", &"URL:Astral Protocol")
        .context("astral 레지스트리 설명 설정에 실패했습니다.")?;
    astral_key
        .set_value("URL Protocol", &"")
        .context("URL Protocol 플래그 설정에 실패했습니다.")?;

    let (icon_key, _) = astral_key
        .create_subkey("DefaultIcon")
        .context("DefaultIcon 하위 키 생성에 실패했습니다.")?;
    icon_key
        .set_value("", &icon)
        .context("DefaultIcon 값 설정에 실패했습니다.")?;

    let (command_key, _) = astral_key
        .create_subkey("shell\\open\\command")
        .context("shell\\open\\command 하위 키 생성에 실패했습니다.")?;
    command_key
        .set_value("", &command)
        .context("프로토콜 실행 명령 설정에 실패했습니다.")?;

    Ok(format!(
        "[HKCU\\Software\\Classes\\astral]\n{}",
        command
    ))
}

#[cfg(not(target_os = "windows"))]
fn register_astral_protocol(_exe_path: &Path) -> Result<String> {
    bail!("프로토콜 레지스트리 등록은 현재 Windows에서만 지원됩니다.")
}

#[cfg(target_os = "windows")]
fn find_steam_install_root() -> Result<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey("SOFTWARE\\Valve\\Steam") {
        if let Ok(path) = key.get_value::<String, _>("SteamPath") {
            return Ok(PathBuf::from(path));
        }

        if let Ok(path) = key.get_value::<String, _>("InstallPath") {
            return Ok(PathBuf::from(path));
        }
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for subkey in [
        "SOFTWARE\\WOW6432Node\\Valve\\Steam",
        "SOFTWARE\\Valve\\Steam",
    ] {
        if let Ok(key) = hklm.open_subkey(subkey) {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                return Ok(PathBuf::from(path));
            }
        }
    }

    bail!("레지스트리에서 Steam 설치 경로를 확인할 수 없습니다.")
}

#[cfg(not(target_os = "windows"))]
fn find_steam_install_root() -> Result<PathBuf> {
    bail!("이 기능은 현재 Windows에서만 지원됩니다.")
}

fn find_local_feimo_path(target: &str) -> Result<PathBuf> {
    match target {
        INT_STEAM_TARGET => find_int_steam_local_feimo_path(),
        CN_BILIBILI_TARGET => find_cn_bilibili_local_feimo_path(),
        other => bail!("현재 지원하지 않는 대상입니다.\n{other}"),
    }
}

fn find_int_steam_local_feimo_path() -> Result<PathBuf> {
    let user_profile = env::var("USERPROFILE").context("USERPROFILE 환경 변수를 확인할 수 없습니다.")?;
    let path = PathBuf::from(user_profile)
        .join("AppData")
        .join("LocalLow")
        .join("feimo")
        .join("AstralParty_INT")
        .join("com.unity.addressables")
        .join("AssetBundles");

    if !path.exists() {
        bail!("경로가 존재하지 않습니다.\n{}", path.display());
    }

    Ok(path)
}

#[cfg(target_os = "windows")]
fn find_cn_bilibili_local_feimo_path() -> Result<PathBuf> {
    let user_profile = env::var("USERPROFILE").context("USERPROFILE 환경 변수를 확인할 수 없습니다.")?;
    let path = PathBuf::from(user_profile)
        .join("AppData")
        .join("LocalLow")
        .join("feimo")
        .join("吉星派对")
        .join("com.unity.addressables")
        .join("AssetBundles");

    if !path.exists() {
        bail!("경로가 존재하지 않습니다.\n{}", path.display());
    }

    Ok(path)
}

#[cfg(not(target_os = "windows"))]
fn find_cn_bilibili_local_feimo_path() -> Result<PathBuf> {
    bail!("CN_BILIBILI 경로 확인은 현재 Windows에서만 지원됩니다.")
}

#[cfg(target_os = "windows")]
fn find_bilibili_game_install_root() -> Result<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey("Software\\AstralParty")
        .context("HKCU\\Software\\AstralParty 레지스트리 키를 열 수 없습니다.")?;
    let value = key
        .get_value::<String, _>("GameInstallPath")
        .context("HKCU\\Software\\AstralParty\\GameInstallPath 값을 읽을 수 없습니다.")?;

    let game_root = PathBuf::from(value.trim().trim_matches(char::from(0)).trim_matches('"'));
    if !game_root.exists() {
        bail!(
            "레지스트리에 등록된 bilibili 게임 경로가 존재하지 않습니다.\n{}",
            game_root.display()
        );
    }

    Ok(game_root)
}

fn prepare_patch_bundle(target: &str) -> Result<PreparedPatch> {
    let release = fetch_latest_release()?;
    let route_marker = format!("-{}-", target.to_ascii_uppercase());

    let selected_asset = release
        .assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_uppercase();
            name.starts_with("ASTRALPARTY-KOREAN-PATCH-")
                && name.contains(&route_marker)
                && name.ends_with(".ZIP")
        })
        .ok_or_else(|| anyhow!("{target}에 해당하는 패치 ZIP 에셋을 찾지 못했습니다."))?;

    let work_root = env::temp_dir().join("AstralAutoPatcherV2").join(target);
    let zip_path = work_root.join("patch.zip");
    let extract_dir = work_root.join("extracted");

    fs::create_dir_all(&work_root).context("작업용 임시 폴더 생성에 실패했습니다.")?;
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).context("기존 임시 압축 해제 폴더 정리에 실패했습니다.")?;
    }
    fs::create_dir_all(&extract_dir).context("압축 해제 폴더 생성에 실패했습니다.")?;

    download_file(&selected_asset.browser_download_url, &zip_path)?;
    extract_zip_file(&zip_path, &extract_dir)?;

    Ok(PreparedPatch {
        release_tag: release.tag_name,
        asset_name: selected_asset.name.clone(),
        extract_dir,
    })
}

fn fetch_latest_release() -> Result<GithubRelease> {
    fetch_latest_release_from(PATCH_BUNDLE_RELEASE_API)
}

fn fetch_latest_release_from(url: &str) -> Result<GithubRelease> {
    let client = reqwest::blocking::Client::new();

    client
        .get(url)
        .header("User-Agent", "AstralAutoPatcherV2")
        .header("Accept", "application/vnd.github+json")
        .send()
        .context("GitHub 릴리스 정보를 요청하지 못했습니다.")?
        .error_for_status()
        .context("GitHub 릴리스 요청이 실패했습니다.")?
        .json::<GithubRelease>()
        .context("GitHub 릴리스 응답을 파싱하지 못했습니다.")
}

fn download_file(url: &str, destination: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let mut response = client
        .get(url)
        .header("User-Agent", "AstralAutoPatcherV2")
        .send()
        .with_context(|| format!("파일 다운로드 요청에 실패했습니다.\n{url}"))?
        .error_for_status()
        .with_context(|| format!("파일 다운로드 응답이 실패 상태입니다.\n{url}"))?;

    let mut file = File::create(destination)
        .with_context(|| format!("다운로드 파일 생성에 실패했습니다.\n{}", destination.display()))?;
    response
        .copy_to(&mut file)
        .with_context(|| format!("다운로드 파일 저장에 실패했습니다.\n{}", destination.display()))?;

    Ok(())
}

fn extract_zip_file(zip_path: &Path, output_dir: &Path) -> Result<()> {
    let file = File::open(zip_path)
        .with_context(|| format!("ZIP 파일 열기에 실패했습니다.\n{}", zip_path.display()))?;
    let mut archive = ZipArchive::new(file).context("ZIP 아카이브 파싱에 실패했습니다.")?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).context("ZIP 항목 읽기에 실패했습니다.")?;
        let Some(enclosed_path) = entry.enclosed_name().map(PathBuf::from) else {
            continue;
        };

        let out_path = output_dir.join(enclosed_path);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .with_context(|| format!("ZIP 디렉터리 생성에 실패했습니다.\n{}", out_path.display()))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("ZIP 부모 디렉터리 생성에 실패했습니다.\n{}", parent.display()))?;
        }

        let mut outfile = File::create(&out_path)
            .with_context(|| format!("ZIP 출력 파일 생성에 실패했습니다.\n{}", out_path.display()))?;
        io::copy(&mut entry, &mut outfile)
            .with_context(|| format!("ZIP 파일 추출에 실패했습니다.\n{}", out_path.display()))?;
    }

    Ok(())
}

fn apply_patch_bundle(
    target: &str,
    extracted_root: &Path,
    game_dir: &Path,
    local_feimo_dir: &Path,
) -> Result<String> {
    let assetbundles_source =
        find_first_directory_named(extracted_root, "AssetBundles").ok_or_else(|| {
            anyhow!(
                "압축 해제 결과에서 AssetBundles 폴더를 찾지 못했습니다.\n{}",
                extracted_root.display()
            )
        })?;

    let standalone_source =
        find_first_directory_named(extracted_root, "StandaloneWindows64").ok_or_else(|| {
            anyhow!(
                "압축 해제 결과에서 StandaloneWindows64 폴더를 찾지 못했습니다.\n{}",
                extracted_root.display()
            )
        })?;

    match target {
        INT_STEAM_TARGET | CN_BILIBILI_TARGET => {}
        other => bail!("현재 지원하지 않는 대상입니다.\n{other}"),
    }

    copy_directory_contents(&assetbundles_source, local_feimo_dir)?;
    copy_directory_contents(&standalone_source, game_dir)?;

    Ok(format!(
        "[AssetBundles]\n{}\n[StandaloneWindows64]\n{}",
        local_feimo_dir.display(),
        game_dir.display()
    ))
}

fn find_first_directory_named(root: &Path, name: &str) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path
            .file_name()
            .and_then(|segment| segment.to_str())
            .map(|segment| segment.eq_ignore_ascii_case(name))
            .unwrap_or(false)
        {
            return Some(path);
        }

        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    stack.push(entry.path());
                }
            }
        }
    }

    None
}

fn copy_directory_contents(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        bail!("원본 디렉터리가 존재하지 않습니다.\n{}", source.display());
    }

    fs::create_dir_all(target)
        .with_context(|| format!("대상 디렉터리 생성에 실패했습니다.\n{}", target.display()))?;

    for entry in fs::read_dir(source)
        .with_context(|| format!("디렉터리 읽기에 실패했습니다.\n{}", source.display()))?
    {
        let entry = entry.with_context(|| format!("디렉터리 항목 읽기에 실패했습니다.\n{}", source.display()))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry
            .file_type()
            .with_context(|| format!("파일 유형 확인에 실패했습니다.\n{}", source_path.display()))?;

        if file_type.is_dir() {
            copy_directory_contents(&source_path, &target_path)?;
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("대상 부모 폴더 생성에 실패했습니다.\n{}", parent.display()))?;
        }

        if let Err(error) = fs::copy(&source_path, &target_path) {
            let mut message = format!(
                "파일 복사에 실패했습니다.\n{} -> {}",
                source_path.display(),
                target_path.display()
            );

            if error.kind() == io::ErrorKind::PermissionDenied {
                message.push_str(
                    "\n[확인] 게임과 런처를 완전히 종료한 뒤 다시 시도하세요.",
                );

                if is_protected_install_path(&target_path) {
                    message.push_str(
                        "\n[권한] Program Files 아래 경로는 관리자 권한이 필요할 수 있습니다.",
                    );
                }
            }

            return Err(error).context(message);
        }
    }

    Ok(())
}

fn is_protected_install_path(path: &Path) -> bool {
    ["ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(|key| env::var(key).ok())
        .map(PathBuf::from)
        .any(|root| path.starts_with(root))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_launch_context,
            start_patch_workflow,
            start_remove_workflow,
            start_install_workflow
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
