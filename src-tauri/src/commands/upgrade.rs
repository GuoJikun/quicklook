//! 应用升级：下载更新包并安装，且支持真正中止下载。
//!
//! 这里没有直接调用 `plugin:updater|download_and_install`，原因是那个命令在拿到
//! `Update` 之后会 clone 一份、在命令自己的 async fn 里一路下载到底；前端
//! `update.close()` 只是把资源从 `resources_table` 里摘掉，**下载任务不受影响**，
//! 于是「点了取消，界面变了，但实际还在下载」，而且下完仍会安装并重启应用。
//!
//! 本模块自己持有下载 future：取消时用 `select!` 提前返回，future 被 drop，
//! reqwest 的响应体随之关闭，TCP 连接断开，下载是真的停下来。中途取消不会留下
//! 半成品 —— 字节只存在于内存，未写入磁盘。

use crate::error::QuickLookError;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use tauri::ipc::Channel;
use tauri::{Manager, ResourceId, Runtime, State, Webview};
use tauri_plugin_updater::Update;
use tokio::sync::oneshot;

/// 下载事件。字段与 plugin-updater 的 `DownloadEvent` 保持一致，
/// 前端沿用同一套回调逻辑（该类型是插件 crate 私有，无法直接复用）。
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum DownloadEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        content_length: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        chunk_length: usize,
    },
    Finished,
}

/// 一次下载任务的中止句柄：`id` 用于区分不同任务，避免旧任务收尾时误清
/// 掉「取消后立刻重试」新设进来的信号。
struct DownloadTask {
    id: u64,
    cancel: oneshot::Sender<()>,
}

/// 升级下载的运行态：`None` 表示当前没有进行中的下载。
#[derive(Default)]
pub struct UpgradeState {
    next_id: AtomicU64,
    task: Mutex<Option<DownloadTask>>,
}

impl UpgradeState {
    /// 锁中毒时取回内部值：中止下载是「尽力而为」的清理动作，不该因中毒而失效。
    fn task_slot(&self) -> MutexGuard<'_, Option<DownloadTask>> {
        self.task.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 登记新任务并返回它的 id 与中止接收端。
    fn begin(&self) -> (u64, oneshot::Receiver<()>) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (cancel, receiver) = oneshot::channel();
        *self.task_slot() = Some(DownloadTask { id, cancel });
        (id, receiver)
    }

    /// 任务结束时注销自己；只有仍是当前任务才清除，避免覆盖后一次下载。
    fn finish(&self, id: u64) {
        let mut slot = self.task_slot();
        if slot.as_ref().is_some_and(|task| task.id == id) {
            *slot = None;
        }
    }

    /// 请求中止当前下载；没有进行中的任务时是空操作。
    pub fn cancel_download(&self) {
        if let Some(task) = self.task_slot().take() {
            // 接收端可能已随命令返回而丢弃，此时 send 失败也无妨
            let _ = task.cancel.send(());
        }
    }
}

/// 下载并安装更新。
///
/// `rid` 来自前端 `check()` 返回的 `Update` 资源。取消时正常返回 `Ok`：
/// 前端已自行把界面切到「已取消」，这里不需要再构造一个错误。
#[tauri::command]
pub async fn upgrade_download_and_install<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, UpgradeState>,
    rid: ResourceId,
    on_event: Channel<DownloadEvent>,
) -> Result<(), QuickLookError> {
    let update = webview
        .resources_table()
        .get::<Update>(rid)
        .map_err(|e| QuickLookError::Other(format!("更新资源已失效: {e}")))?;
    let update = (*update).clone();

    let (task_id, cancel_rx) = state.begin();

    log::info!("开始下载更新 {}", update.version);

    let mut first_chunk = true;
    let progress_channel = on_event.clone();
    let download = update.download(
        move |chunk_length, content_length| {
            if first_chunk {
                first_chunk = false;
                let _ = progress_channel.send(DownloadEvent::Started { content_length });
            }
            let _ = progress_channel.send(DownloadEvent::Progress { chunk_length });
        },
        move || {
            let _ = on_event.send(DownloadEvent::Finished);
        },
    );
    tokio::pin!(download);

    // 两个分支同时就绪时优先看下载结果，避免「已完成却被判为取消」
    let outcome = tokio::select! {
        biased;
        result = &mut download => Some(result),
        _ = cancel_rx => None,
    };
    state.finish(task_id);

    let Some(result) = outcome else {
        log::info!("更新下载已取消");
        return Ok(());
    };

    let bytes = result.map_err(|e| QuickLookError::Other(format!("下载更新失败: {e}")))?;
    log::info!("更新包下载完成，开始安装");
    // Windows 上这一步会拉起安装程序并结束当前进程
    update
        .install(&bytes)
        .map_err(|e| QuickLookError::Other(format!("安装更新失败: {e}")))?;
    Ok(())
}

/// 中止进行中的更新下载（取消按钮、「×」关窗都会走到这里）。
#[tauri::command]
pub fn upgrade_cancel_download(state: State<'_, UpgradeState>) {
    state.cancel_download();
}
