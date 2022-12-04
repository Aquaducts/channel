use anyhow::Result;
use bamboo_common::{websocket::Messages, Job, Repos};
use futures_util::{SinkExt, StreamExt};
use haikunator::Haikunator;
use runner::{config::Config, container::Container, io::FakedIO, lxc};
use serde::__private::de::IdentifierDeserializer;
use serde::{Deserialize, Serialize};
use std::os::unix::io::AsRawFd;
use std::ptr::null_mut;
use tokio::io::AsyncBufReadExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildRequest {
    pub repo: String,
    pub branch: Option<String>,
    /// The name is used to name the build's container, if none is provided
    /// a random haiku will be used.
    pub name: Option<String>,
}

async fn test_builder(build_request: BuildRequest) -> Result<()> {
    let parsed = build_request;

    let name = parsed.name.unwrap_or({
        let haikunator = Haikunator::default();
        haikunator.haikunate()
    });

    println!("Got build request -- {}", name);

    let container = Container::new(name.clone())?;
    _ = container.start();

    let mut fake_io = FakedIO::create(name.clone()).await?;

    let mut attach_options = lxc::lxc_attach_options_t {
        attach_flags: 0,
        namespaces: -1,
        personality: -1,
        initial_cwd: null_mut(),
        uid: 0,
        gid: 0,
        env_policy: 0,
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

    let program = "/sbin/apk";

    let commands = vec![
        format!("{} update", program),
        format!("{} add git", program),
        format!("/usr/bin/which git"),
        format!("/usr/bin/git clone {}", &parsed.repo),
    ];

    for command in commands {
        let join = fake_io.watch().await;
        container.exec(command.try_into()?, &mut attach_options);
        join.abort();
        fake_io.clear().await?;
    }

    container.stop()?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = std::fs::read_to_string("./runner/Config.toml")?;
    let config = toml::from_str::<Config>(&config)?;

    let url = &format!(
        "ws://{}/ws?name={}&password={}",
        config.spire.host, config.name, config.password
    );

    let (ws_stream, _) = connect_async(url).await.unwrap();

    let (mut writer, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if msg.is_text() || msg.is_binary() {
            println!("{:?}", msg);
            if let Ok(request) = serde_json::from_str::<Job>(msg.to_text()?) {
                println!("HELLO");
                // UHMM should the builder be requesting the job's repo through a websocket message? WHO KNOWS!
                writer
                    .send(Message::Text(serde_json::to_string(
                        &Messages::GetJobRepo {
                            job: request.id,
                            repo: request.repo,
                        },
                    )?))
                    .await?;

                let Some(Ok(next_message)) = read.next().await else {
                    panic!("OH NO IM PANICKING!!!");
                };

                if next_message.is_text() || next_message.is_binary() {
                    let repo = serde_json::from_str::<Repos>(next_message.to_text()?)?;
                    if test_builder(BuildRequest {
                        repo: format!("https://github.com/{}/{}.git", repo.owner, repo.name),
                        branch: None,
                        name: None,
                    })
                    .await
                    .is_err()
                    {
                        writer.send(Message::Text(String::from("WHAT"))).await?;
                    }
                }
            }
        }
    }

    Ok(())
}
