use anyhow::Result;
use common::{websocket::Messages, RepoConfig, Step};

use futures_util::{SinkExt, StreamExt};

use runner::{config::Config, container::Container, io::FakedIO, lxc};
use serde::__private::de::IdentifierDeserializer;
use serde::{Deserialize, Serialize};

use std::os::unix::io::AsRawFd;
use std::ptr::null_mut;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;

use tokio::sync::mpsc::{self, Sender};

use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildRequest {
    pub repo_name: String,
    pub repo_owner: String,
    pub branch: Option<String>,
}

async fn test_builder(
    job: i64,
    build_request: BuildRequest,
    config: RepoConfig,
    runner_conf: &Config,
    sender: Sender<Messages>,
) -> Result<()> {
    let parsed = build_request;
    let name = format!("{}-{}", runner_conf.name, job);

    println!("Got build request -- {}", name);

    let container = Container::new(name.clone())?;
    _ = container.start();

    let mut fake_io = FakedIO::create(name.clone(), job).await?;

    let mut attach_options = lxc::lxc_attach_options_t {
        attach_flags: 0,
        namespaces: -1,
        personality: -1,
        initial_cwd: null_mut(),
        uid: 0,
        gid: 0,
        env_policy: lxc::lxc_attach_env_policy_t_LXC_ATTACH_CLEAR_ENV as u32,
        extra_env_vars: null_mut(),
        extra_keep_env: null_mut(),
        log_fd: fake_io.stdout.as_raw_fd(),
        stdout_fd: fake_io.stdout.as_raw_fd(),
        stderr_fd: fake_io.stderr.as_raw_fd(),
        stdin_fd: fake_io.stdin.as_raw_fd(),
        lsm_label: null_mut(),
        groups: lxc::lxc_groups_t {
            size: 0,
            list: null_mut(),
        },
    };

    // /usr/bin/which

    let program = "apk";
    let setup_commands = vec![
        String::from("/sbin/apk update"),
        String::from("/sbin/apk add bash"),
        // We clear path because it, for some reason, takes the host machines? idk...
        String::from("export PATH='$PATH:/bin/:/usr/bin:/sbin'"),
        format!("{} add git", program),
        format!(
            "git clone {}",
            format!(
                "https://github.com/{}/{}.git",
                parsed.repo_owner, parsed.repo_name
            )
        ),
    ];

    for command in setup_commands {
        let join = fake_io.watch(sender.clone(), command.clone(), None).await;
        container.exec(command.try_into()?, &mut attach_options);
        join.abort();
        fake_io.clear().await?;
    }

    // Run the uhm uhm uhm freaking uhm steps

    let spurs = config.spurs.into_iter();
    for spur in spurs {
        // send to the server that we are running X pioe ... TODO

        // UPDATE this lol
        println!("{:?}", spur);
        for step in spur.steps {
            if let Ok(step) = serde_json::from_value::<Step>(step) {
                println!("[{name}] [{:?}] [{}]", step.name, step.run);
                let step_name = step.name;
                let join = fake_io
                    .watch(sender.clone(), step_name, Some(spur.name.to_string()))
                    .await;

                println!(
                    "Command Status: {}",
                    container.exec(step.run.try_into()?, &mut attach_options)
                );
                join.abort();
                fake_io.clear().await?;
            }
        }
    }

    container.stop()?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = std::fs::read_to_string("./runner/Config.toml")?;
    let config = toml::from_str::<Config>(&config)?;

    let url = &format!(
        "ws://{}/api/ws?name={}&password={}",
        config.spire.host, config.name, config.password
    );

    let (ws_stream, _) = connect_async(url).await.unwrap();

    let (writer, read) = ws_stream.split();
    let (writer, reader) = (Arc::new(Mutex::new(writer)), Arc::new(Mutex::new(read)));

    // Create the channels for reading and writing to the ws
    let (writer_sender, mut writer_recv) = mpsc::channel::<Messages>(200);
    let (main_sender, mut main_recv) = mpsc::channel::<Messages>(200);

    let locked_reader = reader.clone();
    let reader = tokio::spawn(async move {
        let mut locked_reader = locked_reader.lock().await;

        while let Some(msg) = locked_reader.next().await {
            let msg = msg.unwrap();
            if msg.is_text() || msg.is_binary() {
                if let Ok(message) = serde_json::from_str::<Messages>(&msg.into_text().unwrap()) {
                    main_sender.send(message).await.unwrap();
                }
            }
        }
    });

    let locked_write = writer.clone();
    let writer = tokio::spawn(async move {
        let mut locked_write = locked_write.lock().await;

        while let Some(to_write) = writer_recv.recv().await {
            locked_write
                .send(Message::Text(serde_json::to_string(&to_write).unwrap()))
                .await
                .unwrap();
        }
    });

    while let Some(to_write) = main_recv.recv().await {
        if let Messages::CreateJobRun { job } = to_write {
            writer_sender
                .send(Messages::UpdateJobStatus {
                    job: job.id,
                    status: 1,
                })
                .await
                .unwrap();

            // Get repo file
            writer_sender
                .send(Messages::GetRepoConfig { repo: job.repo })
                .await
                .unwrap();

            let get_repo_cfg = main_recv.recv().await;
            let Some(Messages::RepoConfig(_config)) = get_repo_cfg else {
                println!("Failed to get repo config. {:?}", get_repo_cfg);
                continue;
            };

            // Get job's repo
            writer_sender
                .send(Messages::GetJobRepo {
                    job: job.id,
                    repo: job.repo,
                })
                .await
                .unwrap();

            let Some(Messages::Repo(repo)) = main_recv.recv().await else {
                println!("Failed to get repo.");
                continue;
            };

            let (tx, mut rx) = mpsc::channel::<Messages>(100);

            let ws_sender = writer_sender.clone();
            let join = tokio::spawn(async move {
                while let Some(Messages::CreateJobLog {
                    job,
                    status,
                    step,
                    output,
                    pipe,
                }) = rx.recv().await
                {
                    println!("Create log");

                    ws_sender
                        .send(Messages::CreateJobLog {
                            job,
                            status,
                            step,
                            output,
                            pipe,
                        })
                        .await
                        .unwrap();

                    println!("h");
                }
            });

            let output = test_builder(
                job.id,
                BuildRequest {
                    repo_name: repo.name,
                    repo_owner: repo.owner,
                    branch: None,
                },
                _config,
                &config,
                tx,
            )
            .await;
            join.abort();

            if output.is_err() {
                writer_sender
                    .send(Messages::UpdateJobStatus {
                        job: job.id,
                        status: 2,
                    })
                    .await
                    .unwrap();
                continue;
            }
            writer_sender
                .send(Messages::UpdateJobStatus {
                    job: job.id,
                    status: 3,
                })
                .await
                .unwrap()
        }
    }

    writer.abort();
    reader.abort();
    Ok(())
}
