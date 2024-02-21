/*
The Kernel Sidecar Client is the main entrypoint for connecting to a Kernel over ZMQ and issuing
Actions (requests) to the Kernel then handling all responses with parent_header_msg id's matching
the original request.

Each Action is "complete" (awaitable) when the Kernel status has gone back to Idle and the expected
reply type has been seen (e.g. kernel_info_reply for kernel_info_request).

Message passing between background tasks is done with mpsc channels.
 - background tasks listening to iopub and shell channels push messages to a central process_message
   worker over mpsc.
 - process_message background task deserializes messages and looks up the appropriate Action based
   on parent header msg id then pushes to the Action handlers over mpsc.

Example usage, run until a kernel info request/reply has been completed and print out all ZMQ
messages coming back over iopub and shell channels:

let connection_info = ConnectionInfo::from_file("/tmp/kernel.json")
    .expect("Make sure to run python -m ipykernel_launcher -f /tmp/kernel.json");
let client = Client::new(connection_info).await;

#[derive(Debug)]
struct DebugHandler;

#[async_trait::async_trait]
impl Handler for DebugHandler {
    async fn handle(&self, msg: &Response) {
        dbg!(msg);
    }
}

let handler = DebugHandler {};
let handlers = vec![Arc::new(handler) as Arc<dyn Handler>];
let action = client.kernel_info_request(handlers).await;
action.await;
*/

use std::collections::HashMap;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Notify, RwLock};
use tokio::time::sleep;
use zeromq::{DealerSocket, ReqSocket, Socket, SocketRecv, SocketSend, SubSocket, ZmqMessage};

use crate::jupyter::actions::Action;
use crate::jupyter::handlers::Handler;
use crate::jupyter::connection_file::ConnectionInfo;
use crate::jupyter::request::Request;
use crate::jupyter::response::Response;
use crate::jupyter::shell_content::execute::ExecuteRequest;
use crate::jupyter::shell_content::kernel_info::KernelInfoRequest;
use crate::jupyter::wire_protocol::WireProtocol;

#[derive(Debug, Clone)]
pub struct Client {
    actions: Arc<RwLock<HashMap<String, mpsc::Sender<Response>>>>,
    connection_info: ConnectionInfo,
    shell_tx: mpsc::Sender<ZmqMessage>,
    shutdown_signal: Arc<Notify>,
}

impl Client {
    pub async fn new(connection_info: &ConnectionInfo) -> Self {
        let actions = Arc::new(RwLock::new(HashMap::new()));
        // message passing for methods to send requests out over shell channel via shell_worker
        let (shell_tx, shell_rx) = mpsc::channel(100);

        // message passing for iopub and shell listeners into process_message_worker
        let (process_msg_tx, process_msg_rx) = mpsc::channel(100);

        // For shutting down ZMQ listeners when Client is dropped
        let shutdown_signal = Arc::new(Notify::new());

        // spawn iopub and shell listeners
        let iopub_address = connection_info.iopub_address();
        let shell_address = connection_info.shell_address();

        tokio::spawn(iopub_worker(
            iopub_address,
            process_msg_tx.clone(),
            shutdown_signal.clone(),
        ));
        tokio::spawn(shell_worker(
            shell_address,
            shell_rx,
            process_msg_tx.clone(),
            shutdown_signal.clone(),
        ));

        // spawn process_message_worker
        tokio::spawn(process_message_worker(
            process_msg_rx,
            actions.clone(),
            shutdown_signal.clone(),
        ));

        Client {
            actions,
            connection_info: connection_info.clone(),
            shell_tx,
            shutdown_signal,
        }
    }

    // Try to connect to the heartbeat channel and send a ping message
    // You can use this as a way to wait for a new Kernel to come up or check if it's connected
    pub async fn heartbeat(&self) {
        loop {
            let mut socket = ReqSocket::new();

            // Try to connect to the heartbeat channel
            if let Err(_e) = socket
                .connect(self.connection_info.heartbeat_address().as_str())
                .await
            {
                sleep(Duration::from_millis(50)).await;
                continue; // If connection fails, retry in the next iteration of the loop
            }

            // Send a ping message
            let ping_msg = ZmqMessage::from("ping");
            if let Err(_e) = socket.send(ping_msg).await {
                sleep(Duration::from_millis(50)).await;
                continue; // If sending fails, retry in the next iteration of the loop
            }

            // Wait for a pong message
            match socket.recv().await {
                Ok(_) => {
                    break; // Successful pong message received, break the loop
                }
                Err(_) => {
                    sleep(Duration::from_millis(50)).await;
                    continue; // If receiving fails, retry in the next iteration of the loop
                }
            }
        }
    }

    // Creates an Action from a request + handlers, serializes the request to be sent over ZMQ,
    // sends over shell channel, and registers the request header msg_id in the Actions hashmap
    // so that all response messages can get routed to the appropriate Action handlers
    async fn send_request(
        &self,
        request: Request,
        handlers: Vec<Arc<Mutex<dyn Handler>>>,
    ) -> Action {
        let (msg_tx, msg_rx) = mpsc::channel(100);
        let action = Action::new(request, handlers, msg_rx);
        let msg_id = action.request.msg_id();
        self.actions.write().await.insert(msg_id.clone(), msg_tx);
        let wp: WireProtocol = action.request.into_wire_protocol(&self.connection_info.key);
        let zmq_msg: ZmqMessage = wp.into();
        self.shell_tx.send(zmq_msg).await.unwrap();
        action
    }

    pub async fn kernel_info_request(&self, handlers: Vec<Arc<Mutex<dyn Handler>>>) -> Action {
        let request = KernelInfoRequest::new();
        self.send_request(request.into(), handlers).await
    }

    pub async fn execute_request(
        &self,
        code: String,
        handlers: Vec<Arc<Mutex<dyn Handler>>>,
    ) -> Action {
        let request = ExecuteRequest::new(code);
        self.send_request(request.into(), handlers).await
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.shutdown_signal.notify_waiters();
    }
}

/// The tasks listening on iopub and shell channels will push any messages they receive into this
/// processing function. Its job is to deserialize ZmqMessage into the appropriate Jupyter message
/// and then delegate it to the appropriate Action to be handled based on parent msg_id.
async fn process_message_worker(
    mut msg_rx: mpsc::Receiver<ZmqMessage>,
    actions: Arc<RwLock<HashMap<String, mpsc::Sender<Response>>>>,
    shutdown_signal: Arc<Notify>, // hook to shutdown background task if Client is dropped
) {
    loop {
        tokio::select! {
            Some(zmq_msg) = msg_rx.recv() => {
                let response: Response = zmq_msg.into();
                let msg_id = response.parent_msg_id();
                if msg_id.is_none() {
                    dbg!("No parent msg id, skipping msg_type {}", response.msg_type());
                    continue;
                }
                let msg_id = msg_id.unwrap();
                if let Some(action) = actions.read().await.get(&msg_id) {
                   let sent = action.send(response).await;
                   // If we're seeing SendError here, it means we're still seeing ZMQ messages with
                   // parent header msg id matching a request / Action that is "completed" and has
                   // shut down its mpsc Receiver channel. That's probably happening because the
                   // Action is not configured to expect some Reply type and is "finishing" when
                   // Kernel status goes Idle but then we send along another Reply messages to a
                   // shutdown mpsc Receiver channel.
                   match sent {
                          Ok(_) => {},
                          Err(e) => {
                            dbg!(e);
                          }
                   }
                }
            },
            _ = shutdown_signal.notified() => {
                break;
            }
        }
    }
}

/// iopub channel background task is only responsible for listening to the iopub channel and pushing
/// messages to the process_message_worker. We never send anything out on the iopub channel.
async fn iopub_worker(
    iopub_address: String,
    msg_tx: mpsc::Sender<ZmqMessage>,
    shutdown_signal: Arc<Notify>,
) {
    let mut socket = SubSocket::new();
    socket.connect(iopub_address.as_str()).await.unwrap();
    socket.subscribe("").await.unwrap();

    loop {
        tokio::select! {
            Ok(msg) = socket.recv() => {
                msg_tx.send(msg).await.unwrap();
            },
            _ = shutdown_signal.notified() => {
                break;
            }
        }
    }
}

/// shell channel background task needs to have a way for the Client to send stuff out over shell
/// in addition to listening for replies coming back on the channel, then pushing those to the
/// process_message_worker.
async fn shell_worker(
    shell_address: String,
    mut msg_rx: mpsc::Receiver<ZmqMessage>, // Client wants to send Jupyter message over ZMQ
    msg_tx: mpsc::Sender<ZmqMessage>,       // Kernel sent a reply over ZMQ, needs to get processed
    shutdown_signal: Arc<Notify>,
) {
    let mut socket = DealerSocket::new();
    socket.connect(shell_address.as_str()).await.unwrap();

    loop {
        tokio::select! {
            Some(client_to_kernel_msg) = msg_rx.recv() => {
                socket.send(client_to_kernel_msg).await.unwrap();
            }
            kernel_to_client_msg = socket.recv() => {
                match kernel_to_client_msg {
                    Ok(msg) => {
                        msg_tx.send(msg).await.unwrap();
                    }
                    Err(e) => {
                        dbg!(e);
                    }
                }
            },
            _ = shutdown_signal.notified() => {
                break;
            }
        }
    }
}
