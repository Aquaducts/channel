// use anyhow::{bail, Result};
// use common::{
//     events::{CreateJobRun, Hello, Identify, RequestRepoConfig, RepoConfig as WsRepoConfig},
//     websocket::{Messages, OpCodes, WebsocketMessage},
//     Job, RepoConfig, Step, Repos, Spurs
// };
// use futures_util::stream::{SplitSink, SplitStream};
// use futures_util::{SinkExt, StreamExt};
// use runner::config::CONFIG;
// use runner::{config::Config, container::Container, io::FakedIO, lxc};
// use serde::__private::de::IdentifierDeserializer;
// use serde::{Deserialize, Serialize};
// use std::{
//     any::{Any, TypeId},
//     boxed::Box,
//     fmt::Debug,
// };
// use tokio::net::TcpStream;
// use tokio::task::JoinHandle;
// use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

// use std::os::unix::io::AsRawFd;
// use std::process::exit;
// use std::ptr::null_mut;
// use std::sync::Arc;
// use tokio::io::AsyncBufReadExt;

// use tokio::sync::mpsc::{self, Receiver, Sender};
// use tracing::{error, info};

// use tokio::sync::Mutex;
// use tokio_tungstenite::{connect_async, tungstenite::Message};

// #[derive(Debug, Serialize, Deserialize)]
// pub struct BuildRequest {
//     pub repo_name: String,
//     pub repo_owner: String,
//     pub branch: Option<String>,
// }

// async fn test_builder(
//     job: i64,
//     build_request: BuildRequest,
//     config: RepoConfig,
//     runner_conf: &Config,
//     sender: Sender<Messages>,
// ) -> Result<()> {
//     let parsed = build_request;
//     let name = format!("{}-{}", runner_conf.name, job);

//     let container = Container::new(name.clone())?;
//     _ = container.start();

//     let mut fake_io = FakedIO::create(name.clone(), job).await?;

//     let mut attach_options = lxc::lxc_attach_options_t {
//         attach_flags: 0,
//         namespaces: -1,
//         personality: -1,
//         initial_cwd: null_mut(),
//         uid: 0,
//         gid: 0,
//         env_policy: lxc::lxc_attach_env_policy_t_LXC_ATTACH_CLEAR_ENV,
//         extra_env_vars: null_mut(),
//         extra_keep_env: null_mut(),
//         log_fd: fake_io.stdout.as_raw_fd(),
//         stdout_fd: fake_io.stdout.as_raw_fd(),
//         stderr_fd: fake_io.stderr.as_raw_fd(),
//         stdin_fd: fake_io.stdin.as_raw_fd(),
//         lsm_label: null_mut(),
//         groups: lxc::lxc_groups_t {
//             size: 0,
//             list: null_mut(),
//         },
//     };

//     // /usr/bin/which

//     let program = "apk";
//     let setup_commands = vec![
//         String::from("/sbin/apk update"),
//         String::from("/sbin/apk add bash"),
//         // We clear path because it, for some reason, takes the host machines? idk...
//         String::from("export PATH='$PATH:/bin/:/usr/bin:/sbin'"),
//         format!("{} add git", program),
//         format!(
//             "git clone {}",
//             format!(
//                 "https://github.com/{}/{}.git",
//                 parsed.repo_owner, parsed.repo_name
//             )
//         ),
//     ];

//     for command in setup_commands {
//         let join = fake_io.watch(sender.clone(), command.clone(), None).await;
//         container.exec(command.try_into()?, &mut attach_options);
//         join.abort();
//         fake_io.clear().await?;
//     }

//     // Run the uhm uhm uhm freaking uhm steps

//     let spurs = config.spurs.into_iter();
//     for spur in spurs {
//         // send to the server that we are running X pioe ... TODO

//         // UPDATE this lol
//         for step in spur.steps {
//             if let Ok(step) = serde_json::from_value::<Step>(step) {
//                 println!("[{name}] [{:?}] [{}]", step.name, step.run);
//                 let step_name = step.name;
//                 let join = fake_io
//                     .watch(sender.clone(), step_name, Some(spur.name.to_string()))
//                     .await;

//                 join.abort();
//                 fake_io.clear().await?;
//             }
//         }
//     }

//     container.stop()?;
//     Ok(())
// }

// pub struct WebsocketConnection {
//     _reader: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
//     _writer: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
//     /// Messages sent through this are read using the `internal_reader` struct.
//     internal_readers_writer: Sender<WebsocketMessage>,
//     /// Messages sent though `internal_writer` are consumed with this in the
//     /// main writer's loop.
//     internal_writers_reader: Arc<Mutex<Receiver<WebsocketMessage>>>,
//     /// Used in the main event loop to recieve messages from the isolated reader.
//     /// We can't use the reader in other places (to my knowledge) so this channel
//     /// is created to recieve stuff from the websocket in other places, not just the main
//     /// reader loop.
//     pub reader: Arc<Mutex<Receiver<WebsocketMessage>>>,
//     /// Used in throughout the code to send messages back to the server through the main writer.
//     /// The main writer (writer field on this struct) should not be used because messages sent from
//     /// some places in the code may not make it through. Use this instead.
//     pub writer: Sender<WebsocketMessage>,
//     pub heartbeat_interval: Option<u64>,
// }

// impl WebsocketConnection {
//     pub async fn create() -> Result<Self> {
//         let url = &format!(
//             "ws://{}/api/ws?name={}&password={}",
//             CONFIG.spire.host, CONFIG.name, CONFIG.password
//         );

//         let (ws_stream, _) = connect_async(url).await?;

//         let (_writer, mut _reader) = ws_stream.split();

//         let Some(Ok(msg)) = _reader.next().await else {
//             panic!("Failed to get hello message from WS.");
//         };

//         let hello_payload = match serde_json::from_str::<WebsocketMessage>(&msg.to_string()) {
//             Ok(hp) => hp,
//             Err(err) => {
//                 error!("Failed to deserialize websocket message: {err:?}");
//                 return Err(err.into());
//             }
//         };

//         let Some(hello_payload) = hello_payload.downcast_event::<Hello>() else {
//             panic!("Failed to get inner type of websocket message.");
//         };

//         let (_writer, _reader) = (Arc::new(Mutex::new(_writer)), Arc::new(Mutex::new(_reader)));

//         let (internal_writer, internal_writers_reader) = mpsc::channel::<WebsocketMessage>(200);
//         let (internal_readers_writer, internal_reader) = mpsc::channel::<WebsocketMessage>(200);

//         Ok(Self {
//             _writer,
//             _reader,
//             reader: Arc::new(Mutex::new(internal_reader)),
//             writer: internal_writer,
//             internal_readers_writer,
//             internal_writers_reader: Arc::new(Mutex::new(internal_writers_reader)),
//             heartbeat_interval: Some(hello_payload.heartbeat),
//         })
//     }

//     pub async fn start_cardiovascular_operations(&self) -> Result<JoinHandle<()>> {
//         if let Some(heartbeat_interval) = self.heartbeat_interval {
//             let writer = self.writer.clone();
//             let hb_sleep_duration = tokio::time::Duration::from_secs(heartbeat_interval);
//             return Ok(tokio::spawn(async move {
//                 loop {
//                     writer
//                         .send(WebsocketMessage {
//                             op: OpCodes::HeartBeatAck,
//                             event: None,
//                         })
//                         .await
//                         .unwrap();

//                     tokio::time::sleep(hb_sleep_duration).await;
//                 }
//             }));
//         }
//         bail!("Heartbeat Interval is none");
//     }

//     pub async fn start_reader(&self) -> Result<JoinHandle<()>> {
//         let locked_reader = self._reader.clone();
//         let outside_code_sender = self.internal_readers_writer.clone();
//         Ok(tokio::spawn(async move {
//             let mut locked_reader = locked_reader.lock().await;

//             while let Some(msg) = locked_reader.next().await {
//                 let msg = msg.unwrap();
//                 if msg.is_text() || msg.is_binary() {
//                     if let Ok(message) =
//                         serde_json::from_str::<WebsocketMessage>(&msg.into_text().unwrap())
//                     {
//                         outside_code_sender.send(message).await.unwrap();
//                     }
//                 }
//             }
//         }))
//     }

//     pub async fn start_writer(&self) -> Result<JoinHandle<()>> {
//         let locked_write = self._writer.clone();
//         let outside_code_reader = self.internal_writers_reader.clone();
//         Ok(tokio::spawn(async move {
//             let mut locked_write = locked_write.lock().await;
//             let mut locked_outside_code_reader = outside_code_reader.lock().await;

//             while let Some(to_write) = locked_outside_code_reader.recv().await {
//                 locked_write
//                     .send(Message::Text(serde_json::to_string(&to_write).unwrap()))
//                     .await
//                     .unwrap();
//             }
//         }))
//     }
// }

// pub struct CurrentRunnerJob<'a> {
//     pub job: &'a Job,
//     pub spurs: &'a Vec<Spurs>,
//     pub repo: &'a Repos
// }

// pub struct Runner<'a> {
//     pub websocket: WebsocketConnection,
//     pub current_job: Arc<Mutex<Option<CurrentRunnerJob<'a>>>>,
// }

// impl<'a> Runner<'a> {
//     pub async fn new() -> Result<Runner<'a>> {
//         let websocket = WebsocketConnection::create().await?;
//         Ok(Runner {
//             websocket,
//             current_job: Arc::new(Mutex::new(None)),
//         })
//     }

//     pub async fn identify(&self) -> Result<()> {
//         Ok(self
//             .websocket
//             .writer
//             .send(WebsocketMessage {
//                 op: OpCodes::Identify,
//                 event: Some(Box::new(Identify {
//                     name: CONFIG.name.to_owned(),
//                     password: CONFIG.password.to_owned(),
//                 })),
//             })
//             .await?)
//     }

//     pub async fn send(&self, msg: WebsocketMessage) -> Result<()> {
//         Ok(self.websocket.writer.send(msg).await?)
//     }

//     pub async fn listen(&self) -> Result<()> {
//         let mut internal_reader = self.websocket.reader.lock().await;
//         let current_job = self.current_job.clone();
//         while let Some(to_write) = internal_reader.recv().await {
//             match to_write.op {
//                 OpCodes::EventCreate => {
//                     if let Some(event) = to_write.event {
//                         let d_any = event.as_any();
//                         if d_any.type_id() == TypeId::of::<CreateJobRun>() {
//                             let Some(data) = d_any.downcast_ref::<CreateJobRun>() else {
//                                 continue;
//                             };

//                             match self
//                                 .send(WebsocketMessage {
//                                     op: OpCodes::EventCreate,
//                                     event: Some(Box::new(RequestRepoConfig {
//                                         repo: data.job.repo,
//                                     })),
//                                 })
//                                 .await
//                             {
//                                 Ok(_) => {}
//                                 Err(err) => {
//                                     error!("Failed to send websocket message:\n{err:?}");
//                                     continue;
//                                 }
//                             }

//                             let Some(repo_config_ack) = internal_reader.recv().await else {
//                                 error!("Requested repo config was empty.");
//                                 return Ok(());
//                             };
//                             let Some(job_repo_config) = repo_config_ack.downcast_event::<WsRepoConfig>() else {
//                                 error!("Failed to get inner type of websocket message.");
//                                 return Ok(());
//                             };

//                             let new_current_job = CurrentRunnerJob {
//                                 job: &data.job,
//                                 spurs: &job_repo_config.spurs,
//                                 repo: &job_repo_config.repo
//                             };

//                             let mut current_job = current_job.lock().await;
//                             // handle last job if i think of some reason to here
//                             *current_job = Some(new_current_job);
//                         }
//                     }
//                 }
//                 _ => {}
//             }
//         }
//         Ok(())
//     }
// }

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     tracing_subscriber::fmt()
//         .with_max_level(tracing::Level::INFO)
//         .pretty()
//         .init();

//     info!("Creating runner and connecting to server.");
//     let runner = Runner::new().await?;

//     let reader = runner.websocket.start_reader().await?;
//     let writer = runner.websocket.start_writer().await?;

//     // Send the identify payload. We have 10 seconds after initially connecting
//     // to the websocket to send this payload or else we'll be disconnected.
//     runner.identify().await?;

//     let cardiovascular_operations = runner.websocket.start_cardiovascular_operations().await?;

//     ctrlc::set_handler(move || {
//         info!("Peacefully shutting down runner...");
//         writer.abort();
//         reader.abort();
//         cardiovascular_operations.abort();

//         exit(0);
//     })?;

//     return Ok(runner.listen().await?);
// }

fn main() {
    println!("Hi");
}
