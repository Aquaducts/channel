use anyhow::Result;

use bamboo_common::websocket::Messages;
use futures_util::{
    SinkExt,
};
use serde::{Deserialize, Serialize};
use std::{fs::create_dir, io::SeekFrom, path::Path};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader},
    sync::{
        mpsc::{Sender},
    },
    task::JoinHandle,
};


pub async fn create_temp_file(builder: &str, file: &str) -> Result<File> {
    if !Path::new("/tmp/bamboo").exists() {
        create_dir("/tmp/bamboo")?;
    }
    let base_path = format!("/tmp/bamboo/{}", builder);
    if !Path::new(&base_path).exists() {
        create_dir(&base_path)?;
    }
    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&format!("{}/{}", base_path, file))
        .await?)
}

pub enum FakedIOType {
    StdOut,
    StdIn,
    StdErr,
}

pub struct FakedIO {
    pub stdout: File,
    pub stdin: File,
    pub stderr: File,
    pub builder: String,
    pub job: i64,
    pub current_command: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CollectedOutput {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl FakedIO {
    pub async fn create(builder: String, job: i64) -> Result<Self> {
        Ok(Self {
            builder: builder.to_string(),
            stdin: create_temp_file(&builder, "stdin").await?,
            stdout: create_temp_file(&builder, "stdout").await?,
            stderr: create_temp_file(&builder, "stderr").await?,
            job,
            current_command: None,
        })
    }
    pub async fn clear(&mut self) -> Result<()> {
        self.stdout.set_len(0).await?;
        self.stdout.seek(SeekFrom::End(0)).await?;

        self.stdin.set_len(0).await?;
        self.stdin.seek(SeekFrom::End(0)).await?;

        self.stderr.set_len(0).await?;
        self.stderr.seek(SeekFrom::End(0)).await?;
        Ok(())
    }

    pub async fn watch(
        &self,
        sender: Sender<Messages>,
        step: String,
        pipe: Option<String>,
    ) -> JoinHandle<()> {
        let builder_name = self.builder.clone();
        let sender = sender;
        let job = self.job;
        tokio::spawn(async move {
            println!("Started.");
            let _fake_io = FakedIO::create(builder_name, job).await.unwrap();
            let mut stdout_bufreader = BufReader::new(_fake_io.stdout);
            let mut stderr_bufreader = BufReader::new(_fake_io.stderr);

            loop {
                // out
                {
                    let mut buf = String::new();
                    _ = stdout_bufreader.read_line(&mut buf).await;
                    if !buf.is_empty() {
                        println!("OUT {buf:?}");
                        sender
                            .send(Messages::CreateJobLog {
                                job,
                                status: 0,
                                step: step.clone(),
                                output: buf,
                                pipe: pipe
                                    .as_ref()
                                    .unwrap_or(&String::from("Container Setup"))
                                    .to_string(),
                            })
                            .await
                            .unwrap()
                    }
                }

                {
                    let mut buf = String::new();
                    _ = stderr_bufreader.read_line(&mut buf).await;
                    if !buf.is_empty() {
                        println!("ERR: {:?}", buf);
                    }
                }
            }
        })
    }
}
