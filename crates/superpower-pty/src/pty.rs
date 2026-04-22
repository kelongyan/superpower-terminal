use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc;

/// PTY 事件
pub enum PtyEvent {
    /// PTY 输出数据
    Data(Vec<u8>),
    /// 子进程退出（读取线程 EOF 后发送）
    Exit(i32),
}

/// PTY 会话管理
pub struct PtySession {
    /// PTY Master 端 — 保留用于 resize
    master: Box<dyn MasterPty + Send>,
    /// PTY Master 端的 writer
    writer: Box<dyn Write + Send>,
    /// 子进程句柄
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// 事件接收端
    pub rx: mpsc::Receiver<PtyEvent>,
}

impl PtySession {
    /// 创建新的 PTY 会话
    pub fn new(
        cols: u16,
        rows: u16,
        shell_program: &str,
        shell_args: &[String],
    ) -> Result<Self, String> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut cmd = CommandBuilder::new(shell_program);
        for arg in shell_args {
            cmd.arg(arg);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone reader: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to take writer: {}", e))?;

        let (tx, rx) = mpsc::channel();

        // 读取线程：读取 PTY 输出，EOF 时发送 Exit 事件
        let read_tx = tx;
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF — 子进程关闭了输出管道，视为退出
                        let _ = read_tx.send(PtyEvent::Exit(0));
                        break;
                    }
                    Ok(n) => {
                        if read_tx.send(PtyEvent::Data(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::debug!("PTY read ended: {}", e);
                        let _ = read_tx.send(PtyEvent::Exit(-1));
                        break;
                    }
                }
            }
        });

        let master = pair.master;

        Ok(Self {
            master,
            writer,
            child,
            rx,
        })
    }

    /// 向 PTY 写入数据，带重试机制
    pub fn write(&mut self, data: &[u8]) -> Result<(), String> {
        const MAX_RETRIES: usize = 3;
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.writer.write_all(data) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        }

        Err(format!(
            "PTY write error after {} retries: {}",
            MAX_RETRIES,
            last_error.unwrap()
        ))
    }

    /// 调整 PTY 大小
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("PTY resize error: {}", e))
    }

    /// 检查子进程是否仍在运行
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// 主动结束子进程，用于标签页关闭等场景
    pub fn kill(&mut self) -> Result<(), String> {
        self.child
            .kill()
            .map_err(|e| format!("PTY kill error: {}", e))
    }
}
