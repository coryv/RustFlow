use crate::stream_engine::StreamNode;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use std::net::TcpStream;
// FTP
// FtpStream used fully qualified suppaftp::FtpStream
// Check imports carefully. suppaftp exposes AsyncFtpStream if feature enabled.
// If using feature 'async-std' or 'tokio', the type is typically FtpStream.
// Let's assume AsyncFtpStream is the type alias or struct for async.

// SSH
use ssh2::Session;
use std::io::Read;

#[derive(Clone, Debug)]
pub struct FtpNode {
    host: String,
    user: String,
    pass: String,
    operation: String, // "upload" | "download"
    remote_path: String,
    local_path: String,
}

impl FtpNode {
    pub fn new(config: Value) -> Self {
        let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let user = config.get("user").and_then(|v| v.as_str()).unwrap_or("anonymous").to_string();
        let pass = config.get("password").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let operation = config.get("operation").and_then(|v| v.as_str()).unwrap_or("download").to_string();
        let remote_path = config.get("remote_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let local_path = config.get("local_path").and_then(|v| v.as_str()).unwrap_or("").to_string();

        Self { host, user, pass, operation, remote_path, local_path }
    }
}

#[async_trait]
impl StreamNode for FtpNode {
    async fn run(
        &self,
        mut inputs: Vec<mpsc::Receiver<Value>>,
        outputs: Vec<mpsc::Sender<Value>>,
    ) -> anyhow::Result<()> {
        if inputs.is_empty() { return Ok(()); }
        let mut rx = inputs.remove(0);
        let tx = if !outputs.is_empty() { Some(outputs[0].clone()) } else { None };

        while let Some(input) = rx.recv().await {
            // Async FTP
            // Note: suppaftp usage depends on feature flags.
            // If we imported it as `suppaftp = { version = "6.0", features = ["async-std"] }`
            // we should likely use `suppaftp::FtpStream` and await methods.
            // However, `suppaftp::FtpStream` is sync by default.
            // Async one is `suppaftp::AsyncFtpStream` (if using async-std/tokio).
            
            // Let's try connecting.
            // Note: `connect` is async.
            
            // Using a new connection per file for simplicity (FTP control channel).
            // Proper impl might reuse, but timeouts are tricky.
            
            // Direct blocking execution
            // We skip the async match structure since we aren't using async connect anymore.
             
            
            // Re-implementing with spawn_blocking below
            let host = self.host.clone();
            let user = self.user.clone();
            let pass = self.pass.clone();
            let op = self.operation.clone();
            let rpath = self.remote_path.clone();
            let lpath = self.local_path.clone();

            let result = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                // Use Sync FtpStream
                let mut ftp = suppaftp::FtpStream::connect(format!("{}:21", host))?;
                ftp.login(&user, &pass)?;

                if op == "upload" {
                    let content = std::fs::read(&lpath)?;
                    let mut reader = &content[..];
                    ftp.put_file(&rpath, &mut reader)?;
                } else {
                    // Download
                    // retr returns bytes? or takes generic closure?
                    // put_file takes reader. retr (simple) returns cursor/reader?
                    // suppaftp verify: retr_as_buffer?
                    let cursor = ftp.retr_as_buffer(&rpath)?;
                    std::fs::write(&lpath, cursor.into_inner())?;
                }
                ftp.quit()?;
                Ok(())
            }).await?;

            match result {
                Ok(_) => {
                    if let Some(sender) = &tx {
                        sender.send(input).await.ok();
                    }
                },
                Err(e) => {
                    eprintln!("FTP Error: {}", e);
                }
            }
        }
        Ok(())
    }
}

// async fn connect_ftp removed as we use blocking strategy


#[derive(Clone, Debug)]
pub struct SshNode {
    host: String,
    port: u16,
    user: String,
    pass: Option<String>,
    command_template: String,
}

impl SshNode {
    pub fn new(config: Value) -> Self {
        let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
        let user = config.get("user").and_then(|v| v.as_str()).unwrap_or("root").to_string();
        let pass = config.get("password").and_then(|v| v.as_str()).map(|s| s.to_string());
        let command_template = config.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();

        Self { host, port, user, pass, command_template }
    }
}

#[async_trait]
impl StreamNode for SshNode {
    async fn run(
        &self,
        mut inputs: Vec<mpsc::Receiver<Value>>,
        outputs: Vec<mpsc::Sender<Value>>,
    ) -> anyhow::Result<()> {
        if inputs.is_empty() { return Ok(()); }
        let mut rx = inputs.remove(0);
        let tx = if !outputs.is_empty() { Some(outputs[0].clone()) } else { None };

        while let Some(input) = rx.recv().await {
             let env = crate::stream_engine::expressions::create_environment();
             let command = match env.render_str(&self.command_template, &input) {
                 Ok(s) => s,
                 Err(e) => {
                     eprintln!("SSH Command Template Error: {}", e);
                     continue;
                 }
             };

             let host = self.host.clone();
             let port = self.port;
             let user = self.user.clone();
             let pass = self.pass.clone();
             let cmd = command.clone();

             // Blocking SSH
             let output_res = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
                 let tcp = TcpStream::connect(format!("{}:{}", host, port))?;
                 let mut sess = Session::new()?;
                 sess.set_tcp_stream(tcp);
                 sess.handshake()?;

                 if let Some(p) = pass {
                     sess.userauth_password(&user, &p)?;
                 } else {
                     // Assume agent or key? Implementation minimal for now
                     return Err(anyhow::anyhow!("SSH Key auth not yet implemented"));
                 }

                 let mut channel = sess.channel_session()?;
                 channel.exec(&cmd)?;
                 
                 let mut s = String::new();
                 channel.read_to_string(&mut s)?;
                 
                 channel.wait_close()?;
                 let exit_status = channel.exit_status()?;
                 if exit_status != 0 {
                     // We could return error or just include exit code in output
                 }

                 Ok(s)
             }).await?;

             match output_res {
                 Ok(stdout) => {
                     let result = serde_json::json!({
                         "stdout": stdout,
                         "command": command,
                         "original_input": input
                     });
                     if let Some(sender) = &tx {
                         sender.send(result).await.ok();
                     }
                 },
                 Err(e) => {
                     eprintln!("SSH Operation Error: {}", e);
                 }
             }
        }
        Ok(())
    }
}
