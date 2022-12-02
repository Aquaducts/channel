use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use haikunator::Haikunator;
use runner::{container::Container, io::FakedIO, lxc};
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

#[derive(Serialize, Deserialize, Debug)]
pub struct JobRequest {
    pub repo: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // use builder::buildrequest_capnp::build_request;
    // test_builder(BuildRequest {
    //     repo: String::from("https://github.com/ibx34/main-test-repo.git"),
    //     branch: None,
    //     name: None,
    // })
    // .await?;

    let (ws_stream, _) =
        connect_async(&"ws://localhost:8080/ws?name=runner1&password=runner1234".to_string())
            .await
            .unwrap();

    let (mut writer, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if msg.is_text() || msg.is_binary() {
            if let Ok(request) = serde_json::from_str::<JobRequest>(msg.to_text()?) {
                if test_builder(BuildRequest {
                    repo: request.repo,
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

    Ok(())
}
