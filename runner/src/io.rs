use anyhow::Result;

use serde::{Deserialize, Serialize};
use std::{fs::create_dir, io::SeekFrom, path::Path};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader},
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CollectedOutput {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl FakedIO {
    pub async fn create(builder: String) -> Result<Self> {
        Ok(Self {
            builder: builder.to_string(),
            stdin: create_temp_file(&builder, "stdin").await?,
            stdout: create_temp_file(&builder, "stdout").await?,
            stderr: create_temp_file(&builder, "stderr").await?,
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

    pub async fn watch(&self) -> JoinHandle<()> {
        let builder_name = self.builder.clone();
        tokio::spawn(async move {
            println!("Started.");
            let _fake_io = FakedIO::create(builder_name).await.unwrap();
            let mut stdout_bufreader = BufReader::new(_fake_io.stdout);
            let mut stderr_bufreader = BufReader::new(_fake_io.stderr);
            loop {
                // out
                {
                    let mut buf = String::new();
                    _ = stdout_bufreader.read_line(&mut buf).await;
                    if !buf.is_empty() {
                        println!("OUT: {:?}", buf);
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
